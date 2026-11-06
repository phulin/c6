//! Source branch leaves for units outside enemy borders and no shared sight.
use c6_core::{loaded_state::LoadedState, map::OutgoingVisibility};
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Requires(&'static str),
}
/// ExpelFromTerritory2aaa300(reason3) visits the current unit collection.
/// GetPlot()==null and plots owned by anyone else reach no Placement call.
pub fn expel_outside_borders(state: &LoadedState, owner: i32, other: i32) -> Result<(), Error> {
    let player = state.world().player(owner).map_err(Error::World)?;
    for unit in player.unit_records().map_err(Error::World)? {
        let at = unit.value.k_location;
        let Some(plot) = state.world().map().plot_at(at.x, at.y) else {
            continue;
        };
        if plot
            .city_attributes
            .as_ref()
            .is_some_and(|a| i32::from(a.player) == other)
        {
            return Err(Error::Requires("expulsion Placement.IsValidLocation/Set"));
        }
    }
    Ok(())
}
/// AddOutgoingVisibilityBlocker2bc4950 reaches layer propagation only for a
/// newly inserted blocker AND an existing outgoing sight edge. Repeated adds
/// still increment their own native wrapping counter.
pub fn block_unshared_visibility(
    state: &mut LoadedState,
    source: i32,
    recipient: i32,
) -> Result<(), Error> {
    state
        .world()
        .visibility(recipient)
        .ok_or(Error::Requires("current recipient visibility validity"))?;
    let visibility = state
        .world()
        .visibility(source)
        .ok_or(Error::Requires("current source visibility validity"))?;
    let exists = visibility
        .outgoing_visibility_blockers
        .iter()
        .any(|v| v.player_id == recipient);
    if !exists
        && visibility
            .outgoing_visibility
            .iter()
            .any(|v| v.player_id == recipient && v.count != 0)
    {
        return Err(Error::Requires(
            "shared visibility layer removal/propagation",
        ));
    }
    let visibility = state
        .world_mut()
        .map_err(Error::State)?
        .visibility_mut(source)
        .expect("checked source visibility");
    if let Some(cell) = visibility
        .outgoing_visibility_blockers
        .iter_mut()
        .find(|v| v.player_id == recipient)
    {
        cell.count = cell.count.wrapping_add(1);
    } else {
        visibility
            .outgoing_visibility_blockers
            .push(OutgoingVisibility {
                player_id: recipient,
                count: 1,
            });
    }
    Ok(())
}
/// IndividualDeclareWarOn2aa5c00 checks each principal's Governors against the
/// other principal's cities. EditAssignedGovernor2b0c430 returns the first
/// governor with an equal complete ComponentID; only an own governor reaches
/// UnassignGovernor. Domestic cities and unrelated assignments do not block war.
/// A reached foreign assignment still requires the ordered unassignment effects.
pub fn no_foreign_city_governors(
    state: &LoadedState,
    first: i32,
    second: i32,
) -> Result<(), Error> {
    for (owner, other) in [(first, second), (second, first)] {
        let players = &state.retained().players;
        let player = players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Requires("current governor roster"))?;
        let foreign = players
            .iter()
            .find(|p| p.header.owner == other)
            .ok_or(Error::Requires("current city roster"))?;
        for city in &foreign.cities.records.entries {
            let city = &city.value.header;
            let assigned = player.governors.entries.iter().find(|slot| {
                let at = &slot.governor.assigned_city;
                i32::from(at.owner) == city.owner && at.component_type == 2 && at.id == city.id
            });
            if assigned.is_some_and(|slot| slot.governor.owning_player == owner) {
                return Err(Error::Requires("foreign assigned Governor unassignment"));
            }
        }
    }
    Ok(())
}
