//! Persistent stock notification services for headless action aftermath.
//!
//! Decisions and gameplay state remain authoritative. Native UI pool addresses,
//! display IDs and localization are not reconstructed. The supported plain-value
//! domain has no injected deleters, text consumers or presentation subscribers.
use super::{
    current_aftermath,
    notifications::{self, lifecycle, processing, Created},
    reporting,
    reporting::yield_monitor::WarmRouteDomain,
    stock_script,
};
#[path = "stock_activation.rs"]
mod activation;
mod capture_owners;
pub mod city_production;
pub mod city_queries;
#[path = "stock_city_strike.rs"]
pub mod city_strike;
pub mod cityless_managers;
#[path = "stock_combat.rs"]
pub mod combat;
pub mod deactivation;
pub mod era_changes;
pub mod founding;
#[path = "stock_notifications_game_update.rs"]
pub mod game_update;
#[path = "stock_gold.rs"]
pub mod gold;
pub mod growth;
pub mod human_activation;
pub mod inactive_control;
#[path = "stock_maintenance.rs"]
pub mod maintenance;
#[path = "stock_manager_queries.rs"]
mod manager_queries;
#[path = "stock_movement.rs"]
pub mod movement;
pub mod npc_activation;
pub mod npc_deactivation;
pub mod npc_leaves;
pub mod npc_pantheon;
pub mod npc_unit_decisions;
pub mod npc_unit_operations;
pub mod policy_modifiers;
pub mod queued_operations;
#[path = "stock_research.rs"]
pub mod research;
#[path = "stock_resource_revelation.rs"]
pub mod resource_revelation;
pub mod round_ai;
pub mod round_backend;
pub mod round_dispatch;
pub mod round_end;
pub mod round_turn;
pub mod round_world;
#[path = "stock_scoring.rs"]
pub mod scoring;
#[path = "stock_sight_discovery.rs"]
pub mod sight_discovery;
#[path = "stock_skip_turn.rs"]
pub mod skip_turn;
pub mod social;
#[path = "stock_turn.rs"]
pub mod turn;
pub mod turn_loop;
#[path = "stock_turn_reports.rs"]
pub mod turn_reports;
pub mod unit_creation;
use c6_core::{
    core_player::initialization::WireBinding,
    diplomacy_manager::{ProcessStatement, StatementHandle},
    loaded_state::LoadedState,
    notifications::ComponentId,
    player_notifications_owner::ObjectId,
    unit_operation_aftermath,
};

