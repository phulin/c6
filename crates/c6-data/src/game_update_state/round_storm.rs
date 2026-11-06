//! Linux bounded dust-storm and hurricane execution (296c600/296db70/2970a50). Rules and
//! configuration are explicit; all random choices use the current GameSync.
//! Occupied damage branches and imported storms without live start plots remain
//! unsupported. Presentation reporting contains no reached gameplay RNG.
use super::{
    round_climate,
    round_random_events::{Error, PossibleEvent, Runtime},
};
use crate::{registry::linux_type_hash, Rules};
use c6_core::{
    loaded_state::LoadedState,
    map::{Component, DefinitionValue, PlotFlag},
    unit::PlotCoord,
};

struct Damage {
    kind: u32,
    percentage: i32,
    coastal_lowland_percentage: Option<i32>,
}
struct Event {
    hash: u32,
    hexes: i32,
    movement: i32,
    spacing: i32,
    duration: i32,
    terrains: Vec<u32>,
    damages: Vec<Damage>,
    unsupported_damage_options: bool,
    yields: Vec<(u32, i32, i32)>,
}
struct Wind {
    min: i32,
    max: i32,
    direction: usize,
    weight: i32,
}
/// Current alive-major order and CivilizationType hashes come from the same
/// configuration as the loaded state. Name usage stays in retained Player.
pub struct Context {
    events: Vec<Event>,
    winds: Vec<Wind>,
    names: Vec<(u32, Vec<Vec<u8>>)>,
    alive_major_ids: Vec<i32>,
    civilization_by_player: Vec<(i32, u32)>,
    start_plots: Vec<i32>,
    water: Vec<u32>,
}
fn int(r: crate::Row<'_>, col: &'static str) -> Result<i32, Error> {
    i32::try_from(r.integer(col).map_err(Error::Rules)?).map_err(|_| Error::Missing(col))
}
fn hash(r: crate::Row<'_>, col: &'static str) -> Result<u32, Error> {
    Ok(linux_type_hash(
        r.text(col).map_err(Error::Rules)?.as_bytes(),
    ))
}
impl Context {
    /// Omitted native Storm.startPlot, in canonical stored-storm order.
    pub fn start_plots(&self) -> &[i32] {
        &self.start_plots
    }
    pub fn from_rules(
        rules: &Rules,
        alive_major_ids: Vec<i32>,
        civilization_by_player: Vec<(i32, u32)>,
    ) -> Result<Self, Error> {
        let table = |n| rules.gameplay().table(n).ok_or(Error::Missing(n));
        let mut event_rows: Vec<_> = table("RandomEvents")?.rows().collect();
        event_rows.sort_by_key(|r| r.row_id());
        let mut events = Vec::new();
        for row in event_rows {
            let name = row.text("RandomEventType").map_err(Error::Rules)?;
            let matching = |t| -> Result<Vec<crate::Row<'_>>, Error> {
                let mut rows: Vec<_> = table(t)?
                    .rows()
                    .filter(|r| r.text("RandomEventType").ok() == Some(name))
                    .collect();
                rows.sort_by_key(|r| r.row_id());
                Ok(rows)
            };
            let terrains = matching("RandomEvent_Terrains")?
                .into_iter()
                .map(|r| hash(r, "TerrainType"))
                .collect::<Result<_, _>>()?;
            let mut damages = Vec::new();
            let mut unsupported_damage_options = false;
            for r in matching("RandomEvent_Damages")? {
                // Turn-bounded damage rows remain explicit unsupported branches.
                if ["MinTurn", "MaxTurn"]
                    .iter()
                    .any(|c| !matches!(r.get(c), Ok(crate::SqlValue::Null)))
                {
                    unsupported_damage_options = true;
                    continue;
                }
                let coastal_lowland_percentage =
                    match r.get("CoastalLowlandPercentage").map_err(Error::Rules)? {
                        crate::SqlValue::Null => None,
                        crate::SqlValue::Integer(value) => Some(
                            i32::try_from(*value)
                                .map_err(|_| Error::Missing("coastal lowland damage percentage"))?,
                        ),
                        _ => return Err(Error::Missing("coastal lowland damage percentage")),
                    };
                damages.push(Damage {
                    kind: hash(r, "DamageType")?,
                    percentage: int(r, "Percentage")?,
                    coastal_lowland_percentage,
                });
            }
            let mut yields = Vec::new();
            for r in matching("RandomEvent_Yields")? {
                yields.push((
                    hash(r, "YieldType")?,
                    int(r, "Percentage")?,
                    int(r, "Amount")?,
                ));
            }
            events.push(Event {
                hash: linux_type_hash(name.as_bytes()),
                hexes: int(row, "Hexes")?,
                movement: int(row, "Movement")?,
                spacing: int(row, "Spacing")?,
                duration: int(row, "Duration")?,
                terrains,
                damages,
                unsupported_damage_options,
                yields,
            });
        }
        let mut winds = Vec::new();
        let mut rows: Vec<_> = table("PrevailingWinds")?.rows().collect();
        rows.sort_by_key(|r| r.row_id());
        for r in rows {
            let direction = match r.text("DirectionType").map_err(Error::Rules)? {
                "DIRECTION_NORTHEAST" => 0,
                "DIRECTION_EAST" => 1,
                "DIRECTION_SOUTHEAST" => 2,
                "DIRECTION_SOUTHWEST" => 3,
                "DIRECTION_WEST" => 4,
                "DIRECTION_NORTHWEST" => 5,
                _ => return Err(Error::Missing("prevailing wind direction")),
            };
            winds.push(Wind {
                min: int(r, "MinimumLatitude")?,
                max: int(r, "MaximumLatitude")?,
                direction,
                weight: int(r, "Weight")?,
            });
        }
        let mut names: Vec<(u32, Vec<Vec<u8>>)> = Vec::new();
        let mut rows: Vec<_> = table("CivilizationCitizenNames")?.rows().collect();
        rows.sort_by_key(|r| r.row_id());
        for r in rows {
            let civ = hash(r, "CivilizationType")?;
            let name = r
                .text("CitizenName")
                .map_err(Error::Rules)?
                .as_bytes()
                .to_vec();
            if let Some((_, v)) = names.iter_mut().find(|(h, _)| *h == civ) {
                v.push(name)
            } else {
                names.push((civ, vec![name]))
            }
        }
        let water = table("Terrains")?
            .rows()
            .filter(|r| r.integer("Water").ok() == Some(1))
            .map(|r| hash(r, "TerrainType"))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            events,
            winds,
            names,
            alive_major_ids,
            civilization_by_player,
            start_plots: Vec::new(),
            water,
        })
    }
    fn event(&self, index: i32) -> Result<&Event, Error> {
        self.events
            .get(index as usize)
            .ok_or(Error::Missing("storm definition"))
    }
    fn latitude(&self, state: &LoadedState, index: i32) -> Result<i32, Error> {
        let map = state.world().map();
        let (x, y) = map
            .coordinates(index as u32)
            .ok_or(Error::Missing("storm coordinates"))?;
        // Map.IInstance.GetPlotLatitude2a44760 chooses X only for vertical-wrap,
        // non-horizontal-wrap maps; both wraps and neither use Y.
        let (axis, size) = if map.header.wrap_y && !map.header.wrap_x {
            (x, map.header.width)
        } else {
            (y, map.header.height)
        };
        if size <= 0 {
            return Err(Error::Missing("storm latitude dimensions"));
        }
        Ok(map.header.bottom_latitude.wrapping_add(
            map.header
                .top_latitude
                .wrapping_sub(map.header.bottom_latitude)
                .wrapping_mul(axis.wrapping_mul(100) / size)
                / 100,
        ))
    }
    fn wind_options(
        &self,
        state: &LoadedState,
        index: i32,
        adjacent: bool,
    ) -> Result<Vec<(i32, i32)>, Error> {
        let latitude = self.latitude(state, index)?;
        let mut out = Vec::new();
        for wind in &self.winds {
            if wind.min <= latitude && latitude <= wind.max {
                if adjacent {
                    if let Some(i) = neighbor(state, index, wind.direction)? {
                        out.push((i, wind.weight));
                    }
                } else {
                    out.push((wind.direction as i32, wind.weight));
                }
            }
        }
        Ok(out)
    }
    fn direction(&self, state: &mut LoadedState, index: i32) -> Result<i32, Error> {
        choose(state, &self.wind_options(state, index, false)?)
    }
    fn next_plot(
        &self,
        state: &mut LoadedState,
        event: i32,
        index: i32,
        budget: &mut i32,
    ) -> Result<i32, Error> {
        let options = self.wind_options(state, index, true)?;
        let next = choose(state, &options)?;
        if next < 0 {
            return Ok(-1);
        }
        let plot = &state.world().map().plots[next as usize];
        let cost = if self.event(event)?.terrains.is_empty()
            || self.event(event)?.terrains.contains(&plot.terrain_hash)
        {
            1
        } else {
            2
        };
        if *budget < cost {
            Ok(-1)
        } else {
            *budget -= cost;
            Ok(next)
        }
    }
    fn name(&self, state: &mut LoadedState, index: i32) -> Result<Vec<u8>, Error> {
        let map = state.world().map();
        let (x, y) = map
            .coordinates(index as u32)
            .ok_or(Error::Missing("storm name plot"))?;
        let distance = |px, py| {
            c6_core::geometry::plot_distance(map.header.width, map.header.wrap_x, x, y, px, py)
        };
        let owner = map.plots[index as usize]
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player));
        let mut nearest = if self.alive_major_ids.contains(&owner) {
            Some(owner)
        } else if owner == -1 {
            None
        } else {
            // IsMajor is not equivalent to IsAlive. This bounded context has
            // proved only the supplied alive-major collection.
            return Err(Error::Requires(
                "storm owned non-alive-major classification",
            ));
        };
        if nearest.is_none() {
            let mut best = i32::MAX;
            for &owner in &self.alive_major_ids {
                let p = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Missing("storm major player"))?;
                for city in &p.cities.records.entries {
                    let h = &city.value.header;
                    let d = distance(h.location_x, h.location_y);
                    if d < best {
                        best = d;
                        nearest = Some(owner);
                    }
                }
            }
            if nearest.is_none() {
                for &owner in &self.alive_major_ids {
                    let p = state
                        .retained()
                        .players
                        .iter()
                        .find(|p| p.header.owner == owner)
                        .ok_or(Error::Missing("storm major starting plot"))?;
                    let d = distance(p.header.starting_plot.x, p.header.starting_plot.y);
                    if d < best {
                        best = d;
                        nearest = Some(owner);
                    }
                }
            }
        }
        let Some(owner) = nearest else {
            return Ok(Vec::new());
        };
        let civ = self
            .civilization_by_player
            .iter()
            .find(|(o, _)| *o == owner)
            .ok_or(Error::Missing("storm current civilization"))?
            .1;
        let names = &self
            .names
            .iter()
            .find(|(h, _)| *h == civ)
            .ok_or(Error::Missing("storm civilization citizen names"))?
            .1;
        let counts = &state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("storm name owner"))?
            .name_manager
            .citizen_name_usage_counts;
        let count = counts.len().min(names.len());
        if names.is_empty() {
            return Ok(Vec::new());
        }
        if count == 0 {
            return Err(Error::Missing("storm citizen usage count"));
        }
        let min = *counts[..count]
            .iter()
            .filter(|&&v| v >= 0)
            .min()
            .ok_or(Error::Requires("storm negative citizen usage"))?;
        let options: Vec<_> = counts[..count]
            .iter()
            .enumerate()
            .filter(|(_, v)| **v == min)
            .map(|(i, _)| (i as i32, 1))
            .collect();
        let selected = choose(state, &options)? as usize;
        let usage = state
            .player_citizen_name_usage_mut(owner)
            .map_err(Error::State)?;
        usage[selected] = usage[selected].wrapping_add(1);
        Ok(names[selected].clone())
    }
}
fn choose(state: &mut LoadedState, options: &[(i32, i32)]) -> Result<i32, Error> {
    if options.is_empty() {
        return Ok(-1);
    }
    let total = options
        .iter()
        .fold(0i32, |sum, (_, w)| sum.wrapping_add(*w));
    let mut value = i32::from(state.game_sync_rand(total as u16).map_err(Error::Random)?);
    let mut selected = -1;
    for &(index, weight) in options {
        selected = index;
        value = value.wrapping_sub(weight);
        if value < 0 {
            break;
        }
    }
    Ok(selected)
}
fn neighbor(state: &LoadedState, index: i32, direction: usize) -> Result<Option<i32>, Error> {
    let map = state.world().map();
    let (x, y) = map
        .coordinates(index as u32)
        .ok_or(Error::Missing("storm neighbor plot"))?;
    const DX: [i32; 6] = [0, 1, 1, 0, -1, -1];
    const DY: [i32; 6] = [1, 0, -1, -1, 0, 1];
    c6_core::found_city::placement::plot_from_delta(
        map,
        PlotCoord { x, y },
        DX[direction],
        DY[direction],
        1,
    )
    .map(|i| i.map(|i| i as i32))
    .map_err(|_| Error::Missing("storm adjacency"))
}
fn supported(event: &Event) -> Result<(), Error> {
    let profile = (event.hash == linux_type_hash(b"RANDOM_EVENT_DUST_STORM_GRADIENT")
        && event.hexes == 3)
        || (event.hash == linux_type_hash(b"RANDOM_EVENT_HURRICANE_CAT_4") && event.hexes == 7);
    if !profile || event.unsupported_damage_options {
        return Err(Error::Requires("positive storm definition/effects"));
    }
    Ok(())
}
fn start_plot(
    state: &mut LoadedState,
    context: &Context,
    event: i32,
    storms: &[c6_core::game_climate::Storm],
) -> Result<i32, Error> {
    let def = context.event(event)?;
    supported(def)?;
    let map = state.world().map();
    let mut options = Vec::new();
    // ChooseStormStartPlot296d840 passes required count1 for Hexes3..18.
    // Thus dust3 and hurricane7 both test only the center terrain.
    for (i, plot) in map.plots.iter().enumerate() {
        if !def.terrains.is_empty() && !def.terrains.contains(&plot.terrain_hash) {
            continue;
        }
        let (x, y) = map
            .coordinates(i as u32)
            .ok_or(Error::Missing("storm target coordinate"))?;
        let mut distance = i32::MAX;
        for storm in storms {
            if let Some(&previous) = storm.turn_end_plots.last() {
                if previous >= 0 {
                    let (px, py) = map
                        .coordinates(previous as u32)
                        .ok_or(Error::Missing("previous storm plot"))?;
                    distance = distance.min(c6_core::geometry::plot_distance(
                        map.header.width,
                        map.header.wrap_x,
                        x,
                        y,
                        px,
                        py,
                    ));
                }
            }
        }
        let score = def
            .spacing
            .wrapping_add(1)
            .wrapping_add(distance.wrapping_sub(def.spacing).min(0));
        if score > 0 {
            options.push((i as i32, score));
        }
    }
    choose(state, &options)
}
fn effects(
    state: &mut LoadedState,
    context: &Context,
    storm: &mut c6_core::game_climate::Storm,
    percent: i32,
    halt: u8,
) -> Result<i32, Error> {
    let def = context.event(storm.random_event_ordinal)?;
    supported(def)?;
    let mut plots = vec![storm.current_plot];
    // ClimateData.GetStormPlots2969450 preserves source direction order.
    let directions: &[usize] = if def.hexes == 7 {
        &[0, 1, 2, 3, 4, 5]
    } else {
        &[0, 5]
    };
    for &d in directions {
        if let Some(p) = neighbor(state, storm.current_plot, d)? {
            plots.push(p);
        }
    }
    let mut fertility = 0i32;
    for index in plots {
        if storm.plots_affected.contains(&index) {
            continue;
        }
        for damage in &def.damages {
            let kind = damage.kind;
            let mut chance = damage.percentage.wrapping_mul(percent) / 100;
            if let Some(override_chance) = damage.coastal_lowland_percentage {
                let terrain = state
                    .world()
                    .map()
                    .components
                    .iter()
                    .find_map(|c| match c {
                        Component::Terrain(t) => Some(t),
                        _ => None,
                    })
                    .ok_or(Error::Missing("storm coastal lowland owner"))?;
                if terrain
                    .coastal_lowlands
                    .iter()
                    .any(|p| p.plot_index == index && p.lowland_type_hash != u32::MAX)
                {
                    // Native override is the raw percentage, including the
                    // final half-strength pass; it is not scaled a second time.
                    chance = override_chance;
                }
            }
            let roll = i32::from(state.game_sync_rand(100).map_err(Error::Random)?);
            if roll < chance {
                let plot = &state.world().map().plots[index as usize];
                let owned = plot.city_attributes.as_ref().is_some_and(|a| a.player >= 0);
                let improvement = kind == linux_type_hash(b"IMPROVEMENT_DESTROYED")
                    || kind == linux_type_hash(b"IMPROVEMENT_PILLAGED");
                let district = kind == linux_type_hash(b"DISTRICT_PILLAGED")
                    || kind == linux_type_hash(b"BUILDING_PILLAGED");
                let naval = kind == linux_type_hash(b"UNIT_DAMAGE_NAVAL");
                if naval {
                    // DealEventDamage -> DamageUnit obtains this exact layer0
                    // vector. Empty means no HP roll, owner or damage callback.
                    if !state
                        .world()
                        .plot_units(0, index as usize)
                        .map_err(|_| Error::Missing("storm current layer0 units"))?
                        .is_empty()
                    {
                        return Err(Error::Requires("storm occupied naval damage effects"));
                    }
                } else if (!improvement && !district)
                    || (owned
                        && ((improvement && plot.improvement_hash != u32::MAX)
                            || (district
                                && plot
                                    .city_attributes
                                    .as_ref()
                                    .is_some_and(|a| a.district != u32::MAX))))
                {
                    return Err(Error::Requires("storm occupied damage effects"));
                }
            }
        }
        let plot = &state.world().map().plots[index as usize];
        if !context.water.contains(&plot.terrain_hash) && !plot.flags.get(PlotFlag::Impassable) {
            if halt != 0 {
                return Err(Error::Requires("storm fertility removal"));
            }
            for &(kind, chance, amount) in &def.yields {
                if i32::from(state.game_sync_rand(100).map_err(Error::Random)?)
                    < chance.wrapping_mul(percent) / 100
                {
                    add_yield(state, index, kind, amount)?;
                    fertility = fertility.wrapping_add(1);
                }
            }
        }
        storm.plots_affected.push(index);
    }
    Ok(fertility)
}
fn add_yield(state: &mut LoadedState, index: i32, kind: u32, amount: i32) -> Result<(), Error> {
    let world = state.world_mut().map_err(Error::State)?;
    let feature = world
        .geographic_components_mut()
        .iter_mut()
        .find_map(|c| {
            if let Component::Feature(f) = c {
                Some(f.as_mut())
            } else {
                None
            }
        })
        .ok_or(Error::Missing("storm feature owner"))?;
    let extra = &mut feature.extra_yields;
    let slot = *extra
        .indices
        .get(index as usize)
        .ok_or(Error::Missing("storm extra yield plot"))?;
    let slot = if slot == -1 {
        let slot = extra.values.len();
        extra.values.push(extra.default.clone());
        extra.indices[index as usize] = slot as i32;
        slot
    } else {
        usize::try_from(slot).map_err(|_| Error::Missing("storm extra yield index"))?
    };
    let values = extra
        .values
        .get_mut(slot)
        .ok_or(Error::Missing("storm extra yield values"))?;
    if let Some(value) = values.iter_mut().find(|v| v.hash == kind) {
        value.value = value.value.wrapping_add(amount)
    } else {
        values.push(DefinitionValue {
            hash: kind,
            value: amount,
        });
    }
    Ok(())
}
/// Called at the native storm application position. History is appended by the
/// RandomEvents owner only after the reached Climate body completes.
pub fn apply(
    state: &mut LoadedState,
    climate: &mut round_climate::Runtime,
    event: PossibleEvent,
    history: &mut c6_core::game_random_events::HistoricalEvent,
) -> Result<(), Error> {
    let context = climate
        .storms
        .as_mut()
        .ok_or(Error::Requires("Climate current storm rules/configuration"))?;
    supported(context.event(event.event)?)?;
    let start = if event.start_plot >= 0 {
        let plot = state
            .world()
            .map()
            .plots
            .get(event.start_plot as usize)
            .ok_or(Error::Missing("storm specified start"))?;
        if context
            .event(event.event)?
            .terrains
            .contains(&plot.terrain_hash)
        {
            event.start_plot
        } else {
            start_plot(state, context, event.event, &climate.manager.storms)?
        }
    } else {
        start_plot(state, context, event.event, &climate.manager.storms)?
    };
    if start < 0 {
        return Ok(());
    }
    let direction = context.direction(state, start)?;
    let name = context.name(state, start)?;
    let mut storm = c6_core::game_climate::Storm {
        wire_version: c6_core::game_climate::WIRE_VERSION,
        random_event_ordinal: event.event,
        severity_ordinal: event.severity,
        active_raw: 1,
        start_turn: state.retained().core.current_turn,
        current_plot: start,
        current_direction_ordinal: direction,
        turn_end_plots: vec![start],
        plots_affected: Vec::new(),
        name_bytes: name,
    };
    let id = climate.manager.storms.len() as i32;
    context.start_plots.push(start);
    climate.live.active_storms.push(id);
    // TriggerEvent inserts a COPY before effects mutate its local stack storm.
    // The stored initial affected cache deliberately remains empty.
    climate.manager.storms.push(storm.clone());
    let result = effects(
        state,
        context,
        &mut storm,
        100,
        climate.live.halt_storm_fertility_raw,
    );
    let fertility = result?;
    let storm = &climate.manager.storms[id as usize];
    history.start_plot = start;
    history.current_plot = start;
    history.current_direction_ordinal = direction;
    history.fertility_added = history.fertility_added.wrapping_add(fertility);
    history.name_bytes = storm.name_bytes.clone();
    Ok(())
}
/// Continues active storms in stable insertion order, including the final
/// half-strength movement/effects before SetStormInactive.
pub fn continue_storms(
    state: &mut LoadedState,
    climate: &mut round_climate::Runtime,
    events: &mut Runtime,
) -> Result<(), Error> {
    if climate.live.active_storms.is_empty() {
        return Ok(());
    }
    let context = climate
        .storms
        .as_ref()
        .ok_or(Error::Requires("Climate current storm rules/configuration"))?;
    for id in climate.live.active_storms.clone() {
        let slot = id as usize;
        let storm = climate
            .manager
            .storms
            .get_mut(slot)
            .ok_or(Error::Missing("active storm identity"))?;
        let def = context.event(storm.random_event_ordinal)?;
        supported(def)?;
        let age = state
            .retained()
            .core
            .current_turn
            .wrapping_add(1)
            .wrapping_sub(storm.start_turn);
        let mut budget = def.movement;
        let percent = if age >= def.duration { 50 } else { 100 };
        let mut fertility = 0i32;
        loop {
            let next = context.next_plot(
                state,
                storm.random_event_ordinal,
                storm.current_plot,
                &mut budget,
            )?;
            if next == -1 {
                break;
            }
            storm.current_plot = next;
            fertility = fertility.wrapping_add(effects(
                state,
                context,
                storm,
                percent,
                climate.live.halt_storm_fertility_raw,
            )?);
        }
        storm.turn_end_plots.push(storm.current_plot);
        storm.current_direction_ordinal = context.direction(state, storm.current_plot)?;
        let start = *context
            .start_plots
            .get(slot)
            .ok_or(Error::Missing("active storm start plot"))?;
        let history = events
            .wire
            .historical_events
            .iter_mut()
            .find(|h| {
                h.start_turn == storm.start_turn
                    && h.random_event_hash == def.hash
                    && h.start_plot == start
            })
            .ok_or(Error::Missing("active storm historical event"))?;
        history.current_plot = storm.current_plot;
        history.current_direction_ordinal = storm.current_direction_ordinal;
        history.fertility_added = history.fertility_added.wrapping_add(fertility);
        if age >= def.duration {
            storm.active_raw = 0;
            climate.live.active_storms.retain(|v| *v != id);
        }
    }
    Ok(())
}
