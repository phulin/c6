//! Bounded Linux XP2 average damage arithmetic, not combat execution.
//!
//! Inputs are already resolved strengths and rule values. Actor selection,
//! modifiers, legal attacks, randomized damage and state application remain
//! unsupported. The native profile and comparison are in `docs/linux-combat-kernel.md`.

/// Original Q8 values are signed raw integers, with eight fractional bits.
/// Rule values must come from the caller's separately identified rules profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageParameters {
    pub attacker_strength_raw: i32,
    pub defender_strength_raw: i32,
    pub defender_max_hit_points: i32,
    pub outer_defense_damage: i32,
    pub outer_defense_max_hit_points: i32,
    pub defense_type: u32,
    pub power_scaling_raw: i32,
    pub minimum_damage: i32,
    pub base_damage: i32,
    /// Present in the native record but unused by this target helper.
    pub power_dampener: i32,
    /// Present in the native record; the helper reads the global rule instead.
    pub max_extra_damage: i32,
    pub bypass_defenses: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DamageError {
    /// The original fixed-point division would divide by zero after wrapping
    /// the positive outer-defense maximum into its Q8 representation.
    ZeroFixedPointDivisor,
    /// The measured original math binding requires Linux x86-64 with glibc.
    UnsupportedMathPlatform,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[link(name = "m")]
extern "C" {
    // The original unversioned GameCore relocation actually resolves to this
    // compatibility implementation in the measured genuine host process.
    #[link_name = "expf@GLIBC_2.2.5"]
    fn target_expf(value: f32) -> f32;
}

fn profile_exponential(value: f32) -> Result<f32, DamageError> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    {
        // SAFETY: glibc's expf accepts every f32 bit pattern and returns f32;
        // the versioned symbol and actual selected implementation are checked
        // against the original native binding by the comparison runner.
        Ok(unsafe { target_expf(value) })
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
    {
        let _ = value;
        Err(DamageError::UnsupportedMathPlatform)
    }
}

fn fixed_multiply(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}

fn fixed_as_float(raw: i32) -> f32 {
    // Preserve the original split conversion and single-precision addition.
    (raw & 255) as f32 * (1.0 / 256.0) + (raw >> 8) as f32
}

fn native_float_to_int(value: f32) -> i32 {
    // x86 CVTTSS2SI yields INT_MIN for NaN/overflow, whereas Rust casts saturate.
    if !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}

/// Evaluate the supplied Linux XP2 helper's average-only branch.
///
/// `global_max_extra_damage` is the actual `COMBAT_MAX_EXTRA_DAMAGE` rule,
/// separate from the identically named but unused parameter-record field.
/// Average mode consumes no RNG, including on either zero-strength early return.
/// The original fixed-point and wrapping operations are retained. Floating-point
/// equivalence requires the validated Linux x86-64/glibc runtime profile and its
/// actual `expf@GLIBC_2.2.5` implementation; it is not a cross-platform guarantee.
pub fn calculate_average_damage(
    p: &DamageParameters,
    global_max_extra_damage: i32,
) -> Result<i32, DamageError> {
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return Err(DamageError::UnsupportedMathPlatform);
    }
    if p.defender_strength_raw == 0 {
        return Ok(p.defender_max_hit_points);
    }
    if p.attacker_strength_raw == 0 {
        return Ok(p.minimum_damage);
    }
    let base = p.base_damage.wrapping_add(global_max_extra_damage / 2) as f32;
    let difference = p
        .attacker_strength_raw
        .wrapping_sub(p.defender_strength_raw);
    let exponent = fixed_as_float(fixed_multiply(p.power_scaling_raw, difference));
    let scaled = profile_exponential(exponent)? * base;
    let mut damage = native_float_to_int((scaled + 0.5).floor());

    // Target enum DISTRICT_DEFENSE_GARRISON: identity from matching DWARF, not an
    // assumed enum ordinal or a cross-platform layout.
    if p.defense_type == 0x5e97_d629 && !p.bypass_defenses {
        let remaining = p
            .outer_defense_max_hit_points
            .wrapping_shl(8)
            .wrapping_sub(p.outer_defense_damage.wrapping_shl(8));
        let mut reduction = 0;
        if remaining > 0 && p.outer_defense_max_hit_points > 0 {
            let divisor = p.outer_defense_max_hit_points.wrapping_shl(8);
            if divisor == 0 {
                return Err(DamageError::ZeroFixedPointDivisor);
            }
            let ratio = ((i64::from(remaining) << 8) / i64::from(divisor)) as i32;
            reduction = fixed_multiply(damage.wrapping_shl(8), ratio) >> 8;
        }
        damage = damage.wrapping_sub(reduction);
    }
    // Keep the native branch order even for contradictory minimum/maximums.
    Ok(if damage < p.minimum_damage {
        p.minimum_damage
    } else {
        damage.min(p.defender_max_hit_points)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_rounding_and_conversion_boundaries() {
        assert_eq!(fixed_multiply(-1, 1), -1);
        assert_eq!(fixed_as_float(-1), -1.0 / 256.0);
        assert_eq!(native_float_to_int(f32::INFINITY), i32::MIN);
        assert_eq!(native_float_to_int(f32::NAN), i32::MIN);
        assert_eq!(native_float_to_int(2_147_483_648.0), i32::MIN);
        assert_eq!(native_float_to_int(2_147_483_520.0), 2_147_483_520);
    }
}
