//! Linux XP2 River.TriggerEvent (2de7e60), bounded unowned floodplains and
//! nonlethal land-combat unit damage. The canonical Map owns flood history and yields. Damage
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
    damages: Vec<Damage>,
    yields: Vec<(u32, u32, i32)>,
    unsupported: bool,
}
struct Damage {
    kind: u32,
    chance: i32,
    min_hp: i32,
    max_hp: i32,
}
struct UnitDefinition {
    hash: u32,
    civilian: bool,
    hero: bool,
}
pub(super) struct Definitions {
    events: Vec<Event>,
    units: Vec<UnitDefinition>,
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
                damages.push(Damage {
                    kind: hash(row, "DamageType")?,
                    chance: integer(row, "Percentage")?,
                    min_hp: integer(row, "MinHP")?,
                    max_hp: integer(row, "MaxHP")?,
                });
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
        let heroes: Vec<_> = table("HeroClasses")?
            .rows()
            .map(|r| hash(r, "UnitType"))
            .collect::<Result<_, _>>()?;
        let units = table("Units")?
            .rows()
            .map(|r| {
                let unit_hash = hash(r, "UnitType")?;
                Ok(UnitDefinition {
                    hash: unit_hash,
                    civilian: r.text("FormationClass").map_err(Error::Rules)?
                        == "FORMATION_CLASS_CIVILIAN",
                    hero: heroes.contains(&unit_hash),
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self { events, units })
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

fn damage_land_units<P>(
    state: &mut LoadedState,
    definitions: &Definitions,
    event: i32,
    damage: &Damage,
    plot: i32,
    backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
) -> Result<(), Error> {
    // DamageUnit2a116e0 takes a copy of this current layer0 vector. Manager
    // lookup may legitimately return null for a stale reference; it is skipped.
    let units = state
        .world()
        .plot_units(0, plot as usize)
        .map_err(|_| Error::Missing("river damage layer0 units"))?
        .to_vec();
    for reference in units {
        let Some(found) = state
            .world()
            .unit(reference)
            .map_err(|_| Error::Missing("river damage unit lookup"))?
        else {
            continue;
        };
        let unit = found.record;
        if unit.b_delayed_death {
            continue;
        }
        let definition = definitions
            .units
            .iter()
            .find(|d| d.hash == unit.e_type_index)
            .ok_or(Error::Missing("river damage unit definition"))?;
        if definition.civilian
            || state
                .unit_runtime(reference.player(), reference.id)
                .map_err(Error::State)?
                .domain
                // DealEventDamage2a100ad passes native DomainTypes2 for
                // UNIT_DAMAGE_LAND; naval damage passes0 at2a10128.
                != 2
        {
            continue;
        }
        let owner = unit.e_owner;
        let disaster_charges = unit
            .i_disaster_charges
            .ok_or(Error::Missing("river current disaster charges"))?;
        let hero = definition.hero;
        // The HP draw precedes owner immunity, disaster-charge and hero checks,
        // and uses MaxHP-MinHP (exclusive), truncated to native uint16.
        let mut delta = i32::from(
            state
                .game_sync_rand(damage.max_hp.wrapping_sub(damage.min_hp) as u16)
                .map_err(Error::Random)?,
        )
        .wrapping_add(damage.min_hp);
        if owner == -1 {
            return Err(Error::Requires("river ownerless unit damage"));
        }
        let tail = &state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("river damage unit owner"))?
            .tail
            .as_ref()
            .ok_or(Error::Missing("river damage player tail"))?;
        // GetRandomEventNoUnitDamage2b32560 returns false outside the retained
        // indexed list; event is the definition's native append ordinal.
        if tail
            .random_event_no_unit_damage
            .get(event as usize)
            .copied()
            .unwrap_or(0)
            > 0
            || disaster_charges > 0
            || hero
        {
            delta = 0;
        }
        let actor = (reference.player(), reference.id);
        let current = state
            .world()
            .unit(reference)
            .map_err(|_| Error::Missing("river current damaged unit"))?
            .ok_or(Error::Missing("river current damaged unit"))?
            .record;
        let maximum = backend
            .river_flood_maximum_damage(state, actor)
            .map_err(Error::RiverUnitDamage)?;
        if current.i_damage.wrapping_add(delta) >= maximum {
            return Err(Error::Requires("river lethal unit damage continuation"));
        }
        backend
            .river_flood_change_damage(state, actor, delta)
            .map_err(Error::RiverUnitDamage)?;
        let current = state
            .world()
            .unit(reference)
            .map_err(|_| Error::Missing("river damaged unit after report"))?
            .ok_or(Error::Missing("river damaged unit after report"))?
            .record;
        if current.i_damage
            >= backend
                .river_flood_maximum_damage(state, actor)
                .map_err(Error::RiverUnitDamage)?
        {
            return Err(Error::Requires("river lethal unit damage after report"));
        }
        // Manager::ChangeDamage is followed by an explicit dead check, then
        // Operations.Clear and SetActivity(AWAKE) on every surviving unit.
        backend
            .river_flood_wake_unit(state, actor)
            .map_err(Error::RiverUnitDamage)?;
    }
    Ok(())
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
    if definition.unsupported || definition.damages.iter().any(|d| !supported_damage(d.kind)) {
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
    // Check current occupancy at event time, after all preceding rounds. Empty
    // initial fixture cells do not establish the state at the disaster boundary.
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
        for &reference in state
            .world()
            .plot_units(0, index as usize)
            .map_err(|_| Error::Missing("river layer0 units"))?
        {
            let unit = state
                .world()
                .unit(reference)
                .map_err(|_| Error::Missing("river current unit resolution"))?;
            let Some(unit) = unit else { continue };
            if unit.record.b_delayed_death {
                continue;
            }
            let unit_definition = definitions
                .units
                .iter()
                .find(|d| d.hash == unit.record.e_type_index)
                .ok_or(Error::Missing("river current unit definition"))?;
            if !unit_definition.civilian {
                continue;
            }
            return Err(Error::RiverOccupied {
                plot: index,
                reference,
                resolved_id: Some(unit.resolved_id),
                location: Some(unit.record.k_location),
                unit_type: Some(unit.record.e_type_index),
                damage: Some(unit.record.i_damage),
                delayed_death: Some(unit.record.b_delayed_death),
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
    for damage in &definition.damages {
        for &plot in &plots {
            let roll = i32::from(state.game_sync_rand(100).map_err(Error::Random)?);
            if roll < damage.chance && damage.kind == linux_type_hash(b"UNIT_DAMAGE_LAND") {
                damage_land_units(state, definitions, event.event, damage, plot, backend)?;
            }
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
