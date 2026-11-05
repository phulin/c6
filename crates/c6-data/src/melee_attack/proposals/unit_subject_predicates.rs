//! Cached admitted promotion text queried through current canonical Unit runtime.
//! Combat subject/context interfaces and ConstRequirement lifecycle remain with
//! the proposal backend; native predicate answers are never definition inputs.
use crate::{
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::InputIdentities, loaded_state::LoadedState, map::UnitReference,
    movement_world::DefinitionLookup,
};

#[derive(Debug)]
pub enum Error {
    Actor(super::super::Error),
    State(c6_core::loaded_state::Error),
    Definitions(unit_initialization_definitions::Error),
    Resources(crate::resource_binding::Error),
    Rules(crate::Error),
    Binding,
    Missing(&'static str),
    PromotionText,
    NativeUnitOrder,
    CurrentDefinition,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "combat promotion definition: {self:?}")
    }
}
impl std::error::Error for Error {}

pub struct Definitions {
    identities: InputIdentities,
    units: LinuxDefinitionRegistry,
    promotion_classes: Vec<Option<Vec<u8>>>,
    tags: Option<crate::loaded_actions::reporting::hold_cycling::commands::Tags>,
}
fn promotion_text(value: &SqlValue) -> Result<Option<Vec<u8>>, Error> {
    match value {
        SqlValue::Null => Ok(None),
        SqlValue::Text(value) => {
            // Native SQL GetText/string assignment and later strlen consume a
            // CString prefix. Preserve NULL separately from a valid empty one.
            let length = value.iter().position(|b| *b == 0).unwrap_or(value.len());
            Ok(Some(value[..length].to_vec()))
        }
        _ => Err(Error::PromotionText),
    }
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
        let native = resources.unit_definitions().map_err(Error::Resources)?;
        if native.lookup.count() != units.entries().len()
            || (0..native.lookup.count()).any(|i| native.lookup.type_hash(i) != units.type_hash(i))
        {
            return Err(Error::NativeUnitOrder);
        }
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let promotion_classes = units
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Unit row"))?;
                promotion_text(row.get("PromotionClass").map_err(Error::Rules)?)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            identities: state.binding().inputs.clone(),
            units,
            promotion_classes,
            tags: None,
        })
    }
    pub fn with_tags(
        mut self,
        tags: crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    ) -> Self {
        self.tags = Some(tags);
        self
    }
    pub fn unit_has_tag(
        &self,
        state: &LoadedState,
        reference: UnitReference,
        tag: &[u8],
    ) -> Result<Option<bool>, Error> {
        if self.identities != state.binding().inputs {
            return Err(Error::Binding);
        }
        let unit = super::super::actor(state, reference).map_err(Error::Actor)?;
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let Some(index) = usize::try_from(runtime.definition_index)
            .ok()
            .filter(|index| *index < self.promotion_classes.len())
        else {
            return Ok(None);
        };
        if self.units.find_index(unit.e_type_index) != Some(index) {
            return Err(Error::CurrentDefinition);
        }
        Ok(Some(
            self.tags
                .as_ref()
                .ok_or(Error::Missing("current Tag repository"))?
                .has(unit.e_type_index, tag),
        ))
    }
    /// Resolve the SAME current actor and initialized native definition index.
    /// At an absent index native Definitions.At returns NULL, so the source
    /// predicate returns state3 before attempting a promotion-text comparison.
    pub fn promotion_class(
        &self,
        state: &LoadedState,
        reference: UnitReference,
    ) -> Result<Option<&[u8]>, Error> {
        if self.identities != state.binding().inputs {
            return Err(Error::Binding);
        }
        let unit = super::super::actor(state, reference).map_err(Error::Actor)?;
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let Some(index) = usize::try_from(runtime.definition_index)
            .ok()
            .filter(|index| *index < self.promotion_classes.len())
        else {
            return Ok(None);
        };
        if self.units.find_index(unit.e_type_index) != Some(index) {
            return Err(Error::CurrentDefinition);
        }
        Ok(self.promotion_classes[index].as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn null_empty_and_embedded_nul_follow_optional_cstring_semantics() {
        assert_eq!(promotion_text(&SqlValue::Null).unwrap(), None);
        assert_eq!(
            promotion_text(&SqlValue::Text(vec![])).unwrap(),
            Some(vec![])
        );
        assert_eq!(
            promotion_text(&SqlValue::Text(b"ANTI\0other".to_vec())).unwrap(),
            Some(b"ANTI".to_vec())
        );
        assert_eq!(
            promotion_text(&SqlValue::Text(vec![0xff])).unwrap(),
            Some(vec![0xff])
        );
    }
    #[test]
    fn unsupported_scalar_storage_is_not_a_missing_promotion_answer() {
        assert!(matches!(
            promotion_text(&SqlValue::Integer(0)),
            Err(Error::PromotionText)
        ));
        assert!(matches!(
            promotion_text(&SqlValue::Blob(vec![])),
            Err(Error::PromotionText)
        ));
    }
}
