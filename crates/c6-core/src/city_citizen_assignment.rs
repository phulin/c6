//! Current Linux City worker scoring and allocation. All yields/capacities must
//! be computed from the current canonical City/plot/rules, not native answers.
//! Plot ownership, appeal updates and worker-change subscribers run in the
//! calling state adapter before/after these same owned Citizens mutations.
use crate::city_citizens::Citizens;

/// CityHexspace.GetNumPlots1fe3bf0 -> ComputePlotsFromRadius(3).
pub const NUM_PLOTS: usize = 37;
/// Source CityHexspace offsets3bffdc0/3bffec0 in increasing City plot index.
/// Preserve this order for scoring ties, callbacks and native array indexing.
const PLOT_X: [i32; NUM_PLOTS] = [
    0, 0, 1, 1, 0, -1, -1, 0, 1, 2, 2, 2, 1, 0, -1, -2, -2, -2, -1, 0, 1, 2, 3, 3, 3, 3, 2, 1, 0,
    -1, -2, -3, -3, -3, -3, -2, -1,
];
const PLOT_Y: [i32; NUM_PLOTS] = [
    0, 1, 0, -1, -1, 0, 1, 2, 1, 0, -1, -2, -2, -2, -1, 0, 1, 2, 2, 3, 2, 1, 0, -1, -2, -3, -3, -3,
    -3, -2, -1, 0, 1, 2, 3, 3, 3,
];
fn row_half(y: i32) -> i32 {
    if y >= 0 {
        y >> 1
    } else {
        let n = y.wrapping_sub(1);
        n.wrapping_sub(n >> 31) >> 1
    }
}
/// Current canonical map lookup corresponding to GetPlotFromCenterLocation
/// 28c9260. No sorting/dedup: a small wrapped map may repeat the same plot at
/// multiple city indexes. Invalid index/center/off-map returns actual absence.
pub fn plot_location(
    map: &crate::map::Map,
    center: crate::unit::PlotCoord,
    index: i32,
) -> Result<Option<crate::unit::PlotCoord>, crate::movement_world::MissingInput> {
    let location = location(
        map.header.width,
        map.header.height,
        map.header.wrap_x,
        map.header.wrap_y,
        center,
        index,
    )?;
    if let Some(value) = location {
        crate::movement_world::plot_at(map, value)?;
    }
    Ok(location)
}
fn location(
    width: i32,
    height: i32,
    wrap_x: bool,
    wrap_y: bool,
    center: crate::unit::PlotCoord,
    index: i32,
) -> Result<Option<crate::unit::PlotCoord>, crate::movement_world::MissingInput> {
    if !crate::geometry::plot_valid(center.x, center.y) || !(0..NUM_PLOTS as i32).contains(&index) {
        return Ok(None);
    }
    if width <= 0 || height <= 0 {
        return Err(crate::movement_world::MissingInput::InvalidMapDimensions);
    }
    let index = index as usize;
    let y = center.y.wrapping_add(PLOT_Y[index]);
    let x = center
        .x
        .wrapping_sub(row_half(center.y))
        .wrapping_add(PLOT_X[index])
        .wrapping_add(row_half(y));
    let x = crate::movement_world::coord_range(x, width, wrap_x);
    let y = crate::movement_world::coord_range(y, height, wrap_y);
    Ok((x >= 0 && y >= 0 && x < width && y < height).then_some(crate::unit::PlotCoord { x, y }))
}

