//! Linux XP2 camp unit selection and creation (2825a00, 2825bc0, 2825cd0,
//! 2825fb0, 2826820, 2828c30). Uses the ordinary canonical Unit factory.
//! Unsupported placement-search and active-clan branches retain their prefix
//! and return errors; they never report a completed camp or invent a Unit.
use crate::{
    registry::linux_type_hash, unit_creation as creation,
    unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::PlotFlag,
    movement_shore::PlacementRequest,
    unit::{ComponentId, PlotCoord},
};

#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Creation(Box<creation::Error<E>>),
    Placement(crate::movement_placement_state::Error),
}
struct Unit {
    hash: u32,
    combat: i32,
    sea: bool,
    major_only: bool,
    prerequisites: [Option<usize>; 2],
}
struct Tribe {
    hash: u32,
    defender: Vec<u8>,
    scout: Vec<u8>,
    military: [Option<Vec<u8>>; 4],
}
pub struct Definitions {
    binding: WireBinding,
    units: Vec<Unit>,
    tribes: Vec<Tribe>,
    names: Vec<(u32, u32)>,
    extra: Vec<(u32, u32, i32)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Missing(name));
        let units = collection(rules, "Units", "UnitType").map_err(Error::Definitions)?;
        let techs =
            collection(rules, "Technologies", "TechnologyType").map_err(Error::Definitions)?;
        let civics = collection(rules, "Civics", "CivicType").map_err(Error::Definitions)?;
        let mut values = Vec::new();
        for definition in units.entries() {
            let row = table("Units")?
                .row_by_id(definition.database_rowid)
                .ok_or(Error::Missing("Unit row"))?;
            let reference = |name, registry: &crate::registry::LinuxDefinitionRegistry| match row
                .get(name)
                .map_err(Error::Rules)?
            {
                SqlValue::Null => Ok(None),
                SqlValue::Text(text) => registry
                    .find_index(linux_type_hash(text))
                    .map(Some)
                    .ok_or(Error::Missing("Unit prerequisite reference")),
                _ => Err(Error::Missing("Unit prerequisite text")),
            };
            let hash = linux_type_hash(&definition.type_name);
            let mut major_only = false;
            for xp in table("Units_XP2")?.rows() {
                if linux_type_hash(xp.text("UnitType").map_err(Error::Rules)?.as_bytes()) == hash {
                    major_only = xp.integer("MajorCivOnly").map_err(Error::Rules)? != 0;
                }
            }
            values.push(Unit {
                hash,
                combat: i32::try_from(row.integer("Combat").map_err(Error::Rules)?)
                    .map_err(|_| Error::Missing("Unit Combat"))?,
                sea: row.text("Domain").map_err(Error::Rules)? == "DOMAIN_SEA",
                major_only,
                prerequisites: [
                    reference("PrereqTech", &techs)?,
                    reference("PrereqCivic", &civics)?,
                ],
            });
        }
        let mut tribes = Vec::new();
        for row in table("BarbarianTribes")?.rows() {
            tribes.push(Tribe {
                hash: linux_type_hash(row.text("TribeType").map_err(Error::Rules)?.as_bytes()),
                defender: row
                    .text("DefenderTag")
                    .map_err(Error::Rules)?
                    .as_bytes()
                    .to_vec(),
                scout: row
                    .text("ScoutTag")
                    .map_err(Error::Rules)?
                    .as_bytes()
                    .to_vec(),
                military: ["MeleeTag", "RangedTag", "SiegeTag", "SupportTag"]
                    .map(|column| match row.get(column) {
                        Ok(SqlValue::Text(value)) => Ok(Some(value.to_vec())),
                        Ok(SqlValue::Null) => Ok(None),
                        Ok(_) => Err(Error::Missing("tribe Unit tag")),
                        Err(error) => Err(Error::Rules(error)),
                    })
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()?
                    .try_into()
                    .map_err(|_| Error::Missing("tribe Unit tag extent"))?,
            });
        }
        let mut names = Vec::new();
        for row in table("BarbarianTribeNames")?.rows() {
            names.push((
                linux_type_hash(row.text("TribeNameType").map_err(Error::Rules)?.as_bytes()),
                linux_type_hash(row.text("TribeType").map_err(Error::Rules)?.as_bytes()),
            ));
        }
        let mut extra = Vec::new();
        for row in table("BarbarianTribe_ExtraUnits")?.rows() {
            extra.push((
                linux_type_hash(row.text("TribeType").map_err(Error::Rules)?.as_bytes()),
                linux_type_hash(row.text("UnitType").map_err(Error::Rules)?.as_bytes()),
                i32::try_from(row.integer("Number").map_err(Error::Rules)?)
                    .map_err(|_| Error::Missing("extra Unit count"))?,
            ));
        }
        Ok(Self {
            binding: state.binding().clone(),
            units: values,
            tribes,
            names,
            extra,
        })
    }
    fn tribe<E>(&self, state: &LoadedState, instance: usize) -> Result<&Tribe, Error<E>> {
        let camp = state
            .retained()
            .barbarian
            .instances
            .get(instance)
            .ok_or(Error::Missing("camp instance"))?;
        let hash = self
            .names
            .iter()
            .find(|n| n.0 == camp.tribe_name_hash)
            .ok_or(Error::Missing("camp tribe name definition"))?
            .1;
        self.tribes
            .iter()
            .find(|t| t.hash == hash)
            .ok_or(Error::Missing("camp tribe definition"))
    }
    /// Current native-order strongest eligible tag member; equal Combat keeps
    /// the earlier definition. This is rules selection, without a policy draw.
    pub fn find_best<E>(
        &self,
        state: &LoadedState,
        instance: usize,
        tag: &[u8],
        build_maps: &creation::signals::BuildMaps,
        tags: &crate::loaded_actions::reporting::hold_cycling::commands::Tags,
        clans_active: bool,
    ) -> Result<Option<u32>, Error<E>> {
        if state.binding() != &self.binding || build_maps.binding != self.binding {
            return Err(Error::Missing("camp Unit binding"));
        }
        let camp = state
            .retained()
            .barbarian
            .instances
            .get(instance)
            .ok_or(Error::Missing("camp instance"))?;
        let owner = camp.player_ordinal;
        let player = state.world().player(owner).map_err(Error::World)?;
        let maps = build_maps
            .players
            .iter()
            .find(|p| p.0 == owner)
            .ok_or(Error::Missing("camp owner Unit build map"))?;
        let mut best: Option<(u32, i32)> = None;
        for (index, unit) in self.units.iter().enumerate() {
            if !tags.has(unit.hash, tag) {
                continue;
            }
            // CanEverTrain21c10a0 reads the current replacement map cell.
            let mapped = maps
                .1
                .get(index)
                .ok_or(Error::Missing("camp Unit replacement cell"))?;
            let mut eligible = *mapped == index as i32 && !unit.major_only;
            if eligible {
                for (kind, prerequisite) in unit.prerequisites.iter().enumerate() {
                    if let Some(prerequisite) = prerequisite {
                        let mask = if kind == 0 {
                            &player.technologies
                        } else {
                            &player.civics
                        };
                        let acquired = mask
                            .as_ref()
                            .and_then(|m| m.get(*prerequisite))
                            .copied()
                            .ok_or(Error::Missing("camp Unit prerequisite mask"))?;
                        if !acquired {
                            if kind == 0 && clans_active {
                                return Err(Error::Missing("active clan acquired technology"));
                            }
                            eligible = false;
                            break;
                        }
                    }
                }
            }
            // Source ORs the clans-specific eligibility with the manager result
            // even when CanTribeTrainUnitNow already succeeded.
            if clans_active {
                return Err(Error::Missing("active clan Unit eligibility"));
            }
            if eligible && best.is_none_or(|(_, combat)| unit.combat > combat) {
                best = Some((unit.hash, unit.combat));
            }
        }
        Ok(best.map(|v| v.0))
    }
}

