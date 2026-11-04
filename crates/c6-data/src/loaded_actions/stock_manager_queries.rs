//! Manager physical queries stay in the canonical Data wrapper; script calls
//! forward to the ONE persistent ScriptState owned by the stock session.
use super::Runtime;
impl<P> c6_core::skip_turn::legality::ManagerQueries<c6_core::unit::Unit> for Runtime<'_, P> {
    type Error = &'static str;
    type Parameters = P;
    type Player = i32;
    type Game = ();
    type Script = u64;
    type EngineUtility = ();
    type Args = crate::loaded_actions::stock_script::Arguments;
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
        c6_core::skip_turn::legality::ManagerQueries::allow_tactical_commands(
            &mut self.script,
            s,
            owner,
        )
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
        c6_core::skip_turn::legality::ManagerQueries::player(&mut self.script, s, owner)
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
        c6_core::skip_turn::legality::ManagerQueries::turn_active(&mut self.script, s, player)
    }
    fn processing_automation(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<i32, Self::Error> {
        c6_core::skip_turn::legality::ManagerQueries::processing_automation(&mut self.script, s)
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
        c6_core::skip_turn::legality::ManagerQueries::player_id(&mut self.script, s, player)
    }
    fn game(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Self::Game, Self::Error> {
        c6_core::skip_turn::legality::ManagerQueries::game(&mut self.script, s)
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
        c6_core::skip_turn::legality::ManagerQueries::player_human(&mut self.script, s, player)
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
        c6_core::skip_turn::legality::ManagerQueries::paused(&mut self.script, s, game)
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
        c6_core::skip_turn::legality::ManagerQueries::runtime_id(&mut self.script, s, hash)
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
        c6_core::skip_turn::legality::ManagerQueries::handler(&mut self.script, s, runtime_id)
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
        c6_core::skip_turn::legality::ManagerQueries::can_be_interrupted(
            &mut self.script,
            s,
            handler,
        )
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
        c6_core::skip_turn::legality::ManagerQueries::operation_disabled(
            &mut self.script,
            s,
            runtime_id,
        )
    }
    fn script_system(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Option<Self::Script>, Self::Error> {
        c6_core::skip_turn::legality::ManagerQueries::script_system(&mut self.script, s)
    }
    fn args_begin(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Self::Args, Self::Error> {
        c6_core::skip_turn::legality::ManagerQueries::args_begin(&mut self.script, s)
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
        c6_core::skip_turn::legality::ManagerQueries::args_push_int(
            &mut self.script,
            s,
            args,
            value,
        )
    }
    fn engine_utility(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Self::EngineUtility, Self::Error> {
        c6_core::skip_turn::legality::ManagerQueries::engine_utility(&mut self.script, s)
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
        c6_core::skip_turn::legality::ManagerQueries::has_game_core_lock(
            &mut self.script,
            s,
            utility,
        )
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
        c6_core::skip_turn::legality::ManagerQueries::release_game_core_lock(
            &mut self.script,
            s,
            utility,
        )
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
        c6_core::skip_turn::legality::ManagerQueries::get_game_core_lock(
            &mut self.script,
            s,
            utility,
        )
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
        c6_core::skip_turn::legality::ManagerQueries::script_call_test_all(
            &mut self.script,
            s,
            script,
            args,
            event,
            initial_value,
        )
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
        c6_core::skip_turn::legality::ManagerQueries::args_end(&mut self.script, s, args)
    }
    fn unit_plot(
        &mut self,
        s: &mut c6_core::skip_turn::legality::ManagerQueryState<
            '_,
            c6_core::unit::Unit,
            Self::Parameters,
        >,
    ) -> Result<Option<Self::Plot>, Self::Error> {
        c6_core::skip_turn::legality::ManagerQueries::unit_plot(&mut self.script, s)
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
        c6_core::skip_turn::legality::ManagerQueries::handler_can_start(
            &mut self.script,
            s,
            handler,
            plot,
            visible_only,
            results,
        )
    }
}
