//! Bounded logical GameEvents graph for the pinned Linux host.
//!
//! This models source-backed factory/cache mutations and builtin TestAll control
//! flow. The cached three-integer no-listener frontier additionally evolves
//! bounded allocator/collector accounting. This is not a complete HKS heap,
//! collector, callback VM, action executor, or turn implementation. Unknown
//! name-map entries are never interpreted as absent.

use std::collections::BTreeMap;

/// Implementation admission bounds, not claims about native HKS integer width.
pub const MAX_GROUP_ENTRIES: usize = 65_536;
pub const MAX_REACHED_CALLBACK_SLOTS: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ScriptObjectId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(Vec<u8>),
    Object(ScriptObjectId),
}

impl LuaValue {
    pub fn truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    UnknownNameMapEntry(String),
    UnknownObject(ScriptObjectId),
    InvalidSeed(&'static str),
    UnsupportedCallbackVm,
    UnsupportedEffect(&'static str),
    IdentityExhausted,
}

/// Each name must be explicitly known absent or linked to a known shared group.
/// Absence of a key in this input means unknown, not absent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegistrySeed {
    pub names: BTreeMap<String, KnownNameEntry>,
    pub shared_groups: BTreeMap<ScriptObjectId, SharedGroup>,
    pub local_groups: BTreeMap<ScriptObjectId, LocalGroup>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KnownNameEntry {
    Absent,
    Shared(ScriptObjectId),
}

/// Entries must already be in the actual native lua_next order. This is not a
/// sorted map, array traversal, or permission to infer order from addresses.
#[derive(Clone, Debug, PartialEq)]
pub struct SharedGroup {
    pub entries_in_native_order: Vec<GroupEntry>,
    pub append_order: AppendOrder,
}

/// Source-closed placement of lCreate's objlen+1 insertion in native next order.
/// A captured order alone does not establish where a new entry will traverse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppendOrder {
    /// Proven complete dense numeric array, without hash entries or holes.
    DenseNumericArray,
    /// Known placement for this one append; subsequent placement becomes unknown.
    KnownNextPosition(usize),
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GroupEntry {
    Table(ScriptObjectId),
    NonTable(LuaValue),
}

/// `objlen` is known native/source-closed state, not inferred from slot count.
/// Missing numeric slots are nil. Sparse arrays require their actual objlen.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalGroup {
    pub objlen: usize,
    pub numeric_slots: BTreeMap<usize, LuaValue>,
}

pub const REGISTERED_METHODS: [&str; 9] = [
    "Add",
    "Remove",
    "RemoveAll",
    "Count",
    "Call",
    "TestAll",
    "TestAny",
    "Accumulate",
    "AccumulateINT",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventIdentity {
    pub event: ScriptObjectId,
    pub shared_group: ScriptObjectId,
    pub local_group: ScriptObjectId,
    /// Stable logical identity of the freshly registered builtin TestAll closure.
    pub test_all: ScriptObjectId,
    /// Logical closure identities in REGISTERED_METHODS order. Other methods
    /// are registered but their execution remains unsupported here.
    pub methods: [ScriptObjectId; 9],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptEventRegistry {
    names: BTreeMap<String, KnownNameEntry>,
    cache: BTreeMap<String, EventIdentity>,
    shared_groups: BTreeMap<ScriptObjectId, SharedGroup>,
    local_groups: BTreeMap<ScriptObjectId, LocalGroup>,
    next_identity: u64,
}

/// Evolving ordinary-allocator projection for the source-closed cached,
/// phase-zero, three-integer, no-listener call path. Initial state must establish
/// the ordinary allocator context; custom allocator callbacks are unsupported.
/// These are semantic GC obligations, not native addresses, allocator bins, or
/// VM stack layouts.
///
/// Interned names must describe the current intern table, not names observed in
/// an unrelated capture. This model cannot intern a missing string, advance a
/// collector, or discharge temporary swept allocations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedExecutionState {
    pub call_capacity: CachedCallCapacity,
    pub collector_phase: u32,
    /// Native signed countdown, updated with 32-bit wrapping subtraction.
    pub step_countdown: i32,
    pub allocated_bytes: u64,
    pub peak_allocated_bytes: u64,
    pub collection_threshold: u64,
    pub collection_inhibited: bool,
    pub interned_names: std::collections::BTreeSet<String>,
    /// Number of still-linked zero-upvalue protected-call closures created by
    /// this projection. Each remains a 48-byte swept allocation after return.
    /// Existing allocations remain represented in allocated_bytes.
    pub protected_closure_obligations: u64,
}

/// Semantic capacity fact for this call shape, established from the initial VM
/// state. Ordinary return preserves it. Other VM activity must invalidate or
/// reestablish it. Unknown capacity cannot omit growth's allocation/GC effects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachedCallCapacity {
    Unknown,
    NoGrowthForThreeIntegers,
}

impl CachedExecutionState {
    fn admit(&self, name: &str) -> Result<(), RegistryError> {
        if self.call_capacity != CachedCallCapacity::NoGrowthForThreeIntegers {
            return Err(RegistryError::UnsupportedEffect(
                "unknown call growth effects",
            ));
        }
        if self.collector_phase != 0 {
            return Err(RegistryError::UnsupportedEffect(
                "collector phase outside cached frontier",
            ));
        }
        for key in ["GameEvents", name, "TestAll"] {
            if !self.interned_names.contains(key) {
                return Err(RegistryError::UnsupportedEffect("string intern miss"));
            }
        }
        // checkStep(1) tests the OLD signed countdown and OLD allocation total.
        if self.step_countdown <= 1
            && self.allocated_bytes > self.collection_threshold
            && !self.collection_inhibited
        {
            return Err(RegistryError::UnsupportedEffect("collector progression"));
        }
        if self.allocated_bytes.checked_add(48).is_none()
            || self.protected_closure_obligations.checked_add(1).is_none()
        {
            return Err(RegistryError::UnsupportedEffect(
                "cached allocation accounting overflow",
            ));
        }
        Ok(())
    }