/// Borrow the existing factory and its actual mutable callback/sight owners.
/// No cloned World, captured chosen Unit, or after-state initializer is accepted.
pub struct Context<'a, 's, 'd, 'r, E> {
    pub factory: &'a mut creation::Context<'s, 'd, 'r, E>,
    pub definitions: &'a Definitions,
    pub build_maps: &'a creation::signals::BuildMaps,
    pub clans_active: bool,
}
impl<E: creation::FactoryEffects> Context<'_, '_, '_, '_, E> {
    pub fn create_tag(
        &mut self,
        instance: usize,
        location: PlotCoord,
        tag: &[u8],
        range: u32,
        count: i32,
    ) -> Result<Vec<ComponentId>, Error<<E as creation::Effects>::Error>> {
        let unit = self.definitions.find_best(
            self.factory.state,
            instance,
            tag,
            self.build_maps,
            self.factory.tags,
            self.clans_active,
        )?;
        match unit {
            Some(hash) => self.create_units(instance, location, hash, range, count),
            None => Ok(Vec::new()),
        }
    }
    pub fn create_units(
        &mut self,
        instance: usize,
        location: PlotCoord,
        hash: u32,
        range: u32,
        count: i32,
    ) -> Result<Vec<ComponentId>, Error<<E as creation::Effects>::Error>> {
        if range > 1024 {
            return Err(Error::Missing("camp spawn radius resource limit"));
        }
        let Some(definition) = self.definitions.units.iter().find(|u| u.hash == hash) else {
            return Ok(Vec::new());
        };
        let sea = definition.sea;
        let f = &mut self.factory;
        let camp = f
            .state
            .retained()
            .barbarian
            .instances
            .get(instance)
            .ok_or(Error::Missing("camp instance"))?;
        let owner = camp.player_ordinal;
        let tribe_index = camp.tribe_index;
        let mut result = Vec::new();
        for radius in 0..=range {
            for (plot_index, at) in c6_core::influence_map_update::threats::ring(
                f.state.world().map(),
                location,
                radius,
            ) {
                if result.len() as i32 >= count {
                    continue;
                }
                let plot = &f.state.world().map().plots[plot_index];
                if plot.flags.get(PlotFlag::Impassable) {
                    continue;
                }
                if crate::unit_placement_services::district_type(
                    f.state,
                    f.effects,
                    plot_index as u32,
                )
                .map_err(|e| Error::Creation(Box::new(creation::Error::Services(e))))?
                    != -1
                {
                    continue;
                }
                if plot.unit_count != 0 {
                    continue;
                }
                let query = f
                    .queries
                    .placement_context(f.state, f.placement.placement)
                    .map_err(Error::Placement)?;
                if query.water(plot).map_err(Error::Placement)? != sea {
                    continue;
                }
                let created = creation::PlayerContext {
                    state: f.state,
                    definitions: f.definitions,
                    resources: f.resources,
                    movement: f.movement,
                    combat: f.combat,
                    runtime: f.runtime,
                    tags: f.tags,
                    effects: f.effects,
                }
                .create_without_city(owner, hash)
                .map_err(|e| Error::Creation(Box::new(e)))?;
                let Some(unit) = created else {
                    continue;
                };
                f.state
                    .unit_mut(owner, unit.id)
                    .map_err(Error::State)?
                    .i_barbarian_tribe_index = tribe_index;
                let query = f
                    .queries
                    .placement_context(f.state, f.placement.placement)
                    .map_err(Error::Placement)?;
                let valid = query
                    .unit_placement(
                        owner,
                        unit.id,
                        at,
                        PlacementRequest::from_validate_options(0x10),
                    )
                    .map_err(Error::Placement)?;
                if valid != 1 {
                    return Err(Error::Missing(
                        "camp spawn IsValidLocation radius-two search/failed Unit removal",
                    ));
                }
                let _ = crate::unit_placement_state::creation::set(
                    f.state,
                    f.placement,
                    f.queries,
                    f.sight,
                    f.effects,
                    crate::unit_placement_state::Request {
                        actor: (owner, unit.id),
                        destination: at,
                        validation_flags: 0,
                    },
                    f.tags,
                )
                .map_err(|e| Error::Creation(Box::new(creation::Error::Placement(Box::new(e)))))?;
                result.push(unit);
                f.effects
                    .game_unit_created(f.state, Some(unit), None)
                    .map_err(|e| Error::Creation(Box::new(e)))?;
            }
        }
        Ok(result)
    }
    pub fn create_scouts(
        &mut self,
        instance: usize,
        location: PlotCoord,
        range: u32,
        count: i32,
    ) -> Result<bool, Error<<E as creation::Effects>::Error>> {
        let tag = self
            .definitions
            .tribe(self.factory.state, instance)?
            .scout
            .clone();
        let units = self.create_tag(instance, location, &tag, range, count)?;
        for unit in &units {
            self.factory
                .state
                .unit_mut(i32::from(unit.owner), unit.id)
                .map_err(Error::State)?
                .e_ai_type = 2;
        }
        let camp = self
            .factory
            .state
            .barbarian_manager_mut()
            .map_err(Error::State)?
            .instances
            .get_mut(instance)
            .ok_or(Error::Missing("camp instance"))?;
        camp.cur_num_scouts = camp.cur_num_scouts.wrapping_add(units.len() as i32);
        Ok(!units.is_empty())
    }
    pub fn create_military(
        &mut self,
        instance: usize,
        location: PlotCoord,
        kind: crate::barbarian_activation::UnitKind,
        range: u32,
    ) -> Result<bool, Error<<E as creation::Effects>::Error>> {
        use crate::barbarian_activation::UnitKind;
        let index = match kind {
            UnitKind::Melee => 0,
            UnitKind::Ranged => 1,
            UnitKind::Siege => 2,
            UnitKind::Support => 3,
        };
        let Some(tag) = self
            .definitions
            .tribe(self.factory.state, instance)?
            .military[index]
            .clone()
        else {
            return Ok(false);
        };
        let units = self.create_tag(instance, location, &tag, range, 1)?;
        if index < 2 {
            let camp = self
                .factory
                .state
                .barbarian_manager_mut()
                .map_err(Error::State)?
                .instances
                .get_mut(instance)
                .ok_or(Error::Missing("camp instance"))?;
            camp.num_standard_military_units = camp
                .num_standard_military_units
                .wrapping_add(units.len() as i32);
        }
        Ok(!units.is_empty())
    }
    pub fn spawn_initial(
        &mut self,
        instance: usize,
        location: PlotCoord,
        scouts: i32,
    ) -> Result<(), Error<<E as creation::Effects>::Error>> {
        let tribe = self.definitions.tribe(self.factory.state, instance)?;
        let hash = tribe.hash;
        let tag = tribe.defender.clone();
        let units = self.create_tag(instance, location, &tag, 0, 1)?;
        for unit in units {
            self.factory
                .state
                .unit_mut(i32::from(unit.owner), unit.id)
                .map_err(Error::State)?
                .e_ai_type = 4;
        }
        self.create_scouts(instance, location, 3, scouts)?;
        let extra: Vec<_> = self
            .definitions
            .extra
            .iter()
            .filter(|e| e.0 == hash)
            .copied()
            .collect();
        for (_, unit, count) in extra {
            self.create_units(instance, location, unit, 3, count)?;
        }
        Ok(())
    }
}

