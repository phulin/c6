//! Activated immutable definitions for the existing Linux plot-yield kernel.
use super::Error;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Row, Rules, SqlValue,
};
use c6_core::{
    improvement_yield as iy,
    movement_world::DefinitionLookup,
    plot_yield as py,
    yields::{BonusYieldChange, ImprovementDefinition, NativeDefinitionIndex},
};
struct Feature {
    natural_wonder: bool,
    double_adjacent: bool,
    adjacent: Vec<py::YieldChange>,
}
struct District {
    citizens: Vec<Option<py::NamedYieldChange>>,
    appeal: Vec<py::AppealYieldChange>,
}
struct Improvement {
    property: py::Improvement,
    base: Vec<iy::ImprovementBaseYield>,
    adjacency: Vec<NativeDefinitionIndex>,
    bonuses: Vec<Option<BonusYieldChange>>,
}
pub struct Definitions {
    pub(super) yields: LinuxDefinitionRegistry,
    pub(super) terrains: LinuxDefinitionRegistry,
    pub(super) features: LinuxDefinitionRegistry,
    pub(super) improvements: LinuxDefinitionRegistry,
    pub(super) resources: LinuxDefinitionRegistry,
    pub(super) districts: LinuxDefinitionRegistry,
    pub(super) buildings: LinuxDefinitionRegistry,
    pub(super) outside_improvements: Vec<u32>,
    feature: Vec<Feature>,
    improvement: Vec<Improvement>,
    resource: Vec<Vec<Option<py::YieldChange>>>,
    district: Vec<District>,
    building: Vec<Vec<Option<py::NamedYieldChange>>>,
    terrain_changes: Vec<py::HashYieldChange>,
    feature_changes: Vec<py::HashYieldChange>,
    types: py::YieldTypes,
    food_min: i32,
    production_min: i32,
    drought_loss: i32,
}
fn integer(row: Row<'_>, column: &'static str) -> Result<i32, Error> {
    crate::game_parameters::sql_integer(row.get(column).map_err(Error::Rules)?)
        .ok_or(Error::Unsupported(column))
}
fn hash(row: Row<'_>, column: &'static str) -> Result<u32, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Null => Ok(u32::MAX),
        SqlValue::Text(name) => Ok(linux_type_hash(name)),
        _ => Err(Error::Unsupported(column)),
    }
}
fn index(lookup: &LinuxDefinitionRegistry, hash: u32) -> Option<NativeDefinitionIndex> {
    lookup
        .find_index(hash)
        .and_then(|i| NativeDefinitionIndex::new(i as u32, lookup.count() as u32))
}
impl Definitions {
    /// Actual native Yield collection order used by citizen scoring.
    pub fn yield_indices(&self) -> std::ops::Range<i32> {
        0..self.yields.count() as i32
    }
    /// Current native Yield collection identity for canonical hash-keyed arrays.
    pub fn yield_hash(&self, index: i32) -> Option<u32> {
        self.yields.type_hash(index as u32 as usize)
    }

    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let registry = |name, col| collection(rules, name, col).map_err(Error::Definitions);
        let yields = registry("Yields", "YieldType")?;
        let terrains = registry("Terrains", "TerrainType")?;
        let features = registry("Features", "FeatureType")?;
        let improvements = registry("Improvements", "ImprovementType")?;
        let resources = registry("Resources", "ResourceType")?;
        let districts = registry("Districts", "DistrictType")?;
        let buildings = registry("Buildings", "BuildingType")?;
        let techs = registry("Technologies", "TechnologyType")?;
        let civics = registry("Civics", "CivicType")?;
        let adjacency = registry("Adjacency_YieldChanges", "ID")?;
        let named = |table_name,
                     source_column,
                     source_hash|
         -> Result<Vec<Option<py::NamedYieldChange>>, Error> {
            let mut out = Vec::new();
            for row in table(table_name)?.rows() {
                if hash(row, source_column)? == source_hash {
                    out.push(Some(py::NamedYieldChange {
                        yield_hash: hash(row, "YieldType")?,
                        change: integer(row, "YieldChange")?,
                    }));
                }
            }
            Ok(out)
        };
        let changes = |table_name, source_column| -> Result<Vec<py::HashYieldChange>, Error> {
            table(table_name)?
                .rows()
                .map(|row| {
                    Ok(py::HashYieldChange {
                        source_hash: hash(row, source_column)?,
                        yield_hash: hash(row, "YieldType")?,
                        change: integer(row, "YieldChange")?,
                    })
                })
                .collect()
        };
        let mut feature = Vec::new();
        for entry in features.entries() {
            let row = table("Features")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("Feature row"))?;
            let mut adjacent = Vec::new();
            for change in table("Feature_AdjacentYields")?.rows() {
                if hash(change, "FeatureType")? == linux_type_hash(&entry.type_name) {
                    adjacent.push(py::YieldChange {
                        yield_index: index(&yields, hash(change, "YieldType")?),
                        change: integer(change, "YieldChange")?,
                    });
                }
            }
            feature.push(Feature {
                natural_wonder: integer(row, "NaturalWonder")? != 0,
                double_adjacent: integer(row, "DoubleAdjacentTerrainYield")? != 0,
                adjacent,
            });
        }
        let mut district = Vec::new();
        for entry in districts.entries() {
            let source = linux_type_hash(&entry.type_name);
            let mut appeal = Vec::new();
            for row in table("Adjacent_AppealYieldChanges")?.rows() {
                if hash(row, "DistrictType")? == source {
                    appeal.push(py::AppealYieldChange {
                        yield_index: index(&yields, hash(row, "YieldType")?),
                        unimproved: integer(row, "Unimproved")? != 0,
                        building_hash: hash(row, "BuildingType")?,
                        minimum: integer(row, "MinimumValue")?,
                        maximum: integer(row, "MaximumValue")?,
                        change: integer(row, "YieldChange")?,
                    });
                }
            }
            district.push(District {
                citizens: named("District_CitizenYieldChanges", "DistrictType", source)?,
                appeal,
            });
        }
        let building = buildings
            .entries()
            .iter()
            .map(|entry| {
                named(
                    "Building_CitizenYieldChanges",
                    "BuildingType",
                    linux_type_hash(&entry.type_name),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut resource = Vec::new();
        for entry in resources.entries() {
            let mut rows = Vec::new();
            for row in table("Resource_YieldChanges")?.rows() {
                if hash(row, "ResourceType")? == linux_type_hash(&entry.type_name) {
                    rows.push(Some(py::YieldChange {
                        yield_index: index(&yields, hash(row, "YieldType")?),
                        change: integer(row, "YieldChange")?,
                    }));
                }
            }
            resource.push(rows);
        }
        let mut improvement = Vec::new();
        for entry in improvements.entries() {
            let source = linux_type_hash(&entry.type_name);
            let row = table("Improvements")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("Improvement row"))?;
            let mut base = Vec::new();
            for change in table("Improvement_YieldChanges")?.rows() {
                if hash(change, "ImprovementType")? == source {
                    if let Some(yield_index) = index(&yields, hash(change, "YieldType")?) {
                        base.push(iy::ImprovementBaseYield {
                            yield_index,
                            change: integer(change, "YieldChange")?,
                        });
                    }
                }
            }
            let mut references = Vec::new();
            for change in table("Improvement_Adjacencies")?.rows() {
                if hash(change, "ImprovementType")? == source {
                    if let Some(reference) = index(&adjacency, hash(change, "YieldChangeId")?) {
                        references.push(reference);
                    }
                }
            }
            let mut bonuses = Vec::new();
            for change in table("Improvement_BonusYieldChanges")?.rows() {
                if hash(change, "ImprovementType")? == source {
                    bonuses.push(Some(BonusYieldChange {
                        yield_index: index(&yields, hash(change, "YieldType")?),
                        prerequisite_technology: index(&techs, hash(change, "PrereqTech")?),
                        prerequisite_civic: index(&civics, hash(change, "PrereqCivic")?),
                        bonus_yield_change: integer(change, "BonusYieldChange")?,
                    }));
                }
            }
            improvement.push(Improvement {
                property: py::Improvement {
                    appeal_yield_hash: hash(row, "YieldFromAppeal")?,
                    appeal_percent: integer(row, "YieldFromAppealPercent")?,
                },
                base,
                adjacency: references,
                bonuses,
            });
        }
        let parameter = |name: &'static str| -> Result<i32, Error> {
            let row = table("GlobalParameters")?
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .ok_or(Error::Unsupported(name))?;
            integer(row, "Value")
        };
        let native_yield = |name: &[u8]| {
            yields
                .find_index(linux_type_hash(name))
                .map(|i| i as i32)
                .ok_or(Error::Unsupported("native Yield definition"))
        };
        let types = py::YieldTypes {
            food: native_yield(b"YIELD_FOOD")?,
            production: native_yield(b"YIELD_PRODUCTION")?,
            gold: native_yield(b"YIELD_GOLD")?,
            science: native_yield(b"YIELD_SCIENCE")?,
            culture: native_yield(b"YIELD_CULTURE")?,
            faith: native_yield(b"YIELD_FAITH")?,
        };
        Ok(Self {
            terrain_changes: changes("Terrain_YieldChanges", "TerrainType")?,
            feature_changes: changes("Feature_YieldChanges", "FeatureType")?,
            food_min: parameter("YIELD_FOOD_CITY_TERRAIN_REPLACE")?,
            production_min: parameter("YIELD_PRODUCTION_CITY_TERRAIN_REPLACE")?,
            drought_loss: parameter("DROUGHT_FOOD_LOSS_PER_TILE")?,
            yields,
            terrains,
            features,
            improvements,
            resources,
            districts,
            buildings,
            outside_improvements: table("Improvement_YieldsOutsideTerritories")?
                .rows()
                .map(|row| hash(row, "ImprovementType"))
                .collect::<Result<Vec<_>, _>>()?,
            feature,
            improvement,
            resource,
            district,
            building,
            types,
        })
    }
    pub fn with_rules<T>(
        &self,
        query: impl FnOnce(&py::Rules<'_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let features: Vec<_> = self
            .feature
            .iter()
            .map(|f| py::Feature {
                natural_wonder: f.natural_wonder,
                double_adjacent_terrain: f.double_adjacent,
                adjacent_yields: Some(&f.adjacent),
            })
            .collect();
        let resources: Vec<_> = self
            .resource
            .iter()
            .map(|r| py::Resource { yields: Some(r) })
            .collect();
        let districts: Vec<_> = self
            .district
            .iter()
            .map(|d| py::District {
                citizen_yields: Some(&d.citizens),
                adjacent_appeal_yields: Some(&d.appeal),
            })
            .collect();
        let buildings: Vec<_> = self
            .building
            .iter()
            .map(|b| py::Building {
                citizen_yields: Some(b),
            })
            .collect();
        let improvements: Vec<_> = self
            .improvement
            .iter()
            .map(|i| py::Improvement {
                appeal_yield_hash: i.property.appeal_yield_hash,
                appeal_percent: i.property.appeal_percent,
            })
            .collect();
        let base: Vec<_> = self
            .improvement
            .iter()
            .enumerate()
            .map(|(n, i)| iy::ImprovementYieldDefinition {
                type_hash: self.improvements.type_hash(n).unwrap(),
                base: Some(&i.base),
                adjacency: Some(&i.adjacency),
            })
            .collect();
        let bonuses: Vec<_> = self
            .improvement
            .iter()
            .enumerate()
            .map(|(n, i)| ImprovementDefinition {
                type_hash: self.improvements.type_hash(n).unwrap(),
                bonus_yield_changes: Some(&i.bonuses),
            })
            .collect();
        let yields = vec![(); self.yields.count()];
        let terrains = vec![(); self.terrains.count()];
        let shape = |_| Error::Unsupported("plot yield definition shape");
        let rules = py::Rules {
            yield_types: self.types,
            yields: py::Definitions::new(&self.yields, &yields).map_err(shape)?,
            terrains: py::Definitions::new(&self.terrains, &terrains).map_err(shape)?,
            features: py::Definitions::new(&self.features, &features).map_err(shape)?,
            improvements: py::Definitions::new(&self.improvements, &improvements).map_err(shape)?,
            resources: py::Definitions::new(&self.resources, &resources).map_err(shape)?,
            districts: py::Definitions::new(&self.districts, &districts).map_err(shape)?,
            buildings: py::Definitions::new(&self.buildings, &buildings).map_err(shape)?,
            terrain_changes: Some(&self.terrain_changes),
            feature_changes: Some(&self.feature_changes),
            improvement_yields: Some(
                iy::OrderedImprovementYieldDefinitions::new(&self.improvements, &base, &bonuses)
                    .map_err(|_| Error::Unsupported("improvement yield definitions"))?,
            ),
            city_food_minimum: Some(self.food_min),
            city_production_minimum: Some(self.production_min),
            drought_food_loss: Some(self.drought_loss),
        };
        query(&rules)
    }
}
