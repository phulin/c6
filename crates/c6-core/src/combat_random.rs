//! Conditional Linux XP2 random damage and the named Game sync stream effect.
//!
//! This operates on already resolved combat parameters and caller-mapped archive
//! ordering keys. It does not execute an attack, reconstruct a native archive
//! tree, identify every gameplay RNG, or reproduce logging/service state.
use crate::combat::{calculate_average_damage, DamageError, DamageParameters};
use crate::rng::{native_serialization::SerializedRandomRecord, RandomState};

/// Complete logical Random fields used by this boundary. The native fixture
/// additionally verifies that the original vptr and seven padding bytes survive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameSyncRandom {
    pub record: SerializedRandomRecord,
    pub lock_count: u32,
}

/// A caller-supplied key whose ordering must match the observed native pointer
/// comparator for this archive. A rank is not a native pointer or an archive ID.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArchiveOrderKey(pub u64);

/// Logical changed-variable membership in the Game sync archive, in comparator
/// order. Allocation, node links/colors and native pointer identities are outside
/// this representation. The fixture validates the pointer-to-key mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameSyncArchiveTracking {
    pub sync_variable_key: ArchiveOrderKey,
    pub changed_keys: Vec<ArchiveOrderKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameSyncDamageState {
    pub random: GameSyncRandom,
    pub archive: GameSyncArchiveTracking,
}

/// Rejection before executing the conditional component, without state changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreflightRejection {
    UnsupportedMathPlatform,
    /// Keys must be unique and strictly increasing under the caller's validated
    /// mapping of the original pointer comparator.
    InvalidArchiveKeyOrder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomDamageOutcome {
    Damage {
        amount: i32,
        draw: u16,
    },
    /// Original execution would trap after the archive edit and RNG draw. The
    /// state retains that prefix; this is not an atomic unsupported operation.
    /// No successful native numeric comparison is claimed for trapping inputs.
    NativeArithmeticFaultAfterDraw {
        draw: u16,
        fault: DamageError,
    },
}

/// Execute the bounded original nonaverage damage prefix and arithmetic.
///
/// Preconditions beyond representational validation: this is the actual named
/// Game sync stream attached to its original Game archive; all existing dirty
/// keys and the sync key have a validated mapping of native pointer ordering;
/// strengths and rule/configuration inputs are already resolved; and the math
/// implementation/FP controls match the accepted Linux XP2 profile.
/// Original allocation and logging callbacks must complete normally, without
/// reentrant gameplay changes. This models state at completed call boundaries,
/// not intermediate observations during the original logging callback.
///
/// The target truncates the global maximum extra damage to u16, marks the wrapper
/// dirty, and draws once even before either zero-strength return. Locks do not
/// suppress that draw. Only seed and call count change in the Random record.
pub fn calculate_random_damage(
    parameters: &DamageParameters,
    global_max_extra_damage: i32,
    state: &mut GameSyncDamageState,
) -> Result<RandomDamageOutcome, PreflightRejection> {
    calculate_random_damage_fields(
        parameters,
        global_max_extra_damage,
        &mut state.random.record,
        state.random.lock_count,
        &mut state.archive,
    )
}

