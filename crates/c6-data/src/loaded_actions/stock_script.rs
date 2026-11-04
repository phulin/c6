//! Persistent stock script services shared by gameplay actions.
//!
//! The caller initializes one ScriptState and retains it between requests. These
//! services execute its real registry plans; they do not supply callback answers.
use super::transition::ScriptState;
use c6_core::{
    loaded_state::LoadedState, unit::PlotCoord, script_event_registry::RegistryError,
    skip_turn::legality::ScriptTestAllResult,
};

#[derive(Debug)]
pub enum Error {
    Identity,
    LockUnderflow,
    LockOverflow,
    Registry(RegistryError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock script: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Logical exclusive core service ownership. This is not an operating-system
/// mutex or a copied native pointer. Failed callbacks retain the released prefix.
pub struct Synchronization {
    depth: u32,
}
impl Synchronization {
    pub fn exclusive() -> Self {
        Self { depth: 1 }
    }
    pub fn core_lock_depth(&self) -> u32 {
        self.depth
    }
    pub fn release(&mut self) -> Result<(), Error> {
        self.depth = self.depth.checked_sub(1).ok_or(Error::LockUnderflow)?;
        Ok(())
    }
    pub fn acquire(&mut self) -> Result<(), Error> {
        self.depth = self.depth.checked_add(1).ok_or(Error::LockOverflow)?;
        Ok(())
    }
    fn call<T>(&mut self, f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        let held = self.depth > 0;
        if held {
            self.release()?;
        }
        let value = f()?;
        if held {
            self.acquire()?;
        }
        Ok(value)
    }
}

/// Executes TestAll while the caller manages its source-ordered lock wrapper.
/// A missing registry event retains the caller's initial value.
pub fn test_all(
    script: &mut ScriptState,
    identity: u64,
    event: &str,
    arguments: [i32; 3],
    initial_value: bool,
) -> Result<ScriptTestAllResult, Error> {
    if script.identity().0 != identity {
        return Err(Error::Identity);
    }
    let plan = script
        .plan_test_all(event, arguments)
        .map_err(Error::Registry)?;
    let result = plan.result();
    script.prepare_plan(plan).map_err(Error::Registry)?.commit();
    Ok(ScriptTestAllResult {
        called: result.is_some(),
        value: result.unwrap_or(initial_value),
    })
}

/// The stock OnUnitMoved leaf. Surrounding signal subscribers remain the
/// responsibility of the gameplay signal dispatcher.
pub fn on_unit_moved(
    script: &mut ScriptState,
    synchronization: &mut Synchronization,
    state: &LoadedState,
    actor: (i32, u32),
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error> {
    if !script.matches(state) {
        return Err(Error::Identity);
    }
    synchronization.call(|| {
        let plan = script
            .plan_call_six(
                "OnUnitMoved",
                [actor.0, actor.1 as i32, old.x, old.y, new.x, new.y],
            )
            .map_err(Error::Registry)?;
        script.prepare_plan(plan).map_err(Error::Registry)?.commit();
        Ok(())
    })
}

/// Owns the same mutable registry, callback execution state and optional matched
/// libc RNG owner across actions. Rules and gameplay state remain independently
/// owned by the session; no state import is performed by these methods.
pub struct Runtime {
    script: ScriptState,
    synchronization: Synchronization,
}
impl Runtime {
    pub fn new(state: &LoadedState, script: ScriptState) -> Result<Self, Error> {
        if !script.matches(state) {
            return Err(Error::Identity);
        }
        Ok(Self {
            script,
            synchronization: Synchronization::exclusive(),
        })
    }
    pub fn script(&self) -> &ScriptState {
        &self.script
    }
    pub fn script_mut(&mut self) -> &mut ScriptState {
        &mut self.script
    }
    pub fn synchronization(&self) -> &Synchronization {
        &self.synchronization
    }
    pub fn test_all(
        &mut self,
        state: &LoadedState,
        event: &str,
        arguments: [i32; 3],
        initial: bool,
    ) -> Result<ScriptTestAllResult, Error> {
        if !self.script.matches(state) {
            return Err(Error::Identity);
        }
        let identity = self.script.identity().0;
        self.synchronization
            .call(|| test_all(&mut self.script, identity, event, arguments, initial))
    }
    pub fn on_unit_moved(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
        old: PlotCoord,
        new: PlotCoord,
    ) -> Result<(), Error> {
        on_unit_moved(
            &mut self.script,
            &mut self.synchronization,
            state,
            actor,
            old,
            new,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_observes_release_and_success_restores_ownership() {
        let mut sync = Synchronization::exclusive();
        sync.call(|| Ok(())).unwrap();
        assert_eq!(sync.core_lock_depth(), 1);
        sync.release().unwrap();
        sync.call(|| Ok(())).unwrap();
        assert_eq!(sync.core_lock_depth(), 0);
        assert!(matches!(sync.release(), Err(Error::LockUnderflow)));
    }
    #[test]
    fn callback_failure_retains_release_prefix() {
        let mut sync = Synchronization::exclusive();
        assert!(matches!(
            sync.call::<()>(|| Err(Error::Identity)),
            Err(Error::Identity)
        ));
        assert_eq!(sync.core_lock_depth(), 0);
        sync.acquire().unwrap();
        assert_eq!(sync.core_lock_depth(), 1);
    }
}
