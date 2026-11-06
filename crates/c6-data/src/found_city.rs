//! Current admitted definitions for Linux FoundCity and placement gates.
//! This does not construct a City or bypass Manager/script legality.
pub mod citizens;
pub mod city_created_callbacks;
pub mod cultural_identity;
pub mod legality;
pub mod names;
pub mod placement;
pub mod route_callbacks;
pub mod routes;
pub mod starting_era;

use crate::{
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{collection, Definitions as Initialization},
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState,
    movement_world::DefinitionLookup,
};

#[derive(Debug)]
pub enum Error {
    Resources(crate::unit_initialization_definitions::Error),
    Rules(crate::Error),
    Binding,
    Missing(&'static str),
    Boolean(&'static str),
    NativeUnitOrder,
    Player(i32),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FoundCity definitions: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Feature {
    pub removable: bool,
    pub settlement: bool,
}
pub struct Definitions {
    binding: WireBinding,
    found: Vec<bool>,
    features: LinuxDefinitionRegistry,
    feature_flags: Vec<Feature>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        Initialization::validate_resources(rules, bound).map_err(Error::Resources)?;
        if bound.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        let units = collection(rules, "Units", "UnitType").map_err(Error::Resources)?;
        let native = bound
            .unit_definitions()
            .map_err(|_| Error::NativeUnitOrder)?;
        if native.lookup.count() != units.count()
            || (0..units.count()).any(|i| native.lookup.type_hash(i) != units.type_hash(i))
        {
            return Err(Error::NativeUnitOrder);
        }
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let found = units
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Units rowid"))?;
                boolean(row.get("FoundCity").map_err(Error::Rules)?, "FoundCity")
            })
            .collect::<Result<_, _>>()?;
        let features = collection(rules, "Features", "FeatureType").map_err(Error::Resources)?;
        let table = rules
            .gameplay()
            .table("Features")
            .ok_or(Error::Missing("Features"))?;
        let feature_flags = features
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Features rowid"))?;
                Ok(Feature {
                    removable: boolean(row.get("Removable").map_err(Error::Rules)?, "Removable")?,
                    settlement: boolean(
                        row.get("Settlement").map_err(Error::Rules)?,
                        "Settlement",
                    )?,
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            found,
            features,
            feature_flags,
        })
    }
    fn check(&self, state: &LoadedState) -> Result<(), Error> {
        if state.binding() == &self.binding {
            Ok(())
        } else {
            Err(Error::Binding)
        }
    }
    /// Native UnitCollection.At(current runtime type index). A genuine absent
    /// definition is distinct from present FoundCity=false.
    pub fn unit_found_city(&self, state: &LoadedState, index: u32) -> Result<Option<bool>, Error> {
        self.check(state)?;
        Ok(self.found.get(index as usize).copied())
    }
    /// Native Feature.Find accepts native indexes and unsigned type hashes.
    pub fn feature_find(
        &self,
        state: &LoadedState,
        index_or_hash: u32,
    ) -> Result<Option<Feature>, Error> {
        self.check(state)?;
        Ok(self
            .features
            .find_index(index_or_hash)
            .map(|i| self.feature_flags[i]))
    }
    /// Placement uses Feature.At, rather than Find's index-or-hash fallback.
    pub fn feature_at(&self, state: &LoadedState, index: u32) -> Result<Option<Feature>, Error> {
        self.check(state)?;
        Ok(self.feature_flags.get(index as usize).copied())
    }
    /// Existing canonical Player.Units flag, not a second gameplay owner.
    pub fn settling_disabled(&self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        self.check(state)?;
        state
            .world()
            .player(owner)
            .map_err(|_| Error::Player(owner))?;
        state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .map(|p| p.units.no_settling_cities)
            .ok_or(Error::Player(owner))
    }
}
fn boolean(value: &SqlValue, field: &'static str) -> Result<bool, Error> {
    match value {
        SqlValue::Integer(0) => Ok(false),
        SqlValue::Integer(1) => Ok(true),
        _ => Err(Error::Boolean(field)),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rule_boolean_domain_is_explicit() {
        assert!(!boolean(&SqlValue::Integer(0), "FoundCity").unwrap());
        assert!(boolean(&SqlValue::Integer(1), "Settlement").unwrap());
        assert!(matches!(
            boolean(&SqlValue::Integer(2), "FoundCity"),
            Err(Error::Boolean("FoundCity"))
        ));
        assert!(boolean(&SqlValue::Null, "Settlement").is_err());
    }
}
