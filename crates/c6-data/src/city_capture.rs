//! Linux combat capture's bounded noncapital City destroy/reconstruction path.
//! The same loaded state, sight, territory and stock creation owners survive
//! every phase. Mandatory continuations have no successful default.
pub mod callbacks;
pub mod cultural_identity;
pub mod emergency;
pub mod retired_district;
use crate::loaded_actions::stock_notifications::founding;
use c6_core::{
    city_capture::{self as core, Snapshot},
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Admission,
    PreConquered,
    PreTransfer,
    Destroy,
    Create,
    Damage,
    Territory,
    Buildings,
    Population,
    Culture,
    Religion,
    Monopolies,
    Properties,
    Conquered,
    Occupation,
    Decision,
}
#[derive(Debug)]
pub struct Error {
    pub stage: Stage,
    /// Fresh identity once creation succeeds; never the old city relabeled.
    pub replacement: Option<ComponentId>,
    pub detail: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "city capture {:?} at {:?}: {}",
            self.stage, self.replacement, self.detail
        )
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    Keep,
    Raze,
    Return,
    Liberate,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingDisposition {
    pub city: ComponentId,
    pub district: ComponentId,
    pub previous_owner: i32,
    pub legal: Vec<Disposition>,
}

/// Domain callbacks and independently implemented destruction compose here;
/// construction, state copy, ordering and pending decisions belong to capture.
/// Implementations must mutate the ongoing owners rather than importing native
/// post-state or accepting an unsupported callback as a successful empty call.
pub trait Effects<'a>: founding::Effects<'a> + crate::melee_attack::city::resolve::Effects {
    fn pre_city_conquered(
        &mut self,
        state: &mut LoadedState,
        old: ComponentId,
        new_owner: i32,
        attacker: UnitReference,
    ) -> Result<(), String>;
    fn pre_city_transferred(
        &mut self,
        state: &mut LoadedState,
        old: ComponentId,
        new_owner: i32,
        transfer: u32,
    ) -> Result<(), String>;
    fn destroy_for_transfer(
        &mut self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        territory: &mut crate::city_territory::Runtime,
        old: ComponentId,
    ) -> Result<(), String>;
    fn cultural_identity_transferred(
        &mut self,
        state: &mut LoadedState,
        territory: &crate::city_territory::Runtime,
        city: ComponentId,
        old_owner: i32,
    ) -> Result<(), String>;
    fn transfer_property(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        property: &c6_core::unit::variants::TypedEntry,
    ) -> Result<(), String>;
    /// Reporting event e05ec908 with keyed values City23739f42,
    /// previous-owner5150e7c3 and transfer-type2be74a24. This is not the
    /// distinct two-argument GameSignals.CityCeded ring at+3d8.
    fn report_city_transferred(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        old_owner: i32,
        transfer: u32,
    ) -> Result<(), String>;
    fn city_conquered(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        old_owner: i32,
    ) -> Result<(), String>;
    fn upgrade_conquering_formation(
        &mut self,
        state: &mut LoadedState,
        attacker: UnitReference,
    ) -> Result<(), String>;
}

pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    damage_percentage: i32,
    population_loss: f32,
    force_keep_cities: bool,
    raze_any_city: bool,
    units: crate::registry::LinuxDefinitionRegistry,
    spies: Vec<bool>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &crate::Rules) -> Result<Self, String> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err("Linux capture rules profile".into());
        }
        let table = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or("capture parameters")?;
        let parameter = |name| -> Result<&crate::SqlValue, String> {
            let mut rows = table.rows().filter(|r| r.text("Name").ok() == Some(name));
            let row = rows.next().ok_or_else(|| format!("missing {name}"))?;
            if rows.next().is_some() {
                return Err(format!("duplicate {name}"));
            }
            row.get("Value").map_err(|e| format!("{e:?}"))
        };
        let damage_percentage =
            crate::game_parameters::sql_integer(parameter("CITY_CAPTURED_DAMAGE_PERCENTAGE")?)
                .ok_or("capture damage percentage")?;
        let population_loss = match parameter("CITY_POPULATION_LOSS_TO_CONQUEST_PERCENTAGE")? {
            crate::SqlValue::Integer(v) => *v as f32,
            crate::SqlValue::RealBits(v) => f64::from_bits(*v) as f32,
            crate::SqlValue::Text(v) => std::str::from_utf8(v)
                .map_err(|e| format!("{e:?}"))?
                .parse::<f32>()
                .map_err(|e| format!("{e:?}"))?,
            _ => return Err("capture population float".into()),
        };
        let force_keep_cities =
            crate::game_parameters::sql_integer(parameter("COMBAT_KEEP_ALL_CITIES_ON_CAPTURE")?)
                .ok_or("capture disposition parameter")?
                != 0;
        let raze_any_city = crate::game_parameters::sql_integer(parameter("COMBAT_RAZE_ANY_CITY")?)
            .ok_or("raze-any-city parameter")?
            != 0;
        let units = crate::unit_initialization_definitions::collection(rules, "Units", "UnitType")
            .map_err(|e| format!("{e:?}"))?;
        let unit_table = rules.gameplay().table("Units").ok_or("capture Units")?;
        let spies = units
            .entries()
            .iter()
            .map(|entry| {
                unit_table
                    .row_by_id(entry.database_rowid)
                    .and_then(|row| row.get("Spy").ok())
                    .and_then(crate::game_parameters::sql_integer)
                    .map(|v| v != 0)
                    .ok_or_else(|| "capture Spy definition".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            damage_percentage,
            population_loss,
            force_keep_cities,
            raze_any_city,
            units,
            spies,
        })
    }
}

pub struct Context<'s, 'a, E> {
    pub state: &'s mut LoadedState,
    pub effects: &'s mut E,
    pub founding: &'s founding::Definitions<'a>,
    pub current: &'s founding::Current,
    pub sight: &'s mut crate::unit_sight_state::Runtime,
    pub territory: &'s mut crate::city_territory::Runtime,
    pub definitions: &'s Definitions,
    pub economic: &'s mut crate::barbarian_camps::services::Economic,
}
impl<'a, E: Effects<'a>> Context<'_, 'a, E> {
    fn error(
        stage: Stage,
        replacement: Option<ComponentId>,
        detail: impl std::fmt::Debug,
    ) -> Error {
        Error {
            stage,
            replacement,
            detail: format!("{detail:?}"),
        }
    }
    /// Validate only branches absent in the first ordinary capture profile.
    /// The observations are read from current typed owners, never defaulted.
    fn snapshot(&self, city: ComponentId, new_owner: i32) -> Result<Snapshot, Error> {
        let fail = |detail| Self::error(Stage::Admission, None, detail);
        let value = self
            .state
            .city(i32::from(city.owner), city.id)
            .map_err(|e| fail(format!("{e:?}")))?;
        let player = |owner| {
            self.state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
        };
        let old = player(i32::from(city.owner)).ok_or_else(|| fail("old player".into()))?;
        let new = player(new_owner).ok_or_else(|| fail("new player".into()))?;
        let major = crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
        if !self
            .founding
            .levels
            .iter()
            .any(|v| v.0 == new_owner && v.1 == major)
            || !self
                .founding
                .levels
                .iter()
                .any(|v| v.0 == i32::from(city.owner) && v.1 == major)
        {
            return Err(fail("non-major capture".into()));
        }
        if value.header.capital
            || old.cities.records.entries.len() <= 1
            || new.cities.records.entries.is_empty()
        {
            return Err(fail("capital/final-city/first-city capture".into()));
        }
        if value.header.original_owner != i32::from(city.owner)
            || value.header.owner_before_occupation != i32::from(city.owner)
            || value.header.occupied
            || self.definitions.force_keep_cities
        {
            return Err(fail("recapture/occupation/forced-keep capture".into()));
        }
        if value.districts.district_ids.len() != 1 {
            return Err(fail("additional captured districts".into()));
        }
        if self
            .state
            .retained()
            .players
            .iter()
            .any(|p| !p.governors.entries.is_empty())
        {
            return Err(fail("assigned governor transfer".into()));
        }
        if value
            .buildings
            .building_locations
            .iter()
            .any(|v| v.value != -1)
        {
            return Err(fail("captured buildings conversion/removal".into()));
        }
        let snapshot = Snapshot::capture(self.state, city).map_err(|e| fail(format!("{e:?}")))?;
        for &index in &snapshot.purchased_plots {
            let plot = self
                .state
                .world()
                .map()
                .plots
                .get(index as usize)
                .ok_or_else(|| fail("purchased plot".into()))?;
            if plot.improvement_hash != u32::MAX
                || plot
                    .city_attributes
                    .as_ref()
                    .is_some_and(|a| a.wonder_building_hash != u32::MAX)
            {
                return Err(fail("captured improvement/wonder".into()));
            }
        }
        let (center, _) =
            c6_core::movement_world::plot_at(self.state.world().map(), snapshot.location)
                .map_err(|e| fail(format!("{e:?}")))?;
        if self
            .state
            .world()
            .map()
            .units
            .layers
            .iter()
            .flat_map(|l| &l.cells)
            .any(|cell| cell.plot_index == center as u32 && !cell.units.is_empty())
        {
            return Err(fail("captured garrison units".into()));
        }
        // Transfer scans adjacent plots, then all UnitContainer layers. A
        // dominant-conquest unit changes the copied religion before creation;
        // this branch is explicit until its pressure continuation is joined.
        for cell in self
            .state
            .world()
            .map()
            .units
            .layers
            .iter()
            .flat_map(|l| &l.cells)
        {
            let (x, y) = self
                .state
                .world()
                .map()
                .coordinates(cell.plot_index)
                .ok_or_else(|| fail("adjacent unit plot".into()))?;
            if !c6_core::movement_world::adjacent(
                self.state.world().map(),
                snapshot.location,
                PlotCoord { x, y },
            )
            .map_err(|e| fail(format!("{e:?}")))?
            {
                continue;
            }
            for &unit in &cell.units {
                let component = unit.component_id();
                if component.owner == -1
                    || component.component_type == -1
                    || component.id == u32::MAX
                {
                    continue;
                }
                let value = crate::melee_attack::actor(self.state, unit)
                    .map_err(|e| fail(format!("{e:?}")))?;
                if value.b_dominant_conquer {
                    return Err(fail("dominant-conquest religion pressure".into()));
                }
            }
        }
        Ok(snapshot)
    }
}

