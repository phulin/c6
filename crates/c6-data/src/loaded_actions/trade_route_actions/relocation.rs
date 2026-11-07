//! Trade.Manager.RelocateTrader2c07800 using current gameplay owners.
use crate::{loaded_upkeep, unit_placement_state as placement, unit_sight_state};
use c6_core::{loaded_state::LoadedState, trade::ComponentId, unit::PlotCoord};

/// Active routes are queried after both war-pair removals. City roster order
/// breaks distance ties. Placement and operation callbacks share state and RNG.
#[allow(clippy::too_many_arguments)]
pub fn relocate<E>(
    state: &mut LoadedState,
    definitions: &placement::Definitions<'_, '_>,
    queries: &placement::Queries<'_>,
    sight: &mut unit_sight_state::Runtime,
    movement: &crate::unit_movement_state::Inputs<'_>,
    effects: &mut E,
    trader: ComponentId,
    preferred: PlotCoord,
) -> Result<(), String>
where
    E: placement::Effects + loaded_upkeep::Effects<Error = placement::EffectsError<E>>,
    placement::EffectsError<E>: std::fmt::Debug,
{
    let actor = (i32::from(trader.owner), trader.id);
    let reference = c6_core::map::UnitReference::from_component_id(c6_core::unit::ComponentId {
        owner: trader.owner,
        component_type: trader.component_type,
        id: trader.id,
    });
    let Some(unit) = state
        .world()
        .unit(reference)
        .map_err(|e| format!("{e:?}"))?
    else {
        return Ok(());
    };
    let unit = unit.record;
    let current = unit.k_location;
    if !c6_core::skip_turn::placement::coordinate_valid(current)
        || unit.b_delayed_death
        || unit.i_damage
            >= definitions
                .movement
                .maximum_damage(unit)
                .map_err(|e| format!("{e:?}"))?
    {
        return Ok(());
    }
    if state
        .trade_graph()
        .map_err(|e| format!("{e:?}"))?
        .graph()
        .origins()
        .iter()
        .flat_map(|o| &o.destinations)
        .any(|d| d.edge.has_active_route && d.edge.most_recent_route.trader_unit == trader)
    {
        return Ok(());
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == actor.0)
        .ok_or("Trade relocation current city roster")?;
    let map = &state.world().map().header;
    let mut distance = i32::MAX;
    let mut selected = None;
    for city in &player.cities.records.entries {
        if city.value.header.delayed_death {
            continue;
        }
        let at = PlotCoord {
            x: city.value.header.location_x,
            y: city.value.header.location_y,
        };
        if c6_core::skip_turn::placement::coordinate_valid(preferred)
            && preferred != current
            && preferred == at
        {
            selected = Some(at);
            break;
        }
        let candidate = c6_core::geometry::plot_distance(
            map.width, map.wrap_x, current.x, current.y, at.x, at.y,
        );
        if candidate < distance {
            distance = candidate;
            selected = Some(at);
        }
    }
    let Some(destination) = selected.filter(|at| *at != current) else {
        return Ok(());
    };
    placement::set_relocation(
        state,
        definitions,
        queries,
        sight,
        effects,
        actor,
        destination,
    )
    .map_err(|e| format!("Trade relocation Placement: {e:?}"))?;
    loaded_upkeep::clear_removed_trade_operations(state, actor.0, actor.1, movement, effects)
        .map_err(|e| format!("Trade relocation Clear: {e:?}"))?;
    loaded_upkeep::finish_unit_moves(state, actor.0, actor.1, movement, effects)
        .map_err(|e| format!("Trade relocation FinishMoves: {e:?}"))
}
