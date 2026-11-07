//! Qualified combat and Unit lifecycle services on the persistent stock owner.
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
/// Qualification for the reached manual `NOTIFICATION_UNIT_ATTACKED` output.
/// This does not qualify arbitrary notification types or custom subscribers.
#[derive(Clone, Copy, Debug)]
pub enum CombatNoticeScope {
    /// Actual stock default factory/manager, plain owned resources, stock old
    /// supersession callbacks, and NotificationAdded with no native gameplay
    /// consumers or synchronous host hooks. Native IDs, pools, text and queued
    /// presentation are omitted; ordered semantic combat statuses are retained.
    StockPlainHeadlessNoAddedConsumers,
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
    pub(super) origin: ActionStamp,
    pub melee_queries: Option<&'a dyn melee::stock::MovementQueries>,
    pub(super) inputs: Option<backend::Inputs>,
    pub(super) bindings: Option<resolve::completion::stock::Bindings>,
    pub(super) capture_owners: Option<super::capture_owners::CombatLifecycleOwners>,
    pub observer: Option<i32>,
    pub(crate) configuration: Option<crate::loaded_actions::configuration::RawGameConfiguration>,
    pub promotion: &'a dyn PromotionQuery,
    pub experience_events: Vec<experience::reporting::Event>,
    pub statuses: Vec<resolve::surviving::Status>,
    combat_notice_scope: Option<CombatNoticeScope>,
    pub deaths: Vec<resolve::UnitDeath>,
    pub resolved: Vec<resolve::completion::ResolvedCombat>,
    pub city_melees: Vec<melee::city::resolve::Resolved>,
    pub city_ranges: Vec<ranged_attack::city::Resolved>,
    pub city_melee_reports: Vec<melee::city::Publication>,
    pub city_melee_report_scope: Option<melee::city::ReportScope>,
    pub city_strikes: Vec<crate::city_strike::Generated>,
    pub city_strike_reports: Vec<crate::city_strike::reports::Output>,
    pub city_strike_report_scope: Option<crate::city_strike::reports::Scope>,
}
impl<'a> State<'a> {
    /// Qualify the manual defender notice reached by surviving Unit combat.
    /// Callers must establish the actual factory, owned-resource and callback
    /// bindings for this input and preserve them while using the combat owner.
    /// Constructors leave this absent, retaining the explicit unsupported path.
    pub fn with_combat_notice_scope(mut self, scope: CombatNoticeScope) -> Self {
        self.combat_notice_scope = Some(scope);
        self
    }

    /// Semantic observations emitted by the reached stock combat callbacks.
    pub fn completion_outputs(&self) -> Option<&[resolve::completion::stock::Output]> {
        self.bindings.as_ref().map(|bindings| bindings.outputs())
    }

