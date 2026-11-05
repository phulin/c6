//! CityBuiltMomentDispatcher29e3f60: ordered current-state predicates.
//! Each successful predicate synchronously calls the caller's one History owner.
use super::{Current, Error};
use crate::{
    registry::LinuxDefinitionRegistry, unit_initialization_definitions::collection, Rules,
};
use c6_core::{
    game_history::{Moment, MomentData, MOMENT_VERSION},
    loaded_state::LoadedState,
    map::Component,
    unit::{ComponentId, PlotCoord},
};

pub trait Effects {
    fn handle_moment(&mut self, state: &mut LoadedState, moment: Moment) -> Result<(), Error>;
}
pub struct Definitions {
    features: LinuxDefinitionRegistry,
    natural_wonder: Vec<bool>,
    terrains: LinuxDefinitionRegistry,
    continents: LinuxDefinitionRegistry,
    volcanoes: LinuxDefinitionRegistry,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let features = collection(rules, "Features", "FeatureType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Features")
            .ok_or(Error::Missing("Features"))?;
        let natural_wonder = features
            .entries()
            .iter()
            .map(|e| {
                let row = table
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Missing("Feature row"))?;
                crate::game_parameters::sql_integer(row.get("NaturalWonder").map_err(Error::Rules)?)
                    .map(|v| v != 0)
                    .ok_or(Error::Missing("Features.NaturalWonder"))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            features,
            natural_wonder,
            terrains: collection(rules, "Terrains", "TerrainType").map_err(Error::Definitions)?,
            continents: collection(rules, "Continents", "ContinentType")
                .map_err(Error::Definitions)?,
            volcanoes: collection(rules, "NamedVolcanoes", "NamedVolcanoType")
                .map_err(Error::Definitions)?,
        })
    }
}
fn has(
    state: &LoadedState,
    owner: i32,
    kind: u32,
    data: Option<(u32, i32)>,
) -> Result<bool, Error> {
    let history = state
        .game_history()
        .map_err(|_| Error::Missing("current History"))?;
    Ok(has_in(&history.moments, owner, kind, data))
}
fn has_in(moments: &[Moment], owner: i32, kind: u32, data: Option<(u32, i32)>) -> bool {
    moments.iter().any(|m| {
        m.acting_player_ordinal == owner
            && m.type_hash == kind
            && data
                .is_none_or(|(k, v)| m.extra_data.iter().any(|d| d.kind_raw == k && d.value == v))
    })
}
fn moment(owner: i32, location: PlotCoord, kind: u32, data: Option<(u32, i32)>) -> Moment {
    // Constructor29dfb00 +10 constant [-1,-1,-1,0], commemorated=false.
    Moment {
        wire_version: MOMENT_VERSION,
        type_hash: kind,
        turn: -1,
        action_plot: location,
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
    }
}
fn count(state: &LoadedState, owner: i32) -> Result<i32, Error> {
    let p = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current City collection"))?;
    i32::try_from(p.cities.records.entries.len()).map_err(|_| Error::Missing("City count range"))
}
fn neighborhood(state: &LoadedState, center: PlotCoord, radius: u32) -> Result<Vec<usize>, Error> {
    let map = state.world().map();
    let mut out = vec![];
    c6_core::unit_placement_tails::neighborhood(center, radius, |p| {
        let x = c6_core::movement_world::coord_range(p.x, map.header.width, map.header.wrap_x);
        let y = c6_core::movement_world::coord_range(p.y, map.header.height, map.header.wrap_y);
        if x >= 0 && x < map.header.width && y >= 0 && y < map.header.height {
            let index = (y as usize) * (map.header.width as usize) + (x as usize);
            if index >= map.plots.len() {
                return Err(Error::Missing("current neighborhood Plot"));
            }
            out.push(index);
        }
        Ok(())
    })?;
    Ok(out)
}
fn city_at(state: &LoadedState, index: usize) -> Result<Option<ComponentId>, Error> {
    let map = state.world().map();
    let (x, y) = map
        .coordinates(index as u32)
        .ok_or(Error::Missing("City Plot coordinates"))?;
    for p in &state.retained().players {
        for e in &p.cities.records.entries {
            let h = &e.value.header;
            if h.location_x == x && h.location_y == y {
                return Ok(Some(ComponentId {
                    owner: h.owner as i16,
                    component_type: 2,
                    id: h.id,
                }));
            }
        }
    }
    Ok(None)
}
/// The dispatcher is invoked only for a non-null newly founded City and false
/// transfer flag. A transfer caller must skip this entire entry at its signal.
pub fn do_city_built(
    state: &mut LoadedState,
    city: ComponentId,
    defs: &Definitions,
    current: &Current<'_>,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let owner = i32::from(city.owner);
    let h = &state.city(owner, city.id)?.header;
    let location = PlotCoord {
        x: h.location_x,
        y: h.location_y,
    };
    // Largest other alive-major city count; strict comparison preserves first tie.
    let own = count(state, owner)?;
    let mut largest = 0;
    let mut other = -1;
    for &p in current.alive {
        if p != owner && current.ever_alive_majors.contains(&p) {
            let n = count(state, p)?;
            if n > largest {
                largest = n;
                other = p;
            }
        }
    }
    if other != -1 && own.wrapping_sub(3) >= largest && !has(state, owner, 0x23dc6db5, None)? {
        effects.handle_moment(
            state,
            moment(owner, location, 0x23dc6db5, Some((0xa6fe45c4, other))),
        )?;
    }
    let nearby = neighborhood(state, location, 2)?;
    // Native lambda overwrites on every match: LAST natural wonder, not nearest.
    let mut wonder = None;
    for &i in &nearby {
        let hash = state.world().map().plots[i].feature_hash;
        if let Some(index) = defs.features.find_index(hash) {
            if defs.natural_wonder[index] {
                wonder = Some(index as i32);
            }
        }
    }
    if let Some(wonder) = wonder {
        if !has(state, owner, 0x4ff93421, Some((0x536178f1, wonder)))? {
            effects.handle_moment(
                state,
                moment(owner, location, 0x4ff93421, Some((0x536178f1, wonder))),
            )?;
        }
    }
    // First revealed foreign-major City whose owner has met this owner.
    let mut foreign = None;
    for i in neighborhood(state, location, 5)? {
        let Some(candidate) = city_at(state, i)? else {
            continue;
        };
        let candidate_owner = i32::from(candidate.owner);
        if candidate_owner == owner {
            continue;
        }
        let visibility = state
            .world()
            .map()
            .visibility
            .iter()
            .find(|p| p.player == owner)
            .ok_or(Error::Missing("current CityBuilt revealed visibility"))?;
        let revealed = *visibility
            .visibility
            .revealed
            .get(i)
            .ok_or(Error::Missing("current revealed cell"))?;
        if revealed == 0 {
            continue;
        }
        if revealed != 1 {
            return Err(Error::Missing("canonical revealed bool"));
        }
        if !current.ever_alive_majors.contains(&candidate_owner) {
            continue;
        }
        if !state
            .game_relations()?
            .has_met(candidate_owner, owner)
            .map_err(|_| Error::Missing("current HasMet"))?
        {
            continue;
        }
        if foreign.is_none() {
            foreign = Some(candidate);
        }
    }
    if let Some(foreign) = foreign {
        let mut m = moment(
            owner,
            location,
            0xf01373a2,
            Some((0xa6fe45c4, i32::from(foreign.owner))),
        );
        m.city_name_bytes = state.city(owner, city.id)?.header.name.clone();
        m.other_city_name_bytes = state
            .city(i32::from(foreign.owner), foreign.id)?
            .header
            .name
            .clone();
        effects.handle_moment(state, m)?;
    }
    // CanBeFlooded uses first matching River in current manager order. Across
    // plots the lambda overwrites, so the final matching neighborhood plot wins.
    let mut river = -1;
    for &i in &nearby {
        let component = state
            .world()
            .map()
            .components
            .iter()
            .find_map(|c| {
                if let Component::River(r) = c {
                    Some(r)
                } else {
                    None
                }
            })
            .ok_or(Error::Missing("current River manager"))?;
        if let Some(r) = component
            .rivers
            .iter()
            .find(|r| r.floodplains.contains(&(i as i32)))
        {
            if r.id != -1 {
                river = r.id;
            }
        }
    }
    if river != -1 && !has(state, owner, 0xc76a2ec5, None)? {
        let mut m = moment(owner, location, 0xc76a2ec5, Some((0x26009007, river)));
        m.city_name_bytes = state.city(owner, city.id)?.header.name.clone();
        effects.handle_moment(state, m)?;
    }
    let mut volcano = -1;
    for &i in &nearby {
        let component = state
            .world()
            .map()
            .components
            .iter()
            .find_map(|c| {
                if let Component::Feature(f) = c {
                    Some(f)
                } else {
                    None
                }
            })
            .ok_or(Error::Missing("current Feature manager"))?;
        if let Some(v) = component.volcanoes.iter().find(|v| v.plot == i as i32) {
            volcano = defs
                .volcanoes
                .find_index(v.name_hash)
                .map_or(-1, |n| n as i32);
        } else if component
            .natural_wonder_volcanoes
            .iter()
            .any(|v| v.plots.contains(&(i as i32)))
        {
            volcano = -1; // IsVolcano true; GetVolcanoType returns INVALID for NW.
        }
    }
    if volcano != -1 && !has(state, owner, 0x9598d6ba, None)? {
        let mut m = moment(owner, location, 0x9598d6ba, Some((0x049a17d1, volcano)));
        m.city_name_bytes = state.city(owner, city.id)?.header.name.clone();
        effects.handle_moment(state, m)?;
    }
    let map = state.world().map();
    let (_, plot) = c6_core::movement_world::plot_at(map, location)
        .map_err(|_| Error::Missing("current CityBuilt center"))?;
    if let Some(continent) = defs.continents.find_index(plot.continent_hash) {
        if count(state, owner)? as u32 >= 2 {
            let mut another = false;
            let p = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or(Error::Missing("current City collection"))?;
            for e in &p.cities.records.entries {
                let h = &e.value.header;
                if h.id == city.id {
                    continue;
                }
                let (_, p) = c6_core::movement_world::plot_at(
                    state.world().map(),
                    PlotCoord {
                        x: h.location_x,
                        y: h.location_y,
                    },
                )
                .map_err(|_| Error::Missing("other current City plot"))?;
                if defs.continents.find_index(p.continent_hash) == Some(continent) {
                    another = true;
                    break;
                }
            }
            if !another {
                effects.handle_moment(
                    state,
                    moment(
                        owner,
                        location,
                        0x93f514e1,
                        Some((0xc345363f, continent as i32)),
                    ),
                )?;
            }
        }
    }
    // Source hardcodes native terrain indices, resolved by the admitted order.
    for (indices, kind) in [
        ([6, 7], 0xf24cd299),
        ([12, 13], 0xb28f2afd),
        ([9, 10], 0x5f16461e),
    ] {
        let (_, p) = c6_core::movement_world::plot_at(state.world().map(), location)
            .map_err(|_| Error::Missing("current City terrain"))?;
        if defs
            .terrains
            .find_index(p.terrain_hash)
            .is_some_and(|v| indices.contains(&v))
        {
            effects.handle_moment(state, moment(owner, location, kind, None))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn different_wonders_and_players_do_not_suppress_a_new_wonder_moment() {
        let recorded = vec![moment(
            0,
            PlotCoord { x: 1, y: 2 },
            0x4ff93421,
            Some((0x536178f1, 7)),
        )];
        assert!(has_in(&recorded, 0, 0x4ff93421, Some((0x536178f1, 7))));
        assert!(!has_in(&recorded, 0, 0x4ff93421, Some((0x536178f1, 8))));
        assert!(!has_in(&recorded, 1, 0x4ff93421, Some((0x536178f1, 7))));
        assert!(!has_in(&recorded, 0, 0x9598d6ba, None));
        assert!(has_in(&recorded, 0, 0x4ff93421, None));
    }
}