    fn allocate_protected_closure(&mut self) {
        self.step_countdown = self.step_countdown.wrapping_sub(1);
        self.allocated_bytes += 48;
        self.peak_allocated_bytes = self.peak_allocated_bytes.max(self.allocated_bytes);
        self.protected_closure_obligations += 1;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProtectedCallbackResult {
    Returned(LuaValue),
    /// Native protected callback failure: continue without changing aggregate.
    Failed,
}

/// Narrow callback effects. It intentionally exposes neither factory lookup,
/// structural changes nor nested TestAll queries on the executing registry.
pub struct CallbackEffects<'a> {
    registry: &'a mut ScriptEventRegistry,
}
impl CallbackEffects<'_> {
    pub fn replace_existing_slot(
        &mut self,
        local: ScriptObjectId,
        index: usize,
        value: LuaValue,
    ) -> Result<(), RegistryError> {
        self.registry.replace_existing_slot(local, index, value)
    }
}

/// Separately evidenced callback provider. Admission is effect-free and runs
/// before lookup mutation, then again for each live slot before execution.
/// Native protected failure is `ProtectedCallbackResult::Failed`; simulator
/// unsupported/provider failure is Err and must never be converted to Failed.
/// This interface admits only the bounded logical projection, not HKS heap/GC.
pub trait CallbackVm {
    fn admit_callback(&self, _: &LuaValue, _: &[i32]) -> Result<(), RegistryError> {
        Err(RegistryError::UnsupportedCallbackVm)
    }
    fn protected_call(
        &mut self,
        effects: &mut CallbackEffects<'_>,
        callback: LuaValue,
        arguments: &[i32],
    ) -> Result<ProtectedCallbackResult, RegistryError>;
}

pub struct UnsupportedCallbackVm;
impl CallbackVm for UnsupportedCallbackVm {
    fn protected_call(
        &mut self,
        _: &mut CallbackEffects<'_>,
        _: LuaValue,
        _: &[i32],
    ) -> Result<ProtectedCallbackResult, RegistryError> {
        Err(RegistryError::UnsupportedCallbackVm)
    }
}

impl Default for ScriptEventRegistry {
    fn default() -> Self {
        Self::from_seed(RegistrySeed::default()).expect("empty unknown seed is valid")
    }
}

impl ScriptEventRegistry {
    /// General live execution remains unsupported. The separate cached-call
    /// frontier requires explicit current collector/allocator state and does
    /// not admit factories, collector progression, or callbacks.
    pub fn require_live_execution_support(&self) -> Result<(), RegistryError> {
        Err(RegistryError::UnsupportedEffect(
            "HKS allocator/GC live execution",
        ))
    }

