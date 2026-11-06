//! Current Linux DiplomaticVisibilitySource name-selection properties. The
//! source's activation flags (including Trader) do not select citizen genders.
use super::Error;
use crate::{
    registry::linux_type_hash, resource_binding::ResourceAuthority,
    unit_initialization_definitions, Rules, SqlValue,
};

pub struct Definitions {
    pub(super) sources: Vec<Source>,
    pub(super) binding: c6_core::core_player::initialization::WireBinding,
}
pub(super) struct Source {
    pub hash: u32,
    pub spy: bool,
    pub dynamic: bool,
}
impl Definitions {
    pub fn from_rules(rules: &Rules, bound: &ResourceAuthority<'_>) -> Result<Self, Error> {
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(Error::Definitions)?;
        let registry = unit_initialization_definitions::collection(
            rules,
            "DiplomaticVisibilitySources",
            "VisibilitySourceType",
        )
        .map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("DiplomaticVisibilitySources")
            .ok_or(Error::Missing("DiplomaticVisibilitySources"))?;
        let mut sources = Vec::new();
        for entry in registry.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("visibility source row"))?;
            let spy = row.integer("Spy").map_err(Error::Rules)? != 0;
            let reference = |column| match row.get(column).map_err(Error::Rules)? {
                SqlValue::Null => Ok(false),
                SqlValue::Text(_) => Ok(true),
                _ => Err(Error::Missing("visibility source reference")),
            };
            sources.push(Source {
                hash: linux_type_hash(&entry.type_name),
                spy,
                dynamic: reference("PrereqTech")? || reference("TraitType")?,
            });
        }
        Ok(Self {
            sources,
            binding: bound.destination_binding().clone(),
        })
    }
}
