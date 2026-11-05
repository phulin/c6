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
    if !targets.knowledge_base.known_cities.is_empty()
        || !targets.knowledge_base.barbarians.is_empty()
        || targets.knowledge_base.player_armies.0.iter().any(|army| {
            army.army_size
                .value
                .as_ref()
                .is_some_and(|types| types.iter().any(|r| !r.value.is_empty()))
        })
    {
        return Err(Error::Unsupported(
            "current historical knowledge aging continuation",
        ));
    }
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
        if plot.city_attributes.as_ref().is_some_and(|c| {
            (c.player >= 0 && c.district != u32::MAX) || c.wonder_building_hash != u32::MAX
        }) {
            let revealed = match state
                .world()
                .visibility(observer)
                .ok_or(Error::Unsupported("current Targets revealed owner"))?
                .revealed
                .get(index)
            {
                Some(0) => false,
                Some(1) => true,
                _ => {
                    return Err(Error::Unsupported(
                        "current Targets canonical revealed byte",
                    ))
                }
            };
            if visible || revealed {
                return Err(Error::Unsupported(
                    "current Targets City observation continuation",
                ));
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
