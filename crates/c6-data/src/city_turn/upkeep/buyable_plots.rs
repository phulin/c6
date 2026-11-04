//! Current culture-expansion candidates. Candidate enumeration is pure; only
//! `next_plot` consumes the canonical GameSync stream, once for a nonempty list.
use crate::{
    city_turn::{tiles, yield_definitions, Error},
    game_parameters::sql_integer,
    registry::linux_type_hash,
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    found_city::placement::plot_from_delta,
    loaded_state::LoadedState,
    map::{Map, Plot},
    unit::PlotCoord,
};

pub struct Definitions {
    binding: WireBinding,
    maximum: i32,
    distance: i32,
    ring: i32,
    water: i32,
    improvement: i32,
    resource: i32,
    wonder: i32,
    yield_point: i32,
    barbarian_camp: Option<u32>,
    terrains: Vec<(u32, bool)>,
    features: Vec<(u32, bool)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("GlobalParameters"))?;
        let parameter = |name| {
            parameters
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .and_then(|r| r.get("Value").ok())
                .and_then(sql_integer)
                .ok_or(Error::Unsupported(name))
        };
        let flags = |table, column, flag| -> Result<Vec<(u32, bool)>, Error> {
            rules
                .gameplay()
                .table(table)
                .ok_or(Error::Unsupported(table))?
                .rows()
                .map(|r| {
                    Ok((
                        linux_type_hash(r.text(column).map_err(Error::Rules)?.as_bytes()),
                        sql_integer(r.get(flag).map_err(Error::Rules)?)
                            .ok_or(Error::Unsupported(flag))?
                            != 0,
                    ))
                })
                .collect()
        };
        let improvements = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(Error::Definitions)?;
        let improvement_table = rules
            .gameplay()
            .table("Improvements")
            .ok_or(Error::Unsupported("Improvements"))?;
        let mut barbarian_camp = None;
        for entry in improvements.entries() {
            let row = improvement_table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("Improvement definition"))?;
            if sql_integer(row.get("BarbarianCamp").map_err(Error::Rules)?)
                .ok_or(Error::Unsupported("BarbarianCamp"))?
                != 0
            {
                barbarian_camp = Some(linux_type_hash(&entry.type_name));
                break;
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            barbarian_camp,
            maximum: parameter("PLOT_INFLUENCE_MAX_ACQUIRE_DISTANCE")?,
            distance: parameter("PLOT_INFLUENCE_DISTANCE_MULTIPLIER")?,
            ring: parameter("PLOT_INFLUENCE_RING_COST")?,
            water: parameter("PLOT_INFLUENCE_WATER_COST")?,
            improvement: parameter("PLOT_INFLUENCE_IMPROVEMENT_COST")?,
            resource: parameter("PLOT_INFLUENCE_RESOURCE_COST")?,
            wonder: parameter("PLOT_INFLUENCE_NW_COST")?,
            yield_point: parameter("PLOT_INFLUENCE_YIELD_POINT_COST")?,
            terrains: flags("Terrains", "TerrainType", "Water")?,
            features: flags("Features", "FeatureType", "NaturalWonder")?,
        })
    }
    fn natural_wonder(&self, plot: &Plot) -> Result<bool, Error> {
        if plot.feature_hash == u32::MAX {
            return Ok(false);
        }
        self.features
            .iter()
            .find(|r| r.0 == plot.feature_hash)
            .map(|r| r.1)
            .ok_or(Error::Unsupported("buyable plot Feature definition"))
    }
    fn water(&self, plot: &Plot) -> Result<bool, Error> {
        self.terrains
            .iter()
            .find(|r| r.0 == plot.terrain_hash)
            .map(|r| r.1)
            .ok_or(Error::Unsupported("buyable plot Terrain definition"))
    }
}
fn owner(plot: &Plot) -> i32 {
    plot.city_attributes
        .as_ref()
        .map_or(-1, |a| i32::from(a.player))
}
fn location(map: &Map, plot: u32) -> Result<PlotCoord, Error> {
    map.coordinates(plot)
        .map(|(x, y)| PlotCoord { x, y })
        .ok_or(Error::Unsupported("buyable plot location"))
}
const DX: [i32; 6] = [0, 1, 1, 0, -1, -1];
const DY: [i32; 6] = [1, 0, -1, -1, 0, 1];
fn neighbor(map: &Map, at: PlotCoord, direction: usize) -> Result<Option<usize>, Error> {
    plot_from_delta(map, at, DX[direction], DY[direction], 1)
        .map_err(|_| Error::Unsupported("buyable plot adjacency"))
}
fn distance(map: &Map, a: PlotCoord, b: PlotCoord) -> i32 {
    c6_core::geometry::plot_distance(map.header.width, map.header.wrap_x, a.x, a.y, b.x, b.y)
}
/// Exact minimum-score tie list in coordinate-visit order. No random draw is
/// performed here; territory acquisition can use its existing selection seam.
pub fn ordered_candidates(
    state: &LoadedState,
    owner_id: i32,
    city_id: u32,
    defs: &Definitions,
    yields: &yield_definitions::Definitions,
    current: &tiles::Current<'_>,
) -> Result<Vec<u32>, Error> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("buyable plot rules binding"));
    }
    let city = state.city(owner_id, city_id)?;
    let center = PlotCoord {
        x: city.header.location_x,
        y: city.header.location_y,
    };
    let map = state.world().map();
    // A negative range has an empty native outer loop. MIN's negation wraps.
    let lower = defs.maximum.wrapping_neg();
    if lower > defs.maximum {
        return Ok(Vec::new());
    }
    tiles::with_context(state, yields, current, |context| {
        let mut result = Vec::new();
        let mut best = i32::MAX;
        for dx in lower..=defs.maximum {
            for dy in lower..=defs.maximum {
                let Some(index) = plot_from_delta(map, center, dx, dy, defs.maximum)
                    .map_err(|_| Error::Unsupported("buyable plot range"))?
                else {
                    continue;
                };
                let plot = &map.plots[index];
                if owner(plot) != -1 {
                    continue;
                }
                let at = location(map, index as u32)?;
                let mut connected = false;
                for direction in 0..6 {
                    if let Some(adj) = neighbor(map, at, direction)? {
                        let adjacent = &map.plots[adj];
                        if owner(adjacent) == owner_id
                            && adjacent
                                .city_attributes
                                .as_ref()
                                .is_some_and(|a| a.owning_city == city_id)
                        {
                            connected = true;
                            break;
                        }
                    }
                }
                if !connected {
                    continue;
                }
                let range = distance(map, at, center);
                let base = range.wrapping_mul(defs.distance).wrapping_mul(2);
                if base <= 0 {
                    continue;
                }
                let visible = current.resource_visibility.visible_resource(
                    state,
                    owner_id,
                    plot.resource_hash as i32,
                )? != -1;
                let mut score = base;
                if visible {
                    if range <= 3 {
                        score = score.wrapping_add(defs.resource);
                    }
                } else {
                    if defs.water(plot)? {
                        score = score.wrapping_add(defs.water);
                    }
                    if range > 3 {
                        score = score.wrapping_add(defs.ring);
                    }
                }
                if let Some(improvement) = yields.improvements.find_index(plot.improvement_hash) {
                    let camp = defs
                        .barbarian_camp
                        .and_then(|hash| yields.improvements.find_index(hash));
                    score = score.wrapping_add(if Some(improvement) == camp {
                        defs.ring
                    } else {
                        defs.improvement
                    });
                }
                if defs.natural_wonder(plot)? {
                    score = score.wrapping_add(defs.wonder);
                }
                for kind in yields.yield_indices() {
                    let value = c6_core::plot_yield::get_yield(
                        context,
                        index as u32,
                        kind,
                        owner_id,
                        false,
                    )
                    .map_err(Error::PlotYield)?;
                    score = score.wrapping_add(value.wrapping_mul(defs.yield_point));
                }
                let mut nearby_wonder = false;
                for direction in 0..6 {
                    let Some(adj) = neighbor(map, at, direction)? else {
                        continue;
                    };
                    let adjacent = &map.plots[adj];
                    if owner(adjacent) != -1 {
                        continue;
                    }
                    let adjacent_distance = distance(map, location(map, adj as u32)?, center);
                    if current.resource_visibility.visible_resource(
                        state,
                        owner_id,
                        adjacent.resource_hash as i32,
                    )? != -1
                    {
                        score = score.wrapping_sub(1);
                    }
                    if defs.natural_wonder(adjacent)? {
                        nearby_wonder |= adjacent_distance < 4;
                        score = score.wrapping_sub(1);
                    }
                }
                score = score.wrapping_sub(i32::from(nearby_wonder));
                retain_minimum(&mut result, &mut best, index as u32, score);
            }
        }
        Ok(result)
    })
}
fn retain_minimum(result: &mut Vec<u32>, best: &mut i32, plot: u32, score: i32) {
    if score < *best {
        result.clear();
        *best = score;
    }
    if score == *best {
        result.push(plot);
    }
}
#[derive(Debug)]
pub enum SelectionError {
    Candidate(Error),
    Random(c6_core::loaded_state::game_sync::AccessError),
    DrawIndex,
}
impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "buyable plot: {self:?}")
    }
}
impl std::error::Error for SelectionError {}
pub fn next_plot(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    defs: &Definitions,
    yields: &yield_definitions::Definitions,
    current: &tiles::Current<'_>,
) -> Result<Option<u32>, SelectionError> {
    let plots = ordered_candidates(state, owner, city, defs, yields, current)
        .map_err(SelectionError::Candidate)?;
    if plots.is_empty() {
        return Ok(None);
    }
    let draw = state
        .game_sync_rand(plots.len() as u16)
        .map_err(SelectionError::Random)?;
    plots
        .get(draw as usize)
        .copied()
        .map(Some)
        .ok_or(SelectionError::DrawIndex)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lower_score_replaces_and_ties_preserve_visit_order_and_duplicates() {
        let mut list = vec![];
        let mut best = i32::MAX;
        for (plot, score) in [(9, 40), (3, 40), (7, 39), (2, 39), (7, 39), (1, 50)] {
            retain_minimum(&mut list, &mut best, plot, score);
        }
        assert_eq!(list, [7, 2, 7]);
    }
}
