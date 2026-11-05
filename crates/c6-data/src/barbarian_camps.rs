//! Linux XP2 `Barbarian::Manager::CreateCamps` (0x282fcb0).
//!
//! Candidate evaluation is rules work, including in AI-named native jobs.
//! This boundary does not implement improvement construction or its callbacks;
//! those reached effects must be supplied over the same loaded state.
use c6_core::{loaded_state::LoadedState, unit::PlotCoord};
pub mod queries;

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub first_turn_percent: i32,
    pub minimum_city_distance: i32,
    pub minimum_camp_distance: i32,
    pub extra_distance_per_low_difficulty: i32,
}

#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    Query(E),
    Random(c6_core::loaded_state::game_sync::AccessError),
}

/// Live predicates, never cached native candidate scores or chosen locations.
/// Implementations must reject unavailable inputs and incomplete effects.
pub trait Context {
    type Error;
    fn is_water(&self, state: &LoadedState, plot: usize) -> Result<bool, Self::Error>;
    fn visible_to_major(&self, state: &LoadedState, plot: usize) -> Result<bool, Self::Error>;
    fn visible_to_any(&self, state: &LoadedState, plot: usize) -> Result<bool, Self::Error>;
    fn is_owned(&self, state: &LoadedState, plot: usize) -> Result<bool, Self::Error>;
    /// None means this is not a major-owned city; Some(true) is a human city.
    fn major_city(&self, state: &LoadedState, plot: usize) -> Result<Option<bool>, Self::Error>;
    /// Minimum handicap among alive human majors, or -1 when there are none.
    fn minimum_human_handicap(&self, state: &LoadedState) -> Result<i32, Self::Error>;
    fn distance(&self, state: &LoadedState, a: PlotCoord, b: PlotCoord)
        -> Result<i32, Self::Error>;
    /// Native CanHaveImprovement(plot, improvement, NO_PLAYER, false).
    fn can_have_improvement(
        &self,
        state: &LoadedState,
        plot: usize,
        improvement: i32,
    ) -> Result<bool, Self::Error>;
    /// Includes SetImprovementType callbacks, manager mutations and RNG. Also
    /// dispatches the revealed-major notification subscribers when reached.
    fn set_camp(
        &mut self,
        state: &mut LoadedState,
        plot: PlotCoord,
        improvement: i32,
        owner: i32,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Summary {
    pub attempted: u32,
    pub created: u32,
}

fn target(capacity: u32, current: u32, land: i32, unseen: i32) -> i32 {
    capacity
        .wrapping_mul(unseen as u32)
        .wrapping_div(land.max(1) as u32)
        .wrapping_sub(current) as i32
}

fn attempts(target: i32, initialized: u32, percent: i32) -> i32 {
    if target <= 0 {
        return 0;
    }
    if initialized != 0 {
        return 1;
    }
    let product = percent.wrapping_mul(target);
    if product < 100 {
        0
    } else {
        product / 100
    }
}

fn coord(index: usize, width: i32) -> PlotCoord {
    PlotCoord {
        x: (index % width as usize) as i32,
        y: (index / width as usize) as i32,
    }
}

fn evaluate<C: Context>(
    state: &LoadedState,
    context: &C,
    p: Parameters,
    improvement: i32,
    index: usize,
) -> Result<Option<i32>, Error<C::Error>> {
    if context.is_owned(state, index).map_err(Error::Query)?
        || context.visible_to_any(state, index).map_err(Error::Query)?
        || !context
            .can_have_improvement(state, index, improvement)
            .map_err(Error::Query)?
    {
        return Ok(None);
    }
    let map = state.world().map();
    let location = coord(index, map.header.width);
    let difficulty = context
        .minimum_human_handicap(state)
        .map_err(Error::Query)?;
    let extra = if difficulty != -1 && difficulty < 3 {
        3i32.wrapping_sub(difficulty)
            .wrapping_mul(p.extra_distance_per_low_difficulty)
    } else {
        0
    };
    let human_distance = p.minimum_city_distance.wrapping_add(extra);
    let radius = human_distance.max(p.minimum_camp_distance) as u32;
    let mut rejected = false;
    let mut city_distance = 0;
    for (neighbor, plot) in map.plots.iter().enumerate() {
        let distance = context
            .distance(state, location, coord(neighbor, map.header.width))
            .map_err(Error::Query)?;
        if distance as u32 > radius {
            continue;
        }
        if let Some(human) = context.major_city(state, neighbor).map_err(Error::Query)? {
            city_distance = city_distance.max(distance);
            rejected |= distance < p.minimum_city_distance;
            if !rejected && human_distance > p.minimum_city_distance && human {
                rejected |= distance < human_distance;
            }
        }
        if distance <= p.minimum_camp_distance
            && plot.improvement_hash == state.retained().barbarian.improvement_hash
        {
            rejected = true;
        }
    }
    if rejected {
        return Ok(None);
    }
    let manager = &state.retained().barbarian;
    let mut nearest = -1;
    let mut second = -1;
    for camp in &manager.instances {
        let distance = context
            .distance(
                state,
                location,
                PlotCoord {
                    x: camp.camp_location.x,
                    y: camp.camp_location.y,
                },
            )
            .map_err(Error::Query)?;
        if nearest < 0 || distance < nearest {
            second = nearest;
            nearest = distance;
        } else if second < 0 || distance < second {
            second = distance;
        }
    }
    let previous = manager
        .camp_locations
        .as_ref()
        .ok_or(Error::Missing("previous camp locations"))?;
    let mut old_distance = None;
    for camp in previous.iter().filter(|camp| camp.plot_index >= 0) {
        let distance = context
            .distance(
                state,
                location,
                coord(camp.plot_index as usize, map.header.width),
            )
            .map_err(Error::Query)?;
        old_distance = Some(old_distance.map_or(distance, |old: i32| old.min(distance)));
    }
    let score = nearest
        .wrapping_add(second)
        .wrapping_add(old_distance.unwrap_or(0))
        .max(0)
        .wrapping_shl(8)
        .wrapping_add(city_distance.wrapping_shl(8));
    Ok(Some(score))
}

/// Runs source ordering over canonical map, manager and GameSync state.
/// `improvement` is the definition-remapped native ordinal (0 is valid).
pub fn create<C: Context>(
    state: &mut LoadedState,
    context: &mut C,
    p: Parameters,
    improvement: i32,
) -> Result<Summary, Error<C::Error>> {
    let manager = &state.retained().barbarian;
    let mut summary = Summary {
        attempted: 0,
        created: 0,
    };
    if improvement == -1 || manager.value_e0 >= manager.value_c8 {
        return Ok(summary);
    }
    let map = state.world().map();
    if map.header.width <= 0 {
        return Err(Error::Missing("valid map width"));
    }
    // Jobs evaluate every eligible region before CreateCamps checks its target.
    let mut regions = Vec::new();
    for region in &map.regions.entries {
        let region = &region.value;
        let area = map
            .areas
            .entries
            .iter()
            .find(|a| a.value.id as u32 == region.area_id);
        if region.plot_count <= 10 && !area.is_some_and(|a| a.value.plot_count >= 11) {
            continue;
        }
        let mut candidates = Vec::new();
        for (index, _) in map
            .plots
            .iter()
            .enumerate()
            .filter(|(_, plot)| plot.region_id == region.id)
        {
            if let Some(score) = evaluate(state, context, p, improvement, index)? {
                candidates.push((index, score));
            }
        }
        candidates.sort_by_key(|&(_, score)| score);
        regions.push(candidates);
    }
    let mut land: i32 = 0;
    let mut unseen: i32 = 0;
    for index in 0..map.plots.len() {
        if !context.is_water(state, index).map_err(Error::Query)? {
            land = land.wrapping_add(1);
            if !context
                .visible_to_major(state, index)
                .map_err(Error::Query)?
            {
                unseen = unseen.wrapping_add(1);
            }
        }
    }
    let count = attempts(
        target(manager.value_c8, manager.value_e0, land, unseen),
        manager.value_f8,
        p.first_turn_percent,
    );
    if count <= 0 {
        return Ok(summary);
    }
    let Some(best) = regions
        .iter()
        .filter_map(|r| r.last().map(|&(_, score)| score))
        .max()
    else {
        return Ok(summary);
    };
    if best < 0 {
        return Ok(summary);
    }
    // Equal-score order remains native plot order after its stable sort.
    let mut weights: Vec<i32> = regions
        .iter()
        .map(|r| r.iter().filter(|&&(_, score)| score == best).count() as i32)
        .collect();
    let mut total = weights.iter().fold(0i32, |a, &b| a.wrapping_add(b));
    for _ in 0..count {
        if total == 0 {
            break;
        }
        let draw = state.game_sync_rand(total as u16).map_err(Error::Random)?;
        let mut remaining = i32::from(draw);
        let mut selected = weights.len() - 1;
        for (index, &weight) in weights.iter().enumerate() {
            remaining = remaining.wrapping_sub(weight);
            if remaining < 0 {
                selected = index;
                break;
            }
        }
        total = total.wrapping_sub(weights[selected]);
        weights[selected] = 0;
        let candidates = &regions[selected];
        let local_best = candidates
            .last()
            .ok_or(Error::Missing("selected region candidates"))?
            .1;
        let first = candidates.partition_point(|&(_, score)| score < local_best);
        let draw = state
            .game_sync_rand((candidates.len() - first) as u16)
            .map_err(Error::Random)?;
        let location = coord(
            candidates[first + usize::from(draw)].0,
            state.world().map().header.width,
        );
        summary.attempted += 1;
        if count >= 2 {
            let mut rejected = false;
            for camp in &state.retained().barbarian.instances {
                if context
                    .distance(
                        state,
                        location,
                        PlotCoord {
                            x: camp.camp_location.x,
                            y: camp.camp_location.y,
                        },
                    )
                    .map_err(Error::Query)?
                    <= p.minimum_camp_distance
                {
                    rejected = true;
                    break;
                }
            }
            if rejected {
                continue;
            }
        }
        context
            .set_camp(
                state,
                location,
                improvement,
                state.retained().barbarian.player_ordinal,
            )
            .map_err(Error::Query)?;
        summary.created += 1;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_uses_unseen_land_and_unsigned_wrapping() {
        assert_eq!(target(6, 0, 100, 50), 3);
        assert_eq!(target(6, 4, 100, 50), -1);
        assert_eq!(target(6, 0, 0, 0), 0);
        assert_eq!(target(u32::MAX, 0, 2, 2), i32::MAX);
    }
    #[test]
    fn first_turn_percent_is_truncated_after_signed_product() {
        assert_eq!(attempts(6, 0, 50), 3);
        assert_eq!(attempts(1, 0, 50), 0);
        assert_eq!(attempts(6, 1, 50), 1);
        assert_eq!(attempts(-1, 1, 50), 0);
        assert_eq!(attempts(i32::MAX, 0, 2), 0);
    }
}
