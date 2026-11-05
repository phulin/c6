//! Initial placement's City.CulturalIdentity.DoCityUnitChanged (28737b0).
//! The native method tail-calls the same RecalculateConversionTrends kernel as
//! DoCityCreated. Current yield/amenity reads below reuse production queries;
//! their callers retain one state and one effect owner throughout placement.
pub use crate::found_city::cultural_identity::{Context, Definitions};
use crate::{
    city_turn::{yield_calculation, Error},
    found_city::cultural_identity,
};
use c6_core::{game_capabilities::Membership, loaded_state::LoadedState, unit::ComponentId};

/// Resolve the current owning City exactly once. The placement caller's absent
/// City arm skips this callback; an unavailable owner collection is not absence.
/// CManager checks owner/id validity (not component_type), then native Find
/// resolves the low16 slot. Subsequent queries use that City's current full ID.
pub fn city_unit_changed(
    state: &mut LoadedState,
    city: ComponentId,
    definitions: &Definitions,
    context: &mut impl Context,
) -> Result<(), Error> {
    if city.owner == -1 || city.id == u32::MAX {
        return Ok(());
    }
    let owner = i32::from(city.owner);
    if state
        .world()
        .valid_player(owner)
        .map_err(|_| Error::Unsupported("placement current City owner"))?
        .is_none()
    {
        return Ok(());
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("placement canonical Cities"))?;
    let Some(entry) = player
        .cities
        .records
        .entries
        .iter()
        .find(|r| r.index == (city.id & 0xffff) as i32)
    else {
        return Ok(());
    };
    if entry.value.header.owner != owner {
        return Err(Error::Unsupported(
            "placement canonical City owner identity",
        ));
    }
    let current_id = entry.value.header.id;
    cultural_identity::do_city_created(state, definitions, context, owner, current_id)
}
fn subtract_consumption(food: i32, population: i32, per_population: i32) -> i32 {
    // GetFoodConsumption2be3380 wraps the integer product before the Q8
    // constructor; GetFoodSurplus288a790 then subtracts that raw Q8 word.
    food.wrapping_sub(population.wrapping_mul(per_population).wrapping_shl(8))
}
/// Build Current at the reached query, borrowing the canonical trade graph
/// only for this read. Do not retain a state borrow across trend cache writes.
pub fn food_surplus_q8(
    state: &LoadedState,
    owner: i32,
    city: u32,
    definitions: &yield_calculation::Definitions,
    current: &yield_calculation::Current<'_>,
) -> Result<i32, Error> {
    let food = definitions
        .yield_index(b"YIELD_FOOD")
        .ok_or(Error::Unsupported("placement Food definition"))?;
    let value =
        yield_calculation::calculate(state, owner, city, food, u32::MAX, definitions, current)?;
    let population = state.city(owner, city)?.header.population;
    Ok(subtract_consumption(
        value,
        population,
        definitions.growth.food_consumption_per_population,
    ))
}
/// Native GetHappiness: actual amenities minus population/capability-dependent
/// demand, then the first matching admitted happiness definition's ordinal.
pub fn happiness(
    state: &LoadedState,
    owner: i32,
    city: u32,
    definitions: &yield_calculation::Definitions,
    capabilities: &Membership,
) -> Result<i32, Error> {
    let amenities =
        definitions
            .amenities
            .amenities(state, owner, city, capabilities, &definitions.growth)?;
    let needed = definitions
        .economy
        .amenities_needed(state.city(owner, city)?.header.population, capabilities);
    Ok(definitions
        .economy
        .happiness_from_surplus(amenities.wrapping_sub(needed)))
}
/// Current raw-property integer projections, grouped by their real owner. A
/// supplied empty map proves a missing key; an unsupplied map remains unknown.
/// Positive/noninteger property decoding belongs to the current typed owner.
pub fn current_player_property(
    maps: &[(i32, Vec<(u32, i32)>)],
    owner: i32,
    key: u32,
) -> Result<Option<i32>, Error> {
    let map = maps
        .iter()
        .find(|p| p.0 == owner)
        .ok_or(Error::Unsupported("current loyalty Player properties"))?;
    Ok(map.1.iter().rev().find(|p| p.0 == key).map(|p| p.1))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn food_consumption_wraps_before_fixed_point_subtraction() {
        assert_eq!(subtract_consumption(1024, 2, 2), 0);
        assert_eq!(subtract_consumption(256, 2, 2), -768);
        // Native int multiplication and Q8 construction both wrap; a wide
        // saturating food-consumption formula would produce another result.
        assert_eq!(subtract_consumption(-1, i32::MAX, 2), 511);
    }
    #[test]
    fn unknown_player_map_does_not_become_a_missing_property() {
        let maps = vec![(1, vec![])];
        assert_eq!(current_player_property(&maps, 1, 0x12345678).unwrap(), None);
        assert!(matches!(
            current_player_property(&maps, 0, 0x12345678),
            Err(Error::Unsupported("current loyalty Player properties"))
        ));
        let maps = vec![(1, vec![(7, -512), (8, 0)])];
        assert_eq!(current_player_property(&maps, 1, 7).unwrap(), Some(-512));
        assert_eq!(current_player_property(&maps, 1, 8).unwrap(), Some(0));
    }
}
