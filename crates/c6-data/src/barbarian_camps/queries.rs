//! Computed camp placement queries over the current map and borrowed services.
//! The supported improvement profile is checked against current rules once;
//! no native predicate answers, candidate scores or selected plots are inputs.
use super::{Context, Parameters};
use crate::{
    game_parameters::sql_integer,
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_national_parks::ParkData,
    loaded_state::LoadedState,
    map::{Component, Plot, PlotFlag},
    unit::PlotCoord,
    visibility_change::BASE_LAYER_HASH,
};

#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    Rules(crate::Error),
    Placement(E),
}
pub trait Placement {
    type Error;
    /// Borrow the same mutable sight owner that placement will update later.
    /// A second imported or cloned visibility owner is not interchangeable.
    fn visibility(&self) -> Result<&crate::unit_sight_state::Runtime, Self::Error>;
    fn set_camp(
        &mut self,
        state: &mut LoadedState,
        location: PlotCoord,
        improvement: i32,
        owner: i32,
    ) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy)]
struct Terrain {
    hash: u32,
    water: bool,
    mountain: bool,
}
#[derive(Clone, Copy)]
struct Feature {
    hash: u32,
    wonder: bool,
}
pub struct Definitions {
    binding: WireBinding,
    improvement: i32,
    improvement_hash: u32,
    terrains: Vec<Terrain>,
    features: Vec<Feature>,
    valid_terrains: Vec<u32>,
    valid_features: Vec<u32>,
    difficulties: LinuxDefinitionRegistry,
    pub parameters: Parameters,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Missing(name));
        let improvements = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(|_| Error::Missing("camp improvement collection"))?;
        let hash = state.retained().barbarian.improvement_hash;
        let index = improvements
            .find_index(hash)
            .ok_or(Error::Missing("current camp improvement"))?;
        let entry = &improvements.entries()[index];
        let row = table("Improvements")?
            .row_by_id(entry.database_rowid)
            .ok_or(Error::Missing("camp improvement row"))?;
        let integer = |key| -> Result<i32, Error<std::convert::Infallible>> {
            i32::try_from(row.integer(key).map_err(Error::Rules)?).map_err(|_| Error::Missing(key))
        };
        // These fields select the finite NO_PLAYER,false branch implemented
        // below. A changed definition is unsupported, never a default value.
        for (key, expected) in [
            ("BarbarianCamp", 1),
            ("Buildable", 0),
            ("SameAdjacentValid", 1),
            ("RequiresRiver", 0),
            ("BuildInLine", 0),
            ("BuildOnFrontier", 0),
            ("OnlyOpenBorders", 0),
            ("AdjacentToLand", 0),
            ("Coast", 0),
            ("NoAdjacentSpecialtyDistrict", 0),
            ("OnePerCity", 0),
            ("AdjacentSeaResource", 0),
            ("RequiresAdjacentBonusOrLuxury", 0),
            ("RequiresAdjacentLuxury", 0),
        ] {
            if integer(key)? != expected {
                return Err(Error::Missing("unsupported camp improvement profile"));
            }
        }
        if row.text("Domain").map_err(Error::Rules)? != "DOMAIN_LAND"
            || !matches!(
                row.get("MinimumAppeal").map_err(Error::Rules)?,
                SqlValue::Null
            )
        {
            return Err(Error::Missing("unsupported camp domain/appeal"));
        }
        let name = row.text("ImprovementType").map_err(Error::Rules)?;
        for extension in ["Improvements_XP2", "Improvements_MODE"] {
            if rules.gameplay().table(extension).is_some_and(|t| {
                t.rows()
                    .any(|r| r.text("ImprovementType").ok() == Some(name))
            }) {
                return Err(Error::Missing("unsupported camp extension definition"));
            }
        }
        for relation in [
            "Improvement_ValidResources",
            "Improvement_ValidAdjacentTerrains",
            "Improvement_InvalidAdjacentFeatures",
            "Improvement_ValidAdjacentResources",
        ] {
            if table(relation)?
                .rows()
                .any(|r| r.text("ImprovementType").ok() == Some(name))
            {
                return Err(Error::Missing("unsupported camp improvement relation"));
            }
        }
        let valid = |relation, field| -> Result<Vec<u32>, Error<std::convert::Infallible>> {
            table(relation)?
                .rows()
                .filter(|r| r.text("ImprovementType").ok() == Some(name))
                .map(|r| {
                    if !matches!(r.get("PrereqTech").map_err(Error::Rules)?, SqlValue::Null)
                        || !matches!(r.get("PrereqCivic").map_err(Error::Rules)?, SqlValue::Null)
                    {
                        return Err(Error::Missing("camp placement prerequisite"));
                    }
                    Ok(linux_type_hash(
                        r.text(field).map_err(Error::Rules)?.as_bytes(),
                    ))
                })
                .collect()
        };
        let terrains = table("Terrains")?
            .rows()
            .map(|r| {
                Ok(Terrain {
                    hash: linux_type_hash(r.text("TerrainType").map_err(Error::Rules)?.as_bytes()),
                    water: r.integer("Water").map_err(Error::Rules)? != 0,
                    mountain: r.integer("Mountain").map_err(Error::Rules)? != 0,
                })
            })
            .collect::<Result<Vec<_>, Error<std::convert::Infallible>>>()?;
        let features = table("Features")?
            .rows()
            .map(|r| {
                Ok(Feature {
                    hash: linux_type_hash(r.text("FeatureType").map_err(Error::Rules)?.as_bytes()),
                    wonder: r.integer("NaturalWonder").map_err(Error::Rules)? != 0,
                })
            })
            .collect::<Result<Vec<_>, Error<std::convert::Infallible>>>()?;
        let globals = table("GlobalParameters")?;
        let parameter = |key| {
            globals
                .rows()
                .find(|r| r.text("Name").ok() == Some(key))
                .and_then(|r| r.get("Value").ok())
                .and_then(sql_integer)
                .ok_or(Error::Missing(key))
        };
        Ok(Self {
            binding: state.binding().clone(),
            improvement: index as i32,
            improvement_hash: hash,
            valid_terrains: valid("Improvement_ValidTerrains", "TerrainType")?,
            valid_features: valid("Improvement_ValidFeatures", "FeatureType")?,
            terrains,
            features,
            difficulties: crate::unit_initialization_definitions::collection(
                rules,
                "Difficulties",
                "DifficultyType",
            )
            .map_err(|_| Error::Missing("Difficulty collection"))?,
            parameters: Parameters {
                first_turn_percent: parameter(
                    "BARBARIAN_CAMP_FIRST_TURN_PERCENT_OF_TARGET_TO_ADD",
                )?,
                minimum_city_distance: parameter("BARBARIAN_CAMP_MINIMUM_DISTANCE_CITY")?,
                minimum_camp_distance: parameter("BARBARIAN_CAMP_MINIMUM_DISTANCE_ANOTHER_CAMP")?,
                extra_distance_per_low_difficulty: parameter(
                    "BARBARIAN_CAMP_EXTRA_DISTANCE_PER_LOW_DIFFICULTY",
                )?,
            },
        })
    }
    pub fn improvement(&self) -> i32 {
        self.improvement
    }
    fn terrain<E>(&self, plot: &Plot) -> Result<Terrain, Error<E>> {
        self.terrains
            .iter()
            .find(|t| t.hash == plot.terrain_hash)
            .copied()
            .ok_or(Error::Missing("camp Terrain definition"))
    }
    fn feature<E>(&self, plot: &Plot) -> Result<Option<Feature>, Error<E>> {
        if plot.feature_hash == u32::MAX {
            return Ok(None);
        }
        self.features
            .iter()
            .find(|f| f.hash == plot.feature_hash)
            .copied()
            .map(Some)
            .ok_or(Error::Missing("camp Feature definition"))
    }
}
/// Actual Player.Manager cached orders and current query owners. These are
/// borrowed; scoring does not rebuild player lists or copy the world.
pub struct Inputs<'a> {
    pub alive_ids: &'a [i32],
    pub alive_major_ids: &'a [i32],
    pub civilization_levels: &'a [(i32, u32)],
    pub national_parks: Option<&'a [ParkData]>,
}
pub struct Queries<'a, P> {
    pub definitions: &'a Definitions,
    pub inputs: Inputs<'a>,
    pub placement: &'a mut P,
}
fn plot<E>(state: &LoadedState, index: usize) -> Result<&Plot, Error<E>> {
    state
        .world()
        .map()
        .plots
        .get(index)
        .ok_or(Error::Missing("current camp Plot"))
}
fn city<E>(state: &LoadedState, index: usize) -> Result<bool, Error<E>> {
    Ok(district_type(state, index)? == Some(0x8a32007d))
}
fn district_type<E>(state: &LoadedState, index: usize) -> Result<Option<u32>, Error<E>> {
    let p = plot(state, index)?;
    if p.flags.get(PlotFlag::HasCityAttributes) != p.city_attributes.is_some() {
        return Err(Error::Missing("camp city attributes"));
    }
    let Some(a) = &p.city_attributes else {
        return Ok(None);
    };
    if a.district == u32::MAX || a.player == -1 {
        return Ok(None);
    }
    let owner = i32::from(a.player);
    if state
        .world()
        .valid_player(owner)
        .map_err(|_| Error::Missing("camp district player lookup"))?
        .is_none()
    {
        return Ok(None);
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("camp District owner"))?;
    let d = player
        .districts
        .records
        .entries
        .iter()
        .find(|r| r.index == (a.district & 0xffff) as i32);
    // CityAttributes.IsCity2bc8170 compares the resolved definition TypeHash.
    // Native Find masks the low slot and returns null for a missing record.
    Ok(d.map(|d| d.value.district_hash).filter(|h| *h != u32::MAX))
}
impl<P: Placement> Queries<'_, P> {
    fn check(&self, state: &LoadedState) -> Result<(), Error<P::Error>> {
        if state.binding() != &self.definitions.binding
            || state.retained().barbarian.improvement_hash != self.definitions.improvement_hash
        {
            return Err(Error::Missing("camp query binding"));
        }
        Ok(())
    }
    fn visible(
        &self,
        state: &LoadedState,
        index: usize,
        owners: &[i32],
    ) -> Result<bool, Error<P::Error>> {
        let map = state.world().map();
        let width = usize::try_from(map.header.width)
            .ok()
            .filter(|w| *w > 0)
            .ok_or(Error::Missing("camp map width"))?;
        for &owner in owners {
            let layers = self
                .placement
                .visibility()
                .map_err(Error::Placement)?
                .shared(owner)
                .map_err(Error::Missing)?;
            let normal = layers
                .0
                .iter()
                .find(|l| l.type_hash == BASE_LAYER_HASH)
                .ok_or(Error::Missing("camp normal visibility"))?;
            let cell = (index / width)
                .checked_mul(normal.grid_width as usize)
                .and_then(|n| n.checked_add(index % width))
                .ok_or(Error::Missing("camp visibility index"))?;
            if *normal
                .values
                .get(cell)
                .ok_or(Error::Missing("camp visibility cell"))?
                > 0
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// CanHaveImprovement2a398f0, NO_PLAYER,false, checked camp definition.
    pub fn can_have(
        &self,
        state: &LoadedState,
        index: usize,
        kind: i32,
    ) -> Result<bool, Error<P::Error>> {
        self.check(state)?;
        if kind != self.definitions.improvement {
            return Err(Error::Missing("camp improvement index"));
        }
        let p = plot(state, index)?;
        if city(state, index)? {
            return Ok(false);
        }
        let parks = self
            .inputs
            .national_parks
            .ok_or(Error::Missing("current NationalParks"))?;
        if parks
            .iter()
            .any(|park| park.plot_indices.contains(&(index as i32)))
        {
            return Ok(false);
        }
        let terrain_component = state
            .world()
            .map()
            .components
            .iter()
            .find_map(|c| {
                if let Component::Terrain(t) = c {
                    Some(t)
                } else {
                    None
                }
            })
            .ok_or(Error::Missing("current Terrain component"))?;
        if terrain_component
            .coastal_lowlands
            .iter()
            .find(|l| l.plot_index == index as i32)
            .is_some_and(|l| l.flooded)
        {
            return Ok(false);
        }
        let terrain = self.definitions.terrain(p)?;
        // Camp has no XP2 BuildOnAdjacentPlot definition. IsCanyon2bc9c00 is
        // constant false on this Linux target, independently of old platforms.
        if p.flags.get(PlotFlag::Impassable) || terrain.mountain {
            return Ok(false);
        }
        if self.definitions.feature(p)?.is_some_and(|f| f.wonder) {
            return Ok(false);
        }
        if district_type(state, index)?.is_some() {
            return Ok(false);
        }
        if p.city_attributes
            .as_ref()
            .is_some_and(|a| a.wonder_building_hash != u32::MAX)
        {
            return Err(Error::Missing("camp completed Wonder query"));
        }
        if p.improvement_hash != u32::MAX {
            return Ok(false);
        }
        if terrain.water {
            return Ok(false);
        }
        // GetVisibleResource(NO_PLAYER,resource)2bcba20 returns the actual
        // resource. Empty ValidResources rejects it before feature/terrain.
        Ok(final_static_legality(
            p.resource_hash != u32::MAX,
            (p.feature_hash != u32::MAX)
                .then(|| self.definitions.valid_features.contains(&p.feature_hash)),
            self.definitions.valid_terrains.contains(&p.terrain_hash),
        ))
    }
}

fn final_static_legality(resource: bool, feature: Option<bool>, terrain: bool) -> bool {
    // Empty ValidResources rejects a resource before ValidFeature. A valid
    // feature returns immediately, while an invalid feature on a non-buildable
    // camp cannot fall through to the otherwise-valid terrain table.
    !resource && feature.unwrap_or(terrain)
}

#[cfg(test)]
mod tests {
    use super::final_static_legality;

    #[test]
    fn camp_resource_rejection_precedes_valid_feature() {
        assert!(!final_static_legality(true, Some(true), true));
        assert!(!final_static_legality(true, None, true));
    }

    #[test]
    fn feature_result_precedes_terrain_for_nonbuildable_camp() {
        assert!(final_static_legality(false, Some(true), false));
        assert!(!final_static_legality(false, Some(false), true));
        assert!(final_static_legality(false, None, true));
        assert!(!final_static_legality(false, None, false));
    }
}
impl<P: Placement> Context for Queries<'_, P> {
    type Error = Error<P::Error>;
    fn is_water(&self, state: &LoadedState, index: usize) -> Result<bool, Self::Error> {
        self.check(state)?;
        Ok(self.definitions.terrain(plot(state, index)?)?.water)
    }
    fn visible_to_major(&self, state: &LoadedState, index: usize) -> Result<bool, Self::Error> {
        self.visible(state, index, self.inputs.alive_major_ids)
    }
    fn visible_to_any(&self, state: &LoadedState, index: usize) -> Result<bool, Self::Error> {
        self.visible(state, index, self.inputs.alive_ids)
    }
    fn is_owned(&self, state: &LoadedState, index: usize) -> Result<bool, Self::Error> {
        crate::movement_placement_state::plot_owner(plot(state, index)?)
            .map(|o| o != -1)
            .map_err(|_| Error::Missing("camp plot owner"))
    }
    fn major_city(&self, state: &LoadedState, index: usize) -> Result<Option<bool>, Self::Error> {
        if !city(state, index)? {
            return Ok(None);
        }
        let owner = crate::movement_placement_state::plot_owner(plot(state, index)?)
            .map_err(|_| Error::Missing("camp city owner"))?;
        let level = self
            .inputs
            .civilization_levels
            .iter()
            .find(|r| r.0 == owner)
            .ok_or(Error::Missing("camp city civilization level"))?
            .1;
        if level != linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV") {
            return Ok(None);
        }
        Ok(Some(
            state
                .player_slot_status(owner)
                .map_err(|_| Error::Missing("camp city human control"))?
                == 3,
        ))
    }
    fn minimum_human_handicap(&self, state: &LoadedState) -> Result<i32, Self::Error> {
        let mut minimum = None;
        for &owner in self.inputs.alive_major_ids {
            if state
                .player_slot_status(owner)
                .map_err(|_| Error::Missing("camp human control"))?
                != 3
            {
                continue;
            }
            let p = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or(Error::Missing("camp human Player"))?;
            let index = self
                .definitions
                .difficulties
                .find_index(p.header.handicap_hash)
                .ok_or(Error::Missing("camp human handicap"))? as i32;
            minimum = Some(minimum.map_or(index, |old: i32| old.min(index)));
        }
        Ok(minimum.unwrap_or(-1))
    }
    fn distance(
        &self,
        state: &LoadedState,
        a: PlotCoord,
        b: PlotCoord,
    ) -> Result<i32, Self::Error> {
        let h = &state.world().map().header;
        Ok(c6_core::geometry::plot_distance(
            h.width, h.wrap_x, a.x, a.y, b.x, b.y,
        ))
    }
    fn can_have_improvement(
        &self,
        state: &LoadedState,
        index: usize,
        kind: i32,
    ) -> Result<bool, Self::Error> {
        self.can_have(state, index, kind)
    }
    fn set_camp(
        &mut self,
        state: &mut LoadedState,
        at: PlotCoord,
        kind: i32,
        owner: i32,
    ) -> Result<(), Self::Error> {
        self.placement
            .set_camp(state, at, kind, owner)
            .map_err(Error::Placement)
    }
}
