//! Source-derived definition orders for the bounded current Linux Unit reader.
//! The registered activated rules snapshot is reused only after actual static
//! resource comparison with the existing loaded-action capture. This does not
//! initialize nested Unit components or establish native action/turn parity.
use crate::{
    loaded_actions::resources::Bound,
    registry::{DefinitionEntry, LinuxDefinitionRegistry},
    Rules, RulesProfile, RulesRowId,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const RULES_REPORT_SHA256: &str =
    "4b85ee47e0e1d031a20aec2f96fbe99d2cda23861ad646ed48073fc3fa2614e1";
#[derive(Debug)]
pub enum Error {
    Profile,
    Report,
    Resources(crate::loaded_actions::resources::Error),
    Rules(crate::Error),
    MissingTable(&'static str),
    MissingRowId(&'static str),
    UnsupportedRowId(i64),
    TypeName(&'static str),
    RowOrder,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Unit initialization definitions: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Four collections missing from the existing Unit/operation capture. Entries
/// preserve SQL row identity separately from the sequential native index. Native
/// duplicate-hash precedence and direct-index priority come from the existing
/// source-verified registry, not SQLite IDs or save-pair order.
pub struct Definitions {
    abilities: LinuxDefinitionRegistry,
    terrains: LinuxDefinitionRegistry,
    features: LinuxDefinitionRegistry,
    great_person_classes: LinuxDefinitionRegistry,
}
impl Definitions {
    /// Validate the registered rules snapshot against actual bound resources.
    /// Other typed definition builders may reuse this gate without duplicating
    /// report parsers or requiring equal observer-profile hashes.
    pub fn validate_resources(
        rules: &Rules,
        bound: &Bound<'_>,
        rules_runtime: &[u8],
    ) -> Result<(), Error> {
        if rules.profile() != RulesProfile::LinuxXp2ActivatedDiagnostic {
            return Err(Error::Profile);
        }
        // This closed report is the original provenance of the registered DBs.
        // A later observer's whole configuration SHA need not equal this one.
        if format!("{:x}", Sha256::digest(rules_runtime)) != RULES_REPORT_SHA256 {
            return Err(Error::Report);
        }
        let report: Value = serde_json::from_slice(rules_runtime).map_err(|_| Error::Report)?;
        if report["configuration_sha256"].as_str()
            != Some(rules.source().native_configuration_sha256.as_str())
        {
            return Err(Error::Report);
        }
        bound
            .validate_static_runtime(rules_runtime)
            .map_err(Error::Resources)
    }
    pub fn from_rules(
        rules: &Rules,
        bound: &Bound<'_>,
        rules_runtime: &[u8],
    ) -> Result<Self, Error> {
        Self::validate_resources(rules, bound, rules_runtime)?;
        Ok(Self {
            abilities: collection(rules, "UnitAbilities", "UnitAbilityType")?,
            terrains: collection(rules, "Terrains", "TerrainType")?,
            features: collection(rules, "Features", "FeatureType")?,
            great_person_classes: collection(rules, "GreatPersonClasses", "GreatPersonClassType")?,
        })
    }
    pub fn abilities(&self) -> &LinuxDefinitionRegistry {
        &self.abilities
    }
    pub fn terrains(&self) -> &LinuxDefinitionRegistry {
        &self.terrains
    }
    pub fn features(&self) -> &LinuxDefinitionRegistry {
        &self.features
    }
    pub fn great_person_classes(&self) -> &LinuxDefinitionRegistry {
        &self.great_person_classes
    }
}

/// Project a table only after its caller proves the native all-row rowid query.
/// This helper does not establish that contract for additional collections.
pub(crate) fn collection(
    rules: &Rules,
    table: &'static str,
    column: &'static str,
) -> Result<LinuxDefinitionRegistry, Error> {
    let table_data = rules
        .gameplay()
        .table(table)
        .ok_or(Error::MissingTable(table))?;
    let mut entries = Vec::with_capacity(table_data.rows().len());
    for row in table_data.rows() {
        let row_id = row.row_id().ok_or(Error::MissingRowId(table))?;
        let name = row.text(column).map_err(Error::Rules)?;
        entries.push(entry(row_id, name.as_bytes(), table)?);
    }
    ordered(entries)
}
fn entry(row_id: RulesRowId, name: &[u8], table: &'static str) -> Result<DefinitionEntry, Error> {
    // Native GetInt stores signed i32. This profile has no narrowing extremes;
    // reject rather than silently inventing their unexecuted behavior.
    i32::try_from(row_id.0).map_err(|_| Error::UnsupportedRowId(row_id.0))?;
    if name.contains(&0) {
        return Err(Error::TypeName(table));
    }
    Ok(DefinitionEntry {
        database_rowid: row_id,
        type_name: name.to_vec(),
    })
}
fn ordered(entries: Vec<DefinitionEntry>) -> Result<LinuxDefinitionRegistry, Error> {
    if entries.len() > i32::MAX as usize
        || entries
            .windows(2)
            .any(|p| p[0].database_rowid.0 >= p[1].database_rowid.0)
    {
        return Err(Error::RowOrder);
    }
    Ok(LinuxDefinitionRegistry::from_native_order(entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::linux_type_hash;
    #[test]
    fn zero_gapped_and_negative_rows_preserve_sequential_native_indexes() {
        let entries = [(-2, "A"), (0, "B"), (12, "A")]
            .into_iter()
            .map(|(id, name)| entry(RulesRowId(id), name.as_bytes(), "synthetic").unwrap())
            .collect();
        let registry = ordered(entries).unwrap();
        assert_eq!(
            registry
                .entries()
                .iter()
                .map(|e| e.database_rowid.0)
                .collect::<Vec<_>>(),
            [-2, 0, 12]
        );
        assert_eq!(registry.find_index(linux_type_hash(b"A")), Some(2));
        assert_eq!(registry.find_index(0), Some(0));
        assert_eq!(registry.find_index(linux_type_hash(b"absent")), None);
    }
    #[test]
    fn unsupported_name_rowid_and_order_are_errors() {
        assert!(matches!(
            entry(RulesRowId(0), b"A\0B", "synthetic"),
            Err(Error::TypeName(_))
        ));
        assert!(matches!(
            entry(RulesRowId(i64::MAX), b"A", "synthetic"),
            Err(Error::UnsupportedRowId(_))
        ));
        let first = entry(RulesRowId(1), b"A", "synthetic").unwrap();
        assert!(matches!(
            ordered(vec![first.clone(), first]),
            Err(Error::RowOrder)
        ));
    }
}
