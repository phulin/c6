//! Original Linux XP2 operation predicates and complete manager query ordering.
//! Live providers remain explicit; this is not accepted action execution.
use crate::unit::Unit;

/// Unit::IsBusy (0x2c27360): signed operation timer strictly above zero.
pub fn is_busy(unit: &Unit) -> bool {
    unit.i_operation_timer > 0
}

/// Unit::CanMove (0x2c27930): signed FixedPointT<8> raw value above zero.
/// A positive fraction below one movement point still passes.
pub fn can_move(unit: &Unit) -> bool {
    unit.x_moves_remaining > 0
}

/// Required original gameplay queries, in native call order. These are not
/// acceptance booleans. Errors mean missing/unsupported inputs, not native false.
/// Implementations must preserve query effects and resolve actual layered map
/// membership, Formation and live rules. See docs/linux-unit-operation-legality.md.
/// Mutable Unit permits synchronous query effects; subsequent predicates reread it.
pub trait SkipTurnQueries {
    type Error;
    fn is_human(&mut self, unit: &mut Unit) -> Result<bool, Self::Error>;
    /// Original Formation overload, false bool argument: unsigned maximum over
    /// existing, non-delayed-death members' original friendly-unit type counts.
    fn friendly_units_of_type_count(&mut self, unit: &mut Unit) -> Result<u32, Self::Error>;
    fn plot_unit_limit(&mut self, unit: &mut Unit) -> Result<i32, Self::Error>;
}

/// Complete SkipTurn::CanStart handler (0x2dac950), separate from the full
/// manager predicate, scripting veto and request ordering. Does not check moves.
/// `testVisible` does not bypass the human branch. On error, any effects already
/// performed by the provider remain; this query is not transactional.
pub fn skip_turn_can_start<Q: SkipTurnQueries>(
    unit: &mut Unit,
    queries: &mut Q,
) -> Result<bool, Q::Error> {
    if queries.is_human(unit)? {
        if unit.i_fortify_turns > 0 || [super::HOLD, 0x024a74eb].contains(&unit.e_activity) {
            return Ok(false);
        }
        let count = queries.friendly_units_of_type_count(unit)?;
        let limit = queries.plot_unit_limit(unit)?;
        // Native CMP/JA compares the same 32 bits unsigned, even for negative limits.
        if count > limit as u32 {
            return Ok(false);
        }
    }
    if unit.e_activity == 0x5ff1bbdf {
        return Ok(false);
    }
    Ok(unit.k_espionage.e_operation == u32::MAX)
}

/// Live operation identity is distinct from wire hashes. Parameters retain the
/// caller's typed representation; scripts may change the referenced operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagerOperation<P> {
    pub runtime_id: i32,
    pub directive: i32,
    pub parameters: P,
    pub flags: i32,
}

/// Pure reads implemented directly for the complete typed Unit. A smaller subject can
/// exercise ordering in synthetic tests; it does not reconstruct native state.
pub trait ManagerUnitState {
    fn operation_timer(&self) -> i32;
    fn owner(&self) -> i32;
    fn unit_id(&self) -> u32;
    fn queue_nonempty(&self) -> bool;
    fn front_operation_hash(&self) -> Option<u32>;
}
impl ManagerUnitState for Unit {
    fn operation_timer(&self) -> i32 {
        self.i_operation_timer
    }
    fn owner(&self) -> i32 {
        self.e_owner
    }
    fn unit_id(&self) -> u32 {
        self.id
    }
    fn queue_nonempty(&self) -> bool {
        !self.k_operations.nodes.is_empty()
    }
    fn front_operation_hash(&self) -> Option<u32> {
        self.k_operations.nodes.first().map(|n| n.operation_hash)
    }
}

/// Each provider receives the actual mutable subject and referenced operation so
/// synchronous effects remain observable to later original reads.
pub struct ManagerQueryState<'a, U, P> {
    pub unit: &'a mut U,
    pub operation: &'a mut ManagerOperation<P>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptTestAllResult {
    /// Original CallTestAll return bit, independently of its out-parameter.
    /// For the proven host backend this reports a protected non-nil result,
    /// not whether callbacks ran.
    pub called: bool,
    pub value: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagerQueryError<E> {
    Provider(E),
    /// Native code dereferences this handler unconditionally. Missing identity
    /// cannot become a native false result or be replaced by a fabricated handler.
    MissingRequestedHandler(i32),
}

