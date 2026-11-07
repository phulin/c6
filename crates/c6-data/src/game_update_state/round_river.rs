//! Linux XP2 River.TriggerEvent (2de7e60), bounded unowned, unoccupied
//! floodplains. The canonical Map owns flood history and added yields. Damage
//! selection still consumes every reached GameSync draw when targets are empty.
use super::{
    round_climate,
    round_random_events::{Error, PossibleEvent, Runtime},
};
use crate::{
    registry::linux_type_hash, unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    game_random_events::HistoricalEvent,
    loaded_state::LoadedState,
    map::{Component, Flood},
};

struct Event {
    hash: u32,
    severity: i32,
    damages: Vec<(u32, i32)>,
    yields: Vec<(u32, u32, i32)>,
    unsupported: bool,
}
pub(super) struct Definitions {
    events: Vec<Event>,
}
impl Definitions {
    pub(super) fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Missing(name));
        let integer = |row: crate::Row<'_>, column| {
            i32::try_from(row.integer(column).map_err(Error::Rules)?)
                .map_err(|_| Error::Missing(column))
        };
        let hash = |row: crate::Row<'_>, column| {
            row.text(column)
                .map(|v| linux_type_hash(v.as_bytes()))
                .map_err(Error::Rules)
        };
        let mut events = Vec::new();
        for entry in collection(rules, "RandomEvents", "RandomEventType")
            .map_err(Error::Definitions)?
            .entries()
        {
            let row = table("RandomEvents")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("river event row"))?;
            let name = row.text("RandomEventType").map_err(Error::Rules)?;
            if row.text("EffectOperatorType").ok() != Some("FLOODPLAIN") {
                events.push(Event {
                    hash: linux_type_hash(entry.type_name.as_slice()),
                    severity: integer(row, "Severity")?,
                    damages: Vec::new(),
                    yields: Vec::new(),
                    unsupported: true,
                });
                continue;
            }
            let mut damages = Vec::new();
            let mut yields = Vec::new();
            let mut unsupported = false;
            let mut damage_rows: Vec<_> = table("RandomEvent_Damages")?
                .rows()
                .filter(|r| r.text("RandomEventType").ok() == Some(name))
                .collect();
            damage_rows.sort_by_key(|r| r.row_id());
            for row in damage_rows {
                unsupported |= ["MinTurn", "MaxTurn", "CoastalLowlandPercentage"]
                    .iter()
                    .any(|c| !matches!(row.get(c), Ok(SqlValue::Null)));
                damages.push((hash(row, "DamageType")?, integer(row, "Percentage")?));
            }
            let mut yield_rows: Vec<_> = table("RandomEvent_Yields")?
                .rows()
                .filter(|r| r.text("RandomEventType").ok() == Some(name))
                .collect();
            yield_rows.sort_by_key(|r| r.row_id());
            for row in yield_rows {
                yields.push((
                    hash(row, "YieldType")?,
                    hash(row, "FeatureType")?,
                    integer(row, "Percentage")?,
                ));
            }
            events.push(Event {
                hash: linux_type_hash(entry.type_name.as_slice()),
                severity: integer(row, "Severity")?,
                damages,
                yields,
                unsupported,
            });
        }
        Ok(Self { events })
    }
}

fn supported_damage(kind: u32) -> bool {
    [
        b"IMPROVEMENT_DESTROYED".as_slice(),
        b"IMPROVEMENT_PILLAGED",
        b"DISTRICT_PILLAGED",
        b"BUILDING_PILLAGED",
        b"POPULATION_LOSS",
        b"UNIT_KILLED_CIVILIAN",
        b"UNIT_DAMAGE_LAND",
        b"CITY_GARRISON",
        b"CITY_WALLS",
    ]
    .iter()
    .any(|name| linux_type_hash(name) == kind)
}

