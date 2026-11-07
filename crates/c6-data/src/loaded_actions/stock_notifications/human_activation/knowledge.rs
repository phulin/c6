//! Gameplay-consumed Unit observations at the stock Targets activation boundary.
//! Planner target lists and exploration scores are separate outputs. Existing
//! historical observations require their aging continuation, never a reset.
use crate::{registry::LinuxDefinitionRegistry, unit_sight_state};
use c6_core::{game_update::Error, loaded_state::LoadedState, visibility_change::BASE_LAYER_HASH};

pub fn observe_visible_units(
    state: &mut LoadedState,
    observer: i32,
    units: &LinuxDefinitionRegistry,
    visibility: &unit_sight_state::Runtime,
    combat: Option<&crate::formation_rules::Definitions>,
) -> Result<(), Error> {
    let targets = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == observer)
        .and_then(|p| p.ai_targets.as_ref())
        .ok_or(Error::Unsupported("current Targets knowledge owner"))?;
    let normal = visibility
        .shared(observer)
        .map_err(Error::Unsupported)?
        .0
        .iter()
        .find(|l| l.type_hash == BASE_LAYER_HASH)
        .ok_or(Error::Unsupported("current knowledge normal visibility"))?;
    let map = state.world().map();
    let width = map.header.width as usize;
    if width == 0 {
        return Err(Error::Unsupported("current knowledge map width"));
    }
    let seen = |location: (i32, i32)| -> Result<Option<usize>, Error> {
        let (Ok(x), Ok(y)) = (usize::try_from(location.0), usize::try_from(location.1)) else {
            return Ok(None);
        };
        if x >= width || y >= map.header.height as usize {
            return Ok(None);
        }
        let index = y * width + x;
        let visible = *normal
            .values
            .get(y * normal.grid_width as usize + x)
            .ok_or(Error::Unsupported("current knowledge visibility extent"))?
            > 0;
        let revealed = *state
            .world()
            .visibility(observer)
            .ok_or(Error::Unsupported("current knowledge revealed owner"))?
            .revealed
            .get(index)
            .ok_or(Error::Unsupported("current knowledge revealed extent"))?;
        if revealed > 1 {
            return Err(Error::Unsupported("current knowledge revealed byte"));
        }
        Ok((visible || revealed == 1).then_some(index))
    };
    let mut remove_camps = Vec::new();
    for camp in &targets.knowledge_base.barbarians {
        remove_camps.push(
            seen((camp.camp_location.x, camp.camp_location.y))?.is_some_and(|i| {
                map.plots[i].improvement_hash != state.retained().barbarian.improvement_hash
            }),
        );
    }
    let mut remove_cities = Vec::new();
    for city in &targets.knowledge_base.known_cities {
        remove_cities.push(seen((city.location.x, city.location.y))?.is_some_and(|i| {
            map.plots[i]
                .city_attributes
                .as_ref()
                .map(|a| i32::from(a.player))
                != Some(city.owner)
        }));
    }
    // Runtime-only known-city identities retain the same observation gate.
    // A disappeared city needs its historical location before it can be pruned.
    let mut remove_identities = Vec::new();
    for &(owner, id) in &targets.observed_city_identities {
        let city = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .and_then(|p| {
                p.cities
                    .records
                    .entries
                    .iter()
                    .find(|c| c.value.header.id == id)
            })
            .ok_or(Error::Unsupported(
                "historical observed city location after destruction",
            ))?;
        remove_identities.push(
            seen((city.value.header.location_x, city.value.header.location_y))?.is_some_and(|i| {
                map.plots[i]
                    .city_attributes
                    .as_ref()
                    .map(|a| i32::from(a.player))
                    != Some(owner)
            }),
        );
    }
    // IncrementTurn20009f0..2000a24 checks current relations, independently
    // of the historical is_active byte. Only scores for wars now at peace age.
    let decay_wars = targets
        .knowledge_base
        .war_knowledge
        .iter()
        .map(|war| {
            if war.opponent == -1 {
                return Ok(false);
            }
            state
                .game_relations()
                .map_err(|_| Error::Unsupported("current knowledge relations"))?
                .is_at_war(observer, war.opponent)
                .map(|at_war| !at_war)
                .map_err(|_| Error::Unsupported("current knowledge war relation"))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let targets = state
        .combat_ai_targets_mut(observer)
        .map_err(|_| Error::Unsupported("current knowledge owner"))?;
    for army in &mut targets.knowledge_base.player_armies.0 {
        if let Some(types) = &mut army.army_size.value {
            for kind in types {
                kind.value.retain_mut(|unit| {
                    let old = unit.turns_since_observation;
                    unit.turns_since_observation = old.wrapping_add(1);
                    old < 21
                });
            }
        }
    }
    for (war, decay) in targets
        .knowledge_base
        .war_knowledge
        .iter_mut()
        .zip(decay_wars)
    {
        if decay {
            war.current_tally_q8 = decay_war_tally(war.current_tally_q8);
        }
    }
    let mut index = 0;
    targets.knowledge_base.barbarians.retain(|_| {
        let keep = !remove_camps[index];
        index += 1;
        keep
    });
    let mut index = 0;
    targets.knowledge_base.known_cities.retain_mut(|city| {
        city.turns_since_observation = city.turns_since_observation.wrapping_add(1);
        let keep = !remove_cities[index];
        index += 1;
        keep
    });
    let mut index = 0;
    targets.observed_city_identities.retain(|_| {
        let keep = !remove_identities[index];
        index += 1;
        keep
    });
    refresh_visible_units(state, observer, units, visibility, combat)
}

/// UpdateKnowledgeBase at the final control-priority signal does not age or
/// reset history. It replaces/adds observations using the current visible map.
pub fn refresh_visible_units(
    state: &mut LoadedState,
    observer: i32,
    units: &LinuxDefinitionRegistry,
    visibility: &unit_sight_state::Runtime,
    combat: Option<&crate::formation_rules::Definitions>,
) -> Result<(), Error> {
    let layers = visibility.shared(observer).map_err(Error::Unsupported)?;
    let normal = layers
        .0
        .iter()
        .find(|l| l.type_hash == BASE_LAYER_HASH)
        .ok_or(Error::Unsupported(
            "current Targets shared normal visibility",
        ))?;
    let map = state.world().map();
    let width = map.header.width as usize;
    if width == 0 {
        return Err(Error::Unsupported("current Targets Map width"));
    }
    enum Observation {
        City(i32, u32),
        Unit(i32, u32),
        Camp(c6_core::unit::PlotCoord),
    }
    let mut observed = Vec::new();
    let camp_hash = state.retained().barbarian.improvement_hash;
    for (index, plot) in map.plots.iter().enumerate() {
        let cell = (index / width)
            .checked_mul(normal.grid_width as usize)
            .and_then(|v| v.checked_add(index % width))
            .ok_or(Error::Unsupported("current Targets visibility index"))?;
        let visible = *normal
            .values
            .get(cell)
            .ok_or(Error::Unsupported("current Targets visibility extent"))?
            > 0;
        if let Some(attributes) = plot
            .city_attributes
            .as_ref()
            .filter(|a| a.player >= 0 && a.district != u32::MAX)
        {
            let revealed = *state
                .world()
                .visibility(observer)
                .ok_or(Error::Unsupported("current Targets revealed owner"))?
                .revealed
                .get(index)
                .ok_or(Error::Unsupported("current Targets revealed plot"))?;
            if revealed > 1 {
                return Err(Error::Unsupported("current Targets revealed byte"));
            }
            if visible || revealed == 1 {
                let owner = i32::from(attributes.player);
                state
                    .world()
                    .player(owner)
                    .map_err(|_| Error::Unsupported("current observed City owner"))?;
                let player = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Unsupported("current observed City manager"))?;
                // The same native district low-slot Find used by current plot
                // queries resolves the owning City before UpdateCity2001030.
                if let Some(district) = player
                    .districts
                    .records
                    .entries
                    .iter()
                    .find(|d| d.index as u32 == attributes.district & 0xffff)
                {
                    if let Some(city) = player
                        .cities
                        .records
                        .entries
                        .iter()
                        .find(|c| c.value.header.id == district.value.city_id)
                    {
                        observed.push(Observation::City(owner, city.value.header.id));
                    }
                }
            }
        }
        if !visible {
            // 21aaae0..21aab27: revealed plots without a district still
            // refresh actual camp knowledge; ordinary improvements do nothing.
            let revealed = state
                .world()
                .visibility(observer)
                .ok_or(Error::Unsupported("current Targets revealed owner"))?
                .revealed
                .get(index)
                .ok_or(Error::Unsupported("current Targets revealed plot"))?;
            match revealed {
                0 => {}
                1 if plot.improvement_hash == camp_hash => {
                    observed.push(Observation::Camp(c6_core::unit::PlotCoord {
                        x: (index % width) as i32,
                        y: (index / width) as i32,
                    }))
                }
                1 => {}
                _ => {
                    return Err(Error::Unsupported(
                        "current Targets canonical revealed byte",
                    ))
                }
            }
            continue;
        }
        for reference in state
            .world()
            .plot_units(0, index)
            .map_err(|_| Error::Unsupported("current Targets Unit layer"))?
        {
            let Some(unit) = state
                .world()
                .unit(*reference)
                .map_err(|_| Error::Unsupported("current Targets Unit identity"))?
            else {
                continue;
            };
            if unit.record.b_has_hidden_visibility {
                return Err(Error::Unsupported(
                    "current Targets hidden Unit observation",
                ));
            }
            observed.push(Observation::Unit(unit.record.e_owner, unit.record.id));
        }
        if plot.improvement_hash == camp_hash {
            observed.push(Observation::Camp(c6_core::unit::PlotCoord {
                x: (index % width) as i32,
                y: (index / width) as i32,
            }));
        }
    }
    // Plot order, then current layer member order, is the native observation
    // order. Never enumerate a player roster or sort by Unit ID instead.
    for observation in observed {
        match observation {
            Observation::City(owner, id) => {
                // Known-city presence gates automatic agenda effects as well
                // as policy scores. Retain that identity. The RNG-free copied
                // snapshot scalars are not authoritative gameplay inputs;
                // reached effects derive them from current canonical City.
                state
                    .combat_ai_targets_mut(observer)
                    .map_err(|_| Error::Unsupported("current City knowledge owner"))?
                    .observe_city_identity(owner, id);
            }
            Observation::Unit(owner, id) => {
                crate::unit_creation::production_effects::observe_current_unit::<
                    std::convert::Infallible,
                >(state, units, observer, (owner, id))
                .map_err(|_| Error::Unsupported("current Targets UnitInfo update"))?
            }
            Observation::Camp(location) => observe_camp(state, observer, location, combat)?,
        }
    }
    Ok(())
}

