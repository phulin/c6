//! Stock improvement pillage on persistent reporting, sight and gameplay owners.
use super::{plunder, stock, Definitions, Effects, Queries, Report};
use crate::{
    loaded_actions::{self as actions, city_range_attack, move_to, stock_notifications::Runtime},
    unit_sight_state,
};
use c6_core::{
    city_power_activation::gossip_delivery::TurnVisibility,
    game_capabilities::Membership,
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, OperationNode, PlotCoord, Unit},
};
#[derive(Debug)]
pub struct Error(pub &'static str, pub String);
fn error(stage: &'static str, value: impl std::fmt::Debug) -> Error {
    Error(stage, format!("{value:?}"))
}
#[derive(Clone, Copy, Debug)]
pub enum Output {
    Operation(Report),
    Plunder(plunder::Report),
    Appeal { plot: u32, value: i32 },
    AppealReport(u32),
}
/// Current static definitions and source-qualified ordered callback registrations.
/// Mutable world values and effective predicate/award answers are never inputs.
pub struct Inputs<'a, 'd, 'r> {
    pub definitions: &'a Definitions,
    pub callbacks: &'a stock::Inputs,
    pub plunder: &'a plunder::Definitions,
    pub capabilities: &'a Membership,
    pub civilization_levels: &'a [(i32, u32)],
    pub cached_majors: &'a [i32],
    pub combat: &'a crate::formation_rules::Definitions,
    pub movement: &'a crate::unit_movement_state::Inputs<'d>,
    pub placement: &'a crate::unit_placement_state::Definitions<'d, 'r>,
    pub queries: &'a crate::unit_placement_state::Queries<'d>,
    pub sight_definitions: &'a city_range_attack::Definitions,
    pub appeal: &'a crate::city_production_turn::appeal::Definitions,
    pub appeal_subscribers: Option<&'a [u64]>,
    /// Actual report cache/translator recipients are evidenced presentation
    /// consumers, without added synchronous gameplay listeners.
    pub presentation_reports_qualified: bool,
    pub loyalty_maximum: Option<i32>,
    pub fortify_turn_max: i32,
}
pub struct Dispatcher<'a, 'd, 'r> {
    pub inputs: &'a Inputs<'a, 'd, 'r>,
    pub gossip_history: &'a [TurnVisibility],
    pub reports: &'a mut Vec<Output>,
}
struct Legality<'a, 'd, 'r>(&'a Inputs<'a, 'd, 'r>);
impl Queries for Legality<'_, '_, '_> {
    type Error = Error;
    fn is_combat(&self, s: &LoadedState, u: &Unit) -> Result<bool, Error> {
        crate::combat_defender::is_combat(s, self.0.combat, u).map_err(|e| error("IsCombat", e))
    }
    fn at_war(&self, s: &LoadedState, o: i32, t: i32) -> Result<bool, Error> {
        // Player.IsAtWarWith rejects invalid/dead targets before relation lookup.
        let Some(target) = s
            .world()
            .valid_player(t)
            .map_err(|e| error("war target", e))?
        else {
            return Ok(false);
        };
        if !target
            .is_alive
            .ok_or_else(|| error("war target alive", "unavailable"))?
            || o == t
        {
            return Ok(false);
        }
        s.game_relations()
            .map_err(|e| error("relations", e))?
            .is_at_war(o, t)
            .map_err(|e| error("war", e))
    }
    fn max_moves(&self, s: &LoadedState, u: &Unit) -> Result<i32, Error> {
        crate::unit_movement_state::unit_max_moves(s, u, self.0.movement)
            .map_err(|e| error("max moves", e))
    }
    fn is_barbarian(&self, _: &LoadedState, u: &Unit) -> Result<bool, Error> {
        self.0
            .civilization_levels
            .iter()
            .find(|r| r.0 == u.e_owner)
            .map(|r| r.1 == 0xd959ec24)
            .ok_or_else(|| error("barbarian identity", "civilization level"))
    }
    fn barbarian_permitted(&self, _: &LoadedState, _: &Unit, _: i32) -> Result<bool, Error> {
        Err(error(
            "barbarian actor",
            "bribe/clan exclusion continuation",
        ))
    }
    fn clans_active(&self) -> Result<bool, Error> {
        Ok(self
            .0
            .capabilities
            .has(c6_core::game_capabilities::BARBARIAN_CLANS))
    }
    fn barbarian_camp_improvement(&self, state: &LoadedState) -> Result<u32, Error> {
        Ok(state.retained().barbarian.improvement_hash)
    }
}
struct Current<'a, 's, 'b, 'd, 'r, P> {
    dispatcher: &'a mut Dispatcher<'s, 'd, 'r>,
    backend: &'a mut Runtime<'b, P>,
    sight: &'a mut unit_sight_state::Runtime,
}
impl<P> Queries for Current<'_, '_, '_, '_, '_, P> {
    type Error = Error;
    fn is_combat(&self, s: &LoadedState, u: &Unit) -> Result<bool, Error> {
        Legality(self.dispatcher.inputs).is_combat(s, u)
    }
    fn at_war(&self, s: &LoadedState, o: i32, t: i32) -> Result<bool, Error> {
        Legality(self.dispatcher.inputs).at_war(s, o, t)
    }
    fn max_moves(&self, s: &LoadedState, u: &Unit) -> Result<i32, Error> {
        Legality(self.dispatcher.inputs).max_moves(s, u)
    }
    fn is_barbarian(&self, s: &LoadedState, u: &Unit) -> Result<bool, Error> {
        Legality(self.dispatcher.inputs).is_barbarian(s, u)
    }
    fn barbarian_permitted(&self, s: &LoadedState, u: &Unit, o: i32) -> Result<bool, Error> {
        Legality(self.dispatcher.inputs).barbarian_permitted(s, u, o)
    }
    fn clans_active(&self) -> Result<bool, Error> {
        Legality(self.dispatcher.inputs).clans_active()
    }
    fn barbarian_camp_improvement(&self, state: &LoadedState) -> Result<u32, Error> {
        Legality(self.dispatcher.inputs).barbarian_camp_improvement(state)
    }
}
impl<P> Effects for Current<'_, '_, '_, '_, '_, P> {
    fn observer_can_see_owner(&self, s: &LoadedState, o: i32) -> Result<bool, Error> {
        let observer = self
            .sight
            .local_observer
            .ok_or_else(|| error("observer", "missing"))?;
        if observer != s.retained().core.local_player && observer != 1000 {
            return Err(error("observer", "changed observer"));
        }
        Ok(crate::melee_attack::experience::reporting::observer_can_see_player(observer, o))
    }
    fn local_unit_visible(&self, s: &LoadedState, a: UnitReference) -> Result<bool, Error> {
        let u = s
            .world()
            .unit(a)
            .map_err(|e| error("visible actor", e))?
            .ok_or_else(|| error("visible actor", "missing"))?
            .record;
        let o = self
            .sight
            .local_observer
            .ok_or_else(|| error("observer", "missing"))?;
        let location = u.k_location;
        city_range_attack::unit_visible_with_counts::<actions::Error>(
            s,
            u,
            o,
            self.dispatcher.inputs.sight_definitions,
            |hash, normal| {
                let layers = self.sight.shared(o).map_err(actions::Error::Unsupported)?;
                let Some(l) = layers.0.iter().find(|l| l.type_hash == hash) else {
                    return if normal {
                        Err(actions::Error::Unsupported("Pillage normal visibility"))
                    } else {
                        Ok(0)
                    };
                };
                let i = l
                    .grid_width
                    .wrapping_mul(location.y as u32)
                    .wrapping_add(location.x as u32) as usize;
                Ok(l.values.get(i).copied().unwrap_or(0))
            },
            || {
                self.sight
                    .hidden_layer_hashes
                    .first()
                    .copied()
                    .ok_or(actions::Error::Unsupported("Pillage hidden layer0"))
            },
        )
        .map_err(|e| error("local visibility", e))
    }
    fn report(&mut self, s: &mut LoadedState, r: Report) -> Result<(), Error> {
        if let Report::Operation { unit, improvement } = r {
            crate::loaded_upkeep::OperationReports::operation_report(
                self.backend,
                s,
                actions::IdentityReport {
                    event: 0x3d4006f2,
                    subtype: super::OPERATION_HASH,
                    identity: unit,
                    first: Some(improvement),
                    second: None,
                },
            )
            .map_err(|e| error("operation report", e))?;
        } else if !self.dispatcher.inputs.presentation_reports_qualified {
            return Err(error("Pillage reports", "unqualified current recipients"));
        }
        self.dispatcher.reports.push(Output::Operation(r));
        Ok(())
    }
    fn improvement_pillaged(&mut self, s: &mut LoadedState, p: usize, h: u32) -> Result<(), Error> {
        stock::Current {
            backend: self.backend,
            inputs: self.dispatcher.inputs.callbacks,
            capabilities: self.dispatcher.inputs.capabilities,
            gossip_history: self.dispatcher.gossip_history,
        }
        .improvement_pillaged(s, p, h)
        .map_err(|e| error("ImprovementPillaged", e))
    }
    fn improvement_notification(
        &mut self,
        s: &mut LoadedState,
        p: usize,
        _: i32,
    ) -> Result<(), Error> {
        use actions::notifications::processing::Backend;
        if !Backend::manager_initialized(self.backend, s)
            .map_err(|e| error("notification initialized", e))?
            || Backend::game_uninitializing(self.backend, s)
                .map_err(|e| error("notification shutdown", e))?
        {
            return Ok(());
        }
        let owner = s.world().map().plots[p]
            .city_attributes
            .as_ref()
            .ok_or_else(|| error("notification owner", "missing"))?
            .player;
        if s.player_slot_status(i32::from(owner))
            .map_err(|e| error("notification human", e))?
            != 3
        {
            return Ok(());
        }
        Err(error(
            "Pillaged notification",
            "human recipient allocation/send continuation",
        ))
    }
    fn visibility_messages_disabled(&self) -> bool {
        self.sight.messages_disabled
    }
    fn update_appeal(&mut self, s: &mut LoadedState, p: usize) -> Result<(), Error> {
        let (x, y) = s
            .world()
            .map()
            .coordinates(p as u32)
            .ok_or_else(|| error("appeal plot", "coordinates"))?;
        crate::city_production_turn::appeal::update(s, [x, y], self.dispatcher.inputs.appeal, self)
            .map(|_| ())
            .map_err(|e| error("appeal update", e))
    }
    fn on_pillage(
        &mut self,
        s: &mut LoadedState,
        a: ComponentId,
        p: usize,
        h: u32,
    ) -> Result<(), Error> {
        stock::Current {
            backend: self.backend,
            inputs: self.dispatcher.inputs.callbacks,
            capabilities: self.dispatcher.inputs.capabilities,
            gossip_history: self.dispatcher.gossip_history,
        }
        .on_pillage(s, a, p, h)
        .map_err(|e| error("OnPillage", e))
    }
    fn post_pillage_effects(
        &mut self,
        s: &mut LoadedState,
        a: UnitReference,
        p: usize,
    ) -> Result<(), Error> {
        let d = self.dispatcher.inputs;
        plunder::Current {
            definitions: d.plunder,
            inputs: plunder::Inputs {
                civilization_levels: d.civilization_levels,
                cached_majors: d.cached_majors,
            },
            services: self,
        }
        .post_pillage_effects(s, a, p)
        .map_err(|e| error("post-pillage", e))
    }
    fn change_moves(
        &mut self,
        s: &mut LoadedState,
        a: UnitReference,
        delta: i32,
    ) -> Result<(), Error> {
        crate::loaded_upkeep::change_unit_moves(
            s,
            a.player(),
            a.id,
            self.dispatcher.inputs.movement,
            self.backend,
            delta,
        )
        .map(|_| ())
        .map_err(|e| error("Pillage moves", e))
    }
    fn award_plunder(
        &mut self,
        s: &mut LoadedState,
        a: UnitReference,
        p: usize,
    ) -> Result<(), Error> {
        let d = self.dispatcher.inputs;
        plunder::Current {
            definitions: d.plunder,
            inputs: plunder::Inputs {
                civilization_levels: d.civilization_levels,
                cached_majors: d.cached_majors,
            },
            services: self,
        }
        .award_plunder(s, a, p)
        .map_err(|e| error("plunder", e))
    }
}
impl<P> crate::city_production_turn::appeal::Effects for Current<'_, '_, '_, '_, '_, P> {
    type Error = Error;
    fn appeal_changed(&mut self, _: &mut LoadedState, p: u32, v: i32) -> Result<(), Error> {
        match self.dispatcher.inputs.appeal_subscribers {
            Some([]) => {
                self.dispatcher
                    .reports
                    .push(Output::Appeal { plot: p, value: v });
                Ok(())
            }
            _ => Err(error(
                "appeal changed",
                "current positive/unknown subscriptions",
            )),
        }
    }
    fn appeal_report(&mut self, _: &mut LoadedState, p: u32) -> Result<(), Error> {
        if !self.dispatcher.inputs.presentation_reports_qualified {
            return Err(error("appeal report", "unqualified recipients"));
        }
        self.dispatcher.reports.push(Output::AppealReport(p));
        Ok(())
    }
}
impl<P> plunder::Services for Current<'_, '_, '_, '_, '_, P> {
    type Error = Error;
    fn scaled_improvement_plunder(
        &mut self,
        _: &LoadedState,
        _: UnitReference,
        _: i32,
    ) -> Result<i32, Error> {
        Err(error(
            "non-healing plunder",
            "economic cost escalation/speed continuation",
        ))
    }
    fn change_damage(
        &mut self,
        s: &mut LoadedState,
        a: UnitReference,
        delta: i32,
    ) -> Result<(), Error> {
        crate::unit_placement_state::change_damage(
            s,
            self.dispatcher.inputs.placement,
            self.dispatcher.inputs.queries,
            self.sight,
            self.backend,
            (a.player(), a.id),
            delta,
        )
        .map(|_| ())
        .map_err(|e| error("Pillage damage", e))
    }
    fn change_yield(&mut self, _: &mut LoadedState, _: i32, _: i32, _: i32) -> Result<(), Error> {
        Err(error(
            "Pillage yield",
            "non-healing/religion award continuation",
        ))
    }
    fn change_loyalty(
        &mut self,
        s: &mut LoadedState,
        c: (i32, u32),
        delta: i32,
    ) -> Result<(), Error> {
        crate::melee_attack::resolve::post_kill::change_loyalty(
            s,
            self.sight,
            self.backend,
            c,
            delta,
            self.dispatcher.inputs.loyalty_maximum,
        )
        .map_err(|e| error("Pillage loyalty", e))
    }
    fn report(&mut self, _: &mut LoadedState, r: plunder::Report) -> Result<(), Error> {
        if matches!(r, plunder::Report::HealSignal { .. }) {
            return Err(error(
                "Pillage heal signal",
                "OnHealFromPillage continuation",
            ));
        }
        if !self.dispatcher.inputs.presentation_reports_qualified {
            return Err(error("plunder report", "unqualified recipients"));
        }
        self.dispatcher.reports.push(Output::Plunder(r));
        Ok(())
    }
}
fn reference(a: (i32, u32)) -> UnitReference {
    UnitReference::from_component_id(ComponentId {
        owner: a.0 as i16,
        component_type: 1,
        id: a.1,
    })
}
impl<P> move_to::Movement<Runtime<'_, P>> for Dispatcher<'_, '_, '_> {
    type Error = Error;
    fn pillage_can_start(
        &mut self,
        s: &LoadedState,
        u: &Unit,
        visible: bool,
        _: &Runtime<'_, P>,
    ) -> Result<Option<bool>, Error> {
        super::can_start(
            s,
            self.inputs.definitions,
            u,
            visible,
            &Legality(self.inputs),
        )
        .map(Some)
        .map_err(|e| error("CanStart", e))
    }
    fn start_pillage(
        &mut self,
        s: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        a: (i32, u32),
        n: &OperationNode,
        b: &mut Runtime<'_, P>,
    ) -> Result<Option<c6_core::skip_turn::StartData>, Error> {
        let defs = self.inputs.definitions;
        super::start(
            s,
            defs,
            reference(a),
            n,
            &mut Current {
                dispatcher: self,
                backend: b,
                sight,
            },
        )
        .map(Some)
        .map_err(|e| error("Start", e))
    }
    fn fortify_turn_max(&self) -> i32 {
        self.inputs.fortify_turn_max
    }
    fn is_fortifyable(&self, _: &LoadedState, _: (i32, u32)) -> Result<bool, Error> {
        Err(error("queued handler", "Fortify"))
    }
    fn alert_enemy_visible(
        &self,
        _: &LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
    ) -> Result<bool, Error> {
        Err(error("queued handler", "Alert"))
    }
    fn city_at(
        &self,
        _: &LoadedState,
        _: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, Error> {
        Err(error("queued handler", "Trade city"))
    }
    fn preflight(&self, _: &LoadedState, _: (i32, u32), _: move_to::Request) -> Result<(), Error> {
        Err(error("queued handler", "MoveTo"))
    }
    fn test_move_to(
        &mut self,
        _: &mut LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
        _: u32,
    ) -> Result<i32, Error> {
        Err(error("queued handler", "TestMoveTo"))
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
        Err(error("queued handler", "MoveTo"))
    }
    fn valid_location(
        &mut self,
        _: &LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
    ) -> Result<i32, Error> {
        Err(error("queued handler", "IsValidLocation"))
    }
}