pub(super) fn apply<P>(
    state: &mut LoadedState,
    climate: &round_climate::Runtime,
    runtime: &Runtime,
    definitions: &Definitions,
    event: PossibleEvent,
    history: &mut HistoricalEvent,
    backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
) -> Result<(), Error> {
    let definition = definitions
        .events
        .get(event.event as usize)
        .ok_or(Error::Missing("river event definition"))?;
    if definition.unsupported
        || definition
            .damages
            .iter()
            .any(|(k, _)| !supported_damage(*k))
    {
        return Err(Error::Requires("river damage definition branch"));
    }
    let rivers = state
        .world()
        .map()
        .components
        .iter()
        .find_map(|c| match c {
            Component::River(r) => Some(&r.rivers),
            _ => None,
        })
        .ok_or(Error::Missing("river canonical owner"))?;
    let mut matching = rivers
        .iter()
        .filter(|r| r.name_hash == history.named_river_hash);
    let river = matching
        .next()
        .ok_or(Error::Missing("selected named river"))?;
    if matching.next().is_some() {
        return Err(Error::Requires("river duplicate named instances"));
    }
    let plots = river.floodplains.clone();
    let first = *plots
        .first()
        .ok_or(Error::Missing("river floodplain location"))?;
    // Resolve the current localized display text separately from gameplay
    // definition identity. Native custom names override localization lookup.
    let name = if !river.custom_name.is_empty() {
        river.custom_name.clone()
    } else {
        runtime
            .river_names
            .as_ref()
            .and_then(|names| names.iter().find(|(hash, _)| *hash == river.name_hash))
            .map(|(_, bytes)| bytes.clone())
            .ok_or(Error::Missing("current river localized name"))?
    };
    // Empty owned/city/unit branches are established from the actual current
    // world on every application; no river identity or fixture seed is special.
    for &index in &plots {
        let plot = state
            .world()
            .map()
            .plots
            .get(index as usize)
            .ok_or(Error::Missing("river floodplain plot"))?;
        if plot.city_attributes.is_some() || plot.improvement_hash != u32::MAX {
            return Err(Error::Requires("river owned floodplain effects"));
        }
        if let Some(&reference) = state
            .world()
            .plot_units(0, index as usize)
            .map_err(|_| Error::Missing("river layer0 units"))?
            .first()
        {
            let unit = state
                .world()
                .unit(reference)
                .map_err(|_| Error::Missing("river current unit resolution"))?;
            return Err(Error::RiverOccupied {
                plot: index,
                reference,
                resolved_id: unit.as_ref().map(|u| u.resolved_id),
                location: unit.as_ref().map(|u| u.record.k_location),
                unit_type: unit.as_ref().map(|u| u.record.e_type_index),
                damage: unit.as_ref().map(|u| u.record.i_damage),
                delayed_death: unit.as_ref().map(|u| u.record.b_delayed_death),
            });
        }
    }
    let turn = state.retained().core.current_turn;
    let world = state.world_mut().map_err(Error::State)?;
    let rivers = world
        .geographic_components_mut()
        .iter_mut()
        .find_map(|c| match c {
            Component::River(r) => Some(r),
            _ => None,
        })
        .ok_or(Error::Missing("river canonical owner"))?;
    // MarkFlooded visits all matching named instances in stored order.
    for river in rivers
        .rivers
        .iter_mut()
        .filter(|r| r.name_hash == history.named_river_hash)
    {
        river.floods.push(Flood {
            turn,
            duration: 1,
            severity: definition.severity,
            mitigated: false,
        });
    }
    for &plot in &plots {
        backend.river_flood_report(
            state,
            0xfc5eac6e,
            0x1c8fa8d7,
            plot,
            definition.severity,
            false,
        )?;
    }
    backend.river_flood_report(
        state,
        0xe869c8ff,
        definition.hash,
        first,
        definition.severity,
        false,
    )?;
    // FloodRiver loops damage rows before plots. AvoidsRandomEvent returns
    // false for these unowned plots; each outer chance is nevertheless drawn.
    for &(_, _chance) in &definition.damages {
        for &_plot in &plots {
            state.game_sync_rand(100).map_err(Error::Random)?;
        }
    }
    // The zero damage array still dispatches the actual Game signal consumers.
    backend.river_flood_damage_signals(state, definition.hash)?;
    if climate.live.halt_flood_fertility_raw == 0 {
        // SpreadYields draws BEFORE checking a plot's current feature, including
        // nonmatching floodplain variants. Native adds exactly one per success.
        for &(kind, feature, chance) in &definition.yields {
            for &plot in &plots {
                let roll = i32::from(state.game_sync_rand(100).map_err(Error::Random)?);
                if roll < chance && state.world().map().plots[plot as usize].feature_hash == feature
                {
                    super::round_storm::add_yield(state, plot, kind, 1)?;
                    history.fertility_added = history.fertility_added.wrapping_add(1);
                }
            }
        }
    }
    history.start_plot = first;
    history.current_plot = first;
    history.current_direction_ordinal = -1;
    history.name_bytes = name;
    Ok(())
}
