//! Persistent stock script services shared by gameplay actions.
//!
//! The caller initializes one ScriptState and retains it between requests. These
//! services execute its real registry plans; they do not supply callback answers.
use super::transition::ScriptState;
#[path = "stock_script_queries.rs"]
pub mod queries;
use c6_core::{
    loaded_state::LoadedState, script_event_registry::RegistryError,
    skip_turn::legality::ScriptTestAllResult, unit::PlotCoord,
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

/// CallHook gameplay projection while the caller retains the source lock/
/// Args lifetime. Empty groups update the SAME registry; nonempty VM callbacks
/// remain explicit errors. This is also used by Runtime's synchronization owner.
pub fn call_empty_hook(
    script: &mut ScriptState,
    state: &LoadedState,
    event: &str,
    arguments: &[i32],
) -> Result<(), Error> {
    if !script.matches(state) {
        return Err(Error::Identity);
    }
    script.call_empty(event, arguments).map_err(Error::Registry)
}
/// Lua.GameSignals.OnCombatOccurred reads current owners/IDs before CallHook;
/// absent native Unit pointers contribute two signed -1 values.
pub fn combat_arguments(
    state: &LoadedState,
    attacker: Option<c6_core::map::UnitReference>,
    defender: Option<c6_core::map::UnitReference>,
) -> Result<[i32; 4], Error> {
    let mut result = [-1; 4];
    for (offset, reference) in [(0, attacker), (2, defender)] {
        if let Some(reference) = reference {
            let unit = crate::melee_attack::actor(state, reference).map_err(|_| Error::Identity)?;
            result[offset] = unit.e_owner;
            result[offset + 1] = unit.id as i32;
        }
    }
    Ok(result)
}

/// Source GameSignals wrapper: form the four arguments while holding the core
/// service, then release/call/reacquire on the same persistent Script registry.
pub fn on_combat_occurred(
    script: &mut ScriptState,
    synchronization: &mut Synchronization,
    state: &LoadedState,
    attacker: Option<c6_core::map::UnitReference>,
    defender: Option<c6_core::map::UnitReference>,
) -> Result<(), Error> {
    let arguments = combat_arguments(state, attacker, defender)?;
    synchronization.call(|| call_empty_hook(script, state, "OnCombatOccurred", &arguments))
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
pub struct Runtime<P = c6_core::unit::OperationParameters> {
    parameters: std::marker::PhantomData<fn() -> P>,
    last_error: Option<Error>,
    script: ScriptState,
    synchronization: Synchronization,
}
impl<P> Runtime<P> {
    pub fn new(state: &LoadedState, script: ScriptState) -> Result<Self, Error> {
        Self::from_parts(state, script, Synchronization::exclusive())
    }
    /// Transfer an existing script and synchronization owner between action
    /// services without resetting its lock depth or registry state.
    pub fn from_parts(
        state: &LoadedState,
        script: ScriptState,
        synchronization: Synchronization,
    ) -> Result<Self, Error> {
        if !script.matches(state) {
            return Err(Error::Identity);
        }
        Ok(Self {
            script,
            parameters: std::marker::PhantomData,
            last_error: None,
            synchronization,
        })
    }
    /// Borrow this SAME registry/RNG/synchronization owner for another action
    /// parameter transport. No script state is cloned, reimported or reset.
    pub fn queries<Q>(&mut self) -> queries::View<'_, P, Q> {
        queries::View {
            runtime: self,
            parameters: std::marker::PhantomData,
        }
    }
    pub fn last_error(&self) -> Option<&Error> {
        self.last_error.as_ref()
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
    /// Source CallHook with two integer arguments and no reached subscribers.
    /// Nonempty callback groups refuse; uses this runtime's persistent registry.
    pub fn call_empty(
        &mut self,
        state: &LoadedState,
        event: &str,
        arguments: &[i32],
    ) -> Result<(), Error> {
        if !self.script.matches(state) {
            return Err(Error::Identity);
        }
        self.synchronization
            .call(|| call_empty_hook(&mut self.script, state, event, arguments))
    }
    /// First TurnManager activation signal's stock Lua consumer (2c9c7d0).
    /// Execute against this same persistent registry; subscribed unsupported
    /// callbacks remain errors, never an inferred empty hook.
    pub fn on_combat_occurred(
        &mut self,
        state: &LoadedState,
        attacker: Option<c6_core::map::UnitReference>,
        defender: Option<c6_core::map::UnitReference>,
    ) -> Result<(), Error> {
        let arguments = combat_arguments(state, attacker, defender)?;
        self.call_empty(state, "OnCombatOccurred", &arguments)
    }
    pub fn on_player_turn_started(&mut self, state: &LoadedState, owner: i32) -> Result<(), Error> {
        self.call_empty(state, "PlayerTurnStarted", &[owner])
    }
    pub fn call_two_empty(
        &mut self,
        state: &LoadedState,
        event: &str,
        arguments: [i32; 2],
    ) -> Result<(), Error> {
        self.call_empty(state, event, &arguments)
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

/// Arguments are scoped to one Manager TestAll call and retain native order.
pub struct Arguments {
    values: Vec<i32>,
}
/// The canonical Data Manager wrapper supplies physical world/rules queries.
/// This adapter supplies persistent script services; direct physical queries
/// remain explicit errors rather than fabricated gameplay answers.
#[allow(unused_variables)]
impl<P> c6_core::skip_turn::legality::ManagerQueries<c6_core::unit::Unit> for Runtime<P> {
    type Error = &'static str;
    type Parameters = P;
    type Player = i32;
    type Game = ();
    type Script = u64;
    type EngineUtility = ();
    type Args = Arguments;
    type Handler = i32;
    type Plot = ();
    type Results = ();
    fn allow_tactical_commands(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        owner: i32,
    ) -> Result<bool, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn player(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        owner: i32,
    ) -> Result<Self::Player, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn turn_active(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn processing_automation(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<i32, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn player_id(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        player: Self::Player,
    ) -> Result<i32, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn game(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Self::Game, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn player_human(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn paused(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        game: Self::Game,
    ) -> Result<bool, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn runtime_id(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        hash: u32,
    ) -> Result<i32, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn handler(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        runtime_id: i32,
    ) -> Result<Option<Self::Handler>, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn can_be_interrupted(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        handler: Self::Handler,
    ) -> Result<bool, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn operation_disabled(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        runtime_id: i32,
    ) -> Result<bool, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn script_system(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Option<Self::Script>, Self::Error> {
        Ok(Some(self.script.identity().0))
    }
    fn args_begin(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Self::Args, Self::Error> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(3)
            .map_err(|_| "script arguments allocation")?;
        Ok(Arguments { values })
    }
    fn args_push_int(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        args: &mut Self::Args,
        value: i32,
    ) -> Result<(), Self::Error> {
        if args.values.len() >= 3 {
            return Err("script argument capacity");
        }
        args.values.push(value);
        Ok(())
    }
    fn engine_utility(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Self::EngineUtility, Self::Error> {
        Ok(())
    }
    fn has_game_core_lock(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        utility: Self::EngineUtility,
    ) -> Result<bool, Self::Error> {
        Ok(self.synchronization.core_lock_depth() > 0)
    }
    fn release_game_core_lock(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        self.synchronization
            .release()
            .map_err(|_| "release unheld core lock")
    }
    fn get_game_core_lock(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        self.synchronization
            .acquire()
            .map_err(|_| "core lock depth overflow")
    }
    fn script_call_test_all(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        script: Self::Script,
        args: &mut Self::Args,
        event: &'static str,
        initial_value: bool,
    ) -> Result<c6_core::skip_turn::legality::ScriptTestAllResult, Self::Error> {
        let arguments = args
            .values
            .as_slice()
            .try_into()
            .map_err(|_| "script argument shape")?;
        test_all(&mut self.script, script, event, arguments, initial_value).map_err(|error| {
            self.last_error = Some(error);
            "script TestAll unavailable"
        })
    }
    fn args_end(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        args: Self::Args,
    ) -> Result<(), Self::Error> {
        if args.values.len() != 3 {
            return Err("script argument shape");
        }
        Ok(())
    }
    fn unit_plot(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Option<Self::Plot>, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
    }
    fn handler_can_start(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
        handler: Self::Handler,
        plot: Self::Plot,
        visible_only: bool,
        results: Option<&mut Self::Results>,
    ) -> Result<bool, Self::Error> {
        Err("physical query must use canonical Data Manager wrapper")
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
