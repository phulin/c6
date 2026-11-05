//! Stock FoundCity handler and placed-City creation over the ongoing owners.
//! The canonical constructor, placement, naming and capital/Palace sequence run
//! here. Required domain effects borrow the SAME stock runtime; no world or
//! modifier registry is constructed for an action.
use super::Runtime;
use crate::{
    city_production_turn::{buildings, modifier_effects},
    found_city,
    loaded_actions::move_to,
};
use c6_core::{
    loaded_state::LoadedState,
    skip_turn::StartData,
    unit::{ComponentId, PlotCoord, Unit},
};

/// Cached current definitions and actual initialization/configuration operands.
/// All references are borrowed; mutable territory and gameplay owners stay with
/// the caller and are loaned to the handler below.
pub struct Definitions<'a> {
    pub founding: &'a found_city::Definitions,
    pub rules: &'a crate::Rules,
    pub cities: &'a crate::city_initialization::Definitions,
    pub names: &'a found_city::names::Definitions,
    pub territory: &'a crate::city_territory::Definitions,
    pub sight: &'a crate::loaded_actions::city_range_attack::Definitions,
    pub districts: &'a crate::district_initialization::Definitions,
    pub appeal: &'a crate::city_production_turn::appeal::Definitions,
    pub phases: &'a crate::unit_operation_manager::PhaseDefinitions,
    pub levels: &'a [(i32, u32)],
    pub routes: &'a found_city::routes::Definitions,
    pub growth: &'a crate::city_turn::GrowthRules,
    pub district_costs: &'a crate::research::districts::Definitions,
    pub civics: &'a crate::research::CivicGraph,
    pub strategic: &'a [(i32, i32, i32)],
    pub buildings: &'a buildings::Definitions,
}
pub struct Naming {
    pub owner: i32,
    pub civilization: Vec<u8>,
    pub leader: Vec<u8>,
}
pub struct Current {
    pub local_observer: i32,
    pub naming: Vec<Naming>,
    pub city_build_managers: Vec<(i32, bool)>,
    pub settled_on_continent_owners: Vec<i32>,
    pub free_cities_level: u32,
}

/// Actual effect leaves reached by the supported creation path. These services
/// retain their reports, script, sight and modifier state between requests.
/// Added consumers and positive unsupported branches must return an error.
pub trait Effects<'a>: found_city::placement::Effects + buildings::Effects {
    fn stock(&mut self) -> &mut Runtime<'a>;
    fn founding_report(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::found_city::Report,
    ) -> Result<(), String>;
    fn remove_feature(&mut self, state: &mut LoadedState, actor: (i32, u32)) -> Result<(), String>;
    fn majority_religion(&mut self, state: &LoadedState, owner: i32) -> Result<i32, String>;
    fn original_capital_allowed(&mut self, state: &LoadedState, owner: i32)
        -> Result<bool, String>;
    fn religion_auto_spreads(
        &mut self,
        state: &LoadedState,
        religion: i32,
    ) -> Result<Option<bool>, String>;
    fn pressure_needed_for_majority(
        &mut self,
        state: &LoadedState,
        city: ComponentId,
        religion: i32,
    ) -> Result<i32, String>;
    fn founding_pressure(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        religion: i32,
        amount: i32,
        reason: i32,
        source: i32,
    ) -> Result<(), String>;
    fn observer_sees_location(
        &mut self,
        state: &LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        actor: (i32, u32),
    ) -> Result<bool, String>;
    fn consume_settler(
        &mut self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        actor: (i32, u32),
    ) -> Result<(), String>;
    fn subscribe_city_signals(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), String>;
    fn generate_later_name(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        location: PlotCoord,
    ) -> Result<Vec<u8>, String>;
    fn renamed_signal(&mut self, state: &mut LoadedState, city: ComponentId) -> Result<(), String>;
    fn city_report(
        &mut self,
        state: &mut LoadedState,
        event: u32,
        city: ComponentId,
    ) -> Result<(), String>;
    fn destroy_failed_placement(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), String>;
    /// Dispatch the actual ordered CityCreated/CityAdded+288 ring, including
    /// its initial listeners and newly registered collection listeners at the
    /// appended positions. The parts helper dispatches that new-listener tail.
    fn city_created(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        initializing: bool,
    ) -> Result<(), String>;
    fn foreign_continent_awards(
        &mut self,
        state: &mut LoadedState,
        receiver: i32,
        city: ComponentId,
        continent: u32,
    ) -> Result<(), String>;
    /// Existing source-qualified listeners, followed by newly registered policy
    /// collection routes at their actual appended CapitalChanged positions.
    fn capital_changed(&mut self, state: &mut LoadedState, city: ComponentId)
        -> Result<(), String>;
    fn cultural_identity_created(
        &mut self,
        state: &mut LoadedState,
        territory: &crate::city_territory::Runtime,
        city: ComponentId,
    ) -> Result<(), String>;
}
/// Loan domain services together with the canonical stock runtime, rather than
/// constructing an independent effects/backend owner for a founding action.
pub trait Domains<'a> {
    type Effects<'s>: Effects<'a>
    where
        Self: 's,
        'a: 's;
    fn borrow<'s>(
        &'s mut self,
        stock: &'s mut Runtime<'a>,
        owners: &'s mut super::unit_creation::ModifierOwners,
    ) -> Self::Effects<'s>
    where
        'a: 's;
}

