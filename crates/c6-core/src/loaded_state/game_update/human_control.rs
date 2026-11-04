//! Human deactivation effects of the stock AI.Control callbacks. Native
//! planning tables/priority bookkeeping are outside the external-policy arena.
//! Executable behavior trees and war-operation dispatch are not discarded.
use crate::{game_update::Error, loaded_state::LoadedState};

/// Execute the unit ownership changes made by Homeland/Tactical.DoTurnEnd.
/// Their human DoTurn selectors do not recruit/assign; CityBuild.QueueNextBuild
/// also returns before choosing production for a human. Priority-signal
/// Planning/Research/Targets consumers only maintain native planning caches;
/// CityBuild.DoTurnDeactivate is empty and Tactical's signal returns for humans.
/// The profiling-only Control.TurnComplete timer is omitted.
pub fn deactivate(state: &mut LoadedState, owner: i32) -> Result<(), Error> {
    if state
        .player_slot_status(owner)
        .map_err(|_| Error::Unsupported("current human controls"))?
        != 3
    {
        return Err(Error::Unsupported(
            "nonhuman deactivation gameplay continuation",
        ));
    }
    let player = state
        .retained
        .players
        .iter()
        .find(|p| p.requested_owner == owner)
        .ok_or(Error::Unsupported("current player control components"))?;
    let targets = player
        .ai_targets
        .as_ref()
        .ok_or(Error::Unsupported("current Targets"))?;
    if targets.new_war_started.0 != 0 {
        return Err(Error::Unsupported(
            "Targets war-operation dispatch before human cleanup",
        ));
    }
    let behavior = player
        .ai_behavior_tree
        .as_ref()
        .ok_or(Error::Unsupported("current behavior trees"))?;
    let priority = player
        .ai_control
        .as_ref()
        .ok_or(Error::Unsupported("current Control priority"))?
        .priority;
    // Update, DoGameCoreActions and CheckForDestroy all require this tree's
    // request priority to equal the current drain priority (26b66ef/26b6aa3/
    // 26b6f54). Trees outside the actual nonnegative drain range are untouched.
    if behavior
        .behavior_trees
        .entries
        .iter()
        .any(|entry| (0..=priority).contains(&entry.tree.request_priority))
    {
        return Err(Error::Unsupported("existing executable behavior trees"));
    }
    let homeland = player
        .ai_homeland
        .as_ref()
        .ok_or(Error::Unsupported("current Homeland control"))?
        .control_instance_id;
    let tactical = player
        .ai_tactical
        .as_ref()
        .ok_or(Error::Unsupported("current Tactical control"))?;
    let tactical = tactical.control_instance_id;
    // DoTurnEnd snapshots matching unit IDs and resolves each current unit;
    // SetAiController is an archived assignment, without synchronous callbacks.
    release_controller(state, owner, homeland)?;
    release_controller(state, owner, tactical)?;
    // Destroyable other controls also call FreeUnits. Their unsaved destruction
    // flags must be supplied if any current unit is still owned by such a control.
    if state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("current controller player"))?
        .unit_records()
        .map_err(|_| Error::Unsupported("current controller units"))?
        .iter()
        .any(|u| u.value.ai_controller != u32::MAX)
    {
        return Err(Error::Unsupported(
            "remaining control destruction flags and FreeUnits",
        ));
    }
    Ok(())
}

fn release_controller(state: &mut LoadedState, owner: i32, controller: u32) -> Result<(), Error> {
    // Assigning INVALID to already-unowned units has no gameplay effect; native
    // archive-dirty metadata is not part of logical gameplay state.
    if controller == u32::MAX {
        return Ok(());
    }
    let ids = state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("controller player"))?
        .unit_records()
        .map_err(|_| Error::Unsupported("controller units"))?
        .iter()
        .filter(|u| u.value.ai_controller == controller)
        .map(|u| u.value.id)
        .collect::<Vec<_>>();
    for id in ids {
        state
            .unit_mut(owner, id)
            .map_err(|_| Error::Unsupported("controller unit archive"))?
            .ai_controller = u32::MAX;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_resolves_current_units_and_only_changes_matching_controller() {
        let mut state = crate::loaded_state::tests::state();
        let owner = state.world().players()[0].owner();
        let id = 0x10001;
        let mut unit = crate::loaded_state::tests::commit_fixture_unit();
        unit.id = id;
        unit.e_owner = owner;
        let units = crate::map::Recycled {
            capacity: 2,
            entries: vec![crate::map::Slot {
                index: 1,
                value: unit,
            }],
            current_id_bits: 0x10000,
            free_head: u32::MAX,
            free_tail: u32::MAX,
            free_links: vec![u32::MAX; 2],
        };
        state.world = crate::world::World::new(
            state.world.map().clone(),
            vec![crate::world::Player::new(owner, Some(units), None).unwrap()],
        )
        .unwrap();
        state.unit_mut(owner, id).unwrap().ai_controller = 43;
        let stamp = state.action_stamp();
        release_controller(&mut state, owner, 42).unwrap();
        assert_eq!(state.action_stamp(), stamp);
        assert_eq!(
            state
                .world()
                .player(owner)
                .unwrap()
                .unit(id)
                .unwrap()
                .unwrap()
                .record
                .ai_controller,
            43
        );
        release_controller(&mut state, owner, 43).unwrap();
        assert_ne!(state.action_stamp(), stamp);
        assert_eq!(
            state
                .world()
                .player(owner)
                .unwrap()
                .unit(id)
                .unwrap()
                .unwrap()
                .record
                .ai_controller,
            u32::MAX
        );
    }
}
