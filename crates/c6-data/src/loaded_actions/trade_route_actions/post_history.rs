//! Linux TradingPostChangedDispatcher29e72b0 over current cities and History.
//! Requests use the existing canonical HandleMoment continuation; selection
//! never records a moment itself or infers admission from the History endpoint.
use super::ComponentId;
use c6_core::{
    game_history::{Moment, MomentData, MOMENT_VERSION},
    loaded_state::LoadedState,
    unit::PlotCoord,
};

const WORLD: u32 = 0x1859a943;
const ALL: u32 = 0x53fd2135;
const FOREIGN: u32 = 0x8c22ce24;
const FOREIGN_PLAYER: u32 = 0xa6fe45c4;

fn select(
    moments: &[Moment],
    owner: i32,
    foreign: i32,
    all: bool,
) -> Option<(u32, Option<(u32, i32)>)> {
    let has = |kind, player: Option<i32>| {
        moments
            .iter()
            .any(|m| m.type_hash == kind && player.is_none_or(|p| m.acting_player_ordinal == p))
    };
    if all {
        if !has(WORLD, None) {
            return Some((WORLD, None));
        }
        return (!has(WORLD, Some(owner)) && !has(ALL, Some(owner))).then_some((ALL, None));
    }
    (!moments.iter().any(|m| {
        m.type_hash == FOREIGN
            && m.acting_player_ordinal == owner
            && m.extra_data
                .iter()
                .any(|d| d.kind_raw == FOREIGN_PLAYER && d.value == foreign)
    }))
    .then_some((FOREIGN, Some((FOREIGN_PLAYER, foreign))))
}
/// `alive` is the live player-manager roster used by the ongoing Session.
/// HasActiveTradingPost || HasInactiveTradingPost equals the constructed bit.
/// An absent post bit has the native bounds-checked false result.
pub fn request(
    state: &LoadedState,
    city: ComponentId,
    owner: i32,
    levels: &[(i32, u32)],
    alive: &[i32],
) -> Result<Option<Moment>, String> {
    let foreign = i32::from(city.owner);
    if foreign == owner {
        return Ok(None);
    }
    let level = |p| {
        levels
            .iter()
            .find(|r| r.0 == p)
            .map(|r| r.1)
            .ok_or_else(|| "Trade post History civilization level".to_string())
    };
    if level(foreign)? == 0xc2577075 {
        return Ok(None);
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == foreign)
        .ok_or("Trade post History foreign Cities")?;
    let has_post = |c: &c6_core::city::City, who: i32| {
        usize::try_from(who)
            .ok()
            .and_then(|i| c.trade.constructed_trading_posts.get(i))
            .copied()
            .unwrap_or(false)
    };
    for row in &player.cities.records.entries {
        if row.value.header.id != city.id && has_post(&row.value, owner) {
            return Ok(None);
        }
    }
    // Native scans all living majors' posts and sets the result only when the
    // current builder qualifies. Other players cannot change this predicate.
    let mut all = false;
    if alive.contains(&owner) && level(owner)? == 0x253718b0 {
        all = true;
        for &other in alive {
            if other == owner || level(other)? != 0x253718b0 {
                continue;
            }
            let cities = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == other)
                .ok_or("Trade post History alive major Cities")?;
            if !cities
                .cities
                .records
                .entries
                .iter()
                .any(|r| has_post(&r.value, owner))
            {
                all = false;
                break;
            }
        }
    }
    let Some((kind, data)) = select(
        &state.game_history().map_err(|e| format!("{e:?}"))?.moments,
        owner,
        foreign,
        all,
    ) else {
        return Ok(None);
    };
    let city = state.city(foreign, city.id).map_err(|e| format!("{e:?}"))?;
    Ok(Some(Moment {
        wire_version: MOMENT_VERSION,
        type_hash: kind,
        turn: -1,
        action_plot: PlotCoord {
            x: city.header.location_x,
            y: city.header.location_y,
        },
        acting_player_ordinal: owner,
        id: -1,
        has_ever_been_commemorated_raw: 0,
        game_era_hash: u32::MAX,
        extra_data: data
            .into_iter()
            .map(|(kind_raw, value)| MomentData { kind_raw, value })
            .collect(),
        city_name_bytes: vec![],
        unit_citizen_name_bytes: vec![],
        other_city_name_bytes: vec![],
        era_score: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn moment(kind: u32, owner: i32, foreign: Option<i32>) -> Moment {
        Moment {
            wire_version: MOMENT_VERSION,
            type_hash: kind,
            turn: 1,
            action_plot: PlotCoord { x: 0, y: 0 },
            acting_player_ordinal: owner,
            id: 4,
            has_ever_been_commemorated_raw: 0,
            game_era_hash: u32::MAX,
            extra_data: foreign
                .into_iter()
                .map(|value| MomentData {
                    kind_raw: FOREIGN_PLAYER,
                    value,
                })
                .collect(),
            city_name_bytes: vec![],
            unit_citizen_name_bytes: vec![],
            other_city_name_bytes: vec![],
            era_score: 0,
        }
    }
    #[test]
    fn all_civilizations_uses_world_first_then_once_per_player() {
        assert_eq!(select(&[], 0, 1, true), Some((WORLD, None)));
        let recorded = vec![moment(WORLD, 1, None)];
        assert_eq!(select(&recorded, 0, 1, true), Some((ALL, None)));
        assert_eq!(select(&recorded, 1, 0, true), None);
        let recorded = vec![moment(WORLD, 1, None), moment(ALL, 0, None)];
        assert_eq!(select(&recorded, 0, 1, true), None);
    }
    #[test]
    fn foreign_duplicate_is_keyed_by_builder_and_destination_civilization() {
        let recorded = vec![moment(FOREIGN, 0, Some(1))];
        assert_eq!(select(&recorded, 0, 1, false), None);
        assert_eq!(
            select(&recorded, 0, 2, false),
            Some((FOREIGN, Some((FOREIGN_PLAYER, 2))))
        );
        assert_eq!(
            select(&recorded, 1, 0, false),
            Some((FOREIGN, Some((FOREIGN_PLAYER, 0))))
        );
        assert_eq!(select(&recorded, 0, 1, true), Some((WORLD, None)));
    }
}
