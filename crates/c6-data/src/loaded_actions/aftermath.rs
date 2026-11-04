//! Current-state continuation after an ordinary UnitOperation manager call.
//!
//! This joins existing diplomacy and notification implementations. It neither
//! stages a second actor nor copies a SkipTurn plan. The provider retains its
//! backend, pending notification allocations and all partial effects on errors.
use super::{notifications::processing, reporting::yield_monitor};
use c6_core::{
    diplomacy_manager::ProcessStatement, loaded_state::LoadedState, unit_operation_aftermath,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Diplomacy,
    Notifications,
    MessagesProcessed,
}

#[derive(Debug)]
pub enum Error<E> {
    /// An earlier unsupported continuation already left its native prefix.
    FailedContinuation(Stage),
    Diplomacy(unit_operation_aftermath::Error),
    Notifications(unit_operation_aftermath::Error),
    WarmRoutes(yield_monitor::Error),
    MessagesProcessed(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "current action aftermath: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

/// The actual final Context.ProcessMessages observer environment. Implementors
/// retain the same ScriptState, RNG and synchronous effects used by notification
/// and statement callbacks. A warm route domain qualifies the existing headless
/// route lookup; it is not permission to discard other reachable consumers.
pub trait Backend: processing::Backend {
    type ObserverError;
    fn messages_processed(
        &mut self,
        state: &mut LoadedState,
        warm_routes: yield_monitor::WarmRouteDomain,
    ) -> Result<(), Self::ObserverError>;
}

/// Own the existing processing provider throughout the continuation. In
/// particular, failure must not drop a detached allocation or silently replace
/// its backend with a fresh script/effect state before inspection.
pub struct Context<'a, B> {
    pub notifications: processing::Provider<'a, B>,
    failed_at: Option<Stage>,
    trace: Vec<Stage>,
}
impl<'a, B: Backend> Context<'a, B> {
    pub fn new(notifications: processing::Provider<'a, B>) -> Self {
        Self {
            notifications,
            failed_at: None,
            trace: Vec::new(),
        }
    }
    pub fn trace(&self) -> &[Stage] {
        &self.trace
    }
    pub fn failed_at(&self) -> Option<Stage> {
        self.failed_at
    }
    pub fn notification_failure(&self) -> Option<&processing::Error> {
        self.notifications.failure()
    }
    fn check(&self) -> Result<(), Error<B::ObserverError>> {
        match self.failed_at {
            Some(stage) => Err(Error::FailedContinuation(stage)),
            None => Ok(()),
        }
    }

    /// Run for every manager call that found its Unit, including a modeled
    /// CanStart rejection. An absent/invalid request never enters this method.
    /// Current owner, notifications and all six stock auto queries are resolved
    /// by the existing provider after diplomacy effects have executed.
    pub fn after_manager(
        &mut self,
        owner: i32,
        budget: &mut usize,
    ) -> Result<(), Error<B::ObserverError>>
    where
        B: ProcessStatement<LoadedState>,
    {
        self.check()?;
        self.trace.push(Stage::Diplomacy);
        if let Err(error) = self
            .notifications
            .state
            .process_pending_diplomacy(&mut self.notifications.backend, budget)
        {
            self.failed_at = Some(Stage::Diplomacy);
            return Err(Error::Diplomacy(error));
        }
        self.trace.push(Stage::Notifications);
        if let Err(error) = self.notifications.update(owner, budget) {
            self.failed_at = Some(Stage::Notifications);
            return Err(Error::Notifications(error));
        }
        Ok(())
    }

    /// Outer Context.ProcessMessages boundary, once after its serialized batch.
    /// Keep this separate from after_manager: a batch can contain several
    /// messages, including messages whose Unit lookup was absent.
    pub fn messages_processed(
        &mut self,
    ) -> Result<yield_monitor::WarmRouteDomain, Error<B::ObserverError>> {
        self.check()?;
        let warm = match yield_monitor::warm_route_domain(self.notifications.state) {
            Ok(warm) => warm,
            Err(error) => {
                self.failed_at = Some(Stage::MessagesProcessed);
                return Err(Error::WarmRoutes(error));
            }
        };
        self.trace.push(Stage::MessagesProcessed);
        if let Err(error) = self
            .notifications
            .backend
            .messages_processed(self.notifications.state, warm)
        {
            self.failed_at = Some(Stage::MessagesProcessed);
            return Err(Error::MessagesProcessed(error));
        }
        Ok(warm)
    }

    /// Convenience for exactly one admitted serialized message. General batch
    /// callers invoke after_manager for reached managers and then the final
    /// messages_processed boundary once.
    pub fn run(
        &mut self,
        owner: i32,
        budget: &mut usize,
    ) -> Result<yield_monitor::WarmRouteDomain, Error<B::ObserverError>>
    where
        B: ProcessStatement<LoadedState>,
    {
        self.after_manager(owner, budget)?;
        self.messages_processed()
    }
}
