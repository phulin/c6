//! Definition-backed Unit.GetEra used by post-combat yield scaling. This is
//! distinct from a player's current era and from the combat era-strength bonus.
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, map::UnitReference,
};
#[derive(Debug)]
pub enum Error {
    Definitions(unit_initialization_definitions::Error),
    Rule(crate::Error),
    Actor(super::super::Error),
    State(c6_core::loaded_state::Error),
    Binding,
    Missing(&'static str),
}
struct UnitEra {
    technology: Option<usize>,
    civic: Option<usize>,
    all_eras: bool,
}
pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    eras: LinuxDefinitionRegistry,
    chronology: Vec<i32>,
    references: Vec<UnitEra>,
}
fn select(technology: Option<(usize, i32)>, civic: Option<(usize, i32)>) -> Option<usize> {
    let technology = technology.filter(|v| v.1 != i32::MIN);
    let chronology = technology.map_or(i32::MIN, |v| v.1);
    civic
        .filter(|v| v.1 > chronology)
        .or(technology)
        .map(|v| v.0)
}
fn first_era(chronology: &[i32]) -> Option<usize> {
    let mut best = i32::MAX;
    let mut result = None;
    for (index, &value) in chronology.iter().enumerate() {
        if value < best {
            best = value;
            result = Some(index);
        }
    }
    result
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        resources: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if resources.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, resources)
            .map_err(Error::Definitions)?;
        let units = collection(rules, "Units", "UnitType").map_err(Error::Definitions)?;
        let eras = collection(rules, "Eras", "EraType").map_err(Error::Definitions)?;
        let era_table = rules
            .gameplay()
            .table("Eras")
            .ok_or(Error::Missing("Eras"))?;
        let chronology = eras
            .entries()
            .iter()
            .map(|entry| {
                let row = era_table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Era row"))?;
                i32::try_from(row.integer("ChronologyIndex").map_err(Error::Rule)?)
                    .map_err(|_| Error::Missing("Era chronology integer"))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let unit_table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let tags = rules
            .gameplay()
            .table("TypeTags")
            .ok_or(Error::Missing("TypeTags"))?;
        let reference = |row: &crate::Row,
                         column: &str,
                         table: &str,
                         key: &str|
         -> Result<Option<usize>, Error> {
            if matches!(row.get(column).map_err(Error::Rule)?, SqlValue::Null) {
                return Ok(None);
            }
            let name = row.text(column).map_err(Error::Rule)?;
            let values = rules
                .gameplay()
                .table(table)
                .ok_or(Error::Missing("prerequisite table"))?;
            let mut rows = values.rows().filter(|r| r.text(key).ok() == Some(name));
            let Some(prerequisite) = rows.next() else {
                return Ok(None);
            };
            if rows.next().is_some() {
                return Err(Error::Missing("duplicate prerequisite"));
            }
            if matches!(
                prerequisite.get("EraType").map_err(Error::Rule)?,
                SqlValue::Null
            ) {
                return Ok(None);
            }
            let name = prerequisite.text("EraType").map_err(Error::Rule)?;
            Ok(eras.find_index(linux_type_hash(name.as_bytes())))
        };
        let references = units
            .entries()
            .iter()
            .map(|entry| {
                let row = unit_table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Unit row"))?;
                let name = row.text("UnitType").map_err(Error::Rule)?;
                Ok(UnitEra {
                    technology: reference(&row, "PrereqTech", "Technologies", "TechnologyType")?,
                    civic: reference(&row, "PrereqCivic", "Civics", "CivicType")?,
                    all_eras: tags.rows().any(|r| {
                        r.text("Type").ok() == Some(name)
                            && r.text("Tag").ok() == Some("CLASS_ALL_ERAS")
                    }),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            units,
            eras,
            chronology,
            references,
        })
    }
    pub fn era_index(&self, state: &LoadedState, reference: UnitReference) -> Result<i32, Error> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        let unit = super::super::actor(state, reference).map_err(Error::Actor)?;
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let expected = self
            .units
            .find_index(unit.e_type_index)
            .map(|i| i as i32)
            .unwrap_or(-1);
        if expected != runtime.definition_index {
            return Err(Error::Binding);
        }
        let Some(definition) = usize::try_from(expected)
            .ok()
            .and_then(|i| self.references.get(i))
        else {
            return Ok(-1);
        };
        let value = |index: usize| (index, self.chronology[index]);
        if let Some(era) = select(
            definition.technology.map(value),
            definition.civic.map(value),
        ) {
            return Ok(era as i32);
        }
        if definition.all_eras {
            let player = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == unit.e_owner)
                .ok_or(Error::Missing("Unit era owner"))?;
            if let Some(index) = self.eras.find_index(player.eras.era_hash) {
                return Ok(index as i32);
            }
        }
        Ok(first_era(&self.chronology).map_or(-1, |v| v as i32))
    }
    pub fn chronology(&self, state: &LoadedState, reference: UnitReference) -> Result<i32, Error> {
        let index = self.era_index(state, reference)?;
        usize::try_from(index)
            .ok()
            .and_then(|i| self.chronology.get(i))
            .copied()
            .ok_or(Error::Missing("Unit.GetEra definition"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn technology_ties_and_invalid_minimum_preserve_source_precedence() {
        assert_eq!(select(Some((2, 7)), Some((3, 7))), Some(2));
        assert_eq!(select(Some((2, 7)), Some((3, 8))), Some(3));
        assert_eq!(select(Some((2, i32::MIN)), None), None);
        assert_eq!(select(None, Some((3, i32::MIN))), None);
    }
    #[test]
    fn fallback_keeps_first_strict_minimum_and_ignores_maximum_sentinel() {
        assert_eq!(first_era(&[i32::MAX, 3, 3, 4]), Some(1));
        assert_eq!(first_era(&[i32::MAX]), None);
    }
}
