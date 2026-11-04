//! Consuming current-Core ownership for state-backed queries and future actions.
//! This is not complete native Read or full action admission.
use crate::core_player::{self, initialization::WireBinding, PlayerInitializations};
use crate::core_stream::{ThroughRandom, ThroughRandomError};
use crate::formation_initialization::FormationRuntime;
use crate::loaded_components::{self, OwnedCore, RetainedPlayer};
use crate::unit_initialization::{InitializationInputs, UnitRuntime};
use crate::{map, unit, world};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

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
            instance_identity: fresh_state_identity().unwrap(),
            admission_generation: 0,
            observed_snapshot: None,
            game_relations: None,
            game_relations_source: None,
            diplomacy_manager: None,
            diplomacy_manager_source: None,
            player_notifications: None,
            player_notifications_source: None,
            notification_runtime_projection_installed: false,
            notification_pools: Vec::new(),
            notification_pool_projection_installed: false,
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

    // Source-only complete typed actor; no native initialization is inferred.
    fn commit_fixture_unit() -> unit::Unit {
        // Synthetic all-zero fields/empty arrays with current nested wire versions.
        // Generated from the public typed Unit fixture; these are not save bytes.
        let mut bytes = vec![0; 762];
        for (offset, value) in [
            (0, 28),
            (142, 2),
            (159, 1),
            (176, 28),
            (311, 8),
            (341, 1),
            (353, 1),
            (397, 7),
            (436, 1),
            (452, 8),
            (516, 10),
            (590, 2),
            (630, 4),
            (652, 7),
            (730, 1),
            (742, 5),
        ] {
            bytes[offset] = value;
        }
        unit::Unit::decode(&bytes, unit::Limits::default()).unwrap()
    }
    fn actor_commit_state() -> (LoadedState, map::UnitReference) {
        use crate::formation_initialization::{DirtyBaseline, LogicalArchive, UNIT_FIELDS};
        let mut state = state();
        let owner = state.world.players()[0].owner();
        let mut unit = commit_fixture_unit();
        unit.id = 0x10001;
        unit.e_owner = owner;
        unit.formation_id = 0x10001;
        let mut formations = state.world.players()[0].formations().unwrap().clone();
        formations.entries[0].value.unit_ids = vec![unit.id];
        let runtime = UnitRuntime {
            archive: LogicalArchive::new(
                unit::ComponentId {
                    owner: owner as i16,
                    component_type: 1,
                    id: unit.id,
                },
                &UNIT_FIELDS,
                DirtyBaseline::Unavailable,
            ),
            definition_index: 0,
            domain: 0,
            formation: Some(crate::unit_initialization::FormationReference {
                owner,
                requested_id: unit.formation_id,
                resolved_id: unit.formation_id,
            }),
            abilities: vec![],
            invalid_terrain: vec![],
            ignore_terrain: vec![],
            ignore_feature: vec![],
            bonus_points_per_kill: vec![],
            bonus_points_per_kill_strength: vec![],
            operations_disabled: vec![],
        };
        let dependencies = UnitDependencies::capture(&unit).unwrap();
        let units = map::Recycled {
            capacity: 2,
            entries: vec![map::Slot {
                index: 1,
                value: unit,
            }],
            current_id_bits: 0x10000,
            free_head: u32::MAX,
            free_tail: u32::MAX,
            free_links: vec![u32::MAX; 2],
        };
        let player = world::Player::new(owner, Some(units), Some(formations)).unwrap();
        state.world = world::World::new(state.world.map().clone(), vec![player]).unwrap();
        state.bind_controls(controls(&state)).unwrap();
        state.runtime = Some(vec![RuntimePlayer {
            owner,
            unit_index: vec![None, Some(0)],
            units: vec![RuntimeUnit {
                value: runtime,
                dependencies,
            }],
        }]);
        state.initialize_formations().unwrap();
        (
            state,
            map::UnitReference {
                owner: (1 << 16) | owner,
                id: 0x10001,
            },
        )
    }
    #[test]
    fn action_stamps_are_instance_specific_move_stable_and_track_admissions() {
        let a = state();
        let b = state();
        assert_eq!(a.binding(), b.binding());
        assert_eq!(a.revision(), b.revision());
        assert_ne!(a.action_stamp(), b.action_stamp());
        let initial = a.action_stamp();
        let mut moved = *Box::new(a);
        assert_eq!(moved.action_stamp(), initial);
        moved.bind_controls(controls(&moved)).unwrap();
        let controlled = moved.action_stamp();
        assert_ne!(controlled, initial);
        assert_eq!(moved.revision(), 0);
        moved.initialize_formations().unwrap();
        let initialized = moved.action_stamp();
        assert_ne!(initialized, controlled);
        assert_eq!(moved.revision(), 0);
        assert!(moved.initialize_formations().is_err());
        assert_eq!(moved.action_stamp(), initialized);
        let owner = moved.world.players()[0].owner();
        moved.formation_with_runtime_mut(owner, 1).unwrap();
        assert_ne!(moved.action_stamp(), initialized);
    }
    #[test]
    fn failed_admissions_and_generation_exhaustion_do_not_write_state() {
        let mut state = state();
        let initial = state.action_stamp();
        let mut invalid = controls(&state);
        invalid.pause_storage = Some(2);
        assert!(matches!(state.bind_controls(invalid), Err(Error::Binding)));
        assert_eq!(state.action_stamp(), initial);
        state.admission_generation = u64::MAX;
        let exhausted = state.action_stamp();
        assert!(matches!(
            state.bind_controls(controls(&state)),
            Err(Error::ResourceLimit)
        ));
        assert_eq!(state.action_stamp(), exhausted);
        assert!(state.controls.is_none());
        assert!(state.world.player_mapping.is_none());
        assert!(state.world.player_slot_present.is_none());
        assert!(state.world.players()[0].slot_status.is_none());
        assert!(matches!(
            state.initialize_formations(),
            Err(Error::ResourceLimit)
        ));
        assert!(state.formation_runtime.is_none());
        assert_eq!(state.action_stamp(), exhausted);
        assert_eq!(state.revision(), 0);
    }
    #[test]
    fn late_owned_component_admission_stales_without_gameplay_revision_change() {
        let (mut state, witness) = game_techs_source(&game_techs_bytes());
        let before = state.action_stamp();
        state
            .install_game_techs(witness, &TechDefinitions, unit::Limits::default())
            .unwrap();
        assert_eq!(state.revision(), 0);
        assert_ne!(state.action_stamp(), before);
    }
    #[test]
    fn prepared_actor_commit_is_exclusive_infallible_and_drop_is_inert() {
        let (mut state, reference) = actor_commit_state();
        let unit = state.world.unit(reference).unwrap().unwrap().record.clone();
        let formation = state
            .world
            .player(reference.player())
            .unwrap()
            .formation(unit.formation_id)
            .unwrap()
            .unwrap()
            .record
            .clone();
        let mut next_unit = unit.clone();
        next_unit.e_activity = crate::skip_turn::HOLD;
        let mut next_formation = formation.clone();
        next_formation.last_path_cache_location = unit::PlotCoord { x: -9999, y: -9999 };
        drop(
            state
                .prepare_unit_formation_commit(
                    reference,
                    0,
                    &unit,
                    &formation,
                    next_unit.clone(),
                    next_formation.clone(),
                )
                .unwrap(),
        );
        assert_eq!(state.revision(), 0);
        assert_eq!(state.world.unit(reference).unwrap().unwrap().record, &unit);
        state
            .prepare_unit_formation_commit(
                reference,
                0,
                &unit,
                &formation,
                next_unit.clone(),
                next_formation.clone(),
            )
            .unwrap()
            .commit();
        assert_eq!(state.revision(), 1);
        assert_eq!(
            state.world.unit(reference).unwrap().unwrap().record,
            &next_unit
        );
        assert_eq!(
            state
                .world
                .player(reference.player())
                .unwrap()
                .formation(unit.formation_id)
                .unwrap()
                .unwrap()
                .record,
            &next_formation
        );
        state
            .unit_runtime(reference.player(), reference.id)
            .unwrap();
        state
            .formation_runtime(reference.player(), unit.formation_id)
            .unwrap();
    }
    #[test]
    fn actor_commit_preconditions_refuse_without_any_record_or_revision_change() {
        let (mut state, reference) = actor_commit_state();
        let unit = state.world.unit(reference).unwrap().unwrap().record.clone();
        let formation = state
            .world
            .player(reference.player())
            .unwrap()
            .formation(unit.formation_id)
            .unwrap()
            .unwrap()
            .record
            .clone();
        for variant in 0..7 {
            let mut requested = reference;
            let mut before_unit = unit.clone();
            let mut before_formation = formation.clone();
            let mut next_unit = unit.clone();
            let mut next_formation = formation.clone();
            let mut revision = 0;
            match variant {
                0 => requested.owner = 44 << 16 | reference.player(),
                1 => revision = 1,
                2 => before_unit.i_damage = 1,
                3 => before_formation.moves_remaining_raw += 1,
                4 => next_unit.id += 1,
                5 => next_formation.unit_ids.clear(),
                6 => next_unit.e_type_index += 1,
                _ => unreachable!(),
            }
            assert!(state
                .prepare_unit_formation_commit(
                    requested,
                    revision,
                    &before_unit,
                    &before_formation,
                    next_unit,
                    next_formation
                )
                .is_err());
            assert_eq!(state.revision(), 0);
            assert_eq!(state.world.unit(reference).unwrap().unwrap().record, &unit);
            assert_eq!(
                state
                    .world
                    .player(reference.player())
                    .unwrap()
                    .formation(unit.formation_id)
                    .unwrap()
                    .unwrap()
                    .record,
                &formation
            );
        }
        state.revision = u64::MAX;
        assert!(matches!(
            state.prepare_unit_formation_commit(
                reference,
                u64::MAX,
                &unit,
                &formation,
                unit.clone(),
                formation.clone()
            ),
            Err(Error::ResourceLimit)
        ));
        assert_eq!(state.revision(), u64::MAX);
        assert_eq!(state.world.unit(reference).unwrap().unwrap().record, &unit);
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
    #[test]
    fn notification_runtime_projection_requires_complete_unique_pristine_bound_input() {
        use crate::player_notifications_owner::{OccurrenceLimits, Runtime};
        let (mut state, witness, collections) = notification_source();
        let record = collections[0].records[0].clone();
        state
            .install_player_notifications(witness, collections)
            .unwrap();
        let binding = state.binding().clone();
        let first = state
            .player_notifications()
            .unwrap()
            .first(0)
            .unwrap()
            .unwrap();
        // Synthetic second occurrence exercises complete/duplicate coverage.
        let second = state
            .player_notifications
            .as_mut()
            .unwrap()
            .append_initialized(0, record, Runtime::default(), OccurrenceLimits::default())
            .unwrap();
        let runtime = Runtime {
            default_blocking: Some(19),
            ..Runtime::default()
        };
        let mut wrong_binding = binding.clone();
        wrong_binding.core_sha256[0] ^= 1;
        for (binding, values) in [
            (
                &wrong_binding,
                vec![(first, runtime.clone()), (second, runtime.clone())],
            ),
            (&binding, vec![(first, runtime.clone())]),
            (
                &binding,
                vec![(first, runtime.clone()), (first, runtime.clone())],
            ),
        ] {
            assert!(matches!(
                state.initialize_notification_runtime(binding, values),
                Err(Error::Binding)
            ));
            assert_eq!(state.revision(), 0);
            assert_eq!(
                state.player_notifications().unwrap().runtime(first),
                Ok(&Runtime::default())
            );
            assert_eq!(
                state.player_notifications().unwrap().runtime(second),
                Ok(&Runtime::default())
            );
        }
        // A prior supplied field blocks the whole installation before writes.
        state
            .player_notifications
            .as_mut()
            .unwrap()
            .runtime_mut(second)
            .unwrap()
            .group_hash = Some(5);
        assert!(matches!(
            state.initialize_notification_runtime(
                &binding,
                vec![(first, runtime.clone()), (second, Runtime::default())]
            ),
            Err(Error::AlreadyBound)
        ));
        assert_eq!(
            state.player_notifications().unwrap().runtime(first),
            Ok(&Runtime::default())
        );
        state
            .player_notifications
            .as_mut()
            .unwrap()
            .runtime_mut(second)
            .unwrap()
            .group_hash = None;
        // Unordered complete input succeeds; explicit unknowns remain unknown.
        state
            .initialize_notification_runtime(
                &binding,
                vec![(second, Runtime::default()), (first, runtime.clone())],
            )
            .unwrap();
        assert_eq!(state.revision(), 0);
        assert_eq!(
            state.player_notifications().unwrap().runtime(first),
            Ok(&runtime)
        );
        assert_eq!(
            state.player_notifications().unwrap().runtime(second),
            Ok(&Runtime::default())
        );
        assert!(matches!(
            state.initialize_notification_runtime(
                &binding,
                vec![(first, Runtime::default()), (second, Runtime::default())]
            ),
            Err(Error::AlreadyBound)
        ));
        assert_eq!(
            state.player_notifications().unwrap().runtime(first),
            Ok(&runtime)
        );
        let (mut advanced, witness, collections) = notification_source();
        advanced
            .install_player_notifications(witness, collections)
            .unwrap();
        let binding = advanced.binding().clone();
        advanced
            .notification_runtime_mut(first)
            .unwrap()
            .default_blocking = Some(5);
        assert!(matches!(
            advanced.initialize_notification_runtime(&binding, vec![(first, Runtime::default())]),
            Err(Error::NotPristine)
        ));
        assert_eq!(advanced.revision(), 1);
        // Empty collections still consume the projection lease exactly once.
        let (mut empty, _, _) = notification_source();
        empty.player_notifications = Some(
            crate::player_notifications_owner::Owner::from_decoded(
                (0..64)
                    .map(|owner| crate::notifications::PlayerNotifications {
                        owner,
                        ..Default::default()
                    })
                    .collect(),
            )
            .unwrap(),
        );
        let binding = empty.binding().clone();
        empty
            .initialize_notification_runtime(&binding, vec![])
            .unwrap();
        assert!(matches!(
            empty.initialize_notification_runtime(&binding, vec![]),
            Err(Error::AlreadyBound)
        ));
    }
    #[test]
    fn notification_occurrence_bridge_retains_handles_and_checks_revision_before_mutation() {
        use crate::player_notifications_owner::{OccurrenceLimits, Runtime};
        let (mut state, witness, collections) = notification_source();
        let original = collections[0].records[0].clone();
        state
            .install_player_notifications(witness.clone(), collections)
            .unwrap();
        let first = state
            .player_notifications()
            .unwrap()
            .first(0)
            .unwrap()
            .unwrap();
        let second = state
            .append_initialized_notification(
                0,
                original.clone(),
                Runtime::default(),
                OccurrenceLimits::default(),
            )
            .unwrap();
        assert_eq!(state.revision(), 1);
        assert_eq!(state.remove_notification(first).unwrap().next, Some(second));
        assert_eq!(state.revision(), 2);
        assert_eq!(
            state.player_notifications().unwrap().first(0),
            Ok(Some(second))
        );
        assert_eq!(
            state.player_notifications().unwrap().ordinal_of(second),
            Ok(0)
        );
        for _ in 0..2 {
            assert!(state.notification_runtime_mut(first).is_err());
            assert!(state.remove_notification(first).is_err());
            assert_eq!(state.revision(), 2);
        }
        state
            .notification_runtime_mut(second)
            .unwrap()
            .default_blocking = Some(11);
        assert_eq!(state.revision(), 3);
        assert_eq!(state.player_notifications_source(), Some(&witness));
        assert_eq!(
            state.player_notifications().unwrap().collections().len(),
            64
        );
        state.revision = u64::MAX;
        assert!(matches!(
            state.notification_runtime_mut(second),
            Err(Error::ResourceLimit)
        ));
        assert!(matches!(
            state.remove_notification(second),
            Err(Error::ResourceLimit)
        ));
        assert!(matches!(
            state.append_initialized_notification(
                0,
                original.clone(),
                Runtime::default(),
                OccurrenceLimits::default()
            ),
            Err(Error::ResourceLimit)
        ));
        assert_eq!(state.revision(), u64::MAX);
        assert_eq!(
            state
                .player_notifications()
                .unwrap()
                .record_by_object(second),
            Ok(&original)
        );
        assert_eq!(
            state
                .player_notifications()
                .unwrap()
                .runtime(second)
                .unwrap()
                .default_blocking,
            Some(11)
        );
    }
    #[test]
    fn notification_occurrence_bridge_refusals_preserve_state_and_callback_prefix_commits() {
        use crate::player_notifications_owner::{OccurrenceLimits, Runtime};
        let (mut state, witness, collections) = notification_source();
        let original = collections[0].records[0].clone();
        state
            .install_player_notifications(witness, collections)
            .unwrap();
        let first = state
            .player_notifications()
            .unwrap()
            .first(0)
            .unwrap()
            .unwrap();
        for limits in [
            OccurrenceLimits {
                max_records: 1,
                max_occurrences: 9,
            },
            OccurrenceLimits {
                max_records: 9,
                max_occurrences: 1,
            },
        ] {
            assert!(state
                .append_initialized_notification(0, original.clone(), Runtime::default(), limits)
                .is_err());
            assert_eq!(state.revision(), 0);
            assert_eq!(
                state
                    .player_notifications()
                    .unwrap()
                    .record_by_object(first),
                Ok(&original)
            );
        }
        assert!(state
            .append_initialized_notification(
                64,
                original.clone(),
                Runtime::default(),
                OccurrenceLimits::default()
            )
            .is_err());
        assert_eq!(state.revision(), 0);
        // Caller bounds cannot raise the LoadedState resource policy.
        state.limits.max_runtime_array_items = 1;
        assert!(state
            .append_initialized_notification(
                0,
                original.clone(),
                Runtime::default(),
                OccurrenceLimits {
                    max_records: usize::MAX,
                    max_occurrences: usize::MAX
                }
            )
            .is_err());
        assert_eq!(state.revision(), 0);
        state.limits.max_runtime_array_items = 10;
        // The required data-provider callback receives the same state after the
        // append. An error keeps both the append and callback's preceding writes.
        let delivery = |state: &mut LoadedState| -> Result<(), &'static str> {
            let object = state
                .append_initialized_notification(
                    0,
                    original.clone(),
                    Runtime::default(),
                    OccurrenceLimits::default(),
                )
                .unwrap();
            state
                .notification_runtime_mut(object)
                .unwrap()
                .default_blocking = Some(23);
            Err("required delivery failed")
        };
        assert_eq!(delivery(&mut state), Err("required delivery failed"));
        assert_eq!(state.revision(), 2);
        let second = state
            .player_notifications()
            .unwrap()
            .next(first)
            .unwrap()
            .unwrap();
        assert_eq!(
            state
                .player_notifications()
                .unwrap()
                .record_by_object(first),
            Ok(&original)
        );
        assert_eq!(
            state
                .player_notifications()
                .unwrap()
                .runtime(second)
                .unwrap()
                .default_blocking,
            Some(23)
        );
    }
    fn notification_pool_source() -> (
        LoadedState,
        crate::player_notifications_owner::ObjectId,
        crate::notification_lifecycle::PoolId,
    ) {
        use crate::notification_lifecycle as lifecycle;
        use crate::player_notifications_owner::Runtime;
        let (mut state, witness, collections) = notification_source();
        state
            .install_player_notifications(witness, collections)
            .unwrap();
        let object = state
            .player_notifications()
            .unwrap()
            .first(0)
            .unwrap()
            .unwrap();
        let pool = lifecycle::PoolId(99);
        let binding = state.binding().clone();
        state
            .initialize_notification_runtime(
                &binding,
                vec![(
                    object,
                    Runtime {
                        live: Some(lifecycle::LiveNotification {
                            object: lifecycle::ObjectId(42),
                            lock_count: -3,
                            pool_binding: Some(pool),
                            definition_icon: lifecycle::DefinitionIcon::Unobserved,
                            activation: lifecycle::ActivationBinding::Base,
                            release: lifecycle::ReleaseBinding::Base,
                            destruction: lifecycle::DestructionBinding::Unsupported,
                        }),
                        default_blocking: Some(17),
                        group_hash: Some(0x12345678),
                        vtable_addresspoint: Some(0x3aec000),
                    },
                )],
            )
            .unwrap();
        (state, object, pool)
    }
    #[test]
    fn notification_pool_moves_same_payload_and_reappend_gets_fresh_occurrence() {
        use crate::player_notifications_owner::OccurrenceLimits;
        let (mut state, object, pool) = notification_pool_source();
        let id = state
            .player_notifications()
            .unwrap()
            .record_by_object(object)
            .unwrap()
            .id;
        state
            .notification_record_mut(0, 0, id)
            .unwrap()
            .message
            .base = vec![0xa5; 64];
        let allocation = state
            .player_notifications()
            .unwrap()
            .record_by_object(object)
            .unwrap()
            .message
            .base
            .as_ptr();
        assert!(
            matches!(state.last_pooled_notification(pool), Err(Error::Missing(MissingInput::NotificationPool(p))) if p == pool)
        );
        state.pool_notification(object, pool).unwrap();
        let revision = state.revision();
        let view = state.last_pooled_notification(pool).unwrap().unwrap();
        let owner = state.player_notifications().unwrap();
        assert!(std::ptr::eq(
            view.record,
            owner.record_by_object(object).unwrap()
        ));
        assert!(std::ptr::eq(view.runtime, owner.runtime(object).unwrap()));
        assert_eq!(view.runtime.live.as_ref().unwrap().pool_binding, None);
        assert_eq!(view.runtime.live.as_ref().unwrap().object.0, 42);
        assert_eq!(owner.first(0), Ok(Some(object)));
        for error in [
            state.pop_pooled_notification(pool).unwrap_err(),
            state.remove_notification(object).unwrap_err(),
            state.pool_notification(object, pool).unwrap_err(),
        ] {
            assert!(
                matches!(error, Error::NotificationPoolPending { pool: p, object: o } if p == pool && o == object)
            );
        }
        assert_eq!(state.revision(), revision);
        // The borrowed pending view follows the actual live payload, not a copy.
        state.notification_record_mut(0, 0, id).unwrap().severity = -23;
        assert_eq!(
            state
                .last_pooled_notification(pool)
                .unwrap()
                .unwrap()
                .record
                .severity,
            -23
        );
        assert_eq!(state.erase_released_notification(object).unwrap(), None);
        assert!(state
            .player_notifications()
            .unwrap()
            .runtime(object)
            .is_err());
        let retained = state.last_pooled_notification(pool).unwrap().unwrap();
        assert_eq!(retained.record.message.base.as_ptr(), allocation);
        let mut popped = state.pop_pooled_notification(pool).unwrap().unwrap();
        assert_eq!(popped.record.message.base.as_ptr(), allocation);
        assert_eq!(popped.record.severity, -23);
        assert_eq!(popped.runtime.live.as_ref().unwrap().lock_count, -3);
        assert_eq!(popped.runtime.default_blocking, Some(17));
        assert_eq!(popped.runtime.group_hash, Some(0x12345678));
        assert_eq!(popped.runtime.vtable_addresspoint, Some(0x3aec000));
        assert!(matches!(
            state.last_pooled_notification(pool),
            Err(Error::Missing(MissingInput::NotificationPool(_)))
        ));
        popped.runtime.live.as_mut().unwrap().pool_binding = Some(pool);
        let fresh = state
            .append_initialized_notification(
                0,
                popped.record,
                popped.runtime,
                OccurrenceLimits::default(),
            )
            .unwrap();
        assert_ne!(fresh, object);
        assert_eq!(
            state
                .player_notifications()
                .unwrap()
                .runtime(fresh)
                .unwrap()
                .live
                .as_ref()
                .unwrap()
                .object
                .0,
            42
        );
        assert_eq!(
            state
                .player_notifications()
                .unwrap()
                .record_by_object(fresh)
                .unwrap()
                .message
                .base
                .as_ptr(),
            allocation
        );
    }
    #[test]
    fn notification_pool_lifo_retains_unknown_prefix_and_pending_order() {
        use crate::player_notifications_owner::OccurrenceLimits;
        let (mut state, first, pool) = notification_pool_source();
        let record = state
            .player_notifications()
            .unwrap()
            .record_by_object(first)
            .unwrap()
            .clone();
        let mut runtime = state
            .player_notifications()
            .unwrap()
            .runtime(first)
            .unwrap()
            .clone();
        runtime.live.as_mut().unwrap().object.0 = 43;
        let second = state
            .append_initialized_notification(0, record, runtime, OccurrenceLimits::default())
            .unwrap();
        state.pool_notification(first, pool).unwrap();
        state.pool_notification(second, pool).unwrap();
        assert_eq!(
            state.erase_released_notification(first).unwrap(),
            Some(second)
        );
        assert!(
            matches!(state.pop_pooled_notification(pool), Err(Error::NotificationPoolPending { object, .. }) if object == second)
        );
        assert_eq!(state.erase_released_notification(second).unwrap(), None);
        assert_eq!(
            state
                .pop_pooled_notification(pool)
                .unwrap()
                .unwrap()
                .runtime
                .live
                .unwrap()
                .object
                .0,
            43
        );
        assert_eq!(
            state
                .pop_pooled_notification(pool)
                .unwrap()
                .unwrap()
                .runtime
                .live
                .unwrap()
                .object
                .0,
            42
        );
        let revision = state.revision();
        assert!(
            matches!(state.pop_pooled_notification(pool), Err(Error::Missing(MissingInput::NotificationPool(p))) if p == pool)
        );
        assert_eq!(state.revision(), revision);
    }
    #[test]
    fn notification_pool_known_empty_full_and_unlisted_pools_are_distinct() {
        use crate::notification_lifecycle::PoolId;
        use crate::player_notifications_owner::PooledNotification;
        let (mut state, first, pool) = notification_pool_source();
        let owner = state.player_notifications().unwrap();
        let mut runtime = owner.runtime(first).unwrap().clone();
        runtime.live.as_mut().unwrap().object.0 = 43;
        runtime.live.as_mut().unwrap().pool_binding = None;
        let retained = PooledNotification {
            record: owner.record_by_object(first).unwrap().clone(),
            runtime,
        };
        let binding = state.binding().clone();
        state
            .initialize_notification_pools(
                &binding,
                vec![(pool, vec![]), (PoolId(100), vec![retained])],
            )
            .unwrap();
        assert_eq!(state.revision(), 0);
        assert!(state.last_pooled_notification(pool).unwrap().is_none());
        assert!(state.pop_pooled_notification(pool).unwrap().is_none());
        assert_eq!(state.revision(), 0);
        assert!(matches!(
            state.last_pooled_notification(PoolId(101)),
            Err(Error::Missing(MissingInput::NotificationPool(PoolId(101))))
        ));
        assert_eq!(
            state
                .pop_pooled_notification(PoolId(100))
                .unwrap()
                .unwrap()
                .runtime
                .live
                .unwrap()
                .object
                .0,
            43
        );
        assert!(state
            .pop_pooled_notification(PoolId(100))
            .unwrap()
            .is_none());
        assert_eq!(state.revision(), 1);
        assert!(matches!(
            state.initialize_notification_pools(&binding, vec![]),
            Err(Error::AlreadyBound)
        ));
        state.pool_notification(first, pool).unwrap();
        state.erase_released_notification(first).unwrap();
        assert_eq!(
            state
                .pop_pooled_notification(pool)
                .unwrap()
                .unwrap()
                .runtime
                .live
                .unwrap()
                .object
                .0,
            42
        );
        assert!(state.last_pooled_notification(pool).unwrap().is_none());
    }
    #[test]
    fn notification_pool_projection_preflights_binding_limits_and_duplicate_authority() {
        use crate::notification_lifecycle::PoolId;
        use crate::player_notifications_owner::PooledNotification;
        for case in 0..6 {
            let (mut state, first, pool) = notification_pool_source();
            let mut binding = state.binding().clone();
            let mut runtime = state
                .player_notifications()
                .unwrap()
                .runtime(first)
                .unwrap()
                .clone();
            runtime.live.as_mut().unwrap().pool_binding = None;
            runtime.live.as_mut().unwrap().object.0 = if case == 3 { 42 } else { 43 };
            let retained = PooledNotification {
                record: state
                    .player_notifications()
                    .unwrap()
                    .record_by_object(first)
                    .unwrap()
                    .clone(),
                runtime,
            };
            let values = match case {
                1 => vec![(pool, vec![]), (pool, vec![retained])],
                _ => vec![(PoolId(100), vec![retained])],
            };
            match case {
                0 => binding.core_sha256[0] ^= 1,
                2 => state.limits.max_runtime_array_items = 0,
                4 => state.limits.max_roster_records = 1,
                5 => state.set_notification_next_id(0, 9).unwrap(),
                _ => (),
            }
            let revision = state.revision();
            assert!(state
                .initialize_notification_pools(&binding, values)
                .is_err());
            assert!(!state.notification_pool_projection_installed);
            assert!(state.notification_pools.is_empty());
            assert_eq!(state.revision(), revision);
            assert_eq!(
                state
                    .player_notifications()
                    .unwrap()
                    .runtime(first)
                    .unwrap()
                    .live
                    .as_ref()
                    .unwrap()
                    .pool_binding,
                Some(pool)
            );
        }
    }
    #[test]
    fn notification_pool_first_projection_cannot_be_duplicated_by_later_runtime_projection() {
        use crate::player_notifications_owner::PooledNotification;
        let (template, first, pool) = notification_pool_source();
        let mut runtime = template
            .player_notifications()
            .unwrap()
            .runtime(first)
            .unwrap()
            .clone();
        runtime.live.as_mut().unwrap().pool_binding = None;
        let retained = PooledNotification {
            record: template
                .player_notifications()
                .unwrap()
                .record_by_object(first)
                .unwrap()
                .clone(),
            runtime: runtime.clone(),
        };
        let (mut state, witness, collections) = notification_source();
        state
            .install_player_notifications(witness, collections)
            .unwrap();
        let binding = state.binding().clone();
        state
            .initialize_notification_pools(&binding, vec![(pool, vec![retained])])
            .unwrap();
        runtime.live.as_mut().unwrap().pool_binding = Some(pool);
        assert!(matches!(
            state.initialize_notification_runtime(&binding, vec![(first, runtime)]),
            Err(Error::RuntimeIdentity)
        ));
        assert!(!state.notification_runtime_projection_installed);
        assert_eq!(
            state.player_notifications().unwrap().runtime(first),
            Ok(&crate::player_notifications_owner::Runtime::default())
        );
        assert_eq!(
            state
                .last_pooled_notification(pool)
                .unwrap()
                .unwrap()
                .runtime
                .live
                .as_ref()
                .unwrap()
                .object
                .0,
            42
        );
        assert_eq!(state.revision(), 0);
    }
    #[test]
    fn notification_pool_refusals_do_not_clear_parent_or_erase_before_release() {
        use crate::notification_lifecycle::PoolId;
        let (mut state, object, pool) = notification_pool_source();
        assert!(matches!(
            state.erase_released_notification(object),
            Err(Error::RuntimeIdentity)
        ));
        assert!(matches!(
            state.pool_notification(object, PoolId(100)),
            Err(Error::RuntimeIdentity)
        ));
        state.limits.max_runtime_array_items = 0;
        assert!(matches!(
            state.pool_notification(object, pool),
            Err(Error::ResourceLimit)
        ));
        assert!(state.notification_pools.is_empty());
        assert_eq!(state.revision(), 0);
        assert_eq!(
            state
                .player_notifications()
                .unwrap()
                .runtime(object)
                .unwrap()
                .live
                .as_ref()
                .unwrap()
                .pool_binding,
            Some(pool)
        );
        // A successful external no-pool destructor is a caller prerequisite,
        // not a callback invented by this structural erase operation.
        state
            .player_notifications
            .as_mut()
            .unwrap()
            .runtime_mut(object)
            .unwrap()
            .live
            .as_mut()
            .unwrap()
            .pool_binding = None;
        assert_eq!(state.erase_released_notification(object).unwrap(), None);
        let revision = state.revision();
        assert!(matches!(
            state.erase_released_notification(object),
            Err(Error::PlayerNotifications(_))
        ));
        assert_eq!(state.revision(), revision);
        assert!(state.notification_pools.is_empty());
    }
    #[test]
    fn notification_pool_revision_exhaustion_retains_release_prefix_and_payload() {
        for phase in 0..3 {
            let (mut state, object, pool) = notification_pool_source();
            if phase >= 1 {
                state.pool_notification(object, pool).unwrap();
            }
            if phase == 2 {
                state.erase_released_notification(object).unwrap();
            }
            state.revision = u64::MAX;
            assert!(matches!(
                state.pool_notification(object, pool),
                Err(Error::ResourceLimit)
            ));
            assert!(matches!(
                state.erase_released_notification(object),
                Err(Error::ResourceLimit)
            ));
            assert!(matches!(
                state.pop_pooled_notification(pool),
                Err(Error::ResourceLimit)
            ));
            assert_eq!(state.revision(), u64::MAX);
            if phase == 0 {
                assert!(state.notification_pools.is_empty());
                assert_eq!(
                    state
                        .player_notifications()
                        .unwrap()
                        .runtime(object)
                        .unwrap()
                        .live
                        .as_ref()
                        .unwrap()
                        .pool_binding,
                    Some(pool)
                );
            } else {
                let view = state.last_pooled_notification(pool).unwrap().unwrap();
                assert_eq!(view.runtime.live.as_ref().unwrap().pool_binding, None);
                assert_eq!(view.runtime.live.as_ref().unwrap().object.0, 42);
                assert_eq!(
                    state
                        .player_notifications()
                        .unwrap()
                        .ordinal_of(object)
                        .is_ok(),
                    phase == 1
                );
            }
        }
    }
    #[test]
    fn notification_pool_rejects_duplicate_live_or_retained_object_on_append() {
        use crate::player_notifications_owner::OccurrenceLimits;
        let (mut state, object, pool) = notification_pool_source();
        for phase in 0..3 {
            let (record, runtime) = if phase < 2 {
                let owner = state.player_notifications().unwrap();
                (
                    owner.record_by_object(object).unwrap().clone(),
                    owner.runtime(object).unwrap().clone(),
                )
            } else {
                let value = state.last_pooled_notification(pool).unwrap().unwrap();
                (value.record.clone(), value.runtime.clone())
            };
            let revision = state.revision();
            assert!(matches!(
                state.append_initialized_notification(
                    0,
                    record,
                    runtime,
                    OccurrenceLimits::default()
                ),
                Err(Error::RuntimeIdentity)
            ));
            assert_eq!(state.revision(), revision);
            match phase {
                0 => state.pool_notification(object, pool).unwrap(),
                1 => {
                    state.erase_released_notification(object).unwrap();
                }
                _ => (),
            }
        }
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
    fn blocking_store_preserves_raw_words_and_checks_revision_before_write() {
        let mut state = state();
        let owner = state.world.players()[0].owner();
        let before = state.player_end_turn_blocking(owner).unwrap();
        assert!(!state.store_player_end_turn_blocking(owner, before).unwrap());
        assert_eq!(state.revision(), 0);
        assert!(state
            .store_player_end_turn_blocking(owner, i32::MIN)
            .unwrap());
        assert_eq!(state.player_end_turn_blocking(owner).unwrap(), i32::MIN);
        assert_eq!(state.revision(), 1);
        assert!(!state
            .store_player_end_turn_blocking(owner, i32::MIN)
            .unwrap());
        assert_eq!(state.revision(), 1);
        assert!(state.store_player_end_turn_blocking(-1, 7).is_err());
        assert_eq!(state.revision(), 1);
        state.revision = u64::MAX;
        assert!(matches!(
            state.store_player_end_turn_blocking(owner, i32::MAX),
            Err(Error::ResourceLimit)
        ));
        assert_eq!(state.player_end_turn_blocking(owner).unwrap(), i32::MIN);
        assert_eq!(state.revision(), u64::MAX);
        assert!(!state
            .store_player_end_turn_blocking(owner, i32::MIN)
            .unwrap());
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
    ActorCommitConflict,
    ActorCommitIdentity,
    AlreadyInitialized,
    AlreadyBound,
    NotPristine,
    GameRelations(crate::game_diplomatic_relations::Error),
    DiplomacyManager(crate::diplomacy_manager::Error),
    PlayerNotifications(crate::player_notifications_owner::Error),
    NotificationPoolPending {
        pool: crate::notification_lifecycle::PoolId,
        object: crate::player_notifications_owner::ObjectId,
    },
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
    NotificationPool(crate::notification_lifecycle::PoolId),
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
enum NotificationPoolEntry {
    Linked {
        occurrence: crate::player_notifications_owner::ObjectId,
        logical_object: crate::notification_lifecycle::ObjectId,
    },
    Retained(crate::player_notifications_owner::PooledNotification),
}
#[derive(Debug)]
struct RetainedNotificationPool {
    id: crate::notification_lifecycle::PoolId,
    /// False retains an unknown older prefix below these known appended entries.
    prefix_known: bool,
    entries: Vec<NotificationPoolEntry>,
}
/// Process-local capability stamp. It is not gameplay state or a native ID.
/// Independent loads receive different identities; ordinary Rust moves preserve it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionStamp {
    instance: u64,
    revision: u64,
    admission_generation: u64,
}
static NEXT_STATE_IDENTITY: AtomicU64 = AtomicU64::new(1);
fn fresh_state_identity() -> Result<u64, Error> {
    NEXT_STATE_IDENTITY
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| Error::ResourceLimit)
}