#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockPlainHeadlessNoAddedConsumers,
}
/// Current runtime facts, kept separate from saved transport values. Missing
/// values fail only when their source query is reached.
pub struct Facts {
    pub manager_initialized: Option<bool>,
    pub game_uninitializing: Option<bool>,
    pub local_player: Option<i32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    pub owner: i32,
    pub type_hash: u32,
    pub location: [i32; 2],
    pub component: Option<ComponentId>,
}
/// Informational gameplay output; it never requests a policy choice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notice {
    ImprovementPillaged {
        owner: i32,
        actor_owner: i32,
        location: [i32; 2],
        improvement: u32,
        city: u32,
    },
    TechnologyCompleted {
        owner: i32,
        technology: i32,
    },
    CivicCompleted {
        owner: i32,
        civic: i32,
    },
    TechnologyBoosted {
        owner: i32,
        technology: i32,
        old_progress: i32,
        source: i32,
        percent: i32,
    },
    CivicBoosted {
        owner: i32,
        civic: i32,
        old_progress: i32,
        source: i32,
        percent: i32,
    },
}
pub struct Runtime<'a, P = c6_core::unit::OperationParameters> {
    pub script: stock_script::Runtime<P>,
    growth: Option<growth::Inputs>,
    social_capabilities: Option<c6_core::game_capabilities::Membership>,
    era_changes: Option<era_changes::State>,
    queued_operations: Option<Box<dyn queued_operations::Services<P> + 'a>>,
    pantheon_definitions: Option<super::pantheon_actions::Definitions>,
    pending_pantheons: Vec<(i32, bool)>,
    npc_unit_decisions: Option<Box<dyn npc_unit_decisions::Provider<P> + 'a>>,
    pub reporting: reporting::Provider<'a>,
    pub decisions: Vec<Decision>,
    pub notices: Vec<Notice>,
    acquisition_policies: crate::research::policy::AcquisitionPolicies,
    /// Ordered emitted notification publications; client UI playback is omitted.
    pub publications: Vec<lifecycle::Report>,
    /// Source-ordered changes after the canonical Player blocking store.
    pub blocking_changes: Vec<(u32, i32, i32)>,
    pub turn_report_records: Vec<turn_reports::Record>,
    pub last_error: Option<String>,
    pub reached: Vec<&'static str>,
    npc_deactivation: Vec<npc_deactivation::State>,
    round_ai: Vec<round_ai::Inputs>,
    binding: WireBinding,
    facts: Facts,
    temporary_serial: u64,
    positive_temporaries: Vec<c6_core::notification_lifecycle::ObjectId>,
    player_services: Option<crate::unit_placement_services::Runtime>,
    unit_creation: Option<unit_creation::State<'a>>,
    city_placement: Option<unit_creation::PlacementInputs<'a>>,
    modifier_owners: Option<unit_creation::ModifierOwners>,
    unit_production_signals: Option<city_production::unit::Signals>,
    movement: Option<movement::State<'a>>,
    combat: Option<combat::State<'a>>,
    capture_dispositions: Vec<crate::city_capture::PendingDisposition>,
    initialized_completed_dispositions: Vec<c6_core::unit::ComponentId>,
    resource_revelation: Option<resource_revelation::State>,
    city_queries: Option<city_queries::Inputs<'a>>,
    gold_signals: Option<gold::Signals>,
    pub(super) research_acquisition: Option<research::acquisition::Binding>,
    pub(super) research_unit_updates: Option<&'a crate::unit_movement_state::Definitions>,
    pub(super) research_queries: Option<research::Queries<'a>>,
    pub(super) research_boosts: Option<research::boosts::Inputs<'a>>,
    pub(crate) turn_progress: Option<turn_loop::Progress>,
    pub(crate) completion_inflight: Option<i32>,
    round_unit_healed: Option<round_end::UnitHealedSignals>,
    round_services: Option<Box<dyn round_dispatch::Services<P> + 'a>>,
    turn_messages: std::collections::VecDeque<round_dispatch::TurnMessage>,
    pub(crate) activation_progress:
        Option<crate::game_update_state::activation::upkeep::ActivationProgress>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn new(
        state: &LoadedState,
        script: stock_script::Runtime<P>,
        reporting: reporting::Provider<'a>,
        facts: Facts,
        _: Scope,
    ) -> Result<Self, processing::Error> {
        if !script.script().matches(state) {
            return Err(processing::Error::Unsupported(
                "stock aftermath script identity",
            ));
        }
        Ok(Self {
            script,
            reporting,
            decisions: Vec::new(),
            notices: Vec::new(),
            acquisition_policies: Default::default(),
            publications: Vec::new(),
            blocking_changes: Vec::new(),
            turn_report_records: Vec::new(),
            last_error: None,
            reached: Vec::new(),
            npc_deactivation: Vec::new(),
            round_ai: Vec::new(),
            binding: state.binding().clone(),
            facts,
            temporary_serial: 0,
            positive_temporaries: Vec::new(),
            player_services: None,
            unit_creation: None,
            city_placement: None,
            modifier_owners: None,
            growth: None,
            social_capabilities: None,
            era_changes: None,
            unit_production_signals: None,
            movement: None,
            combat: None,
            capture_dispositions: Vec::new(),
            initialized_completed_dispositions: Vec::new(),
            resource_revelation: None,
            city_queries: None,
            gold_signals: None,
            research_acquisition: None,
            research_unit_updates: None,
            research_queries: None,
            research_boosts: None,
            activation_progress: None,
            turn_progress: None,
            completion_inflight: None,
            round_unit_healed: None,
            round_services: None,
            turn_messages: std::collections::VecDeque::new(),
            queued_operations: None,
            npc_unit_decisions: None,
            pantheon_definitions: None,
            pending_pantheons: Vec::new(),
        })
    }
    /// Semantic stock informational publication. The source caller has run its
    /// gameplay predicates and qualified the plain stock factory/consumer path.
    /// Native text, UI occurrence pools and display cache identity are omitted.
    pub fn publish_notice(
        &mut self,
        state: &LoadedState,
        notice: Notice,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        let owner = match &notice {
            Notice::ImprovementPillaged { owner, .. }
            | Notice::TechnologyCompleted { owner, .. }
            | Notice::CivicCompleted { owner, .. }
            | Notice::CivicBoosted { owner, .. }
            | Notice::TechnologyBoosted { owner, .. } => *owner,
        };
        if !self
            .facts
            .manager_initialized
            .ok_or(processing::Error::Unsupported(
                "notice manager initialization",
            ))?
        {
            return Ok(());
        }
        if self
            .facts
            .game_uninitializing
            .ok_or(processing::Error::Unsupported("notice game uninitializing"))?
        {
            return Ok(());
        }
        if state.partial_player_controls(owner)?.slot_status != 3 {
            return Ok(());
        }
        self.notices.push(notice);
        Ok(())
    }
    /// City.BuildQueue.CheckProduction284f6e0: a live City with no front
    /// entry requests an external production choice. This publishes the
    /// semantic decision without constructing native notification/UI pools.
    pub fn publish_city_production(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        let current = state.city(owner, city)?;
        if current.header.delayed_death || !current.build_queue.entries.is_empty() {
            return Ok(());
        }
        let decision = Decision {
            owner: current.header.owner,
            type_hash: 0x40fb_81ed,
            location: [current.header.location_x, current.header.location_y],
            component: Some(ComponentId {
                owner: current
                    .header
                    .owner
                    .try_into()
                    .map_err(|_| processing::Error::Unsupported("City owner range"))?,
                component_type: 2,
                id: current.header.id,
            }),
        };
        research::insert_pending_decision(&mut self.decisions, decision);
        Ok(())
    }
    /// Publish a decision already established by the source gameplay caller
    /// (for example, completed research requiring the next technology). This
    /// does not choose a policy or create native UI objects. Callers still run
    /// their source-ordered predicates/getters before reaching this effect.
    pub fn publish_decision(
        &mut self,
        state: &LoadedState,
        owner: i32,
        type_hash: u32,
        location: [i32; 2],
        component: Option<ComponentId>,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        state
            .world()
            .player(owner)
            .map_err(|error| processing::Error::Gameplay(super::Error::World(error)))?;
        notifications::allocation::stock_auto_pool(type_hash)?;
        let decision = Decision {
            owner,
            type_hash,
            location,
            component,
        };
        research::insert_pending_decision(&mut self.decisions, decision);
        Ok(())
    }
    pub fn into_current<'s>(
        self,
        state: &'s mut LoadedState,
        inputs: processing::Inputs<'s>,
        limits: c6_core::player_notifications_owner::OccurrenceLimits,
    ) -> Result<current_aftermath::Context<'s, Self>, processing::Error> {
        self.check(state)?;
        Ok(current_aftermath::Context::new(processing::Provider::new(
            state,
            inputs,
            self,
            limits,
            processing::Scope::SourceInitializedStockXp2,
        )))
    }
    pub(crate) fn check(&self, state: &LoadedState) -> Result<(), processing::Error> {
        if state.binding() != &self.binding {
            return Err(processing::Error::Unsupported(
                "stock aftermath state identity",
            ));
        }
        Ok(())
    }
    fn decision(&mut self, candidate: &Created) {
        let record = &candidate.record;
        let decision = Decision {
            owner: i32::from(record.id.owner),
            type_hash: record.type_hash,
            location: record.location,
            component: (!(record.component.owner == -1 && record.component.id == u32::MAX))
                .then_some(record.component),
        };
        research::insert_pending_decision(&mut self.decisions, decision);
    }
}