impl<'a, E: Effects<'a>> Context<'_, 'a, E> {
    /// Execute the admitted empty-garrison, no-wall, noncapital major capture.
    /// Any error retains the exact applied prefix and the fresh identity, if
    /// allocated. The caller must keep its enclosing action incomplete.
    pub fn capture(
        &mut self,
        city: ComponentId,
        defeated: ComponentId,
        attacker: UnitReference,
        new_owner: i32,
    ) -> Result<PendingDisposition, Error> {
        let mut replacement = None;
        macro_rules! step {
            ($stage:ident, $value:expr) => {
                $value.map_err(|e| Self::error(Stage::$stage, replacement, e))?
            };
        }
        if self.state.binding() != &self.definitions.binding
            || attacker.player() != new_owner
            || new_owner == i32::from(city.owner)
            || attacker.component_id().component_type != 1
        {
            return Err(Self::error(
                Stage::Admission,
                None,
                "capture input identity",
            ));
        }
        let _ = step!(Admission, crate::melee_attack::actor(self.state, attacker));
        let admission = self.snapshot(city, new_owner)?;
        let old_owner = i32::from(city.owner);
        let (was_occupied, before_occupation) = {
            let value = step!(Admission, self.state.city(old_owner, city.id));
            (value.header.occupied, value.header.owner_before_occupation)
        };
        let district = step!(
            Admission,
            self.state.district(i32::from(defeated.owner), defeated.id)
        );
        if defeated.component_type != 3
            || defeated.owner != city.owner
            || district.city_id != city.id
            || district.district_hash != c6_core::city_district::CITY_CENTER_HASH
            || step!(Admission, self.state.city(old_owner, city.id))
                .districts
                .district_ids
                != [defeated.id]
        {
            return Err(Self::error(
                Stage::Admission,
                None,
                "defeated CityCenter identity",
            ));
        }
        if step!(
            Admission,
            crate::loaded_actions::city_range_attack::maximum_damage(
                self.state,
                district,
                self.founding.sight,
                self.founding.levels,
                true
            )
        ) != 0
        {
            return Err(Self::error(
                Stage::Admission,
                None,
                "captured outer defenses",
            ));
        }
        drop(admission);
        // Empty garrisons and absent buildings were established from the live
        // records above; their native capture-removal loops have no iterations.
        step!(
            PreConquered,
            self.effects
                .pre_city_conquered(self.state, city, new_owner, attacker)
        );
        step!(
            PreTransfer,
            self.effects
                .pre_city_transferred(self.state, city, new_owner, core::TRANSFER_COMBAT)
        );
        // Transfer snapshots AFTER synchronous pre-transfer listeners.
        let snapshot = self.snapshot(city, new_owner).map_err(|mut error| {
            error.stage = Stage::PreTransfer;
            error
        })?;
        step!(
            Destroy,
            self.effects
                .destroy_for_transfer(self.state, self.sight, self.territory, city)
        );
        step!(
            Destroy,
            check_occupying_major_cities(self.state, old_owner, new_owner, self.founding.levels)
        );
        let fresh = step!(
            Create,
            founding::create_transferred(
                self.state,
                (attacker.player(), attacker.id),
                self.effects,
                self.founding,
                self.current,
                self.sight,
                self.territory,
                &snapshot,
                new_owner,
            )
        )
        .ok_or_else(|| Self::error(Stage::Create, None, "native City creation failed"))?;
        replacement = Some(fresh);
        let center_id = *step!(Damage, self.state.city(new_owner, fresh.id))
            .districts
            .district_ids
            .first()
            .ok_or_else(|| Self::error(Stage::Damage, replacement, "new CityCenter"))?;
        let center = ComponentId {
            owner: fresh.owner,
            component_type: 3,
            id: center_id,
        };
        let maximum = step!(
            Damage,
            crate::loaded_actions::city_range_attack::maximum_damage(
                self.state,
                step!(Damage, self.state.district(new_owner, center_id)),
                self.founding.sight,
                self.founding.levels,
                false
            )
        );
        step!(
            Damage,
            set_captured_center_damage(
                self.state,
                self.effects,
                center,
                maximum,
                self.definitions.damage_percentage
            )
        );
        for &plot in &snapshot.purchased_plots {
            let (x, y) = self.state.world().map().coordinates(plot).ok_or_else(|| {
                Self::error(Stage::Territory, replacement, "purchased plot location")
            })?;
            let id = c6_core::city_territory::CityId {
                owner: new_owner,
                id: fresh.id,
            };
            let mut territory = crate::city_territory::Context {
                state: self.state,
                runtime: self.territory,
                sight: self.sight,
                sight_definitions: self.founding.sight,
                definitions: self.founding.territory,
                effects: self.effects,
                city: id,
                actor: (attacker.player(), attacker.id),
            };
            use c6_core::city_territory::{AnnexContext, OwnerContext, PurchasedContext};
            if step!(Territory, territory.owning_city(plot)) != id {
                step!(
                    Territory,
                    c6_core::city_territory::set_owner(&mut territory, plot, id.owner, id.id)
                );
                step!(
                    Territory,
                    territory.set_working_city(PlotCoord { x, y }, id)
                );
                let added = step!(Territory, territory.add_plot_yields(plot, id, -1));
                let old = step!(Territory, territory.plot_yield_effect(plot));
                if old != 0 {
                    step!(Territory, territory.remove_plot_yields(old));
                }
                step!(Territory, territory.set_plot_yield_effect(plot, added));
                step!(Territory, territory.set_valid_domains(plot));
            }
        }
        step!(Buildings, transfer_reporting_block(self.effects, true));
        core::copy_buildings(
            &mut step!(Buildings, self.state.city_mut(new_owner, fresh.id)).buildings,
            snapshot.buildings,
        );
        let percent = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == new_owner)
            .ok_or_else(|| Self::error(Stage::Population, replacement, "new player cities"))?
            .cities
            .population_after_conquest;
        let population = core::population_after_conquest(
            snapshot.population,
            percent,
            self.definitions.population_loss,
        );
        let current_population = step!(Population, self.state.city(new_owner, fresh.id))
            .header
            .population;
        step!(
            Population,
            crate::city_turn::change_population(
                self.state,
                new_owner,
                fresh.id,
                population.wrapping_sub(current_population),
                self.founding.growth,
                self.effects
            )
        );
        step!(
            Culture,
            self.effects.cultural_identity_transferred(
                self.state,
                self.territory,
                fresh,
                old_owner
            )
        );
        let previous_majority = snapshot.religion.majority_religion_hash;
        step!(Religion, self.state.city_mut(new_owner, fresh.id)).religion = snapshot.religion;
        step!(
            Religion,
            crate::city_turn::religion::recompute(
                self.state,
                new_owner,
                fresh.id,
                previous_majority,
                new_owner,
                self.founding.growth,
                self.effects
            )
        );
        let alive_majors = self.territory.alive_major_ids.as_deref().ok_or_else(|| {
            Self::error(Stage::Religion, replacement, "current alive major order")
        })?;
        step!(
            Religion,
            no_spies_to_displace(self.state, fresh, self.definitions, alive_majors)
        );
        if step!(Religion, self.state.city(new_owner, fresh.id))
            .header
            .city_ai_id
            != u32::MAX
        {
            let (_, minimum, additional) = *self
                .founding
                .strategic
                .iter()
                .find(|r| r.0 == new_owner)
                .ok_or_else(|| {
                    Self::error(
                        Stage::Religion,
                        replacement,
                        "current CityBuild strategic parameters",
                    )
                })?;
            let parameters = c6_core::city_build_initialization::founding::Parameters::bind(
                self.state, new_owner, minimum, additional,
            );
            step!(
                Religion,
                c6_core::city_build_initialization::founding::on_found_city_external_policy(
                    self.state,
                    new_owner,
                    fresh.id,
                    &parameters
                )
            );
        }
        step!(
            Monopolies,
            recalculate_monopolies_unchanged(
                self.economic,
                self.territory
                    .alive_major_ids
                    .as_deref()
                    .ok_or_else(|| Self::error(
                        Stage::Monopolies,
                        replacement,
                        "current alive major order"
                    ))?
            )
        );
        if let Some(property) = snapshot
            .properties
            .entries
            .iter()
            .find(|entry| entry.key == 0x64a7_7ab9)
        {
            step!(
                Properties,
                self.effects.transfer_property(self.state, fresh, property)
            );
        }
        step!(Properties, transfer_reporting_block(self.effects, false));
        step!(
            Properties,
            self.effects.report_city_transferred(
                self.state,
                fresh,
                old_owner,
                core::TRANSFER_COMBAT
            )
        );
        step!(
            Conquered,
            self.effects.city_conquered(self.state, fresh, old_owner)
        );
        let upgrade = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == new_owner)
            .ok_or_else(|| Self::error(Stage::Conquered, replacement, "conquering player"))?
            .units
            .upgrade_military_formation_on_city_conquest_count;
        if upgrade > 0 {
            step!(
                Conquered,
                self.effects
                    .upgrade_conquering_formation(self.state, attacker)
            );
        }
        step!(Occupation, self.state.city_mut(new_owner, fresh.id))
            .header
            .owner_before_occupation = before_occupation;
        let legal = step!(
            Decision,
            legal_dispositions(self.state, fresh, self.definitions, self.founding.levels)
        );
        // Native policy selection is replaced by an external semantic decision;
        // reached ownership/diplomacy writes are still executed in source order.
        step!(Occupation, self.state.city_mut(new_owner, fresh.id))
            .header
            .just_conquered_from = old_owner;
        if !was_occupied {
            step!(Occupation, self.state.city_mut(new_owner, fresh.id))
                .header
                .owner_before_occupation = old_owner;
        } else if before_occupation == new_owner {
            step!(Occupation, self.state.city_mut(new_owner, fresh.id))
                .header
                .occupied = false;
        }
        step!(
            Occupation,
            check_occupying_major_cities(
                self.state,
                new_owner,
                before_occupation,
                self.founding.levels
            )
        );
        if legal.is_empty() {
            return Err(Self::error(
                Stage::Decision,
                replacement,
                "missing legal captured-city decision",
            ));
        }
        Ok(PendingDisposition {
            city: fresh,
            district: center,
            previous_owner: old_owner,
            legal,
        })
    }
}