    /// Qualified Unit lifecycle/report callbacks without attack execution.
    /// Mutable effects and requirements stay in the existing ModifierOwners.
    /// Attack inputs, completion bindings and configuration remain unavailable;
    /// requesting a melee or ranged attack returns an explicit error.
    pub fn for_unit_cleanup(
        state: &LoadedState,
        observer: Option<i32>,
        promotion: &'a dyn PromotionQuery,
        _: Scope,
    ) -> Self {
        Self {
            origin: state.action_stamp(),
            melee_queries: None,
            inputs: None,
            bindings: None,
            capture_owners: None,
            observer,
            configuration: None,
            promotion,
            experience_events: Vec::new(),
            statuses: Vec::new(),
            combat_notice_scope: None,
            deaths: Vec::new(),
            resolved: Vec::new(),
            city_melees: Vec::new(),
            city_ranges: Vec::new(),
            city_melee_reports: Vec::new(),
            city_melee_report_scope: None,
            city_strikes: Vec::new(),
            city_strike_reports: Vec::new(),
            city_strike_report_scope: None,
        }
    }
    /// Bind attack inputs and callback rings while retaining the caller's existing
    /// canonical modifier owners. The caller must install those owners on Runtime
    /// before binding this State; no mutable gameplay stores are constructed here.
    pub fn for_attack(
        state: &LoadedState,
        inputs: backend::Inputs,
        bindings: resolve::completion::stock::Bindings,
        observer: Option<i32>,
        promotion: &'a dyn PromotionQuery,
        scope: Scope,
    ) -> Self {
        let mut combat = Self::for_unit_cleanup(state, observer, promotion, scope);
        combat.configuration = Some(inputs.configuration);
        combat.inputs = Some(inputs);
        combat.bindings = Some(bindings);
        combat
    }
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
                configuration: Some(configuration),
                melee_queries: None,
                origin,
                inputs: Some(inputs),
                bindings: Some(bindings),
                capture_owners: None,
                observer,
                promotion,
                experience_events: Vec::new(),
                statuses: Vec::new(),
                combat_notice_scope: None,
                deaths: Vec::new(),
                resolved: Vec::new(),
                city_melees: Vec::new(),
                city_ranges: Vec::new(),
                city_melee_reports: Vec::new(),
                city_melee_report_scope: None,
                city_strikes: Vec::new(),
                city_strike_reports: Vec::new(),
                city_strike_report_scope: None,
            },
            requirements,
            effects,
        )
    }
}
impl<'a, P> Runtime<'a, P> {
    /// Same-owner semantic queue; native notification objects are not created.
    pub fn queue_city_disposition(
        &mut self,
        state: &LoadedState,
        choice: &crate::city_capture::PendingDisposition,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("capture decision World"))?;
        state
            .city(i32::from(choice.city.owner), choice.city.id)
            .map_err(|_| Error::Missing("capture decision City"))?;
        if choice.legal.is_empty()
            || self
                .capture_dispositions
                .iter()
                .any(|old| old.city == choice.city)
        {
            return Err(Error::Missing("unique legal capture decision"));
        }
        self.capture_dispositions.push(choice.clone());
        Ok(())
    }
    pub fn pending_city_dispositions(&self) -> &[crate::city_capture::PendingDisposition] {
        &self.capture_dispositions
    }

    /// Retain a typed completed decision when initializing a current snapshot.
    /// This is not a historical melee record. Normal captures still queue at
    /// their source site and become ready only after melee completion.
    pub fn initialize_completed_city_disposition(
        &mut self,
        state: &LoadedState,
        choice: &crate::city_capture::PendingDisposition,
        definitions: &crate::loaded_actions::city_disposition_actions::Definitions,
        levels: &[(i32, u32)],
    ) -> Result<(), Error> {
        if self
            .combat
            .as_ref()
            .is_some_and(|combat| combat.capture_owners.is_some())
        {
            return Err(Error::Missing("disposition initialization during capture"));
        }
        let current = crate::loaded_actions::city_disposition_actions::pending_from_current(
            state,
            choice.city,
            definitions,
            levels,
        )
        .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        if &current != choice {
            return Err(Error::Missing(
                "initialized disposition differs from current legal decision",
            ));
        }
        self.queue_city_disposition(state, choice)?;
        self.initialized_completed_dispositions.push(choice.city);
        Ok(())
    }
    pub(crate) fn completed_city_disposition(
        &self,
        choice: &crate::city_capture::PendingDisposition,
    ) -> bool {
        self.initialized_completed_dispositions
            .contains(&choice.city)
            || self.combat.as_ref().is_some_and(|combat| {
                combat.city_melees.iter().any(|row| {
                    row.captured
                        .as_ref()
                        .is_some_and(|capture| capture.disposition.city == choice.city)
                })
            })
    }

    /// Remove exactly the completed choice after its synchronous disposition
    /// callbacks and normal blocking refresh succeed. Combat history is retained.
    pub(crate) fn complete_city_disposition(
        &mut self,
        state: &LoadedState,
        choice: &crate::city_capture::PendingDisposition,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("disposition World"))?;
        let at = self
            .capture_dispositions
            .iter()
            .position(|current| current == choice)
            .ok_or(Error::Missing("current completed disposition choice"))?;
        self.capture_dispositions.remove(at);
        self.initialized_completed_dispositions
            .retain(|city| *city != choice.city);
        Ok(())
    }

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
    pub(crate) fn city_strike_notification_required(
        &self,
        state: &LoadedState,
        defender: UnitReference,
    ) -> Result<bool, Error> {
        let configuration = self
            .combat_current(state)?
            .configuration
            .ok_or(Error::Missing("combat configuration"))?;
        resolve::surviving::combat_notification_required(
            state,
            &configuration,
            self.facts
                .manager_initialized
                .zip(self.facts.game_uninitializing),
            defender,
        )
        .map_err(|e| Error::Requirement(format!("{e:?}")))
    }
    pub(crate) fn combat_mut(&mut self) -> Result<&mut State<'a>, Error> {
        self.combat
            .as_mut()
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
        let city_target = {
            let attacker = melee::actor(state, actor).map_err(|e| ranged_attack::Error {
                stage: "stock city target",
                detail: format!("{e:?}"),
            })?;
            let district = melee::target_selection::enemy_defensible_district_with_definitions(
                state,
                Some(inputs.generation.maximum_damage),
                attacker.e_owner,
                destination,
            )
            .map_err(|e| ranged_attack::Error {
                stage: "stock city target",
                detail: format!("{e:?}"),
            })?;
            if let Some(d) = district {
                let owner = crate::loaded_actions::city_range_attack::district_owner(state, d)
                    .map_err(|e| ranged_attack::Error {
                        stage: "stock city target owner",
                        detail: format!("{e:?}"),
                    })?;
                Some(c6_core::unit::ComponentId {
                    owner: owner as i16,
                    component_type: 3,
                    id: d.id,
                })
            } else {
                None
            }
        };
        let mut combat = self
            .combat
            .take()
            .ok_or_else(|| missing("combat services"))?;
        if combat.inputs.is_none() || combat.bindings.is_none() || combat.configuration.is_none() {
            self.combat = Some(combat);
            return Err(missing(
                "ranged action services unavailable in cleanup context",
            ));
        }
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
                if let Some(defender) = city_target {
                    let mut bindings = combat.bindings.take().expect("combat bindings retained");
                    self.combat = Some(combat);
                    let (_, mut backend) = backend.with_reporting(melee::transition::Reports(self));
                    let result = ranged_attack::city::execute(
                        state,
                        sight,
                        &mut backend,
                        inputs,
                        &mut bindings,
                        actor,
                        defender,
                    );
                    let (_, backend) = backend.with_reporting(());
                    combat = self.combat.take().expect("combat outputs retained");
                    combat.bindings = Some(bindings);
                    let result = result.map(|resolved| {
                        combat.city_ranges.push(resolved);
                        true
                    });
                    (backend, result)
                } else {
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
    #[allow(clippy::too_many_arguments)]
    /// Loan the SAME mutable modifier stores for one synchronous attack, then
    /// restore them, including every completed prefix when resolution fails.
    pub fn melee_attack<D>(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        damage: &mut D,
        inputs: &melee::transition::Inputs<'_, '_, '_>,
        actor: UnitReference,
        destination: PlotCoord,
    ) -> Result<bool, melee::transition::Error>
    where
        D: melee::transition::Damage<Self, backend::requirements::Runtime>,
    {
        let missing = |detail| melee::transition::Error {
            stage: melee::transition::Stage::Backend,
            detail: format!("{detail:?}"),
        };
        let mut combat = self
            .combat
            .take()
            .ok_or_else(|| missing("combat services"))?;
        if combat.inputs.is_none()
            || combat.bindings.is_none()
            || combat.configuration.is_none()
            || combat.capture_owners.is_some()
        {
            self.combat = Some(combat);
            return Err(missing(
                "melee action services unavailable in cleanup context",
            ));
        }
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
        combat.capture_owners = Some(super::capture_owners::CombatLifecycleOwners {
            collections,
            unit_created,
        });
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
                let ((), mut session) = melee::transition::Session::from_backend(state, backend);
                let mut bindings = combat.bindings.take().expect("combat bindings retained");
                self.combat = Some(combat);
                let result = session.execute(
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
                let result = result.map(|resolved| match resolved {
                    melee::transition::Outcome::Resolved(resolved) => {
                        combat.resolved.push(resolved);
                        true
                    }
                    melee::transition::Outcome::CityResolved(resolved) => {
                        combat.city_melees.push(resolved);
                        true
                    }
                    melee::transition::Outcome::NoTarget => false,
                });
                (
                    session
                        .into_backend()
                        .expect("melee backend restored after Result"),
                    result,
                )
            }
        };
        let ((), requirements, effects, backend_inputs) = backend.into_parts();
        combat.inputs = Some(backend_inputs);
        let super::capture_owners::CombatLifecycleOwners {
            collections,
            unit_created,
        } = combat
            .capture_owners
            .take()
            .expect("combat lifecycle owners retained across Result");
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
        let configuration = combat.configuration.ok_or(Error::Missing(
            "combat configuration unavailable in cleanup context",
        ))?;
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
        if required && self.combat_current(state)?.combat_notice_scope.is_none() {
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
        state: &mut LoadedState,
        _: &mut resolve::completion::stock::Bindings,
        owner: i32,
        definition: usize,
    ) -> Result<(), Error> {
        use resolve::completion::boosts::{ResearchTrigger, Trigger};
        let inputs = self
            .unit_creation
            .as_ref()
            .and_then(|input| input.research)
            .ok_or(Error::Missing("combat canonical research definitions"))?;
        let mut runtime = self
            .action_research
            .take()
            .ok_or(Error::Missing("combat canonical research loan"))?;
        let result = {
            let mut effects =
                super::research::Effects::new(self, &mut runtime, inputs.choices, inputs.culture);
            ResearchTrigger {
                definitions: inputs.definitions,
                effects: &mut effects,
            }
            .trigger(state, owner, definition)
            .map_err(|error| Error::Requirement(format!("combat boost: {error:?}")))
        };
        self.action_research = Some(runtime);
        result
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

impl<P> resolve::surviving::Effects for Runtime<'_, P> {
    fn both_alive_statuses(
        &mut self,
        state: &mut LoadedState,
        request: resolve::surviving::Status,
    ) -> Result<(), Error> {
        let configuration = self
            .combat_current(state)?
            .configuration
            .ok_or(Error::Missing("combat configuration"))?;
        self.combat
            .as_mut()
            .expect("combat output owner")
            .statuses
            .push(request);
        if resolve::surviving::combat_notification_required(
            state,
            &configuration,
            self.facts
                .manager_initialized
                .zip(self.facts.game_uninitializing),
            request.defender,
        )
        .map_err(|e| Error::Requirement(format!("{e:?}")))?
            && self.combat_current(state)?.combat_notice_scope.is_none()
        {
            return Err(Error::Missing(
                "human multiplayer combat notification creation",
            ));
        }
        Ok(())
    }
    fn combat_movement_cost(
        &mut self,
        state: &LoadedState,
        sight: &sight::Runtime,
        attacker: UnitReference,
        location: PlotCoord,
    ) -> Result<i32, Error> {
        self.combat_current(state)?
            .melee_queries
            .ok_or(Error::Missing("melee movement queries"))?
            .cost(state, sight, attacker, location)
    }
    fn visible_enemy_at(
        &mut self,
        state: &LoadedState,
        sight: &sight::Runtime,
        attacker: UnitReference,
        location: PlotCoord,
    ) -> Result<(bool, bool), Error> {
        self.combat_current(state)?
            .melee_queries
            .ok_or(Error::Missing("melee movement queries"))?
            .visible(state, sight, attacker, location)
    }
    fn enemy_defensible_district(
        &mut self,
        state: &LoadedState,
        attacker: UnitReference,
        location: PlotCoord,
    ) -> Result<bool, Error> {
        self.combat_current(state)?
            .melee_queries
            .ok_or(Error::Missing("melee movement queries"))?
            .district(state, attacker, location)
    }
}

impl<P> resolve::completion::stock::UnitDistrictEffects for Runtime<'_, P> {
    fn lua_unit_district_combat_occurred(
        &mut self,
        state: &mut LoadedState,
        _: &mut resolve::completion::stock::Bindings,
        members: &melee::city::Generated,
    ) -> Result<(), Error> {
        self.script
            .on_combat_occurred(state, Some(members.attacker), None)
            .map_err(Error::Script)
    }
}
impl<P> melee::city::resolve::Effects for Runtime<'_, P> {
    fn city_melee_publication(
        &mut self,
        state: &mut LoadedState,
        publication: melee::city::Publication,
    ) -> Result<(), String> {
        let current = self.combat_current(state).map_err(|e| format!("{e:?}"))?;
        if !current
            .city_melee_report_scope
            .as_ref()
            .is_some_and(|s| s.matches(state))
        {
            return Err("unqualified city melee publication routes".into());
        }
        self.combat
            .as_mut()
            .expect("combat output owner")
            .city_melee_reports
            .push(publication);
        Ok(())
    }
}
