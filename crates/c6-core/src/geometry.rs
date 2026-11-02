//! Isolated native coordinate primitives. These do not validate map topology.

/// Native PlotCoord sentinel check; this is not map bounds validation.
pub const fn plot_valid(x: i32, y: i32) -> bool {
    x > -9999 && y > -9999
}

/// Isolated Map::IInstance::GetPlotDistance arithmetic (XP2 RVA 0x15ec0).
/// Wrapping and signed comparisons deliberately preserve native extreme-input
/// behavior, including the unusual negative-row conversion at i32::MIN.
pub fn plot_distance(width: i32, wrap: bool, x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    fn half(y: i32) -> i32 {
        if y >= 0 {
            y >> 1
        } else {
            let a = y.wrapping_sub(1);
            a.wrapping_sub(a >> 31) >> 1
        }
    }
    fn metric(dq: i32, dr: i32) -> i32 {
        if (dq >= 0) == (dr >= 0) {
            dq.wrapping_abs().wrapping_add(dr.wrapping_abs())
        } else {
            dq.wrapping_abs().max(dr.wrapping_abs())
        }
    }
    let dq = x2
        .wrapping_sub(half(y2))
        .wrapping_sub(x1.wrapping_sub(half(y1)));
    let dr = y2.wrapping_sub(y1);
    let distance = metric(dq, dr);
    if wrap {
        distance
            .min(metric(dq.wrapping_sub(width), dr))
            .min(metric(dq.wrapping_add(width), dr))
    } else {
        distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sentinel_check_is_not_bounds_check() {
        assert!(!plot_valid(-9999, 0));
        assert!(plot_valid(-9998, i32::MAX));
    }
    #[test]
    fn rows_and_horizontal_wrapping() {
        assert_eq!(plot_distance(10, false, 0, 0, 9, 0), 9);
        assert_eq!(plot_distance(10, true, 0, 0, 9, 0), 1);
        assert_eq!(plot_distance(10, false, 0, 0, 0, 1), 1);
        assert_eq!(plot_distance(10, false, 0, 0, 0, -1), 1);
    }
    #[test]
    fn extreme_arithmetic_does_not_panic() {
        for x in [i32::MIN, i32::MAX, -1, 0, 1] {
            let _ = plot_distance(x, true, i32::MIN, x, i32::MAX, i32::MIN);
        }
    }
}