#[allow(unused_variables)]
impl<P> lifecycle::Backend for Runtime<'_, P> {
    fn current_turn(&mut self, state: &mut LoadedState) -> Result<i32, lifecycle::Error> {
        self.reached.push("notification current turn");
        Ok(state.retained().core.current_turn)
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        object: ObjectId,
        report: lifecycle::Report,
    ) -> Result<(), lifecycle::Error> {
        self.reached.push("notification report");
        self.publications.push(report);
        let (identity, argument) = match report {
            lifecycle::Report::Added(_) => {
                let record = state
                    .player_notifications()?
                    .record_by_object(object)
                    .map_err(lifecycle::Error::Owner)?;
                let decision = Decision {
                    owner: i32::from(record.id.owner),
                    type_hash: record.type_hash,
                    location: record.location,
                    component: (!(record.component.owner == -1 && record.component.id == u32::MAX))
                        .then_some(record.component),
                };
                research::insert_pending_decision(&mut self.decisions, decision);
                // Stock plain headless publication exposes the semantic decision;
                // native client cache/DTO playback has no policy role here.
                return Ok(());
            }
            lifecycle::Report::Dismissed(_) => {
                let record = state
                    .player_notifications()?
                    .record_by_object(object)
                    .map_err(lifecycle::Error::Owner)?;
                let expired = Decision {
                    owner: i32::from(record.id.owner),
                    type_hash: record.type_hash,
                    location: record.location,
                    component: (!(record.component.owner == -1 && record.component.id == u32::MAX))
                        .then_some(record.component),
                };
                self.decisions.retain(|d| {
                    !research::same_global_progression_choice(d, &expired)
                        && (d.owner != i32::from(record.id.owner)
                            || d.type_hash != record.type_hash
                            || d.location != record.location)
                });
                return Ok(());
            }
            lifecycle::Report::Activated { identity, argument } => (identity, argument),
        };
        self.reporting
            .send_notification_activated_report(state, object, identity, argument)
            .map_err(|e| {
                self.last_error = Some(format!("{e:?}"));
                lifecycle::Error::Backend("stock notification publication")
            })
    }
    fn overridden_activate(
        &mut self,
        state: &mut LoadedState,
        object: ObjectId,
        argument: bool,
    ) -> Result<(), lifecycle::Error> {
        Err(lifecycle::Error::Backend(
            "stock notification override/destructor",
        ))
    }
    fn overridden_on_add(
        &mut self,
        state: &mut LoadedState,
        object: ObjectId,
    ) -> Result<(), lifecycle::Error> {
        Err(lifecycle::Error::Backend(
            "stock notification override/destructor",
        ))
    }
    fn overridden_supersede(
        &mut self,
        state: &mut LoadedState,
        receiver: ObjectId,
        candidate: ObjectId,
    ) -> Result<(), lifecycle::Error> {
        Err(lifecycle::Error::Backend(
            "stock notification override/destructor",
        ))
    }
    fn deleting_destructor(
        &mut self,
        state: &mut LoadedState,
        object: ObjectId,
    ) -> Result<(), lifecycle::Error> {
        Err(lifecycle::Error::Backend(
            "stock notification override/destructor",
        ))
    }
}

