//! Consuming current-Core ownership for state-backed queries and future actions.
//! This is not complete native Read or full action admission.
use crate::core_player::{self, initialization::WireBinding, PlayerInitializations};
use crate::core_stream::{ThroughRandom, ThroughRandomError};
use crate::formation_initialization::FormationRuntime;
use crate::loaded_components::{self, OwnedCore, RetainedPlayer};
use crate::unit_initialization::{InitializationInputs, UnitRuntime};
use crate::{map, unit, world};
use std::sync::Arc;

/// Unique input storage moves without copying; explicitly shared immutable
/// storage remains shared. Neither variant permits an implicit World clone.
#[derive(Debug)]
pub enum CoreImage {
    Unique(Vec<u8>),
    Shared(Arc<[u8]>),
}

#[cfg(test)]
mod tests {
    use super::*;
    // Private source-only state carrier: no public fabricated native factory.
    fn state() -> LoadedState {
        let (bytes, _, _) = crate::core_stream::tests::random_prefix_fixture();
        let binding = crate::core_stream::tests::context_binding(&bytes);
        let contexts = |_: core_player::initialization::PlayerIdentity| None;
        let parsed = ThroughRandom::decode(
            &bytes,
            map::Limits::default(),
            unit::Limits::default(),
            binding.clone(),
            &contexts,
        )
        .unwrap();
        let (mut map, core) = loaded_components::split_core(parsed.global_ai.players.core);
        map.header.width = 1;
        map.header.height = 1;
        map.plots = vec![map::Plot::decode(&[0; 55], 6, map::Limits::default()).unwrap()];
        map.units.width = 1;
        map.units.height = 1;
        let mut player = crate::core_player::tests::source_first_player();
        player.formations.capacity = 2;
        player.formations.entries = vec![map::Slot {
            index: 1,
            value: unit::Formation {
                id: 0x10001,
                owner: player.header.owner,
                location: unit::PlotCoord { x: 0, y: 0 },
                moves_remaining_raw: 0,
                domain: 0,
                is_embarked: false,
                can_embark: false,
                can_attack: false,
                is_ranged_combat: false,
                can_capture: false,
                has_moved: false,
                has_moved_into_zoc: false,
                ignores_zoc: false,
                last_path: vec![],
                last_path_cache_location: unit::PlotCoord { x: 0, y: 0 },
                unit_ids: vec![],
            },
        }];
        let (player, retained) = loaded_components::split_player(player).unwrap();
        let world = world::World::new(map, vec![player]).unwrap();
        let statistics = ImportStatistics {
            players: 1,
            units: world.players()[0].unit_records().unwrap().len(),
            formations: world.players()[0].formation_records().unwrap().len(),
            consumed: parsed.consumed,
            items_used: parsed.items_used,
        };
        let retained = RetainedComponents {
            core,
            religion: parsed.global_ai.players.player_prefix.religion,
            players: vec![retained],
            global_ai: parsed.global_ai.global_ai,
            barbarian: parsed.barbarian,
            free_cities: parsed.free_cities,
            random: parsed.random,
        };
        LoadedState {
            image: CoreImage::Unique(bytes),
            binding,
            world,
            retained,
            statistics,
            controls: None,
            runtime: None,
            formation_runtime: None,
            limits: AssemblyLimits::default(),
            revision: 0,
            observed_snapshot: None,
            game_relations: None,
            game_relations_source: None,
            diplomacy_manager: None,
            diplomacy_manager_source: None,
            player_notifications: None,
            player_notifications_source: None,
            game_techs: None,
            game_techs_source: None,
            trade_graph: None,
            trade_graph_source: None,
        }
    }
    fn controls(state: &LoadedState) -> ControlInputs {
        let owner = state.world.players()[0].owner();
        let mut slots = [false; 65];
        slots[owner as usize] = true;
        let mut mapping = [-1; 65];
        mapping[owner as usize] = 0;
        ControlInputs {
            binding: state.binding.clone(),
            player_mapping: mapping,
            player_slots: slots,
            players: vec![PlayerControls {
                owner,
                slot_status: 3,
                team: 4,
                is_alive: true,
                turn_active: false,
            }],
            processing_automation: Some(-1),
            pause_storage: Some(0),
        }
    }
    #[test]
    fn explicit_image_sharing_never_copies_bytes() {
        let bytes: Arc<[u8]> = Arc::from([1u8, 2, 3]);
        let pointer = bytes.as_ptr();
        let image = CoreImage::Shared(Arc::clone(&bytes));
        assert_eq!(image.as_ref().as_ptr(), pointer);
        assert_eq!(Arc::strong_count(&bytes), 2);
        let raw = vec![4, 5, 6];
        let pointer = raw.as_ptr();
        assert_eq!(CoreImage::Unique(raw).as_ref().as_ptr(), pointer);
    }
    #[test]
    fn live_controls_are_bound_atomic_and_never_inferred_from_saved_membership() {
        let mut state = state();
        let owner = state.world.players()[0].owner();
        assert!(state.world.player_mapping.is_none());
        assert!(state.world.players()[0].is_alive.is_none());
        assert!(matches!(
            state.turn_active(owner),
            Err(Error::Missing(MissingInput::ManagerSlots))
        ));
        assert!(matches!(
            state.processing_automation(),
            Err(Error::Missing(MissingInput::ProcessingAutomation))
        ));
        let mut bad = controls(&state);
        bad.binding.occurrence_id.push(1);
        assert!(matches!(state.bind_controls(bad), Err(Error::Binding)));
        assert!(state.world.player_mapping.is_none());
        let mut bad = controls(&state);
        bad.players.push(bad.players[0]);
        assert!(matches!(state.bind_controls(bad), Err(Error::Binding)));
        assert!(state.world.players()[0].is_alive.is_none());
        let actual = controls(&state);
        state.bind_controls(actual).unwrap();
        assert!(!state.turn_active(owner).unwrap());
        assert_eq!(state.processing_automation().unwrap(), -1);
        assert_eq!(state.world.player(owner).unwrap().team, Some(4));
        let mut absent = controls(&state);
        absent.players.clear();
        absent.pause_storage = None;
        assert!(matches!(
            state.bind_controls(absent),
            Err(Error::AlreadyBound)
        ));
        assert_eq!(state.world.player(owner).unwrap().is_alive, Some(true));
        assert_eq!(state.pause_storage().unwrap(), 0);
        assert!(matches!(
            state.unit_runtime(owner, 0),
            Err(Error::Missing(MissingInput::UnitRuntime))
        ));
    }
    fn witness(s: &LoadedState) -> ObservedSnapshotWitness {
        ObservedSnapshotWitness {
            source_role: core_player::initialization::WireRole::SavedOutputCore,
            snapshot_sha256: s.binding.core_sha256,
            snapshot_bytes: s.source_image().len(),
            capture_runtime_sha256: [7; 32],
            capture_facts_sha256: [8; 32],
            capture_configuration_sha256: [9; 32],
        }
    }
    #[test]
    fn snapshot_authority_is_one_shot_and_decoder_role_stays_separate() {
        let mut s = state();
        let before = s.binding.clone();
        let mut bad = witness(&s);
        bad.snapshot_bytes += 1;
        assert!(matches!(
            s.bind_observed_snapshot_controls(bad, controls(&s)),
            Err(Error::Binding)
        ));
        assert!(s.observed_snapshot().is_none());
        let w = witness(&s);
        s.bind_observed_snapshot_controls(w.clone(), controls(&s))
            .unwrap();
        assert_eq!(s.binding, before);
        assert_eq!(s.observed_snapshot(), Some(&w));
        assert!(matches!(
            s.bind_observed_snapshot_controls(w, controls(&s)),
            Err(Error::AlreadyBound)
        ));
        let mut advanced = state();
        advanced.initialize_formations().unwrap();
        let owner = advanced.world.players()[0].owner();
        advanced
            .formation_with_runtime_mut(owner, 0x20001)
            .unwrap()
            .0
            .moves_remaining_raw = 7;
        assert_eq!(advanced.revision(), 1);
        assert!(matches!(
            advanced.bind_observed_snapshot_controls(witness(&advanced), controls(&advanced)),
            Err(Error::NotPristine)
        ));
    }
    #[test]
    fn formation_initialization_needs_no_unrelated_controls_and_never_resets_live_changes() {
        let mut s = state();
        let owner = s.world.players()[0].owner();
        s.initialize_formations().unwrap();
        let (f, r) = s.formation_with_runtime_mut(owner, 0x90001).unwrap();
        f.moves_remaining_raw = 123;
        r.archive.mark_dirty(14).unwrap();
        assert!(matches!(
            s.initialize_formations(),
            Err(Error::AlreadyInitialized)
        ));
        assert_eq!(
            s.world
                .player(owner)
                .unwrap()
                .formation(1)
                .unwrap()
                .unwrap()
                .record
                .moves_remaining_raw,
            123
        );
        assert!(s
            .formation_runtime(owner, 1)
            .unwrap()
            .archive
            .dirty_calls()
            .contains(&14));
    }
    #[test]
    fn unit_initialization_is_one_shot_and_first_controls_do_not_discard_it() {
        struct Empty;
        impl crate::movement_world::DefinitionLookup for Empty {
            fn count(&self) -> usize {
                0
            }
            fn find_index(&self, _: u32) -> Option<usize> {
                None
            }
            fn type_hash(&self, _: usize) -> Option<u32> {
                None
            }
        }
        let mut s = state();
        let identity = s.binding.inputs.clone();
        let empty = Empty;
        let properties: Vec<crate::unit_initialization::UnitDefinition> = vec![];
        let inputs = InitializationInputs {
            identities: &identity,
            units: crate::movement_world::DefinitionTable::new(&empty, &properties).unwrap(),
            abilities: &empty,
            terrains: &empty,
            features: &empty,
            great_person_classes: &empty,
            operations: &empty,
        };
        assert!(s.controls.is_none());
        s.initialize_rosters(&inputs).unwrap();
        let pointer = s.runtime.as_ref().unwrap().as_ptr();
        assert!(matches!(
            s.initialize_rosters(&inputs),
            Err(Error::AlreadyInitialized)
        ));
        s.bind_controls(controls(&s)).unwrap();
        assert_eq!(s.runtime.as_ref().unwrap().as_ptr(), pointer);
        assert!(matches!(
            s.bind_controls(controls(&s)),
            Err(Error::AlreadyBound)
        ));
    }
    fn diplomacy_state() -> LoadedState {
        let mut state = state();
        state.retained.players[0].header.ordered_flags[1] = true;
        let witness = witness(&state);
        let mut controls = controls(&state);
        controls.players[0].slot_status = 2; // source-only AI configuration
        state
            .bind_observed_snapshot_controls(witness.clone(), controls)
            .unwrap();
        let owner = state.world.players()[0].owner();
        state.install_diplomacy_manager(crate::diplomacy_manager::PendingStateSnapshot {
            witness,
            ever_alive: vec![crate::diplomacy_manager::PlayerHandle { owner }],
            allocation_slots: {
                let mut slots = vec![None; 65];
                slots[0] = Some(crate::diplomacy_manager::PlayerHandle { owner });
                slots
            },
            open_sessions: crate::diplomacy_manager::EmptyOpenSessions::from_has_pending_session_statements(false).unwrap(),
            pending_actions: vec![],
        }).unwrap();
        state
    }
    #[test]
    fn diplomacy_snapshot_requires_current_dynamic_context_and_cannot_replay() {
        let mut state = state();
        let w = witness(&state);
        let snapshot = || crate::diplomacy_manager::PendingStateSnapshot {
            witness: w.clone(),
            ever_alive: vec![],
            allocation_slots: vec![None; 65],
            open_sessions:
                crate::diplomacy_manager::EmptyOpenSessions::from_has_pending_session_statements(
                    false,
                )
                .unwrap(),
            pending_actions: vec![],
        };
        assert!(matches!(
            state.install_diplomacy_manager(snapshot()),
            Err(Error::Binding)
        ));
        assert!(state.diplomacy_manager().is_err());
        let mut state = diplomacy_state();
        let source = state.diplomacy_manager_source().unwrap().clone();
        assert_eq!(state.revision(), 0);
        assert!(matches!(
            state.install_diplomacy_manager(snapshot()),
            Err(Error::AlreadyBound)
        ));
        state
            .diplomacy_manager_mut()
            .unwrap()
            .pending_actions_mut()
            .push(crate::diplomacy_manager::PendingAction {
                id: 9,
                from: 0,
                to: 1,
            });
        assert_eq!(state.diplomacy_manager_source(), Some(&source));
        assert_eq!(
            state.diplomacy_manager().unwrap().pending_actions().len(),
            1
        );
    }
    struct StatementProcessor {
        seen: Vec<u64>,
        owner: i32,
    }
    impl crate::diplomacy_manager::ProcessStatement<LoadedState> for StatementProcessor {
        fn process_statement(
            &mut self,
            world: &mut LoadedState,
            statement: crate::diplomacy_manager::StatementHandle,
        ) -> Result<(), crate::unit_operation_aftermath::Error> {
            use crate::diplomacy_manager::StatementHandle;
            use crate::unit_operation_aftermath::{Pending, Session};
            self.seen.push(statement.0);
            if statement.0 == 10 {
                // Reentrant callback populates an earlier session. Next attempt
                // must restart at the current head, not retain an old iterator.
                world
                    .diplomacy_manager_mut()
                    .unwrap()
                    .open_sessions_mut()
                    .insert(
                        0,
                        Session {
                            from: self.owner,
                            to: 3,
                            pending: vec![Pending {
                                from: self.owner,
                                to: 3,
                                statement: StatementHandle(11),
                            }],
                        },
                    );
                Ok(())
            } else {
                Err(
                    crate::unit_operation_aftermath::Error::ExternalDecisionRequired(
                        "actual Statement subtype decision",
                    ),
                )
            }
        }
    }
    #[test]
    fn diplomacy_drain_is_state_backed_and_requires_real_statement_provider() {
        use crate::diplomacy_manager::StatementHandle;
        use crate::unit_operation_aftermath::{Error as AftermathError, Pending, Session};
        let mut state = diplomacy_state();
        let owner = state.world.players()[0].owner();
        let mut processor = StatementProcessor {
            seen: vec![],
            owner,
        };
        let mut budget = 0;
        state
            .process_pending_diplomacy(&mut processor, &mut budget)
            .unwrap();
        assert!(processor.seen.is_empty());
        assert_eq!(state.revision(), 0); // Empty processing is a real read-only path.
        state
            .diplomacy_manager_mut()
            .unwrap()
            .open_sessions_mut()
            .push(Session {
                from: owner,
                to: 2,
                pending: vec![
                    Pending {
                        from: owner,
                        to: 2,
                        statement: StatementHandle(10),
                    },
                    Pending {
                        from: owner,
                        to: 2,
                        statement: StatementHandle(12),
                    },
                ],
            });
        budget = 8;
        assert_eq!(
            state.process_pending_diplomacy(&mut processor, &mut budget),
            Err(AftermathError::ExternalDecisionRequired(
                "actual Statement subtype decision"
            ))
        );
        assert_eq!(processor.seen, [10, 11]);
        assert_eq!(budget, 6);
        assert_eq!(
            state.diplomacy_manager().unwrap().open_sessions()[1].pending[0].statement,
            StatementHandle(12)
        );
    }
    #[test]
    fn diplomacy_revision_exhaustion_never_reopens_pristine_or_removes_a_statement() {
        use crate::diplomacy_manager::StatementHandle;
        use crate::unit_operation_aftermath::{Error as AftermathError, Pending, Session};
        let mut state = diplomacy_state();
        let owner = state.world.players()[0].owner();
        let mut processor = StatementProcessor {
            seen: vec![],
            owner,
        };
        state.revision = u64::MAX;
        assert!(matches!(
            state.diplomacy_manager_mut(),
            Err(Error::ResourceLimit)
        ));
        state
            .process_pending_diplomacy(&mut processor, &mut 0)
            .unwrap();
        assert_eq!(state.revision(), u64::MAX);
        assert!(!state.is_pristine());
        // Private source-only injection, not an available production mutation.
        state
            .diplomacy_manager
            .as_mut()
            .unwrap()
            .open_sessions_mut()
            .push(Session {
                from: owner,
                to: 2,
                pending: vec![Pending {
                    from: owner,
                    to: 2,
                    statement: StatementHandle(10),
                }],
            });
        assert_eq!(
            state.process_pending_diplomacy(&mut processor, &mut 8),
            Err(AftermathError::Unsupported(
                "loaded state revision exhausted"
            ))
        );
        assert_eq!(state.revision(), u64::MAX);
        assert!(!state.is_pristine());
        assert!(processor.seen.is_empty());
        assert!(state.diplomacy_manager().unwrap().has_outgoing(owner));
    }
    #[test]
    fn diplomacy_inventory_uses_allocation_slots_not_owner_id_range() {
        use crate::diplomacy_manager::{EmptyOpenSessions, PendingStateSnapshot, PlayerHandle};
        fn owner64_state() -> LoadedState {
            let mut state = state();
            state.world = world::World::new(
                state.world.map().clone(),
                vec![world::Player::new(64, None, None).unwrap()],
            )
            .unwrap();
            state.retained.players[0].header.owner = 64;
            state.retained.players[0].header.ordered_flags[1] = true;
            state
                .bind_observed_snapshot_controls(witness(&state), controls(&state))
                .unwrap();
            state
        }
        let mut state = owner64_state();
        let w = state.observed_snapshot().unwrap().clone();
        let mut allocation = vec![None; 65];
        allocation[0] = Some(PlayerHandle { owner: 64 });
        let snapshot = |ever_alive, allocation_slots| PendingStateSnapshot {
            witness: w.clone(),
            ever_alive,
            allocation_slots,
            open_sessions: EmptyOpenSessions::from_has_pending_session_statements(false).unwrap(),
            pending_actions: vec![],
        };
        assert!(matches!(
            state.install_diplomacy_manager(snapshot(vec![], allocation.clone())),
            Err(Error::Binding)
        ));
        assert!(state.diplomacy_manager().is_err());
        state
            .install_diplomacy_manager(snapshot(vec![PlayerHandle { owner: 64 }], allocation))
            .unwrap();
        assert_eq!(
            state.diplomacy_manager().unwrap().ever_alive(),
            [PlayerHandle { owner: 64 }]
        );
        // InitializeLists excludes allocation slot64, irrespective of owner ID.
        let mut last = owner64_state();
        let mut allocation = vec![None; 65];
        allocation[64] = Some(PlayerHandle { owner: 64 });
        last.install_diplomacy_manager(snapshot(vec![], allocation))
            .unwrap();
        assert!(last.diplomacy_manager().unwrap().ever_alive().is_empty());
    }
    fn notification_source() -> (
        LoadedState,
        ComponentSnapshotWitness,
        Vec<crate::notifications::PlayerNotifications>,
    ) {
        let mut state = state();
        let mut collections: Vec<_> = (0..64)
            .map(|owner| crate::notifications::PlayerNotifications {
                owner,
                next_id: 0,
                records: vec![],
            })
            .collect();
        collections[0]
            .records
            .push(crate::notifications::Notification {
                id: crate::notifications::ComponentId {
                    owner: 0,
                    component_type: 38,
                    id: 7,
                },
                ..crate::notifications::Notification::default()
            });
        let start = state.image.as_ref().len();
        let CoreImage::Unique(bytes) = &mut state.image else {
            unreachable!()
        };
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(64u32.to_le_bytes());
        for player in &collections {
            bytes.extend(
                player
                    .encode(crate::notifications::Limits::default())
                    .unwrap(),
            );
        }
        state.binding = crate::core_stream::tests::context_binding(bytes);
        let witness = ComponentSnapshotWitness {
            source_role: core_player::initialization::WireRole::SavedOutputCore,
            snapshot_sha256: state.binding.core_sha256,
            snapshot_bytes: bytes.len(),
            owner_trace_sha256: [5; 32],
            owner_range: [start, bytes.len()],
            decoded_prefix_bytes: bytes.len() - start,
        };
        (state, witness, collections)
    }
    #[test]
    fn notifications_require_pristine_exact_one_time_source_admission() {
        let (mut state, witness, collections) = notification_source();
        let mut bad = witness.clone();
        bad.snapshot_sha256[0] ^= 1;
        assert!(matches!(
            state.install_player_notifications(bad, collections.clone()),
            Err(Error::Binding)
        ));
        let mut bad = collections.clone();
        bad[0].next_id = 99;
        assert!(matches!(
            state.install_player_notifications(witness.clone(), bad),
            Err(Error::Binding)
        ));
        assert!(state.player_notifications().is_err());
        assert_eq!(state.revision(), 0);
        let mut bad = witness.clone();
        bad.owner_range[1] = usize::MAX;
        assert!(matches!(
            state.install_player_notifications(bad, collections.clone()),
            Err(Error::Binding)
        ));
        state
            .install_player_notifications(witness.clone(), collections.clone())
            .unwrap();
        assert_eq!(
            state.player_notifications().unwrap().collections(),
            collections.as_slice()
        );
        assert_eq!(state.player_notifications_source(), Some(&witness));
        assert!(matches!(
            state.install_player_notifications(witness, collections),
            Err(Error::AlreadyBound)
        ));
        let (mut advanced, witness, collections) = notification_source();
        advanced.revision = 1;
        assert!(matches!(
            advanced.install_player_notifications(witness, collections),
            Err(Error::NotPristine)
        ));
    }
    #[test]
    fn notification_mutation_is_indexed_identity_checked_and_atomic_on_revision_exhaustion() {
        let (mut state, witness, collections) = notification_source();
        let id = collections[0].records[0].id;
        state
            .install_player_notifications(witness.clone(), collections)
            .unwrap();
        let mut wrong = id;
        wrong.id ^= 1;
        assert!(state.notification_record_mut(0, 0, wrong).is_err());
        assert!(state.set_notification_next_id(64, 2).is_err());
        assert_eq!(state.revision(), 0);
        state
            .notification_record_mut(0, 0, id)
            .unwrap()
            .has_been_activated = true;
        state.set_notification_next_id(0, 8).unwrap();
        assert_eq!(state.revision(), 2);
        assert_eq!(state.player_notifications_source(), Some(&witness));
        // Another collection mutation advances the shared revision without
        // invalidating the current record or its original source witness.
        state.set_notification_next_id(63, 1).unwrap();
        assert_eq!(state.revision(), 3);
        assert!(
            state
                .player_notifications()
                .unwrap()
                .record(0, 0)
                .unwrap()
                .has_been_activated
        );
        state.revision = u64::MAX;
        assert!(matches!(
            state.notification_record_mut(0, 0, id),
            Err(Error::ResourceLimit)
        ));
        assert!(matches!(
            state.set_notification_next_id(0, 99),
            Err(Error::ResourceLimit)
        ));
        assert_eq!(state.revision(), u64::MAX);
        assert_eq!(
            state
                .player_notifications()
                .unwrap()
                .collection(0)
                .unwrap()
                .next_id,
            8
        );
        assert!(
            state
                .player_notifications()
                .unwrap()
                .record(0, 0)
                .unwrap()
                .has_been_activated
        );
    }
    fn trade_graph_carrier(frame: &[u8]) -> (LoadedState, ComponentSnapshotWitness, [u8; 32]) {
        use sha2::{Digest, Sha256};
        let mut s = state();
        let start = s.source_image().len();
        let mut image = s.source_image().to_vec();
        image.extend_from_slice(frame);
        s.binding.core_sha256 = Sha256::digest(&image).into();
        s.image = CoreImage::Unique(image);
        let witness = ComponentSnapshotWitness {
            source_role: core_player::initialization::WireRole::SavedOutputCore,
            snapshot_sha256: s.binding.core_sha256,
            snapshot_bytes: s.source_image().len(),
            owner_trace_sha256: [5; 32],
            owner_range: [start, start + frame.len()],
            decoded_prefix_bytes: frame.len(),
        };
        (s, witness, Sha256::digest(frame).into())
    }
    #[test]
    fn trade_graph_admission_rejects_mismatched_frames_atomically() {
        let bytes = crate::trade::Graph::new(42, vec![])
            .unwrap()
            .encode(crate::trade::Limits::default())
            .unwrap();
        for case in 0..8 {
            let (mut s, mut witness, mut hash) = trade_graph_carrier(&bytes);
            match case {
                0 => witness.source_role = core_player::initialization::WireRole::ReadInputCore,
                1 => witness.snapshot_sha256[0] ^= 1,
                2 => witness.snapshot_bytes += 1,
                3 => witness.owner_trace_sha256 = [0; 32],
                4 => witness.owner_range[1] = usize::MAX,
                5 => witness.decoded_prefix_bytes -= 1,
                6 => hash[0] ^= 1,
                _ => witness.owner_range.reverse(),
            }
            assert!(s
                .install_trade_graph(witness, hash, crate::trade::Limits::default())
                .is_err());
            assert!(s.trade_graph().is_err());
            assert!(s.trade_graph_source().is_none());
            assert_eq!(s.revision(), 0);
        }
        let mut trailing = bytes;
        trailing.push(0);
        let (mut s, witness, hash) = trade_graph_carrier(&trailing);
        assert!(matches!(
            s.install_trade_graph(witness, hash, crate::trade::Limits::default()),
            Err(Error::TradeGraph(crate::trade_graph_state::Error::Codec(
                crate::trade::Error::TrailingBytes
            )))
        ));
        assert!(s.trade_graph().is_err());
        assert!(s.trade_graph_source().is_none());
    }
    #[test]
    fn trade_graph_owner_survives_unrelated_writes_but_cannot_be_replayed_or_late_installed() {
        let bytes = crate::trade::Graph::new(42, vec![])
            .unwrap()
            .encode(crate::trade::Limits::default())
            .unwrap();
        let (mut s, witness, hash) = trade_graph_carrier(&bytes);
        s.install_trade_graph(witness.clone(), hash, crate::trade::Limits::default())
            .unwrap();
        assert_eq!(s.revision(), 0);
        assert!(matches!(
            s.install_trade_graph(witness.clone(), hash, crate::trade::Limits::default()),
            Err(Error::AlreadyBound)
        ));
        let player = &s.world.players()[0];
        let owner = player.owner();
        let id = player.formation_records().unwrap()[0].value.id;
        s.initialize_formations().unwrap();
        s.formation_with_runtime_mut(owner, id)
            .unwrap()
            .0
            .moves_remaining_raw = 7;
        assert_eq!(s.revision(), 1);
        assert_eq!(s.trade_graph().unwrap().graph().total_edges_created(), 42);
        assert_eq!(s.trade_graph_source(), Some(&witness));
        let (mut late, witness, hash) = trade_graph_carrier(&bytes);
        late.initialize_formations().unwrap();
        late.formation_with_runtime_mut(owner, id)
            .unwrap()
            .0
            .moves_remaining_raw = 7;
        assert!(matches!(
            late.install_trade_graph(witness, hash, crate::trade::Limits::default()),
            Err(Error::NotPristine)
        ));
        assert!(late.trade_graph().is_err());
    }
    struct TechDefinitions;
    impl crate::movement_world::DefinitionLookup for TechDefinitions {
        fn count(&self) -> usize {
            2
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            match key {
                0..=1 => Some(key as usize),
                100..=101 => Some((key - 100) as usize),
                _ => None,
            }
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            (index < 2).then_some(index as u32 + 100)
        }
    }
    fn game_techs_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        for word in [3u32, 1, 3, 100] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.push(1);
        for word in [i32::MIN as u32, 1, 101, 1, 100] {
            bytes.extend(word.to_le_bytes());
        }
        bytes
    }
    fn game_techs_source(bytes: &[u8]) -> (LoadedState, ComponentSnapshotWitness) {
        let mut state = state();
        let CoreImage::Unique(image) = &mut state.image else {
            unreachable!()
        };
        let start = image.len();
        image.extend_from_slice(bytes);
        state.binding = crate::core_stream::tests::context_binding(image);
        let witness = ComponentSnapshotWitness {
            source_role: core_player::initialization::WireRole::SavedOutputCore,
            snapshot_sha256: state.binding.core_sha256,
            snapshot_bytes: image.len(),
            owner_trace_sha256: [7; 32],
            owner_range: [start, image.len()],
            decoded_prefix_bytes: bytes.len(),
        };
        (state, witness)
    }
    #[test]
    fn game_techs_admission_retains_graph_and_scope_without_advancing_revision() {
        let (mut state, witness) = game_techs_source(&game_techs_bytes());
        assert!(matches!(
            state.game_techs(),
            Err(Error::Missing(MissingInput::GameTechs))
        ));
        state
            .install_game_techs(witness.clone(), &TechDefinitions, unit::Limits::default())
            .unwrap();
        let graph = state.game_techs().unwrap();
        assert_eq!(graph.iteration(), [0]);
        assert_eq!(graph.nodes()[0].cost_raw, i32::MIN);
        assert!(graph.has_tech(0));
        assert_eq!(graph.prerequisites(0).unwrap(), [1]);
        assert_eq!(
            graph.prerequisites(-1),
            Err(crate::game_techs::Error::FallbackUnknown)
        );
        assert_eq!(state.game_techs_source(), Some(&witness));
        assert_eq!(state.revision(), 0);
        assert!(matches!(
            state.install_game_techs(witness, &TechDefinitions, unit::Limits::default()),
            Err(Error::AlreadyBound)
        ));
    }
    #[test]
    fn game_techs_admission_rejects_bad_witnesses_without_binding() {
        for kind in 0..8 {
            let (mut state, mut witness) = game_techs_source(&game_techs_bytes());
            match kind {
                0 => witness.source_role = core_player::initialization::WireRole::ReadInputCore,
                1 => witness.snapshot_sha256[0] ^= 1,
                2 => witness.snapshot_bytes += 1,
                3 => witness.owner_trace_sha256 = [0; 32],
                4 => witness.owner_range[1] = usize::MAX,
                5 => witness.decoded_prefix_bytes -= 1,
                6 => witness.owner_range.reverse(),
                _ => witness.owner_range = [0, 0],
            }
            assert!(matches!(
                state.install_game_techs(witness, &TechDefinitions, unit::Limits::default()),
                Err(Error::Binding)
            ));
            assert!(state.game_techs().is_err());
            assert!(state.game_techs_source().is_none());
            assert_eq!(state.revision(), 0);
        }
    }
    #[test]
    fn game_techs_decode_bounds_and_rebuild_gaps_are_not_partial_installs() {
        let bytes = game_techs_bytes();
        for limits in [
            unit::Limits {
                max_bytes: bytes.len() - 1,
                max_items: 100,
            },
            unit::Limits {
                max_bytes: bytes.len(),
                max_items: 2,
            },
        ] {
            let (mut state, witness) = game_techs_source(&bytes);
            assert!(matches!(
                state.install_game_techs(witness, &TechDefinitions, limits),
                Err(Error::GameTechs(crate::game_techs::Error::ResourceLimit))
            ));
            assert!(state.game_techs().is_err());
            assert!(state.game_techs_source().is_none());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        let (mut state, witness) = game_techs_source(&trailing);
        assert!(matches!(
            state.install_game_techs(witness, &TechDefinitions, unit::Limits::default()),
            Err(Error::Binding)
        ));
        assert!(state.game_techs().is_err());
        let mut empty = Vec::new();
        for word in [3u32, 0, 1, 100] {
            empty.extend(word.to_le_bytes());
        }
        let (mut state, witness) = game_techs_source(&empty);
        assert!(matches!(
            state.install_game_techs(witness, &TechDefinitions, unit::Limits::default()),
            Err(Error::GameTechs(
                crate::game_techs::Error::BuildTechsRequired
            ))
        ));
        assert!(state.game_techs().is_err());
        assert_eq!(state.revision(), 0);
    }
    #[test]
    fn game_techs_owner_survives_unrelated_mutation_and_rejects_late_import() {
        let (mut state, witness) = game_techs_source(&game_techs_bytes());
        state
            .install_game_techs(witness.clone(), &TechDefinitions, unit::Limits::default())
            .unwrap();
        let owner = state.world.players()[0].owner();
        let id = state.world.players()[0].formation_records().unwrap()[0]
            .value
            .id;
        state.initialize_formations().unwrap();
        state
            .formation_with_runtime_mut(owner, id)
            .unwrap()
            .0
            .moves_remaining_raw = 7;
        assert_eq!(state.revision(), 1);
        assert_eq!(state.game_techs().unwrap().iteration(), [0]);
        assert_eq!(state.game_techs_source(), Some(&witness));
        let (mut late, witness) = game_techs_source(&game_techs_bytes());
        late.initialize_formations().unwrap();
        late.formation_with_runtime_mut(owner, id)
            .unwrap()
            .0
            .moves_remaining_raw = 7;
        assert!(matches!(
            late.install_game_techs(witness, &TechDefinitions, unit::Limits::default()),
            Err(Error::NotPristine)
        ));
        assert!(late.game_techs().is_err());
    }
    #[test]
    fn component_export_requires_exact_snapshot_and_bounded_owner_witness() {
        let mut s = state();
        let mut w = ComponentSnapshotWitness {
            source_role: core_player::initialization::WireRole::SavedOutputCore,
            snapshot_sha256: s.binding.core_sha256,
            snapshot_bytes: s.source_image().len(),
            owner_trace_sha256: [5; 32],
            owner_range: [0, usize::MAX],
            decoded_prefix_bytes: 0,
        };
        assert!(matches!(
            s.install_game_relations(w.clone(), &[], &[], unit::Limits::default()),
            Err(Error::Binding)
        ));
        w.owner_range = [0, 1];
        w.snapshot_sha256[0] ^= 1;
        assert!(matches!(
            s.install_game_relations(w, &[], &[], unit::Limits::default()),
            Err(Error::Binding)
        ));
        assert!(matches!(
            s.game_relations(),
            Err(Error::Missing(MissingInput::GameRelations))
        ));
        assert!(s.is_pristine());
    }
}
impl AsRef<[u8]> for CoreImage {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Unique(v) => v,
            Self::Shared(v) => v,
        }
    }
}
#[derive(Debug)]
pub enum Error {
    Decode(ThroughRandomError),
    Split(loaded_components::Error),
    World(world::MissingInput),
    IncompletePlayers,
    Binding,
    ResourceLimit,
    Missing(MissingInput),
    UnitInitialization(crate::unit_initialization::Error),
    FormationLookup,
    FormationOwner(crate::formation_initialization::OwnerLookupError),
    RuntimeIdentity,
    RuntimeDependenciesChanged,
    AlreadyInitialized,
    AlreadyBound,
    NotPristine,
    GameRelations(crate::game_diplomatic_relations::Error),
    DiplomacyManager(crate::diplomacy_manager::Error),
    PlayerNotifications(crate::player_notifications_owner::Error),
    GameTechs(crate::game_techs::Error),
    TradeGraph(crate::trade_graph_state::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loaded state: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingInput {
    ManagerSlots,
    PlayerNotifications,
    GameTechs,
    TradeGraph,
    PlayerConfiguration(i32),
    PlayerTurnActive(i32),
    Player(i32),
    Unit { owner: i32, id: u32 },
    Formation { owner: i32, id: u32 },
    UnitRuntime,
    FormationRuntime,
    ProcessingAutomation,
    GamePause,
    GameRelations,
    DiplomacyManager,
}

/// Actual supplied live facts, distinct from saved flags and roster presence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerControls {
    pub owner: i32,
    pub slot_status: i32,
    pub team: i32,
    pub is_alive: bool,
    pub turn_active: bool,
}
/// Bind external facts to this exact admitted stream role. These values must
/// come from an attributed capture or faithful initialization; this API checks
/// consistency, not execution authenticity. Unknown facts are not defaults.
#[derive(Debug)]
pub struct ControlInputs {
    pub binding: WireBinding,
    pub player_mapping: [i32; 65],
    pub player_slots: [bool; 65],
    pub players: Vec<PlayerControls>,
    pub processing_automation: Option<i32>,
    pub pause_storage: Option<u8>,
}
/// Provenance of an observed mutable-state export, independent of the decoder's
/// Read/Write framing role. The resource adapter validates actual preimages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedSnapshotWitness {
    pub source_role: core_player::initialization::WireRole,
    pub snapshot_sha256: [u8; 32],
    pub snapshot_bytes: usize,
    pub capture_runtime_sha256: [u8; 32],
    pub capture_facts_sha256: [u8; 32],
    pub capture_configuration_sha256: [u8; 32],
}
/// A separately attributed component export, not a general Core section index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentSnapshotWitness {
    pub source_role: core_player::initialization::WireRole,
    pub snapshot_sha256: [u8; 32],
    pub snapshot_bytes: usize,
    pub owner_trace_sha256: [u8; 32],
    pub owner_range: [usize; 2],
    pub decoded_prefix_bytes: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct AssemblyLimits {
    pub max_roster_records: usize,
    pub max_runtime_array_items: usize,
}
impl Default for AssemblyLimits {
    fn default() -> Self {
        Self {
            max_roster_records: 4_000_000,
            max_runtime_array_items: 16_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImportStatistics {
    pub players: usize,
    pub units: usize,
    pub formations: usize,
    pub consumed: usize,
    pub items_used: usize,
}
#[derive(Debug)]
pub struct RetainedComponents {
    pub core: OwnedCore,
    pub religion: core_player::Religion,
    pub players: Vec<RetainedPlayer>,
    pub global_ai: crate::game_global_ai::GlobalAi,
    pub barbarian: crate::game_barbarian::Manager,
    pub free_cities: crate::game_free_cities::Manager,
    pub random: crate::game_random_wire::Random,
}
#[derive(Debug)]
struct RuntimePlayer {
    owner: i32,
    unit_index: Vec<Option<usize>>,
    units: Vec<RuntimeUnit>,
}
#[derive(Debug)]
struct RuntimeFormations {
    owner: i32,
    formation_index: Vec<Option<usize>>,
    formations: Vec<FormationRuntime>,
}
#[derive(Debug)]
struct RuntimeUnit {
    value: UnitRuntime,
    dependencies: UnitDependencies,
}
/// Exact saved inputs used by the covered Read application. Captured once;
/// comparisons allocate nothing and do not reset mutable live arrays.
#[derive(Debug)]
struct UnitDependencies {
    type_hash: u32,
    formation_id: u32,
    abilities: Vec<unit::HashValue<i32>>,
    invalid_terrain: Vec<unit::HashValue<bool>>,
    ignore_terrain: Vec<unit::HashValue<bool>>,
    ignore_feature: Vec<unit::HashValue<bool>>,
    bonus: Vec<unit::HashValue<i32>>,
    strength: Vec<unit::HashValue<i32>>,
    disabled: Vec<unit::HashValue<bool>>,
}
impl UnitDependencies {
    fn capture(u: &unit::Unit) -> Result<Self, Error> {
        fn copy<T: Clone>(v: &[T]) -> Result<Vec<T>, Error> {
            let mut out = reserved(v.len())?;
            out.extend_from_slice(v);
            Ok(out)
        }
        Ok(Self {
            type_hash: u.e_type_index,
            formation_id: u.formation_id,
            abilities: copy(&u.ai_abilities_possessed)?,
            invalid_terrain: copy(&u.ab_invalid_terrain_types)?,
            ignore_terrain: copy(&u.ab_ignore_terrain_types)?,
            ignore_feature: copy(&u.ab_ignore_feature_types)?,
            bonus: copy(&u.ai_bonus_points_per_kill)?,
            strength: copy(&u.ai_bonus_points_per_kill_strength)?,
            disabled: copy(&u.k_operations_disabled)?,
        })
    }
    fn matches(&self, u: &unit::Unit) -> bool {
        self.type_hash == u.e_type_index
            && self.formation_id == u.formation_id
            && self.abilities == u.ai_abilities_possessed
            && self.invalid_terrain == u.ab_invalid_terrain_types
            && self.ignore_terrain == u.ab_ignore_terrain_types
            && self.ignore_feature == u.ab_ignore_feature_types
            && self.bonus == u.ai_bonus_points_per_kill
            && self.strength == u.ai_bonus_points_per_kill_strength
            && self.disabled == u.k_operations_disabled
    }
    fn items(u: &unit::Unit) -> Option<usize> {
        [
            u.ai_abilities_possessed.len(),
            u.ab_invalid_terrain_types.len(),
            u.ab_ignore_terrain_types.len(),
            u.ab_ignore_feature_types.len(),
            u.ai_bonus_points_per_kill.len(),
            u.ai_bonus_points_per_kill_strength.len(),
            u.k_operations_disabled.len(),
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)
    }
}
#[derive(Debug)]
pub struct LoadedState {
    image: CoreImage,
    binding: WireBinding,
    world: world::World,
    retained: RetainedComponents,
    statistics: ImportStatistics,
    controls: Option<ControlInputs>,
    runtime: Option<Vec<RuntimePlayer>>,
    formation_runtime: Option<Vec<RuntimeFormations>>,
    limits: AssemblyLimits,
    revision: u64,
    observed_snapshot: Option<ObservedSnapshotWitness>,
    game_relations: Option<crate::game_diplomatic_relations::GameDiplomaticRelations>,
    game_relations_source: Option<ComponentSnapshotWitness>,
    diplomacy_manager: Option<crate::diplomacy_manager::Manager>,
    diplomacy_manager_source: Option<ObservedSnapshotWitness>,
    player_notifications: Option<crate::player_notifications_owner::Owner>,
    player_notifications_source: Option<ComponentSnapshotWitness>,
    game_techs: Option<crate::game_techs::Graph>,
    game_techs_source: Option<ComponentSnapshotWitness>,
    trade_graph: Option<crate::trade_graph_state::GraphState>,
    trade_graph_source: Option<ComponentSnapshotWitness>,
}
fn reserved<T>(n: usize) -> Result<Vec<T>, Error> {
    n.checked_mul(std::mem::size_of::<T>())
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or(Error::ResourceLimit)?;
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
    Ok(v)
}
fn slot_index<T>(capacity: u32, entries: &[map::Slot<T>]) -> Result<Vec<Option<usize>>, Error> {
    if capacity > 65_536 {
        return Err(Error::World(world::MissingInput::InvalidSlot));
    }
    let mut index = reserved(capacity as usize)?;
    index.resize(capacity as usize, None);
    for (i, e) in entries.iter().enumerate() {
        let slot =
            usize::try_from(e.index).map_err(|_| Error::World(world::MissingInput::InvalidSlot))?;
        let target = index
            .get_mut(slot)
            .ok_or(Error::World(world::MissingInput::InvalidSlot))?;
        if target.replace(i).is_some() {
            return Err(Error::World(world::MissingInput::DuplicateSlot));
        }
    }
    Ok(index)
}
impl LoadedState {
    /// Parse once, move all decoded records, retain the entire original image.
    /// Shared byte storage is allowed explicitly; parsed records are never shared.
    pub fn decode(
        image: CoreImage,
        shared: map::Limits,
        post_map: unit::Limits,
        binding: WireBinding,
        contexts: &impl PlayerInitializations,
        limits: AssemblyLimits,
    ) -> Result<Self, Error> {
        let parsed =
            ThroughRandom::decode(image.as_ref(), shared, post_map, binding.clone(), contexts)
                .map_err(Error::Decode)?;
        if !parsed
            .global_ai
            .players
            .player_prefix
            .complete_player_manager_wire()
        {
            return Err(Error::IncompletePlayers);
        }
        let consumed = parsed.consumed;
        let items_used = parsed.items_used;
        let prefix = parsed.global_ai.players.player_prefix;
        if prefix.initialization_binding.as_ref() != Some(&binding) {
            return Err(Error::Binding);
        }
        let count = prefix.players().count();
        if count > limits.max_roster_records {
            return Err(Error::ResourceLimit);
        }
        let mut players = reserved(count)?;
        let mut retained_players = reserved(count)?;
        let mut units = 0usize;
        let mut formations = 0usize;
        for p in prefix
            .first_player
            .into_iter()
            .chain(prefix.additional_players)
        {
            units = units
                .checked_add(p.units.records.entries.len())
                .ok_or(Error::ResourceLimit)?;
            formations = formations
                .checked_add(p.formations.entries.len())
                .ok_or(Error::ResourceLimit)?;
            if units
                .checked_add(formations)
                .is_none_or(|n| n > limits.max_roster_records)
            {
                return Err(Error::ResourceLimit);
            }
            let (player, retained) = loaded_components::split_player(p).map_err(Error::Split)?;
            players.push(player);
            retained_players.push(retained);
        }
        let (map, core) = loaded_components::split_core(parsed.global_ai.players.core);
        let world = world::World::new(map, players).map_err(Error::World)?;
        let retained = RetainedComponents {
            core,
            religion: prefix.religion,
            players: retained_players,
            global_ai: parsed.global_ai.global_ai,
            barbarian: parsed.barbarian,
            free_cities: parsed.free_cities,
            random: parsed.random,
        };
        Ok(Self {
            image,
            binding,
            world,
            retained,
            statistics: ImportStatistics {
                players: count,
                units,
                formations,
                consumed,
                items_used,
            },
            controls: None,
            runtime: None,
            formation_runtime: None,
            limits,
            revision: 0,
            observed_snapshot: None,
            game_relations: None,
            game_relations_source: None,
            diplomacy_manager: None,
            diplomacy_manager_source: None,
            player_notifications: None,
            player_notifications_source: None,
            game_techs: None,
            game_techs_source: None,
            trade_graph: None,
            trade_graph_source: None,
        })
    }
    pub fn world(&self) -> &world::World {
        &self.world
    }
    pub fn retained(&self) -> &RetainedComponents {
        &self.retained
    }
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
    pub fn statistics(&self) -> ImportStatistics {
        self.statistics
    }
    pub fn source_image(&self) -> &[u8] {
        self.image.as_ref()
    }
    pub fn unparsed_tail(&self) -> &[u8] {
        &self.image.as_ref()[self.statistics.consumed..]
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn is_pristine(&self) -> bool {
        self.revision == 0
    }
    pub fn observed_snapshot(&self) -> Option<&ObservedSnapshotWitness> {
        self.observed_snapshot.as_ref()
    }
    pub fn game_relations(
        &self,
    ) -> Result<&crate::game_diplomatic_relations::GameDiplomaticRelations, Error> {
        self.game_relations
            .as_ref()
            .ok_or(Error::Missing(MissingInput::GameRelations))
    }
    pub fn game_relations_source(&self) -> Option<&ComponentSnapshotWitness> {
        self.game_relations_source.as_ref()
    }
    /// Decode only an explicitly attributed source-owner export; the general
    /// byte-zero reader's consumed boundary is unchanged. Definition inputs must
    /// be admitted by the resource adapter, not guessed from saved words.
    pub fn install_game_relations(
        &mut self,
        witness: ComponentSnapshotWitness,
        war_hashes: &[u32],
        alliances: &[(u32, i32)],
        limits: unit::Limits,
    ) -> Result<(), Error> {
        if self.game_relations.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if witness.source_role != core_player::initialization::WireRole::SavedOutputCore
            || witness.snapshot_sha256 != self.binding.core_sha256
            || witness.snapshot_bytes != self.image.as_ref().len()
            || witness.owner_range[0] >= witness.owner_range[1]
        {
            return Err(Error::Binding);
        }
        let bytes = self
            .image
            .as_ref()
            .get(witness.owner_range[0]..witness.owner_range[1])
            .ok_or(Error::Binding)?;
        let decoded =
            crate::game_diplomatic_relations::GameDiplomaticRelations::decode_prefix(bytes, limits)
                .map_err(Error::GameRelations)?;
        if decoded.consumed != witness.decoded_prefix_bytes {
            return Err(Error::Binding);
        }
        let mut owner = decoded.state;
        owner
            .initialize_definitions(war_hashes, alliances)
            .map_err(Error::GameRelations)?;
        self.game_relations = Some(owner);
        self.game_relations_source = Some(witness);
        Ok(())
    }
    /// Admit one complete, explicitly attributed Graph frame. The producer
    /// verifies actual owner-trace attribution and resource/module identities;
    /// this layer verifies the exact source-image/range/hash once, then owns
    /// immutable source-rebuilt logical keys and paths. No broad prefix parse.
    pub fn install_trade_graph(
        &mut self,
        witness: ComponentSnapshotWitness,
        frame_sha256: [u8; 32],
        limits: crate::trade::Limits,
    ) -> Result<(), Error> {
        use sha2::{Digest, Sha256};
        if self.trade_graph.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if witness.source_role != core_player::initialization::WireRole::SavedOutputCore
            || witness.snapshot_sha256 != self.binding.core_sha256
            || witness.snapshot_bytes != self.image.as_ref().len()
            || witness.owner_trace_sha256 == [0; 32]
            || witness.owner_range[0] >= witness.owner_range[1]
            || witness.decoded_prefix_bytes != witness.owner_range[1] - witness.owner_range[0]
        {
            return Err(Error::Binding);
        }
        let frame = self
            .image
            .as_ref()
            .get(witness.owner_range[0]..witness.owner_range[1])
            .ok_or(Error::Binding)?;
        if <[u8; 32]>::from(Sha256::digest(frame)) != frame_sha256 {
            return Err(Error::Binding);
        }
        let graph = crate::trade_graph_state::GraphState::decode(frame, limits)
            .map_err(Error::TradeGraph)?;
        self.trade_graph = Some(graph);
        self.trade_graph_source = Some(witness);
        Ok(())
    }
    /// Point queries do not scan source hashes or consult a global revision.
    /// Graph remains owned across unrelated Unit writes. Any future operation
    /// affecting this graph must apply its exact edits or reject before success.
    pub fn trade_graph(&self) -> Result<&crate::trade_graph_state::GraphState, Error> {
        self.trade_graph
            .as_ref()
            .ok_or(Error::Missing(MissingInput::TradeGraph))
    }
    pub fn trade_graph_source(&self) -> Option<&ComponentSnapshotWitness> {
        self.trade_graph_source.as_ref()
    }
    /// Admit an attributed owner-local Game.Techs Read/Validate and nonempty
    /// InitializePostLoad projection once. This is a source-rebuilt graph, not
    /// evidence of final native runtime state across other load callbacks. The
    /// resource adapter authenticates the trace and actual registry; this layer
    /// checks complete source/binding consistency without general tail offsets.
    pub fn install_game_techs(
        &mut self,
        witness: ComponentSnapshotWitness,
        definitions: &dyn crate::movement_world::DefinitionLookup,
        limits: unit::Limits,
    ) -> Result<(), Error> {
        if self.game_techs.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if witness.source_role != core_player::initialization::WireRole::SavedOutputCore
            || witness.snapshot_sha256 != self.binding.core_sha256
            || witness.snapshot_bytes != self.image.as_ref().len()
            || witness.owner_trace_sha256 == [0; 32]
            || witness.owner_range[0] >= witness.owner_range[1]
            || witness.owner_range[1] - witness.owner_range[0] != witness.decoded_prefix_bytes
        {
            return Err(Error::Binding);
        }
        let bytes = self
            .image
            .as_ref()
            .get(witness.owner_range[0]..witness.owner_range[1])
            .ok_or(Error::Binding)?;
        let limits = unit::Limits {
            max_bytes: limits.max_bytes.min(bytes.len()),
            max_items: limits.max_items.min(self.limits.max_runtime_array_items),
        };
        let (wire, consumed, _) = crate::game_techs::Techs::decode_prefix_with_items(bytes, limits)
            .map_err(Error::GameTechs)?;
        if consumed != bytes.len() {
            return Err(Error::Binding);
        }
        let owner = wire
            .project_read(definitions)
            .map_err(Error::GameTechs)?
            .post_load_without_rebuild()
            .map_err(Error::GameTechs)?;
        self.game_techs = Some(owner);
        self.game_techs_source = Some(witness);
        Ok(())
    }
    /// Immutable typed source projection. Missing owner input is never an empty
    /// graph. Unrelated Unit changes do not reconstruct or replace this owner.
    pub fn game_techs(&self) -> Result<&crate::game_techs::Graph, Error> {
        self.game_techs
            .as_ref()
            .ok_or(Error::Missing(MissingInput::GameTechs))
    }
    pub fn game_techs_source(&self) -> Option<&ComponentSnapshotWitness> {
        self.game_techs_source.as_ref()
    }
    /// Admit the complete saved notification owner once, without inferring live
    /// lock/default-blocking/pool/subtype state. The producer authenticates the
    /// owner trace; this layer checks exact source bytes and binding consistency.
    pub fn install_player_notifications(
        &mut self,
        witness: ComponentSnapshotWitness,
        collections: Vec<crate::notifications::PlayerNotifications>,
    ) -> Result<(), Error> {
        if self.player_notifications.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if witness.source_role != core_player::initialization::WireRole::SavedOutputCore
            || witness.snapshot_sha256 != self.binding.core_sha256
            || witness.snapshot_bytes != self.image.as_ref().len()
            || witness.owner_trace_sha256 == [0; 32]
            || witness.owner_range[0] >= witness.owner_range[1]
            || witness.owner_range[1] - witness.owner_range[0] != witness.decoded_prefix_bytes
        {
            return Err(Error::Binding);
        }
        let bytes = self
            .image
            .as_ref()
            .get(witness.owner_range[0]..witness.owner_range[1])
            .ok_or(Error::Binding)?;
        let owner = crate::player_notifications_owner::Owner::from_decoded(collections)
            .map_err(Error::PlayerNotifications)?;
        let defaults = crate::notifications::Limits::default();
        let limits = crate::notifications::Limits {
            max_bytes: defaults.max_bytes.min(bytes.len()),
            max_records: defaults.max_records.min(self.limits.max_roster_records),
            max_items: defaults.max_items.min(self.limits.max_runtime_array_items),
            max_depth: defaults.max_depth,
        };
        if !owner
            .matches_source(bytes, limits)
            .map_err(Error::PlayerNotifications)?
        {
            return Err(Error::Binding);
        }
        self.player_notifications = Some(owner);
        self.player_notifications_source = Some(witness);
        Ok(())
    }
    pub fn player_notifications(&self) -> Result<&crate::player_notifications_owner::Owner, Error> {
        self.player_notifications
            .as_ref()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))
    }
    pub fn player_notifications_source(&self) -> Option<&ComponentSnapshotWitness> {
        self.player_notifications_source.as_ref()
    }
    /// Reserve the global revision before exposing mutable saved record fields.
    /// Identity/index failures leave both the revision and records unchanged.
    pub fn notification_record_mut(
        &mut self,
        slot: i32,
        index: usize,
        expected_id: crate::notifications::ComponentId,
    ) -> Result<&mut crate::notifications::Notification, Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let record = self
            .player_notifications
            .as_mut()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))?
            .record_mut(slot, index, expected_id)
            .map_err(Error::PlayerNotifications)?;
        self.revision = next;
        Ok(record)
    }
    pub fn set_notification_next_id(&mut self, slot: i32, value: u32) -> Result<(), Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.player_notifications
            .as_mut()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))?
            .set_next_id(slot, value)
            .map_err(Error::PlayerNotifications)?;
        self.revision = next;
        Ok(())
    }
    /// Install current pending-session state only from the same observed dynamic
    /// context as the admitted controls. Equal save bytes alone are insufficient.
    /// The caller validates capture preimages; this API checks their consistency.
    pub fn install_diplomacy_manager(
        &mut self,
        snapshot: crate::diplomacy_manager::PendingStateSnapshot,
    ) -> Result<(), Error> {
        if self.diplomacy_manager.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if self.observed_snapshot.as_ref() != Some(&snapshot.witness) {
            return Err(Error::Binding);
        }
        let controls = self
            .controls
            .as_ref()
            .ok_or(Error::Missing(MissingInput::ManagerSlots))?;
        // These are two separate native tables. IDs need not follow allocation
        // order, and owner64 may occur within the first64 allocation slots.
        if snapshot.allocation_slots.len() != 65 {
            return Err(Error::Binding);
        }
        let mut present = [false; 65];
        let mut expected = reserved(64)?;
        for (index, handle) in snapshot.allocation_slots.iter().enumerate() {
            let Some(handle) = handle else { continue };
            let owner = usize::try_from(handle.owner)
                .ok()
                .filter(|owner| *owner < 65)
                .ok_or(Error::Binding)?;
            if !controls.player_slots[owner] || std::mem::replace(&mut present[owner], true) {
                return Err(Error::Binding);
            }
            let record = self
                .retained
                .players
                .iter()
                .find(|p| p.header.owner == handle.owner)
                .ok_or(Error::Missing(MissingInput::Player(handle.owner)))?;
            if index < 64 && record.header.saved_was_ever_alive() {
                self.player_controls(handle.owner)?;
                expected.push(*handle);
            }
        }
        if present != controls.player_slots || snapshot.ever_alive != expected {
            return Err(Error::Binding);
        }
        let manager = crate::diplomacy_manager::Manager::from_observed_empty_sessions(
            snapshot.open_sessions,
            snapshot.ever_alive,
            snapshot.pending_actions,
        )
        .map_err(Error::DiplomacyManager)?;
        self.diplomacy_manager = Some(manager);
        self.diplomacy_manager_source = Some(snapshot.witness);
        Ok(())
    }
    pub fn diplomacy_manager(&self) -> Result<&crate::diplomacy_manager::Manager, Error> {
        self.diplomacy_manager
            .as_ref()
            .ok_or(Error::Missing(MissingInput::DiplomacyManager))
    }
    pub fn diplomacy_manager_source(&self) -> Option<&ObservedSnapshotWitness> {
        self.diplomacy_manager_source.as_ref()
    }
    /// Explicit mutation boundary for actual session/action providers. This does
    /// not supply Statement payloads, policy choices or session routing rules.
    pub fn diplomacy_manager_mut(
        &mut self,
    ) -> Result<&mut crate::diplomacy_manager::Manager, Error> {
        if self.diplomacy_manager.is_none() {
            return Err(Error::Missing(MissingInput::DiplomacyManager));
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(self.diplomacy_manager.as_mut().unwrap())
    }
    /// Exact outgoing drain over stable copied handles and current configuration
    /// reads. Removal precedes the required synchronous ProcessStatement call.
    /// Provider errors preserve already-performed effects, as in the kernel.
    pub fn process_pending_diplomacy<P: crate::diplomacy_manager::ProcessStatement<Self>>(
        &mut self,
        processor: &mut P,
        budget: &mut usize,
    ) -> Result<(), crate::unit_operation_aftermath::Error> {
        use crate::unit_operation_aftermath::Error as AftermathError;
        let players = self
            .diplomacy_manager()
            .map_err(|_| AftermathError::Unsupported("Game.Diplomacy.Manager state"))?
            .copy_ever_alive()
            .map_err(|_| AftermathError::Unsupported("ever-alive handle copy allocation"))?;
        for player in players {
            let id = player.owner;
            let controls = self
                .player_controls(id)
                .map_err(|_| AftermathError::Unsupported("current player configuration"))?;
            // IsAI = !IsHuman; actual IsHuman is SlotStatus == SS_TAKEN (3).
            if controls.slot_status == 3 {
                continue;
            }
            loop {
                // Check exhaustion before removal, without turning an empty
                // read into a mutation or reopening pristine admission on wrap.
                if !self.diplomacy_manager.as_ref().unwrap().has_outgoing(id) {
                    break;
                }
                let next_revision =
                    self.revision
                        .checked_add(1)
                        .ok_or(AftermathError::Unsupported(
                            "loaded state revision exhausted",
                        ))?;
                let statement = self
                    .diplomacy_manager
                    .as_mut()
                    .unwrap()
                    .remove_next_outgoing(id)
                    .ok_or(AftermathError::Unsupported(
                        "outgoing session changed without callback",
                    ))?;
                self.revision = next_revision;
                *budget = budget
                    .checked_sub(1)
                    .ok_or(AftermathError::DiagnosticBudgetExceeded)?;
                processor.process_statement(self, statement)?;
            }
        }
        Ok(())
    }
    fn require_initial_controls(&self) -> Result<(), Error> {
        if self.controls.is_some() || self.observed_snapshot.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        Ok(())
    }
    /// Consume one observed SavedOutput snapshot export. Decoder provenance is
    /// retained unchanged; these are not facts from its Read process.
    pub fn bind_observed_snapshot_controls(
        &mut self,
        witness: ObservedSnapshotWitness,
        controls: ControlInputs,
    ) -> Result<(), Error> {
        self.require_initial_controls()?;
        if witness.source_role != core_player::initialization::WireRole::SavedOutputCore
            || witness.snapshot_sha256 != self.binding.core_sha256
            || witness.snapshot_bytes != self.image.as_ref().len()
        {
            return Err(Error::Binding);
        }
        self.install_controls(controls)?;
        self.observed_snapshot = Some(witness);
        Ok(())
    }

    /// Validate every control before changing any World field. Saved membership
    /// does not supply Manager validity or nonnull-slot facts.
    pub fn bind_controls(&mut self, controls: ControlInputs) -> Result<(), Error> {
        self.require_initial_controls()?;
        self.install_controls(controls)
    }
    fn install_controls(&mut self, controls: ControlInputs) -> Result<(), Error> {
        if controls.binding != self.binding {
            return Err(Error::Binding);
        }
        if controls.pause_storage.is_some_and(|p| p > 1) {
            return Err(Error::Binding);
        }
        for (i, p) in controls.players.iter().enumerate() {
            if !(0..65).contains(&p.owner)
                || !controls.player_slots[p.owner as usize]
                || controls.players[..i].iter().any(|q| q.owner == p.owner)
            {
                return Err(Error::Binding);
            }
            self.world.player(p.owner).map_err(Error::World)?;
        }
        let mut owners = reserved(self.world.players().len())?;
        owners.extend(self.world.players().iter().map(|p| p.owner()));
        for owner in owners {
            let player = self.world.player_mut(owner).map_err(Error::World)?;
            player.slot_status = None;
            player.team = None;
            player.is_alive = None;
        }
        for p in &controls.players {
            let player = self.world.player_mut(p.owner).map_err(Error::World)?;
            player.slot_status = Some(p.slot_status);
            player.team = Some(p.team);
            player.is_alive = Some(p.is_alive);
        }
        self.world.player_mapping = Some(controls.player_mapping);
        self.world.player_slot_present = Some(controls.player_slots);
        self.controls = Some(controls);
        Ok(())
    }
    pub fn player_controls(&self, owner: i32) -> Result<&PlayerControls, Error> {
        self.controls
            .as_ref()
            .ok_or(Error::Missing(MissingInput::ManagerSlots))?
            .players
            .iter()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::PlayerConfiguration(owner)))
    }
    pub fn turn_active(&self, owner: i32) -> Result<bool, Error> {
        Ok(self.player_controls(owner)?.turn_active)
    }
    pub fn processing_automation(&self) -> Result<i32, Error> {
        self.controls
            .as_ref()
            .and_then(|c| c.processing_automation)
            .ok_or(Error::Missing(MissingInput::ProcessingAutomation))
    }
    pub fn pause_storage(&self) -> Result<u8, Error> {
        self.controls
            .as_ref()
            .and_then(|c| c.pause_storage)
            .ok_or(Error::Missing(MissingInput::GamePause))
    }

    /// Prepare bounded source-covered Read sidecars without cloning/mutating
    /// imported records. Nested Unit remaps and global load completion remain open.
    pub fn initialize_rosters(&mut self, inputs: &InitializationInputs<'_>) -> Result<(), Error> {
        if self.runtime.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if inputs.identities != &self.binding.inputs {
            return Err(Error::Binding);
        }
        let array_count = inputs
            .abilities
            .count()
            .checked_add(
                inputs
                    .terrains
                    .count()
                    .checked_mul(2)
                    .ok_or(Error::ResourceLimit)?,
            )
            .and_then(|n| n.checked_add(inputs.features.count()))
            .and_then(|n| n.checked_add(inputs.great_person_classes.count().checked_mul(2)?))
            .and_then(|n| n.checked_add(inputs.operations.count()))
            .ok_or(Error::ResourceLimit)?;
        let wire_items = self
            .world
            .players()
            .iter()
            .try_fold(0usize, |n, p| {
                p.unit_records()
                    .ok()?
                    .iter()
                    .try_fold(n, |n, u| n.checked_add(UnitDependencies::items(&u.value)?))
            })
            .ok_or(Error::ResourceLimit)?;
        if array_count
            .checked_mul(self.statistics.units)
            .and_then(|n| n.checked_add(wire_items))
            .is_none_or(|n| n > self.limits.max_runtime_array_items)
        {
            return Err(Error::ResourceLimit);
        }
        let mut runtime = reserved(self.world.players().len())?;
        for player in self.world.players() {
            let records = player.units().map_err(Error::World)?;
            let mut units = reserved(records.entries.len())?;
            for u in &records.entries {
                units.push(RuntimeUnit {
                    value: UnitRuntime::from_read(&u.value, &self.binding.inputs, inputs, player)
                        .map_err(Error::UnitInitialization)?,
                    dependencies: UnitDependencies::capture(&u.value)?,
                });
            }
            runtime.push(RuntimePlayer {
                owner: player.owner(),
                unit_index: slot_index(records.capacity, &records.entries)?,
                units,
            });
        }
        self.runtime = Some(runtime);
        Ok(())
    }
    /// Owner lookup is independent of Unit definition/array preparation.
    pub fn initialize_formations(&mut self) -> Result<(), Error> {
        if self.formation_runtime.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        let mut runtime = reserved(self.world.players().len())?;
        for player in self.world.players() {
            let records = player.formations().map_err(Error::World)?;
            let mut formations = reserved(records.entries.len())?;
            for f in &records.entries {
                let retained = self
                    .retained
                    .players
                    .iter()
                    .find(|p| p.header.owner == player.owner())
                    .ok_or(Error::FormationLookup)?;
                formations.push(
                    FormationRuntime::from_owning_player_read(
                        &f.value,
                        retained.requested_owner,
                        retained.header.owner,
                    )
                    .map_err(Error::FormationOwner)?,
                );
            }
            runtime.push(RuntimeFormations {
                owner: player.owner(),
                formation_index: slot_index(records.capacity, &records.entries)?,
                formations,
            });
        }
        self.formation_runtime = Some(runtime);
        Ok(())
    }
    /// Existing pure legality queries can use the actual imported Unit without
    /// requiring unrelated UI or native dirty-tree machinery.
    pub fn unit_mut(&mut self, owner: i32, id: u32) -> Result<&mut unit::Unit, Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let value = self
            .world
            .player_mut(owner)
            .map_err(Error::World)?
            .unit_mut(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        self.revision = next;
        Ok(value)
    }
    pub fn unit_runtime(&self, owner: i32, id: u32) -> Result<&UnitRuntime, Error> {
        let row = self
            .runtime
            .as_ref()
            .ok_or(Error::Missing(MissingInput::UnitRuntime))?
            .iter()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let index = row
            .unit_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .filter(|_| id != u32::MAX)
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let found = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .unit(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let identity = row.units[index].value.archive.identity();
        if identity.owner as i32 != found.record.e_owner || identity.id != found.record.id {
            return Err(Error::RuntimeIdentity);
        }
        if !row.units[index].dependencies.matches(found.record) {
            return Err(Error::RuntimeDependenciesChanged);
        }
        let current = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .formation(found.record.formation_id)
            .map_err(Error::World)?;
        match (row.units[index].value.formation, current) {
            (None, None) => {}
            (Some(saved), Some(actual))
                if saved.owner == actual.record.owner
                    && saved.requested_id == actual.requested_id
                    && saved.resolved_id == actual.resolved_id => {}
            _ => return Err(Error::RuntimeDependenciesChanged),
        }
        Ok(&row.units[index].value)
    }
    pub fn formation_runtime(&self, owner: i32, id: u32) -> Result<&FormationRuntime, Error> {
        let row = self
            .formation_runtime
            .as_ref()
            .ok_or(Error::Missing(MissingInput::FormationRuntime))?
            .iter()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let index = row
            .formation_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .filter(|_| id != u32::MAX)
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?;
        let found = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .formation(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?;
        let identity = row.formations[index].archive.identity();
        if identity.owner as i32 != found.record.owner || identity.id != found.record.id {
            return Err(Error::RuntimeIdentity);
        }
        Ok(&row.formations[index])
    }
    pub fn unit_with_runtime_mut(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(&mut unit::Unit, &mut UnitRuntime), Error> {
        self.unit_runtime(owner, id)?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let row = self
            .runtime
            .as_mut()
            .ok_or(Error::Missing(MissingInput::UnitRuntime))?
            .iter_mut()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let index = row
            .unit_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .filter(|_| id != u32::MAX)
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let unit = self
            .world
            .player_mut(owner)
            .map_err(Error::World)?
            .unit_mut(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let identity = row.units[index].value.archive.identity();
        if identity.owner as i32 != unit.e_owner || identity.id != unit.id {
            return Err(Error::RuntimeIdentity);
        }
        self.revision = next;
        Ok((unit, &mut row.units[index].value))
    }
    pub fn formation_with_runtime_mut(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(&mut unit::Formation, &mut FormationRuntime), Error> {
        self.formation_runtime(owner, id)?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let row = self
            .formation_runtime
            .as_mut()
            .ok_or(Error::Missing(MissingInput::FormationRuntime))?
            .iter_mut()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let index = row
            .formation_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .filter(|_| id != u32::MAX)
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?;
        let formation = self
            .world
            .player_mut(owner)
            .map_err(Error::World)?
            .formation_mut(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?;
        let identity = row.formations[index].archive.identity();
        if identity.owner as i32 != formation.owner || identity.id != formation.id {
            return Err(Error::RuntimeIdentity);
        }
        self.revision = next;
        Ok((formation, &mut row.formations[index]))
    }
}
