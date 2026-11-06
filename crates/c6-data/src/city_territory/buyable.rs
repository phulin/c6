//! Founding and capture placement's current cheapest culture-claim list.
//! This is the SAME GetBuyablePlotList kernel used by culture expansion;
//! gold-purchasable plots use a different eligibility and cost calculation.
pub use crate::city_turn::upkeep::buyable_plots::Definitions;
use crate::{
    city_territory::Runtime,
    city_turn::{tiles, upkeep::buyable_plots, yield_calculation, Error},
};
use c6_core::{city_territory::CityId, loaded_state::LoadedState};

/// Compute current minimum-score ties in native coordinate order, retaining
/// wrapped duplicates. Read yields/resource visibility from the same canonical
/// definitions and territory owner used by the ongoing placement callbacks.
///
/// This performs no random draw. The enclosing territory AcquireContext draws
/// exactly once from GameSync for a nonempty list, including a one-item list,
/// and recomputes this list after each actual annexation.
pub fn ordered_candidates(
    state: &LoadedState,
    territory: &Runtime,
    city: CityId,
    definitions: &Definitions,
    yields: &yield_calculation::Definitions,
    drought_ids: Option<&[i32]>,
) -> Result<Vec<u32>, Error> {
    buyable_plots::ordered_candidates(
        state,
        city.owner,
        city.id,
        definitions,
        &yields.tiles,
        &tiles::Current {
            territory,
            drought_ids,
            resource_visibility: &yields.resources,
        },
    )
}
