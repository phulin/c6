//! Concrete ordinary forest removal callbacks on canonical stock owners.
use super::{Definitions, Report};
use crate::{
    loaded_actions::{self as actions, move_to, stock_notifications::Runtime},
    unit_sight_state,
};
use c6_core::unit::variants::{
    ArrayVariant, TypedEntry, TypedVariant, TypedVariantMap, VariantPayload,
};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, OperationNode, PlotCoord, Unit},
};
#[derive(Debug)]
pub struct Error(pub &'static str, pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chop {}: {}", self.0, self.1)
    }
}
impl std::error::Error for Error {}
fn err(stage: &'static str, e: impl std::fmt::Debug) -> Error {
    Error(stage, format!("{e:?}"))
}
/// Ordered actual Player.EventManager registrations. Changed receives NO_FEATURE;
/// Removed receives the old feature, once in SetFeatureType and again in Start.
pub struct Callbacks {
    binding: c6_core::core_player::initialization::WireBinding,
    pub placed: Vec<i32>,
    pub removed: Vec<i32>,
}
impl Callbacks {
    pub fn from_initial(state: &LoadedState, placed: Vec<i32>, removed: Vec<i32>) -> Self {
        Self {
            binding: state.binding().clone(),
            placed,
            removed,
        }
    }
}
pub struct Dispatcher<'a, 'd> {
    pub definitions: &'a Definitions,
    pub callbacks: &'a Callbacks,
    pub economic: c6_core::research_acquisition::units::Economic,
    pub civic_graph: &'a actions::culture_notifications::GameCulture,
    pub fallout: &'a [i32],
    pub national_parks: &'a [c6_core::game_national_parks::ParkData],
    pub buildings: &'a crate::city_production_turn::buildings::Definitions,
    pub completion: Option<actions::stock_notifications::city_production::completion::Services<'a>>,
    pub death: Option<&'a crate::builder_improvement::death::Inputs<'a, 'd>>,
    pub upkeep: &'a crate::unit_movement_state::Inputs<'d>,
    pub appeal: &'a crate::city_production_turn::appeal::Definitions,
    pub appeal_subscribers: Option<&'a [u64]>,
    pub presentation_reports_qualified: bool,
    pub fortify_turn_max: i32,
    pub reports: &'a mut Vec<Output>,
}
#[derive(Clone, Copy, Debug)]
pub enum Output {
    Operation(Report),
    Production { owner: i32, city: u32, amount: i32 },
    Appeal { plot: u32, value: i32 },
    AppealReport(u32),
}
struct Current<'a, 's, 'r, 'd, P> {
    dispatcher: &'a mut Dispatcher<'s, 'd>,
    backend: &'a mut Runtime<'r, P>,
    sight: &'a mut unit_sight_state::Runtime,
}
fn reference(actor: (i32, u32)) -> UnitReference {
    UnitReference::from_component_id(ComponentId {
        owner: actor.0 as i16,
        component_type: 1,
        id: actor.1,
    })
}
impl<P> super::Effects for Current<'_, '_, '_, '_, P> {
    type Error = Error;
    fn observer_can_see_owner(&self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        let observer = self
            .sight
            .local_observer
            .ok_or_else(|| err("observer", "missing"))?;
        if observer != state.retained().core.local_player && observer != 1000 {
            return Err(err("observer", "changed current observer"));
        }
        Ok(crate::melee_attack::experience::reporting::observer_can_see_player(observer, owner))
    }
    fn report(&mut self, state: &mut LoadedState, report: Report) -> Result<(), Error> {
        match report {
            Report::Operation(unit) => {
                crate::loaded_upkeep::OperationReports::operation_report(
                    self.backend,
                    state,
                    actions::IdentityReport {
                        event: 0x3d4006f2,
                        subtype: super::OPERATION_HASH,
                        identity: unit,
                        first: Some(0),
                        second: None,
                    },
                )
                .map_err(|e| err("operation report", e))?;
            }
            Report::Charges { unit, .. } => self
                .backend
                .reporting
                .observe_builder_charges(state, UnitReference::from_component_id(unit))
                .map_err(|e| err("charge report", e))?,
            Report::LastCharge(unit) => {
                crate::loaded_upkeep::OperationReports::placement_report(
                    self.backend,
                    state,
                    c6_core::unit_placement_transition::PlacementReport {
                        event_hash: 0xc1300ee7,
                        identity: unit,
                        payload: c6_core::unit_placement_transition::PlacementPayload::Visibility {
                            state: 0,
                            initial_placement: None,
                        },
                    },
                )
                .map_err(|e| err("last charge report", e))?;
            }
            _ => {}
        }
        if !self.dispatcher.presentation_reports_qualified {
            return Err(err("reports", "unqualified recipients"));
        }
        self.dispatcher.reports.push(Output::Operation(report));
        Ok(())
    }
    fn remove_feature(&mut self, state: &mut LoadedState, plot: usize) -> Result<(), Error> {
        if state.binding() != &self.dispatcher.callbacks.binding {
            return Err(err("callbacks", "binding"));
        }
        let p = state
            .world()
            .map()
            .plots
            .get(plot)
            .ok_or_else(|| err("feature plot", "missing"))?;
        if !self
            .dispatcher
            .definitions
            .passable_terrains
            .contains(&p.terrain_hash)
            || p.feature_hash != self.dispatcher.definitions.feature_hash
            || p.feature_layout != 0
            || p.improvement_hash != u32::MAX
            || p.resource_hash != u32::MAX
            || p.flags.get(c6_core::map::PlotFlag::Impassable)
        {
            return Err(err(
                "feature removal",
                "ordinary passable single-plot unimproved forest",
            ));
        }
        let p = state
            .world_mut()
            .map_err(|e| err("mutable feature", e))?
            .plot_mut(plot)
            .ok_or_else(|| err("feature plot", "missing"))?;
        p.feature_hash = u32::MAX;
        p.feature_layout = 0;
        reset_direction(&mut p.properties)?;
        // OnTerrainOrFeatureChanged tests the unchanged passability bit; the
        // removed ordinary forest has no volcano/natural-wonder continuation.
        // RemoveFeature's report then Appeal.Update precede setter signals.
        if !self.sight.messages_disabled {
            self.update_appeal(state, plot)?;
        }
        // Feature.Manager.OnFeatureChanged returns immediately for NO_FEATURE.
        // Setter calls OnTerrainOrFeatureChanged again, then signals placed and
        // removed. Placed(NO_FEATURE).Stats is the verified ret leaf.
        for &owner in &self.dispatcher.callbacks.placed {
            if !state
                .retained()
                .players
                .iter()
                .any(|p| p.header.owner == owner)
            {
                return Err(err("placed receiver", "missing"));
            }
        }
        self.feature_removed(state, plot, self.dispatcher.definitions.feature_index)
    }
    fn feature_removed(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        _feature: i32,
    ) -> Result<(), Error> {
        let owner = crate::movement_placement_state::plot_owner(&state.world().map().plots[plot])
            .map_err(|e| err("removed owner", e))?;
        for &receiver in &self.dispatcher.callbacks.removed {
            if receiver != owner
                && state
                    .game_relations()
                    .map_err(|e| err("removed relations", e))?
                    .has_met(receiver, owner)
                    .map_err(|e| err("removed HasMet", e))?
            {
                let index = usize::try_from(owner).map_err(|e| err("removed owner index", e))?;
                let stats = state
                    .player_stats_mut(receiver)
                    .map_err(|e| err("removed stats", e))?;
                let count = stats
                    .removed_feature_counts
                    .get_mut(index)
                    .ok_or_else(|| err("removed stats index", "out of range"))?;
                *count = count.wrapping_add(1);
            }
        }
        Ok(())
    }
    fn obtain_yield(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        _feature: i32,
        base: i32,
    ) -> Result<i32, Error> {
        let owner = actor.player();
        let unit = state
            .world()
            .unit(actor)
            .map_err(|e| err("yield actor", e))?
            .ok_or_else(|| err("yield actor", "missing"))?
            .record;
        let location = unit.k_location;
        let (_, plot) = c6_core::movement_world::plot_at(state.world().map(), location)
            .map_err(|e| err("yield plot", e))?;
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or_else(|| err("yield player", "missing"))?;
        let owning = plot
            .city_attributes
            .as_ref()
            .filter(|a| i32::from(a.player) == owner)
            .map(|a| a.owning_city)
            .filter(|id| {
                player
                    .cities
                    .records
                    .entries
                    .iter()
                    .any(|c| c.value.header.id == *id)
            });
        let map = state.world().map();
        let city = owning.or_else(|| {
            player
                .cities
                .records
                .entries
                .iter()
                .min_by_key(|c| {
                    c6_core::geometry::plot_distance(
                        map.header.width,
                        map.header.wrap_x,
                        location.x,
                        location.y,
                        c.value.header.location_x,
                        c.value.header.location_y,
                    )
                })
                .map(|c| c.value.header.id)
        });
        let mut amount = base;
        if base > 0 {
            let graph = state
                .game_techs()
                .map_err(|e| err("yield technologies", e))?;
            let defs = self.dispatcher.definitions;
            let active = graph
                .iteration()
                .iter()
                .filter_map(|&i| graph.node(i).filter(|n| n.active).map(|_| i))
                .collect::<Vec<_>>();
            let civics = self.dispatcher.civic_graph;
            let civic_active = civics
                .iteration()
                .iter()
                .filter_map(|&i| {
                    civics
                        .nodes()
                        .iter()
                        .find(|n| n.native_type == i && n.active)
                        .map(|_| i)
                })
                .collect::<Vec<_>>();
            let percent = c6_core::research_acquisition::units::percent_complete(
                &player.techs.has_tech,
                &defs.technologies,
                &active,
                defs.technologies.len() as u32,
            )
            .map_err(|e| err("technology percent", e))?
            .max(
                c6_core::research_acquisition::units::percent_complete(
                    &player.culture.has_civic,
                    &defs.civics,
                    &civic_active,
                    defs.civics.len() as u32,
                )
                .map_err(|e| err("civic percent", e))?,
            );
            let economic = self.dispatcher.economic;
            amount = ((economic.game_cost_escalation.wrapping_mul(base) / 100).wrapping_sub(base))
                .wrapping_mul(percent)
                .wrapping_div(100)
                .wrapping_add(base);
            amount = c6_core::research_acquisition::units::adjust_for_game_speed(
                amount,
                economic.speed_cost_multiplier,
            );
            if let Some(city) = city {
                amount = state
                    .city(owner, city)
                    .map_err(|e| err("harvest city", e))?
                    .resources
                    .harvest_bonus_modifier
                    .wrapping_add(100)
                    .wrapping_mul(amount)
                    / 100;
            }
        }
        if let Some(city) = city {
            let mut effects=actions::stock_notifications::city_production::Effects::new(self.backend,self.dispatcher.buildings,self.dispatcher.appeal,actions::stock_notifications::city_production::Scope::StockPlainHeadlessNoAddedConsumers);
            if let Some(completion) = self.dispatcher.completion {
                effects = effects.with_completion(completion);
            }
            let outcome = crate::city_production_turn::add_progress(
                state,
                owner,
                city,
                amount.wrapping_shl(8),
                amount.wrapping_shl(8),
                true,
                &mut effects,
            )
            .map_err(|e| err("harvest production", e))?;
            if matches!(
                outcome,
                crate::city_production_turn::Outcome::NeedsProduction
                    | crate::city_production_turn::Outcome::Completed {
                        needs_production: true
                    }
            ) {
                self.backend
                    .publish_city_production(state, owner, city)
                    .map_err(|e| err("harvest production decision", e))?;
            }
            self.dispatcher.reports.push(Output::Production {
                owner,
                city,
                amount,
            });
        }
        Ok(amount)
    }
    fn harvest_bonuses(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        feature: i32,
        total: i32,
    ) -> Result<(), Error> {
        let p = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == actor.player())
            .ok_or_else(|| err("harvest player", "missing"))?;
        if total > 0 && p.religion.add_faith_harvest_modifier > 0 {
            return Err(err("harvest faith", "positive continuation"));
        }
        if total > 0
            && p.tail
                .as_ref()
                .and_then(|t| t.alive.as_ref())
                .ok_or_else(|| err("harvest behaviors", "missing"))?
                .chop_behaviors
                .iter()
                .any(|b| b.feature_wire == feature && b.amount > 0)
        {
            return Err(err("harvest yield bonus", "positive continuation"));
        }
        Ok(())
    }
    fn delayed_destroy(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
    ) -> Result<(), Error> {
        let inputs = self
            .dispatcher
            .death
            .ok_or_else(|| err("last charge", "current lifecycle services"))?;
        crate::builder_improvement::death::delayed_destroy(
            state,
            self.sight,
            self.backend,
            inputs,
            (actor.player(), actor.id),
        )
        .map_err(|e| err("delayed destroy", e))
    }
    fn local_plot_visible(&self, state: &LoadedState, actor: UnitReference) -> Result<bool, Error> {
        let u = state
            .world()
            .unit(actor)
            .map_err(|e| err("visible unit", e))?
            .ok_or_else(|| err("visible unit", "missing"))?
            .record;
        let observer = self
            .sight
            .local_observer
            .ok_or_else(|| err("visible observer", "missing"))?;
        let shared = self
            .sight
            .shared(observer)
            .map_err(|e| err("shared sight", e))?;
        let layer = shared
            .0
            .iter()
            .find(|l| l.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
            .ok_or_else(|| err("base sight", "missing"))?;
        let index = layer
            .grid_width
            .wrapping_mul(u.k_location.y as u32)
            .wrapping_add(u.k_location.x as u32) as usize;
        Ok(layer.values.get(index).copied().unwrap_or(0) > 0)
    }
    fn finish_moves(&mut self, state: &mut LoadedState, actor: UnitReference) -> Result<(), Error> {
        crate::loaded_upkeep::finish_unit_moves(
            state,
            actor.player(),
            actor.id,
            self.dispatcher.upkeep,
            self.backend,
        )
        .map_err(|e| err("FinishMoves", e))
    }
}
impl<P> Current<'_, '_, '_, '_, P> {
    fn update_appeal(&mut self, state: &mut LoadedState, plot: usize) -> Result<(), Error> {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot as u32)
            .ok_or_else(|| err("appeal plot", "coordinates"))?;
        crate::city_production_turn::appeal::update(state, [x, y], self.dispatcher.appeal, self)
            .map(|_| ())
            .map_err(|e| err("appeal", e))
    }
}
impl<P> crate::city_production_turn::appeal::Effects for Current<'_, '_, '_, '_, P> {
    type Error = Error;
    fn appeal_changed(&mut self, _: &mut LoadedState, plot: u32, value: i32) -> Result<(), Error> {
        if self.dispatcher.appeal_subscribers != Some(&[]) {
            return Err(err("appeal signal", "unknown/positive subscribers"));
        }
        self.dispatcher.reports.push(Output::Appeal { plot, value });
        Ok(())
    }
    fn appeal_report(&mut self, _: &mut LoadedState, plot: u32) -> Result<(), Error> {
        if !self.dispatcher.presentation_reports_qualified {
            return Err(err("appeal report", "unqualified recipients"));
        }
        self.dispatcher.reports.push(Output::AppealReport(plot));
        Ok(())
    }
}
impl<P> move_to::Movement<Runtime<'_, P>> for Dispatcher<'_, '_> {
    type Error = Error;
    fn chop_can_start(
        &mut self,
        state: &LoadedState,
        unit: &Unit,
        visible: bool,
        _: &Runtime<'_, P>,
    ) -> Result<Option<bool>, Error> {
        super::can_start(
            state,
            self.definitions,
            &super::Inputs {
                fallout_turns: self.fallout,
                national_parks: self.national_parks,
            },
            unit,
            visible,
        )
        .map(Some)
        .map_err(|e| err("CanStart", e))
    }
    fn start_chop(
        &mut self,
        state: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        actor: (i32, u32),
        node: &OperationNode,
        backend: &mut Runtime<'_, P>,
    ) -> Result<Option<c6_core::skip_turn::StartData>, Error> {
        let defs = self.definitions;
        super::start(
            state,
            &mut Current {
                dispatcher: self,
                backend,
                sight,
            },
            defs,
            reference(actor),
            node,
        )
        .map(Some)
        .map_err(|e| err("Start", e))
    }
    fn fortify_turn_max(&self) -> i32 {
        self.fortify_turn_max
    }
    fn is_fortifyable(&self, _: &LoadedState, _: (i32, u32)) -> Result<bool, Error> {
        Err(err("other handler", "Fortify"))
    }
    fn alert_enemy_visible(
        &self,
        _: &LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
    ) -> Result<bool, Error> {
        Err(err("other handler", "Alert"))
    }
    fn city_at(
        &self,
        _: &LoadedState,
        _: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, Error> {
        Err(err("other handler", "Trade"))
    }
    fn preflight(&self, _: &LoadedState, _: (i32, u32), _: move_to::Request) -> Result<(), Error> {
        Err(err("other handler", "MoveTo"))
    }
    fn test_move_to(
        &mut self,
        _: &mut LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
        _: u32,
    ) -> Result<i32, Error> {
        Err(err("other handler", "MoveTo"))
    }
    fn move_to(
        &mut self,
        _: &mut LoadedState,
        _: &mut unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
        _: u32,
        _: &mut Runtime<'_, P>,
    ) -> Result<i32, Error> {
        Err(err("other handler", "MoveTo"))
    }
    fn valid_location(
        &mut self,
        _: &LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
    ) -> Result<i32, Error> {
        Err(err("other handler", "MoveTo"))
    }
}

fn reset_direction(properties: &mut Option<TypedVariantMap>) -> Result<(), Error> {
    let properties = properties.get_or_insert_with(TypedVariantMap::default);
    if !properties.entries.iter().any(|e| e.key == 0x2f95b305) {
        properties.entries.push(TypedEntry {
            key: 0x2f95b305,
            name: None,
            value: TypedVariant {
                type_bits: 10,
                modifier_bits: 0,
                value: ArrayVariant {
                    count_flags: 0x05000000,
                    element_size: 0,
                    payload: VariantPayload::TypedVariantMap(Box::default()),
                },
            },
        });
    }
    let entry = properties
        .entries
        .iter_mut()
        .find(|e| e.key == 0x2f95b305)
        .ok_or_else(|| err("feature property map", "missing"))?;
    if entry.value.type_bits != 10 || entry.value.modifier_bits != 0 {
        return Err(err("feature properties", "non-map"));
    }
    let VariantPayload::TypedVariantMap(feature) = &mut entry.value.value.payload else {
        return Err(err("feature property payload", "non-map"));
    };
    feature.entries.retain(|e| e.key != 0x4344acef);
    Ok(())
}
