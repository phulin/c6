//! Linux BuildQueue arithmetic shared by canonical turn execution.
//! Values are signed raw Q8. This module does not select production or create
//! completed objects; those operations belong to the canonical state adapter.

/// GetCurrentProductionKind284dff0 for a resolved current order. In particular
/// an empty/project queue returns NO_KIND (-1), while generic CalculateYield
/// uses0. These are distinct arguments even when their values happen to match.
pub fn production_kind(order: Option<super::Order>) -> u32 {
    match order {
        Some(super::Order::Construct { .. }) => 0xd8a30267,
        Some(super::Order::Train { .. }) => 0x15d11e18,
        Some(super::Order::Zone { .. }) => 0x080d3734,
        _ => u32::MAX,
    }
}

/// FixedPointT<8>::operator* at 1fceb20: signed 64-bit product, shift, low word.
pub fn multiply(left: i32, right: i32) -> i32 {
    ((i64::from(left) * i64::from(right)) >> 8) as i32
}

/// AddProgress284e030 computes this before modifying the selected progress.
pub fn remaining(cost: i32, progress: i32) -> i32 {
    cost.wrapping_shl(8).wrapping_sub(progress).max(0)
}

/// The generic yield caps overflow when production-kind modifiers amplify the
/// selected yield. The pre-existing overflow is consumed by the current build;
/// it is deliberately absent from this calculation (284e1e5..284e257).
pub fn completion_overflow(
    selected_yield: i32,
    generic_yield: i32,
    remaining_before: i32,
    allow_overflow: bool,
) -> i32 {
    if !allow_overflow || generic_yield < remaining_before {
        return 0;
    }
    generic_yield
        .min(selected_yield)
        .wrapping_sub(remaining_before)
        .max(0)
}

/// GetPercentComplete.Building2853260 includes the still-live overflow before
/// AddProgress clears it. ComputePercentComplete2d57aa0 uses Q8 multiplication,
/// Q8 division, arithmetic AsInt and an upper cap of100. None identifies the
/// native divide-by-zero case caused by a wrapped positive cost.
pub fn percent_complete(cost: i32, progress: i32, overflow: i32) -> Option<i32> {
    if cost <= 0 {
        return Some(-1);
    }
    let denominator = cost.wrapping_shl(8);
    if denominator == 0 {
        return None;
    }
    let numerator = multiply(progress.wrapping_add(overflow), 100 * 256);
    let quotient = ((i64::from(numerator) << 8) / i64::from(denominator)) as i32;
    Some((quotient >> 8).min(100))
}

/// UpdateUnit/BuildingProgress accumulates its Q8 multiplier first, then
/// multiplies selected yield plus old overflow once. Reassociating these
/// operations loses different fractional bits.
pub fn modified_progress(selected_yield: i32, overflow: i32, factors: &[i32]) -> i32 {
    let multiplier = factors
        .iter()
        .fold(256, |value, factor| multiply(value, *factor));
    multiply(selected_yield.wrapping_add(overflow), multiplier)
}

fn native_cvtt(value: f32) -> i32 {
    if value.is_finite() && (-2147483648.0..2147483648.0).contains(&value) {
        value as i32
    } else {
        i32::MIN
    }
}
/// FixedPoint(float) uses floor and a separate low fractional byte.
pub fn from_float(value: f32) -> i32 {
    let whole = value.floor();
    native_cvtt(whole).wrapping_shl(8) | i32::from(native_cvtt((value - whole) * 256.0) as u8)
}
/// Government/district production bonuses use single-precision 0.01 first.
pub fn percent_multiplier(percent: i32) -> i32 {
    from_float(percent as f32 * 0.01 + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_project_and_generic_kind_are_not_conflated() {
        assert_eq!(production_kind(None), u32::MAX);
        assert_eq!(
            production_kind(Some(super::super::Order::Advance { project_hash: 9 })),
            u32::MAX
        );
        assert_eq!(
            production_kind(Some(super::super::Order::Construct { building_hash: 9 })),
            0xd8a30267
        );
        assert_ne!(production_kind(None), 0);
    }
    #[test]
    fn multiplication_keeps_native_negative_floor_and_low_word() {
        assert_eq!(multiply(-1, 1), -1);
        assert_eq!(multiply(257, 257), 258);
        assert_eq!(multiply(i32::MAX, 512), -2);
    }
    #[test]
    fn modifier_rounding_and_existing_overflow_are_not_reordered() {
        assert_eq!(modified_progress(249, 1, &[250, 264]), 250);
        assert_ne!(multiply(multiply(250, 250), 264), 250);
    }
    #[test]
    fn production_percentage_uses_native_float_then_q8_floor() {
        assert_eq!(percent_multiplier(25), 320);
        assert_eq!(percent_multiplier(-1), 253);
        assert_eq!(percent_multiplier(-100), 0);
        assert_eq!(from_float(-0.1), -26);
    }
    #[test]
    fn report_percentage_observes_overflow_before_it_is_cleared() {
        assert_eq!(percent_complete(60, 1281, 1024), Some(15));
        assert_eq!(percent_complete(60, 1281, 0), Some(8));
        assert_eq!(percent_complete(60, 61 * 256, 0), Some(100));
        assert_eq!(percent_complete(60, -1, 0), Some(-1));
        assert_eq!(percent_complete(0, 256, 0), Some(-1));
        assert_eq!(percent_complete(1 << 24, 256, 0), None);
    }
    #[test]
    fn completion_caps_kind_bonus_and_never_carries_old_overflow() {
        assert_eq!(
            completion_overflow(20 * 256, 10 * 256, 8 * 256, true),
            2 * 256
        );
        assert_eq!(completion_overflow(5 * 256, 10 * 256, 8 * 256, true), 0);
        assert_eq!(completion_overflow(20 * 256, 10 * 256, 12 * 256, true), 0);
        assert_eq!(completion_overflow(20 * 256, 10 * 256, 8 * 256, false), 0);
        assert_eq!(remaining(10, 11 * 256), 0);
    }
}
