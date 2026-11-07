//! Owned immutable resource input for initialized snapshots, not save decoding.
//!
//! Digests are JSON arrays of exactly 32 bytes. Ordered definition arrays retain
//! the caller's actual runtime collection order, including duplicate hashes and
//! proven null phase slots. Supplying these values does not authenticate their
//! origin or establish complete gameplay parity.
use crate::{
    resource_binding::{
        self, ResourceAuthority, ResourceDefinitions, ResourceIdentity, RuntimeResourceSource,
    },
    unit_operation_manager::{OperationDefinition, TurnPhaseDefinition, TurnSegmentDefinition},
    Rules, RulesRowId,
};
use c6_core::{
    core_player::initialization::WireBinding, skip_turn::placement::UnitProperties,
    unit_initialization::UnitDefinition,
};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub enum Error {
    SchemaVersion(u32),
    Binding(resource_binding::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemaVersion(version) => {
                write!(f, "unsupported resource input schema version {version}")
            }
            Self::Binding(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl From<resource_binding::Error> for Error {
    fn from(error: resource_binding::Error) -> Self {
        Self::Binding(error)
    }
}

/// Versioned resource bundle. Both database digests identify actual exported SQL
/// bytes used by `Rules`, rather than a save or the complete runtime configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcesInput {
    pub schema_version: u32,
    pub runtime: RuntimeResourceSourceInput,
    pub configuration_database_sha256: [u8; 32],
    pub gameplay_database_sha256: [u8; 32],
    pub definitions: ResourceDefinitionsInput,
}
impl ResourcesInput {
    pub fn build_definitions(&self) -> Result<ResourceDefinitions, Error> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(Error::SchemaVersion(self.schema_version));
        }
        Ok(self.definitions.build()?)
    }
    /// Validate the supplied rules, runtime identities and ordered definitions
    /// using the production authority checks. No native engine is required.
    pub fn bind<'a>(
        &self,
        destination: WireBinding,
        rules: &'a Rules,
    ) -> Result<ResourceAuthority<'a>, Error> {
        Ok(ResourceAuthority::bind_database_snapshot(
            destination,
            rules,
            self.runtime.to_source(),
            self.configuration_database_sha256,
            self.gameplay_database_sha256,
            self.build_definitions()?,
        )?)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceIdentityInput {
    pub target_sha256: [u8; 32],
    pub host_sha256: [u8; 32],
    pub assets_sha256: [u8; 32],
    pub base_content_sha256: [u8; 32],
    /// Null for an initialized game that has no loaded saved configuration.
    pub saved_configuration_sha256: Option<[u8; 32]>,
    pub dlc_descriptors_sha256: [u8; 32],
    pub full_content_sha256: [u8; 32],
    pub activation_selection_sha256: [u8; 32],
}
impl ResourceIdentityInput {
    pub fn to_identity(&self) -> ResourceIdentity {
        ResourceIdentity {
            target_sha256: self.target_sha256,
            host_sha256: self.host_sha256,
            assets_sha256: self.assets_sha256,
            base_content_sha256: self.base_content_sha256,
            saved_configuration_sha256: self.saved_configuration_sha256,
            dlc_descriptors_sha256: self.dlc_descriptors_sha256,
            full_content_sha256: self.full_content_sha256,
            activation_selection_sha256: self.activation_selection_sha256,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResourceSourceInput {
    /// Must equal the destination WireBinding's runtime configuration digest.
    pub configuration_sha256: [u8; 32],
    pub resources: ResourceIdentityInput,
}
impl RuntimeResourceSourceInput {
    pub fn to_source(&self) -> RuntimeResourceSource {
        RuntimeResourceSource {
            configuration_sha256: self.configuration_sha256,
            resources: self.resources.to_identity(),
        }
    }
}

/// Unit arrays correspond by index and must match `Rules` unit hash order when
/// bound. A null unit property is rejected as incomplete by the authority.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDefinitionsInput {
    pub unit_hashes: Vec<u32>,
    pub units: Vec<Option<UnitPropertiesInput>>,
    pub initialization: Vec<UnitInitializationInput>,
    /// Collection indices must be consecutive, starting at zero.
    pub operations: Vec<OperationDefinitionInput>,
    /// Null denotes an observed null slot, never an unknown definition.
    pub phase_slots: Vec<Option<TurnPhaseDefinitionInput>>,
    pub configured_phase_hash: u32,
}
impl ResourceDefinitionsInput {
    pub fn build(&self) -> Result<ResourceDefinitions, resource_binding::Error> {
        ResourceDefinitions::new(
            self.unit_hashes.clone(),
            self.units
                .iter()
                .map(|unit| {
                    unit.as_ref().map(|u| UnitProperties {
                        domain_hash: u.domain_hash,
                        formation_class_hash: u.formation_class_hash,
                        make_trade_route: u.make_trade_route,
                        spy: u.spy,
                        combat: u.combat,
                        range: u.range,
                    })
                })
                .collect(),
            self.initialization
                .iter()
                .map(|u| UnitDefinition {
                    domain_hash: u.domain_hash,
                })
                .collect(),
            self.operations
                .iter()
                .map(|o| OperationDefinition {
                    collection_index: o.collection_index,
                    hash: o.hash,
                })
                .collect(),
            self.phase_slots
                .iter()
                .map(|phase| {
                    phase.as_ref().map(|p| TurnPhaseDefinition {
                        row_id: RulesRowId(p.row_id),
                        phase_type_hash: p.phase_type_hash,
                        phase_order: p.phase_order,
                        active_segment: p.active_segment.as_ref().map(|s| TurnSegmentDefinition {
                            row_id: RulesRowId(s.row_id),
                            flags: s.flags,
                        }),
                    })
                })
                .collect(),
            self.configured_phase_hash,
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitPropertiesInput {
    pub domain_hash: u32,
    pub formation_class_hash: u32,
    pub make_trade_route: bool,
    pub spy: bool,
    pub combat: i32,
    pub range: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitInitializationInput {
    pub domain_hash: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDefinitionInput {
    pub collection_index: u32,
    pub hash: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnPhaseDefinitionInput {
    pub row_id: i64,
    pub phase_type_hash: u32,
    pub phase_order: i32,
    pub active_segment: Option<TurnSegmentDefinitionInput>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnSegmentDefinitionInput {
    pub row_id: i64,
    /// Raw definition flags, not an evaluated predicate or external policy.
    pub flags: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn input() -> ResourcesInput {
        ResourcesInput {
            schema_version: SCHEMA_VERSION,
            runtime: RuntimeResourceSourceInput {
                configuration_sha256: [1; 32],
                resources: ResourceIdentityInput {
                    target_sha256: [2; 32],
                    host_sha256: [3; 32],
                    assets_sha256: [4; 32],
                    base_content_sha256: [5; 32],
                    saved_configuration_sha256: None,
                    dlc_descriptors_sha256: [6; 32],
                    full_content_sha256: [7; 32],
                    activation_selection_sha256: [8; 32],
                },
            },
            configuration_database_sha256: [9; 32],
            gameplay_database_sha256: [10; 32],
            definitions: ResourceDefinitionsInput {
                unit_hashes: vec![11],
                units: vec![Some(UnitPropertiesInput {
                    domain_hash: 12,
                    formation_class_hash: 13,
                    make_trade_route: false,
                    spy: true,
                    combat: -1,
                    range: 2,
                })],
                initialization: vec![UnitInitializationInput { domain_hash: 12 }],
                operations: vec![OperationDefinitionInput {
                    collection_index: 0,
                    hash: 14,
                }],
                phase_slots: vec![
                    None,
                    Some(TurnPhaseDefinitionInput {
                        row_id: 15,
                        phase_type_hash: 16,
                        phase_order: -1,
                        active_segment: Some(TurnSegmentDefinitionInput {
                            row_id: 17,
                            flags: 128,
                        }),
                    }),
                ],
                configured_phase_hash: 16,
            },
        }
    }
    #[test]
    fn roundtrip_preserves_order_null_slots_and_identities() {
        let original = input();
        let json = serde_json::to_vec(&original).unwrap();
        let decoded: ResourcesInput = serde_json::from_slice(&json).unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), json);
        assert_eq!(decoded.runtime.to_source().configuration_sha256, [1; 32]);
        assert_eq!(
            decoded.runtime.resources.to_identity().target_sha256,
            [2; 32]
        );
        assert_eq!(
            decoded
                .runtime
                .resources
                .to_identity()
                .saved_configuration_sha256,
            None
        );
        assert!(decoded.build_definitions().is_ok());
    }
    #[test]
    fn rejects_version_and_incomplete_or_mismatched_definitions() {
        let mut value = input();
        value.schema_version += 1;
        assert!(matches!(
            value.build_definitions(),
            Err(Error::SchemaVersion(_))
        ));
        value.schema_version = SCHEMA_VERSION;
        value.definitions.units.clear();
        assert!(value.build_definitions().is_err());
        value = input();
        value.definitions.initialization[0].domain_hash ^= 1;
        assert!(value.build_definitions().is_err());
        value = input();
        value.definitions.operations[0].collection_index = 1;
        assert!(value.build_definitions().is_err());
        value = input();
        value.definitions.configured_phase_hash ^= 1;
        assert!(value.build_definitions().is_err());
    }
    #[test]
    fn rejects_digest_lengths_and_unknown_fields_at_nested_boundaries() {
        let original = serde_json::to_value(input()).unwrap();
        for length in [0, 31, 33] {
            let mut value = original.clone();
            value["runtime"]["resources"]["target_sha256"] = json!(vec![0u8; length]);
            assert!(serde_json::from_value::<ResourcesInput>(value).is_err());
        }
        for pointer in [
            "",
            "/runtime",
            "/runtime/resources",
            "/definitions",
            "/definitions/units/0",
            "/definitions/initialization/0",
            "/definitions/operations/0",
            "/definitions/phase_slots/1",
            "/definitions/phase_slots/1/active_segment",
        ] {
            let mut value = original.clone();
            value
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unknown".into(), Value::Null);
            assert!(
                serde_json::from_value::<ResourcesInput>(value).is_err(),
                "{pointer}"
            );
        }
    }
}