/// Player.Diplomacy.CheckIfOccupyingCities2aac240 for the major-player branch.
/// IsOccupied(true) reads the stored flag and ignores NoOccupationPenalties.
/// This writes the real canonical 64-player array even when the value repeats.
pub fn check_occupying_major_cities(
    state: &mut LoadedState,
    owner: i32,
    other: i32,
    levels: &[(i32, u32)],
) -> Result<(), String> {
    if other == -1 {
        return Ok(());
    }
    state.world().player(other).map_err(|e| format!("{e:?}"))?;
    if levels.iter().find(|r| r.0 == other).map(|r| r.1)
        != Some(crate::registry::linux_type_hash(
            b"CIVILIZATION_LEVEL_FULL_CIV",
        ))
    {
        return Err("occupation check non-major branch".into());
    }
    let at = usize::try_from(other)
        .ok()
        .filter(|&i| i < 64)
        .ok_or("occupation target")?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or("occupation owner")?;
    let occupied = player
        .cities
        .records
        .entries
        .iter()
        .any(|r| r.value.header.occupied && r.value.header.owner_before_occupation == other);
    state
        .war_diplomacy_mut(owner)
        .map_err(|e| format!("{e:?}"))?
        .occupies_original_city[at] = occupied;
    Ok(())
}

