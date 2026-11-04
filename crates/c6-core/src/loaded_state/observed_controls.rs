//! Same-epoch initial refinement of unsaved current configuration fields.
use super::{Error, LoadedState, MissingInput, ObservedSnapshotWitness};

/// Fill unknown current teams only. This is not a gameplay team-change action.
/// A conflicting known value or duplicate owner rejects the whole input before
/// changing either current controls or the World's corresponding Player field.
pub fn extend_teams(
    state: &mut LoadedState,
    witness: &ObservedSnapshotWitness,
    values: &[(i32, i32)],
) -> Result<(), Error> {
    if !state.is_pristine() {
        return Err(Error::NotPristine);
    }
    if state.observed_snapshot.as_ref() != Some(witness) {
        return Err(Error::Binding);
    }
    let controls = state.controls.as_ref().ok_or(Error::Binding)?;
    let mut pending = Vec::new();
    pending
        .try_reserve(values.len())
        .map_err(|_| Error::ResourceLimit)?;
    for (i, &(owner, team)) in values.iter().enumerate() {
        if values[..i].iter().any(|p| p.0 == owner) {
            return Err(Error::Binding);
        }
        let index = controls
            .players
            .iter()
            .position(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let player = state.world.player(owner).map_err(Error::World)?;
        if player.team != controls.players[index].team {
            return Err(Error::Binding);
        }
        match controls.players[index].team {
            Some(old) if old != team => return Err(Error::Binding),
            Some(_) => {}
            None => pending.push((index, owner, team)),
        }
    }
    if !pending.is_empty() {
        state.advance_admission_generation()?;
        for (index, owner, team) in pending {
            state.controls.as_mut().expect("checked").players[index].team = Some(team);
            state
                .world
                .player_mut(owner)
                .expect("validated before writes")
                .team = Some(team);
        }
    }
    Ok(())
}
