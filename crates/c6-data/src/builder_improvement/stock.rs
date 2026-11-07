//! Concrete ordinary Farm services borrowing the ongoing stock backend and sight.
use super::{legality, placement, services, workers, Definitions, Report};
use crate::{
    barbarian_camp_placement as shared,
    loaded_actions::{self as actions, city_range_attack, move_to, stock_notifications::Runtime},
    unit_sight_state,
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
        write!(f, "Farm {}: {}", self.0, self.1)
    }
}
impl std::error::Error for Error {}
fn err(stage: &'static str, e: impl std::fmt::Debug) -> Error {
    Error(stage, format!("{e:?}"))
}
/// Cached static services; caller constructs these once for its admitted Rules.
pub struct Cached {
    pub workers: crate::city_turn::upkeep::citizens::Definitions,
    pub appeal: crate::city_production_turn::appeal::Definitions,
    pub fortify_turn_max: i32,
}
impl Cached {
    pub fn from_rules(state: &LoadedState, rules: &crate::Rules) -> Result<Self, Error> {
        Ok(Self {
            workers: crate::city_turn::upkeep::citizens::Definitions::from_rules(rules)
                .map_err(|e| err("worker definitions", e))?,
            appeal: crate::city_production_turn::appeal::Definitions::from_current_rules(
                state, rules,
            )
            .map_err(|e| err("appeal definitions", e))?,
            fortify_turn_max: crate::game_parameters::Parameters::from_rules(rules)
                .map_err(|e| err("parameters", e))?
                .integer(crate::game_parameters::IntegerParameter::FortifyTurnMax)
                .map_err(|e| err("fortify parameter", e))?,
        })
    }
}
pub struct Dispatcher<'a, 'd> {
    pub death: Option<&'a super::death::Inputs<'a, 'd>>,
    pub definitions: &'a Definitions,
    pub cached: &'a Cached,
    pub services: &'a mut services::Current,
    pub legality: legality::Inputs<'a>,
    pub sight_definitions: &'a city_range_attack::Definitions,
    pub upkeep: &'a crate::unit_movement_state::Inputs<'d>,
    /// Borrow the SAME current Gossip history retained across actions/turns.
    pub gossip_history: &'a [c6_core::city_power_activation::gossip_delivery::TurnVisibility],
    pub alive_major_ids: &'a [i32],
    pub reports: &'a mut Vec<Report>,
    pub worker_reports: &'a mut Vec<workers::Report>,
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
            .ok_or_else(|| err("report observer", "missing"))?;
        if observer != state.retained().core.local_player && observer != 1000 {
            return Err(err("report observer", "changed current observer"));
        }
        Ok(crate::melee_attack::experience::reporting::observer_can_see_player(observer, owner))
    }
    fn set_improvement(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        defs: &Definitions,
        owner: i32,
    ) -> Result<(), Error> {
        placement::first(state, self, plot, defs, owner).map_err(|e| err("first placement", e))
    }
    fn culture_bomb(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        _: i32,
    ) -> Result<(), Error> {
        let p = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == actor.player())
            .ok_or_else(|| err("culture owner", "missing"))?;
        if p.culture
            .culture_bomb_triggers
            .iter()
            .any(|t| t.improvement_hash == self.dispatcher.definitions.improvement_hash)
        {
            return Err(err("culture bomb", "positive Farm trigger"));
        }
        Ok(())
    }
    fn report(&mut self, state: &mut LoadedState, report: Report) -> Result<(), Error> {
        match report {
            Report::Operation { unit, improvement } => {
                crate::loaded_upkeep::OperationReports::operation_report(
                    self.backend,
                    state,
                    actions::IdentityReport {
                        event: 0x3d4006f2,
                        subtype: c6_core::unit::BUILD_IMPROVEMENT_HASH,
                        identity: unit,
                        first: Some(improvement),
                        second: None,
                    },
                )
                .map_err(|e| err("operation reporting", e))?;
            }
            Report::LastCharge(unit) => crate::loaded_upkeep::OperationReports::placement_report(
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
            .map_err(|e| err("last-charge visibility reporting", e))?,
            Report::ChargesChanged { unit, .. } => self
                .backend
                .reporting
                .observe_builder_charges(state, UnitReference::from_component_id(unit))
                .map_err(|e| err("charge reporting", e))?,
            // The remaining reached stock reports publish derived city/map/unit
            // snapshots and presentation events. Preserve semantic output here.
            _ => {}
        }
        self.dispatcher.reports.push(report);
        Ok(())
    }
    fn local_unit_visible(&self, state: &LoadedState, actor: UnitReference) -> Result<bool, Error> {
        let unit = state
            .world()
            .unit(actor)
            .map_err(|e| err("visible unit", e))?
            .ok_or_else(|| err("visible unit", "missing"))?
            .record;
        let observer = self
            .sight
            .local_observer
            .ok_or_else(|| err("visible observer", "missing"))?;
        let location = unit.k_location;
        city_range_attack::unit_visible_with_counts::<actions::Error>(
            state,
            unit,
            observer,
            self.dispatcher.sight_definitions,
            |hash, normal| {
                let layers = self
                    .sight
                    .shared(observer)
                    .map_err(actions::Error::Unsupported)?;
                let Some(layer) = layers.0.iter().find(|l| l.type_hash == hash) else {
                    return if normal {
                        Err(actions::Error::Unsupported("Farm normal visibility"))
                    } else {
                        Ok(0)
                    };
                };
                let index = layer
                    .grid_width
                    .wrapping_mul(location.y as u32)
                    .wrapping_add(location.x as u32) as usize;
                Ok(layer.values.get(index).copied().unwrap_or(0))
            },
            || {
                self.sight
                    .hidden_layer_hashes
                    .first()
                    .copied()
                    .ok_or(actions::Error::Unsupported("Farm hidden layer0"))
            },
        )
        .map_err(|e| err("local unit visibility", e))
    }
    fn reallocate_owning_city(
        &mut self,
        state: &mut LoadedState,
        location: PlotCoord,
    ) -> Result<Option<ComponentId>, Error> {
        let (_, plot) = c6_core::movement_world::plot_at(state.world().map(), location)
            .map_err(|e| err("owning city plot", e))?;
        let Some(a) = &plot.city_attributes else {
            return Ok(None);
        };
        if a.player < 0 || a.owning_city == u32::MAX {
            return Ok(None);
        }
        let owner = i32::from(a.player);
        let id = a.owning_city;
        let present = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .and_then(|p| {
                p.cities
                    .records
                    .entries
                    .iter()
                    .find(|c| c.value.header.id == id)
            })
            .is_some();
        if !present {
            return Ok(None);
        }
        crate::city_turn::upkeep::citizens::reallocate(
            state,
            owner,
            id,
            &mut workers::Current {
                backend: self.backend,
                definitions: &self.dispatcher.cached.workers,
                appeal: &self.dispatcher.cached.appeal,
                fallout: self.dispatcher.legality.fallout_turns,
                appeal_subscribers: self
                    .dispatcher
                    .services
                    .appeal_changed_subscribers
                    .as_deref(),
                reports: self.dispatcher.worker_reports,
            },
        )
        .map_err(|e| err("ReallocateCitizens", e))?;
        Ok(Some(ComponentId {
            owner: owner as i16,
            component_type: 2,
            id,
        }))
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
    fn delayed_destroy(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
    ) -> Result<(), Error> {
        let inputs = self.dispatcher.death.ok_or_else(|| {
            err(
                "last-charge delayed destruction",
                "current lifecycle services",
            )
        })?;
        super::death::delayed_destroy(
            state,
            self.sight,
            self.backend,
            inputs,
            (actor.player(), actor.id),
        )
    }
}
impl<P> shared::OwnerContext for Current<'_, '_, '_, '_, P> {
    type Error = Error;
    fn valid_player(&self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        state
            .world()
            .valid_player(owner)
            .map(|v| v.is_some())
            .map_err(|e| err("valid owner", e))
    }
    fn adjacent(
        &self,
        state: &LoadedState,
        plot: usize,
        direction: u8,
    ) -> Result<Option<usize>, Error> {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot as u32)
            .ok_or_else(|| err("owner adjacent", "plot"))?;
        crate::barbarian_camps::queries::adjacent(state, PlotCoord { x, y }, direction as usize)
            .map_err(|e| err("owner adjacent", e))
    }
    fn visibility(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        plot: usize,
    ) -> Result<(), Error> {
        let source = ComponentId {
            owner: -1,
            component_type: 0,
            id: u32::MAX,
        };
        let mut context = unit_sight_state::Context {
            state,
            runtime: self.sight,
            effects: self.backend,
            sight_definitions: self.dispatcher.sight_definitions,
            actor: (-1, u32::MAX),
        };
        c6_core::visibility_change::change_visibility_count(
            &mut context,
            c6_core::unit_sight::VisibilityChange {
                owner,
                plot_index: plot as u32,
                delta: 1,
                hidden_type: -1,
                source_flag: true,
                near_origin: false,
                origin_index: -1,
                sight_range: 1,
                source,
            },
        )
        .map(|_| ())
        .map_err(|e| err("improvement owner sight", e))
    }
    fn owner_changed(&mut self, _: &mut LoadedState, _: usize, _: i32) -> Result<(), Error> {
        Ok(())
    }
}
impl<P> shared::VisualContext for Current<'_, '_, '_, '_, P> {
    type Error = Error;
    fn alive_ids(&self) -> Result<Vec<i32>, Error> {
        use crate::unit_placement_services::Services;
        let services = self
            .backend
            .placement_service_runtime()
            .map_err(|e| err("alive services", e))?;
        let (token, count) = services
            .alive_begin::<Error>()
            .map_err(|e| err("alive begin", e))?;
        (0..count)
            .map(|i| {
                services
                    .alive_at::<Error>(token, i)
                    .map_err(|e| err("alive entry", e))
            })
            .collect()
    }
    fn messages_disabled(&self) -> bool {
        self.sight.messages_disabled
    }
    fn local_observer(&self) -> Option<i32> {
        self.sight.local_observer
    }
    fn is_revealed(&self, state: &LoadedState, owner: i32, plot: usize) -> Result<bool, Error> {
        match state
            .world()
            .visibility(owner)
            .and_then(|v| v.revealed.get(plot))
            .copied()
        {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(err("revealed improvement", "noncanonical cell")),
        }
    }
    fn revealed_state(&self, state: &LoadedState, owner: i32, plot: usize) -> Result<i32, Error> {
        let shared = self
            .sight
            .shared(owner)
            .map_err(|e| err("shared visibility", e))?;
        let layer = shared
            .0
            .iter()
            .find(|l| l.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
            .ok_or_else(|| err("revealed normal layer", "missing"))?;
        let width = state.world().map().header.width as usize;
        let index = (plot / width) * layer.grid_width as usize + plot % width;
        match layer.values.get(index) {
            Some(v) if *v > 0 => Ok(2),
            Some(_) => self.is_revealed(state, owner, plot).map(i32::from),
            None => Err(err("revealed normal layer", "cell")),
        }
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        event: u32,
        _: i32,
    ) -> Result<(), Error> {
        if event == 0xe5b20b82 {
            // RestoreVisualState sends OnImprovementAddedToMap before updating
            // visibility. The Map recipient (return27bcc05) queries this
            // improvement with a nonnull contribution vector. Preserve its
            // temporary PlotYields lifecycle on the SAME canonical owner even
            // when the resulting presentation values are omitted.
            let current = state
                .world()
                .map()
                .plots
                .get(plot)
                .ok_or_else(|| err("added improvement report", "plot"))?;
            if current.improvement_hash != self.dispatcher.definitions.improvement_hash {
                return Err(err(
                    "added improvement report",
                    "current definition changed",
                ));
            }
            let city = match &current.city_attributes {
                Some(attributes)
                    if attributes.player >= 0 && attributes.owning_city != u32::MAX =>
                {
                    let owner = i32::from(attributes.player);
                    state
                        .city(owner, attributes.owning_city)
                        .map_err(|e| err("added improvement report owning city", e))?;
                    Some(c6_core::city_territory::CityId {
                        owner,
                        id: attributes.owning_city,
                    })
                }
                _ => None,
            };
            let query = self
                .backend
                .city_query_inputs()
                .ok_or_else(|| err("added improvement report", "current city query owner"))?;
            let mut territory = query
                .current
                .territory
                .write()
                .map_err(|e| err("added improvement report mutable territory", e))?;
            crate::loaded_actions::pillage::reporting::temporary_plot_yields(
                &mut territory,
                plot as u32,
                city,
                self.dispatcher.definitions.improvement_index,
            )
            .map_err(|e| err("added improvement report effects", e))?;
        } else if !matches!(event, 0xa625c726 | 0xf391d7b3) {
            return Err(err("improvement visual report", "unsupported event"));
        }
        Ok(())
    }
}
impl<P> placement::Effects for Current<'_, '_, '_, '_, P> {
    fn constructed(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        defs: &Definitions,
    ) -> Result<(), Error> {
        self.dispatcher
            .services
            .constructed(state, plot, defs, self.dispatcher.gossip_history)
            .map_err(|e| err("constructed", e))
    }
    fn calculate_monopolies(&mut self, _: &mut LoadedState) -> Result<(), Error> {
        self.dispatcher
            .services
            .calculate_monopolies(self.dispatcher.alive_major_ids)
            .map_err(|e| err("Economic monopolies", e))
    }
}
impl<P> move_to::Movement<Runtime<'_, P>> for Dispatcher<'_, '_> {
    type Error = Error;
    fn farm_can_start(
        &mut self,
        state: &LoadedState,
        unit: &Unit,
        improvement: u32,
        visible: bool,
        _: &Runtime<'_, P>,
    ) -> Result<Option<bool>, Error> {
        legality::can_start(
            state,
            self.definitions,
            &self.legality,
            reference((unit.e_owner, unit.id)),
            unit,
            improvement,
            visible,
        )
        .map(Some)
        .map_err(|e| err("CanStart", e))
    }
    fn start_farm(
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
        self.cached.fortify_turn_max
    }
    fn is_fortifyable(&self, _: &LoadedState, _: (i32, u32)) -> Result<bool, Error> {
        Err(err("other queued handler", "Fortify"))
    }
    fn alert_enemy_visible(
        &self,
        _: &LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
    ) -> Result<bool, Error> {
        Err(err("other queued handler", "Alert"))
    }
    fn city_at(
        &self,
        _: &LoadedState,
        _: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, Error> {
        Err(err("other queued handler", "Trade city"))
    }
    fn preflight(&self, _: &LoadedState, _: (i32, u32), _: move_to::Request) -> Result<(), Error> {
        Err(err("other queued handler", "MoveTo preflight"))
    }
    fn test_move_to(
        &mut self,
        _: &mut LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
        _: u32,
    ) -> Result<i32, Error> {
        Err(err("other queued handler", "TestMoveTo"))
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
        Err(err("other queued handler", "MoveTo"))
    }
    fn valid_location(
        &mut self,
        _: &LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
    ) -> Result<i32, Error> {
        Err(err("other queued handler", "IsValidLocation"))
    }
}
