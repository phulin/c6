//! DiplomacyRandomEffect's once-per-subject value preparation on Linux XP2.
//!
//! The random sample already lives in `first_impressions`; Apply consumes no
//! RNG. The canonical modifier owner must subsequently run SimpleEffect::Apply
//! followed by SimpleEffect::Remove, including when this returns `None` or zero.
use super::Error;
use c6_core::player_diplomacy_blackboard::{Blackboard, LongArray};

/// RandomEffect's virtual GetEffectNameTextKey returns this key directly;
/// its inherited configurable text field is not used for tracker activation.
pub const EFFECT_NAME: &[u8] = b"LOC_TOOLTIP_SAMPLE_DIPLOMACY_RANDOM";

/// Concrete initialized effect fields, not a policy-selected opinion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Parameters {
    /// DiplomacyRandomEffect+78, initialized from RandomRange (default 3).
    pub amount: i32,
    /// DiplomacyRandomEffect+7c, initialized from DifficultyOffset (default 0).
    pub human_adjustment: i32,
    /// Current RELATIONSHIP_SET global. Stock Linux XP2 uses 0xe59a8aec.
    pub relationship_set: u32,
}

/// Set the canonical relationship gate and return the SetModifierValue argument
/// only on its first application. `None` leaves the effect's extra-value fields
/// untouched; it does not suppress the following simple-effect lifecycle.
pub fn prepare(
    board: &mut Blackboard,
    parameters: Parameters,
    instance: u32,
    subject: i32,
    subject_is_human: bool,
) -> Result<Option<i32>, Error> {
    let index = usize::try_from(subject)
        .ok()
        .filter(|index| *index < 64)
        .ok_or(Error::Missing("random diplomacy subject"))?;
    prepare_gate(
        &mut board.bool_arrays,
        board.first_impressions[index],
        parameters,
        instance,
        index,
        subject_is_human,
    )
}

fn prepare_gate(
    arrays: &mut Vec<LongArray<bool>>,
    first_impression: i32,
    parameters: Parameters,
    instance: u32,
    subject: usize,
    subject_is_human: bool,
) -> Result<Option<i32>, Error> {
    if arrays.windows(2).any(|pair| pair[0].key >= pair[1].key) {
        return Err(Error::Missing("random diplomacy bool map order"));
    }
    let key = ((u64::from(instance) << 32) | u64::from(parameters.relationship_set)) as i64;
    let at = arrays.partition_point(|entry| entry.key < key);
    if arrays.get(at).is_none_or(|entry| entry.key != key) {
        arrays.insert(
            at,
            LongArray {
                key,
                values: [false; 64],
            },
        );
    }
    if arrays[at].values[subject] {
        return Ok(None);
    }
    arrays[at].values[subject] = true;
    Ok(Some(value(first_impression, parameters, subject_is_human)))
}

/// Preserve the native float-to-Q8 conversion before integer multiplication.
/// Converting directly to an integer percentile changes values at bin edges.
pub fn value(first_impression: i32, parameters: Parameters, subject_is_human: bool) -> i32 {
    let fraction = first_impression as f32 * f32::from_bits(0x3c23d70a);
    let whole = fraction.floor();
    let raw =
        cvttss2si(whole).wrapping_shl(8) | i32::from(cvttss2si((fraction - whole) * 256.0) as u8);
    let range = parameters
        .amount
        .wrapping_mul(2)
        .wrapping_add(1)
        .wrapping_shl(8);
    let product = ((i64::from(raw) * i64::from(range)) >> 8) as i32;
    let result = product.wrapping_sub(parameters.amount.wrapping_shl(8)) >> 8;
    if subject_is_human {
        result.wrapping_add(parameters.human_adjustment)
    } else {
        result
    }
}

// SSE CVTTSS2SI returns the indefinite word for an unrepresentable float.
fn cvttss2si(value: f32) -> i32 {
    if !(-2147483648.0..2147483648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const STOCK: Parameters = Parameters {
        amount: 3,
        human_adjustment: -1,
        relationship_set: 0xe59a8aec,
    };

    #[test]
    fn retained_samples_and_quantized_edges() {
        assert_eq!(value(65, STOCK, false), 1);
        assert_eq!(value(21, STOCK, true), -3);
        assert_eq!(value(0, STOCK, false), -3);
        assert_eq!(value(100, STOCK, false), 4);
        assert_eq!(value(14, STOCK, false), -3);
        assert_eq!(value(15, STOCK, false), -2);
        assert_eq!(value(-1, STOCK, false), -4);
    }

    #[test]
    fn first_subject_marks_once_and_preserves_other_subjects() {
        let mut arrays = Vec::new();
        assert_eq!(
            prepare_gate(&mut arrays, 65, STOCK, 187, 1, false).unwrap(),
            Some(1)
        );
        assert_eq!(arrays[0].key, 0xbbe59a8aec);
        assert!(arrays[0].values[1]);
        assert!(!arrays[0].values[0]);
        assert_eq!(
            prepare_gate(&mut arrays, 0, STOCK, 187, 1, true).unwrap(),
            None
        );
        assert_eq!(
            prepare_gate(&mut arrays, 21, STOCK, 187, 0, true).unwrap(),
            Some(-3)
        );
        assert_eq!(arrays.len(), 1);
    }
}