/// Retain this adapter beside the existing movement provider across requests.
/// The fallback retains the existing scalar and movement query services.
pub struct Movement<'s, 'a, S, M> {
    pub definitions: &'s Definitions<'a>,
    pub current: &'s Current,
    pub domains: &'s mut S,
    pub territory: &'s mut crate::city_territory::Runtime,
    pub fallback: &'s mut M,
}

fn text(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
struct Start<'s, 'd, E> {
    state: &'s mut LoadedState,
    actor: (i32, u32),
    effects: &'s mut E,
    definitions: &'s Definitions<'d>,
    current: &'s Current,
    core: &'s c6_core::city_initialization::Definitions<'d>,
    sight: &'s mut crate::unit_sight_state::Runtime,
    territory: &'s mut crate::city_territory::Runtime,
}
impl<E> Start<'_, '_, E> {
    fn unit(&self) -> Result<&Unit, String> {
        self.state
            .world()
            .player(self.actor.0)
            .map_err(text)?
            .unit(self.actor.1)
            .map_err(text)?
            .map(|u| u.record)
            .ok_or_else(|| "current founding Unit missing".into())
    }
    fn location(&self) -> Result<PlotCoord, String> {
        Ok(self.unit()?.k_location)
    }
}
impl<'a, E: Effects<'a>> c6_core::found_city::StartContext for Start<'_, '_, E> {
    type Error = String;
    type Plot = usize;
    type City = ComponentId;
    fn unit_plot(&mut self) -> Result<usize, String> {
        let location = Start::location(self)?;
        c6_core::movement_world::plot_at(self.state.world().map(), location)
            .map(|p| p.0)
            .map_err(text)
    }
    fn feature_type(&mut self, plot: usize) -> Result<i32, String> {
        self.state
            .world()
            .map()
            .plots
            .get(plot)
            .map(|p| p.feature_hash as i32)
            .ok_or_else(|| "founding plot missing".into())
    }
    fn feature_definition(
        &mut self,
        kind: i32,
    ) -> Result<Option<c6_core::found_city::Feature>, String> {
        self.definitions
            .founding
            .feature_find(self.state, kind as u32)
            .map(|v| {
                v.map(|f| c6_core::found_city::Feature {
                    removable: f.removable,
                })
            })
            .map_err(text)
    }
    fn owner(&mut self) -> Result<i32, String> {
        Ok(self.unit()?.e_owner)
    }
    fn location(&mut self) -> Result<PlotCoord, String> {
        Start::location(self)
    }
    fn identity(&mut self) -> Result<ComponentId, String> {
        let u = self.unit()?;
        Ok(ComponentId {
            owner: u.e_owner as i16,
            component_type: 1,
            id: u.id,
        })
    }
    fn report(&mut self, report: c6_core::found_city::Report) -> Result<(), String> {
        self.effects.founding_report(self.state, report)
    }
    fn remove_feature_at_current_location(&mut self) -> Result<(), String> {
        self.effects.remove_feature(self.state, self.actor)
    }
    fn observer_can_see_player(&mut self, owner: i32) -> Result<bool, String> {
        Ok(self.current.local_observer == owner || self.current.local_observer == 1000)
    }
    fn operation_hash(&mut self) -> Result<u32, String> {
        self.unit()?
            .k_operations
            .nodes
            .first()
            .map(|n| n.operation_hash)
            .ok_or_else(|| "current founding operation missing".into())
    }
    fn majority_religion(&mut self, owner: i32) -> Result<i32, String> {
        let cities = &self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or("current Religion owner")?
            .cities
            .records
            .entries;
        if cities.is_empty() {
            Ok(-1)
        } else {
            self.effects.majority_religion(self.state, owner)
        }
    }
    fn city_count(&mut self, owner: i32) -> Result<i32, String> {
        Ok(self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or("current City owner")?
            .cities
            .records
            .entries
            .len() as i32)
    }
    fn can_set_original_capital(&mut self, owner: i32) -> Result<bool, String> {
        self.effects.original_capital_allowed(self.state, owner)
    }
    fn create_city(
        &mut self,
        creation: c6_core::found_city::Creation,
    ) -> Result<Option<ComponentId>, String> {
        let parameters = c6_core::city_initialization::Parameters {
            original_owner: creation.owner,
            transfer_type: creation.transfer_type as i32,
            original_capital: creation.original_capital,
            founded_turn: -1,
        };
        let core = self.core;
        c6_core::found_city::creation::create(self, core, parameters, creation, false).map_err(text)
    }
    fn religion_auto_spreads(&mut self, religion: i32) -> Result<Option<bool>, String> {
        self.effects.religion_auto_spreads(self.state, religion)
    }
    fn pressure_needed_for_majority(
        &mut self,
        city: ComponentId,
        religion: i32,
    ) -> Result<i32, String> {
        self.effects
            .pressure_needed_for_majority(self.state, city, religion)
    }
    fn add_religious_pressure(
        &mut self,
        city: ComponentId,
        religion: i32,
        amount: i32,
        reason: i32,
        source: i32,
    ) -> Result<(), String> {
        self.effects
            .founding_pressure(self.state, city, religion, amount, reason, source)
    }
    fn local_observer_sees_current_location(&mut self) -> Result<bool, String> {
        self.effects
            .observer_sees_location(self.state, self.sight, self.actor)
    }
    fn delayed_destroy_unit(&mut self) -> Result<(), String> {
        self.effects
            .consume_settler(self.state, self.sight, self.actor)
    }
}
impl<'a, E: Effects<'a>> c6_core::city_initialization::Context for Start<'_, '_, E> {
    type Error = String;
    fn state(&self) -> &LoadedState {
        self.state
    }
    fn state_mut(&mut self) -> &mut LoadedState {
        self.state
    }
    fn subscribe_city_signals(&mut self, city: ComponentId) -> Result<(), String> {
        self.effects.subscribe_city_signals(self.state, city)
    }
    fn initialize_city_build(&mut self, city: ComponentId) -> Result<Option<u32>, String> {
        let owner = i32::from(city.owner);
        let initialized = self
            .current
            .city_build_managers
            .iter()
            .find(|r| r.0 == owner)
            .ok_or("current CityManager initialization")?
            .1;
        let inputs =
            c6_core::city_build_initialization::Inputs::bind(self.state, owner, initialized);
        c6_core::city_build_initialization::create(self.state, owner, city.id, &inputs)
            .map_err(text)
    }
}
impl<'a, E: Effects<'a>> c6_core::found_city::creation::Context for Start<'_, '_, E> {
    fn generate_name(
        &mut self,
        city: ComponentId,
        first: bool,
        location: PlotCoord,
    ) -> Result<Vec<u8>, String> {
        if !first {
            return self.effects.generate_later_name(self.state, city, location);
        }
        let naming = self
            .current
            .naming
            .iter()
            .find(|n| n.owner == i32::from(city.owner))
            .ok_or("current founding civilization/leader")?;
        self.definitions
            .names
            .generate_first_capital(self.state, &naming.civilization, &naming.leader)
            .map_err(text)
    }
    fn renamed_signal(&mut self, city: ComponentId) -> Result<(), String> {
        self.effects.renamed_signal(self.state, city)
    }
    fn report(&mut self, event: u32, city: ComponentId) -> Result<(), String> {
        self.effects.city_report(self.state, event, city)
    }
    fn place(
        &mut self,
        city: ComponentId,
        location: PlotCoord,
        _initializing: bool,
    ) -> Result<i32, String> {
        let d = self.definitions;
        found_city::placement::Context {
            state: self.state,
            actor: self.actor,
            effects: self.effects,
            definitions: d.founding,
            rules: d.rules,
            visibility: self.sight,
            territory: self.territory,
            territory_definitions: d.territory,
            sight_definitions: d.sight,
            district_definitions: d.districts,
            appeal_definitions: d.appeal,
            phases: d.phases,
            levels: d.levels,
            route_definitions: d.routes,
            growth_rules: d.growth,
            district_cost_definitions: d.district_costs,
            civic_graph: d.civics,
            strategic_parameters: d.strategic,
        }
        .place(city, location)
    }
    fn destroy_failed_placement(&mut self, city: ComponentId) -> Result<(), String> {
        self.effects.destroy_failed_placement(self.state, city)
    }
    fn city_created(&mut self, city: ComponentId, initializing: bool) -> Result<(), String> {
        self.effects.city_created(self.state, city, initializing)
    }
    fn current_continent(&mut self, city: ComponentId) -> Result<u32, String> {
        let city = self
            .state
            .city(i32::from(city.owner), city.id)
            .map_err(text)?;
        let location = PlotCoord {
            x: city.header.location_x,
            y: city.header.location_y,
        };
        c6_core::movement_world::plot_at(self.state.world().map(), location)
            .map(|p| p.1.continent_hash)
            .map_err(text)
    }
    fn settled_on_continent(&mut self, city: ComponentId, continent: u32) -> Result<(), String> {
        for &receiver in &self.current.settled_on_continent_owners {
            let home = self
                .state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == receiver)
                .ok_or("current settlement receiver")?
                .cities
                .home_continent_hash;
            if c6_core::found_city::creation::foreign_continent_bonus_applies(
                home, receiver, city, continent,
            ) {
                self.effects
                    .foreign_continent_awards(self.state, receiver, city, continent)?;
            }
        }
        Ok(())
    }
    fn is_free_cities(&mut self, owner: i32) -> Result<bool, String> {
        Ok(self
            .definitions
            .levels
            .iter()
            .find(|r| r.0 == owner)
            .ok_or("current founding civilization level")?
            .1
            == self.current.free_cities_level)
    }
    fn capital_changed(&mut self, city: ComponentId) -> Result<(), String> {
        self.effects.capital_changed(self.state, city)
    }
    fn add_palace(&mut self, city: ComponentId) -> Result<(), String> {
        let definitions = self.definitions.buildings;
        let capitals = definitions
            .buildings
            .iter()
            .filter(|b| b.capital)
            .map(|b| b.hash)
            .collect::<Vec<_>>();
        for building in capitals {
            let current = self
                .state
                .city(i32::from(city.owner), city.id)
                .map_err(text)?;
            let location = [current.header.location_x, current.header.location_y];
            buildings::create_full(
                self.state,
                i32::from(city.owner),
                city.id,
                building,
                location,
                definitions,
                self.effects,
            )
            .map_err(text)?;
        }
        Ok(())
    }
    fn cultural_identity_created(&mut self, city: ComponentId) -> Result<(), String> {
        self.effects
            .cultural_identity_created(self.state, self.territory, city)
    }
    fn check_production(&mut self, city: ComponentId) -> Result<(), String> {
        self.effects
            .stock()
            .publish_city_production(self.state, i32::from(city.owner), city.id)
            .map_err(text)
    }
}
impl<'a, S: Domains<'a>, M> move_to::Movement<Runtime<'a>> for Movement<'_, 'a, S, M>
where
    M: move_to::Movement<Runtime<'a>>,
    M::Error: std::fmt::Debug,
{
    type Error = String;
    fn found_city_rules(&self) -> Option<(&found_city::Definitions, &crate::Rules)> {
        Some((self.definitions.founding, self.definitions.rules))
    }
    fn start_found_city(
        &mut self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        actor: (i32, u32),
        stock: &mut Runtime<'a>,
    ) -> Result<Option<StartData>, String> {
        stock.check(state).map_err(text)?;
        let core = self.definitions.cities.core(state).map_err(text)?;
        stock
            .with_modifier_owners(|stock, owners| {
                let mut effects = self.domains.borrow(stock, owners);
                let mut context = Start {
                    state,
                    actor,
                    effects: &mut effects,
                    definitions: self.definitions,
                    current: self.current,
                    core: &core,
                    sight,
                    territory: self.territory,
                };
                let mut data = StartData {
                    action: false,
                    delete: false,
                };
                c6_core::found_city::start(&mut context, &mut data)?;
                Ok(Some(data))
            })
            .map_err(text)?
    }
    fn fortify_turn_max(&self) -> i32 {
        self.fallback.fortify_turn_max()
    }
    fn is_fortifyable(&self, s: &LoadedState, a: (i32, u32)) -> Result<bool, String> {
        self.fallback.is_fortifyable(s, a).map_err(text)
    }
    fn alert_enemy_visible(
        &self,
        s: &LoadedState,
        v: &crate::unit_sight_state::Runtime,
        a: (i32, u32),
    ) -> Result<bool, String> {
        self.fallback.alert_enemy_visible(s, v, a).map_err(text)
    }
    fn city_at(
        &self,
        s: &LoadedState,
        p: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, String> {
        self.fallback.city_at(s, p).map_err(text)
    }
    fn preflight(&self, s: &LoadedState, a: (i32, u32), r: move_to::Request) -> Result<(), String> {
        self.fallback.preflight(s, a, r).map_err(text)
    }
    fn test_move_to(
        &mut self,
        s: &mut LoadedState,
        v: &crate::unit_sight_state::Runtime,
        a: (i32, u32),
        p: PlotCoord,
        f: u32,
    ) -> Result<i32, String> {
        self.fallback.test_move_to(s, v, a, p, f).map_err(text)
    }
    fn move_to(
        &mut self,
        s: &mut LoadedState,
        v: &mut crate::unit_sight_state::Runtime,
        a: (i32, u32),
        p: PlotCoord,
        f: u32,
        e: &mut Runtime<'a>,
    ) -> Result<i32, String> {
        self.fallback.move_to(s, v, a, p, f, e).map_err(text)
    }
    fn valid_location(
        &mut self,
        s: &LoadedState,
        v: &crate::unit_sight_state::Runtime,
        a: (i32, u32),
        p: PlotCoord,
    ) -> Result<i32, String> {
        self.fallback.valid_location(s, v, a, p).map_err(text)
    }
}

/// Dispatch newly registered CityAdded routes while the canonical owner is
/// already loaned by another synchronous stock callback.
pub fn city_added(
    state: &mut LoadedState,
    owners: &mut super::unit_creation::ModifierOwners,
    city: ComponentId,
    reports: &mut impl modifier_effects::Effects,
) -> Result<(), String> {
    city_added_parts(
        state,
        &mut owners.effects,
        &mut owners.requirements,
        city,
        reports,
    )
}
/// The same dispatch for a scope that already split the canonical owner loan.
pub fn city_added_parts(
    state: &mut LoadedState,
    effects: &mut crate::melee_attack::backend::EffectRuntime,
    requirements: &mut crate::melee_attack::backend::requirements::Runtime,
    city: ComponentId,
    reports: &mut impl modifier_effects::Effects,
) -> Result<(), String> {
    let mut index = 0;
    while let Some(route) = effects
        .player_cities_subscriptions()
        .map_err(text)?
        .get(index)
        .copied()
    {
        index += 1;
        effects
            .notify_player_city_item(state, requirements, route, city, true, reports)
            .map_err(text)?;
    }
    Ok(())
}
/// Reconcile the live capital set using the already loaned canonical owner.
pub fn capital_changed(
    state: &mut LoadedState,
    owners: &mut super::unit_creation::ModifierOwners,
    city: ComponentId,
    reports: &mut impl modifier_effects::Effects,
) -> Result<(), String> {
    capital_changed_parts(
        state,
        &mut owners.effects,
        &mut owners.requirements,
        city,
        reports,
    )
}
/// Reconcile capital subjects while effect and requirement owners are split.
pub fn capital_changed_parts(
    state: &mut LoadedState,
    effects: &mut crate::melee_attack::backend::EffectRuntime,
    requirements: &mut crate::melee_attack::backend::requirements::Runtime,
    city: ComponentId,
    reports: &mut impl modifier_effects::Effects,
) -> Result<(), String> {
    let mut index = 0;
    while let Some(route) = effects
        .player_capital_subscriptions()
        .map_err(text)?
        .get(index)
        .copied()
    {
        index += 1;
        effects
            .notify_player_capital_changed(
                state,
                requirements,
                route,
                i32::from(city.owner),
                reports,
            )
            .map_err(text)?;
    }
    Ok(())
}
impl Runtime<'_> {
    /// Invoke the newly registered CityCreated/CityAdded+288 listener tail,
    /// after previously installed source listeners in that same signal ring.
    /// Registration order stays in the canonical owner; no side registry exists.
    pub fn founding_city_added(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        reports: &mut impl modifier_effects::Effects,
    ) -> Result<(), String> {
        self.with_modifier_owners(|_, owners| city_added(state, owners, city, reports))
            .map_err(text)?
    }
    /// Invoke each newly appended PlayerCapitalCity listener at CapitalChanged
    /// +408. The helper recomputes the LIVE capital set, including removals.
    pub fn founding_capital_changed(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        reports: &mut impl modifier_effects::Effects,
    ) -> Result<(), String> {
        self.with_modifier_owners(|_, owners| capital_changed(state, owners, city, reports))
            .map_err(text)?
    }
}
