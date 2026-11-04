//! Native short-vector accumulation and hex traversal for influence jobs.
use crate::{map::Map, player_ai_influence_map::InfluenceMap, unit::PlotCoord};

/// GetPlotFromDeltaWithRangeCheck2103f20, then GetPlotFromDelta2104a50.
pub fn from_delta(
    map: &Map,
    origin: PlotCoord,
    dx: i32,
    dy: i32,
    range: i32,
) -> Option<(usize, PlotCoord)> {
    let ax = dx.wrapping_abs();
    let ay = dy.wrapping_abs();
    let distance = if dx ^ dy < 0 {
        ax.max(ay)
    } else {
        ax.wrapping_add(ay)
    };
    if distance > range {
        return None;
    }
    from_axial(map, origin, dx, dy)
}
fn row_half(y: i32) -> i32 {
    if y >= 0 {
        y >> 1
    } else {
        let value = y.wrapping_sub(1);
        value.wrapping_add(((value as u32) >> 31) as i32) >> 1
    }
}
pub fn from_axial(map: &Map, origin: PlotCoord, dx: i32, dy: i32) -> Option<(usize, PlotCoord)> {
    if map.header.width <= 0 || map.header.height <= 0 {
        return None;
    }
    let y = origin.y.wrapping_add(dy);
    let x = origin
        .x
        .wrapping_sub(row_half(origin.y))
        .wrapping_add(dx)
        .wrapping_add(row_half(y));
    let x = crate::movement_world::coord_range(x, map.header.width, map.header.wrap_x);
    let y = crate::movement_world::coord_range(y, map.header.height, map.header.wrap_y);
    map.plot_at(x, y)?;
    Some((
        (y as usize) * map.header.width as usize + x as usize,
        PlotCoord { x, y },
    ))
}
const TRANSLATIONS: [(i32, i32); 6] = [(0, 1), (-1, 0), (1, -1), (0, -1), (1, 0), (-1, 1)];
const WALK: [usize; 6] = [1, 2, 0, 4, 5, 3];
/// EnumerateRing2a44b30 visits six corners and their intervening points. Null
/// plots are omitted; wrapping duplicates remain actual callback invocations.
pub fn ring(map: &Map, origin: PlotCoord, radius: u32) -> Vec<(usize, PlotCoord)> {
    if radius == 0 {
        return from_axial(map, origin, 0, 0).into_iter().collect();
    }
    let mut result = Vec::new();
    for direction in 0..6 {
        let (dx, dy) = TRANSLATIONS[direction];
        let (sx, sy) = (
            dx.wrapping_mul(radius as i32),
            dy.wrapping_mul(radius as i32),
        );
        let step = TRANSLATIONS[WALK[direction]];
        for index in 0..radius {
            if let Some(plot) = from_axial(
                map,
                origin,
                sx.wrapping_add(step.0.wrapping_mul(index as i32)),
                sy.wrapping_add(step.1.wrapping_mul(index as i32)),
            ) {
                result.push(plot);
            }
        }
    }
    result
}
pub fn add(map: &mut InfluenceMap, plot: usize, strength: i32, religious: bool) -> Option<()> {
    let values = if religious {
        map.religious_influence.as_mut()?
    } else {
        &mut map.influence
    };
    let value = values.get_mut(plot)?;
    *value = value.wrapping_add(strength as i16);
    Some(())
}
/// Friendly ranged defenses retain at least one positive threat. Both the
/// existing threat and attack strength are sign-extended native shorts.
pub fn suppress(map: &mut InfluenceMap, plot: usize, strength: i32) -> Option<()> {
    let value = map.influence.get_mut(plot)?;
    if *value > 0 {
        *value = (i32::from(*value).wrapping_sub(i32::from(strength as i16))).max(1) as i16;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_ring_order_odd_rows_and_wrapping_nulls_remain_visible() {
        let mut map = crate::world::tests::map();
        map.header.width = 8;
        map.header.height = 8;
        map.plots.resize(64, map.plots[0].clone());
        let origin = PlotCoord { x: 3, y: 3 };
        assert_eq!(
            ring(&map, origin, 1)
                .into_iter()
                .map(|(_, at)| (at.x, at.y))
                .collect::<Vec<_>>(),
            [(4, 4), (2, 3), (4, 2), (3, 2), (4, 3), (3, 4)]
        );
        let second = ring(&map, origin, 2);
        assert_eq!(second.len(), 12);
        assert_eq!(
            second
                .iter()
                .map(|(_, at)| (at.x, at.y))
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            12
        );
        assert!(from_delta(&map, origin, 1, 1, 1).is_none());
        assert!(from_delta(&map, origin, 1, -1, 1).is_some());
        map.header.wrap_x = true;
        assert!(from_axial(&map, PlotCoord { x: 0, y: 0 }, -8, 0).is_none());
        assert_eq!(
            from_axial(&map, PlotCoord { x: 0, y: 0 }, -1, 0)
                .unwrap()
                .1
                .x,
            7
        );
    }
    #[test]
    fn contribution_preserves_stream_short_wrap_and_friendly_floor() {
        let mut map = InfluenceMap {
            wire_version: 2,
            influence: vec![32760, 30, 0, -5],
            religious_influence: Some(vec![0; 4]),
        };
        add(&mut map, 0, 20, false).unwrap();
        add(&mut map, 1, 131071, true).unwrap();
        assert_eq!(map.influence[0], -32756);
        assert_eq!(map.religious_influence.as_ref().unwrap()[1], -1);
        suppress(&mut map, 1, 50).unwrap();
        suppress(&mut map, 2, 50).unwrap();
        suppress(&mut map, 3, 50).unwrap();
        assert_eq!(map.influence, [-32756, 1, 0, -5]);
        suppress(&mut map, 1, 65535).unwrap();
        assert_eq!(map.influence[1], 2);
    }
}
