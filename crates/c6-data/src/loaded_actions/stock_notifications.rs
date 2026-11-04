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
pub struct Runtime<'a, P = c6_core::unit::OperationParameters> {
    pub script: stock_script::Runtime<P>,
    pub reporting: reporting::Provider<'a>,
    pub decisions: Vec<Decision>,
    pub last_error: Option<String>,
    pub reached: Vec<&'static str>,
    binding: WireBinding,
    facts: Facts,
    temporary_serial: u64,
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
            last_error: None,
            reached: Vec::new(),
            binding: state.binding().clone(),
            facts,
            temporary_serial: 0,
        })
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
        if !self.decisions.contains(&decision) {
            self.decisions.push(decision);
        }
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
    fn check(&self, state: &LoadedState) -> Result<(), processing::Error> {
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
        if !self.decisions.contains(&decision) {
            self.decisions.push(decision);
        }
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
        let lifecycle::Report::Activated { identity, argument } = report else {
            return Err(lifecycle::Error::Backend(
                "stock notification report subtype",
            ));
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
        if existing_target.owner != -1 || existing_target.id != u32::MAX {
            return Ok(());
        }
        Err(processing::Error::Unsupported(
            "positive notification residual target type",
        ))
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
        self.facts
            .local_player
            .map(Some)
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
        self.last_error = Some(format!("blocking change {event:08x}: {old} -> {new}"));
        Err(processing::Error::Unsupported(
            "stock blocking-changed consumers",
        ))
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