/// Retained immutable factory inputs. Each callback borrows the caller's SAME
/// state, sight and effects only for that call; the outer camp loop holds no
/// mutable state alias and no native endpoint initializer.
pub struct FactoryInputs<'a, 'd, 'r> {
    pub definitions: &'a Definitions,
    pub creation: &'a creation::Definitions,
    pub resources: &'a crate::resource_binding::ResourceAuthority<'r>,
    pub movement: &'a crate::unit_movement_state::Inputs<'d>,
    pub combat: &'a crate::formation_rules::Definitions,
    pub costs: &'a crate::research_unit_costs::Definitions,
    pub placement: &'a crate::unit_placement_state::Definitions<'d, 'r>,
    pub queries: &'a crate::unit_placement_state::Queries<'d>,
    pub runtime: &'a creation::Runtime,
    pub tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub build_maps: &'a creation::signals::BuildMaps,
    pub clans_active: bool,
}
impl FactoryInputs<'_, '_, '_> {
    fn with_context<R, E: creation::FactoryEffects>(
        &self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        effects: &mut E,
        apply: impl FnOnce(
            &mut Context<'_, '_, '_, '_, E>,
        ) -> Result<R, Error<<E as creation::Effects>::Error>>,
    ) -> Result<R, Error<<E as creation::Effects>::Error>> {
        let mut factory = creation::Context {
            state,
            definitions: self.creation,
            resources: self.resources,
            movement: self.movement,
            combat: self.combat,
            costs: self.costs,
            placement: self.placement,
            queries: self.queries,
            runtime: self.runtime,
            tags: self.tags,
            sight,
            effects,
        };
        apply(&mut Context {
            factory: &mut factory,
            definitions: self.definitions,
            build_maps: self.build_maps,
            clans_active: self.clans_active,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_initial<E: creation::FactoryEffects>(
        &self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        effects: &mut E,
        instance: usize,
        location: PlotCoord,
        scouts: i32,
    ) -> Result<(), Error<<E as creation::Effects>::Error>> {
        self.with_context(state, sight, effects, |context| {
            context.spawn_initial(instance, location, scouts)
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create_scouts<E: creation::FactoryEffects>(
        &self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        effects: &mut E,
        instance: usize,
        location: PlotCoord,
        range: u32,
        count: i32,
    ) -> Result<bool, Error<<E as creation::Effects>::Error>> {
        self.with_context(state, sight, effects, |context| {
            context.create_scouts(instance, location, range, count)
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create_military<E: creation::FactoryEffects>(
        &self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        effects: &mut E,
        instance: usize,
        location: PlotCoord,
        kind: crate::barbarian_activation::UnitKind,
        range: u32,
    ) -> Result<bool, Error<<E as creation::Effects>::Error>> {
        self.with_context(state, sight, effects, |context| {
            context.create_military(instance, location, kind, range)
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create_tag<E: creation::FactoryEffects>(
        &self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        effects: &mut E,
        instance: usize,
        location: PlotCoord,
        tag: &[u8],
        range: u32,
        count: i32,
    ) -> Result<Vec<ComponentId>, Error<<E as creation::Effects>::Error>> {
        self.with_context(state, sight, effects, |context| {
            context.create_tag(instance, location, tag, range, count)
        })
    }
}
