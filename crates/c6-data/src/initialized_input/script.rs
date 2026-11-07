//! Owned initialized script semantics, independent of native VM layouts.
//!
//! Object IDs are logical identities, not native addresses. Every reached
//! group's order and listener length is supplied explicitly. Missing name-map
//! entries remain unknown; unobserved execution and RNG facts stay unavailable.
//! This transport does not authenticate observations or implement a general VM.
use crate::loaded_actions::transition::{ScriptState, ScriptSystemIdentity};
use c6_core::{loaded_state::LoadedState, lua_math_random, script_event_registry as registry};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u32 = 1;
pub type Result<T> = std::result::Result<T, String>;

/// Required explicit knowledge: omission is never interpreted as an empty or
/// default observed object.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Facts<T> {
    Unobserved {},
    Supplied { value: T },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema_version: u32,
    pub system_identity: u64,
    pub registry: Registry,
    pub execution: Facts<Execution>,
    pub factory: Factory,
    pub intern: Facts<Intern>,
    pub memberships: Memberships,
    pub math_rng: Facts<MathRng>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub names: Vec<Name>,
    pub shared_groups: Vec<SharedGroup>,
    pub local_groups: Vec<LocalGroup>,
    pub cache: Vec<CachedEvent>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Name {
    pub name: String,
    pub entry: NameEntry,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NameEntry {
    Absent {},
    Shared { id: u64 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedGroup {
    pub id: u64,
    pub entries_in_native_order: Vec<GroupEntry>,
    pub append_order: AppendOrder,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AppendOrder {
    DenseNumericArray {},
    KnownNextPosition { index: usize },
    Unknown {},
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GroupEntry {
    Table { id: u64 },
    NonTable { value: Value },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Value {
    Nil {},
    Boolean { value: bool },
    Number { value: f64 },
    String { bytes: Vec<u8> },
    Object { id: u64 },
}
impl Value {
    fn build(&self) -> Result<registry::LuaValue> {
        Ok(match self {
            Self::Nil {} => registry::LuaValue::Nil,
            Self::Boolean { value } => registry::LuaValue::Boolean(*value),
            Self::Number { value } if value.is_finite() => registry::LuaValue::Number(*value),
            Self::Number { .. } => return Err("nonfinite script number".into()),
            Self::String { bytes } => registry::LuaValue::String(bytes.clone()),
            Self::Object { id } => registry::LuaValue::Object(registry::ScriptObjectId(*id)),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalGroup {
    pub id: u64,
    pub objlen: usize,
    /// Omitted slots within the explicitly supplied length are known nil, as in
    /// the logical registry constructor; omitting the group itself is unknown.
    pub numeric_slots: Vec<Slot>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub index: usize,
    pub value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CachedEvent {
    pub name: String,
    pub identity: EventIdentity,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventIdentity {
    pub event: u64,
    pub shared_group: u64,
    pub local_group: u64,
    pub test_all: u64,
    /// Closure identities in `script_event_registry::REGISTERED_METHODS` order.
    pub methods: [u64; 9],
}
impl EventIdentity {
    fn build(&self) -> registry::EventIdentity {
        registry::EventIdentity {
            event: registry::ScriptObjectId(self.event),
            shared_group: registry::ScriptObjectId(self.shared_group),
            local_group: registry::ScriptObjectId(self.local_group),
            test_all: registry::ScriptObjectId(self.test_all),
            methods: self.methods.map(registry::ScriptObjectId),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallCapacity {
    Unknown,
    NoGrowthForThreeIntegers,
    NoGrowthForFactoryThreeIntegers,
    NoGrowthForFactorySixIntegers,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub call_capacity: CallCapacity,
    pub collector_phase: u32,
    pub step_countdown: i32,
    pub allocated_bytes: u64,
    pub peak_allocated_bytes: u64,
    pub collection_threshold: u64,
    pub collection_inhibited: bool,
    pub interned_names: Vec<String>,
    pub protected_closure_obligations: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Factory {
    pub game_event_spare_names: Vec<String>,
    pub name_map_spare_names: Vec<String>,
    pub shared_spare_slots: Vec<SpareSlot>,
    pub weak_shared_groups: Vec<u64>,
    /// Parsed and retained by transport, but nonempty lists currently refuse
    /// `Input::build`: factory-only object identity reservation is unsupported.
    pub factory_objects: Vec<FactoryObjects>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpareSlot {
    pub group: u64,
    pub index: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactoryObjects {
    pub event: EventIdentity,
    pub protected_closure: u64,
    pub factory_closure: u64,
    pub event_metatable: u64,
    pub protected_metatable: u64,
    pub weak_metatable: OptionalObject,
    pub swept_construction_order: Vec<u64>,
}
/// A known absent/present optional object, distinct from unobserved VM facts.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OptionalObject {
    Absent {},
    Present { id: u64 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intern {
    pub count: u32,
    pub capacity: u32,
    pub scan_index: Facts<u32>,
    pub known_absent: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Memberships {
    /// Retain supplied hit/miss facts; all other names remain unknown.
    Single {},
    Alternatives {
        unknown_names: Vec<String>,
        namespace: Facts<Namespace>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Namespace {
    pub capacity: u32,
    pub occupied: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MathRngKind {
    Lcg,
    Additive7,
    Additive15,
    Additive31,
    Additive63,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MathRng {
    pub kind: MathRngKind,
    pub words: Vec<u32>,
    pub front: usize,
    pub rear: usize,
    pub state_header: u32,
}
impl MathRng {
    fn build(&self) -> Result<lua_math_random::State> {
        let kind = match self.kind {
            MathRngKind::Lcg => lua_math_random::Kind::Lcg,
            MathRngKind::Additive7 => lua_math_random::Kind::Additive7,
            MathRngKind::Additive15 => lua_math_random::Kind::Additive15,
            MathRngKind::Additive31 => lua_math_random::Kind::Additive31,
            MathRngKind::Additive63 => lua_math_random::Kind::Additive63,
        };
        lua_math_random::State::from_parts(kind, self.words.clone(), self.front, self.rear)
            .map_err(|_| "invalid script math RNG words or cursors".into())
    }
}
fn map<K: Ord, V>(
    entries: impl IntoIterator<Item = (K, V)>,
    label: &str,
) -> Result<BTreeMap<K, V>> {
    let mut result = BTreeMap::new();
    for (key, value) in entries {
        if result.insert(key, value).is_some() {
            return Err(format!("duplicate {label}"));
        }
    }
    Ok(result)
}
fn set<T: Ord + Clone>(entries: &[T], label: &str) -> Result<BTreeSet<T>> {
    let result: BTreeSet<_> = entries.iter().cloned().collect();
    if result.len() != entries.len() {
        return Err(format!("duplicate {label}"));
    }
    Ok(result)
}
impl Registry {
    pub fn build(&self) -> Result<registry::ScriptEventRegistry> {
        let names = map(
            self.names.iter().map(|row| {
                (
                    row.name.clone(),
                    match row.entry {
                        NameEntry::Absent {} => registry::KnownNameEntry::Absent,
                        NameEntry::Shared { id } => {
                            registry::KnownNameEntry::Shared(registry::ScriptObjectId(id))
                        }
                    },
                )
            }),
            "script name",
        )?;
        let shared_groups = map(
            self.shared_groups
                .iter()
                .map(|row| {
                    let entries = row
                        .entries_in_native_order
                        .iter()
                        .map(|entry| {
                            Ok(match entry {
                                GroupEntry::Table { id } => {
                                    registry::GroupEntry::Table(registry::ScriptObjectId(*id))
                                }
                                GroupEntry::NonTable { value } => {
                                    registry::GroupEntry::NonTable(value.build()?)
                                }
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let append_order = match row.append_order {
                        AppendOrder::DenseNumericArray {} => {
                            registry::AppendOrder::DenseNumericArray
                        }
                        AppendOrder::KnownNextPosition { index } => {
                            registry::AppendOrder::KnownNextPosition(index)
                        }
                        AppendOrder::Unknown {} => registry::AppendOrder::Unknown,
                    };
                    Ok((
                        registry::ScriptObjectId(row.id),
                        registry::SharedGroup {
                            entries_in_native_order: entries,
                            append_order,
                        },
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
            "script shared group",
        )?;
        let local_groups = map(
            self.local_groups
                .iter()
                .map(|row| {
                    let slots = row
                        .numeric_slots
                        .iter()
                        .map(|s| Ok((s.index, s.value.build()?)))
                        .collect::<Result<Vec<_>>>()?;
                    Ok((
                        registry::ScriptObjectId(row.id),
                        registry::LocalGroup {
                            objlen: row.objlen,
                            numeric_slots: map(slots, "script numeric slot")?,
                        },
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
            "script local group",
        )?;
        let cache = map(
            self.cache
                .iter()
                .map(|row| (row.name.clone(), row.identity.build())),
            "script cache name",
        )?;
        registry::ScriptEventRegistry::from_seed_with_cache(
            registry::RegistrySeed {
                names,
                shared_groups,
                local_groups,
            },
            cache,
        )
        .map_err(|e| format!("script registry: {e:?}"))
    }
}
impl Execution {
    fn build(&self) -> Result<registry::CachedExecutionState> {
        Ok(registry::CachedExecutionState {
            call_capacity: match self.call_capacity {
                CallCapacity::Unknown => registry::CachedCallCapacity::Unknown,
                CallCapacity::NoGrowthForThreeIntegers => {
                    registry::CachedCallCapacity::NoGrowthForThreeIntegers
                }
                CallCapacity::NoGrowthForFactoryThreeIntegers => {
                    registry::CachedCallCapacity::NoGrowthForFactoryThreeIntegers
                }
                CallCapacity::NoGrowthForFactorySixIntegers => {
                    registry::CachedCallCapacity::NoGrowthForFactorySixIntegers
                }
            },
            collector_phase: self.collector_phase,
            step_countdown: self.step_countdown,
            allocated_bytes: self.allocated_bytes,
            peak_allocated_bytes: self.peak_allocated_bytes,
            collection_threshold: self.collection_threshold,
            collection_inhibited: self.collection_inhibited,
            interned_names: set(&self.interned_names, "script interned name")?,
            protected_closure_obligations: self.protected_closure_obligations,
        })
    }
}
impl Factory {
    fn build(&self) -> Result<registry::FactoryExecutionState> {
        Ok(registry::FactoryExecutionState {
            game_event_spare_names: set(&self.game_event_spare_names, "script spare event name")?,
            name_map_spare_names: set(&self.name_map_spare_names, "script spare name-map name")?,
            shared_spare_slots: map(
                self.shared_spare_slots
                    .iter()
                    .map(|s| (registry::ScriptObjectId(s.group), s.index)),
                "script shared spare slot",
            )?,
            weak_shared_groups: set(&self.weak_shared_groups, "script weak shared group")?
                .into_iter()
                .map(registry::ScriptObjectId)
                .collect(),
            factory_objects: self
                .factory_objects
                .iter()
                .map(|o| registry::FactoryObjects {
                    event: o.event.build(),
                    protected_closure: registry::ScriptObjectId(o.protected_closure),
                    factory_closure: registry::ScriptObjectId(o.factory_closure),
                    event_metatable: registry::ScriptObjectId(o.event_metatable),
                    protected_metatable: registry::ScriptObjectId(o.protected_metatable),
                    weak_metatable: match o.weak_metatable {
                        OptionalObject::Absent {} => None,
                        OptionalObject::Present { id } => Some(registry::ScriptObjectId(id)),
                    },
                    swept_construction_order: o
                        .swept_construction_order
                        .iter()
                        .copied()
                        .map(registry::ScriptObjectId)
                        .collect(),
                })
                .collect(),
        })
    }
}
impl Intern {
    fn build(&self) -> Result<registry::InternExecutionState> {
        Ok(registry::InternExecutionState {
            count: self.count,
            capacity: self.capacity,
            scan_index: match self.scan_index {
                Facts::Unobserved {} => None,
                Facts::Supplied { value } => Some(value),
            },
            known_absent: set(&self.known_absent, "script absent intern name")?,
        })
    }
}
impl Input {
    fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err("script input schema version".into());
        }
        // The registry constructor reserves graph/cache identities, but cannot
        // yet reserve factory-only temporary/metatable identities. Admitting
        // these obligations could alias them on a subsequent factory allocation.
        if !self.factory.factory_objects.is_empty() {
            return Err("importing retained script factory objects is unsupported".into());
        }
        Ok(())
    }
    /// Bind supplied semantics to this initialized state without manufacturing
    /// observation attribution. Unsupported execution facts remain unavailable.
    pub fn build(&self, state: &LoadedState) -> Result<ScriptState> {
        self.validate()?;
        if !state.is_pristine() {
            return Err("script input requires initialized pristine state".into());
        }
        let registry = self.registry.build()?;
        let factory = self.factory.build()?;
        let mut script = match &self.execution {
            Facts::Unobserved {} => {
                if matches!(self.intern, Facts::Supplied { .. }) {
                    return Err("script intern facts require execution facts".into());
                }
                ScriptState::from_semantic_registry(
                    state.binding().clone(),
                    state.observed_snapshot().cloned(),
                    ScriptSystemIdentity(self.system_identity),
                    registry,
                    factory,
                )
            }
            Facts::Supplied { value } => ScriptState::from_supplied(
                state.binding().clone(),
                state.observed_snapshot().cloned(),
                ScriptSystemIdentity(self.system_identity),
                registry,
                value.build()?,
                factory,
                match &self.intern {
                    Facts::Unobserved {} => None,
                    Facts::Supplied { value } => Some(value.build()?),
                },
            ),
        };
        if let Memberships::Alternatives {
            unknown_names,
            namespace,
        } = &self.memberships
        {
            let namespace = match namespace {
                Facts::Unobserved {} => None,
                Facts::Supplied { value } => Some((value.capacity, value.occupied)),
            };
            script = script
                .with_unknown_memberships(unknown_names, namespace)
                .map_err(|e| format!("script memberships: {e:?}"))?;
        }
        if let Facts::Supplied { value } = &self.math_rng {
            script
                .bind_supplied_math_rng(state, value.build()?, value.state_header)
                .map_err(|e| e.to_string())?;
        }
        Ok(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> Registry {
        Registry {
            names: vec![],
            shared_groups: vec![SharedGroup {
                id: 1,
                entries_in_native_order: vec![GroupEntry::Table { id: 2 }],
                append_order: AppendOrder::Unknown {},
            }],
            local_groups: vec![LocalGroup {
                id: 2,
                objlen: 0,
                numeric_slots: vec![],
            }],
            cache: vec![CachedEvent {
                name: "CanStartOperation".into(),
                identity: EventIdentity {
                    event: 3,
                    shared_group: 1,
                    local_group: 2,
                    test_all: 9,
                    methods: [4, 5, 6, 7, 8, 9, 10, 11, 12],
                },
            }],
        }
    }
    fn execution() -> Execution {
        Execution {
            call_capacity: CallCapacity::NoGrowthForThreeIntegers,
            collector_phase: 0,
            step_countdown: 5,
            allocated_bytes: 100,
            peak_allocated_bytes: 100,
            collection_threshold: 100,
            collection_inhibited: false,
            interned_names: vec![
                "GameEvents".into(),
                "CanStartOperation".into(),
                "TestAll".into(),
            ],
            protected_closure_obligations: 0,
        }
    }
    #[test]
    fn cached_graph_preserves_unknown_names_and_computes_execution() {
        let graph = graph();
        let registry = graph.build().unwrap();
        let mut effects = execution().build().unwrap();
        assert_eq!(
            registry.cached_event("CanStartOperation"),
            Some(graph.cache[0].identity.build())
        );
        assert_eq!(
            registry.test_all_cached("CanStartOperation", [0, 4, 36], &mut effects),
            Ok(None)
        );
        assert_eq!(effects.allocated_bytes, 148);
        assert_eq!(effects.step_countdown, 4);
        assert_eq!(effects.protected_closure_obligations, 1);
        assert!(matches!(
            registry.test_all_cached("UnknownEvent", [0, 4, 36], &mut effects),
            Err(registry::RegistryError::UnknownNameMapEntry(_))
        ));
    }
    #[test]
    fn listeners_are_preserved_and_never_assumed_empty() {
        let mut graph = graph();
        graph.local_groups[0].objlen = 1;
        graph.local_groups[0].numeric_slots.push(Slot {
            index: 1,
            value: Value::Object { id: 77 },
        });
        let registry = graph.build().unwrap();
        assert_eq!(
            registry
                .local_group(registry::ScriptObjectId(2))
                .unwrap()
                .numeric_slots[&1],
            registry::LuaValue::Object(registry::ScriptObjectId(77))
        );
        let mut effects = execution().build().unwrap();
        let before = effects.clone();
        assert!(registry
            .test_all_cached("CanStartOperation", [0, 4, 36], &mut effects)
            .is_err());
        assert_eq!(effects, before);
    }
    #[test]
    fn duplicate_transport_entries_and_invalid_graphs_are_rejected() {
        let mut graph = graph();
        graph.cache.push(graph.cache[0].clone());
        assert!(graph
            .build()
            .unwrap_err()
            .contains("duplicate script cache name"));
        graph.cache.pop();
        graph.local_groups.push(graph.local_groups[0].clone());
        assert!(graph
            .build()
            .unwrap_err()
            .contains("duplicate script local group"));
        graph.local_groups.pop();
        graph.local_groups[0].numeric_slots = vec![
            Slot {
                index: 1,
                value: Value::Nil {},
            },
            Slot {
                index: 1,
                value: Value::Boolean { value: false },
            },
        ];
        assert!(graph
            .build()
            .unwrap_err()
            .contains("duplicate script numeric slot"));
        graph.local_groups[0].numeric_slots.clear();
        graph.cache[0].identity.test_all = 13;
        assert!(graph
            .build()
            .unwrap_err()
            .contains("TestAll identity mismatch"));
    }
    #[test]
    fn serde_requires_explicit_knowledge_and_complete_listener_shape() {
        assert!(
            serde_json::from_str::<Facts<Execution>>(r#"{"status":"unobserved","value":{}}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<LocalGroup>(r#"{"id":2,"numeric_slots":[]}"#).is_err());
        assert!(serde_json::from_str::<LocalGroup>(
            r#"{"id":2,"objlen":0,"numeric_slots":[],"listeners_empty":true}"#
        )
        .is_err());
        let input = Input {
            schema_version: SCHEMA_VERSION,
            system_identity: 55,
            registry: graph(),
            execution: Facts::Unobserved {},
            factory: Factory {
                game_event_spare_names: vec![],
                name_map_spare_names: vec![],
                shared_spare_slots: vec![],
                weak_shared_groups: vec![],
                factory_objects: vec![],
            },
            intern: Facts::Unobserved {},
            memberships: Memberships::Single {},
            math_rng: Facts::Unobserved {},
        };
        assert!(serde_json::from_str::<Value>(r#"{"kind":"nil","extra":0}"#).is_err());
        assert!(serde_json::from_str::<NameEntry>(r#"{"kind":"absent","id":1}"#).is_err());
        assert!(serde_json::from_str::<AppendOrder>(r#"{"kind":"unknown","index":0}"#).is_err());
        assert!(
            serde_json::from_str::<Memberships>(r#"{"kind":"single","unknown_names":[]}"#).is_err()
        );
        let mut value = serde_json::to_value(input).unwrap();
        let decoded: Input = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(decoded.execution, Facts::Unobserved {}));
        assert!(matches!(decoded.math_rng, Facts::Unobserved {}));
        value.as_object_mut().unwrap().remove("math_rng");
        assert!(serde_json::from_value::<Input>(value).is_err());
    }
    #[test]
    fn exact_rng_transport_preserves_nondefault_words_and_cursor() {
        let mut input = MathRng {
            kind: MathRngKind::Additive7,
            words: vec![u32::MAX, 2, 3, 4, 5, 6, 7],
            front: 5,
            rear: 2,
            state_header: 91,
        };
        let state = input.build().unwrap();
        assert_eq!(state.words(), input.words);
        assert_eq!((state.front(), state.rear()), (5, 2));
        input.rear = 1;
        assert!(input.build().is_err());
    }
    #[test]
    fn explicit_intern_and_factory_obligations_survive_conversion() {
        let intern = Intern {
            count: 42,
            capacity: 128,
            scan_index: Facts::Unobserved {},
            known_absent: vec!["Absent".into()],
        }
        .build()
        .unwrap();
        assert_eq!(intern.count, 42);
        assert_eq!(intern.scan_index, None);
        assert!(intern.known_absent.contains("Absent"));
        let objects = FactoryObjects {
            event: graph().cache[0].identity.clone(),
            protected_closure: 15,
            factory_closure: 16,
            event_metatable: 17,
            protected_metatable: 18,
            weak_metatable: OptionalObject::Present { id: 19 },
            swept_construction_order: vec![3, 17, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 15, 18, 19],
        };
        let factory_input = Factory {
            game_event_spare_names: vec!["Next".into()],
            name_map_spare_names: vec!["Next".into()],
            shared_spare_slots: vec![SpareSlot { group: 1, index: 2 }],
            weak_shared_groups: vec![1],
            factory_objects: vec![objects],
        };
        let input = Input {
            schema_version: SCHEMA_VERSION,
            system_identity: 55,
            registry: graph(),
            execution: Facts::Unobserved {},
            factory: factory_input.clone(),
            intern: Facts::Unobserved {},
            memberships: Memberships::Single {},
            math_rng: Facts::Unobserved {},
        };
        assert_eq!(
            input.validate().unwrap_err(),
            "importing retained script factory objects is unsupported"
        );
        let factory = factory_input.build().unwrap();
        assert_eq!(factory.shared_spare_slots[&registry::ScriptObjectId(1)], 2);
        assert_eq!(
            factory.factory_objects[0].weak_metatable,
            Some(registry::ScriptObjectId(19))
        );
        assert_eq!(
            factory.factory_objects[0].swept_construction_order.last(),
            Some(&registry::ScriptObjectId(19))
        );
    }
}