#[derive(Debug)]
/// Deliberately not Clone: any future independent copy must allocate a fresh
/// instance identity rather than copy its action capabilities.
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
    instance_identity: u64,
    admission_generation: u64,
    observed_snapshot: Option<ObservedSnapshotWitness>,
    game_relations: Option<crate::game_diplomatic_relations::GameDiplomaticRelations>,
    game_relations_source: Option<ComponentSnapshotWitness>,
    diplomacy_manager: Option<crate::diplomacy_manager::Manager>,
    diplomacy_manager_source: Option<ObservedSnapshotWitness>,
    player_notifications: Option<crate::player_notifications_owner::Owner>,
    player_notifications_source: Option<ComponentSnapshotWitness>,
    // Once-only source projection bookkeeping, not native runtime readiness.
    notification_runtime_projection_installed: bool,
    notification_pools: Vec<RetainedNotificationPool>,
    notification_pool_projection_installed: bool,
    game_techs: Option<crate::game_techs::Graph>,
    game_techs_source: Option<ComponentSnapshotWitness>,
    trade_graph: Option<crate::trade_graph_state::GraphState>,
    trade_graph_source: Option<ComponentSnapshotWitness>,
}
/// Exclusive, validated replacement of one actor and its singleton Formation.
/// Dropping this guard changes nothing. Commit moves only these two records and
/// advances the checked revision once; it never invokes callbacks or clones World.
#[must_use = "a prepared actor commit must be committed or deliberately discarded"]
pub struct PreparedUnitFormationCommit<'a> {
    state: &'a mut LoadedState,
    owner: i32,
    unit_id: u32,
    formation_id: u32,
    next_revision: u64,
    unit: unit::Unit,
    formation: unit::Formation,
}
impl PreparedUnitFormationCommit<'_> {
    pub fn commit(self) {
        // prepare validated both slots, and this guard exclusively owns State.
        *self
            .state
            .world
            .player_mut(self.owner)
            .expect("prepared owner")
            .unit_mut(self.unit_id)
            .expect("prepared Unit collection")
            .expect("prepared Unit slot") = self.unit;
        *self
            .state
            .world
            .player_mut(self.owner)
            .expect("prepared owner")
            .formation_mut(self.formation_id)
            .expect("prepared Formation collection")
            .expect("prepared Formation slot") = self.formation;
        self.state.revision = self.next_revision;
    }
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
            instance_identity: fresh_state_identity()?,
            admission_generation: 0,
            observed_snapshot: None,
            game_relations: None,
            game_relations_source: None,
            diplomacy_manager: None,
            diplomacy_manager_source: None,
            player_notifications: None,
            player_notifications_source: None,
            notification_runtime_projection_installed: false,
            notification_pools: Vec::new(),
            notification_pool_projection_installed: false,
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
    /// Invalidates prepared actions on any gameplay mutation or successful
    /// admission. No native memory identity, RNG, or gameplay value is changed.
    pub fn action_stamp(&self) -> ActionStamp {
        ActionStamp {
            instance: self.instance_identity,
            revision: self.revision,
            admission_generation: self.admission_generation,
        }
    }
    fn advance_admission_generation(&mut self) -> Result<(), Error> {
        self.admission_generation = self
            .admission_generation
            .checked_add(1)
            .ok_or(Error::ResourceLimit)?;
        Ok(())
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
        self.advance_admission_generation()?;
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
        self.advance_admission_generation()?;
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
        self.advance_admission_generation()?;
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
        self.advance_admission_generation()?;
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
    /// Once-only pristine projection of omitted runtime fields. The caller
    /// supplies provenance: this does not authenticate a live capture or make
    /// source-projected values observed native state. Unknowns may remain None.
    /// Complete unique current occurrence coverage is validated before writes.
    pub fn initialize_notification_runtime(
        &mut self,
        binding: &core_player::initialization::WireBinding,
        mut values: Vec<(
            crate::player_notifications_owner::ObjectId,
            crate::player_notifications_owner::Runtime,
        )>,
    ) -> Result<(), Error> {
        if self.notification_runtime_projection_installed {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if binding != &self.binding {
            return Err(Error::Binding);
        }
        if values.len() > self.limits.max_runtime_array_items {
            return Err(Error::ResourceLimit);
        }
        let owner = self.player_notifications()?;
        let count = owner
            .collections()
            .iter()
            .try_fold(0usize, |total, collection| {
                total
                    .checked_add(collection.records.len())
                    .ok_or(Error::ResourceLimit)
            })?;
        if values.len() != count {
            return Err(Error::Binding);
        }
        // Sort the supplied owned vector without allocating an auxiliary set.
        values.sort_unstable_by_key(|(object, _)| *object);
        let mut supplied = values.iter();
        for (slot, collection) in owner.collections().iter().enumerate() {
            for ordinal in 0..collection.records.len() {
                let object = owner
                    .object(slot as i32, ordinal)
                    .map_err(Error::PlayerNotifications)?;
                if supplied.next().map(|(handle, _)| *handle) != Some(object) {
                    return Err(Error::Binding);
                }
                if owner.runtime(object).map_err(Error::PlayerNotifications)?
                    != &crate::player_notifications_owner::Runtime::default()
                {
                    return Err(Error::AlreadyBound);
                }
            }
        }
        // Pools can be source-projected first. A later complete runtime
        // projection must not introduce a second authority for a retained
        // logical object (or duplicate supplied live logical objects).
        let retained_count = self.notification_pools.iter().try_fold(0usize, |n, pool| {
            n.checked_add(pool.entries.len())
                .ok_or(Error::ResourceLimit)
        })?;
        let mut logical = reserved(
            values
                .len()
                .checked_add(retained_count)
                .ok_or(Error::ResourceLimit)?,
        )?;
        for (_, runtime) in &values {
            if let Some(live) = &runtime.live {
                logical.push(live.object);
            }
        }
        for pool in &self.notification_pools {
            for entry in &pool.entries {
                let NotificationPoolEntry::Retained(value) = entry else {
                    return Err(Error::RuntimeIdentity);
                };
                logical.push(
                    value
                        .runtime
                        .live
                        .as_ref()
                        .ok_or(Error::RuntimeIdentity)?
                        .object,
                );
            }
        }
        logical.sort_unstable();
        if logical.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(Error::RuntimeIdentity);
        }
        self.advance_admission_generation()?;
        let owner = self
            .player_notifications
            .as_mut()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))?;
        for (object, runtime) in values {
            *owner
                .runtime_mut(object)
                .map_err(Error::PlayerNotifications)? = runtime;
        }
        self.notification_runtime_projection_installed = true;
        Ok(())
    }
    /// Reserve the shared revision before exposing explicitly supplied runtime
    /// fields. Stale handles reject without changing the revision or storage.
    pub fn notification_runtime_mut(
        &mut self,
        object: crate::player_notifications_owner::ObjectId,
    ) -> Result<&mut crate::player_notifications_owner::Runtime, Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let runtime = self
            .player_notifications
            .as_mut()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))?
            .runtime_mut(object)
            .map_err(Error::PlayerNotifications)?;
        self.revision = next;
        Ok(runtime)
    }
    /// Admit only the explicitly supplied, completely known pools. Unlisted
    /// PoolIds remain unknown. The producer supplies runtime/source authority;
    /// neither saved notification bytes nor a new PoolId prove an empty pool.
    pub fn initialize_notification_pools(
        &mut self,
        binding: &core_player::initialization::WireBinding,
        pools: Vec<(
            crate::notification_lifecycle::PoolId,
            Vec<crate::player_notifications_owner::PooledNotification>,
        )>,
    ) -> Result<(), Error> {
        if self.notification_pool_projection_installed || !self.notification_pools.is_empty() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if binding != &self.binding {
            return Err(Error::Binding);
        }
        let owner = self.player_notifications()?;
        let active = owner
            .collections()
            .iter()
            .try_fold(0usize, |n, collection| {
                n.checked_add(collection.records.len())
                    .ok_or(Error::ResourceLimit)
            })?;
        let retained = pools.iter().try_fold(0usize, |n, (_, records)| {
            n.checked_add(records.len()).ok_or(Error::ResourceLimit)
        })?;
        let total = active.checked_add(retained).ok_or(Error::ResourceLimit)?;
        if pools.len() > self.limits.max_runtime_array_items
            || retained > self.limits.max_runtime_array_items
            || total > self.limits.max_roster_records
        {
            return Err(Error::ResourceLimit);
        }
        let mut ids = reserved(pools.len())?;
        let mut objects = reserved(total)?;
        for (slot, collection) in owner.collections().iter().enumerate() {
            for ordinal in 0..collection.records.len() {
                let occurrence = owner
                    .object(slot as i32, ordinal)
                    .map_err(Error::PlayerNotifications)?;
                if let Some(live) = &owner
                    .runtime(occurrence)
                    .map_err(Error::PlayerNotifications)?
                    .live
                {
                    objects.push(live.object);
                }
            }
        }
        for (id, records) in &pools {
            ids.push(*id);
            for record in records {
                let live = record.runtime.live.as_ref().ok_or(Error::RuntimeIdentity)?;
                if live.pool_binding.is_some() {
                    return Err(Error::RuntimeIdentity);
                }
                objects.push(live.object);
            }
        }
        ids.sort_unstable();
        objects.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1])
            || objects.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(Error::RuntimeIdentity);
        }
        let mut prepared = reserved(pools.len())?;
        for (id, records) in pools {
            let mut entries = reserved(records.len())?;
            entries.extend(records.into_iter().map(NotificationPoolEntry::Retained));
            prepared.push(RetainedNotificationPool {
                id,
                prefix_known: true,
                entries,
            });
        }
        self.advance_admission_generation()?;
        self.notification_pools = prepared;
        self.notification_pool_projection_installed = true;
        Ok(())
    }
    fn notification_pool_link(
        &self,
        object: crate::player_notifications_owner::ObjectId,
    ) -> Option<(usize, usize)> {
        self.notification_pools
            .iter()
            .enumerate()
            .find_map(|(pool, values)| {
                values.entries.iter().position(|entry| matches!(entry,
                NotificationPoolEntry::Linked { occurrence, .. } if *occurrence == object
            )).map(|index| (pool, index))
            })
    }
    fn notification_logical_in_use(
        &self,
        logical: crate::notification_lifecycle::ObjectId,
        except: Option<crate::player_notifications_owner::ObjectId>,
    ) -> Result<bool, Error> {
        let owner = self.player_notifications()?;
        for (slot, collection) in owner.collections().iter().enumerate() {
            for ordinal in 0..collection.records.len() {
                let occurrence = owner
                    .object(slot as i32, ordinal)
                    .map_err(Error::PlayerNotifications)?;
                if Some(occurrence) != except
                    && owner
                        .runtime(occurrence)
                        .map_err(Error::PlayerNotifications)?
                        .live
                        .as_ref()
                        .is_some_and(|live| live.object == logical)
                {
                    return Ok(true);
                }
            }
        }
        for pool in &self.notification_pools {
            for entry in &pool.entries {
                match entry {
                    NotificationPoolEntry::Linked {
                        occurrence,
                        logical_object,
                    } if Some(*occurrence) != except && *logical_object == logical => {
                        return Ok(true);
                    }
                    NotificationPoolEntry::Retained(value)
                        if value
                            .runtime
                            .live
                            .as_ref()
                            .is_some_and(|live| live.object == logical) =>
                    {
                        return Ok(true);
                    }
                    _ => (),
                }
            }
        }
        Ok(false)
    }
    /// Source BaseRelease prefix: append a reference to the SAME logical object
    /// before clearing its live parent/pool association. No payload is cloned.
    /// Until erase_released_notification, the owner node retains that payload.
    pub fn pool_notification(
        &mut self,
        object: crate::player_notifications_owner::ObjectId,
        pool: crate::notification_lifecycle::PoolId,
    ) -> Result<(), Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let owner = self.player_notifications()?;
        let live = owner
            .runtime(object)
            .map_err(Error::PlayerNotifications)?
            .live
            .as_ref()
            .ok_or(Error::RuntimeIdentity)?;
        if let Some((index, _)) = self.notification_pool_link(object) {
            return Err(Error::NotificationPoolPending {
                pool: self.notification_pools[index].id,
                object,
            });
        }
        if live.pool_binding != Some(pool)
            || self.notification_logical_in_use(live.object, Some(object))?
        {
            return Err(Error::RuntimeIdentity);
        }
        let logical_object = live.object;
        let count = self.notification_pools.iter().try_fold(0usize, |n, p| {
            n.checked_add(p.entries.len()).ok_or(Error::ResourceLimit)
        })?;
        if count >= self.limits.max_runtime_array_items {
            return Err(Error::ResourceLimit);
        }
        let index = self
            .notification_pools
            .iter()
            .position(|value| value.id == pool);
        let mut new_entries = Vec::new();
        if let Some(index) = index {
            self.notification_pools[index]
                .entries
                .try_reserve(1)
                .map_err(|_| Error::ResourceLimit)?;
        } else {
            if self.notification_pools.len() >= self.limits.max_runtime_array_items {
                return Err(Error::ResourceLimit);
            }
            self.notification_pools
                .try_reserve(1)
                .map_err(|_| Error::ResourceLimit)?;
            new_entries = reserved(1)?;
        }
        let live = self
            .player_notifications
            .as_mut()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))?
            .runtime_mut(object)
            .map_err(Error::PlayerNotifications)?
            .live
            .as_mut()
            .ok_or(Error::RuntimeIdentity)?;
        let entry = NotificationPoolEntry::Linked {
            occurrence: object,
            logical_object,
        };
        if let Some(index) = index {
            self.notification_pools[index].entries.push(entry);
        } else {
            new_entries.push(entry);
            self.notification_pools.push(RetainedNotificationPool {
                id: pool,
                prefix_known: false,
                entries: new_entries,
            });
        }
        live.pool_binding = None;
        self.revision = next;
        Ok(())
    }
    /// Called only after the provider's required Release/destructor succeeds.
    /// Pool-linked payloads move into their existing entry; no-pool payloads
    /// drop only here. A later refusal retains every earlier Release effect.
    pub fn erase_released_notification(
        &mut self,
        object: crate::player_notifications_owner::ObjectId,
    ) -> Result<Option<crate::player_notifications_owner::ObjectId>, Error> {
        let next_revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let owner = self.player_notifications()?;
        owner
            .ordinal_of(object)
            .map_err(Error::PlayerNotifications)?;
        let live = owner
            .runtime(object)
            .map_err(Error::PlayerNotifications)?
            .live
            .as_ref();
        if live.is_some_and(|value| value.pool_binding.is_some()) {
            return Err(Error::RuntimeIdentity);
        }
        let linked = self.notification_pool_link(object);
        if let Some((pool, index)) = linked {
            let NotificationPoolEntry::Linked { logical_object, .. } =
                &self.notification_pools[pool].entries[index]
            else {
                unreachable!("preflight found a linked pool entry")
            };
            if live.map(|value| value.object) != Some(*logical_object) {
                return Err(Error::RuntimeIdentity);
            }
        }
        let removed = self
            .player_notifications
            .as_mut()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))?
            .remove(object)
            .map_err(Error::PlayerNotifications)?;
        let next = removed.next;
        if let Some((pool, index)) = linked {
            self.notification_pools[pool].entries[index] = NotificationPoolEntry::Retained(
                crate::player_notifications_owner::PooledNotification {
                    record: removed.record,
                    runtime: removed.runtime,
                },
            );
        }
        self.revision = next_revision;
        Ok(next)
    }
    /// Last known appended object, borrowing its authoritative current payload.
    /// Reaching an unknown old prefix returns Missing, never invented emptiness.
    pub fn last_pooled_notification(
        &self,
        pool: crate::notification_lifecycle::PoolId,
    ) -> Result<Option<crate::player_notifications_owner::PooledNotificationRef<'_>>, Error> {
        let values = self
            .notification_pools
            .iter()
            .find(|value| value.id == pool)
            .ok_or(Error::Missing(MissingInput::NotificationPool(pool)))?;
        match values.entries.last() {
            Some(NotificationPoolEntry::Retained(value)) => Ok(Some(
                crate::player_notifications_owner::PooledNotificationRef {
                    record: &value.record,
                    runtime: &value.runtime,
                },
            )),
            Some(NotificationPoolEntry::Linked {
                occurrence,
                logical_object,
            }) => {
                let owner = self.player_notifications()?;
                let runtime = owner
                    .runtime(*occurrence)
                    .map_err(Error::PlayerNotifications)?;
                let live = runtime.live.as_ref().ok_or(Error::RuntimeIdentity)?;
                if live.object != *logical_object || live.pool_binding.is_some() {
                    return Err(Error::RuntimeIdentity);
                }
                Ok(Some(
                    crate::player_notifications_owner::PooledNotificationRef {
                        record: owner
                            .record_by_object(*occurrence)
                            .map_err(Error::PlayerNotifications)?,
                        runtime,
                    },
                ))
            }
            None if values.prefix_known => Ok(None),
            None => Err(Error::Missing(MissingInput::NotificationPool(pool))),
        }
    }
    /// Transfer a fully erased retained payload in native LIFO order. A linked
    /// Release prefix cannot be allocated twice while its owner node survives.
    pub fn pop_pooled_notification(
        &mut self,
        pool: crate::notification_lifecycle::PoolId,
    ) -> Result<Option<crate::player_notifications_owner::PooledNotification>, Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let index = self
            .notification_pools
            .iter()
            .position(|value| value.id == pool)
            .ok_or(Error::Missing(MissingInput::NotificationPool(pool)))?;
        let values = &mut self.notification_pools[index];
        match values.entries.last() {
            Some(NotificationPoolEntry::Linked { occurrence, .. }) => {
                return Err(Error::NotificationPoolPending {
                    pool,
                    object: *occurrence,
                });
            }
            None if values.prefix_known => return Ok(None),
            None => return Err(Error::Missing(MissingInput::NotificationPool(pool))),
            Some(NotificationPoolEntry::Retained(_)) => (),
        }
        let Some(NotificationPoolEntry::Retained(value)) = values.entries.pop() else {
            unreachable!("preflight retained entry cannot change without callbacks")
        };
        self.revision = next;
        Ok(Some(value))
    }
    /// Raw structural append, not native Add. The provider owns exact ID/turn
    /// assignment and all synchronous callbacks. Once committed, callback errors
    /// must retain this mutation and its revision.
    pub fn append_initialized_notification(
        &mut self,
        slot: i32,
        record: crate::notifications::Notification,
        runtime: crate::player_notifications_owner::Runtime,
        limits: crate::player_notifications_owner::OccurrenceLimits,
    ) -> Result<crate::player_notifications_owner::ObjectId, Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        if let Some(live) = &runtime.live {
            if self.notification_logical_in_use(live.object, None)? {
                return Err(Error::RuntimeIdentity);
            }
        }
        let limits = crate::player_notifications_owner::OccurrenceLimits {
            max_records: limits.max_records.min(self.limits.max_roster_records),
            max_occurrences: limits
                .max_occurrences
                .min(self.limits.max_runtime_array_items),
        };
        let object = self
            .player_notifications
            .as_mut()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))?
            .append_initialized(slot, record, runtime, limits)
            .map_err(Error::PlayerNotifications)?;
        self.revision = next;
        Ok(object)
    }
    /// Raw erasure only: the provider must call actual Release first. If that
    /// callback fails, it must keep the node and the preceding callback effects.
    pub fn remove_notification(
        &mut self,
        object: crate::player_notifications_owner::ObjectId,
    ) -> Result<crate::player_notifications_owner::Removed, Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        if let Some((index, _)) = self.notification_pool_link(object) {
            return Err(Error::NotificationPoolPending {
                pool: self.notification_pools[index].id,
                object,
            });
        }
        let removed = self
            .player_notifications
            .as_mut()
            .ok_or(Error::Missing(MissingInput::PlayerNotifications))?
            .remove(object)
            .map_err(Error::PlayerNotifications)?;
        self.revision = next;
        Ok(removed)
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
        self.advance_admission_generation()?;
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
        self.advance_admission_generation()?;
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
    /// Current raw EndTurnBlockingTypes word. Native Read/Write uses plain i32,
    /// with no definition remap; the source setter's field is Player+0x7a8.
    pub fn player_end_turn_blocking(&self, owner: i32) -> Result<i32, Error> {
        self.world.player(owner).map_err(Error::World)?;
        self.retained
            .players
            .iter()
            .find(|player| player.header.owner == owner)
            .map(|player| player.header.ordered_scalars[14])
            .ok_or(Error::Missing(MissingInput::Player(owner)))
    }
    /// Source field mutation only: reserve revision, then store before the
    /// caller's optional blocking-change report. Publishing is not done here.
    /// Returns false for the native same-value early return.
    pub fn store_player_end_turn_blocking(
        &mut self,
        owner: i32,
        value: i32,
    ) -> Result<bool, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|player| player.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        if self.retained.players[index].header.ordered_scalars[14] == value {
            return Ok(false);
        }
        let revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.retained.players[index].header.ordered_scalars[14] = value;
        self.revision = revision;
        Ok(true)
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
        self.advance_admission_generation()?;
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
        self.advance_admission_generation()?;
        self.formation_runtime = Some(runtime);
        Ok(())
    }
    /// Validate a small staged actor replacement before exposing any mutation.
    /// These are consistency checks, not native execution or action admission.
    /// Runtime-derived dependencies and ownership stay fixed; callers must plan
    /// all gameplay effects before preparing the exclusive commit guards.
    pub fn prepare_unit_formation_commit(
        &mut self,
        reference: map::UnitReference,
        expected_revision: u64,
        before_unit: &unit::Unit,
        before_formation: &unit::Formation,
        unit: unit::Unit,
        formation: unit::Formation,
    ) -> Result<PreparedUnitFormationCommit<'_>, Error> {
        use crate::formation_initialization::{FORMATION_COMPONENT_TYPE, UNIT_COMPONENT_TYPE};
        if self.revision != expected_revision {
            return Err(Error::ActorCommitConflict);
        }
        let next_revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        if reference.component_id().component_type != UNIT_COMPONENT_TYPE {
            return Err(Error::ActorCommitIdentity);
        }
        let owner = reference.player();
        let actual = self
            .world
            .unit(reference)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit {
                owner,
                id: reference.id,
            }))?;
        if actual.record != before_unit {
            return Err(Error::ActorCommitConflict);
        }
        let current_formation = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .formation(actual.record.formation_id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Formation {
                owner,
                id: actual.record.formation_id,
            }))?;
        if current_formation.record != before_formation {
            return Err(Error::ActorCommitConflict);
        }
        let runtime = self.unit_runtime(owner, reference.id)?;
        let formation_runtime = self.formation_runtime(owner, before_formation.id)?;
        if runtime.archive.identity().component_type != UNIT_COMPONENT_TYPE
            || formation_runtime.archive.identity().component_type != FORMATION_COMPONENT_TYPE
            || before_unit.e_owner != owner
            || before_formation.owner != owner
            || before_formation.unit_ids != [before_unit.id]
            || unit.id != before_unit.id
            || unit.e_owner != owner
            || unit.formation_id != before_unit.formation_id
            || formation.id != before_formation.id
            || formation.owner != owner
            || formation.unit_ids != before_formation.unit_ids
        {
            return Err(Error::ActorCommitIdentity);
        }
        let row = self
            .runtime
            .as_ref()
            .expect("validated Unit runtime")
            .iter()
            .find(|p| p.owner == owner)
            .expect("validated runtime owner");
        let index =
            row.unit_index[(reference.id & 0xffff) as usize].expect("validated runtime slot");
        if !row.units[index].dependencies.matches(&unit) {
            return Err(Error::RuntimeDependenciesChanged);
        }
        Ok(PreparedUnitFormationCommit {
            state: self,
            owner,
            unit_id: unit.id,
            formation_id: formation.id,
            next_revision,
            unit,
            formation,
        })
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