    /// Inputs describe known source state; this does not authenticate a capture.
    /// In particular, native-3 does not supply its indexer name-map pointees.
    pub fn from_seed(seed: RegistrySeed) -> Result<Self, RegistryError> {
        if seed
            .shared_groups
            .keys()
            .any(|id| seed.local_groups.contains_key(id))
        {
            return Err(RegistryError::InvalidSeed("object identities overlap"));
        }
        for entry in seed.names.values() {
            if let KnownNameEntry::Shared(id) = entry {
                if !seed.shared_groups.contains_key(id) {
                    return Err(RegistryError::UnknownObject(*id));
                }
            }
        }
        if seed
            .local_groups
            .values()
            .any(|g| g.objlen > MAX_REACHED_CALLBACK_SLOTS)
        {
            return Err(RegistryError::InvalidSeed(
                "callback objlen exceeds bounded projection",
            ));
        }
        for group in seed.shared_groups.values() {
            if group.entries_in_native_order.len() > MAX_GROUP_ENTRIES {
                return Err(RegistryError::InvalidSeed(
                    "group entries exceed bounded projection",
                ));
            }
            let mut reached_slots = 0usize;
            if matches!(group.append_order, AppendOrder::KnownNextPosition(i) if i > group.entries_in_native_order.len())
            {
                return Err(RegistryError::InvalidSeed(
                    "append position outside group order",
                ));
            }
            for entry in &group.entries_in_native_order {
                if let GroupEntry::NonTable(LuaValue::Object(id)) = entry {
                    if seed.shared_groups.contains_key(id) || seed.local_groups.contains_key(id) {
                        return Err(RegistryError::InvalidSeed(
                            "non-table value names a known table identity",
                        ));
                    }
                }
                if let GroupEntry::Table(id) = entry {
                    let local = seed
                        .local_groups
                        .get(id)
                        .ok_or(RegistryError::UnknownObject(*id))?;
                    reached_slots = reached_slots
                        .checked_add(local.objlen)
                        .filter(|n| *n <= MAX_REACHED_CALLBACK_SLOTS)
                        .ok_or(RegistryError::InvalidSeed(
                            "reached callback slots exceed bounded projection",
                        ))?;
                }
            }
        }
        if seed
            .local_groups
            .values()
            .any(|g| g.numeric_slots.contains_key(&0))
        {
            return Err(RegistryError::InvalidSeed(
                "numeric slot zero is outside bounded arrays",
            ));
        }
        let max_id = seed
            .shared_groups
            .keys()
            .chain(seed.local_groups.keys())
            .map(|id| id.0)
            .max()
            .unwrap_or(0);
        // Object values refer to external closure/VM identities. Reserve those
        // identities too, so freshly constructed tables cannot alias them.
        let value_ids = seed
            .local_groups
            .values()
            .flat_map(|g| g.numeric_slots.values())
            .chain(
                seed.shared_groups
                    .values()
                    .flat_map(|g| g.entries_in_native_order.iter())
                    .filter_map(|e| match e {
                        GroupEntry::NonTable(v) => Some(v),
                        _ => None,
                    }),
            )
            .filter_map(|v| match v {
                LuaValue::Object(id) => Some(id.0),
                _ => None,
            });
        let max_id = value_ids.fold(max_id, u64::max);
        let next_identity = max_id
            .checked_add(1)
            .ok_or(RegistryError::IdentityExhausted)?;
        Ok(Self {
            names: seed.names,
            cache: BTreeMap::new(),
            shared_groups: seed.shared_groups,
            local_groups: seed.local_groups,
            next_identity,
        })
    }

    /// Import explicitly known cached event identities. This validates the
    /// logical graph; it neither authenticates a fixture nor establishes HKS
    /// collector, allocator, global-table, or string-intern state.
    pub fn from_seed_with_cache(
        seed: RegistrySeed,
        cache: BTreeMap<String, EventIdentity>,
    ) -> Result<Self, RegistryError> {
        let mut registry = Self::from_seed(seed)?;
        let mut owned: std::collections::BTreeSet<_> = registry
            .shared_groups
            .keys()
            .chain(registry.local_groups.keys())
            .copied()
            .collect();
        owned.extend(registry.local_groups.values().flat_map(|g| {
            g.numeric_slots.values().filter_map(|v| match v {
                LuaValue::Object(id) => Some(*id),
                _ => None,
            })
        }));
        owned.extend(registry.shared_groups.values().flat_map(|g| {
            g.entries_in_native_order.iter().filter_map(|e| match e {
                GroupEntry::NonTable(LuaValue::Object(id)) => Some(*id),
                _ => None,
            })
        }));
        let mut next_identity = registry.next_identity;
        for (name, event) in &cache {
            if registry.names.get(name) != Some(&KnownNameEntry::Shared(event.shared_group)) {
                return Err(RegistryError::InvalidSeed("cache name/group mismatch"));
            }
            let group = registry
                .shared_groups
                .get(&event.shared_group)
                .ok_or(RegistryError::UnknownObject(event.shared_group))?;
            if !registry.local_groups.contains_key(&event.local_group)
                || !group
                    .entries_in_native_order
                    .contains(&GroupEntry::Table(event.local_group))
            {
                return Err(RegistryError::InvalidSeed(
                    "cache local group is not linked",
                ));
            }
            if event.test_all != event.methods[5] {
                return Err(RegistryError::InvalidSeed(
                    "cache TestAll identity mismatch",
                ));
            }
            for id in std::iter::once(event.event).chain(event.methods) {
                if !owned.insert(id) {
                    return Err(RegistryError::InvalidSeed(
                        "cache object identities overlap",
                    ));
                }
                next_identity = next_identity.max(
                    id.0.checked_add(1)
                        .ok_or(RegistryError::IdentityExhausted)?,
                );
            }
        }
        registry.cache = cache;
        registry.next_identity = next_identity;
        Ok(registry)
    }

    pub fn cached_event(&self, name: &str) -> Option<EventIdentity> {
        self.cache.get(name).copied()
    }

    pub fn shared_group(&self, id: ScriptObjectId) -> Option<&SharedGroup> {
        self.shared_groups.get(&id)
    }

    pub fn local_group(&self, id: ScriptObjectId) -> Option<&LocalGroup> {
        self.local_groups.get(&id)
    }