/// DisplaceSpiesOnCityCapture2be9f80 traverses the current alive-major order,
/// then each canonical Unit collection. The no-matching-spy branch has no
/// operation, reset, movement, notification or RNG effect. A matching spy
/// remains an explicit unsupported continuation rather than staying in place.
pub fn no_spies_to_displace(
    state: &LoadedState,
    city: ComponentId,
    definitions: &Definitions,
    alive_majors: &[i32],
) -> Result<(), String> {
    if state.binding() != &definitions.binding {
        return Err("capture Spy binding".into());
    }
    state
        .city(i32::from(city.owner), city.id)
        .map_err(|e| format!("{e:?}"))?;
    for &owner in alive_majors {
        let player = state.world().player(owner).map_err(|e| format!("{e:?}"))?;
        for slot in player.unit_records().map_err(|e| format!("{e:?}"))? {
            let unit = &slot.value;
            if unit.b_delayed_death {
                continue;
            }
            let Some(index) = definitions.units.find_index(unit.e_type_index) else {
                continue;
            };
            if !definitions.spies[index] {
                continue;
            }
            let (_, plot) =
                match c6_core::movement_world::plot_at(state.world().map(), unit.k_location) {
                    Ok(value) => value,
                    Err(c6_core::movement_world::MissingInput::PlotOutsideMap) => continue,
                    Err(error) => return Err(format!("{error:?}")),
                };
            if plot.city_attributes.as_ref().is_some_and(|a| {
                i32::from(a.player) == i32::from(city.owner) && a.owning_city == city.id
            }) {
                return Err("captured-city spy operation reset/displacement".into());
            }
        }
    }
    Ok(())
}

