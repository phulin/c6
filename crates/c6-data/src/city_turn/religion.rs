//! Canonical population-driven City.Religion updates and ordered conversion
//! effects. Called by growth and founding against the same LoadedState owner.
use super::{Error, GrowthRules};
use c6_core::{city_religion as core, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReligionIdentity {
    pub hash: u32,
    /// Current native definition index; NO_RELIGION is -1.
    pub native_type: i32,
}
fn identity(rules: &GrowthRules, hash: u32) -> Result<ReligionIdentity, Error> {
    let native_type = if hash == core::NO_RELIGION {
        -1
    } else {
        rules
            .religions
            .find_index(hash)
            .map(|i| i as i32)
            .ok_or(Error::Unsupported("current Religion definition"))?
    };
    Ok(ReligionIdentity { hash, native_type })
}

pub trait Effects {
    fn civilization_level(&mut self, state: &LoadedState, owner: i32) -> Result<u32, Error>;
    fn give_free_envoy(
        &mut self,
        state: &mut LoadedState,
        founder: i32,
        recipient: i32,
    ) -> Result<(), Error>;
    fn city_religion_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        old: ReligionIdentity,
        new: ReligionIdentity,
    ) -> Result<(), Error>;
    fn report_city_religion(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        event: u32,
        religion: Option<ReligionIdentity>,
    ) -> Result<(), Error>;
    fn player_religion_changed(
        &mut self,
        state: &mut LoadedState,
        converter: i32,
        owner: i32,
        religion: ReligionIdentity,
    ) -> Result<(), Error>;
    fn city_converted_by(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        converter: i32,
        city: u32,
    ) -> Result<(), Error>;
    /// Actual current GetAlive traversal order, not ever-alive or sorted IDs.
    fn alive_players(&mut self, state: &LoadedState) -> Result<Vec<i32>, Error>;
}
fn founded(state: &LoadedState, owner: i32) -> Option<&c6_core::core_player::ReligionData> {
    state
        .retained()
        .religion
        .records
        .iter()
        .find(|r| r.founder == owner)
}
fn ally_founded(
    state: &LoadedState,
    owner: i32,
    rules: &GrowthRules,
) -> Result<Option<u32>, Error> {
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("current religion player"))?;
    if player.religion.adopts_ally_founded_religions <= 0 || player.influence.suzerain == -1 {
        return Ok(None);
    }
    state
        .world()
        .player(player.influence.suzerain)
        .map_err(|_| Error::Unsupported("current suzerain player"))?;
    // HasCreatedReligion is strictly positive native ReligionTypes. The
    // canonical hash must resolve to a real founded global religion record;
    // a pantheon/no-religion record is not promoted to a created religion.
    Ok(founded(state, player.influence.suzerain)
        .filter(|r| {
            rules
                .religions
                .find_index(r.religion_hash)
                .is_some_and(|i| i > 0)
        })
        .map(|r| r.religion_hash))
}

pub fn population_changed(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    delta: i32,
    rules: &GrowthRules,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let old = state.city(owner, city)?.religion.majority_religion_hash;
    if delta > 0 {
        let raw = rules
            .atheism_pressure_per_pop
            .wrapping_mul(delta)
            .wrapping_shl(8);
        core::add_pressure(
            &mut state.city_mut(owner, city)?.religion.religion_status,
            old,
            raw,
        );
    }
    if delta != 0 {
        recompute(state, owner, city, old, -1, rules, effects)?;
    }
    Ok(())
}

pub fn recompute(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    previous_majority: u32,
    converter: i32,
    rules: &GrowthRules,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let population = state.city(owner, city)?.header.population;
    if population <= 0 {
        return Ok(());
    }
    let ally = ally_founded(state, owner, rules)?;
    let outcome = core::recompute(
        &mut state.city_mut(owner, city)?.religion,
        population,
        previous_majority,
        rules.atheism_pressure_per_pop,
        ally,
    )
    .map_err(|_| Error::Unsupported("native religious pressure division"))?;
    if !outcome.changed {
        return Ok(());
    }
    if !outcome.previously_initiated && effects.civilization_level(state, owner)? == 0xc2577075 {
        let award = state
            .retained()
            .religion
            .records
            .iter()
            .find(|r| r.religion_hash == outcome.majority)
            .filter(|r| r.awards_envoy)
            .map(|r| r.founder);
        if let Some(founder) = award {
            effects.give_free_envoy(state, founder, owner)?;
        }
    }
    let previous = identity(rules, previous_majority)?;
    let current = identity(rules, outcome.majority)?;
    effects.city_religion_changed(state, owner, city, previous, current)?;
    if !outcome.previously_initiated {
        if let Some(row) = state
            .city_mut(owner, city)?
            .religion
            .religion_status
            .iter_mut()
            .find(|r| r.religion_hash == outcome.majority)
        {
            row.initiation_bonus_paid = true;
        }
    }
    effects.report_city_religion(state, owner, city, 0x51c84cba, Some(current))?;
    effects.player_religion_changed(state, converter, owner, current)?;
    if converter != -1 && converter != owner {
        if let Some(religion) = founded(state, owner) {
            if rules
                .religions
                .find_index(religion.religion_hash)
                .is_some_and(|i| i > 0)
                && religion.religion_hash != outcome.majority
            {
                effects.city_converted_by(state, owner, converter, city)?;
            }
        }
    }
    let alive = effects.alive_players(state)?;
    mark_nearby_cities_dirty(state, owner, city, &alive, rules.adjacent_religion_distance)?;
    effects.report_city_religion(state, owner, city, 0x99e85ec6, None)
}

/// Game.Religion.MarkNearbyCitiesDirty: alive-player order, then native live
/// city slot order; SetNextReligionDirty writes only these two canonical fields.
pub fn mark_nearby_cities_dirty(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    alive_players: &[i32],
    adjacent_distance: i32,
) -> Result<(), Error> {
    let origin = state.city(owner, city)?;
    let location = (origin.header.location_x, origin.header.location_y);
    let cutoff = adjacent_distance.wrapping_mul(2);
    for &viewer in alive_players {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == viewer)
            .ok_or(Error::Unsupported("current alive religion player"))?;
        let cities = &player.cities;
        let reached: Vec<u32> = cities
            .records
            .entries
            .iter()
            .filter_map(|entry| {
                let header = &entry.value.header;
                (crate::loaded_actions::city_range_attack::plot_distance(
                    state,
                    location,
                    (header.location_x, header.location_y),
                ) < cutoff)
                    .then_some(header.id)
            })
            .collect();
        for id in reached {
            let religion = &mut state.city_mut(viewer, id)?.religion;
            religion.next_religion_hash = core::NO_RELIGION;
            religion.turns_to_next_religion = -1;
        }
    }
    Ok(())
}
