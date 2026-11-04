//! Current Game sync metadata beside the ONE retained canonical Random record.
use super::{Error, LoadedState, ObservedSnapshotWitness};
use crate::{
    combat::DamageParameters,
    combat_random::{self, GameSyncArchiveTracking, RandomDamageOutcome},
    rng::native_serialization::SerializedRandomRecord,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoggingScope {
    /// Stock GetLogFile(fcf144ab) finds no channel. No logger methods execute.
    MissingChannel,
}
pub struct Inputs {
    pub record: SerializedRandomRecord,
    pub lock_count: u32,
    pub archive: GameSyncArchiveTracking,
    pub logging: Option<LoggingScope>,
}
#[derive(Debug)]
pub(super) struct Runtime {
    lock_count: u32,
    archive: GameSyncArchiveTracking,
    logging: Option<LoggingScope>,
}
pub struct Snapshot<'a> {
    pub record: &'a SerializedRandomRecord,
    pub lock_count: u32,
    pub archive: &'a GameSyncArchiveTracking,
}
#[derive(Debug)]
pub enum AccessError {
    State(Error),
    Missing,
    AlreadyBound,
    RecordMismatch,
    InvalidArchive,
    UnsupportedLogging,
    Preflight(combat_random::PreflightRejection),
}
impl LoadedState {
    /// Missing live fields are supplied independently; the saved record is
    /// checked, never silently replaced by a second RNG or a new seed.
    pub fn initialize_game_sync(
        &mut self,
        witness: &ObservedSnapshotWitness,
        inputs: Inputs,
    ) -> Result<(), AccessError> {
        if self.game_sync_runtime.is_some() {
            return Err(AccessError::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if self.observed_snapshot.as_ref() != Some(witness) {
            return Err(AccessError::State(Error::Binding));
        }
        if self.retained.random.record != inputs.record {
            return Err(AccessError::RecordMismatch);
        }
        if inputs.archive.changed_keys.windows(2).any(|p| p[0] >= p[1]) {
            return Err(AccessError::InvalidArchive);
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_sync_runtime = Some(Runtime {
            lock_count: inputs.lock_count,
            archive: inputs.archive,
            logging: inputs.logging,
        });
        Ok(())
    }
    pub fn game_sync(&self) -> Result<Snapshot<'_>, AccessError> {
        let runtime = self
            .game_sync_runtime
            .as_ref()
            .ok_or(AccessError::Missing)?;
        Ok(Snapshot {
            record: &self.retained.random.record,
            lock_count: runtime.lock_count,
            archive: &runtime.archive,
        })
    }
    pub fn lock_game_sync(&mut self) -> Result<(), AccessError> {
        if self.game_sync_runtime.is_none() {
            return Err(AccessError::Missing);
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        self.game_sync_runtime.as_mut().expect("checked").lock_count = self
            .game_sync_runtime
            .as_ref()
            .expect("checked")
            .lock_count
            .wrapping_add(1);
        self.revision = next;
        Ok(())
    }
    pub fn unlock_game_sync(&mut self) -> Result<(), AccessError> {
        if self.game_sync_runtime.is_none() {
            return Err(AccessError::Missing);
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        self.game_sync_runtime.as_mut().expect("checked").lock_count = self
            .game_sync_runtime
            .as_ref()
            .expect("checked")
            .lock_count
            .saturating_sub(1);
        self.revision = next;
        Ok(())
    }
    /// Linux Game.GetRandNum: edit the existing sync variable before every
    /// draw, including bound zero. No replacement stream or seed is created.
    pub fn game_sync_rand(&mut self, bound: u16) -> Result<u16, AccessError> {
        let runtime = self
            .game_sync_runtime
            .as_ref()
            .ok_or(AccessError::Missing)?;
        if runtime.logging != Some(LoggingScope::MissingChannel) {
            return Err(AccessError::UnsupportedLogging);
        }
        if runtime
            .archive
            .changed_keys
            .windows(2)
            .any(|p| p[0] >= p[1])
        {
            return Err(AccessError::InvalidArchive);
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        self.revision = next;
        let runtime = self.game_sync_runtime.as_mut().expect("checked");
        if let Err(at) = runtime
            .archive
            .changed_keys
            .binary_search(&runtime.archive.sync_variable_key)
        {
            runtime
                .archive
                .changed_keys
                .insert(at, runtime.archive.sync_variable_key);
        }
        let record = &mut self.retained.random.record;
        let mut logical = crate::rng::RandomState {
            seed: record.seed,
            calls: record.calls,
            resets: record.resets,
            lock_count: runtime.lock_count,
            synchronous: record.synchronous_byte != 0,
        };
        let draw = logical.draw(bound);
        record.seed = logical.seed;
        record.calls = logical.calls;
        Ok(draw)
    }
    /// Existing Linux-validated damage arithmetic/draw implementation, borrowing
    /// the retained record directly. Locks and raw synchronous byte survive.
    pub fn calculate_game_sync_damage(
        &mut self,
        parameters: &DamageParameters,
        extra: i32,
    ) -> Result<RandomDamageOutcome, AccessError> {
        let runtime = self
            .game_sync_runtime
            .as_ref()
            .ok_or(AccessError::Missing)?;
        if runtime.logging != Some(LoggingScope::MissingChannel) {
            return Err(AccessError::UnsupportedLogging);
        }
        if !cfg!(all(
            target_os = "linux",
            target_arch = "x86_64",
            target_env = "gnu"
        )) {
            return Err(AccessError::Preflight(
                combat_random::PreflightRejection::UnsupportedMathPlatform,
            ));
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        self.revision = next;
        let runtime = self.game_sync_runtime.as_mut().expect("checked");
        combat_random::calculate_random_damage_fields(
            parameters,
            extra,
            &mut self.retained.random.record,
            runtime.lock_count,
            &mut runtime.archive,
        )
        .map_err(AccessError::Preflight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{combat_random::ArchiveOrderKey, core_player::initialization::WireRole};
    #[test]
    fn generic_draw_edits_same_archive_and_zero_bound_consumes() {
        let mut state = super::super::tests::state();
        let before = state.retained.random.record;
        state.game_sync_runtime = Some(Runtime {
            lock_count: 17,
            archive: GameSyncArchiveTracking {
                sync_variable_key: ArchiveOrderKey(4),
                changed_keys: vec![ArchiveOrderKey(2), ArchiveOrderKey(9)],
            },
            logging: Some(LoggingScope::MissingChannel),
        });
        let mut expected = crate::rng::RandomState {
            seed: before.seed,
            calls: before.calls,
            resets: before.resets,
            lock_count: 17,
            synchronous: before.synchronous_byte != 0,
        };
        for bound in [0, 1, 7, u16::MAX] {
            assert_eq!(state.game_sync_rand(bound).unwrap(), expected.draw(bound));
        }
        let now = state.game_sync().unwrap();
        assert_eq!(now.record.seed, expected.seed);
        assert_eq!(now.record.calls, before.calls.wrapping_add(4));
        assert_eq!(now.record.resets, before.resets);
        assert_eq!(now.record.synchronous_byte, before.synchronous_byte);
        assert_eq!(now.lock_count, 17);
        assert_eq!(
            now.archive.changed_keys,
            [ArchiveOrderKey(2), ArchiveOrderKey(4), ArchiveOrderKey(9)]
        );
        state.game_sync_runtime.as_mut().unwrap().logging = None;
        let before = state.retained.random.record;
        assert!(matches!(
            state.game_sync_rand(4),
            Err(AccessError::UnsupportedLogging)
        ));
        assert_eq!(state.retained.random.record, before);
    }
    #[test]
    fn real_loaded_owner_mutates_the_retained_record_and_preserves_live_fields() {
        let mut state = super::super::tests::state();
        let witness = ObservedSnapshotWitness {
            source_role: WireRole::SavedOutputCore,
            snapshot_sha256: [1; 32],
            snapshot_bytes: 1,
            capture_runtime_sha256: [2; 32],
            capture_facts_sha256: [3; 32],
            capture_configuration_sha256: [4; 32],
        };
        state.observed_snapshot = Some(witness.clone());
        let before = state.retained.random.record;
        state
            .initialize_game_sync(
                &witness,
                Inputs {
                    record: before,
                    lock_count: 7,
                    archive: GameSyncArchiveTracking {
                        sync_variable_key: ArchiveOrderKey(1),
                        changed_keys: vec![],
                    },
                    logging: Some(LoggingScope::MissingChannel),
                },
            )
            .unwrap();
        let p = DamageParameters {
            attacker_strength_raw: 0,
            defender_strength_raw: 0,
            defender_max_hit_points: 100,
            outer_defense_damage: 0,
            outer_defense_max_hit_points: 0,
            defense_type: u32::MAX,
            power_scaling_raw: 0,
            minimum_damage: 1,
            base_damage: 0,
            power_dampener: 0,
            max_extra_damage: 0,
            bypass_defenses: false,
        };
        assert!(matches!(
            state.calculate_game_sync_damage(&p, 0).unwrap(),
            RandomDamageOutcome::Damage {
                amount: 100,
                draw: 0
            }
        ));
        let current = state.game_sync().unwrap();
        assert_eq!(
            current.record.seed,
            before.seed.wrapping_mul(0x41c64e6d).wrapping_add(0x3039)
        );
        assert_eq!(current.record.calls, before.calls.wrapping_add(1));
        assert_eq!(current.record.resets, before.resets);
        assert_eq!(current.record.synchronous_byte, before.synchronous_byte);
        assert_eq!(current.lock_count, 7);
        assert_eq!(current.archive.changed_keys, [ArchiveOrderKey(1)]);
        assert_eq!(state.retained().random.record, *current.record);
    }
}
