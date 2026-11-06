//! PantheonFounded29e22d0 constructs an actual moment and invokes the existing
//! live History manager. First/ordinary is based on OTHER alive players having
//! a pantheon at this signal position, rather than saved moment-list emptiness.
use super::Current;
use crate::melee_attack::resolve::completion::history;
use c6_core::{
    game_history::{Moment, MomentData, MOMENT_VERSION},
    loaded_state::LoadedState,
    unit::PlotCoord,
};
#[allow(clippy::too_many_arguments)]
pub fn founded<E: history::Effects>(
    state: &mut LoadedState,
    owner: i32,
    belief: i32,
    current: &Current<'_>,
    definitions: &history::Definitions,
    inputs: history::Inputs<'_>,
    effects: &mut E,
) -> Result<Option<i32>, history::Error<E::Error>> {
    if owner == -1 {
        return Ok(None);
    }
    let owners = super::alive::<std::convert::Infallible>(state, current)
        .map_err(|_| history::Error::Missing("PantheonFounded current GetAlive"))?;
    let mut kind = 0x870dcb50;
    for other in owners {
        if other == owner {
            continue;
        }
        let p = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == other)
            .ok_or(history::Error::Missing(
                "PantheonFounded other Player.Religion",
            ))?;
        if p.religion.pantheon_hash != u32::MAX {
            kind = 0xc8ed86d6;
            break;
        }
    }
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
                kind_raw: 0xfff6ad73,
                value: belief,
            }],
            city_name_bytes: vec![],
            unit_citizen_name_bytes: vec![],
            other_city_name_bytes: vec![],
            era_score: 0,
        },
        effects,
    )
}
