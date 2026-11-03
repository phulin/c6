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
    /// Twenty-eight API cells, three nested C frames, and bounded depth.
    NoGrowthForFactoryThreeIntegers,
}

/// Current semantic facts for the all-intern-hit, no-destination-growth factory
/// frontier. Facts are supplied from actual initial state, not inferred from a
/// successful call. Insertion facts are one-use and must be invalidated by any
/// external VM activity. An absent fact is unsupported, never spare capacity.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FactoryExecutionState {
    pub game_event_spare_names: std::collections::BTreeSet<String>,
    pub name_map_spare_names: std::collections::BTreeSet<String>,
    /// Exact native objlen+1, already allocated and nil, for the next append.
    /// This does not assert any additional spare slot after that append.
    pub shared_spare_slots: BTreeMap<ScriptObjectId, usize>,
    pub weak_shared_groups: std::collections::BTreeSet<ScriptObjectId>,
    /// Retained identities in execution order, including unswept temporaries.
    pub factory_objects: Vec<FactoryObjects>,
}

/// All newly linked swept objects, in native construction order. Backing
/// allocations are retained by their table: event hash16 (640 bytes), event
/// metatable hash4 (160), new shared array1 (16), weak metatable hash1 (40).
/// Headers are64 bytes; methods/factory closure64; protected closure48.
/// The protected and factory closures remain112 bytes of garbage obligations;
/// no admitted call collects them. The factory closure retains shared_group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactoryObjects {
    pub event: EventIdentity,
    pub protected_closure: ScriptObjectId,
    pub factory_closure: ScriptObjectId,
    pub event_metatable: ScriptObjectId,
    pub protected_metatable: ScriptObjectId,
    pub weak_metatable: Option<ScriptObjectId>,
    pub swept_construction_order: Vec<ScriptObjectId>,
}

// Small effect-free scalar plan, not a heap/graph transaction or native replay.
struct AllocationPlan {
    countdown: i32,
    allocated: u64,
    peak: u64,
    threshold: u64,
    inhibited: bool,
}
impl AllocationPlan {
    fn charge(&mut self, units: i32, allocations: &[u64], free: u64) -> Result<(), RegistryError> {
        if self.countdown <= units && self.allocated > self.threshold && !self.inhibited {
            return Err(RegistryError::UnsupportedEffect("collector progression"));
        }
        self.countdown = self.countdown.wrapping_sub(units);
        for bytes in allocations {
            self.allocated =
                self.allocated
                    .checked_add(*bytes)
                    .ok_or(RegistryError::UnsupportedEffect(
                        "factory allocation accounting overflow",
                    ))?;
            self.peak = self.peak.max(self.allocated);
        }
        self.allocated =
            self.allocated
                .checked_sub(free)
                .ok_or(RegistryError::UnsupportedEffect(
                    "factory allocation accounting underflow",
                ))?;
        Ok(())
    }
}

