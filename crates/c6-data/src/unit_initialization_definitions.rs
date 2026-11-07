//! Source-derived definition orders for the bounded current Linux Unit reader.
//! Rules are reused only after checked immutable resource binding. This does not
//! initialize nested Unit components or establish native action/turn parity.
use crate::{
    registry::{DefinitionEntry, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    Rules, RulesRowId,
};
#[derive(Debug)]
pub enum Error {
    Resources(crate::resource_binding::Error),
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
    hills: Vec<bool>,
    features: LinuxDefinitionRegistry,
    great_person_classes: LinuxDefinitionRegistry,
}
impl Definitions {
    /// Validate the registered rules snapshot against actual bound resources.
    /// Other typed definition builders may reuse this gate without duplicating
    /// report parsers or requiring equal observer-profile hashes.
    pub fn validate_resources(rules: &Rules, bound: &ResourceAuthority<'_>) -> Result<(), Error> {
        bound.validate_rules(rules).map_err(Error::Resources)
    }
    pub fn from_rules(rules: &Rules, bound: &ResourceAuthority<'_>) -> Result<Self, Error> {
        Self::validate_resources(rules, bound)?;
        Ok(Self {
            abilities: collection(rules, "UnitAbilities", "UnitAbilityType")?,
            terrains: collection(rules, "Terrains", "TerrainType")?,
            hills: rules
                .gameplay()
                .table("Terrains")
                .ok_or(Error::MissingTable("Terrains"))?
                .rows()
                .map(|row| row.integer("Hills").map(|v| v != 0).map_err(Error::Rules))
                .collect::<Result<_, _>>()?,
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
    /// Current native terrain-definition order and source IsHills flag.
    pub fn hills(&self) -> &[bool] {
        &self.hills
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