/// UpdateBarbarianCamp20012b0 and UpdateCamp1fff4c0. This remains gameplay
/// knowledge: PlayerCivilized::Test237ca00 counts these locations near cities.
/// GetBestDefender's flags1, NO_PLAYER viewer and null attacker select exactly
/// the existing default profile: no war/visibility query, callback or RNG.
fn observe_camp(
    state: &mut LoadedState,
    observer: i32,
    location: c6_core::unit::PlotCoord,
    combat: Option<&crate::formation_rules::Definitions>,
) -> Result<(), Error> {
    let camps = &mut state
        .combat_ai_targets_mut(observer)
        .map_err(|_| Error::Unsupported("current camp knowledge owner"))?
        .knowledge_base
        .barbarians;
    let index = match camps
        .iter()
        .position(|v| v.camp_location.x == location.x && v.camp_location.y == location.y)
    {
        Some(index) => index,
        None => {
            camps.push(c6_core::player_ai_targets::BarbarianKnowledge {
                camp_location: c6_core::player_ai_targets::PlotCoord {
                    x: location.x,
                    y: location.y,
                },
                turns_since_observation: 0,
                guard_hash: u32::MAX,
            });
            camps.len() - 1
        }
    };
    camps[index].turns_since_observation = 0;
    let combat = combat.ok_or(Error::Unsupported("current camp guard combat definitions"))?;
    let guard = crate::combat_defender::get_best_defender(state, combat, location)
        .map_err(|_| Error::Unsupported("current camp guard defender query"))?
        .map_or(u32::MAX, |v| v.e_type_index);
    state
        .combat_ai_targets_mut(observer)
        .map_err(|_| Error::Unsupported("current camp knowledge guard owner"))?
        .knowledge_base
        .barbarians[index]
        .guard_hash = guard;
    Ok(())
}

/// WarKnowledge::DecayScore1fff5b0 changes only the tally. FixedPoint Abs
/// wraps at MIN, and IsPositive includes zero. There are no callbacks or RNG.
fn decay_war_tally(tally: i32) -> i32 {
    let step = if tally.wrapping_abs() > 1000 * 256 {
        100 * 256
    } else {
        10 * 256
    };
    if tally >= 0 {
        tally.wrapping_sub(step).max(0)
    } else {
        tally.wrapping_add(step).min(0)
    }
}

#[cfg(test)]
mod tests {
    use super::decay_war_tally;

    #[test]
    fn war_decay_clamps_and_uses_strict_threshold_in_raw_q8() {
        for (input, expected) in [
            (0, 0),
            (1, 0),
            (-1, 0),
            (10 * 256, 0),
            (-10 * 256, 0),
            (1000 * 256, 990 * 256),
            (1000 * 256 + 1, 900 * 256 + 1),
            (-1000 * 256, -990 * 256),
            (-1000 * 256 - 1, -900 * 256 - 1),
            (i32::MIN, i32::MIN + 10 * 256),
            (i32::MAX, i32::MAX - 100 * 256),
        ] {
            assert_eq!(decay_war_tally(input), expected, "input {input}");
        }
    }
}
