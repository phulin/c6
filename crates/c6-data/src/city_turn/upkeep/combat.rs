//! City.Combat.Restore2871ba0 -> District.Combat.Restore291cb60. This
//! restores attacks when actual CanAttack succeeds; it does not heal damage.
use crate::{city_turn::Error, loaded_actions::city_range_attack as combat};
use c6_core::loaded_state::LoadedState;
pub fn restore(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    definitions: &combat::Definitions,
    civilization_levels: &[(i32, u32)],
) -> Result<(), Error> {
    let mut index = 0;
    loop {
        let ids = &state.city(owner, city)?.districts.district_ids;
        if index == ids.len() {
            break;
        }
        let id = *ids
            .get(index)
            .ok_or(Error::Unsupported("invalidated city district cursor"))?;
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("current City combat owner"))?;
        if player
            .districts
            .records
            .entries
            .iter()
            .any(|s| s.value.id == id)
        {
            let district = state
                .district(owner, id)
                .map_err(|_| Error::Unsupported("current City combat District"))?;
            let strength = combat::attack_strength_current_q8(
                state,
                district,
                definitions,
                civilization_levels,
            )
            .map_err(|e| Error::CityCombat(Box::new(e)))?;
            if strength > 0 {
                if let Some(city) = combat::district_city_optional(state, district)
                    .map_err(|e| Error::CityCombat(Box::new(e)))?
                {
                    let attacks = city.header.attacks_per_turn;
                    state
                        .district_mut(owner, id)
                        .map_err(|_| Error::Unsupported("current City combat District"))?
                        .remaining_attacks = attacks;
                }
            }
        }
        index += 1;
    }
    Ok(())
}
