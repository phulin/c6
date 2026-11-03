//! Admitted Linux activated RandomEvent collection for pre-load enum conversion.
//! Native BuildCollection assigns append indexes from the unfiltered rowid query;
//! row IDs remain separate database identities. This provider does not establish
//! that an arbitrary raw save uses this activation profile or implement events.
use crate::{
    registry::LinuxDefinitionRegistry,
    unit_initialization_definitions::{self, collection},
    Rules, RulesProfile, SourceIdentity,
};
use c6_core::{map::FeatureComponent, movement_world::DefinitionLookup};

#[derive(Debug)]
pub enum Error {
    Profile,
    Binding,
    Rules(crate::Error),
    Definitions(unit_initialization_definitions::Error),
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux RandomEvent definitions: {self:?}")
    }
}
impl std::error::Error for Error {}

/// A real registered rules collection, available before a LoadedState exists.
/// There is no fallback empty constructor. Missing/unadmitted definitions are
/// errors (or caller None), whereas a lookup miss in this owner returns -1.
pub struct Definitions {
    source: SourceIdentity,
    registry: LinuxDefinitionRegistry,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let profile = RulesProfile::LinuxXp2ActivatedDiagnostic;
        if rules.profile() != profile
            || rules.source() != &profile.source_identity().map_err(Error::Rules)?
        {
            return Err(Error::Profile);
        }
        // The Rules owner already checked the registered source identity and
        // complete database fingerprints. This constructor checks consistency;
        // native export/report authentication belongs to private input adapters.
        // Exact GetBuilderSql0x2208c30: all RandomEvents ORDER BY rowid;
        // RandomEventType is column22. Index = size before append, not rowid.
        let registry =
            collection(rules, "RandomEvents", "RandomEventType").map_err(Error::Definitions)?;
        Ok(Self {
            source: rules.source().clone(),
            registry,
        })
    }
    pub fn source_identity(&self) -> &SourceIdentity {
        &self.source
    }
    pub fn registry(&self) -> &LinuxDefinitionRegistry {
        &self.registry
    }
    /// Compare the complete caller-admitted active rules/configuration identity.
    /// This checks consistency, not authenticity of a raw save's declaration.
    pub fn validate_source(&self, active_source: &SourceIdentity) -> Result<(), Error> {
        if active_source != &self.source {
            return Err(Error::Binding);
        }
        Ok(())
    }
    /// Conversion projection with an explicit caller-bound active source. An
    /// unavailable raw activation is not replaced by this diagnostic profile.
    pub fn resolve_ice_for_source(
        &self,
        feature: &FeatureComponent,
        active_source: &SourceIdentity,
    ) -> Result<Vec<i32>, Error> {
        self.validate_source(active_source)?;
        self.resolve_ice(feature)
    }
    /// Native HashToIndex0x2bf7a50: ordinal-first Find, then unsigned hash lookup;
    /// the last inserted equal hash wins, and a genuine miss returns signed -1.
    pub fn hash_to_index(&self, index_or_hash: u32) -> i32 {
        hash_to_index(&self.registry, index_or_hash)
    }
    /// Transient source-derived import projection in exact Ice order. Compute
    /// this before apply_legacy_ice_read mutates Plots. It is not fixture answers
    /// or a persistent independently mutable gameplay sidecar.
    pub fn resolve_ice(&self, feature: &FeatureComponent) -> Result<Vec<i32>, Error> {
        let mut indexes = Vec::new();
        indexes
            .try_reserve_exact(feature.ice.len())
            .map_err(|_| Error::ResourceLimit)?;
        indexes.extend(
            feature
                .ice
                .iter()
                .map(|ice| self.hash_to_index(ice.random_event_hash)),
        );
        Ok(indexes)
    }
}
fn hash_to_index(registry: &LinuxDefinitionRegistry, key: u32) -> i32 {
    // Shared collection() rejects counts beyond signed native index range.
    registry.find_index(key).map_or(-1, |index| index as i32)
}
impl DefinitionLookup for Definitions {
    fn count(&self) -> usize {
        self.registry.count()
    }
    fn find_index(&self, key: u32) -> Option<usize> {
        self.registry.find_index(key)
    }
    fn type_hash(&self, index: usize) -> Option<u32> {
        self.registry.type_hash(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        registry::{linux_type_hash, DefinitionEntry},
        RulesRowId,
    };
    #[test]
    fn conversion_retains_ordinal_precedence_duplicate_hash_order_and_valid_miss() {
        let registry = LinuxDefinitionRegistry::from_native_order(
            [(7, "A"), (17, "B"), (41, "A")]
                .into_iter()
                .map(|(id, name)| DefinitionEntry {
                    database_rowid: RulesRowId(id),
                    type_name: name.as_bytes().to_vec(),
                })
                .collect(),
        );
        assert_eq!(hash_to_index(&registry, 0), 0);
        assert_eq!(hash_to_index(&registry, 1), 1);
        assert_eq!(hash_to_index(&registry, linux_type_hash(b"A")), 2);
        assert_eq!(hash_to_index(&registry, linux_type_hash(b"B")), 1);
        assert_eq!(hash_to_index(&registry, u32::MAX), -1);
        assert_eq!(
            hash_to_index(&LinuxDefinitionRegistry::from_native_order(vec![]), 0),
            -1
        );
    }
}