    /// Execute the bounded cached builtin TestAll path with its evolving
    /// collector/allocator effects. A cache miss, string miss, nonempty reached
    /// group, or collector progression is refused before any mutation.
    ///
    /// The typed cache supplies the existing builtin fields; names are not
    /// treated as absent or synthesized. Native host stack/activation capacity
    /// belongs to oracle-context admission, not the simulator's object model.
    /// This is not support for complete live actions or arbitrary callbacks.
    pub fn test_all_cached(
        &self,
        name: &str,
        _arguments: [i32; 3],
        effects: &mut CachedExecutionState,
    ) -> Result<Option<bool>, RegistryError> {
        if name.as_bytes().contains(&0) {
            return Err(RegistryError::UnsupportedEffect(
                "embedded NUL in native field name",
            ));
        }
        let event = match self.cached_event(name) {
            Some(event) => event,
            None => {
                if !self.names.contains_key(name) {
                    return Err(RegistryError::UnknownNameMapEntry(name.to_owned()));
                }
                return Err(RegistryError::UnsupportedEffect("live event factory"));
            }
        };
        let mut any_nonempty = false;
        for entry in &self.shared_groups[&event.shared_group].entries_in_native_order {
            if let GroupEntry::Table(local) = entry {
                // Seed admission bounds objlen below i32::MAX. Thus the
                // target's signed low-32-bit <=0 test is exactly objlen==0
                // here, without silently reinterpreting unsupported lengths.
                any_nonempty |= self.local_groups[local].objlen != 0;
            }
        }
        if any_nonempty {
            return Err(RegistryError::UnsupportedCallbackVm);
        }
        effects.admit(name)?;
        // Source order: protected cclosure allocation precedes the three
        // intern-hit field lookups and empty TestAll traversal. Phase zero
        // intern hits and the reached empty groups have no further effects.
        effects.allocate_protected_closure();
        Ok(any_nonempty.then_some(true))
    }

    /// Source-backed ordinary lookup: indexer creates the shared group if known
    /// absent, creates a local array and event/method closure, and caches event.
    /// This is a logical projection; HKS allocation/GC effects remain unsupported.
    pub fn lookup(&mut self, name: &str) -> Result<EventIdentity, RegistryError> {
        if let Some(event) = self.cached_event(name) {
            return Ok(event);
        }
        let entry = self
            .names
            .get(name)
            .copied()
            .ok_or_else(|| RegistryError::UnknownNameMapEntry(name.to_owned()))?;
        // Preflight bounded identity allocation so a rejected operation is inert.
        let count = if entry == KnownNameEntry::Absent {
            12
        } else {
            11
        };
        if let KnownNameEntry::Shared(id) = entry {
            if self.shared_groups[&id].entries_in_native_order.len() >= MAX_GROUP_ENTRIES {
                return Err(RegistryError::UnsupportedEffect(
                    "group entries exceed bounded projection",
                ));
            }
            if self.shared_groups[&id].append_order == AppendOrder::Unknown {
                return Err(RegistryError::UnsupportedEffect(
                    "unknown shared-group append traversal",
                ));
            }
        }
        let end = self
            .next_identity
            .checked_add(count)
            .ok_or(RegistryError::IdentityExhausted)?;
        let mut next = self.next_identity;
        let shared_group = match entry {
            KnownNameEntry::Absent => {
                let id = ScriptObjectId(next);
                next += 1;
                self.shared_groups.insert(
                    id,
                    SharedGroup {
                        entries_in_native_order: vec![],
                        append_order: AppendOrder::DenseNumericArray,
                    },
                );
                self.names
                    .insert(name.to_owned(), KnownNameEntry::Shared(id));
                id
            }
            KnownNameEntry::Shared(id) => id,
        };
        let local_group = ScriptObjectId(next);
        let event = ScriptObjectId(next + 1);
        let methods = std::array::from_fn(|i| ScriptObjectId(next + 2 + i as u64));
        let test_all = methods[5];
        self.local_groups.insert(
            local_group,
            LocalGroup {
                objlen: 0,
                numeric_slots: BTreeMap::new(),
            },
        );
        let group = self
            .shared_groups
            .get_mut(&shared_group)
            .expect("validated shared group");
        let position = match group.append_order {
            AppendOrder::DenseNumericArray => group.entries_in_native_order.len(),
            AppendOrder::KnownNextPosition(i) => {
                group.append_order = AppendOrder::Unknown;
                i
            }
            AppendOrder::Unknown => unreachable!("preflight checked append placement"),
        };
        group
            .entries_in_native_order
            .insert(position, GroupEntry::Table(local_group));
        let identity = EventIdentity {
            event,
            shared_group,
            local_group,
            test_all,
            methods,
        };
        self.cache.insert(name.to_owned(), identity);
        self.next_identity = end;
        Ok(identity)
    }

    /// Only existing raw slots can change here. Shape/objlen recomputation and
    /// lua_next structural mutation require HKS table semantics not modeled here.
    pub fn replace_existing_slot(
        &mut self,
        local: ScriptObjectId,
        index: usize,
        value: LuaValue,
    ) -> Result<(), RegistryError> {
        let next_identity = match &value {
            LuaValue::Object(id) if id.0 >= self.next_identity => {
                id.0.checked_add(1)
                    .ok_or(RegistryError::IdentityExhausted)?
            }
            _ => self.next_identity,
        };
        let group = self
            .local_groups
            .get_mut(&local)
            .ok_or(RegistryError::UnknownObject(local))?;
        let slot = group
            .numeric_slots
            .get_mut(&index)
            .ok_or(RegistryError::UnsupportedEffect(
                "array shape/objlen mutation",
            ))?;
        if matches!(slot, LuaValue::Nil) != matches!(value, LuaValue::Nil) {
            return Err(RegistryError::UnsupportedEffect(
                "nil boundary requires native objlen state",
            ));
        }
        *slot = value;
        self.next_identity = next_identity;
        Ok(())
    }

