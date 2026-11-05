//! Qualified ordinary ranged combat services on the persistent stock owner.
//! Current gameplay callbacks execute in source order. Presentation statuses,
//! XP caches and platform achievements become typed output in this stock scope.
use super::{movement::Error, unit_creation::ModifierOwners, Runtime};
use crate::{loaded_upkeep, melee_attack as melee, ranged_attack, unit_sight_state as sight};
use c6_core::{
    loaded_state::{ActionStamp, LoadedState},
    map::UnitReference,
    unit::PlotCoord,
};
use melee::{backend, experience, resolve};

#[derive(Clone, Copy, Debug)]
pub enum Scope {
    /// Qualified stock XP/status/death routes with no added synchronous
    /// consumers. Native output caches and platform achievements are omitted.
    StockHeadlessNoAddedConsumers,
}
pub trait PromotionQuery {
    fn available(&self, state: &LoadedState, unit: UnitReference) -> Result<bool, String>;
}
impl PromotionQuery for crate::formation_movement_cost::Definitions<'_> {
    fn available(&self, state: &LoadedState, unit: UnitReference) -> Result<bool, String> {
        experience::reporting::promotion_available_after_change(state, self, unit)
            .map_err(|e| format!("{e:?}"))
    }
}
/// Immutable combat inputs and retained callback rings. Mutable modifier stores
/// remain in Runtime::modifier_owners, shared with unit/building creation.
pub struct State<'a> {
    origin: ActionStamp,
    inputs: Option<backend::Inputs>,
    bindings: Option<resolve::completion::stock::Bindings>,
    pub observer: Option<i32>,
    configuration: crate::loaded_actions::configuration::RawGameConfiguration,
    pub promotion: &'a dyn PromotionQuery,
    pub experience_events: Vec<experience::reporting::Event>,
    pub statuses: Vec<resolve::surviving::Status>,
    pub deaths: Vec<resolve::UnitDeath>,
    pub resolved: Vec<resolve::completion::ResolvedCombat>,
}
impl<'a> State<'a> {
    /// Split an initialized backend once; install returned stores in the SAME
    /// stock ModifierOwners, alongside the current collection/factory owners.
    pub fn from_backend(
        backend: backend::Backend<(), backend::requirements::Runtime>,
        bindings: resolve::completion::stock::Bindings,
        observer: Option<i32>,
        promotion: &'a dyn PromotionQuery,
        _: Scope,
    ) -> (Self, backend::requirements::Runtime, backend::EffectRuntime) {
        let origin = backend.origin();
        let ((), requirements, effects, inputs) = backend.into_parts();
        let configuration = inputs.configuration;
        (
            Self {
                configuration,
                origin,
                inputs: Some(inputs),
                bindings: Some(bindings),
                observer,
                promotion,
                experience_events: Vec::new(),
                statuses: Vec::new(),
                deaths: Vec::new(),
                resolved: Vec::new(),
            },
            requirements,
            effects,
        )
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_combat(&mut self, state: &LoadedState, combat: State<'a>) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("combat World"))?;
        self.modifier_owners()?;
        if self.combat.is_some() || !combat.origin.same_instance(state.action_stamp()) {
            return Err(Error::Missing("combat already bound or different World"));
        }
        self.combat = Some(combat);
        Ok(())
    }
    pub fn combat(&self) -> Result<&State<'a>, Error> {
        self.combat
            .as_ref()
            .ok_or(Error::Missing("current combat services"))
    }
    #[allow(clippy::too_many_arguments)]
    /// Loan the SAME mutable modifier stores for one synchronous attack, then
    /// restore them, including every completed prefix when resolution fails.
    pub fn ranged_attack<D>(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        damage: &mut D,
        inputs: &melee::transition::Inputs<'_, '_, '_>,
        actor: UnitReference,
        destination: PlotCoord,
    ) -> Result<bool, ranged_attack::Error>
    where
        D: ranged_attack::Damage<Self, backend::requirements::Runtime>,
    {
        let missing = |detail| ranged_attack::Error {
            stage: "stock owners",
            detail: format!("{detail:?}"),
        };
        let mut combat = self
            .combat
            .take()
            .ok_or_else(|| missing("combat services"))?;
        let Some(owners) = self.modifier_owners.take() else {
            self.combat = Some(combat);
            return Err(missing("canonical modifier owners"));
        };
        let ModifierOwners {
            effects,
            requirements,
            collections,
            unit_created,
        } = owners;
        let backend_inputs = combat
            .inputs
            .take()
            .expect("combat inputs retained outside synchronous loan");
        let backend = backend::Backend::resume_owned(
            state,
            combat.origin,
            (),
            requirements,
            effects,
            backend_inputs,
        );
        let (backend, result) = match backend {
            Err(backend) => (backend, Err(missing("different combat World"))),
            Ok(backend) => {
                let ((), mut session) = ranged_attack::Session::from_backend(state, backend);
                let mut bindings = combat.bindings.take().expect("combat bindings retained");
                self.combat = Some(combat);
                let result = session.execute_with_damage(
                    state,
                    sight,
                    self,
                    damage,
                    inputs,
                    &mut bindings,
                    actor,
                    destination,
                );
                combat = self.combat.take().expect("combat outputs retained");
                combat.bindings = Some(bindings);
                let result = result.map(|resolved| {
                    if let Some(resolved) = resolved {
                        combat.resolved.push(resolved);
                        true
                    } else {
                        false
                    }
                });
                (
                    session
                        .into_backend()
                        .expect("ranged backend restored after Result"),
                    result,
                )
            }
        };
        let ((), requirements, effects, backend_inputs) = backend.into_parts();
        combat.inputs = Some(backend_inputs);
        self.modifier_owners = Some(ModifierOwners {
            effects,
            requirements,
            collections,
            unit_created,
        });
        self.combat = Some(combat);
        result
    }
}
impl<P> Runtime<'_, P> {
    fn combat_current(&self, state: &LoadedState) -> Result<&State<'_>, Error> {
        let combat = self.combat()?;
        if !combat.origin.same_instance(state.action_stamp()) {
            return Err(Error::Missing("combat output World"));
        }
        Ok(combat)
    }
}
impl<P> ranged_attack::Effects for Runtime<'_, P> {
    fn ranged_surviving_statuses(
        &mut self,
        state: &mut LoadedState,
        attacker: UnitReference,
        defender: UnitReference,
        damage: i32,
        status_id: u32,
    ) -> Result<(), Error> {
        let combat = self.combat_current(state)?;
        // Configuration lives in the loaned backend while resolving. Preserve
        // the explicit multiplayer gate using the current stock runtime facts.
        let configuration = combat.configuration;
        let _ = combat;
        let location = melee::actor(state, defender)
            .map_err(|e| Error::Requirement(format!("{e:?}")))?
            .k_location;
        self.combat
            .as_mut()
            .expect("combat output owner")
            .statuses
            .push(resolve::surviving::Status {
                attacker,
                defender,
                location,
                attacker_damage: 0,
                defender_damage: damage,
                status_id,
            });
        let required = resolve::surviving::combat_notification_required(
            state,
            &configuration,
            self.facts
                .manager_initialized
                .zip(self.facts.game_uninitializing),
            defender,
        )
        .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        if required {
            return Err(Error::Missing(
                "human multiplayer combat notification creation",
            ));
        }
        Ok(())
    }
}
impl<P> experience::Reports for Runtime<'_, P> {
    type Error = Error;
    fn observer_can_see_player(&mut self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        let observer = self
            .combat_current(state)?
            .observer
            .ok_or(Error::Missing("experience observer"))?;
        Ok(experience::reporting::observer_can_see_player(
            observer, owner,
        ))
    }
    fn gained(
        &mut self,
        state: &mut LoadedState,
        unit: UnitReference,
        amount: i32,
        promotable: bool,
    ) -> Result<(), Error> {
        self.combat_current(state)?;
        self.combat
            .as_mut()
            .expect("combat output owner")
            .experience_events
            .push(experience::reporting::Event::Gained {
                unit,
                amount,
                promotable,
            });
        Ok(())
    }
    fn changed(&mut self, state: &mut LoadedState, unit: UnitReference) -> Result<(), Error> {
        self.combat_current(state)?;
        let combat = self.combat.as_mut().expect("combat output owner");
        combat
            .experience_events
            .push(experience::reporting::Event::Changed { unit });
        if combat
            .promotion
            .available(state, unit)
            .map_err(Error::Requirement)?
        {
            return Err(Error::Missing("positive UnitPromoted publication"));
        }
        Ok(())
    }
}
impl<P> resolve::DeathReports for Runtime<'_, P> {
    fn unit_death_notifications(
        &mut self,
        state: &mut LoadedState,
        request: resolve::UnitDeath,
    ) -> Result<(), Error> {
        self.combat_current(state)?;
        self.combat
            .as_mut()
            .expect("combat output owner")
            .deaths
            .push(request);
        Ok(())
    }
}
impl<P> resolve::winning::Effects for Runtime<'_, P> {
    fn defender_death_notifications(
        &mut self,
        state: &mut LoadedState,
        request: resolve::UnitDeath,
    ) -> Result<(), Error> {
        resolve::DeathReports::unit_death_notifications(self, state, request)
    }
}
impl<P> resolve::kill_awards::Effects for Runtime<'_, P> {
    fn great_people_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: i32,
        _: i32,
        _: i32,
    ) -> Result<(), Error> {
        Err(Error::Missing("GreatPeoplePoints changed signal/report"))
    }
    fn kill_reward_feedback(
        &mut self,
        _: &mut LoadedState,
        _: UnitReference,
        _: PlotCoord,
        _: u32,
        _: i32,
    ) -> Result<(), Error> {
        Err(Error::Missing("positive kill reward publication"))
    }
}
impl<P> resolve::post_kill::Effects for Runtime<'_, P> {}
impl<P> backend::PostCombatYields for Runtime<'_, P> {
    fn credit_post_combat_yield(
        &mut self,
        _: &mut LoadedState,
        _: UnitReference,
        _: i32,
        _: i32,
    ) -> Result<(), Error> {
        Err(Error::Missing("positive post-combat yield credit"))
    }
}
impl<P> resolve::completion::stock::Effects for Runtime<'_, P> {
    type Error = Error;
    fn clear_defender_operations(
        &mut self,
        state: &mut LoadedState,
        _: &mut resolve::completion::stock::Bindings,
        unit: UnitReference,
    ) -> Result<(), Error> {
        loaded_upkeep::clear_unit_operations(state, unit.player(), unit.id, self)
            .map_err(|e| Error::Requirement(format!("{e:?}")))
    }
    fn set_defender_awake(
        &mut self,
        _: &mut LoadedState,
        _: &mut resolve::completion::stock::Bindings,
        _: UnitReference,
    ) -> Result<(), Error> {
        Err(Error::Missing("defender SetActivity continuation"))
    }
    fn lua_combat_occurred(
        &mut self,
        state: &mut LoadedState,
        _: &mut resolve::completion::stock::Bindings,
        members: &melee::execution::GeneratedMembers,
    ) -> Result<(), Error> {
        self.script
            .on_combat_occurred(
                state,
                Some(members.damage.attacker),
                Some(members.damage.defender),
            )
            .map_err(Error::Script)
    }
    fn trigger_boost(
        &mut self,
        _: &mut LoadedState,
        _: &mut resolve::completion::stock::Bindings,
        _: i32,
        _: usize,
    ) -> Result<(), Error> {
        Err(Error::Missing(
            "positive combat boost research continuation",
        ))
    }
    fn change_era_score(
        &mut self,
        _: &mut LoadedState,
        _: &mut resolve::completion::stock::Bindings,
        _: i32,
        _: i32,
        _: i32,
    ) -> Result<(), Error> {
        Err(Error::Missing("positive HistoricMoment score"))
    }
    fn era_score_notice(
        &mut self,
        _: &mut LoadedState,
        _: &mut resolve::completion::stock::Bindings,
        _: &c6_core::game_history::Moment,
    ) -> Result<(), Error> {
        Err(Error::Missing("positive HistoricMoment notice"))
    }
}
impl<P> Runtime<'_, P> {
    /// Reached stock combat publications; the Treasury and unit snapshot
    /// queries execute before omitting native cache/host presentation output.
    pub(super) fn deliver_combat_placement_report(
        &mut self,
        state: &LoadedState,
        report: c6_core::unit_placement_transition::PlacementReport,
    ) -> Result<(), Error> {
        use c6_core::unit_placement_transition::PlacementPayload;
        self.combat_current(state)?;
        let valid = match report.event_hash {
            0x35597c21 | 0x7274f04b => matches!(report.payload, PlacementPayload::Integer(_)),
            0x754cbd1b => matches!(report.payload, PlacementPayload::OtherIdentity(_)),
            _ => false,
        };
        if !valid {
            return Err(Error::UnsupportedPlacementReport(report.event_hash));
        }
        if report.event_hash == 0x754cbd1b {
            crate::loaded_actions::reporting::yield_monitor::warm_route_domain(state)
                .map_err(|e| Error::Requirement(format!("combat Treasury routes {e:?}")))?;
            let player = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == i32::from(report.identity.owner))
                .ok_or(Error::Missing("combat Treasury Player"))?;
            if !player.treasury.gold_diplomatic_deals.is_empty() {
                return Err(Error::Missing("combat Treasury deal queries"));
            }
        } else {
            self.reporting
                .observe_unit_snapshot(state, UnitReference::from_component_id(report.identity))
                .map_err(Error::Rules)?;
        }
        Ok(())
    }
}
