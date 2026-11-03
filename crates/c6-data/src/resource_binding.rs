//! Checked immutable resource inputs. Consistency checks do not authenticate a
//! native capture or establish complete gameplay/state/turn parity.
use crate::{
    registry::LinuxDefinitionHashRegistry,
    unit_operation_manager::{
        OperationDefinition, OperationDefinitions, PhaseDefinitions, TurnPhaseDefinition,
    },
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    movement_world::{DefinitionLookup, DefinitionTable},
    skip_turn::placement::UnitProperties,
    unit_initialization::UnitDefinition,
};

/// Digests of complete resource values; observers may have different whole
/// configuration identities while supplying the same actual static resources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceIdentity {
    pub target_sha256: [u8; 32],
    pub host_sha256: [u8; 32],
    pub assets_sha256: [u8; 32],
    pub base_content_sha256: [u8; 32],
    pub saved_configuration_sha256: [u8; 32],
    pub dlc_descriptors_sha256: [u8; 32],
    pub full_content_sha256: [u8; 32],
    pub activation_selection_sha256: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RulesResourceSource {
    pub configuration_sha256: [u8; 32],
    pub resources: ResourceIdentity,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeResourceSource {
    pub configuration_sha256: [u8; 32],
    pub resources: ResourceIdentity,
}
#[derive(Debug)]
pub enum Error {
    Identity(&'static str),
    Definitions(&'static str),
    Operations(crate::unit_operation_manager::Error),
    World(c6_core::movement_world::MissingInput),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "resource binding: {self:?}")
    }
}
impl std::error::Error for Error {}
fn digest_matches(text: &str, digest: &[u8; 32]) -> bool {
    text.len() == 64
        && text
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .zip(digest)
            .all(|(hex, expected)| {
                std::str::from_utf8(hex)
                    .ok()
                    .and_then(|s| u8::from_str_radix(s, 16).ok())
                    == Some(*expected)
            })
}

fn validate_sources(
    destination: &WireBinding,
    source: &RulesResourceSource,
    runtime: &RuntimeResourceSource,
) -> Result<(), Error> {
    if source.resources != runtime.resources {
        return Err(Error::Identity("static resources differ"));
    }
    if runtime.configuration_sha256 != destination.inputs.configuration_sha256 {
        return Err(Error::Identity(
            "runtime configuration differs from destination",
        ));
    }
    Ok(())
}

/// Complete ordered providers, owned and immutable after construction. Duplicate
/// hashes retain the registry's native last-definition/direct-index behavior.
pub struct ResourceDefinitions {
    unit_lookup: LinuxDefinitionHashRegistry,
    units: Vec<Option<UnitProperties>>,
    initialization: Vec<UnitDefinition>,
    operation_lookup: LinuxDefinitionHashRegistry,
    operations: OperationDefinitions,
    phases: PhaseDefinitions,
    configured_phase_hash: u32,
}
impl ResourceDefinitions {
    pub fn new(
        unit_hashes: Vec<u32>,
        units: Vec<Option<UnitProperties>>,
        initialization: Vec<UnitDefinition>,
        operations: Vec<OperationDefinition>,
        phase_slots: Vec<Option<TurnPhaseDefinition>>,
        configured_phase_hash: u32,
    ) -> Result<Self, Error> {
        if unit_hashes.len() > i32::MAX as usize
            || unit_hashes.len() != units.len()
            || units.len() != initialization.len()
        {
            return Err(Error::Definitions("unit collection lengths"));
        }
        if units
            .iter()
            .zip(&initialization)
            .any(|(p, i)| p.as_ref().is_none_or(|p| p.domain_hash != i.domain_hash))
        {
            return Err(Error::Definitions("complete corresponding unit domains"));
        }
        if phase_slots
            .iter()
            .flatten()
            .any(|p| p.phase_type_hash != configured_phase_hash)
        {
            return Err(Error::Definitions("configured phase selection"));
        }
        let operation_lookup = LinuxDefinitionHashRegistry::from_native_order(
            operations.iter().map(|d| d.hash).collect(),
        );
        let operations =
            OperationDefinitions::from_native_order(&operations).map_err(Error::Operations)?;
        Ok(Self {
            unit_lookup: LinuxDefinitionHashRegistry::from_native_order(unit_hashes),
            units,
            initialization,
            operation_lookup,
            operations,
            phases: PhaseDefinitions::from_native_slots(phase_slots),
            configured_phase_hash,
        })
    }
}
/// Ties immutable definitions to one checked Rules object and decoder destination.
/// Actual capture authentication belongs to the caller's input adapter.
pub struct ResourceAuthority<'a> {
    rules: &'a Rules,
    destination: WireBinding,
    source: RulesResourceSource,
    definitions: ResourceDefinitions,
}
impl<'a> ResourceAuthority<'a> {
    pub fn bind(
        destination: WireBinding,
        rules: &'a Rules,
        source: RulesResourceSource,
        runtime: RuntimeResourceSource,
        definitions: ResourceDefinitions,
    ) -> Result<Self, Error> {
        validate_sources(&destination, &source, &runtime)?;
        if !digest_matches(
            &rules.source().native_configuration_sha256,
            &source.configuration_sha256,
        ) {
            return Err(Error::Identity("rules configuration"));
        }
        let r = &source.resources;
        if !digest_matches(rules.profile().gamecore_sha256(), &r.target_sha256) {
            return Err(Error::Identity("rules target"));
        }
        let modules = rules
            .source()
            .module_identities
            .as_array()
            .ok_or(Error::Identity("rules modules"))?;
        for (name, digest) in [
            ("libGameCore_XP2.so", &r.target_sha256),
            ("Civ6", &r.host_sha256),
        ] {
            let mut matching = modules.iter().filter(|m| m["path"] == name);
            if !matching
                .next()
                .and_then(|m| m["sha256"].as_str())
                .is_some_and(|s| digest_matches(s, digest))
                || matching.next().is_some()
            {
                return Err(Error::Identity("rules module identity"));
            }
        }
        if rules.units().len() != definitions.units.len() {
            return Err(Error::Definitions("rules/unit count"));
        }
        let hashes: Vec<_> = rules
            .units()
            .iter()
            .map(|u| crate::registry::linux_type_hash(u.unit_type.as_bytes()))
            .collect();
        if hashes != definitions.unit_lookup.hashes() {
            return Err(Error::Definitions("rules/unit hash order"));
        }
        Ok(Self {
            rules,
            destination,
            source,
            definitions,
        })
    }
    pub fn validate_rules(&self, rules: &Rules) -> Result<(), Error> {
        if !std::ptr::eq(self.rules, rules)
            || rules.source() != self.rules.source()
            || !digest_matches(
                &rules.source().native_configuration_sha256,
                &self.source.configuration_sha256,
            )
        {
            return Err(Error::Identity("bound rules differ"));
        }
        Ok(())
    }
    pub fn destination_binding(&self) -> &WireBinding {
        &self.destination
    }
    pub fn resource_identity(&self) -> &ResourceIdentity {
        &self.source.resources
    }
    pub fn placement_definitions(
        &self,
    ) -> Result<DefinitionTable<'_, Option<UnitProperties>>, Error> {
        DefinitionTable::new(&self.definitions.unit_lookup, &self.definitions.units)
            .map_err(Error::World)
    }
    pub fn unit_definitions(&self) -> Result<DefinitionTable<'_, UnitDefinition>, Error> {
        DefinitionTable::new(
            &self.definitions.unit_lookup,
            &self.definitions.initialization,
        )
        .map_err(Error::World)
    }
    pub fn operations(&self) -> &OperationDefinitions {
        &self.definitions.operations
    }
    pub fn operation_lookup(&self) -> &dyn DefinitionLookup {
        &self.definitions.operation_lookup
    }
    pub fn phases(&self) -> &PhaseDefinitions {
        &self.definitions.phases
    }
    pub fn configured_phase_hash(&self) -> u32 {
        self.definitions.configured_phase_hash
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn definitions_reject_incomplete_units_operation_order_and_phase_selection() {
        assert!(ResourceDefinitions::new(vec![10], vec![], vec![], vec![], vec![], 1).is_err());
        assert!(ResourceDefinitions::new(
            vec![10],
            vec![None],
            vec![UnitDefinition { domain_hash: 2 }],
            vec![],
            vec![],
            1
        )
        .is_err());
        assert!(ResourceDefinitions::new(
            vec![],
            vec![],
            vec![],
            vec![OperationDefinition {
                collection_index: 1,
                hash: 3
            }],
            vec![],
            1
        )
        .is_err());
        assert!(ResourceDefinitions::new(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![Some(TurnPhaseDefinition {
                row_id: crate::RulesRowId(1),
                phase_type_hash: 2,
                phase_order: 0,
                active_segment: None
            })],
            1
        )
        .is_err());
        assert!(ResourceDefinitions::new(vec![], vec![], vec![], vec![], vec![None], 1).is_ok());
    }
    #[test]
    fn source_binding_rejects_each_static_identity_and_runtime_configuration_change() {
        use c6_core::core_player::initialization::{DatabaseIdentity, InputIdentities, WireRole};
        let id = ResourceIdentity {
            target_sha256: [1; 32],
            host_sha256: [2; 32],
            assets_sha256: [3; 32],
            base_content_sha256: [4; 32],
            saved_configuration_sha256: [5; 32],
            dlc_descriptors_sha256: [6; 32],
            full_content_sha256: [7; 32],
            activation_selection_sha256: [8; 32],
        };
        let source = RulesResourceSource {
            configuration_sha256: [10; 32],
            resources: id.clone(),
        };
        let runtime = RuntimeResourceSource {
            configuration_sha256: [11; 32],
            resources: id,
        };
        let destination = WireBinding {
            role: WireRole::ReadInputCore,
            core_sha256: [12; 32],
            occurrence_id: vec![],
            output_id: None,
            inputs: InputIdentities {
                configuration_sha256: [11; 32],
                rules_inputs_sha256: [0; 32],
                database: DatabaseIdentity::LiveSnapshotUnavailable,
                dlc_inputs_sha256: [0; 32],
                mods_inputs_sha256: [0; 32],
                content_inputs_sha256: [0; 32],
            },
        };
        assert!(validate_sources(&destination, &source, &runtime).is_ok());
        for index in 0..8 {
            let mut changed = runtime.clone();
            let fields = [
                &mut changed.resources.target_sha256,
                &mut changed.resources.host_sha256,
                &mut changed.resources.assets_sha256,
                &mut changed.resources.base_content_sha256,
                &mut changed.resources.saved_configuration_sha256,
                &mut changed.resources.dlc_descriptors_sha256,
                &mut changed.resources.full_content_sha256,
                &mut changed.resources.activation_selection_sha256,
            ];
            fields[index][31] ^= 1;
            assert!(validate_sources(&destination, &source, &changed).is_err());
        }
        let mut changed = runtime;
        changed.configuration_sha256[31] ^= 1;
        assert!(validate_sources(&destination, &source, &changed).is_err());
    }
    #[test]
    fn identity_checks_compare_every_digest_byte() {
        let digest = [0xab; 32];
        assert!(digest_matches(&"ab".repeat(32), &digest));
        assert!(!digest_matches(&format!("{}aa", "ab".repeat(31)), &digest));
        assert!(!digest_matches(&"ab".repeat(31), &digest));
        assert!(!digest_matches(&"zz".repeat(32), &digest));
    }
}
