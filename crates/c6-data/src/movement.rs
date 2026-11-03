//! Bridge actual native movement registry exports to the independent core.
//! This never infers native indices/properties from SQL rows. All values still
//! require original module/configuration/input provenance at the import boundary.
use crate::registry::{linux_type_hash, LinuxDefinitionHashRegistry, LinuxDefinitionRegistry};
use c6_core::movement_world::{
    DefinitionLookup, DefinitionTable, MovementDefinitions, TerrainProperties,
};
use serde::{Deserialize, Serialize};

impl DefinitionLookup for LinuxDefinitionRegistry {
    fn count(&self) -> usize {
        self.entries().len()
    }
    fn find_index(&self, key: u32) -> Option<usize> {
        self.find_index(key)
    }
    fn type_hash(&self, index: usize) -> Option<u32> {
        self.entries()
            .get(index)
            .map(|e| linux_type_hash(&e.type_name))
    }
}
impl DefinitionLookup for LinuxDefinitionHashRegistry {
    fn count(&self) -> usize {
        self.hashes().len()
    }
    fn find_index(&self, key: u32) -> Option<usize> {
        self.find_index(key)
    }
    fn type_hash(&self, index: usize) -> Option<u32> {
        self.hashes().get(index).copied()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryKind {
    Terrain,
    Feature,
    Improvement,
    Route,
    District,
}
impl RegistryKind {
    fn position(self) -> usize {
        match self {
            Self::Terrain => 0,
            Self::Feature => 1,
            Self::Improvement => 2,
            Self::Route => 3,
            Self::District => 4,
        }
    }
}
/// Current native registry row, with optional future original IsWater export.
/// Missing fields remain missing; serde never supplies gameplay defaults.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry {
    pub query: u32,
    pub index: u32,
    pub hash: u32,
    pub integer_cost: Option<i32>,
    pub float_bits: Option<u32>,
    pub free_embark: Option<bool>,
    pub is_water: Option<bool>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryExport {
    pub kind: String,
    pub name: RegistryKind,
    pub count: u32,
    pub entries: Vec<RegistryEntry>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Property {
    IntegerCost,
    FloatBits,
    FreeEmbark,
    IsWater,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    RegistryCount,
    DuplicateRegistry(RegistryKind),
    MissingRegistry(RegistryKind),
    UnsupportedRecord,
    NativeOrder {
        kind: RegistryKind,
        position: usize,
    },
    MissingProperty {
        kind: RegistryKind,
        index: u32,
        property: Property,
    },
    CoreTable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported native movement definitions: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Structurally complete observed collections. cb6113b supplies these hashes and
/// costs, but omits Terrain IsWater. Such inputs support Unit array remapping;
/// full plot resolution is unavailable until every required property is observed.
#[derive(Clone, Debug)]
pub struct NativeMovementExports {
    registries: [LinuxDefinitionHashRegistry; 5],
    entries: [Vec<RegistryEntry>; 5],
}
impl NativeMovementExports {
    pub fn from_records(records: &[RegistryExport]) -> Result<Self, Error> {
        let mut selected: [Option<&RegistryExport>; 5] = [None; 5];
        for record in records {
            if record.kind != "registry" {
                return Err(Error::UnsupportedRecord);
            }
            let at = record.name.position();
            if selected[at].replace(record).is_some() {
                return Err(Error::DuplicateRegistry(record.name));
            }
            if record.count > 65536 || record.entries.len() != record.count as usize {
                return Err(Error::RegistryCount);
            }
            for (position, e) in record.entries.iter().enumerate() {
                if e.query as usize != position || e.index as usize != position {
                    return Err(Error::NativeOrder {
                        kind: record.name,
                        position,
                    });
                }
            }
        }
        for kind in [
            RegistryKind::Terrain,
            RegistryKind::Feature,
            RegistryKind::Improvement,
            RegistryKind::Route,
            RegistryKind::District,
        ] {
            if selected[kind.position()].is_none() {
                return Err(Error::MissingRegistry(kind));
            }
        }
        // All five slots were checked above. No database row IDs or names enter.
        let entries = selected.map(|r| r.expect("checked registry slot").entries.clone());
        let registries = std::array::from_fn(|i| {
            LinuxDefinitionHashRegistry::from_native_order(
                entries[i].iter().map(|e| e.hash).collect(),
            )
        });
        Ok(Self {
            registries,
            entries,
        })
    }
    pub fn registry(&self, kind: RegistryKind) -> &LinuxDefinitionHashRegistry {
        &self.registries[kind.position()]
    }
    pub fn complete(&self) -> Result<NativeMovementDefinitions, Error> {
        let missing = |kind, index, property| Error::MissingProperty {
            kind,
            index,
            property,
        };
        let terrains = self.entries[0]
            .iter()
            .map(|e| {
                Ok(TerrainProperties {
                    movement_cost: e.integer_cost.ok_or_else(|| {
                        missing(RegistryKind::Terrain, e.index, Property::IntegerCost)
                    })?,
                    is_water: e.is_water.ok_or_else(|| {
                        missing(RegistryKind::Terrain, e.index, Property::IsWater)
                    })?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let integer = |kind: RegistryKind| {
            self.entries[kind.position()]
                .iter()
                .map(|e| {
                    e.integer_cost
                        .ok_or_else(|| missing(kind, e.index, Property::IntegerCost))
                })
                .collect::<Result<Vec<_>, Error>>()
        };
        let features = integer(RegistryKind::Feature)?;
        let improvements = integer(RegistryKind::Improvement)?;
        let routes = self.entries[3]
            .iter()
            .map(|e| {
                e.float_bits
                    .ok_or_else(|| missing(RegistryKind::Route, e.index, Property::FloatBits))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let districts = self.entries[4]
            .iter()
            .map(|e| {
                e.free_embark
                    .ok_or_else(|| missing(RegistryKind::District, e.index, Property::FreeEmbark))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(NativeMovementDefinitions {
            exports: self.clone(),
            terrains,
            features,
            improvements,
            routes,
            districts,
        })
    }
}
/// Build once per native rules identity. Per-query views borrow these arrays.
pub struct NativeMovementDefinitions {
    exports: NativeMovementExports,
    terrains: Vec<TerrainProperties>,
    features: Vec<i32>,
    improvements: Vec<i32>,
    routes: Vec<u32>,
    districts: Vec<bool>,
}
impl NativeMovementDefinitions {
    pub fn as_core(&self) -> Result<MovementDefinitions<'_>, Error> {
        let table_error = |_| Error::CoreTable;
        Ok(MovementDefinitions {
            terrains: DefinitionTable::new(
                self.exports.registry(RegistryKind::Terrain),
                &self.terrains,
            )
            .map_err(table_error)?,
            features: DefinitionTable::new(
                self.exports.registry(RegistryKind::Feature),
                &self.features,
            )
            .map_err(table_error)?,
            improvements: DefinitionTable::new(
                self.exports.registry(RegistryKind::Improvement),
                &self.improvements,
            )
            .map_err(table_error)?,
            routes: DefinitionTable::new(self.exports.registry(RegistryKind::Route), &self.routes)
                .map_err(table_error)?,
            districts: DefinitionTable::new(
                self.exports.registry(RegistryKind::District),
                &self.districts,
            )
            .map_err(table_error)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source_only_records() -> Vec<RegistryExport> {
        [
            RegistryKind::Terrain,
            RegistryKind::Feature,
            RegistryKind::Improvement,
            RegistryKind::Route,
            RegistryKind::District,
        ]
        .into_iter()
        .map(|name| RegistryExport {
            kind: "registry".into(),
            name,
            count: 2,
            entries: (0..2)
                .map(|index| RegistryEntry {
                    query: index,
                    index,
                    hash: if index == 0 { 900 } else { 700 },
                    integer_cost: Some(index as i32 - 1),
                    float_bits: Some(0x7fc0_1234),
                    free_embark: Some(index == 1),
                    is_water: Some(index == 1),
                })
                .collect(),
        })
        .collect()
    }
    #[test]
    fn source_only_native_order_properties_and_raw_float_bits() {
        let input = NativeMovementExports::from_records(&source_only_records()).unwrap();
        let owned = input.complete().unwrap();
        let core = owned.as_core().unwrap();
        assert_eq!(core.terrains.plot_index(700).unwrap(), 1);
        assert_eq!(core.terrains.at(1).unwrap().movement_cost, 0);
        assert!(core.terrains.at(1).unwrap().is_water);
        assert_eq!(core.routes.at(0), Some(&0x7fc0_1234));
        assert_eq!(core.districts.at(1), Some(&true));
    }
    #[test]
    fn source_only_missing_water_is_typed_and_order_is_not_sql_order() {
        let mut records = source_only_records();
        records[0].entries[0].is_water = None;
        let input = NativeMovementExports::from_records(&records).unwrap();
        assert!(matches!(
            input.complete(),
            Err(Error::MissingProperty {
                kind: RegistryKind::Terrain,
                index: 0,
                property: Property::IsWater
            })
        ));
        assert_eq!(
            input.registry(RegistryKind::Terrain).find_index(700),
            Some(1)
        );
        records[1].entries.swap(0, 1);
        assert!(matches!(
            NativeMovementExports::from_records(&records),
            Err(Error::NativeOrder {
                kind: RegistryKind::Feature,
                position: 0
            })
        ));
    }
    #[test]
    fn source_only_hash_lookup_matches_named_registry_contract() {
        let names = [b"TERRAIN_A".as_slice(), b"TERRAIN_B", b"TERRAIN_A"];
        let named = LinuxDefinitionRegistry::from_native_order(
            names
                .iter()
                .enumerate()
                .map(|(i, n)| crate::registry::DefinitionEntry {
                    database_rowid: crate::RulesRowId(100 - i as i64),
                    type_name: n.to_vec(),
                })
                .collect(),
        );
        let hashed = LinuxDefinitionHashRegistry::from_native_order(
            names.iter().map(|n| linux_type_hash(n)).collect(),
        );
        for key in [
            0,
            1,
            2,
            3,
            u32::MAX,
            linux_type_hash(names[0]),
            linux_type_hash(names[1]),
        ] {
            assert_eq!(named.find_index(key), hashed.find_index(key));
        }
        assert_eq!(hashed.find_index(linux_type_hash(names[0])), Some(2));
    }
}