impl CachedExecutionState {
    fn admit(&self, name: &str) -> Result<(), RegistryError> {
        if !matches!(
            self.call_capacity,
            CachedCallCapacity::NoGrowthForThreeIntegers
                | CachedCallCapacity::NoGrowthForFactoryThreeIntegers
        ) {
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
    /// An unknown indexer name-map entry stays unknown: the cached path does
    /// not reach it. Explicitly supplied contradictory entries are rejected.
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
            if registry
                .names
                .get(name)
                .is_some_and(|entry| *entry != KnownNameEntry::Shared(event.shared_group))
            {
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

    /// Execute cached TestAll or the bounded absent-event factory from current
    /// typed state. All source-used strings must already be interned. Unknown
    /// capacity, map state, intern misses, collection, or callbacks refuse before
    /// any mutation. This source projection does not establish native parity or
    /// implement complete actions, turns, arbitrary HKS execution, or collection.
    pub fn test_all_bounded(
        &mut self,
        name: &str,
        arguments: [i32; 3],
        effects: &mut CachedExecutionState,
        factory: &mut FactoryExecutionState,
    ) -> Result<Option<bool>, RegistryError> {
        if self.cached_event(name).is_some() {
            return self.test_all_cached(name, arguments, effects);
        }
        if name.as_bytes().contains(&0) {
            return Err(RegistryError::UnsupportedEffect(
                "embedded NUL in native field name",
            ));
        }
        let entry = self
            .names
            .get(name)
            .copied()
            .ok_or_else(|| RegistryError::UnknownNameMapEntry(name.to_owned()))?;
        if effects.call_capacity != CachedCallCapacity::NoGrowthForFactoryThreeIntegers {
            return Err(RegistryError::UnsupportedEffect(
                "unknown factory call growth effects",
            ));
        }
        if effects.collector_phase != 0 {
            return Err(RegistryError::UnsupportedEffect(
                "collector phase outside factory frontier",
            ));
        }
        for key in [
            "GameEvents",
            name,
            "TestAll",
            "GameEvent::lCreate",
            "func",
            "Add",
            "Count",
            "Remove",
            "RemoveAll",
            "Call",
            "Accumulate",
            "AccumulateINT",
            "TestAny",
            "__newindex",
            "__metatable",
        ] {
            if !effects.interned_names.contains(key) {
                return Err(RegistryError::UnsupportedEffect("string intern miss"));
            }
        }
        if !factory.game_event_spare_names.contains(name) {
            return Err(RegistryError::UnsupportedEffect(
                "unknown GameEvents insertion growth",
            ));
        }
        let append_position = match entry {
            KnownNameEntry::Absent => {
                if !factory.name_map_spare_names.contains(name) {
                    return Err(RegistryError::UnsupportedEffect(
                        "unknown name-map insertion growth",
                    ));
                }
                if !effects.interned_names.contains("__mode")
                    || !effects.interned_names.contains("v")
                {
                    return Err(RegistryError::UnsupportedEffect("string intern miss"));
                }
                0
            }
            KnownNameEntry::Shared(id) => {
                let group = &self.shared_groups[&id];
                if !factory.weak_shared_groups.contains(&id) {
                    return Err(RegistryError::UnsupportedEffect(
                        "unknown shared-group weak semantics",
                    ));
                }
                if !factory
                    .shared_spare_slots
                    .get(&id)
                    .is_some_and(|slot| *slot > 0 && *slot <= i32::MAX as usize)
                {
                    return Err(RegistryError::UnsupportedEffect(
                        "unknown shared-group spare array slot",
                    ));
                }
                if group.entries_in_native_order.len() >= MAX_GROUP_ENTRIES {
                    return Err(RegistryError::UnsupportedEffect(
                        "group entries exceed bounded projection",
                    ));
                }
                for existing in &group.entries_in_native_order {
                    if let GroupEntry::Table(local) = existing {
                        if self.local_groups[local].objlen != 0 {
                            return Err(RegistryError::UnsupportedCallbackVm);
                        }
                    }
                }
                match group.append_order {
                    AppendOrder::DenseNumericArray => {
                        if factory.shared_spare_slots[&id]
                            != group.entries_in_native_order.len() + 1
                        {
                            return Err(RegistryError::UnsupportedEffect(
                                "inconsistent shared-group spare array slot",
                            ));
                        }
                        group.entries_in_native_order.len()
                    }
                    AppendOrder::KnownNextPosition(position) => position,
                    AppendOrder::Unknown => {
                        return Err(RegistryError::UnsupportedEffect(
                            "unknown shared-group append traversal",
                        ))
                    }
                }
            }
        };
        let protected_obligations = effects.protected_closure_obligations.checked_add(1).ok_or(
            RegistryError::UnsupportedEffect("cached allocation accounting overflow"),
        )?;
        let mut allocation = AllocationPlan {
            countdown: effects.step_countdown,
            allocated: effects.allocated_bytes,
            peak: effects.peak_allocated_bytes,
            threshold: effects.collection_threshold,
            inhibited: effects.collection_inhibited,
        };
        // Ordered source ledger: each check sees OLD signed countdown and OLD
        // current bytes. Resize allocates before freeing old backing. Intern
        // hits at phase zero have no charge; missing membership was rejected.
        allocation.charge(1, &[48], 0)?;
        if entry == KnownNameEntry::Absent {
            allocation.charge(3, &[64], 0)?; // shared table
            allocation.charge(3, &[64, 16], 0)?; // weak metatable
            allocation.charge(1, &[40], 16)?; // __mode string insertion
        }
        allocation.charge(1, &[64], 0)?; // temporary factory closure
        allocation.charge(3, &[64], 0)?; // event
        allocation.charge(3, &[64], 0)?; // local array
        if entry == KnownNameEntry::Absent {
            allocation.charge(1, &[16], 0)?; // first shared numeric slot
        }
        // Native registration order differs from REGISTERED_METHODS indexing.
        const METHOD_ORDER: [usize; 9] = [0, 3, 1, 2, 4, 7, 8, 6, 5];
        let mut old_hash_bytes = 0;
        for insertion in 1u64..=9 {
            allocation.charge(1, &[64], 0)?;
            if matches!(insertion, 1 | 2 | 3 | 5 | 9) {
                let hash_bytes = 40 * insertion.next_power_of_two();
                allocation.charge(1, &[hash_bytes], old_hash_bytes)?;
                old_hash_bytes = hash_bytes;
            }
        }
        allocation.charge(3, &[64, 160], 0)?; // event metatable
        allocation.charge(3, &[64], 0)?; // protected empty metatable

        // Reserve every new swept identity before mutation, including garbage
        // obligations and metatables. Assign in actual source construction order.
        let count = if entry == KnownNameEntry::Absent {
            17
        } else {
            15
        };
        let end = self
            .next_identity
            .checked_add(count)
            .ok_or(RegistryError::IdentityExhausted)?;
        let mut next = self.next_identity;
        let mut order = Vec::new();
        let mut fresh = || {
            let id = ScriptObjectId(next);
            next += 1;
            order.push(id);
            id
        };
        let protected_closure = fresh();
        let (shared_group, weak_metatable) = match entry {
            KnownNameEntry::Absent => (fresh(), Some(fresh())),
            KnownNameEntry::Shared(id) => (id, None),
        };
        let factory_closure = fresh();
        let event = fresh();
        let local_group = fresh();
        let mut methods = [ScriptObjectId(0); 9];
        for method in METHOD_ORDER {
            methods[method] = fresh();
        }
        let event_metatable = fresh();
        let protected_metatable = fresh();
        let identity = EventIdentity {
            event,
            shared_group,
            local_group,
            test_all: methods[5],
            methods,
        };
        debug_assert_eq!(next, end);

        // All fallible admission is complete. Commit just the reached objects,
        // consumed facts, and scalar allocator projection.
        if entry == KnownNameEntry::Absent {
            self.shared_groups.insert(
                shared_group,
                SharedGroup {
                    entries_in_native_order: vec![],
                    append_order: AppendOrder::DenseNumericArray,
                },
            );
            self.names
                .insert(name.to_owned(), KnownNameEntry::Shared(shared_group));
            factory.weak_shared_groups.insert(shared_group);
            factory.name_map_spare_names.clear();
        }
        self.local_groups.insert(
            local_group,
            LocalGroup {
                objlen: 0,
                numeric_slots: BTreeMap::new(),
            },
        );
        let shared = self
            .shared_groups
            .get_mut(&shared_group)
            .expect("admitted group");
        shared
            .entries_in_native_order
            .insert(append_position, GroupEntry::Table(local_group));
        if matches!(shared.append_order, AppendOrder::KnownNextPosition(_)) {
            shared.append_order = AppendOrder::Unknown;
        }
        self.cache.insert(name.to_owned(), identity);
        self.next_identity = end;
        factory.game_event_spare_names.clear();
        factory.shared_spare_slots.remove(&shared_group);
        factory.factory_objects.push(FactoryObjects {
            event: identity,
            protected_closure,
            factory_closure,
            event_metatable,
            protected_metatable,
            weak_metatable,
            swept_construction_order: order,
        });
        effects.step_countdown = allocation.countdown;
        effects.allocated_bytes = allocation.allocated;
        effects.peak_allocated_bytes = allocation.peak;
        effects.protected_closure_obligations = protected_obligations;
        // Compute builtin TestAll from the actual committed group, rather than
        // returning a factory-specific constant or replayed native answer.
        let any_nonempty = self.shared_groups[&shared_group].entries_in_native_order.iter().any(|entry| {
            matches!(entry, GroupEntry::Table(local) if self.local_groups[local].objlen != 0)
        });
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
    fn factory_context(
        reused: bool,
    ) -> (
        ScriptEventRegistry,
        CachedExecutionState,
        FactoryExecutionState,
    ) {
        let mut seed = seed();
        let mut factory = FactoryExecutionState {
            game_event_spare_names: ["CanStartOperation".into(), "later".into()].into(),
            name_map_spare_names: ["CanStartOperation".into(), "later".into()].into(),
            ..Default::default()
        };
        seed.names.insert("later".into(), KnownNameEntry::Absent);
        if reused {
            let shared = ScriptObjectId(10);
            seed.names
                .insert("CanStartOperation".into(), KnownNameEntry::Shared(shared));
            seed.shared_groups.insert(
                shared,
                SharedGroup {
                    entries_in_native_order: vec![],
                    append_order: AppendOrder::DenseNumericArray,
                },
            );
            factory.shared_spare_slots.insert(shared, 1);
            factory.weak_shared_groups.insert(shared);
        }
        let mut effects = cached_effects();
        effects.call_capacity = CachedCallCapacity::NoGrowthForFactoryThreeIntegers;
        effects.step_countdown = 1000;
        effects.collection_threshold = u64::MAX;
        effects.interned_names.extend(
            [
                "GameEvent::lCreate",
                "func",
                "Add",
                "Count",
                "Remove",
                "RemoveAll",
                "Call",
                "Accumulate",
                "AccumulateINT",
                "TestAny",
                "__newindex",
                "__metatable",
                "__mode",
                "v",
                "later",
            ]
            .map(str::to_owned),
        );
        (
            ScriptEventRegistry::from_seed(seed).unwrap(),
            effects,
            factory,
        )
    }

    #[test]
    fn factory_new_and_reused_accounting_identity_and_cached_continuation() {
        for reused in [false, true] {
            let (mut registry, mut effects, mut factory) = factory_context(reused);
            let original = effects.clone();
            assert_eq!(
                registry.test_all_bounded(
                    "CanStartOperation",
                    [0, 7, 36],
                    &mut effects,
                    &mut factory
                ),
                Ok(None)
            );
            let charge = if reused { 28 } else { 36 };
            let net = if reused { 1744 } else { 1928 };
            let peak = if reused { 1776 } else { 1960 };
            assert_eq!(effects.step_countdown, original.step_countdown - charge);
            assert_eq!(effects.allocated_bytes, original.allocated_bytes + net);
            assert_eq!(
                effects.peak_allocated_bytes,
                original.allocated_bytes + peak
            );
            assert_eq!(effects.protected_closure_obligations, 1);
            assert_eq!(effects.interned_names, original.interned_names);
            let objects = &factory.factory_objects[0];
            let event = objects.event;
            assert_eq!(registry.cached_event("CanStartOperation"), Some(event));
            assert_eq!(event.test_all, event.methods[5]);
            assert!(event.event.0 < event.local_group.0);
            // Actual source order, preserving the public method-index contract.
            let registration = [0, 3, 1, 2, 4, 7, 8, 6, 5].map(|i| event.methods[i].0);
            assert!(registration.windows(2).all(|ids| ids[0] + 1 == ids[1]));
            assert_eq!(
                objects.swept_construction_order.len(),
                if reused { 15 } else { 17 }
            );
            assert!(objects
                .swept_construction_order
                .windows(2)
                .all(|ids| ids[0].0 + 1 == ids[1].0));
            assert_eq!(objects.weak_metatable.is_some(), !reused);
            assert!(factory.weak_shared_groups.contains(&event.shared_group));
            assert_eq!(
                registry.shared_groups[&event.shared_group].entries_in_native_order,
                [GroupEntry::Table(event.local_group)]
            );
            assert_eq!(registry.local_groups[&event.local_group].objlen, 0);
            assert!(factory.game_event_spare_names.is_empty());
            assert_eq!(factory.name_map_spare_names.is_empty(), !reused);
            assert!(!factory.shared_spare_slots.contains_key(&event.shared_group));
            let graph = registry.clone();
            let obligations = factory.clone();
            assert_eq!(
                registry.test_all_bounded(
                    "CanStartOperation",
                    [i32::MIN, -1, i32::MAX],
                    &mut effects,
                    &mut factory
                ),
                Ok(None)
            );
            assert_eq!(registry, graph);
            assert_eq!(factory, obligations);
            assert_eq!(effects.allocated_bytes, original.allocated_bytes + net + 48);
            assert_eq!(effects.step_countdown, original.step_countdown - charge - 1);
            // A second uncached name cannot reuse an invalidated destination fact.
            let before = (registry.clone(), effects.clone(), factory.clone());
            assert!(registry
                .test_all_bounded("later", [0; 3], &mut effects, &mut factory)
                .is_err());
            assert_eq!((registry.clone(), effects.clone(), factory.clone()), before);
            let later = registry.lookup("later").unwrap();
            assert!(later.shared_group.0 > objects_last_id(&obligations));
        }
    }
    fn objects_last_id(factory: &FactoryExecutionState) -> u64 {
        factory
            .factory_objects
            .last()
            .unwrap()
            .swept_construction_order
            .last()
            .unwrap()
            .0
    }

    #[test]
    fn factory_source_guard_order_uses_current_after_resize_free_and_not_peak() {
        for reused in [false, true] {
            let (mut registry, mut effects, mut factory) = factory_context(reused);
            let charge = if reused { 28 } else { 36 };
            let before_final = if reused { 1680 } else { 1864 };
            effects.step_countdown = charge;
            effects.collection_threshold = 100 + before_final;
            // Final check is old_countdown==3 and current==threshold, although
            // the earlier allocate-before-free resize peak exceeds threshold.
            assert_eq!(
                registry.test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory),
                Ok(None)
            );
            assert_eq!(effects.step_countdown, 0);
            assert!(effects.peak_allocated_bytes > effects.collection_threshold);
            let (mut registry, mut effects, mut factory) = factory_context(reused);
            effects.step_countdown = charge;
            effects.collection_threshold = 100 + before_final - 1;
            let before = (registry.clone(), effects.clone(), factory.clone());
            assert_eq!(
                registry.test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory),
                Err(RegistryError::UnsupportedEffect("collector progression"))
            );
            assert_eq!((registry, effects, factory), before);
        }
    }

    #[test]
    fn factory_signed_countdown_wrap_and_old_guard() {
        let (mut registry, mut effects, mut factory) = factory_context(true);
        effects.step_countdown = i32::MIN;
        effects.collection_inhibited = true;
        effects.peak_allocated_bytes = 9999;
        assert_eq!(
            registry.test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory),
            Ok(None)
        );
        assert_eq!(effects.step_countdown, i32::MIN.wrapping_sub(28));
        assert_eq!(effects.peak_allocated_bytes, 9999);
        let (mut registry, mut effects, mut factory) = factory_context(true);
        effects.step_countdown = i32::MIN;
        effects.collection_threshold = 99;
        let before = (registry.clone(), effects.clone(), factory.clone());
        assert_eq!(
            registry.test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory),
            Err(RegistryError::UnsupportedEffect("collector progression"))
        );
        assert_eq!((registry, effects, factory), before);
    }

