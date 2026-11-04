//! Cached native rules inputs for canonical fresh City construction.
use crate::{
    game_parameters::{IntegerParameter, Parameters},
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use c6_core::{
    city_initialization::{BuildingSupport, Definitions as CoreDefinitions},
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    movement_world::DefinitionLookup,
};

#[derive(Debug)]
pub enum Error {
    Binding,
    Definitions(unit_initialization_definitions::Error),
    Rules(crate::Error),
    Parameter(crate::game_parameters::Error),
    Missing(&'static str),
    Scalar(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "City initialization rules: {self:?}")
    }
}
impl std::error::Error for Error {}

pub struct Definitions {
    identities: InputIdentities,
    yields: Vec<u32>,
    units: Vec<u32>,
    buildings: Vec<u32>,
    districts: Vec<u32>,
    projects: Vec<u32>,
    terrains: Vec<u32>,
    resources: Vec<u32>,
    features: Vec<u32>,
    production_yield: i32,
    loyalty_start: i32,
    building_support: Vec<BuildingSupport>,
}
fn hashes(registry: &LinuxDefinitionRegistry) -> Vec<u32> {
    (0..registry.entries().len())
        .map(|i| registry.type_hash(i).expect("registry entry"))
        .collect()
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if authority.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(Error::Definitions)?;
        let registry = |table, column| collection(rules, table, column).map_err(Error::Definitions);
        let yields = registry("Yields", "YieldType")?;
        let buildings = registry("Buildings", "BuildingType")?;
        let production_yield = i32::try_from(
            yields
                .find_index(linux_type_hash(b"YIELD_PRODUCTION"))
                .ok_or(Error::Missing("YIELD_PRODUCTION"))?,
        )
        .map_err(|_| Error::Scalar("YIELD_PRODUCTION index"))?;
        // Unit_BuildingPrereq.GetBuilderSql221e480 returns SELECT rowid,
        // NumSupported, PrereqBuilding, Unit ... ORDER BY rowid (2e4346d).
        let support = rules
            .gameplay()
            .table("Unit_BuildingPrereqs")
            .ok_or(Error::Missing("Unit_BuildingPrereqs"))?;
        let mut ordered = support
            .rows()
            .map(|row| {
                Ok((
                    row.row_id()
                        .ok_or(Error::Missing("Unit_BuildingPrereqs rowid"))?,
                    row,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        ordered.sort_by_key(|(id, _)| id.0);
        let building_support = ordered
            .into_iter()
            .map(|(_, row)| {
                let value = crate::game_parameters::sql_integer(
                    row.get("NumSupported").map_err(Error::Rules)?,
                )
                .ok_or(Error::Scalar("NumSupported"))? as i16;
                let building_index = match row.get("PrereqBuilding").map_err(Error::Rules)? {
                    SqlValue::Null => None,
                    SqlValue::Text(bytes) => {
                        buildings.find_index(linux_type_hash(bytes.as_slice()))
                    }
                    _ => return Err(Error::Scalar("PrereqBuilding")),
                };
                Ok(BuildingSupport {
                    building_index,
                    num_supported: value,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            identities: state.binding().inputs.clone(),
            yields: hashes(&yields),
            buildings: hashes(&buildings),
            units: hashes(&registry("Units", "UnitType")?),
            districts: hashes(&registry("Districts", "DistrictType")?),
            projects: hashes(&registry("Projects", "ProjectType")?),
            terrains: hashes(&registry("Terrains", "TerrainType")?),
            resources: hashes(&registry("Resources", "ResourceType")?),
            features: hashes(&registry("Features", "FeatureType")?),
            production_yield,
            loyalty_start: Parameters::from_rules(rules)
                .map_err(Error::Parameter)?
                .integer(IntegerParameter::LoyaltyStart)
                .map_err(Error::Parameter)?,
            building_support,
        })
    }
    pub fn core<'a>(&'a self, state: &LoadedState) -> Result<CoreDefinitions<'a>, Error> {
        if self.identities != state.binding().inputs {
            return Err(Error::Binding);
        }
        Ok(CoreDefinitions {
            yields: &self.yields,
            units: &self.units,
            buildings: &self.buildings,
            districts: &self.districts,
            projects: &self.projects,
            terrains: &self.terrains,
            resources: &self.resources,
            features: &self.features,
            production_yield: self.production_yield,
            loyalty_start: self.loyalty_start,
            building_support: &self.building_support,
        })
    }
}