/// Native 28c90d0 inverse. Radius four has indices beyond the ordinary37 slots;
/// callers must retain the worker-vector bounds test rather than clamp them.
pub fn plot_index(
    map: &crate::map::Map,
    center: crate::unit::PlotCoord,
    plot: crate::unit::PlotCoord,
) -> Result<i32, crate::movement_world::MissingInput> {
    inverse(
        map.header.width,
        map.header.height,
        map.header.wrap_x,
        map.header.wrap_y,
        center,
        plot,
    )
}
fn inverse(
    width: i32,
    height: i32,
    wrap_x: bool,
    wrap_y: bool,
    center: crate::unit::PlotCoord,
    plot: crate::unit::PlotCoord,
) -> Result<i32, crate::movement_world::MissingInput> {
    if width <= 0 || height <= 0 {
        return Err(crate::movement_world::MissingInput::InvalidMapDimensions);
    }
    fn difference(value: i32, size: i32, wrap: bool) -> i32 {
        if !wrap {
            return value;
        }
        let half = ((size as u32) >> 1) as i32;
        if value > half {
            value.wrapping_sub(size)
        } else if value < half.wrapping_neg() {
            value.wrapping_add(size)
        } else {
            value
        }
    }
    const XY: [[i32; 9]; 9] = [
        [-1, -1, -1, -1, 53, 54, 55, 56, 57],
        [-1, -1, -1, 52, 31, 32, 33, 34, 58],
        [-1, -1, 51, 30, 15, 16, 17, 35, 59],
        [-1, 50, 29, 14, 5, 6, 18, 36, 60],
        [49, 28, 13, 4, 0, 1, 7, 19, 37],
        [48, 27, 12, 3, 2, 8, 20, 38, -1],
        [47, 26, 11, 10, 9, 21, 39, -1, -1],
        [46, 25, 24, 23, 22, 40, -1, -1, -1],
        [45, 44, 43, 42, 41, -1, -1, -1, -1],
    ];
    let dx = difference(plot.x.wrapping_sub(center.x), width, wrap_x);
    let dy = difference(plot.y.wrapping_sub(center.y), height, wrap_y);
    let new_y = center.y.wrapping_add(dy);
    let axial = center
        .x
        .wrapping_add(dx)
        .wrapping_sub(row_half(new_y))
        .wrapping_sub(center.x.wrapping_sub(row_half(center.y)));
    let dx = difference(axial, width, wrap_x);
    let distance = if (dx >= 0) == (dy >= 0) {
        dx.wrapping_abs().wrapping_add(dy.wrapping_abs())
    } else {
        dx.wrapping_abs().max(dy.wrapping_abs())
    };
    if distance > 4 {
        return Ok(-1);
    }
    XY.get(dx.wrapping_add(4) as u32 as usize)
        .and_then(|r| r.get(dy.wrapping_add(4) as u32 as usize))
        .copied()
        .ok_or(crate::movement_world::MissingInput::PlotOutsideMap)
}
/// Native286f480 adds both ordinary and forced workers, wrapping the sum.
/// A short ordinary vector proves out-of-range0; it does not supply missing
/// forced-vector contents for an otherwise in-range ordinary slot.
pub fn plot_workers(
    citizens: &Citizens,
    map: &crate::map::Map,
    center: crate::unit::PlotCoord,
    plot: crate::unit::PlotCoord,
) -> Result<i32, &'static str> {
    let index = plot_index(map, center, plot).map_err(|_| "current citizen plot index")?;
    index_workers(citizens, index)
}
fn index_workers(citizens: &Citizens, index: i32) -> Result<i32, &'static str> {
    let Some(ordinary) = citizens.working_plot.get(index as u32 as usize) else {
        return Ok(0);
    };
    let forced = citizens
        .forced_working_plot
        .get(index as usize)
        .ok_or("current forced-worker vector")?;
    Ok(ordinary.wrapping_add(*forced))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Scoring {
    pub food: i32,
    pub favored: i32,
    pub other: i32,
}
impl Scoring {
    fn add(self, b: Self) -> Self {
        Self {
            food: self.food.wrapping_add(b.food),
            favored: self.favored.wrapping_add(b.favored),
            other: self.other.wrapping_add(b.other),
        }
    }
    fn better(self, b: Self, needed_food: i32) -> bool {
        if b.food < needed_food && self.food >= needed_food {
            return true;
        }
        (self.favored, self.other) > (b.favored, b.other)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Yield {
    pub native_index: i32,
    pub amount: i32,
    pub favored: bool,
    pub disfavored: bool,
}
/// SimpleScoring.Initialize286fbd0: food counts even when disfavored; gold is
/// unscaled, other yields are doubled before favored/ordinary accumulation.
pub fn score(yields: impl IntoIterator<Item = Yield>, food_type: i32, gold_type: i32) -> Scoring {
    let mut out = Scoring::default();
    for y in yields {
        if y.native_index == food_type {
            out.food = out.food.wrapping_add(y.amount);
        }
        if y.disfavored {
            continue;
        }
        let amount = if y.native_index == gold_type {
            y.amount
        } else {
            y.amount.wrapping_shl(1)
        };
        if y.favored {
            out.favored = out.favored.wrapping_add(amount);
        } else {
            out.other = out.other.wrapping_add(amount);
        }
    }
    out
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    AllocationLimit,
    ArrayIndex,
    CurrentArrayShape,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "City citizen assignment: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Default)]
struct Tracker {
    score: Scoring,
    take: bool,
}
/// Source bounded_knapsack286ff40. Each candidate represents ONE available
/// worker slot (capacity expansion preserves plot order). Strict score ties
/// retain the earlier item. Backtracking emits the actual reverse item order
/// consumed by TryAssignAllWorkers; never sort it before subscriber delivery.
pub fn choose_workers(
    candidates: &[Scoring],
    unassigned: u32,
    food_surplus_q8: i32,
    max_cells: usize,
) -> Result<Vec<usize>, Error> {
    if unassigned == 0 {
        return Ok(vec![]);
    }
    let cols = (unassigned as usize)
        .checked_add(1)
        .ok_or(Error::AllocationLimit)?;
    let rows = candidates
        .len()
        .checked_add(1)
        .ok_or(Error::AllocationLimit)?;
    let cells = rows
        .checked_mul(cols)
        .filter(|n| *n <= max_cells)
        .ok_or(Error::AllocationLimit)?;
    let mut table = Vec::new();
    table
        .try_reserve_exact(cells)
        .map_err(|_| Error::AllocationLimit)?;
    table.resize(cells, Tracker::default());
    // AsInt is arithmetic shift; source neg+cmovs clamps the food deficit to zero.
    let needed = (food_surplus_q8 >> 8).wrapping_neg().max(0);
    for (i, item) in candidates.iter().enumerate() {
        for n in 0..cols {
            let previous = table[i * cols + n].score;
            let next = if n > 0 {
                let proposed = table[i * cols + n - 1].score.add(*item);
                if proposed.better(previous, needed) {
                    Tracker {
                        score: proposed,
                        take: true,
                    }
                } else {
                    Tracker {
                        score: previous,
                        take: false,
                    }
                }
            } else {
                Tracker {
                    score: previous,
                    take: false,
                }
            };
            table[(i + 1) * cols + n] = next;
        }
    }
    let mut out = Vec::new();
    out.try_reserve_exact(candidates.len().min(unassigned as usize))
        .map_err(|_| Error::AllocationLimit)?;
    let mut n = unassigned as usize;
    for i in (1..rows).rev() {
        if table[i * cols + n].take {
            out.push(i - 1);
            n -= 1;
        }
    }
    Ok(out)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkerChange {
    pub city_plot_index: i32,
    pub previous_workers: i32,
}
/// Exact AssignCitizenToWorkPlot286ec00 arithmetic after the current plot has
/// been resolved to its CityHexspace index. Capacity is evaluated only when
/// consume_citizen=true; center index0 consumes no population even then.
/// Returns the old-value report payload when the caller requested reporting.
pub fn assign_to_index(
    citizens: &mut Citizens,
    index: i32,
    num_city_plots: i32,
    consume_citizen: bool,
    capacity: Option<i32>,
    send: bool,
) -> Result<Option<WorkerChange>, Error> {
    if index < 0 || index >= num_city_plots {
        return Ok(None);
    }
    let at = index as usize;
    let current = *citizens.working_plot.get(at).ok_or(Error::ArrayIndex)?;
    if consume_citizen {
        let capacity = capacity.ok_or(Error::CurrentArrayShape)?;
        if current >= capacity {
            return Ok(None);
        }
    }
    let previous_workers = current.wrapping_add(
        *citizens
            .forced_working_plot
            .get(at)
            .ok_or(Error::ArrayIndex)?,
    );
    citizens.working_plot[at] = current.wrapping_add(1);
    if index != 0 {
        citizens.num_citizens_working_plots = citizens.num_citizens_working_plots.wrapping_add(1);
        if consume_citizen {
            citizens.num_unassigned_citizens = citizens.num_unassigned_citizens.wrapping_sub(1);
        }
    }
    Ok(send.then_some(WorkerChange {
        city_plot_index: index,
        previous_workers,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn city_index_order_preserves_odd_rows_and_wrapped_duplicates() {
        use crate::unit::PlotCoord;
        let center = PlotCoord { x: 18, y: 11 };
        let ring = (0..7)
            .map(|i| location(44, 26, true, false, center, i).unwrap().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            ring,
            [
                (18, 11),
                (19, 12),
                (19, 11),
                (19, 10),
                (18, 10),
                (17, 11),
                (18, 12)
            ]
            .map(|(x, y)| PlotCoord { x, y })
        );
        assert_eq!(
            location(1, 1, true, true, PlotCoord { x: 0, y: 0 }, 0).unwrap(),
            Some(PlotCoord { x: 0, y: 0 })
        );
        assert_eq!(
            location(1, 1, true, true, PlotCoord { x: 0, y: 0 }, 2).unwrap(),
            Some(PlotCoord { x: 0, y: 0 })
        );
        // Native CoordRange(-dimension) yields dimension, therefore no plot.
        assert_eq!(
            location(1, 1, true, true, PlotCoord { x: 0, y: 0 }, 5).unwrap(),
            None
        );
        assert_eq!(location(44, 26, false, false, center, 37).unwrap(), None);
        assert_eq!(
            location(0, 0, true, true, PlotCoord { x: -9999, y: -9999 }, 0).unwrap(),
            None
        );
    }
    #[test]
    fn scoring_keeps_food_when_disfavored_and_gold_unscaled() {
        assert_eq!(
            score(
                [
                    Yield {
                        native_index: 0,
                        amount: 3,
                        favored: false,
                        disfavored: true
                    },
                    Yield {
                        native_index: 1,
                        amount: 5,
                        favored: true,
                        disfavored: false
                    },
                    Yield {
                        native_index: 2,
                        amount: 2,
                        favored: false,
                        disfavored: false
                    }
                ],
                0,
                1
            ),
            Scoring {
                food: 3,
                favored: 5,
                other: 4
            }
        );
    }
    #[test]
    fn allocation_food_threshold_ties_and_native_backtracking_order() {
        let a = Scoring {
            food: 0,
            favored: 0,
            other: 8,
        };
        let b = Scoring {
            food: 1,
            favored: 0,
            other: 2,
        };
        assert_eq!(choose_workers(&[a, b], 1, -256, 100), Ok(vec![1]));
        assert_eq!(choose_workers(&[a, a], 1, 0, 100), Ok(vec![0]));
        assert_eq!(choose_workers(&[a, b], 2, -256, 100), Ok(vec![1, 0]));
        assert_eq!(choose_workers(&[a, b], 0, 0, 0), Ok(vec![]));
        assert_eq!(
            choose_workers(&[a, b], 1, 0, 2),
            Err(Error::AllocationLimit)
        );
    }
    #[test]
    fn adding_center_never_consumes_population_and_reports_old_worker_count() {
        let mut c = Citizens {
            working_plot: vec![0, 0],
            forced_working_plot: vec![0, 0],
            favored_yields: vec![],
            disfavored_yields: vec![],
            num_unassigned_citizens: 1,
            num_citizens_working_plots: 0,
            force_update_plot_appeal: false,
        };
        assert_eq!(
            assign_to_index(&mut c, 0, 2, false, None, true),
            Ok(Some(WorkerChange {
                city_plot_index: 0,
                previous_workers: 0
            }))
        );
        assert_eq!(
            (c.num_unassigned_citizens, c.num_citizens_working_plots),
            (1, 0)
        );
        assert_eq!(
            assign_to_index(&mut c, 1, 2, true, Some(1), true),
            Ok(Some(WorkerChange {
                city_plot_index: 1,
                previous_workers: 0
            }))
        );
        assert_eq!(
            (c.num_unassigned_citizens, c.num_citizens_working_plots),
            (0, 1)
        );
        assert_eq!(assign_to_index(&mut c, 1, 2, true, Some(1), true), Ok(None));
        assert_eq!(c.working_plot, [1, 1]);
        c.working_plot[1] = 0;
        c.forced_working_plot[1] = 2;
        assert_eq!(
            assign_to_index(&mut c, 1, 2, true, Some(1), true),
            Ok(Some(WorkerChange {
                city_plot_index: 1,
                previous_workers: 2
            }))
        );
    }
}

#[cfg(test)]
mod inverse_tests {
    use super::*;
    use crate::unit::PlotCoord;
    #[test]
    fn all_standard_slots_roundtrip_across_both_wrapped_edges() {
        for center in [
            PlotCoord { x: 0, y: 0 },
            PlotCoord { x: 43, y: 25 },
            PlotCoord { x: 18, y: 11 },
        ] {
            for index in 0..37 {
                let plot = location(44, 26, true, true, center, index)
                    .unwrap()
                    .unwrap();
                assert_eq!(inverse(44, 26, true, true, center, plot), Ok(index));
            }
        }
    }
    #[test]
    fn half_map_ties_keep_direction_and_small_maps_keep_aliases() {
        assert_eq!(
            inverse(
                8,
                8,
                true,
                true,
                PlotCoord { x: 0, y: 0 },
                PlotCoord { x: 4, y: 0 }
            ),
            Ok(41)
        );
        assert_eq!(
            inverse(
                8,
                8,
                true,
                true,
                PlotCoord { x: 4, y: 0 },
                PlotCoord { x: 0, y: 0 }
            ),
            Ok(53)
        );
        let center = PlotCoord { x: 0, y: 0 };
        let alias = location(3, 3, true, true, center, 22).unwrap().unwrap();
        assert_eq!(alias, center);
        assert_eq!(inverse(3, 3, true, true, center, alias), Ok(0));
    }
    #[test]
    fn worker_count_sums_forced_and_ordinary_and_bounds_before_reading() {
        let citizens = Citizens {
            working_plot: vec![i32::MAX],
            forced_working_plot: vec![1],
            favored_yields: vec![],
            disfavored_yields: vec![],
            num_unassigned_citizens: 0,
            num_citizens_working_plots: 0,
            force_update_plot_appeal: false,
        };
        assert_eq!(index_workers(&citizens, 0), Ok(i32::MIN));
        assert_eq!(index_workers(&citizens, -1), Ok(0));
        assert_eq!(index_workers(&citizens, 41), Ok(0));
    }
}
