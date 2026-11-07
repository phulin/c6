//! Owned transport of initialized component bytes and supplied framing facts.
//!
//! This is simulator input, not a Civ VI save loader or game initializer. Source
//! bytes are retained exactly; controls and unsaved runtime owners must still be
//! explicitly installed before actions. Consistency checks do not authenticate
//! the caller's observations.
use c6_core::{
    core_player::{self, initialization as init},
    core_stream, game_barbarian, game_free_cities, game_global_ai, game_random_wire,
    loaded_components,
    loaded_state::{AssemblyLimits, CoreImage, LoadedState, RetainedComponents},
    map, unit, world,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::Arc};

pub mod resources;
pub mod script;

pub const SCHEMA: &str = "c6-initialized-components-v1";
pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema: String,
    pub source: Vec<u8>,
    pub binding: Binding,
    pub components: Vec<Component>,
    pub players: Vec<PlayerContext>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub source_sha256: [u8; 32],
    pub occurrence_id: Vec<u8>,
    pub output_id: Vec<u8>,
    pub configuration_sha256: [u8; 32],
    pub rules_inputs_sha256: [u8; 32],
    /// None explicitly means the live database snapshot identity is unavailable.
    pub database_sha256: Option<[u8; 32]>,
    pub dlc_inputs_sha256: [u8; 32],
    pub mods_inputs_sha256: [u8; 32],
    pub content_inputs_sha256: [u8; 32],
}
impl Binding {
    pub fn wire(&self) -> init::WireBinding {
        init::WireBinding {
            role: init::WireRole::SavedOutputComponents,
            core_sha256: self.source_sha256,
            occurrence_id: self.occurrence_id.clone(),
            output_id: Some(self.output_id.clone()),
            inputs: init::InputIdentities {
                configuration_sha256: self.configuration_sha256,
                rules_inputs_sha256: self.rules_inputs_sha256,
                database: self.database_sha256.map_or(
                    init::DatabaseIdentity::LiveSnapshotUnavailable,
                    init::DatabaseIdentity::Snapshot,
                ),
                dlc_inputs_sha256: self.dlc_inputs_sha256,
                mods_inputs_sha256: self.mods_inputs_sha256,
                content_inputs_sha256: self.content_inputs_sha256,
            },
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Kind {
    CorePrefix,
    Map,
    Player { owner: i32, manager_index: u32 },
    Religion,
    GlobalAi,
    Barbarian,
    FreeCities,
    Random,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub component: Kind,
    pub range: [usize; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vector {
    pub values_i32: Vec<i32>,
}
impl Vector {
    fn typed(&self) -> Result<init::VectorState> {
        let size = u32::try_from(self.values_i32.len()).map_err(|_| "Military vector length")?;
        // Canonical adapter bookkeeping only: no native address is transported
        // or inferred. The Write codec consumes lengths and semantic values.
        Ok(init::VectorState {
            data_pointer: u64::from(size != 0),
            capacity_u32: size,
            size_u32: size,
            padding_u32: 0,
            tail_padding_u32: 0,
            values_i32: self.values_i32.clone(),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub database_row_i32: i32,
    pub collection_index_u32: u32,
    pub operation_type_hash_u32: u32,
    pub default_value_i32: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definitions {
    pub ordered: Vec<Definition>,
}
impl Definitions {
    fn typed(&self) -> init::DefinitionCollection {
        // Ordinal tokens satisfy the legacy layout container without carrying
        // process addresses. They are not reconstructed native identities.
        let end = self.ordered.len() as u64 * 8;
        init::DefinitionCollection {
            collection_pointer: 0,
            begin_pointer: 0,
            end_pointer: end,
            capacity_pointer: end,
            ordered: self
                .ordered
                .iter()
                .enumerate()
                .map(|(i, d)| init::DefinitionEntry {
                    pointer: i as u64 + 1,
                    database_row_i32: d.database_row_i32,
                    collection_index_u32: d.collection_index_u32,
                    operation_type_hash_u32: d.operation_type_hash_u32,
                    default_value_i32: d.default_value_i32,
                })
                .collect(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerContext {
    pub owner: i32,
    pub manager_index: u32,
    /// Exact Military record interval in `source`, not the entire Player range.
    /// None is required for a Player that was never alive.
    pub military_range: Option<[usize; 2]>,
    pub was_ever_alive_raw: u8,
    pub handicap_i32: i32,
    pub allowed: Vector,
    pub per_war: Vector,
    pub current: Vector,
    pub definitions: Option<Definitions>,
}
/// Resource policy is supplied separately from untrusted transport data.
pub struct Limits {
    pub max_json_bytes: usize,
    pub max_source_bytes: usize,
    pub max_components: usize,
    pub map: map::Limits,
    pub component: unit::Limits,
    pub initialization: init::Limits,
    pub assembly: AssemblyLimits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_json_bytes: 256 * 1024 * 1024,
            max_source_bytes: 64 * 1024 * 1024,
            max_components: 72,
            map: map::Limits::default(),
            component: unit::Limits::default(),
            initialization: init::Limits::default(),
            assembly: AssemblyLimits::default(),
        }
    }
}
struct Contexts(Vec<(init::PlayerIdentity, Arc<init::InitializationSnapshot>)>);
impl core_player::PlayerInitializations for Contexts {
    fn for_player(&self, id: init::PlayerIdentity) -> Option<Arc<init::InitializationSnapshot>> {
        self.0
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, v)| v.clone())
    }
}
impl Input {
    pub fn from_json(bytes: &[u8], limits: &Limits) -> Result<Self> {
        if bytes.len() > limits.max_json_bytes {
            return Err("initialized input JSON limit".into());
        }
        let input: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        input.validate(limits)?;
        Ok(input)
    }
    fn validate(&self, limits: &Limits) -> Result<()> {
        if self.schema != SCHEMA {
            return Err("initialized input schema".into());
        }
        if self.source.len() > limits.max_source_bytes
            || self.components.len() > limits.max_components
            || self.players.len() > limits.max_components
        {
            return Err("initialized input extent limit".into());
        }
        if self.binding.source_sha256 != <[u8; 32]>::from(Sha256::digest(&self.source)) {
            return Err("initialized input source identity".into());
        }
        if self.binding.occurrence_id.is_empty()
            || self.binding.output_id.is_empty()
            || self.binding.occurrence_id.len() > limits.initialization.max_identity_bytes
            || self.binding.output_id.len() > limits.initialization.max_identity_bytes
            || [
                self.binding.configuration_sha256,
                self.binding.rules_inputs_sha256,
                self.binding.dlc_inputs_sha256,
                self.binding.mods_inputs_sha256,
                self.binding.content_inputs_sha256,
            ]
            .contains(&[0; 32])
            || self.binding.database_sha256 == Some([0; 32])
        {
            return Err("initialized input binding identity".into());
        }
        let mut ranges = Vec::new();
        let mut kinds = BTreeSet::new();
        let mut owners = BTreeSet::new();
        for c in &self.components {
            if c.range[0] >= c.range[1] || c.range[1] > self.source.len() {
                return Err("component range".into());
            }
            let key = match c.component {
                Kind::Player { owner, .. } => {
                    if !owners.insert(owner) {
                        return Err("duplicate Player owner".into());
                    }
                    format!("player:{owner}")
                }
                ref other => format!("{other:?}"),
            };
            if !kinds.insert(key) {
                return Err("duplicate component".into());
            }
            ranges.push(c.range);
        }
        ranges.sort_unstable();
        if ranges.windows(2).any(|r| r[0][1] > r[1][0]) {
            return Err("overlapping component ranges".into());
        }
        let mut contexts = BTreeSet::new();
        let mut manager_indices = BTreeSet::new();
        for p in &self.players {
            if !(0..65).contains(&p.owner)
                || p.manager_index >= 65
                || !manager_indices.insert(p.manager_index)
            {
                return Err("Player manager identity".into());
            }
            if !contexts.insert(p.owner) {
                return Err("duplicate Player context".into());
            }
            let values = p
                .allowed
                .values_i32
                .len()
                .checked_add(p.per_war.values_i32.len())
                .and_then(|n| n.checked_add(p.current.values_i32.len()))
                .and_then(|n| n.checked_add(p.definitions.as_ref().map_or(0, |d| d.ordered.len())))
                .ok_or("Military extent overflow")?;
            if values > limits.initialization.max_values {
                return Err("Military values limit".into());
            }
            let c = self
                .components
                .iter()
                .find(|c| {
                    c.component
                        == Kind::Player {
                            owner: p.owner,
                            manager_index: p.manager_index,
                        }
                })
                .ok_or("Player context component identity")?;
            match p.military_range {
                Some(range)
                    if p.was_ever_alive_raw != 0
                        && range[0] >= c.range[0]
                        && range[0] < range[1]
                        && range[1] <= c.range[1] => {}
                None if p.was_ever_alive_raw == 0
                    && p.allowed.values_i32.is_empty()
                    && p.per_war.values_i32.is_empty()
                    && p.current.values_i32.is_empty()
                    && p.definitions.is_none() => {}
                _ => return Err("Military range/alive context".into()),
            }
        }
        if owners != contexts {
            return Err("Player context coverage".into());
        }
        for kind in [
            Kind::CorePrefix,
            Kind::Map,
            Kind::Religion,
            Kind::GlobalAi,
            Kind::Barbarian,
            Kind::FreeCities,
            Kind::Random,
        ] {
            self.one(&kind)?;
        }
        Ok(())
    }
    fn one(&self, kind: &Kind) -> Result<&Component> {
        self.components
            .iter()
            .find(|c| &c.component == kind)
            .ok_or_else(|| format!("missing component {kind:?}"))
    }
    /// Decode and consume the transport into one canonical state owner.
    pub fn decode(self, limits: &Limits) -> Result<LoadedState> {
        self.validate(limits)?;
        let binding = self.binding.wire();
        let contexts = Contexts(
            self.players
                .iter()
                .filter(|p| p.was_ever_alive_raw != 0)
                .map(|p| {
                    let facts = init::InitializationFacts {
                        binding: binding.clone(),
                        owner: p.owner,
                        was_ever_alive_raw: p.was_ever_alive_raw,
                        handicap_i32: p.handicap_i32,
                        core_range: p.military_range.ok_or("missing Military range")?,
                        vectors: init::Vectors {
                            allowed: p.allowed.typed()?,
                            per_war: p.per_war.typed()?,
                            current: p.current.typed()?,
                        },
                        active_definitions: p.definitions.as_ref().map(Definitions::typed),
                        provenance: init::InitializationProvenance::Write,
                    };
                    let snapshot = init::InitializationSnapshot::from_supplied_facts(
                        facts,
                        limits.initialization,
                    )
                    .map_err(|e| e.to_string())?;
                    Ok((
                        init::PlayerIdentity {
                            manager_index: p.manager_index,
                            requested_owner: p.owner,
                            record_owner: p.owner,
                        },
                        Arc::new(snapshot),
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
        );
        let raw = |c: &Component| &self.source[c.range[0]..c.range[1]];
        let carrier = self.one(&Kind::CorePrefix)?;
        let prefix =
            core_stream::CorePrefix::decode(raw(carrier), limits.map).map_err(|e| e.to_string())?;
        if prefix.consumed != raw(carrier).len() {
            return Err("CorePrefix trailing bytes".into());
        }
        let (prefix_map, mut core) = loaded_components::split_core(prefix);
        let map =
            map::Map::decode(raw(self.one(&Kind::Map)?), limits.map).map_err(|e| e.to_string())?;
        if prefix_map != map {
            return Err("component Map differs from Core prefix".into());
        }
        core.map_start = core
            .map_start
            .checked_add(carrier.range[0])
            .ok_or("Core range overflow")?;
        core.map_end = core
            .map_end
            .checked_add(carrier.range[0])
            .ok_or("Core range overflow")?;
        core.consumed = core
            .consumed
            .checked_add(carrier.range[0])
            .ok_or("Core range overflow")?;
        let mut players = Vec::new();
        let mut retained_players = Vec::new();
        for c in &self.components {
            if let Kind::Player {
                owner,
                manager_index,
            } = c.component
            {
                let (player, used, _) = core_player::FirstPlayer::decode_conditional_at(
                    raw(c),
                    limits.component,
                    owner,
                    manager_index,
                    &binding,
                    &contexts,
                    c.range[0],
                )
                .map_err(|e| e.to_string())?;
                if used != raw(c).len() {
                    return Err("Player trailing bytes".into());
                }
                let supplied = self
                    .players
                    .iter()
                    .find(|p| p.owner == owner)
                    .ok_or("missing Player context")?;
                if player.header.saved_was_ever_alive() != (supplied.was_ever_alive_raw != 0) {
                    return Err("Player alive context differs from header".into());
                }
                if let Some(range) = supplied.military_range {
                    let military = c6_core::player_ai_military::Military::decode_in_context(
                        &self.source[range[0]..range[1]],
                        limits.component,
                        c6_core::player_ai_military::Context {
                            operation_type_arrays_present: !supplied.allowed.values_i32.is_empty(),
                        },
                    )
                    .map_err(|e| e.to_string())?;
                    if player.ai_military.as_ref() != Some(&military) {
                        return Err("Military context range differs from decoded Player".into());
                    }
                }
                let (world, retained) =
                    loaded_components::split_player(player).map_err(|e| e.to_string())?;
                players.push(world);
                retained_players.push(retained);
            }
        }
        macro_rules! component {
            ($kind:expr,$decoder:path) => {{
                let bytes = raw(self.one(&$kind)?);
                let (value, used, _) =
                    $decoder(bytes, limits.component).map_err(|e| e.to_string())?;
                if used != bytes.len() {
                    return Err(format!("{:?} trailing bytes", $kind));
                }
                value
            }};
        }
        let retained = RetainedComponents {
            core,
            players: retained_players,
            religion: component!(
                Kind::Religion,
                core_player::Religion::decode_prefix_with_items
            ),
            global_ai: component!(
                Kind::GlobalAi,
                game_global_ai::GlobalAi::decode_prefix_with_items
            ),
            barbarian: component!(
                Kind::Barbarian,
                game_barbarian::Manager::decode_prefix_with_items
            ),
            free_cities: component!(
                Kind::FreeCities,
                game_free_cities::Manager::decode_prefix_with_items
            ),
            random: component!(
                Kind::Random,
                game_random_wire::Random::decode_prefix_with_items
            ),
        };
        let world = world::World::new(map, players).map_err(|e| e.to_string())?;
        LoadedState::from_typed_components(
            CoreImage::Unique(self.source),
            binding,
            world,
            Some(retained),
            limits.assembly,
        )
        .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Framing-only synthetic bytes deliberately do not impersonate game records.
    fn framing() -> Input {
        let kinds = [
            Kind::CorePrefix,
            Kind::Map,
            Kind::Religion,
            Kind::GlobalAi,
            Kind::Barbarian,
            Kind::FreeCities,
            Kind::Random,
        ];
        let source = vec![0; kinds.len()];
        Input {
            schema: SCHEMA.into(),
            binding: Binding {
                source_sha256: Sha256::digest(&source).into(),
                occurrence_id: b"synthetic-occurrence".to_vec(),
                output_id: b"synthetic-output".to_vec(),
                configuration_sha256: [1; 32],
                rules_inputs_sha256: [2; 32],
                database_sha256: None,
                dlc_inputs_sha256: [3; 32],
                mods_inputs_sha256: [4; 32],
                content_inputs_sha256: [5; 32],
            },
            source,
            components: kinds
                .into_iter()
                .enumerate()
                .map(|(i, component)| Component {
                    component,
                    range: [i, i + 1],
                })
                .collect(),
            players: vec![],
        }
    }

    #[test]
    fn framing_roundtrip_is_distinct_from_successful_component_decode() {
        let input = framing();
        let encoded = serde_json::to_vec(&input).unwrap();
        let parsed = Input::from_json(&encoded, &Limits::default()).unwrap();
        assert_eq!(parsed.source, input.source);
        assert_eq!(
            parsed.binding.wire().role,
            init::WireRole::SavedOutputComponents
        );
        assert!(parsed.decode(&Limits::default()).is_err());
    }

    #[test]
    fn rejects_unknown_schema_fields_identity_and_resource_limits() {
        let mut input = framing();
        input.schema.push('x');
        assert!(input.validate(&Limits::default()).is_err());
        let mut input = framing();
        input.source[0] = 1;
        assert!(input.validate(&Limits::default()).is_err());
        let mut input = framing();
        input.binding.output_id.clear();
        assert!(input.validate(&Limits::default()).is_err());
        let mut input = framing();
        input.binding.configuration_sha256 = [0; 32];
        assert!(input.validate(&Limits::default()).is_err());
        let mut json = serde_json::to_value(framing()).unwrap();
        json["native_address"] = 123.into();
        assert!(Input::from_json(&serde_json::to_vec(&json).unwrap(), &Limits::default()).is_err());
        let limits = Limits {
            max_json_bytes: 1,
            ..Limits::default()
        };
        assert!(Input::from_json(b"{}", &limits).is_err());
        let limits = Limits {
            max_source_bytes: 1,
            ..Limits::default()
        };
        assert!(framing().validate(&limits).is_err());
    }

    #[test]
    fn rejects_missing_duplicate_overlapping_and_out_of_bounds_ranges() {
        let mut input = framing();
        input.components.pop();
        assert!(input.validate(&Limits::default()).is_err());
        let mut input = framing();
        input.components[1].component = Kind::CorePrefix;
        assert!(input.validate(&Limits::default()).is_err());
        for range in [[0, 2], [1, 1], [1, usize::MAX]] {
            let mut input = framing();
            input.components[1].range = range;
            assert!(input.validate(&Limits::default()).is_err());
        }
    }

    #[test]
    fn player_context_must_match_owner_manager_and_contained_military_range() {
        let mut input = framing();
        input.components.push(Component {
            component: Kind::Player {
                owner: 0,
                manager_index: 0,
            },
            range: [7, 9],
        });
        input.source.extend_from_slice(&[0, 0]);
        input.binding.source_sha256 = Sha256::digest(&input.source).into();
        assert!(input.validate(&Limits::default()).is_err());
        input.players.push(PlayerContext {
            owner: 0,
            manager_index: 0,
            military_range: None,
            was_ever_alive_raw: 0,
            handicap_i32: 0,
            allowed: Vector { values_i32: vec![] },
            per_war: Vector { values_i32: vec![] },
            current: Vector { values_i32: vec![] },
            definitions: None,
        });
        assert!(input.validate(&Limits::default()).is_ok());
        input.players[0].manager_index = 1;
        assert!(input.validate(&Limits::default()).is_err());
        input.players[0].manager_index = 0;
        input.players[0].military_range = Some([6, 8]);
        assert!(input.validate(&Limits::default()).is_err());
        input.players[0].military_range = None;
        input.players.push(input.players[0].clone());
        assert!(input.validate(&Limits::default()).is_err());
    }
}