/// Borrow an existing canonical record and its separate live runtime metadata.
/// Same conditional logging/math contract as calculate_random_damage; this
/// does not allocate a second stream or change the record's ownership.
pub fn calculate_random_damage_fields(
    parameters: &DamageParameters,
    global_max_extra_damage: i32,
    record: &mut SerializedRandomRecord,
    lock_count: u32,
    archive: &mut GameSyncArchiveTracking,
) -> Result<RandomDamageOutcome, PreflightRejection> {
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return Err(PreflightRejection::UnsupportedMathPlatform);
    }
    if archive
        .changed_keys
        .windows(2)
        .any(|keys| keys[0] >= keys[1])
    {
        return Err(PreflightRejection::InvalidArchiveKeyOrder);
    }
    if let Err(index) = archive
        .changed_keys
        .binary_search(&archive.sync_variable_key)
    {
        archive
            .changed_keys
            .insert(index, archive.sync_variable_key);
    }
    let mut logical = RandomState {
        seed: record.seed,
        calls: record.calls,
        resets: record.resets,
        lock_count,
        synchronous: record.synchronous_byte != 0,
    };
    let draw = logical.draw(global_max_extra_damage as u16);
    record.seed = logical.seed;
    record.calls = logical.calls;

    if parameters.defender_strength_raw == 0 {
        return Ok(RandomDamageOutcome::Damage {
            amount: parameters.defender_max_hit_points,
            draw,
        });
    }
    if parameters.attacker_strength_raw == 0 {
        return Ok(RandomDamageOutcome::Damage {
            amount: parameters.minimum_damage,
            draw,
        });
    }

    // Target CFG: u16-zero-extended draw is added to saved base damage using
    // one wrapping i32 ADD, then converted once by signed CVTSI2SS. Passing the
    // identical wrapped integer through the unchanged average arithmetic with
    // an explicit zero extra preserves that conversion and all later FP/Q8
    // operations. Both early returns above occur after the draw, before this ADD.
    let mut resolved = *parameters;
    resolved.base_damage = parameters.base_damage.wrapping_add(i32::from(draw));
    match calculate_average_damage(&resolved, 0) {
        Ok(amount) => Ok(RandomDamageOutcome::Damage { amount, draw }),
        Err(fault) => Ok(RandomDamageOutcome::NativeArithmeticFaultAfterDraw { draw, fault }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> GameSyncDamageState {
        GameSyncDamageState {
            random: GameSyncRandom {
                record: SerializedRandomRecord {
                    seed: 1,
                    calls: u32::MAX,
                    resets: 7,
                    synchronous_byte: 255,
                },
                lock_count: 3,
            },
            archive: GameSyncArchiveTracking {
                sync_variable_key: ArchiveOrderKey(2),
                changed_keys: vec![ArchiveOrderKey(1), ArchiveOrderKey(4)],
            },
        }
    }
    fn parameters() -> DamageParameters {
        DamageParameters {
            attacker_strength_raw: 5120,
            defender_strength_raw: 5120,
            defender_max_hit_points: 100,
            outer_defense_damage: 0,
            outer_defense_max_hit_points: 100,
            defense_type: u32::MAX,
            power_scaling_raw: 10,
            minimum_damage: 1,
            base_damage: 24,
            power_dampener: 0,
            max_extra_damage: 0,
            bypass_defenses: false,
        }
    }
    #[test]
    fn early_return_still_draws_and_marks_once_with_live_fields_preserved() {
        let mut state = state();
        let mut p = parameters();
        p.defender_strength_raw = 0;
        assert_eq!(
            calculate_random_damage(&p, 12, &mut state),
            Ok(RandomDamageOutcome::Damage {
                amount: 100,
                draw: 3
            })
        );
        assert_eq!(state.random.record.seed, 1_103_527_590);
        assert_eq!(state.random.record.calls, 0);
        assert_eq!(state.random.record.resets, 7);
        assert_eq!(state.random.record.synchronous_byte, 255);
        assert_eq!(state.random.lock_count, 3);
        assert_eq!(
            state.archive.changed_keys,
            vec![ArchiveOrderKey(1), ArchiveOrderKey(2), ArchiveOrderKey(4)]
        );
        let keys = state.archive.changed_keys.clone();
        calculate_random_damage(&p, 12, &mut state).unwrap();
        assert_eq!(state.archive.changed_keys, keys);
    }
    #[test]
    fn malformed_tracking_rejection_is_before_any_mutation() {
        let mut state = state();
        state.archive.changed_keys.reverse();
        let before = state.clone();
        assert_eq!(
            calculate_random_damage(&parameters(), 12, &mut state),
            Err(PreflightRejection::InvalidArchiveKeyOrder)
        );
        assert_eq!(state, before);
    }
    #[test]
    fn native_divide_fault_retains_draw_and_tracking_prefix() {
        let mut state = state();
        let mut p = parameters();
        p.defense_type = 0x5e97d629;
        p.outer_defense_max_hit_points = 16_777_216;
        p.outer_defense_damage = -1;
        assert_eq!(
            calculate_random_damage(&p, 12, &mut state),
            Ok(RandomDamageOutcome::NativeArithmeticFaultAfterDraw {
                draw: 3,
                fault: DamageError::ZeroFixedPointDivisor
            })
        );
        assert_eq!(state.random.record.seed, 1_103_527_590);
        assert_eq!(state.random.record.calls, 0);
        assert!(state.archive.changed_keys.contains(&ArchiveOrderKey(2)));
    }
}
