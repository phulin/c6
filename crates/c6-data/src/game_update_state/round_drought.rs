//! Linux XP2 drought target selection (296ece0/296ed60/2972030).
//! The supported targetless branch appends initialized event history without a
//! target RNG draw or drought object. Positive application remains explicit.
use super::{
    round_climate,
    round_random_events::{Error, PossibleEvent},
};
use crate::{registry::linux_type_hash, Rules};
use c6_core::{loaded_state::LoadedState, map::PlotFlag, unit::PlotCoord};

/// Live drought records omitted entirely by Climate wire version14.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Drought {
    pub random_event_ordinal: i32,
    pub severity_ordinal: i32,
    pub start_turn: i32,
    pub end_turn: i32,
    pub current_plot: i32,
    pub start_plot: i32,
    pub plots_affected: Vec<i32>,
    pub active_raw: u8,
    pub name_bytes: Vec<u8>,
}
struct Event {
    hexes: i32,
    spacing: i32,
    terrains: Vec<u32>,
}
pub struct Context {
    events: Vec<Event>,
    water: Vec<u32>,
    ocean_min_size: i32,
    speed_costs: Vec<(u32, i32)>,
}
fn int(row: crate::Row<'_>, column: &'static str) -> Result<i32, Error> {
    i32::try_from(row.integer(column).map_err(Error::Rules)?).map_err(|_| Error::Missing(column))
}
impl Context {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Missing(name));
        let mut rows: Vec<_> = table("RandomEvents")?.rows().collect();
        rows.sort_by_key(|r| r.row_id());
        let mut events = Vec::new();
        for row in rows {
            let name = row.text("RandomEventType").map_err(Error::Rules)?;
            let terrains = table("RandomEvent_Terrains")?
                .rows()
                .filter(|r| r.text("RandomEventType").ok() == Some(name))
                .map(|r| {
                    Ok(linux_type_hash(
                        r.text("TerrainType").map_err(Error::Rules)?.as_bytes(),
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?;
            events.push(Event {
                hexes: int(row, "Hexes")?,
                spacing: int(row, "Spacing")?,
                terrains,
            });
        }
        let water = table("Terrains")?
            .rows()
            .filter(|r| r.integer("Water").ok() == Some(1))
            .map(|r| {
                Ok(linux_type_hash(
                    r.text("TerrainType").map_err(Error::Rules)?.as_bytes(),
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let ocean_min_size = table("GlobalParameters")?
            .rows()
            .find(|r| r.text("Name").ok() == Some("OCEAN_MIN_WATER_SIZE"))
            .ok_or(Error::Missing("OCEAN_MIN_WATER_SIZE"))?
            .text("Value")
            .map_err(Error::Rules)?
            .parse()
            .map_err(|_| Error::Missing("OCEAN_MIN_WATER_SIZE"))?;
        let speed_costs = table("GameSpeeds")?
            .rows()
            .map(|r| {
                Ok((
                    linux_type_hash(r.text("GameSpeedType").map_err(Error::Rules)?.as_bytes()),
                    int(r, "CostMultiplier")?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            speed_costs,
            events,
            water,
            ocean_min_size,
        })
    }
    fn valid(
        &self,
        state: &LoadedState,
        climate: &round_climate::Runtime,
        event: &Event,
        index: usize,
    ) -> Result<bool, Error> {
        let map = state.world().map();
        let plot = map
            .plots
            .get(index)
            .ok_or(Error::Missing("drought plot index"))?;
        if plot.feature_hash != u32::MAX
            || plot.river_crossing_count > 0
            || (!event.terrains.is_empty() && !event.terrains.contains(&plot.terrain_hash))
            || climate
                .manager
                .storms
                .iter()
                .any(|s| s.plots_affected.contains(&(index as i32)))
        {
            return Ok(false);
        }
        // IsCoastalLand(-1) uses OCEAN_MIN_WATER_SIZE and current landmasses.
        if self.water.contains(&plot.terrain_hash) {
            return Ok(false);
        }
        for direction in 0..6 {
            let Some(next) = neighbor(state, index, direction)? else {
                continue;
            };
            let adjacent = &map.plots[next];
            if self.water.contains(&adjacent.terrain_hash)
                && !adjacent.flags.get(PlotFlag::Impassable)
                && (self.ocean_min_size <= 0
                    || map
                        .landmasses
                        .entries
                        .iter()
                        .find(|r| r.index == adjacent.landmass_id)
                        .is_some_and(|r| r.value.plot_count >= self.ocean_min_size))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn footprint(
        &self,
        state: &LoadedState,
        climate: &round_climate::Runtime,
        event: &Event,
        index: usize,
    ) -> Result<bool, Error> {
        if !self.valid(state, climate, event, index)? {
            return Ok(false);
        }
        if event.hexes == 0 {
            return Ok(false);
        }
        if event.hexes == 1 {
            return Ok(true);
        }
        // FindRequiredTilesAroundPlot(7) requires the entire radius1 circle.
        // IterateNeighbors visits center then native host translation order.
        if event.hexes != 7 {
            return Err(Error::Requires("drought non-seven footprint"));
        }
        for direction in [0, 4, 2, 3, 1, 5] {
            let Some(next) = neighbor(state, index, direction)? else {
                return Ok(false);
            };
            if !self.valid(state, climate, event, next)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
fn neighbor(state: &LoadedState, index: usize, direction: usize) -> Result<Option<usize>, Error> {
    let map = state.world().map();
    let (x, y) = map
        .coordinates(index as u32)
        .ok_or(Error::Missing("drought plot coordinates"))?;
    c6_core::found_city::placement::plot_from_delta(
        map,
        PlotCoord { x, y },
        [0, 1, 1, 0, -1, -1][direction],
        [1, 0, -1, -1, 0, 1][direction],
        1,
    )
    .map_err(|_| Error::Missing("drought adjacency"))
}
/// TriggerEvent returns immediately if ChooseDroughtStartPlot has no candidates.
/// This branch never constructs a drought, applies damage, or draws GameSync.
pub(super) fn apply(
    state: &LoadedState,
    climate: &round_climate::Runtime,
    event: PossibleEvent,
    runtime: &super::round_random_events::Runtime,
    history: &mut c6_core::game_random_events::HistoricalEvent,
) -> Result<(), Error> {
    let context = climate
        .drought_context
        .as_ref()
        .ok_or(Error::Requires("Climate current drought rules/prior state"))?;
    let definition = context
        .events
        .get(event.event as usize)
        .ok_or(Error::Missing("drought event definition"))?;
    if event.start_plot >= 0
        && context.valid(state, climate, definition, event.start_plot as usize)?
    {
        return Err(Error::Requires("positive drought application"));
    }
    for index in 0..state.world().map().plots.len() {
        if !context.footprint(state, climate, definition, index)? {
            continue;
        }
        let map = state.world().map();
        let (x, y) = map
            .coordinates(index as u32)
            .ok_or(Error::Missing("drought score coordinates"))?;
        let mut distance = i32::MAX;
        for drought in &climate.droughts {
            if drought.start_plot < 0 {
                continue;
            }
            let (px, py) = map
                .coordinates(drought.start_plot as u32)
                .ok_or(Error::Missing("previous drought coordinates"))?;
            distance = distance.min(c6_core::geometry::plot_distance(
                map.header.width,
                map.header.wrap_x,
                x,
                y,
                px,
                py,
            ));
        }
        let score = definition
            .spacing
            .wrapping_add(1)
            .wrapping_add(distance.wrapping_sub(definition.spacing).min(0));
        if score > 0 {
            return Err(Error::Requires("positive drought application"));
        }
    }
    // Unlike storms, ApplyEvent scales DROUGHT history duration by game speed
    // even if TriggerEvent returned before constructing a live drought.
    let percent = context
        .speed_costs
        .iter()
        .find(|(h, _)| *h == runtime.scheduling.game_speed_hash)
        .map_or(100, |(_, percent)| *percent);
    let duration = history.end_turn.wrapping_sub(history.start_turn);
    let q8 = i64::from(duration.wrapping_shl(8)) * i64::from(percent.wrapping_shl(8)) / 25_600;
    let scaled = (q8.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32) >> 8;
    history.end_turn = history.start_turn.wrapping_add(scaled);
    Ok(())
}
