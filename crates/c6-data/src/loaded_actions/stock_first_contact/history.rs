//! PlayersMet History dispatcher (Linux 0x29e2a00).
use super::Error;
use crate::melee_attack::resolve::completion::history;
use c6_core::{
    game_history::{Moment, MomentData, MOMENT_VERSION},
    loaded_state::LoadedState,
    unit::PlotCoord,
};
const MAJOR_MET: u32 = 0x30a05382;
const ALL_MAJORS_FIRST: u32 = 0xca271f14;
const ALL_MAJORS: u32 = 0x3cf13636;
fn error(value: impl std::fmt::Debug) -> Error {
    Error::Requirement(format!("first-contact history: {value:?}"))
}
/// The native pointer roster and the ID roster are separate current lists.
/// Null pointers are skipped; each nonself ID requires directed HasMet.
/// Both moment types are selected before either HandleMoment callback runs.
pub fn select(
    state: &LoadedState,
    alive_major_players: &[Option<i32>],
    alive_major_ids: &[i32],
    first: i32,
    second: i32,
) -> Result<[u32; 2], Error> {
    let relations = state.game_relations().map_err(Error::State)?;
    let mut all = [false; 2];
    let mut third_complete = false;
    for &owner in alive_major_players.iter().flatten() {
        let complete = alive_major_ids
            .iter()
            .copied()
            .filter(|&other| other != owner)
            .try_fold(true, |met, other| {
                if !met {
                    return Ok(false);
                }
                relations.has_met(owner, other).map_err(error)
            })?;
        if owner == first {
            all[0] = complete;
        } else if owner == second {
            all[1] = complete;
        } else if complete {
            third_complete = true;
        }
    }
    Ok(all.map(|complete| {
        if !complete {
            MAJOR_MET
        } else if third_complete {
            ALL_MAJORS
        } else {
            ALL_MAJORS_FIRST
        }
    }))
}
#[allow(clippy::too_many_arguments)]
pub fn on_met<E: history::Effects>(
    state: &mut LoadedState,
    definitions: &history::Definitions,
    inputs: history::Inputs<'_>,
    alive_major_players: &[Option<i32>],
    alive_major_ids: &[i32],
    first: i32,
    second: i32,
    effects: &mut E,
) -> Result<(), Error>
where
    E::Error: std::fmt::Debug,
{
    let kinds = select(state, alive_major_players, alive_major_ids, first, second)?;
    for ((owner, other), kind) in [(first, second), (second, first)].into_iter().zip(kinds) {
        history::handle_moment(
            state,
            definitions,
            inputs,
            Moment {
                wire_version: MOMENT_VERSION,
                type_hash: kind,
                turn: -1,
                action_plot: PlotCoord { x: -9999, y: -9999 },
                acting_player_ordinal: owner,
                id: -1,
                has_ever_been_commemorated_raw: 0,
                game_era_hash: u32::MAX,
                extra_data: vec![MomentData {
                    kind_raw: 0xa6fe45c4,
                    value: other,
                }],
                city_name_bytes: vec![],
                unit_citizen_name_bytes: vec![],
                other_city_name_bytes: vec![],
                era_score: 0,
            },
            effects,
        )
        .map_err(error)?;
    }
    Ok(())
}