    /// Bounded logical empty-group path. Unsupported callback semantics refuse
    /// during effect-free admission before lookup/cache mutation. This is not
    /// admission of HKS allocator/GC effects or a complete live action.
    pub fn test_all(
        &mut self,
        name: &str,
        arguments: &[i32],
    ) -> Result<Option<bool>, RegistryError> {
        self.test_all_with_vm_diagnostic(name, arguments, &mut UnsupportedCallbackVm)
    }

    /// Diagnostic source-projection orchestration only, never a live action
    /// entrypoint. Provider failure after admitted dispatch retains the genuine
    /// executed prefix; Err must not be treated as action success or native
    /// protected failure. Live action admission is separately denied above.
    pub fn test_all_with_vm_diagnostic<V: CallbackVm>(
        &mut self,
        name: &str,
        arguments: &[i32],
        vm: &mut V,
    ) -> Result<Option<bool>, RegistryError> {
        // Preflight all currently reachable slots before changing the graph.
        // New local groups created by lookup are empty. Existing groups cannot
        // structurally mutate through CallbackEffects; later slot replacements
        // receive a fresh admission check at their actual execution boundary.
        let shared = if let Some(event) = self.cached_event(name) {
            Some(event.shared_group)
        } else {
            match self
                .names
                .get(name)
                .copied()
                .ok_or_else(|| RegistryError::UnknownNameMapEntry(name.to_owned()))?
            {
                KnownNameEntry::Absent => None,
                KnownNameEntry::Shared(id) => Some(id),
            }
        };
        if let Some(shared) = shared {
            for entry in &self.shared_groups[&shared].entries_in_native_order {
                if let GroupEntry::Table(local) = entry {
                    let group = &self.local_groups[local];
                    for index in 1..=group.objlen {
                        vm.admit_callback(
                            group.numeric_slots.get(&index).unwrap_or(&LuaValue::Nil),
                            arguments,
                        )?;
                    }
                }
            }
        }
        let event = self.lookup(name)?;
        let mut any_nonempty = false;
        let mut aggregate = true;
        let mut cursor = 0;
        loop {
            // Read reached group live rather than flattening callback values.
            let entry = self.shared_groups[&event.shared_group]
                .entries_in_native_order
                .get(cursor)
                .cloned();
            cursor += 1;
            let local = match entry {
                None => break,
                Some(GroupEntry::NonTable(_)) => continue,
                Some(GroupEntry::Table(id)) => id,
            };
            let length = self.local_groups[&local].objlen;
            if length != 0 {
                any_nonempty = true;
            }
            for index in 1..=length {
                let callback = self.local_groups[&local]
                    .numeric_slots
                    .get(&index)
                    .cloned()
                    .unwrap_or(LuaValue::Nil);
                vm.admit_callback(&callback, arguments)?;
                match vm.protected_call(
                    &mut CallbackEffects { registry: self },
                    callback,
                    arguments,
                )? {
                    ProtectedCallbackResult::Returned(value) => aggregate &= value.truthy(),
                    ProtectedCallbackResult::Failed => {}
                }
            }
        }
        Ok(any_nonempty.then_some(aggregate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cached_seed() -> (RegistrySeed, BTreeMap<String, EventIdentity>) {
        let mut registry = ScriptEventRegistry::from_seed(seed()).unwrap();
        let event = registry.lookup("CanStartOperation").unwrap();
        (
            RegistrySeed {
                names: registry.names,
                shared_groups: registry.shared_groups,
                local_groups: registry.local_groups,
            },
            [("CanStartOperation".into(), event)].into(),
        )
    }
    fn cached_effects() -> CachedExecutionState {
        CachedExecutionState {
            call_capacity: CachedCallCapacity::NoGrowthForThreeIntegers,
            collector_phase: 0,
            step_countdown: 2,
            allocated_bytes: 100,
            peak_allocated_bytes: 120,
            collection_threshold: 100,
            collection_inhibited: false,
            interned_names: ["GameEvents", "CanStartOperation", "TestAll"]
                .map(str::to_owned)
                .into(),
            protected_closure_obligations: 0,
        }
    }

    #[test]
    fn cached_calls_evolve_collector_accounting_and_refuse_at_next_boundary() {
        let (seed, cache) = cached_seed();
        let registry = ScriptEventRegistry::from_seed_with_cache(seed, cache).unwrap();
        let before = registry.clone();
        let mut effects = cached_effects();
        assert_eq!(
            registry.test_all_cached("CanStartOperation", [0, 7, 36], &mut effects),
            Ok(None)
        );
        assert_eq!(effects.step_countdown, 1);
        assert_eq!(effects.allocated_bytes, 148);
        assert_eq!(effects.peak_allocated_bytes, 148);
        assert_eq!(effects.protected_closure_obligations, 1);
        let stopped = effects.clone();
        assert_eq!(
            registry.test_all_cached("CanStartOperation", [0, 7, 36], &mut effects),
            Err(RegistryError::UnsupportedEffect("collector progression"))
        );
        assert_eq!(effects, stopped);
        assert_eq!(registry, before);
    }

    #[test]
    fn cached_countdown_threshold_and_inhibit_match_preallocation_guards() {
        let (seed, cache) = cached_seed();
        let registry = ScriptEventRegistry::from_seed_with_cache(seed, cache).unwrap();
        let mut effects = cached_effects();
        effects.step_countdown = 1;
        // Equality is checked before the allocation, so this call succeeds.
        assert_eq!(
            registry.test_all_cached("CanStartOperation", [0; 3], &mut effects),
            Ok(None)
        );
        assert_eq!(effects.step_countdown, 0);
        effects.collection_inhibited = true;
        effects.step_countdown = i32::MIN;
        assert_eq!(
            registry.test_all_cached("CanStartOperation", [0; 3], &mut effects),
            Ok(None)
        );
        assert_eq!(effects.step_countdown, i32::MAX);
        assert_eq!(effects.allocated_bytes, 196);
        assert_eq!(effects.protected_closure_obligations, 2);
        // A wrapped positive subtraction result does not satisfy signed JG:
        // overflow participates in the native condition.
        effects.step_countdown = i32::MIN;
        effects.collection_inhibited = false;
        let before = effects.clone();
        assert_eq!(
            registry.test_all_cached("CanStartOperation", [0; 3], &mut effects),
            Err(RegistryError::UnsupportedEffect("collector progression"))
        );
        assert_eq!(effects, before);
        // Positive old countdown bypasses even an exceeded allocation limit.
        effects.step_countdown = 2;
        effects.peak_allocated_bytes = 1_000;
        assert_eq!(
            registry.test_all_cached("CanStartOperation", [0; 3], &mut effects),
            Ok(None)
        );
        assert_eq!(effects.allocated_bytes, 244);
        assert_eq!(effects.peak_allocated_bytes, 1_000);
        assert_eq!(effects.protected_closure_obligations, 3);
    }

    #[test]
    fn cached_preflight_gaps_preserve_both_states() {
        let (seed, cache) = cached_seed();
        let registry = ScriptEventRegistry::from_seed_with_cache(seed.clone(), cache).unwrap();
        let uncached = ScriptEventRegistry::from_seed(seed).unwrap();
        for (target, name, mut effects) in [
            (&uncached, "CanStartOperation", cached_effects()),
            (&registry, "unknown", cached_effects()),
            (&registry, "CanStartOperation", {
                let mut e = cached_effects();
                e.call_capacity = CachedCallCapacity::Unknown;
                e
            }),
            (&registry, "CanStartOperation", {
                let mut e = cached_effects();
                e.collector_phase = 2;
                e
            }),
            (&registry, "CanStartOperation", {
                let mut e = cached_effects();
                e.interned_names.remove("TestAll");
                e
            }),
            (&registry, "CanStartOperation", {
                let mut e = cached_effects();
                e.allocated_bytes = u64::MAX;
                e
            }),
        ] {
            let before = effects.clone();
            let graph = target.clone();
            assert!(target.test_all_cached(name, [0; 3], &mut effects).is_err());
            assert_eq!(effects, before);
            assert_eq!(*target, graph);
        }
    }

    #[test]
    fn cached_nonempty_group_is_refused_before_allocator_effects() {
        let mut registry = callbacks();
        registry.lookup("event").unwrap();
        let before = registry.clone();
        let mut effects = cached_effects();
        effects.interned_names.insert("event".into());
        let initial_effects = effects.clone();
        assert_eq!(
            registry.test_all_cached("event", [0; 3], &mut effects),
            Err(RegistryError::UnsupportedCallbackVm)
        );
        assert_eq!(effects, initial_effects);
        assert_eq!(registry, before);
    }

    #[test]
    fn cached_seed_preserves_builtin_identity_and_reserves_future_ids() {
        let (mut seed, cache) = cached_seed();
        seed.names.insert("later".into(), KnownNameEntry::Absent);
        let event = cache["CanStartOperation"];
        let mut registry = ScriptEventRegistry::from_seed_with_cache(seed, cache).unwrap();
        let before = registry.clone();
        assert_eq!(registry.lookup("CanStartOperation"), Ok(event));
        assert_eq!(registry, before);
        assert_eq!(event.test_all, event.methods[5]);
        let later = registry.lookup("later").unwrap();
        assert!(later.shared_group.0 > event.methods.iter().map(|id| id.0).max().unwrap());
    }

    #[test]
    fn cached_seed_rejects_contradictory_identities() {
        let (seed, cache) = cached_seed();
        let mut invalid = cache.clone();
        invalid.get_mut("CanStartOperation").unwrap().test_all = ScriptObjectId(99);
        assert_eq!(
            ScriptEventRegistry::from_seed_with_cache(seed.clone(), invalid),
            Err(RegistryError::InvalidSeed(
                "cache TestAll identity mismatch"
            ))
        );
        let mut invalid = cache.clone();
        let event = invalid.get_mut("CanStartOperation").unwrap();
        event.methods[0] = event.local_group;
        assert_eq!(
            ScriptEventRegistry::from_seed_with_cache(seed.clone(), invalid),
            Err(RegistryError::InvalidSeed(
                "cache object identities overlap"
            ))
        );
        let mut invalid = cache;
        invalid.get_mut("CanStartOperation").unwrap().local_group = ScriptObjectId(99);
        assert_eq!(
            ScriptEventRegistry::from_seed_with_cache(seed, invalid),
            Err(RegistryError::InvalidSeed(
                "cache local group is not linked"
            ))
        );
    }

    fn seed() -> RegistrySeed {
        RegistrySeed {
            names: [("CanStartOperation".into(), KnownNameEntry::Absent)].into(),
            ..Default::default()
        }
    }
    #[test]
    fn source_generated_lookup_creates_graph_and_reuses_cache() {
        let mut registry = ScriptEventRegistry::from_seed(seed()).unwrap();
        let before = registry.clone();
        let first = registry.lookup("CanStartOperation").unwrap();
        assert_ne!(before, registry);
        let after = registry.clone();
        assert_eq!(registry.lookup("CanStartOperation"), Ok(first));
        assert_eq!(registry, after);
        assert_eq!(
            registry.test_all("CanStartOperation", &[0, 15794187, 36]),
            Ok(None)
        );
        assert_eq!(registry, after);
        assert_eq!(
            registry
                .shared_group(first.shared_group)
                .unwrap()
                .entries_in_native_order,
            [GroupEntry::Table(first.local_group)]
        );
    }
    #[test]
    fn unknown_map_and_bad_seed_are_inert() {
        let mut registry = ScriptEventRegistry::default();
        let before = registry.clone();
        assert_eq!(
            registry.test_all("CanStartOperation", &[]),
            Err(RegistryError::UnknownNameMapEntry(
                "CanStartOperation".into()
            ))
        );
        assert_eq!(registry, before);
        let mut invalid = seed();
        invalid
            .names
            .insert("x".into(), KnownNameEntry::Shared(ScriptObjectId(3)));
        assert_eq!(
            ScriptEventRegistry::from_seed(invalid),
            Err(RegistryError::UnknownObject(ScriptObjectId(3)))
        );
    }
    fn callbacks() -> ScriptEventRegistry {
        let mut s = RegistrySeed::default();
        s.names
            .insert("event".into(), KnownNameEntry::Shared(ScriptObjectId(1)));
        s.shared_groups.insert(
            ScriptObjectId(1),
            SharedGroup {
                entries_in_native_order: vec![
                    GroupEntry::NonTable(LuaValue::Number(0.0)),
                    GroupEntry::Table(ScriptObjectId(2)),
                    GroupEntry::Table(ScriptObjectId(3)),
                ],
                append_order: AppendOrder::KnownNextPosition(3),
            },
        );
        s.local_groups.insert(
            ScriptObjectId(2),
            LocalGroup {
                objlen: 2,
                numeric_slots: [(1, LuaValue::Boolean(false)), (2, LuaValue::Boolean(true))].into(),
            },
        );
        s.local_groups.insert(
            ScriptObjectId(3),
            LocalGroup {
                objlen: 1,
                numeric_slots: BTreeMap::new(),
            },
        );
        ScriptEventRegistry::from_seed(s).unwrap()
    }
    struct Vm {
        calls: Vec<LuaValue>,
        fail_at: Option<usize>,
    }
    impl CallbackVm for Vm {
        fn admit_callback(&self, _: &LuaValue, _: &[i32]) -> Result<(), RegistryError> {
            Ok(())
        }
        fn protected_call(
            &mut self,
            registry: &mut CallbackEffects<'_>,
            callback: LuaValue,
            args: &[i32],
        ) -> Result<ProtectedCallbackResult, RegistryError> {
            assert_eq!(args, [7, -1, 36]);
            self.calls.push(callback.clone());
            if self.calls.len() == 1 {
                registry.replace_existing_slot(ScriptObjectId(2), 2, LuaValue::String(vec![]))?;
            }
            if self.fail_at == Some(self.calls.len()) {
                return Err(RegistryError::UnsupportedCallbackVm);
            }
            if callback == LuaValue::Nil {
                Ok(ProtectedCallbackResult::Failed)
            } else {
                Ok(ProtectedCallbackResult::Returned(callback))
            }
        }
    }
    #[test]
    fn synthetic_dispatch_native_order_live_slots_truthiness_and_no_short_circuit() {
        let mut registry = callbacks();
        let mut vm = Vm {
            calls: vec![],
            fail_at: None,
        };
        assert_eq!(
            registry.test_all_with_vm_diagnostic("event", &[7, -1, 36], &mut vm),
            Ok(Some(false))
        );
        assert_eq!(
            vm.calls,
            [
                LuaValue::Boolean(false),
                LuaValue::String(vec![]),
                LuaValue::Nil
            ]
        );
        assert!(LuaValue::Number(0.0).truthy());
        assert!(LuaValue::String(vec![]).truthy());
        assert!(!LuaValue::Nil.truthy());
    }
    #[test]
    fn unsupported_vm_preserves_lookup_and_prior_callback_effects() {
        let mut registry = callbacks();
        assert_eq!(
            registry.test_all("event", &[]),
            Err(RegistryError::UnsupportedCallbackVm)
        );
        assert!(registry.cached_event("event").is_none());
        let mut vm = Vm {
            calls: vec![],
            fail_at: Some(2),
        };
        assert_eq!(
            registry.test_all_with_vm_diagnostic("event", &[7, -1, 36], &mut vm),
            Err(RegistryError::UnsupportedCallbackVm)
        );
        assert_eq!(
            registry
                .local_group(ScriptObjectId(2))
                .unwrap()
                .numeric_slots[&2],
            LuaValue::String(vec![])
        );
        assert_eq!(vm.calls.len(), 2);
    }

    #[test]
    fn unknown_append_order_refuses_before_mutation() {
        let mut r = callbacks();
        r.shared_groups
            .get_mut(&ScriptObjectId(1))
            .unwrap()
            .append_order = AppendOrder::Unknown;
        let before = r.clone();
        assert_eq!(
            r.lookup("event"),
            Err(RegistryError::UnsupportedEffect(
                "unknown shared-group append traversal"
            ))
        );
        assert_eq!(r, before);
    }
    #[test]
    fn all_protected_errors_still_return_true_for_nonempty_group() {
        struct Errors;
        impl CallbackVm for Errors {
            fn admit_callback(&self, _: &LuaValue, _: &[i32]) -> Result<(), RegistryError> {
                Ok(())
            }
            fn protected_call(
                &mut self,
                _: &mut CallbackEffects<'_>,
                _: LuaValue,
                _: &[i32],
            ) -> Result<ProtectedCallbackResult, RegistryError> {
                Ok(ProtectedCallbackResult::Failed)
            }
        }
        let mut r = callbacks();
        assert_eq!(
            r.test_all_with_vm_diagnostic("event", &[], &mut Errors),
            Ok(Some(true))
        );
    }

    #[test]
    fn nil_boundary_mutation_is_rejected_without_changing_state() {
        let mut r = callbacks();
        let before = r.clone();
        assert!(matches!(
            r.replace_existing_slot(ScriptObjectId(2), 1, LuaValue::Nil),
            Err(RegistryError::UnsupportedEffect(_))
        ));
        assert_eq!(r, before);
        r.local_groups
            .get_mut(&ScriptObjectId(3))
            .unwrap()
            .numeric_slots
            .insert(1, LuaValue::Nil);
        let before = r.clone();
        assert!(matches!(
            r.replace_existing_slot(ScriptObjectId(3), 1, LuaValue::Boolean(true)),
            Err(RegistryError::UnsupportedEffect(_))
        ));
        assert_eq!(r, before);
    }
    #[test]
    fn default_vm_and_live_execution_admission_are_inert() {
        let mut r = callbacks();
        let before = r.clone();
        assert_eq!(
            r.test_all("event", &[]),
            Err(RegistryError::UnsupportedCallbackVm)
        );
        assert_eq!(r, before);
        assert_eq!(
            r.require_live_execution_support(),
            Err(RegistryError::UnsupportedEffect(
                "HKS allocator/GC live execution"
            ))
        );
        assert_eq!(r, before);
    }

    #[test]
    fn contradictory_non_table_identity_is_rejected() {
        let r = callbacks();
        let mut s = RegistrySeed {
            names: r.names,
            shared_groups: r.shared_groups,
            local_groups: r.local_groups,
        };
        s.shared_groups
            .get_mut(&ScriptObjectId(1))
            .unwrap()
            .entries_in_native_order
            .push(GroupEntry::NonTable(LuaValue::Object(ScriptObjectId(2))));
        assert_eq!(
            ScriptEventRegistry::from_seed(s),
            Err(RegistryError::InvalidSeed(
                "non-table value names a known table identity"
            ))
        );
    }

    #[test]
    fn sparse_huge_objlen_seed_is_rejected_before_iteration() {
        let mut s = seed();
        s.local_groups.insert(
            ScriptObjectId(1),
            LocalGroup {
                objlen: usize::MAX,
                numeric_slots: BTreeMap::new(),
            },
        );
        assert_eq!(
            ScriptEventRegistry::from_seed(s),
            Err(RegistryError::InvalidSeed(
                "callback objlen exceeds bounded projection"
            ))
        );
        let r = callbacks();
        let mut s = RegistrySeed {
            names: r.names,
            shared_groups: r.shared_groups,
            local_groups: r.local_groups,
        };
        s.local_groups.get_mut(&ScriptObjectId(2)).unwrap().objlen = MAX_REACHED_CALLBACK_SLOTS;
        assert_eq!(
            ScriptEventRegistry::from_seed(s),
            Err(RegistryError::InvalidSeed(
                "reached callback slots exceed bounded projection"
            ))
        );
    }
    #[test]
    fn object_ids_do_not_alias_seeded_callback_identity() {
        let mut s = seed();
        s.local_groups.insert(
            ScriptObjectId(5),
            LocalGroup {
                objlen: 1,
                numeric_slots: [(1, LuaValue::Object(ScriptObjectId(100)))].into(),
            },
        );
        let mut r = ScriptEventRegistry::from_seed(s).unwrap();
        assert_eq!(
            r.lookup("CanStartOperation").unwrap().shared_group,
            ScriptObjectId(101)
        );
    }
}