    #[test]
    fn factory_all_admission_failures_are_inert() {
        for case in 0..12 {
            let (mut registry, mut effects, mut factory) = factory_context(true);
            match case {
                0 => effects.call_capacity = CachedCallCapacity::NoGrowthForThreeIntegers,
                1 => effects.collector_phase = 1,
                2 => {
                    effects.interned_names.remove("Count");
                }
                3 => {
                    factory.game_event_spare_names.clear();
                }
                4 => {
                    factory.shared_spare_slots.clear();
                }
                5 => {
                    factory.weak_shared_groups.clear();
                }
                6 => {
                    registry.names.remove("CanStartOperation");
                }
                7 => registry.next_identity = u64::MAX,
                8 => effects.allocated_bytes = u64::MAX - 500,
                9 => effects.protected_closure_obligations = u64::MAX,
                10 => {
                    factory.shared_spare_slots.insert(ScriptObjectId(10), 2);
                }
                11 => {
                    registry
                        .shared_groups
                        .get_mut(&ScriptObjectId(10))
                        .unwrap()
                        .append_order = AppendOrder::Unknown
                }
                _ => unreachable!(),
            }
            let before = (registry.clone(), effects.clone(), factory.clone());
            assert!(
                registry
                    .test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory)
                    .is_err(),
                "case {case}"
            );
            assert_eq!((registry, effects, factory), before, "case {case}");
        }
        for key in ["__mode", "v"] {
            let (mut registry, mut effects, mut factory) = factory_context(false);
            effects.interned_names.remove(key);
            let before = (registry.clone(), effects.clone(), factory.clone());
            assert!(registry
                .test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory)
                .is_err());
            assert_eq!((registry, effects, factory), before);
        }
        let (mut registry, mut effects, mut factory) = factory_context(false);
        factory.name_map_spare_names.clear();
        let before = (registry.clone(), effects.clone(), factory.clone());
        assert!(registry
            .test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory)
            .is_err());
        assert_eq!((registry, effects, factory), before);
    }

    #[test]
    fn factory_reused_group_preserves_native_order_and_refuses_live_callbacks() {
        let (mut registry, mut effects, mut factory) = factory_context(true);
        for (id, position) in [(20, 0), (21, 1)] {
            registry.local_groups.insert(
                ScriptObjectId(id),
                LocalGroup {
                    objlen: 0,
                    numeric_slots: BTreeMap::new(),
                },
            );
            registry
                .shared_groups
                .get_mut(&ScriptObjectId(10))
                .unwrap()
                .entries_in_native_order
                .insert(position, GroupEntry::Table(ScriptObjectId(id)));
        }
        registry.next_identity = 22;
        registry
            .shared_groups
            .get_mut(&ScriptObjectId(10))
            .unwrap()
            .append_order = AppendOrder::KnownNextPosition(1);
        factory.shared_spare_slots.insert(ScriptObjectId(10), 8); // known sparse objlen7
        let mut rejected = registry.clone();
        rejected
            .local_groups
            .get_mut(&ScriptObjectId(21))
            .unwrap()
            .objlen = 1;
        let before = (rejected.clone(), effects.clone(), factory.clone());
        assert_eq!(
            rejected.test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory),
            Err(RegistryError::UnsupportedCallbackVm)
        );
        assert_eq!((rejected, effects.clone(), factory.clone()), before);
        assert_eq!(
            registry.test_all_bounded("CanStartOperation", [0; 3], &mut effects, &mut factory),
            Ok(None)
        );
        let event = registry.cached_event("CanStartOperation").unwrap();
        assert_eq!(
            registry.shared_groups[&ScriptObjectId(10)].entries_in_native_order,
            [
                GroupEntry::Table(ScriptObjectId(20)),
                GroupEntry::Table(event.local_group),
                GroupEntry::Table(ScriptObjectId(21))
            ]
        );
        assert_eq!(
            registry.shared_groups[&ScriptObjectId(10)].append_order,
            AppendOrder::Unknown
        );
    }

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
    fn cached_seed_does_not_require_an_unreached_name_map_entry() {
        let (mut seed, cache) = cached_seed();
        seed.names.clear();
        let event = cache["CanStartOperation"];
        let mut registry = ScriptEventRegistry::from_seed_with_cache(seed, cache).unwrap();
        let mut effects = cached_effects();
        assert_eq!(registry.lookup("CanStartOperation"), Ok(event));
        assert_eq!(
            registry.test_all_cached("CanStartOperation", [0; 3], &mut effects),
            Ok(None)
        );
        assert!(registry.names.is_empty());
        assert_eq!(
            registry.lookup("unreached"),
            Err(RegistryError::UnknownNameMapEntry("unreached".into()))
        );
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
