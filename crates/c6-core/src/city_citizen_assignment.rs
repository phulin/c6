//! Current Linux City worker scoring and allocation. All yields/capacities must
//! be computed from the current canonical City/plot/rules, not native answers.
//! Plot ownership, appeal updates and worker-change subscribers run in the
//! calling state adapter before/after these same owned Citizens mutations.
use crate::city_citizens::Citizens;

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