impl<P> processing::Backend for Runtime<'_, P> {
    fn pantheon_needed(&self, state: &LoadedState, owner: i32) -> Result<bool, processing::Error> {
        self.check(state)?;
        social::pantheon_needed(
            state,
            self.social_capabilities
                .as_ref()
                .ok_or(processing::Error::Unsupported(
                    "current social capability membership",
                ))?,
            owner,
        )
        .map_err(|_| processing::Error::Unsupported("current pantheon eligibility"))
    }
    fn capture_disposition_pending(
        &self,
        state: &LoadedState,
        owner: i32,
    ) -> Result<bool, processing::Error> {
        self.check(state)?;
        Ok(self
            .capture_dispositions
            .iter()
            .any(|choice| i32::from(choice.city.owner) == owner && choice.legal.len() > 1))
    }
    fn manager_initialized(&mut self, state: &mut LoadedState) -> Result<bool, processing::Error> {
        self.check(state)?;
        self.reached.push("notification manager initialized");
        self.facts
            .manager_initialized
            .ok_or(processing::Error::Unsupported(
                "notification manager initialization",
            ))
    }
    fn game_uninitializing(&mut self, state: &mut LoadedState) -> Result<bool, processing::Error> {
        self.check(state)?;
        self.reached.push("game uninitializing");
        self.facts
            .game_uninitializing
            .ok_or(processing::Error::Unsupported("game uninitializing"))
    }
    fn allocate(
        &mut self,
        state: &mut LoadedState,
        type_hash: u32,
    ) -> Result<processing::Allocation, processing::Error> {
        self.check(state)?;
        let pool = notifications::allocation::stock_auto_pool(type_hash)?;
        let serial = self.temporary_serial;
        self.temporary_serial = serial.checked_add(1).filter(|n| *n < (1 << 63)).ok_or(
            processing::Error::Unsupported("temporary identity capacity"),
        )?;
        // Semantic temporary identity, never a native address or display ID.
        // No native pool emptiness/history is asserted by this projection.
        Ok(processing::Allocation {
            object: c6_core::notification_lifecycle::ObjectId((1 << 63) | serial),
            pool,
            prior: None,
        })
    }
    fn validate_duplicate_identity(
        &mut self,
        state: &mut LoadedState,
        candidate: &Created,
        existing_target: c6_core::notifications::ComponentId,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        self.decision(candidate);
        let _ = existing_target;
        if !self.positive_temporaries.contains(&candidate.live.object) {
            self.positive_temporaries.push(candidate.live.object);
        }
        Ok(())
    }

    fn release_pending(
        &mut self,
        state: &mut LoadedState,
        candidate: &mut Option<Created>,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        let created = candidate
            .as_ref()
            .ok_or(processing::Error::Unsupported("temporary already consumed"))?;
        if created.live.pool_binding
            != Some(notifications::allocation::stock_auto_pool(
                created.record.type_hash,
            )?)
        {
            return Err(processing::Error::Unsupported("temporary release binding"));
        }
        let positive = self
            .positive_temporaries
            .iter()
            .position(|id| *id == created.live.object);
        if let Some(index) = positive {
            self.positive_temporaries.remove(index);
        } else {
            self.decisions.retain(|d| {
                d.owner != i32::from(created.record.id.owner)
                    || d.type_hash != created.record.type_hash
            });
        }
        candidate.take();
        Ok(())
    }
    fn initialize_text(
        &mut self,
        state: &mut LoadedState,
        candidate: &mut Created,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        self.decision(candidate);
        // Source-closed plain localization has no gameplay or RNG effects in
        // this scope. Semantic decision data is retained; strings are omitted.
        Ok(())
    }
    fn manager_game_local_player(
        &mut self,
        state: &mut LoadedState,
    ) -> Result<Option<i32>, processing::Error> {
        self.check(state)?;
        // Presence establishes initial knowledge; subsequent source hotseat
        // changes update this canonical field on the same LoadedState.
        self.facts
            .local_player
            .map(|_| Some(state.retained().core.local_player))
            .ok_or(processing::Error::Unsupported("notification local player"))
    }
    fn blocking_changed(
        &mut self,
        state: &mut LoadedState,
        event: u32,
        old: i32,
        new: i32,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        if event != 0x55f15a7b {
            return Err(processing::Error::Unsupported("stock blocking event"));
        }
        // The Player store has already happened. Fixed stock listeners fill
        // reflected Player score/agenda output and publish client DTOs. Their
        // stock scalar queries have no gameplay/RNG effect; canonical warm
        // route paths exclude the hypothetical-route/pathfinder branches.
        // Keep the semantic scheduling change, omit native display/cache work.
        super::reporting::yield_monitor::warm_route_domain(state)
            .map_err(|_| processing::Error::Unsupported("blocking report current route domain"))?;
        self.blocking_changes.push((event, old, new));
        Ok(())
    }
}
impl<P> ProcessStatement<LoadedState> for Runtime<'_, P> {
    fn process_statement(
        &mut self,
        _: &mut LoadedState,
        _: StatementHandle,
    ) -> Result<(), unit_operation_aftermath::Error> {
        Err(unit_operation_aftermath::Error::Unsupported(
            "diplomacy statement requires semantic external decision",
        ))
    }
}
impl<P> current_aftermath::Backend for Runtime<'_, P> {
    type ObserverError = processing::Error;
    fn messages_processed(
        &mut self,
        state: &mut LoadedState,
        routes: WarmRouteDomain,
    ) -> Result<(), Self::ObserverError> {
        self.check(state)?;
        if routes.revision != state.revision() {
            return Err(processing::Error::Unsupported("stale warm route domain"));
        }
        // The source stock headless observer has only derived snapshot/cache
        // and presentation effects after the current route-domain work. No
        // added callbacks or aliased custom values are admitted by Scope.
        self.reached
            .push("messages processed stock headless observer");
        Ok(())
    }
}
