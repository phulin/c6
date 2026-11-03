//! Compose computed registry execution with existing Manager queries.
//!
//! Receiver, argument and synchronization semantics remain with the environment.
//! Only direct TestAll is computed here. Its owned collector state evolves across
//! calls; it is not a replayed oracle result. Unsupported intern, growth, listener
//! or collector paths refuse before changing the logical registry or collector.
use c6_core::{
    script_event_registry::{
        CachedExecutionState, FactoryExecutionState, RegistryError, ScriptEventRegistry,
    },
    skip_turn::legality::{ManagerQueries, ManagerQueryState, ScriptTestAllResult},
    unit::Unit,
};

enum RegistryAccess<'a> {
    Cached(&'a ScriptEventRegistry),
    Factory(&'a mut ScriptEventRegistry, &'a mut FactoryExecutionState),
}

pub struct RegistryQueries<'a, Q: ManagerQueries<Unit>> {
    delegate: &'a mut Q,
    registry: RegistryAccess<'a>,
    execution: &'a mut CachedExecutionState,
    script: Q::Script,
    last_error: Option<RegistryError>,
}
impl<'a, Q: ManagerQueries<Unit>> RegistryQueries<'a, Q> {
    /// `script` is the canonical logical receiver for this registry. Input
    /// attribution is separate from this typed consistency check.
    pub fn new(
        delegate: &'a mut Q,
        registry: &'a ScriptEventRegistry,
        execution: &'a mut CachedExecutionState,
        script: Q::Script,
    ) -> Self {
        Self {
            delegate,
            registry: RegistryAccess::Cached(registry),
            execution,
            script,
            last_error: None,
        }
    }
    /// Enables the bounded factory using persistent initial capacity facts and
    /// retained created-object obligations. Nothing is reconstructed per call.
    pub fn with_factory(
        delegate: &'a mut Q,
        registry: &'a mut ScriptEventRegistry,
        execution: &'a mut CachedExecutionState,
        factory: &'a mut FactoryExecutionState,
        script: Q::Script,
    ) -> Self {
        Self {
            delegate,
            registry: RegistryAccess::Factory(registry, factory),
            execution,
            script,
            last_error: None,
        }
    }
    pub fn last_registry_error(&self) -> Option<&RegistryError> {
        self.last_error.as_ref()
    }
}
impl<Q> ManagerQueries<Unit> for RegistryQueries<'_, Q>
where
    Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>,
    Q::Script: PartialEq,
    Q::Args: AsRef<[i32]>,
{
    type Error = &'static str;
    type Parameters = ();
    type Player = Q::Player;
    type Game = Q::Game;
    type Script = Q::Script;
    type EngineUtility = Q::EngineUtility;
    type Args = Q::Args;
    type Handler = Q::Handler;
    type Plot = Q::Plot;
    type Results = Q::Results;
    fn allow_tactical_commands(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        owner: i32,
    ) -> Result<bool, Self::Error> {
        self.delegate.allow_tactical_commands(s, owner)
    }
    fn player(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        owner: i32,
    ) -> Result<Self::Player, Self::Error> {
        self.delegate.player(s, owner)
    }
    fn turn_active(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        self.delegate.turn_active(s, player)
    }
    fn processing_automation(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<i32, Self::Error> {
        self.delegate.processing_automation(s)
    }
    fn player_id(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<i32, Self::Error> {
        self.delegate.player_id(s, player)
    }
    fn game(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::Game, Self::Error> {
        self.delegate.game(s)
    }
    fn player_human(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        self.delegate.player_human(s, player)
    }
    fn paused(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        game: Self::Game,
    ) -> Result<bool, Self::Error> {
        self.delegate.paused(s, game)
    }
    fn runtime_id(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        hash: u32,
    ) -> Result<i32, Self::Error> {
        self.delegate.runtime_id(s, hash)
    }
    fn handler(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<Option<Self::Handler>, Self::Error> {
        self.delegate.handler(s, runtime_id)
    }
    fn can_be_interrupted(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        handler: Self::Handler,
    ) -> Result<bool, Self::Error> {
        self.delegate.can_be_interrupted(s, handler)
    }
    fn operation_disabled(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<bool, Self::Error> {
        self.delegate.operation_disabled(s, runtime_id)
    }
    fn script_system(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Option<Self::Script>, Self::Error> {
        self.delegate.script_system(s)
    }
    fn args_begin(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::Args, Self::Error> {
        self.delegate.args_begin(s)
    }
    fn args_push_int(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        args: &mut Self::Args,
        value: i32,
    ) -> Result<(), Self::Error> {
        self.delegate.args_push_int(s, args, value)
    }
    fn engine_utility(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::EngineUtility, Self::Error> {
        self.delegate.engine_utility(s)
    }
    fn has_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<bool, Self::Error> {
        self.delegate.has_game_core_lock(s, utility)
    }
    fn release_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        self.delegate.release_game_core_lock(s, utility)
    }
    fn get_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        self.delegate.get_game_core_lock(s, utility)
    }
    fn script_call_test_all(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        script: Self::Script,
        args: &mut Self::Args,
        event: &'static str,
        initial_value: bool,
    ) -> Result<ScriptTestAllResult, Self::Error> {
        if script != self.script {
            return Err("registry ScriptSystem identity mismatch");
        }
        let arguments: [i32; 3] = args
            .as_ref()
            .try_into()
            .map_err(|_| "registry requires the actual three Manager arguments")?;
        if arguments != [s.unit.e_owner, s.unit.id as i32, s.operation.runtime_id] {
            return Err("registry Manager argument identity mismatch");
        }
        let answer = match &mut self.registry {
            RegistryAccess::Cached(registry) => {
                registry.test_all_cached(event, arguments, self.execution)
            }
            RegistryAccess::Factory(registry, factory) => {
                registry.test_all_bounded(event, arguments, self.execution, factory)
            }
        };
        match answer {
            Ok(value) => Ok(ScriptTestAllResult {
                called: value.is_some(),
                value: value.unwrap_or(initial_value),
            }),
            Err(error) => {
                self.last_error = Some(error);
                Err("registry execution unsupported")
            }
        }
    }
    fn args_end(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        args: Self::Args,
    ) -> Result<(), Self::Error> {
        self.delegate.args_end(s, args)
    }
    fn unit_plot(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Option<Self::Plot>, Self::Error> {
        self.delegate.unit_plot(s)
    }
    fn handler_can_start(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        handler: Self::Handler,
        plot: Self::Plot,
        visible_only: bool,
        results: Option<&mut Self::Results>,
    ) -> Result<bool, Self::Error> {
        self.delegate
            .handler_can_start(s, handler, plot, visible_only, results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::script_event_registry::{
        AppendOrder, CachedCallCapacity, GroupEntry, KnownNameEntry, LocalGroup, RegistrySeed,
        ScriptObjectId, SharedGroup,
    };
    use c6_core::skip_turn::legality::ManagerOperation;
    use std::collections::BTreeMap;
    // Source-only environment probe. No native handle/lock behavior is claimed.
    struct Environment;
    #[allow(unused_variables)]
    impl ManagerQueries<Unit> for Environment {
        type Error = &'static str;
        type Parameters = ();
        type Player = ();
        type Game = ();
        type Script = u64;
        type EngineUtility = ();
        type Args = Vec<i32>;
        type Handler = ();
        type Plot = ();
        type Results = ();
        fn allow_tactical_commands(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            owner: i32,
        ) -> Result<bool, Self::Error> {
            Err("source-only unused environment")
        }
        fn player(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            owner: i32,
        ) -> Result<Self::Player, Self::Error> {
            Err("source-only unused environment")
        }
        fn turn_active(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            player: Self::Player,
        ) -> Result<bool, Self::Error> {
            Err("source-only unused environment")
        }
        fn processing_automation(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        ) -> Result<i32, Self::Error> {
            Err("source-only unused environment")
        }
        fn player_id(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            player: Self::Player,
        ) -> Result<i32, Self::Error> {
            Err("source-only unused environment")
        }
        fn game(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        ) -> Result<Self::Game, Self::Error> {
            Err("source-only unused environment")
        }
        fn player_human(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            player: Self::Player,
        ) -> Result<bool, Self::Error> {
            Err("source-only unused environment")
        }
        fn paused(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            game: Self::Game,
        ) -> Result<bool, Self::Error> {
            Err("source-only unused environment")
        }
        fn runtime_id(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            hash: u32,
        ) -> Result<i32, Self::Error> {
            Err("source-only unused environment")
        }
        fn handler(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            runtime_id: i32,
        ) -> Result<Option<Self::Handler>, Self::Error> {
            Err("source-only unused environment")
        }
        fn can_be_interrupted(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            handler: Self::Handler,
        ) -> Result<bool, Self::Error> {
            Err("source-only unused environment")
        }
        fn operation_disabled(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            runtime_id: i32,
        ) -> Result<bool, Self::Error> {
            Err("source-only unused environment")
        }
        fn script_system(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        ) -> Result<Option<Self::Script>, Self::Error> {
            Err("source-only unused environment")
        }
        fn args_begin(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        ) -> Result<Self::Args, Self::Error> {
            Err("source-only unused environment")
        }
        fn args_push_int(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            args: &mut Self::Args,
            value: i32,
        ) -> Result<(), Self::Error> {
            Err("source-only unused environment")
        }
        fn engine_utility(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        ) -> Result<Self::EngineUtility, Self::Error> {
            Err("source-only unused environment")
        }
        fn has_game_core_lock(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            utility: Self::EngineUtility,
        ) -> Result<bool, Self::Error> {
            Err("source-only unused environment")
        }
        fn release_game_core_lock(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            utility: Self::EngineUtility,
        ) -> Result<(), Self::Error> {
            Err("source-only unused environment")
        }
        fn get_game_core_lock(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            utility: Self::EngineUtility,
        ) -> Result<(), Self::Error> {
            Err("source-only unused environment")
        }
        fn script_call_test_all(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            script: Self::Script,
            args: &mut Self::Args,
            event: &'static str,
            initial_value: bool,
        ) -> Result<ScriptTestAllResult, Self::Error> {
            panic!("registry result must be computed, not delegated")
        }
        fn args_end(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            args: Self::Args,
        ) -> Result<(), Self::Error> {
            Err("source-only unused environment")
        }
        fn unit_plot(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        ) -> Result<Option<Self::Plot>, Self::Error> {
            Err("source-only unused environment")
        }
        fn handler_can_start(
            &mut self,
            s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
            handler: Self::Handler,
            plot: Self::Plot,
            visible_only: bool,
            results: Option<&mut Self::Results>,
        ) -> Result<bool, Self::Error> {
            Err("source-only unused environment")
        }
    }
    fn fixture() -> (ScriptEventRegistry, CachedExecutionState) {
        let identity = c6_core::script_event_registry::EventIdentity {
            event: ScriptObjectId(3),
            shared_group: ScriptObjectId(1),
            local_group: ScriptObjectId(2),
            test_all: ScriptObjectId(9),
            methods: std::array::from_fn(|i| ScriptObjectId(4 + i as u64)),
        };
        let seed = RegistrySeed {
            names: [(
                "CanStartOperation".into(),
                KnownNameEntry::Shared(ScriptObjectId(1)),
            )]
            .into(),
            shared_groups: [(
                ScriptObjectId(1),
                SharedGroup {
                    entries_in_native_order: vec![GroupEntry::Table(ScriptObjectId(2))],
                    append_order: AppendOrder::DenseNumericArray,
                },
            )]
            .into(),
            local_groups: [(
                ScriptObjectId(2),
                LocalGroup {
                    objlen: 0,
                    numeric_slots: BTreeMap::new(),
                },
            )]
            .into(),
        };
        (
            ScriptEventRegistry::from_seed_with_cache(
                seed,
                [("CanStartOperation".into(), identity)].into(),
            )
            .unwrap(),
            CachedExecutionState {
                call_capacity: CachedCallCapacity::NoGrowthForThreeIntegers,
                collector_phase: 0,
                step_countdown: 3,
                allocated_bytes: 100,
                peak_allocated_bytes: 100,
                collection_threshold: 100,
                collection_inhibited: false,
                interned_names: ["GameEvents", "CanStartOperation", "TestAll"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                protected_closure_obligations: 0,
            },
        )
    }
    #[test]
    fn computed_nil_preserves_out_value_and_evolves_collector_until_frontier() {
        let (registry, mut execution) = fixture();
        let mut environment = Environment;
        let mut unit = crate::loaded_actions::tests::synthetic_unit();
        let mut operation = ManagerOperation {
            runtime_id: 36,
            directive: -1,
            parameters: (),
            flags: 0,
        };
        let mut args = vec![unit.e_owner, unit.id as i32, 36];
        let mut state = ManagerQueryState {
            unit: &mut unit,
            operation: &mut operation,
        };
        {
            let mut query = RegistryQueries::new(&mut environment, &registry, &mut execution, 91);
            assert_eq!(
                query.script_call_test_all(&mut state, 91, &mut args, "CanStartOperation", true),
                Ok(ScriptTestAllResult {
                    called: false,
                    value: true
                })
            );
            assert_eq!(
                query.script_call_test_all(&mut state, 91, &mut args, "CanStartOperation", false),
                Ok(ScriptTestAllResult {
                    called: false,
                    value: false
                })
            );
        }
        assert_eq!(execution.allocated_bytes, 196);
        assert_eq!(execution.protected_closure_obligations, 2);
        let before = execution.clone();
        let mut query = RegistryQueries::new(&mut environment, &registry, &mut execution, 91);
        assert!(query
            .script_call_test_all(&mut state, 91, &mut args, "CanStartOperation", false)
            .is_err());
        assert_eq!(
            query.last_registry_error(),
            Some(&RegistryError::UnsupportedEffect("collector progression"))
        );
        assert_eq!(execution, before);
    }
    #[test]
    fn wrong_receiver_or_arguments_refuse_before_registry_effects() {
        let (registry, mut execution) = fixture();
        let before = execution.clone();
        let mut environment = Environment;
        let mut unit = crate::loaded_actions::tests::synthetic_unit();
        let mut operation = ManagerOperation {
            runtime_id: 36,
            directive: -1,
            parameters: (),
            flags: 0,
        };
        let mut args = vec![unit.e_owner, unit.id as i32, 36];
        let mut state = ManagerQueryState {
            unit: &mut unit,
            operation: &mut operation,
        };
        let mut query = RegistryQueries::new(&mut environment, &registry, &mut execution, 91);
        assert!(query
            .script_call_test_all(&mut state, 92, &mut args, "CanStartOperation", false)
            .is_err());
        args[2] = 35;
        assert!(query
            .script_call_test_all(&mut state, 91, &mut args, "CanStartOperation", false)
            .is_err());
        assert_eq!(execution, before);
    }
    fn factory_fixture() -> (
        ScriptEventRegistry,
        CachedExecutionState,
        FactoryExecutionState,
    ) {
        let registry = ScriptEventRegistry::from_seed(RegistrySeed {
            names: [("CanStartOperation".into(), KnownNameEntry::Absent)].into(),
            ..RegistrySeed::default()
        })
        .unwrap();
        let (_, mut execution) = fixture();
        execution.call_capacity = CachedCallCapacity::NoGrowthForFactoryThreeIntegers;
        execution.step_countdown = 1000;
        execution.interned_names.extend(
            [
                "GameEvent::lCreate",
                "func",
                "Add",
                "Count",
                "Remove",
                "RemoveAll",
                "Call",
                "Accumulate",
                "AccumulateINT",
                "TestAny",
                "__newindex",
                "__metatable",
                "__mode",
                "v",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        let factory = FactoryExecutionState {
            game_event_spare_names: ["CanStartOperation".into()].into(),
            name_map_spare_names: ["CanStartOperation".into()].into(),
            ..FactoryExecutionState::default()
        };
        (registry, execution, factory)
    }
    #[test]
    fn factory_query_publishes_once_then_computes_cached_reuse() {
        let (mut registry, mut execution, mut factory) = factory_fixture();
        let mut environment = Environment;
        let mut unit = crate::loaded_actions::tests::synthetic_unit();
        let mut operation = ManagerOperation {
            runtime_id: 36,
            directive: -1,
            parameters: (),
            flags: 0,
        };
        let mut args = vec![unit.e_owner, unit.id as i32, 36];
        let mut state = ManagerQueryState {
            unit: &mut unit,
            operation: &mut operation,
        };
        {
            let mut query = RegistryQueries::with_factory(
                &mut environment,
                &mut registry,
                &mut execution,
                &mut factory,
                91,
            );
            assert_eq!(
                query.script_call_test_all(&mut state, 91, &mut args, "CanStartOperation", false),
                Ok(ScriptTestAllResult {
                    called: false,
                    value: false
                })
            );
        }
        let identity = registry.cached_event("CanStartOperation").unwrap();
        assert_eq!(execution.step_countdown, 964);
        assert_eq!(execution.allocated_bytes, 2028);
        assert_eq!(execution.peak_allocated_bytes, 2060);
        assert_eq!(factory.factory_objects.len(), 1);
        assert_eq!(factory.factory_objects[0].event, identity);
        assert!(factory.game_event_spare_names.is_empty());
        {
            let mut query = RegistryQueries::with_factory(
                &mut environment,
                &mut registry,
                &mut execution,
                &mut factory,
                91,
            );
            assert_eq!(
                query.script_call_test_all(&mut state, 91, &mut args, "CanStartOperation", true),
                Ok(ScriptTestAllResult {
                    called: false,
                    value: true
                })
            );
        }
        assert_eq!(registry.cached_event("CanStartOperation"), Some(identity));
        assert_eq!(factory.factory_objects.len(), 1);
        assert_eq!(execution.step_countdown, 963);
        assert_eq!(execution.allocated_bytes, 2076);
        assert_eq!(execution.peak_allocated_bytes, 2076);
        assert_eq!(execution.protected_closure_obligations, 2);
    }
    #[test]
    fn missing_factory_capacity_preserves_registry_and_collector() {
        let (mut registry, mut execution, mut factory) = factory_fixture();
        factory.game_event_spare_names.clear();
        let before = (registry.clone(), execution.clone(), factory.clone());
        let mut environment = Environment;
        let mut unit = crate::loaded_actions::tests::synthetic_unit();
        let mut operation = ManagerOperation {
            runtime_id: 36,
            directive: -1,
            parameters: (),
            flags: 0,
        };
        let mut args = vec![unit.e_owner, unit.id as i32, 36];
        let mut state = ManagerQueryState {
            unit: &mut unit,
            operation: &mut operation,
        };
        {
            let mut query = RegistryQueries::with_factory(
                &mut environment,
                &mut registry,
                &mut execution,
                &mut factory,
                91,
            );
            assert!(query
                .script_call_test_all(&mut state, 91, &mut args, "CanStartOperation", false)
                .is_err());
            assert_eq!(
                query.last_registry_error(),
                Some(&RegistryError::UnsupportedEffect(
                    "unknown GameEvents insertion growth"
                ))
            );
        }
        assert_eq!((registry, execution, factory), before);
    }
}