/// Reuse the current Economic manager projection shared by supported Farm and
/// camp actions. Nonpositive wrapped Q8 totals make every HasMonopoly query
/// false; no summary owned by an alive major means the source loop changes
/// no fields and dispatches no monopoly callback or random draw.
pub fn recalculate_monopolies_unchanged(
    economic: &crate::barbarian_camps::services::Economic,
    alive_majors: &[i32],
) -> Result<(), String> {
    if economic.resource_count != economic.totals.len()
        || economic.resource_count != economic.summaries.len()
        || economic.owned.len() != 64
        || economic
            .owned
            .iter()
            .any(|row| row.len() != economic.resource_count)
    {
        return Err("capture Economic resource array dimensions".into());
    }
    if economic.totals.iter().any(|v| v.wrapping_shl(8) > 0)
        || economic
            .summaries
            .iter()
            .any(|(owner, _)| alive_majors.contains(owner))
    {
        return Err("positive capture monopoly update".into());
    }
    Ok(())
}

impl Disposition {
    /// Actual City.Commands.Destroy directive enum values.
    pub const fn native_directive(self) -> i32 {
        match self {
            Self::Liberate => 0,
            Self::Return => 1,
            Self::Keep => 2,
            Self::Raze => 3,
        }
    }
}

/// Rules.Players.CanRaze2bf1820; alliance queries use canonical current
/// Game.DiplomaticRelations and never a policy or a captured boolean answer.
pub fn can_raze(
    state: &LoadedState,
    city: ComponentId,
    definitions: &Definitions,
    levels: &[(i32, u32)],
) -> Result<bool, String> {
    if state.binding() != &definitions.binding {
        return Err("capture raze binding".into());
    }
    if definitions.raze_any_city {
        return Ok(true);
    }
    let city = state
        .city(i32::from(city.owner), city.id)
        .map_err(|e| format!("{e:?}"))?;
    let original = city.header.original_owner;
    let previous = city.header.owner_before_occupation;
    let owner = city.header.owner;
    if city.header.original_capital {
        let major = crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
        let level = levels
            .iter()
            .find(|r| r.0 == original)
            .ok_or("original-owner civilization level")?
            .1;
        if level == major {
            return Ok(false);
        }
    }
    if original == owner || previous == owner {
        return Ok(false);
    }
    let relations = state.game_relations().map_err(|e| format!("{e:?}"))?;
    if relations
        .is_allied(owner, original)
        .map_err(|e| format!("{e:?}"))?
    {
        return Ok(false);
    }
    Ok(!relations
        .is_allied(owner, previous)
        .map_err(|e| format!("{e:?}"))?)
}

