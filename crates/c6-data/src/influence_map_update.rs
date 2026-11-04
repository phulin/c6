//! Current-state eager InfluenceMap rebuild with source movement reachability.
mod unit_jobs;
use crate::{
    formation_movement_cost, formation_rules,
    game_parameters::{IntegerParameter, Parameters},
    loaded_actions::city_range_attack,
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    unit_placement_services::AlivePlayers,
    unit_sight_state, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    influence_map_update as kernel,
    loaded_state::{self, LoadedState},
    map::UnitReference,
    player_districts::District,
    unit::{PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    State(loaded_state::Error),
    World(c6_core::world::MissingInput),
    Rules(crate::Error),
    Definitions(unit_initialization_definitions::Error),
    Parameter(crate::game_parameters::Error),
    Combat(crate::loaded_actions::Error),
    Defender(crate::combat_defender::Error),
    Movement(formation_movement_cost::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Path(crate::formation_path_state::Error),
    Strength(crate::melee_attack::strength::Error),
    Missing(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "influence rebuild: {self:?}")
    }
}
impl std::error::Error for Error {}

pub struct Definitions {
    binding: WireBinding,
    features: LinuxDefinitionRegistry,
    danger: Vec<i32>,
    districts: LinuxDefinitionRegistry,
    hit_points: Vec<Option<i32>>,
    city_strength: Vec<i32>,
    levels: LinuxDefinitionRegistry,
    receives_influence: Vec<i32>,
    militia: i32,
    bombard_modifier: i32,
    max_hit_points: i32,
    buildings: LinuxDefinitionRegistry,
    building_strength: Vec<i32>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if state.binding() != authority.destination_binding() {
            return Err(Error::Missing("rules binding"));
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(Error::Definitions)?;
        let values = |table, field| -> Result<Vec<Option<i32>>, Error> {
            let table = rules.gameplay().table(table).ok_or(Error::Missing(table))?;
            let mut rows = table
                .rows()
                .map(|row| Ok((row.row_id().ok_or(Error::Missing("definition rowid"))?, row)))
                .collect::<Result<Vec<_>, Error>>()?;
            rows.sort_by_key(|(id, _)| id.0);
            rows.into_iter()
                .map(|(_, row)| match row.get(field).map_err(Error::Rules)? {
                    SqlValue::Null => Ok(None),
                    value => crate::game_parameters::sql_integer(value)
                        .map(Some)
                        .ok_or(Error::Missing(field)),
                })
                .collect()
        };
        let integer = |table, field| -> Result<Vec<i32>, Error> {
            values(table, field)?
                .into_iter()
                .map(|v| v.ok_or(Error::Missing(field)))
                .collect()
        };
        Ok(Self {
            binding: state.binding().clone(),
            features: collection(rules, "Features", "FeatureType").map_err(Error::Definitions)?,
            danger: integer("Features", "DangerValue")?,
            districts: collection(rules, "Districts", "DistrictType")
                .map_err(Error::Definitions)?,
            hit_points: values("Districts", "HitPoints")?,
            city_strength: integer("Districts", "CityStrengthModifier")?,
            levels: collection(rules, "CivilizationLevels", "CivilizationLevelType")
                .map_err(Error::Definitions)?,
            receives_influence: integer("CivilizationLevels", "CanReceiveInfluence")?,
            max_hit_points: Parameters::from_rules(rules)
                .map_err(Error::Parameter)?
                .integer(IntegerParameter::CombatMaxHitPoints)
                .map_err(Error::Parameter)?,
            buildings: collection(rules, "Buildings", "BuildingType")
                .map_err(Error::Definitions)?,
            building_strength: integer("Buildings", "OuterDefenseStrength")?,
            militia: Parameters::from_rules(rules)
                .map_err(Error::Parameter)?
                .integer(IntegerParameter::CombatGarrisonMilitiaModifier)
                .map_err(Error::Parameter)?,
            bombard_modifier: Parameters::from_rules(rules)
                .map_err(Error::Parameter)?
                .integer(IntegerParameter::CombatBombardVsUnitStrengthModifier)
                .map_err(Error::Parameter)?,
        })
    }
}

pub struct Queries<'a, 'r> {
    pub alive_players: &'a AlivePlayers,
    pub visibility: &'a unit_sight_state::Runtime,
    pub movement: &'a formation_movement_cost::Definitions<'r>,
    pub formation: &'a formation_rules::Definitions,
    pub city_range: &'a city_range_attack::Queries<'a>,
    pub path: Option<&'a crate::formation_path_state::Definitions<'a, 'r>>,
    pub path_runtime: Option<&'a crate::formation_path_state::Runtime<'a>>,
    pub strength: Option<&'a crate::melee_attack::strength::Definitions>,
}
type DistrictId = (i32, u32);
struct Current<'a, 'q, 'r> {
    input_stamp: loaded_state::ActionStamp,
    state: &'a mut LoadedState,
    definitions: &'a Definitions,
    queries: &'a Queries<'q, 'r>,
    owner: i32,
}
impl Current<'_, '_, '_> {
    fn player(&self, owner: i32) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
        self.state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("current retained Player"))
    }
    fn unit(&self, unit: UnitReference) -> Result<&Unit, Error> {
        self.state
            .world()
            .unit(unit)
            .map_err(Error::World)?
            .map(|u| u.record)
            .ok_or(Error::Missing("current Unit"))
    }
    fn district(&self, (owner, id): DistrictId) -> Result<&District, Error> {
        self.player(owner)?
            .districts
            .records
            .entries
            .iter()
            .find(|e| e.value.id == id)
            .map(|e| &e.value)
            .ok_or(Error::Missing("current District"))
    }
    fn plot(&self, district: &District) -> Result<usize, Error> {
        let map = self.state.world().map();
        let (x, y) = district.location;
        if x == -9999 || y == -9999 {
            return Err(Error::Missing("District.GetPlot location"));
        }
        let x = c6_core::movement_world::coord_range(x, map.header.width, map.header.wrap_x);
        let y = c6_core::movement_world::coord_range(y, map.header.height, map.header.wrap_y);
        map.plot_at(x, y)
            .ok_or(Error::Missing("District.GetPlot"))?;
        Ok((y as usize) * map.header.width as usize + x as usize)
    }
    fn owner_city(&self, district: &District) -> Result<(i32, &c6_core::city::City), Error> {
        let index = self.plot(district)?;
        let plot = &self.state.world().map().plots[index];
        let owner = crate::movement_placement_state::plot_owner(plot)
            .map_err(|_| Error::Missing("district plot owner"))?;
        Ok((
            owner,
            self.state
                .city(owner, district.city_id)
                .map_err(Error::State)?,
        ))
    }
    fn level(&self, owner: i32) -> Result<u32, Error> {
        let mut values = self
            .queries
            .city_range
            .civilization_level_types
            .iter()
            .filter(|(id, _)| *id == owner);
        let level = values
            .next()
            .ok_or(Error::Missing("current CivilizationLevel"))?
            .1;
        if values.next().is_some() {
            return Err(Error::Missing("duplicate CivilizationLevel owner"));
        }
        Ok(level)
    }
    fn defender_without_air(&self, location: PlotCoord) -> Result<Option<UnitReference>, Error> {
        use c6_core::combat_defender::{self, Context};
        let mut current = crate::combat_defender::Current::new(
            self.state,
            self.queries.formation,
            location,
            Some(0),
        );
        let mut index = 0;
        let mut selected = None;
        while index < current.candidate_count().map_err(Error::Defender)? {
            if let Some(unit) = current.resolve(index).map_err(Error::Defender)? {
                if !current.delayed_death(unit).map_err(Error::Defender)?
                    && current.owner(unit).map_err(Error::Defender)? != -1
                    && combat_defender::is_combat(&mut current, unit).map_err(Error::Defender)?
                {
                    // flags0x10 excludes air after IsCombat, even with null
                    // attacker. Read the live cached domain, not a SQL default.
                    let record = self.unit(unit)?;
                    if self
                        .state
                        .unit_runtime(record.e_owner, record.id)
                        .map_err(Error::State)?
                        .domain
                        != 1
                    {
                        let replace = match selected {
                            None => true,
                            Some(old) => {
                                current.combat_false(unit).map_err(Error::Defender)?
                                    > current.combat_false(old).map_err(Error::Defender)?
                            }
                        };
                        if replace {
                            selected = Some(unit);
                        }
                    }
                }
            }
            index += 1;
        }
        Ok(selected)
    }
    fn defense(&self, district: &District) -> Result<i32, Error> {
        let Some(index) = self
            .definitions
            .districts
            .find_index(district.district_hash)
        else {
            return Ok(0);
        };
        if self.definitions.hit_points[index].unwrap_or(0) <= 0 {
            return Ok(0);
        }
        let (owner, city) = self.owner_city(district)?;
        let stats_owner = if self.level(owner)? == 0x04e6b0a0 {
            city.header.original_owner
        } else {
            owner
        };
        let base = self
            .player(stats_owner)?
            .stats
            .max_melee_strength_trained
            .wrapping_shl(8)
            .wrapping_sub(self.definitions.militia.wrapping_shl(8));
        let mut modifiers = 0i32;
        if let Some(garrison) = self.defender_without_air(PlotCoord {
            x: district.location.0,
            y: district.location.1,
        })? {
            let unit = self.unit(garrison)?;
            let combat = self
                .queries
                .formation
                .combat(self.state, unit)
                .map_err(Error::Combat)?
                .wrapping_shl(8);
            let percent = unit
                .i_damage
                .wrapping_mul(100)
                .checked_div(self.definitions.max_hit_points)
                .ok_or(Error::Missing("District garrison damage division"))?;
            let reduction = self
                .queries
                .strength
                .ok_or(Error::Missing("garrison combat definitions"))?
                .reduction_for_damage(
                    self.state,
                    percent,
                    unit.k_combat_modifers
                        .i_strength_reduction_for_damage_modifier,
                )
                .map_err(Error::Strength)?;
            let strength = combat.wrapping_sub(reduction);
            if strength > base {
                modifiers = strength.wrapping_sub(base);
            }
        }
        let urban = if self.level(owner)? == 0x04e6b0a0 {
            self.state
                .retained()
                .free_cities
                .instances
                .iter()
                .find(|free| {
                    free.living_raw != 0
                        && city.header.location_x != -9999
                        && city.header.location_y != -9999
                        && free.city_location.x == city.header.location_x
                        && free.city_location.y == city.header.location_y
                })
                .map_or(0, |free| free.urban_defenses)
        } else {
            self.player(owner)?.cities.urban_defenses
        };
        let maximum = city_range_attack::ordinary_outer_defense_hit_points(
            city,
            urban,
            self.queries.city_range.definitions,
        )
        .map_err(Error::Combat)?;
        if district.outer_defenses_damage < maximum {
            let mut outer = 0i32;
            for (i, building) in self.definitions.buildings.entries().iter().enumerate() {
                let hash = crate::registry::linux_type_hash(&building.type_name);
                let mut entries = city
                    .buildings
                    .building_locations
                    .iter()
                    .filter(|e| e.definition_hash == hash);
                let location = entries
                    .next()
                    .ok_or(Error::Missing("Building location array"))?;
                if entries.next().is_some() {
                    return Err(Error::Missing("duplicate Building location"));
                }
                if location.value >= 0 {
                    outer = outer.wrapping_add(self.definitions.building_strength[i]);
                }
            }
            if outer > 0 {
                modifiers = modifiers.wrapping_add(
                    outer
                        .wrapping_add(city.combat.outer_defense_strength_modifier)
                        .wrapping_shl(8),
                );
            }
        }
        let mut district_strength = 0i32;
        for id in &city.districts.district_ids {
            if let Some(entry) = self
                .player(owner)?
                .districts
                .records
                .entries
                .iter()
                .find(|e| e.value.id == *id)
            {
                let district = &entry.value;
                if district.is_complete && !district.pillaged {
                    let i = self
                        .definitions
                        .districts
                        .find_index(district.district_hash)
                        .ok_or(Error::Missing("City District definition"))?;
                    district_strength =
                        district_strength.wrapping_add(self.definitions.city_strength[i]);
                }
            }
        }
        if self.state.player_alive(owner).map_err(Error::State)? {
            if let Some(i) = self.definitions.levels.find_index(self.level(owner)?) {
                if self.definitions.receives_influence[i] != 0 {
                    return Err(Error::Missing("District defense received-envoy strength"));
                }
            }
        }
        Ok(base
            .wrapping_add(modifiers)
            .wrapping_add(district_strength.max(0).wrapping_shl(8))
            .wrapping_add(city.combat.inner_defense_strength_modifier.wrapping_shl(8)))
    }
}
impl kernel::Context for Current<'_, '_, '_> {
    type Error = Error;
    type Unit = UnitReference;
    type District = DistrictId;
    fn is_human(&mut self) -> Result<bool, Error> {
        Ok(self
            .state
            .player_slot_status(self.owner)
            .map_err(Error::State)?
            == 3)
    }
    fn turn_active(&mut self) -> Result<bool, Error> {
        self.state.turn_active(self.owner).map_err(Error::State)
    }
    fn plot_count(&mut self) -> Result<usize, Error> {
        Ok(self.state.world().map().plots.len())
    }
    fn influence(&mut self) -> Result<&mut c6_core::player_ai_influence_map::InfluenceMap, Error> {
        self.state
            .influence_map_mut(self.owner)
            .map_err(Error::State)
    }
    fn alive_count(&mut self) -> Result<usize, Error> {
        Ok(self.queries.alive_players.values().len())
    }
    fn alive_at(&mut self, index: usize) -> Result<i32, Error> {
        self.queries
            .alive_players
            .values()
            .get(index)
            .copied()
            .ok_or(Error::Missing("AliveIDs index"))
    }
    fn at_war(&mut self, other: i32) -> Result<bool, Error> {
        if self
            .state
            .world()
            .valid_player(self.owner)
            .map_err(Error::World)?
            .is_none()
            || !self.state.player_alive(self.owner).map_err(Error::State)?
            || other == self.owner
        {
            return Ok(false);
        }
        self.state
            .game_relations()
            .map_err(Error::State)?
            .is_at_war(other, self.owner)
            .map_err(Error::Relations)
    }
    fn unit_count(&mut self, owner: i32) -> Result<usize, Error> {
        Ok(self
            .state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .unit_records()
            .map_err(Error::World)?
            .len())
    }
    fn unit_at(&mut self, owner: i32, index: usize) -> Result<UnitReference, Error> {
        let records = self
            .state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .unit_records()
            .map_err(Error::World)?;
        let unit = &records
            .get(index)
            .ok_or(Error::Missing("Unit iterator"))?
            .value;
        Ok(UnitReference::from_component_id(
            c6_core::unit::ComponentId {
                owner: owner as i16,
                component_type: 1,
                id: unit.id,
            },
        ))
    }
    fn visibility(&mut self) -> Result<(), Error> {
        self.player(self.owner).map(|_| ())
    }
    fn delayed_death(&mut self, unit: UnitReference) -> Result<bool, Error> {
        Ok(self.unit(unit)?.b_delayed_death)
    }
    fn is_combat(&mut self, unit: UnitReference) -> Result<bool, Error> {
        crate::combat_defender::is_combat(self.state, self.queries.formation, self.unit(unit)?)
            .map_err(Error::Defender)
    }
    fn valid_location(&mut self, unit: UnitReference) -> Result<bool, Error> {
        let location = self.unit(unit)?.k_location;
        Ok(location.x > -9999 && location.y > -9999)
    }
    fn unit_visible(&mut self, unit: UnitReference) -> Result<bool, Error> {
        self.queries
            .movement
            .unit_visible(
                self.state,
                self.unit(unit)?,
                self.owner,
                self.queries.visibility,
            )
            .map_err(Error::Movement)
    }
    fn unit_jobs(&mut self, units: &[UnitReference]) -> Result<(), Error> {
        self.apply_unit_jobs(units)
    }
    fn enemy_district(&mut self, district: DistrictId) -> Result<(), Error> {
        self.apply_enemy_district(district)
    }
    fn friendly_ranged_district(&mut self, district: DistrictId) -> Result<(), Error> {
        self.suppress_friendly_district(district)
    }
    fn district_count(&mut self, owner: Option<i32>) -> Result<usize, Error> {
        Ok(self
            .player(owner.unwrap_or(self.owner))?
            .districts
            .records
            .entries
            .len())
    }
    fn district_at(&mut self, owner: Option<i32>, index: usize) -> Result<DistrictId, Error> {
        let owner = owner.unwrap_or(self.owner);
        Ok((
            owner,
            self.player(owner)?
                .districts
                .records
                .entries
                .get(index)
                .ok_or(Error::Missing("District iterator"))?
                .value
                .id,
        ))
    }
    fn district_visible(&mut self, district: DistrictId) -> Result<bool, Error> {
        self.visibility()?;
        let index = self.plot(self.district(district)?)?;
        crate::formation_path_state::shared_plot_visible(self.queries.visibility, self.owner, index)
            .map_err(|_| Error::Missing("District visibility"))
    }
    fn district_plot(&mut self, district: DistrictId) -> Result<usize, Error> {
        self.plot(self.district(district)?)
    }
    fn district_defense_q8(&mut self, district: DistrictId) -> Result<i32, Error> {
        self.defense(self.district(district)?)
    }
    fn district_can_attack(&mut self, district: DistrictId) -> Result<bool, Error> {
        let district = self.district(district)?;
        let (_, city) = self.owner_city(district)?;
        Ok(city_range_attack::attack_strength_q8(
            self.state,
            city,
            district,
            self.queries.city_range,
        )
        .map_err(Error::Combat)?
            > 0)
    }
    fn feature_danger(&mut self, plot: usize) -> Result<Option<i32>, Error> {
        let feature = self
            .state
            .world()
            .map()
            .plots
            .get(plot)
            .ok_or(Error::Missing("Map.GetPlotByIndex"))?
            .feature_hash;
        if feature == u32::MAX {
            return Ok(None);
        }
        let index = self
            .definitions
            .features
            .find_index(feature)
            .ok_or(Error::Missing("Feature definition"))?;
        Ok(Some(self.definitions.danger[index]))
    }
}

pub fn dispatch_if_active(
    state: &mut LoadedState,
    definitions: &Definitions,
    queries: &Queries<'_, '_>,
    owner: i32,
) -> Result<bool, kernel::Error<Error>> {
    if state.binding() != &definitions.binding {
        return Err(kernel::Error::Backend(Error::Missing(
            "Influence rules binding",
        )));
    }
    kernel::on_unit_removed(&mut Current {
        input_stamp: state.action_stamp(),
        state,
        definitions,
        queries,
        owner,
    })
}

/// Turn activation calls UpdateInfluenceMap directly, including non-human
/// players; its caller owns the preceding turn transition and callback order.
pub fn update(
    state: &mut LoadedState,
    definitions: &Definitions,
    queries: &Queries<'_, '_>,
    owner: i32,
) -> Result<(), kernel::Error<Error>> {
    if state.binding() != &definitions.binding {
        return Err(kernel::Error::Backend(Error::Missing(
            "Influence rules binding",
        )));
    }
    kernel::update(&mut Current {
        input_stamp: state.action_stamp(),
        state,
        definitions,
        queries,
        owner,
    })
}