/// Complete Manager::CanStart dependencies. No default implementation exists.
/// Player/Game/Script/Handler/Plot handles must preserve actual receiver identity
/// across effects; they are not IDs to resolve again against a changed owner.
/// Implementations must supply original virtual behavior and propagate unknown
/// live data as errors. Script handle destruction includes its real effects.
pub trait ManagerQueries<U: ManagerUnitState> {
    type Error;
    type Parameters;
    type Player: Copy;
    type Game: Copy;
    type Script: Copy;
    type EngineUtility: Copy;
    type Args;
    type Handler: Copy;
    type Plot: Copy;
    type Results;
    fn allow_tactical_commands(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        owner: i32,
    ) -> Result<bool, Self::Error>;
    fn player(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        owner: i32,
    ) -> Result<Self::Player, Self::Error>;
    fn turn_active(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error>;
    fn processing_automation(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
    ) -> Result<i32, Self::Error>;
    fn player_id(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        player: Self::Player,
    ) -> Result<i32, Self::Error>;
    fn game(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
    ) -> Result<Self::Game, Self::Error>;
    fn player_human(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error>;
    fn paused(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        game: Self::Game,
    ) -> Result<bool, Self::Error>;
    /// Pure identity translation: resolve the wire queue's hash to its actual
    /// live numeric identity. This introduces no new native gameplay query or
    /// side effect; unresolved identity must return an error.
    fn runtime_id(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        hash: u32,
    ) -> Result<i32, Self::Error>;
    fn handler(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<Option<Self::Handler>, Self::Error>;
    fn can_be_interrupted(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        handler: Self::Handler,
    ) -> Result<bool, Self::Error>;
    /// Original definition FindIndex and IndexList validity/value semantics;
    /// a wire hash search alone is not a proven live implementation.
    fn operation_disabled(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<bool, Self::Error>;
    fn script_system(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
    ) -> Result<Option<Self::Script>, Self::Error>;
    fn args_begin(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
    ) -> Result<Self::Args, Self::Error>;
    fn args_push_int(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        args: &mut Self::Args,
        value: i32,
    ) -> Result<(), Self::Error>;
    /// Resolve the current original utility separately at every native fetch.
    /// Missing/unverified receiver state is unsupported, never a false lock bit.
    fn engine_utility(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
    ) -> Result<Self::EngineUtility, Self::Error>;
    /// Original utility +0x30; concrete host implementation refetches active
    /// controller and tests whether the observing thread owns its core lock.
    fn has_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<bool, Self::Error>;
    /// Original utility +0x38: RELEASE one held core-lock level before Lua.
    /// Concrete backend refetches the active controller independently.
    fn release_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error>;
    /// Original utility +0x40: ACQUIRE one core-lock level after normal Lua return.
    /// Concrete backend refetches the active controller independently.
    fn get_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error>;
    /// Retained original ScriptSystem +0x80, including faithful VM state,
    /// separate VM critical-section/protected-call behavior, actual callbacks
    /// and both return/out-value bits. This is the direct call, without utility
    /// wrapping. Presence alone proves neither a valid VM nor empty callbacks;
    /// unclosed providers must error. A native protected failure that returns
    /// normally must retain its actual bits, not become an unsupported error.
    fn script_call_test_all(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        script: Self::Script,
        args: &mut Self::Args,
        event: &'static str,
        initial_value: bool,
    ) -> Result<ScriptTestAllResult, Self::Error>;
    fn args_end(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        args: Self::Args,
    ) -> Result<(), Self::Error>;
    fn unit_plot(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
    ) -> Result<Option<Self::Plot>, Self::Error>;
    fn handler_can_start(
        &mut self,
        s: &mut ManagerQueryState<'_, U, Self::Parameters>,
        handler: Self::Handler,
        plot: Self::Plot,
        visible_only: bool,
        results: Option<&mut Self::Results>,
    ) -> Result<bool, Self::Error>;
}

/// Complete Lua::Utility::CallTestAll wrapper (0x2ce79b0). Retain the first lock
/// bit, not the first utility receiver: release/reacquire each refetch utility.
/// ScriptSystem identity remains the originally passed receiver across effects.
/// The native body has no exception-safe reacquire path. A provider error stops
/// before subsequent calls, preserving effects; it is not a native false result
/// or a claim to emulate C++ exceptions or Rust panic unwinding.
pub fn script_test_all<U: ManagerUnitState, Q: ManagerQueries<U>>(
    s: &mut ManagerQueryState<'_, U, Q::Parameters>,
    q: &mut Q,
    script: Q::Script,
    args: &mut Q::Args,
    event: &'static str,
    initial_value: bool,
) -> Result<ScriptTestAllResult, Q::Error> {
    let utility = q.engine_utility(s)?;
    let held = q.has_game_core_lock(s, utility)?;
    if held {
        let utility = q.engine_utility(s)?;
        q.release_game_core_lock(s, utility)?;
    }
    let result = q.script_call_test_all(s, script, args, event, initial_value)?;
    if held {
        let utility = q.engine_utility(s)?;
        q.get_game_core_lock(s, utility)?;
    }
    Ok(result)
}

/// Original Manager::CanCurrentOperationBeInterrupted (0x2da2a60). Only an
/// actual current-handler query can reject; absent/same operations return true.
fn current_can_be_interrupted<U: ManagerUnitState, Q: ManagerQueries<U>>(
    s: &mut ManagerQueryState<'_, U, Q::Parameters>,
    q: &mut Q,
    requested_id: i32,
) -> Result<bool, Q::Error> {
    if !s.unit.queue_nonempty() {
        return Ok(true);
    }
    let Some(hash) = s.unit.front_operation_hash() else {
        return Ok(true);
    };
    let current = q.runtime_id(s, hash)?;
    if current == requested_id {
        return Ok(true);
    }
    let Some(handler) = q.handler(s, current)? else {
        return Ok(true);
    };
    q.can_be_interrupted(s, handler)
}

/// Complete original Manager predicate ordering (0x2da20f0), conditional on
/// faithful live providers and the verified original manager implementation.
/// This is not action admission, a default gameplay provider or native parity.
/// Errors preserve prior effects; no transaction or implicit policy is added.
pub fn manager_can_start<U: ManagerUnitState, Q: ManagerQueries<U>>(
    unit: &mut U,
    operation: &mut ManagerOperation<Q::Parameters>,
    plot: Option<Q::Plot>,
    visible_only: bool,
    results: Option<&mut Q::Results>,
    q: &mut Q,
) -> Result<bool, ManagerQueryError<Q::Error>> {
    let mut s = ManagerQueryState { unit, operation };
    let run =
        |s: &mut ManagerQueryState<'_, U, Q::Parameters>, q: &mut Q| -> Result<bool, Q::Error> {
            if s.unit.operation_timer() > 0 {
                return Ok(false);
            }
            if !visible_only {
                let owner = s.unit.owner();
                if !q.allow_tactical_commands(s, owner)? {
                    return Ok(false);
                }
            }
            let owner = s.unit.owner();
            let player = q.player(s, owner)?;
            if visible_only {
                let _ = q.game(s)?;
            } else {
                if !q.turn_active(s, player)? {
                    let automation = q.processing_automation(s)?;
                    if automation != q.player_id(s, player)? {
                        return Ok(false);
                    }
                }
                let game = q.game(s)?;
                if q.player_human(s, player)? && q.paused(s, game)? {
                    return Ok(false);
                }
            }
            if s.unit.queue_nonempty() {
                let requested = s.operation.runtime_id;
                if !current_can_be_interrupted(s, q, requested)? {
                    return Ok(false);
                }
            }
            let requested = s.operation.runtime_id;
            if q.operation_disabled(s, requested)? {
                return Ok(false);
            }
            if let Some(script) = q.script_system(s)? {
                let mut args = q.args_begin(s)?;
                let test = (|| {
                    let owner = s.unit.owner();
                    q.args_push_int(s, &mut args, owner)?;
                    let id = s.unit.unit_id() as i32;
                    q.args_push_int(s, &mut args, id)?;
                    let requested = s.operation.runtime_id;
                    q.args_push_int(s, &mut args, requested)?;
                    script_test_all(s, q, script, &mut args, "CanStartOperation", false)
                })();
                // Manager has an original Args destructor cleanup path. Result
                // errors retain that explicit cleanup, without adding a utility
                // reacquire guarantee or claiming C++/panic unwind equivalence.
                let cleanup = q.args_end(s, args);
                let test = test?;
                cleanup?;
                if test.called && !test.value {
                    return Ok(false);
                }
            }
            Ok(true)
        };
    if !run(&mut s, q).map_err(ManagerQueryError::Provider)? {
        return Ok(false);
    }
    let plot = match plot {
        Some(plot) => plot,
        None => match q.unit_plot(&mut s).map_err(ManagerQueryError::Provider)? {
            Some(plot) => plot,
            None => return Ok(false),
        },
    };
    let requested = s.operation.runtime_id;
    let handler = q
        .handler(&mut s, requested)
        .map_err(ManagerQueryError::Provider)?
        .ok_or(ManagerQueryError::MissingRequestedHandler(requested))?;
    q.handler_can_start(&mut s, handler, plot, visible_only, results)
        .map_err(ManagerQueryError::Provider)
}

#[cfg(test)]
mod manager_tests {
    use super::*;
    #[derive(Default)]
    struct Subject {
        timer: i32,
        owner: i32,
        id: u32,
        queue: Option<u32>,
    }
    impl ManagerUnitState for Subject {
        fn operation_timer(&self) -> i32 {
            self.timer
        }
        fn owner(&self) -> i32 {
            self.owner
        }
        fn unit_id(&self) -> u32 {
            self.id
        }
        fn queue_nonempty(&self) -> bool {
            self.queue.is_some()
        }
        fn front_operation_hash(&self) -> Option<u32> {
            self.queue
        }
    }
    type State<'a> = ManagerQueryState<'a, Subject, ()>;
    struct Queries {
        trace: Vec<String>,
        stop: &'static str,
        script: bool,
        mutate: bool,
        active: bool,
        human: bool,
        automation: i32,
        current: i32,
        current_present: bool,
        requested_present: bool,
        plot: bool,
        script_answer: ScriptTestAllResult,
        lock_held: bool,
        current_utility: i32,
        current_script: i32,
        replace_globals: bool,
    }
    impl Default for Queries {
        fn default() -> Self {
            Self {
                trace: Vec::new(),
                stop: "",
                script: false,
                mutate: false,
                active: true,
                human: true,
                automation: 7,
                current: 19,
                current_present: true,
                requested_present: true,
                plot: true,
                lock_held: false,
                current_utility: 10,
                current_script: 1,
                replace_globals: false,
                script_answer: ScriptTestAllResult {
                    called: true,
                    value: true,
                },
            }
        }
    }
    impl Queries {
        fn note(&mut self, label: &str) {
            self.trace.push(label.into());
        }
    }
    impl ManagerQueries<Subject> for Queries {
        type Error = &'static str;
        type Parameters = ();
        type Player = i32;
        type Game = i32;
        type Script = i32;
        type EngineUtility = i32;
        type Args = Vec<i32>;
        type Handler = i32;
        type Plot = i32;
        type Results = Vec<i32>;
        fn allow_tactical_commands(
            &mut self,
            _: &mut State<'_>,
            owner: i32,
        ) -> Result<bool, Self::Error> {
            self.note(&format!("tactical:{owner}"));
            Ok(self.stop != "tactical")
        }
        fn player(&mut self, _: &mut State<'_>, owner: i32) -> Result<i32, Self::Error> {
            self.note(&format!("player:{owner}"));
            Ok(7)
        }
        fn turn_active(&mut self, _: &mut State<'_>, p: i32) -> Result<bool, Self::Error> {
            assert_eq!(p, 7);
            self.note("active");
            Ok(self.active)
        }
        fn processing_automation(&mut self, _: &mut State<'_>) -> Result<i32, Self::Error> {
            self.note("automation");
            Ok(self.automation)
        }
        fn player_id(&mut self, _: &mut State<'_>, p: i32) -> Result<i32, Self::Error> {
            self.note("player_id");
            Ok(p)
        }
        fn game(&mut self, _: &mut State<'_>) -> Result<i32, Self::Error> {
            self.note("game");
            Ok(3)
        }
        fn player_human(&mut self, _: &mut State<'_>, p: i32) -> Result<bool, Self::Error> {
            assert_eq!(p, 7);
            self.note("human");
            Ok(self.human)
        }
        fn paused(&mut self, _: &mut State<'_>, game: i32) -> Result<bool, Self::Error> {
            assert_eq!(game, 3);
            self.note("paused");
            Ok(self.stop == "paused")
        }
        fn runtime_id(&mut self, _: &mut State<'_>, hash: u32) -> Result<i32, Self::Error> {
            assert_eq!(hash, 0x580f2f68);
            self.note("resolve_current");
            Ok(self.current)
        }
        fn handler(&mut self, _: &mut State<'_>, id: i32) -> Result<Option<i32>, Self::Error> {
            self.note(&format!("handler:{id}"));
            Ok(
                if (id == self.current && !self.current_present)
                    || (id != self.current && !self.requested_present)
                {
                    None
                } else {
                    Some(id)
                },
            )
        }
        fn can_be_interrupted(&mut self, _: &mut State<'_>, _: i32) -> Result<bool, Self::Error> {
            self.note("interrupt");
            Ok(self.stop != "interrupt")
        }
        fn operation_disabled(&mut self, _: &mut State<'_>, id: i32) -> Result<bool, Self::Error> {
            self.note(&format!("disabled:{id}"));
            Ok(self.stop == "disabled")
        }
        fn script_system(&mut self, _: &mut State<'_>) -> Result<Option<i32>, Self::Error> {
            self.note("script");
            Ok(self.script.then_some(self.current_script))
        }
        fn args_begin(&mut self, _: &mut State<'_>) -> Result<Vec<i32>, Self::Error> {
            self.note("args_begin");
            Ok(vec![])
        }
        fn args_push_int(
            &mut self,
            s: &mut State<'_>,
            args: &mut Vec<i32>,
            value: i32,
        ) -> Result<(), Self::Error> {
            self.note(&format!("push:{value}"));
            args.push(value);
            if self.mutate {
                match args.len() {
                    1 => s.unit.id = u32::MAX,
                    2 => s.operation.runtime_id = 77,
                    _ => {}
                }
            }
            if self.stop == "push_error" {
                return Err("push");
            }
            Ok(())
        }
        fn engine_utility(&mut self, _: &mut State<'_>) -> Result<i32, Self::Error> {
            self.note(&format!("utility:{}", self.current_utility));
            if self.stop == "utility_error" {
                return Err("utility");
            }
            Ok(self.current_utility)
        }
        fn has_game_core_lock(
            &mut self,
            _: &mut State<'_>,
            utility: i32,
        ) -> Result<bool, Self::Error> {
            self.note(&format!("has:{utility}"));
            if self.stop == "has_error" {
                return Err("has");
            }
            if self.replace_globals {
                self.current_utility = 11;
            }
            Ok(self.lock_held)
        }
        fn release_game_core_lock(
            &mut self,
            _: &mut State<'_>,
            utility: i32,
        ) -> Result<(), Self::Error> {
            self.note(&format!("release:{utility}"));
            self.lock_held = false;
            if self.replace_globals {
                self.current_script = 2;
            }
            if self.stop == "release_error" {
                return Err("release");
            }
            Ok(())
        }
        fn get_game_core_lock(
            &mut self,
            _: &mut State<'_>,
            utility: i32,
        ) -> Result<(), Self::Error> {
            self.note(&format!("get:{utility}"));
            self.lock_held = true;
            if self.stop == "get_error" {
                return Err("get");
            }
            Ok(())
        }
        fn script_call_test_all(
            &mut self,
            s: &mut State<'_>,
            script: i32,
            _: &mut Vec<i32>,
            event: &'static str,
            initial: bool,
        ) -> Result<ScriptTestAllResult, Self::Error> {
            assert_eq!(script, 1);
            assert_eq!(event, "CanStartOperation");
            assert!(!initial);
            self.note("test_all");
            if self.replace_globals {
                self.current_utility = 12;
                self.current_script = 3;
            }
            if self.mutate {
                s.unit.timer = 100;
                s.unit.owner = 99;
            }
            if self.stop == "script_error" {
                return Err("script");
            }
            Ok(self.script_answer)
        }
        fn args_end(&mut self, s: &mut State<'_>, _: Vec<i32>) -> Result<(), Self::Error> {
            self.note("args_end");
            if self.mutate {
                s.operation.runtime_id = 88;
            }
            Ok(())
        }
        fn unit_plot(&mut self, _: &mut State<'_>) -> Result<Option<i32>, Self::Error> {
            self.note("plot");
            Ok(self.plot.then_some(4))
        }
        fn handler_can_start(
            &mut self,
            _: &mut State<'_>,
            handler: i32,
            plot: i32,
            visible: bool,
            results: Option<&mut Vec<i32>>,
        ) -> Result<bool, Self::Error> {
            self.note(&format!("can_start:{handler}:{plot}:{visible}"));
            if let Some(r) = results {
                r.push(handler);
            }
            Ok(self.stop != "handler")
        }
    }
    fn operation() -> ManagerOperation<()> {
        ManagerOperation {
            runtime_id: 36,
            directive: -1,
            parameters: (),
            flags: 0,
        }
    }
    fn subject() -> Subject {
        Subject {
            timer: 0,
            owner: 2,
            id: 5,
            queue: None,
        }
    }
    fn run(
        s: &mut Subject,
        q: &mut Queries,
        visible: bool,
    ) -> Result<bool, ManagerQueryError<&'static str>> {
        manager_can_start(s, &mut operation(), None, visible, None, q)
    }
    #[test]
    fn manager_order_and_early_rejection() {
        for (stop, tail) in [
            ("tactical", "tactical:2"),
            ("paused", "paused"),
            ("disabled", "disabled:36"),
            ("handler", "can_start:36:4:false"),
        ] {
            let mut q = Queries {
                stop,
                ..Default::default()
            };
            assert_eq!(run(&mut subject(), &mut q, false), Ok(false));
            assert_eq!(q.trace.last().unwrap(), tail);
        }
        let mut q = Queries::default();
        let mut s = subject();
        s.timer = 1;
        assert_eq!(run(&mut s, &mut q, true), Ok(false));
        assert!(q.trace.is_empty());
        s.timer = i32::MIN;
        assert_eq!(run(&mut s, &mut q, false), Ok(true));
        assert_eq!(
            q.trace,
            [
                "tactical:2",
                "player:2",
                "active",
                "game",
                "human",
                "paused",
                "disabled:36",
                "script",
                "plot",
                "handler:36",
                "can_start:36:4:false"
            ]
        );
    }
    #[test]
    fn manager_visible_automation_and_nonhuman_branches() {
        let mut q = Queries {
            stop: "paused",
            active: false,
            automation: -1,
            ..Default::default()
        };
        assert_eq!(run(&mut subject(), &mut q, true), Ok(true));
        assert_eq!(
            q.trace,
            [
                "player:2",
                "game",
                "disabled:36",
                "script",
                "plot",
                "handler:36",
                "can_start:36:4:true"
            ]
        );
        q.trace.clear();
        assert_eq!(run(&mut subject(), &mut q, false), Ok(false));
        assert_eq!(
            q.trace,
            [
                "tactical:2",
                "player:2",
                "active",
                "automation",
                "player_id"
            ]
        );
        q.trace.clear();
        q.automation = 7;
        q.human = false;
        assert_eq!(run(&mut subject(), &mut q, false), Ok(true));
        assert!(!q.trace.iter().any(|x| x == "paused"));
    }
    #[test]
    fn manager_interruption_uses_current_registry_identity() {
        for (current, present, stop, allowed) in [
            (36, true, "interrupt", true),
            (19, false, "interrupt", true),
            (19, true, "interrupt", false),
            (19, true, "", true),
        ] {
            let mut s = subject();
            s.queue = Some(0x580f2f68);
            let mut q = Queries {
                current,
                current_present: present,
                stop,
                ..Default::default()
            };
            assert_eq!(run(&mut s, &mut q, true), Ok(allowed));
            assert_eq!(
                q.trace.iter().any(|x| x == "interrupt"),
                current != 36 && present
            );
        }
    }
    #[test]
    fn manager_script_two_bits_cleanup_and_rereads() {
        for (called, value, allowed) in [
            (false, false, true),
            (false, true, true),
            (true, false, false),
            (true, true, true),
        ] {
            for lock_held in [false, true] {
                let mut q = Queries {
                    script: true,
                    lock_held,
                    script_answer: ScriptTestAllResult { called, value },
                    ..Default::default()
                };
                assert_eq!(run(&mut subject(), &mut q, true), Ok(allowed));
                let end = q.trace.iter().position(|x| x == "args_end").unwrap();
                assert_eq!(q.trace.iter().any(|x| x == "plot"), allowed);
                assert_eq!(q.trace.iter().any(|x| x == "get:10"), lock_held);
                assert_eq!(q.lock_held, lock_held);
                if allowed {
                    assert_eq!(q.trace[end + 1], "plot");
                }
            }
        }
        let mut q = Queries {
            script: true,
            mutate: true,
            ..Default::default()
        };
        let mut s = subject();
        let mut o = operation();
        let mut results = vec![];
        assert_eq!(
            manager_can_start(&mut s, &mut o, Some(9), false, Some(&mut results), &mut q),
            Ok(true)
        );
        assert!(q.trace.windows(7).any(|w| w
            == [
                "push:2",
                "push:-1",
                "push:77",
                "utility:10",
                "has:10",
                "test_all",
                "args_end"
            ]));
        assert_eq!(q.trace.last().unwrap(), "can_start:88:9:false");
        assert!(!q.trace.iter().any(|x| x == "plot"));
        assert_eq!((s.timer, s.owner, o.runtime_id), (100, 99, 88));
        assert_eq!(results, [88]);
        for (stop, error) in [("push_error", "push"), ("script_error", "script")] {
            let mut q = Queries {
                script: true,
                stop,
                ..Default::default()
            };
            assert_eq!(
                run(&mut subject(), &mut q, true),
                Err(ManagerQueryError::Provider(error))
            );
            assert_eq!(q.trace.last().unwrap(), "args_end");
        }
    }
    #[test]
    fn manager_script_utility_refetch_and_retained_script() {
        for held in [false, true] {
            let mut q = Queries {
                script: true,
                lock_held: held,
                replace_globals: true,
                ..Default::default()
            };
            assert_eq!(run(&mut subject(), &mut q, true), Ok(true));
            let mut expected = vec![
                "player:2",
                "game",
                "disabled:36",
                "script",
                "args_begin",
                "push:2",
                "push:5",
                "push:36",
                "utility:10",
                "has:10",
            ];
            if held {
                expected.extend(["utility:11", "release:11"]);
            }
            expected.push("test_all");
            if held {
                expected.extend(["utility:12", "get:12"]);
            }
            expected.extend(["args_end", "plot", "handler:36", "can_start:36:4:true"]);
            assert_eq!(q.trace, expected);
            // Has replaced the global utility; Release replaced the global
            // script; ScriptSystem call still asserted its captured receiver 1.
            assert_eq!(q.current_script, 3);
            assert_eq!(q.lock_held, held);
        }
    }
    #[test]
    fn manager_script_utility_errors_do_not_invent_reacquire() {
        for (stop, error, called, released, acquired) in [
            ("utility_error", "utility", false, false, false),
            ("has_error", "has", false, false, false),
            ("release_error", "release", false, true, false),
            ("script_error", "script", true, true, false),
            ("get_error", "get", true, true, true),
        ] {
            let mut q = Queries {
                script: true,
                lock_held: true,
                replace_globals: true,
                stop,
                ..Default::default()
            };
            assert_eq!(
                run(&mut subject(), &mut q, true),
                Err(ManagerQueryError::Provider(error))
            );
            assert_eq!(q.trace.last().unwrap(), "args_end");
            assert_eq!(q.trace.iter().any(|v| v == "test_all"), called);
            assert_eq!(q.trace.iter().any(|v| v == "release:11"), released);
            assert_eq!(q.trace.iter().any(|v| v == "get:12"), acquired);
            assert_eq!(
                q.trace.iter().filter(|v| v.starts_with("get:")).count(),
                usize::from(acquired)
            );
            assert_eq!(q.lock_held, !released || acquired);
            assert!(!q.trace.iter().any(|v| v == "plot"));
        }
    }
    #[test]
    fn manager_missing_plot_and_requested_handler_are_distinct() {
        let mut q = Queries {
            plot: false,
            ..Default::default()
        };
        assert_eq!(run(&mut subject(), &mut q, true), Ok(false));
        assert_eq!(q.trace.last().unwrap(), "plot");
        let mut q = Queries {
            requested_present: false,
            ..Default::default()
        };
        assert_eq!(
            run(&mut subject(), &mut q, true),
            Err(ManagerQueryError::MissingRequestedHandler(36))
        );
    }
}
