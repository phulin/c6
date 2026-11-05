//! GovernmentEnactedMomentDispatcher29ea130 over the live History owner.
use super::Definitions;
use crate::melee_attack::resolve::completion::history;
use c6_core::{
    game_history::{Moment, MomentData, MOMENT_VERSION},
    loaded_state::LoadedState,
    unit::PlotCoord,
};

/// Captured tier type hashes in the actual dispatcher, in source comparison order.
#[derive(Clone, Copy, Debug)]
pub struct Tiers(pub [u32; 4]);
const MOMENTS: [(u32, u32); 4] = [
    (0x8f4b40ce, 0x69d266b8),
    (0xa683f43c, 0x583a7c25),
    (0x08eb65ad, 0xfe4d7791),
    (0xf5129dd8, 0x3bea491f),
];

fn select(moments: &[Moment], owner: i32, pair: (u32, u32)) -> Option<u32> {
    let has = |kind, player: Option<i32>| {
        moments
            .iter()
            .any(|m| m.type_hash == kind && player.is_none_or(|p| m.acting_player_ordinal == p))
    };
    if !has(pair.0, None) {
        return Some(pair.0);
    }
    if has(pair.0, Some(owner)) || has(pair.1, Some(owner)) {
        None
    } else {
        Some(pair.1)
    }
}
/// The first adopter globally receives the world-first moment; later adopters
/// receive the ordinary moment once. The manager still performs its own live
/// capability/major/era admission and synchronous score/report continuations.
#[allow(clippy::too_many_arguments)]
pub fn enacted<E: history::Effects>(
    state: &mut LoadedState,
    governments: &Definitions,
    tiers: Tiers,
    owner: i32,
    government: i32,
    definitions: &history::Definitions,
    inputs: history::Inputs<'_>,
    effects: &mut E,
) -> Result<Option<i32>, history::Error<E::Error>> {
    if state.binding() != &governments.binding {
        return Err(history::Error::Binding);
    }
    let Some(tier) = governments.government_tier(government) else {
        return Ok(None);
    };
    let Some(index) = tiers.0.iter().position(|&t| t == tier) else {
        return Ok(None);
    };
    let Some(kind) = select(
        &state
            .game_history()
            .map_err(history::Error::History)?
            .moments,
        owner,
        MOMENTS[index],
    ) else {
        return Ok(None);
    };
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
                kind_raw: 0xad622ff5,
                value: government,
            }],
            city_name_bytes: vec![],
            unit_citizen_name_bytes: vec![],
            other_city_name_bytes: vec![],
            era_score: 0,
        },
        effects,
    )
}

/// Stock cache/achievement publishers with no added synchronous consumers.
/// Initializers must qualify the current MomentRecorded ring and report routes;
/// custom gameplay subscribers require a concrete `history::Effects` service.
pub struct StockEffects<'a> {
    pub recorded: &'a [history::RecordedListener],
    pub reports: Vec<Report>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Report {
    Recorded { id: i32, kind: u32, owner: i32 },
    Moment { id: i32, kind: u32 },
    EraScore { owner: i32, amount: i32 },
    Notice { id: i32, owner: i32 },
}
#[derive(Debug)]
pub enum StockError {
    Recorded(history::RecordedError),
    Eras(c6_core::loaded_state::game_eras::AccessError),
    Unsupported(&'static str),
}
impl history::Effects for StockEffects<'_> {
    type Error = StockError;
    fn moment_recorded(
        &mut self,
        _: &mut LoadedState,
        id: i32,
        kind: u32,
        owner: i32,
    ) -> Result<(), StockError> {
        history::stock_recorded(self.recorded, kind).map_err(StockError::Recorded)?;
        self.reports.push(Report::Recorded { id, kind, owner });
        Ok(())
    }
    fn report(&mut self, _: &mut LoadedState, id: i32, kind: u32) -> Result<(), StockError> {
        self.reports.push(Report::Moment { id, kind });
        Ok(())
    }
    fn change_era_score(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        amount: i32,
        reason: i32,
    ) -> Result<(), StockError> {
        if reason != 0 {
            return Err(StockError::Unsupported("nonhistoric EraScore reason"));
        }
        if state
            .change_game_era_score(owner, amount, reason)
            .map_err(StockError::Eras)?
        {
            // Source2998080(reason0) publishes788b7534 after the canonical cell
            // mutation; its stock consumer274d500 refreshes numeric cache only.
            self.reports.push(Report::EraScore { owner, amount });
        }
        Ok(())
    }
    fn era_score_notice(&mut self, _: &mut LoadedState, moment: &Moment) -> Result<(), StockError> {
        // PrideMoment is nonblocking. Under this qualified stock scope its
        // name/translation/Host output has no gameplay or RNG consumer.
        self.reports.push(Report::Notice {
            id: moment.id,
            owner: moment.acting_player_ordinal,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(owner: i32, kind: u32) -> Moment {
        Moment {
            wire_version: MOMENT_VERSION,
            type_hash: kind,
            turn: 1,
            action_plot: PlotCoord { x: -9999, y: -9999 },
            acting_player_ordinal: owner,
            id: 0,
            has_ever_been_commemorated_raw: 0,
            game_era_hash: 0,
            extra_data: vec![],
            city_name_bytes: vec![],
            unit_citizen_name_bytes: vec![],
            other_city_name_bytes: vec![],
            era_score: 0,
        }
    }
    #[test]
    fn first_world_adopter_and_later_player_are_distinct_once() {
        let pair = MOMENTS[0];
        assert_eq!(select(&[], 0, pair), Some(pair.0));
        let mut rows = vec![entry(1, pair.0)];
        assert_eq!(select(&rows, 0, pair), Some(pair.1));
        assert_eq!(select(&rows, 1, pair), None);
        rows.push(entry(0, pair.1));
        assert_eq!(select(&rows, 0, pair), None);
        assert_eq!(select(&rows, 2, pair), Some(pair.1));
    }
}
