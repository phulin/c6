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
    if state
        .retained()
        .players
        .iter()
        .any(|p| !p.cities.records.entries.is_empty())
    {
        return Err(Error::Unsupported(
            "current Targets City observation continuation",
        ));
    }
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
    let mut observed = Vec::new();
    for (index, plot) in map.plots.iter().enumerate() {
        let cell = (index / width)
            .checked_mul(normal.grid_width as usize)
            .and_then(|v| v.checked_add(index % width))
            .ok_or(Error::Unsupported("current Targets visibility index"))?;
        if *normal
            .values
            .get(cell)
            .ok_or(Error::Unsupported("current Targets visibility extent"))?
            <= 0
        {
            continue;
        }
        if plot.improvement_hash != u32::MAX {
            return Err(Error::Unsupported(
                "current Targets improvement observation continuation",
            ));
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
            observed.push((unit.record.e_owner, unit.record.id));
        }
    }
    // Plot order, then current layer member order, is the native observation
    // order. Never enumerate a player roster or sort by Unit ID instead.
    for unit in observed {
        crate::unit_creation::production_effects::observe_current_unit::<std::convert::Infallible>(
            state, units, observer, unit,
        )
        .map_err(|_| Error::Unsupported("current Targets UnitInfo update"))?;
    }
    Ok(())
}
