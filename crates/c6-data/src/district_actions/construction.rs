//! Fresh Campus construction on the same canonical City, District and Map.
//! Synchronous gameplay services are mandatory and run in native order.
use super::{cost, missing, Definitions};
use crate::{city_production_turn as production, district_initialization as districts};
use c6_core::{loaded_state::LoadedState, unit::PlotCoord};

pub trait Effects: districts::Effects<Error = production::Error> {
    fn complete_route(
        &mut self,
        state: &mut LoadedState,
        district: districts::DistrictReference,
    ) -> Result<(), production::Error>;
    /// AwardDistrictCreateYield runs immediately after SetDistrictType.
    fn award_creation_yields(
        &mut self,
        state: &mut LoadedState,
        district: districts::DistrictReference,
    ) -> Result<(), production::Error>;
    fn recompute_cost(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        district_hash: u32,
    ) -> Result<(), production::Error>;
    /// PlayerSignals.DistrictZoned; policies may observe, never select a choice.
    fn district_zoned(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        district_type: i32,
    ) -> Result<(), production::Error>;
    fn reallocate_citizens(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), production::Error>;
    /// Culture bomb/capital effects, construction report and appeal, in source
    /// order. Unsupported reached leaves must fail.
    fn starting_buildings(
        &mut self,
        state: &mut LoadedState,
        district: districts::DistrictReference,
    ) -> Result<(), production::Error>;
    fn completed_aftermath(
        &mut self,
        state: &mut LoadedState,
        district: districts::DistrictReference,
    ) -> Result<(), production::Error>;
}

/// PreCreateDistrict's fresh ordinary path. The caller has evaluated current
/// CanZone before pushing this front entry. No clone or alternate district
/// roster is introduced; failures retain the source mutation prefix.
#[allow(clippy::too_many_arguments)]
pub fn place(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    location: PlotCoord,
    definitions: &Definitions,
    combat: &crate::loaded_actions::city_range_attack::Queries<'_>,
    effects: &mut impl Effects,
) -> Result<districts::DistrictReference, production::Error> {
    if state.binding() != &definitions.binding {
        return Err(production::Error::Binding);
    }
    let hash = definitions.campus_hash;
    let map = state.world().map();
    if map.plot_at(location.x, location.y).is_none() {
        return Err(missing("Campus construction plot"));
    }
    let index = location
        .y
        .wrapping_mul(map.header.width)
        .wrapping_add(location.x);
    let index = i16::try_from(index).map_err(|_| missing("Campus zoning plot index"))?;
    let zoning = state
        .city(owner, city)?
        .build_queue
        .zoning_plot
        .iter()
        .rposition(|v| v.definition_hash == hash)
        .ok_or(missing("Campus zoning cell"))?;
    state.city_mut(owner, city)?.build_queue.zoning_plot[zoning].value = index;
    let reference = districts::create_in_city(
        state,
        &definitions.district_initialization,
        combat,
        effects,
        districts::Creation {
            owner,
            city_id: city,
            location,
            district_type: definitions.campus_index,
            complete: false,
            bought: false,
        },
    )
    .map_err(|e| production::Error::DistrictInitialization(Box::new(e)))?
    .ok_or(missing("Campus district allocation"))?;
    effects.award_creation_yields(state, reference)?;
    let locked_cost = cost(state, owner, city, hash)?;
    state
        .district_mut(owner, reference.id)
        .map_err(production::Error::District)?
        .cost = locked_cost;
    effects.recompute_cost(state, owner, hash)?;
    effects.district_zoned(state, owner, definitions.campus_index)?;
    effects.reallocate_citizens(state, owner, city)?;
    Ok(reference)
}

/// PostCreateDistrict for an already placed, fresh ordinary Campus. Queue pop
/// and finished-production publication remain in the surrounding build queue.
pub fn complete(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    definitions: &Definitions,
    effects: &mut impl Effects,
) -> Result<districts::DistrictReference, production::Error> {
    if state.binding() != &definitions.binding {
        return Err(production::Error::Binding);
    }
    let ids = &state.city(owner, city)?.districts.district_ids;
    let mut found = None;
    for id in ids {
        let d = state
            .district(owner, *id)
            .map_err(production::Error::District)?;
        if d.district_hash == definitions.campus_hash {
            if found.is_some() {
                return Err(missing("ambiguous Campus membership"));
            }
            if d.is_complete || d.pillaged {
                return Err(missing("Campus repair/recompletion"));
            }
            found = Some(districts::DistrictReference { owner, id: *id });
        }
    }
    let reference = found.ok_or(missing("placed Campus"))?;
    // Native construction uses bought=true here, even for ordinary progress.
    districts::set_complete(
        state,
        &definitions.district_initialization,
        effects,
        reference,
        true,
        true,
    )
    .map_err(|e| production::Error::DistrictInitialization(Box::new(e)))?;
    effects.complete_route(state, reference)?;
    let progress = state
        .city(owner, city)?
        .build_queue
        .district_progress
        .iter()
        .rposition(|p| p.definition_hash == definitions.campus_hash)
        .ok_or(missing("Campus district progress"))?;
    let zoning = state
        .city(owner, city)?
        .build_queue
        .zoning_plot
        .iter()
        .rposition(|p| p.definition_hash == definitions.campus_hash)
        .ok_or(missing("Campus zoning cell"))?;
    // RemoveDistrictProgress clears the matching construction's location too.
    state.city_mut(owner, city)?.build_queue.district_progress[progress].value = 0;
    state.city_mut(owner, city)?.build_queue.zoning_plot[zoning].value = -1;
    effects.completed_aftermath(state, reference)?;
    effects.recompute_cost(state, owner, definitions.campus_hash)?;
    effects.starting_buildings(state, reference)?;
    Ok(reference)
}
