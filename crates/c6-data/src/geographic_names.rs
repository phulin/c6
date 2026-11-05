//! Current Linux rules inputs for geographic discovery. Native all-row queries
//! order these collections by rowid; references order Civilizations by rowid.
use crate::{registry::linux_type_hash, unit_initialization_definitions::collection, Rules};
use c6_core::{
    geographic_names::{
        CivilizationName, LabelThresholds, NameDefinition, NameRules, TerritoryRules,
    },
    map::LocalizedText,
};
#[derive(Debug)]
pub enum Error {
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    Missing(&'static str),
    Range,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "geographic naming rules: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug)]
pub struct Definitions {
    pub volcanoes: NameRules,
    pub rivers: NameRules,
    pub territories: TerritoryRules,
    maps: Vec<(u32, LabelThresholds)>,
    /// Map collection hashes; absent primary Map definition also returns early.
    map_sizes: Vec<u32>,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let map_sizes = collection(rules, "Maps", "MapSizeType")
            .map_err(Error::Definitions)?
            .entries()
            .iter()
            .map(|e| linux_type_hash(&e.type_name))
            .collect();
        let maps = rules
            .gameplay()
            .table("Maps_XP2")
            .ok_or(Error::Missing("Maps_XP2"))?
            .rows()
            .map(|row| {
                let integer = |column| {
                    crate::game_parameters::sql_integer(row.get(column).map_err(Error::Rules)?)
                        .ok_or(Error::Range)
                };
                Ok((
                    linux_type_hash(row.text("MapSizeType").map_err(Error::Rules)?.as_bytes()),
                    LabelThresholds {
                        mountain: integer("MountainPlotCountToLabel")?,
                        desert: integer("DesertPlotCountToLabel")?,
                        lake: integer("LakePlotCountToLabel")?,
                        sea: integer("SeaPlotCountToLabel")?,
                        ocean: integer("OceanPlotCountToLabel")?,
                    },
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            volcanoes: names(
                rules,
                "NamedVolcanoes",
                "NamedVolcanoType",
                "NamedVolcanoCivilizations",
            )?,
            rivers: names(
                rules,
                "NamedRivers",
                "NamedRiverType",
                "NamedRiverCivilizations",
            )?,
            territories: TerritoryRules {
                mountains: names(
                    rules,
                    "NamedMountains",
                    "NamedMountainType",
                    "NamedMountainCivilizations",
                )?,
                deserts: names(
                    rules,
                    "NamedDeserts",
                    "NamedDesertType",
                    "NamedDesertCivilizations",
                )?,
                lakes: names(
                    rules,
                    "NamedLakes",
                    "NamedLakeType",
                    "NamedLakeCivilizations",
                )?,
                seas: names(rules, "NamedSeas", "NamedSeaType", "NamedSeaCivilizations")?,
                oceans: names(
                    rules,
                    "NamedOceans",
                    "NamedOceanType",
                    "NamedOceanCivilizations",
                )?,
            },
            maps,
            map_sizes,
        })
    }
    pub fn label_thresholds(&self, map_size_hash: u32) -> Option<LabelThresholds> {
        self.map_sizes.contains(&map_size_hash).then_some(())?;
        self.maps
            .iter()
            .rev()
            .find(|(hash, _)| *hash == map_size_hash)
            .map(|(_, v)| *v)
    }
}
fn names(
    rules: &Rules,
    table_name: &'static str,
    type_column: &'static str,
    associations_name: &'static str,
) -> Result<NameRules, Error> {
    let registry = collection(rules, table_name, type_column).map_err(Error::Definitions)?;
    let civilizations =
        collection(rules, "Civilizations", "CivilizationType").map_err(Error::Definitions)?;
    let table = rules
        .gameplay()
        .table(table_name)
        .ok_or(Error::Missing(table_name))?;
    let association_table = rules
        .gameplay()
        .table(associations_name)
        .ok_or(Error::Missing(associations_name))?;
    let mut civilization_names = vec![];
    for row in association_table.rows() {
        let civ_name = row.text("CivilizationType").map_err(Error::Rules)?;
        let name = row.text(type_column).map_err(Error::Rules)?;
        // The scalar SQL reference selects by type text and ORDER BY rowid LIMIT 1.
        let resolved = registry
            .entries()
            .iter()
            .find(|e| e.type_name == name.as_bytes());
        let civilization = civilizations
            .entries()
            .iter()
            .find(|e| e.type_name == civ_name.as_bytes());
        civilization_names.push(CivilizationName {
            civilization: civilization.map(|e| linux_type_hash(&e.type_name)),
            name_hash: resolved.map(|e| linux_type_hash(&e.type_name)),
        });
    }
    let mut definitions = vec![];
    for (index, entry) in registry.entries().iter().enumerate() {
        let row = table
            .row_by_id(entry.database_rowid)
            .ok_or(Error::Missing("name definition row"))?;
        let mut references = vec![];
        // Collection reference SQL joins by text and orders by Civilization rowid,
        // not the order of Named*Civilizations associations.
        for civilization in civilizations.entries() {
            for association in association_table.rows() {
                if association
                    .text(type_column)
                    .map_err(Error::Rules)?
                    .as_bytes()
                    == entry.type_name
                    && association
                        .text("CivilizationType")
                        .map_err(Error::Rules)?
                        .as_bytes()
                        == civilization.type_name
                {
                    references.push(linux_type_hash(&civilization.type_name));
                }
            }
        }
        definitions.push(NameDefinition {
            hash: linux_type_hash(&entry.type_name),
            collection_index: i32::try_from(index).map_err(|_| Error::Range)?,
            name: LocalizedText {
                base: row.text("Name").map_err(Error::Rules)?.as_bytes().to_vec(),
                arguments: vec![],
                named_arguments: vec![],
            },
            civilizations: references,
        });
    }
    Ok(NameRules {
        definitions,
        civilization_names,
    })
}