/// Capture028d8380's source option count is Keep + previous-owner Return
/// when different + original-owner Liberate when different + CanRaze. Destroy
/// CanStart2819cc0 itself returns true; these native directives are distinct
/// even when Return and Liberate currently target the same player.
pub fn legal_dispositions(
    state: &LoadedState,
    identity: ComponentId,
    definitions: &Definitions,
    levels: &[(i32, u32)],
) -> Result<Vec<Disposition>, String> {
    let city = state
        .city(i32::from(identity.owner), identity.id)
        .map_err(|e| format!("{e:?}"))?;
    if definitions.force_keep_cities {
        return Ok(vec![Disposition::Keep]);
    }
    let mut legal = Vec::with_capacity(4);
    if city.header.original_owner != city.header.owner {
        legal.push(Disposition::Liberate);
    }
    if city.header.owner_before_occupation != city.header.owner {
        legal.push(Disposition::Return);
    }
    legal.push(Disposition::Keep);
    if can_raze(state, identity, definitions, levels)? {
        legal.push(Disposition::Raze);
    }
    Ok(legal)
}

/// Core reporting uses the same live block stack as visibility. The stock
/// Linux host Translate ignores this hash (its only active block is5a9a0329).
/// EndBlock pops the current tail without searching for a matching hash.
pub fn transfer_reporting_block<E: crate::unit_sight_state::Effects<Error = String>>(
    effects: &mut E,
    begin: bool,
) -> Result<(), String> {
    let blocks = effects.visibility_reporting_blocks()?;
    if begin {
        blocks
            .try_reserve(1)
            .map_err(|_| "capture reporting stack allocation")?;
        blocks.push(0xe05e_c908);
    } else {
        blocks.pop();
    }
    Ok(())
}

/// Transfer calls District.SetDamage291f790: clamp the wrapping percentage
/// result, store it, and synchronously emit DistrictDamageChanged on a change.
/// It does not execute ChangeDamage's LastTurnDamaged write.
pub fn set_captured_center_damage<E: crate::melee_attack::city::resolve::Effects>(
    state: &mut LoadedState,
    effects: &mut E,
    identity: ComponentId,
    maximum: i32,
    percentage: i32,
) -> Result<(), String> {
    if maximum < 0 {
        return Err("capture District maximum damage".into());
    }
    let owner = i32::from(identity.owner);
    let previous = state
        .district(owner, identity.id)
        .map_err(|e| format!("{e:?}"))?
        .garrison_damage;
    let current = core::captured_center_damage(maximum, percentage).clamp(0, maximum);
    state
        .district_mut(owner, identity.id)
        .map_err(|e| format!("{e:?}"))?
        .garrison_damage = current;
    if current != previous {
        effects.city_melee_publication(
            state,
            crate::melee_attack::city::Publication::DistrictDamageChanged {
                district: identity,
                defense_type: 0x5e97_d629,
                current,
                previous,
            },
        )?;
    }
    Ok(())
}
