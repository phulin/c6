//! Borrow one script owner with the parameter type required by an action.
use super::{test_all, Arguments, Runtime};
pub struct View<'a, P, Q> {
    pub(super) runtime: &'a mut Runtime<P>,
    pub(super) parameters: std::marker::PhantomData<fn() -> Q>,
}
#[allow(unused_variables)]
impl<P, Q> c6_core::skip_turn::legality::ManagerQueries<c6_core::unit::Unit> for View<'_, P, Q> {
    type Error = &'static str;
    type Parameters = Q;
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
        Ok(Some(self.runtime.script.identity().0))
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
        Ok(self.runtime.synchronization.core_lock_depth() > 0)
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
        self.runtime
            .synchronization
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
        self.runtime
            .synchronization
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
        test_all(
            &mut self.runtime.script,
            script,
            event,
            arguments,
            initial_value,
        )
        .map_err(|error| {
            self.runtime.last_error = Some(error);
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
