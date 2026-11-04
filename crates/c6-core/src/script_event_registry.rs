//! Bounded logical GameEvents graph for the pinned Linux host.
//!
//! This models source-backed factory/cache mutations and builtin TestAll/Call
//! control flow. The three-integer TestAll and six-integer Call no-listener
//! frontiers additionally evolve exact or explicitly alternative allocator accounting. This is not a complete HKS heap,
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
/// an unrelated capture. Factory intern misses additionally require explicit
/// `InternExecutionState`. This model cannot advance a collector or discharge
/// temporary swept allocations.
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
    /// Source-qualified six integer Call including factory: 150 cells, nine frames.
    NoGrowthForFactorySixIntegers,
}

/// Current semantic facts for the no-destination-growth factory frontier. Facts are supplied from actual initial state, not inferred from a
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

/// Current ordinary intern-table facts. Missing names are unknown unless in
/// `known_absent`; external string/collector activity must invalidate these facts.
/// Growth requires scan index zero, so no old-string marking prefix must be
/// preserved. Hit-only calls do not read count/capacity/scan index; misses without
/// growth preserve any scan index. Collection and nonzero-prefix marking are unsupported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternExecutionState {
    /// Exact native count from a valid, probe-consistent current intern table.
    pub count: u32,
    /// Logical mask+1; this bounded profile admits powers of two through 2^31.
    pub capacity: u32,
    /// Unknown is retained through hit-only and non-growing insertion paths.
    pub scan_index: Option<u32>,
    pub known_absent: std::collections::BTreeSet<String>,
}

struct InternPlan<'a> {
    initial: &'a InternExecutionState,
    count: u32,
    capacity: u32,
    added: std::collections::BTreeSet<String>,
}
impl<'a> InternPlan<'a> {
    fn new(initial: &'a InternExecutionState) -> Self {
        Self {
            initial,
            count: initial.count,
            capacity: initial.capacity,
            added: Default::default(),
        }
    }
}

fn plan_intern(
    name: &str,
    effects: &CachedExecutionState,
    intern: &mut Option<InternPlan<'_>>,
    allocation: &mut AllocationPlan,
) -> Result<(), RegistryError> {
    if effects.interned_names.contains(name) {
        if intern
            .as_ref()
            .is_some_and(|p| p.initial.known_absent.contains(name))
        {
            return Err(RegistryError::UnsupportedEffect(
                "contradictory intern membership",
            ));
        }
        return Ok(());
    }
    let plan = intern
        .as_mut()
        .ok_or(RegistryError::UnsupportedEffect("string intern miss"))?;
    if plan.added.contains(name) {
        return Ok(());
    }
    if !plan.initial.known_absent.contains(name) {
        return Err(RegistryError::UnsupportedEffect(
            "unknown string intern membership",
        ));
    }
    if !plan.capacity.is_power_of_two()
        || plan.count >= plan.capacity
        || effects.interned_names.len() as u64 > u64::from(plan.initial.count)
    {
        return Err(RegistryError::UnsupportedEffect(
            "unknown intern table count or capacity",
        ));
    }
    let next = plan
        .count
        .checked_add(1)
        .ok_or(RegistryError::UnsupportedEffect("intern count overflow"))?;
    if next > plan.capacity / 2 {
        if plan.initial.scan_index != Some(0) {
            return Err(RegistryError::UnsupportedEffect(
                "unknown intern growth mark prefix",
            ));
        }
        let capacity = plan
            .capacity
            .checked_mul(2)
            .ok_or(RegistryError::UnsupportedEffect("intern capacity overflow"))?;
        // DIRECT ordinary allocation: no checkStep; allocate before old backing free.
        allocation.allocate(&[8 * u64::from(capacity)], 8 * u64::from(plan.capacity))?;
        plan.capacity = capacity;
    }
    let bytes = u64::try_from(name.len())
        .ok()
        .and_then(|n| n.checked_add(27))
        .ok_or(RegistryError::UnsupportedEffect(
            "intern string size overflow",
        ))?;
    allocation.charge(1, &[bytes], 0)?;
    plan.count = next;
    plan.added.insert(name.to_owned());
    Ok(())
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
#[derive(Debug, PartialEq)]
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
        self.allocate(allocations, free)
    }

    fn allocate(&mut self, allocations: &[u64], free: u64) -> Result<(), RegistryError> {
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum RegistryArguments {
    TestAll([i32; 3]),
    Call([i32; 6]),
}
impl RegistryArguments {
    fn method(self) -> &'static str {
        match self {
            Self::TestAll(_) => "TestAll",
            Self::Call(_) => "Call",
        }
    }
}

/// Deferred effects for one bounded three-integer TestAll call. This owns only
/// newly created objects, intern additions, and allocator scalars. Preparation
/// rechecks the source computation against current inputs before any write.
#[derive(Debug, PartialEq)]
pub struct RegistryExecutionPlan {
    name: String,
    arguments: RegistryArguments,
    namespace_growth: bool,
    with_intern: bool,
    allocation: AllocationPlan,
    protected_obligations: u64,
    result: Option<bool>,
    factory: Option<FactoryPlan>,
}

#[derive(Debug, PartialEq)]
struct FactoryPlan {
    entry: KnownNameEntry,
    append_position: usize,
    end: u64,
    objects: FactoryObjects,
    intern_delta: Option<(u32, u32, std::collections::BTreeSet<String>)>,
}

impl RegistryExecutionPlan {
    /// Every admitted reached local group is empty. The builtin aggregate is
    /// therefore nil; callbacks with nonempty groups remain unsupported.
    pub fn result(&self) -> Option<bool> {
        self.result
    }

    fn commit_effects(&self, effects: &mut CachedExecutionState) {
        effects.step_countdown = self.allocation.countdown;
        effects.allocated_bytes = self.allocation.allocated;
        effects.peak_allocated_bytes = self.allocation.peak;
        effects.protected_closure_obligations = self.protected_obligations;
    }
}

/// Exclusive borrows retain validated inputs until the infallible commit.
/// Dropping this guard without committing leaves all inputs unchanged.
pub struct PreparedRegistryCommit<'a> {
    registry: &'a mut ScriptEventRegistry,
    effects: &'a mut CachedExecutionState,
    factory: &'a mut FactoryExecutionState,
    intern: Option<&'a mut InternExecutionState>,
    plan: RegistryExecutionPlan,
}

impl PreparedRegistryCommit<'_> {
    pub fn commit(self) -> Option<bool> {
        self.registry
            .commit_plan(self.effects, self.factory, self.intern, self.plan)
    }
}

impl CachedExecutionState {
    fn admit(&self, name: &str, arguments: RegistryArguments) -> Result<(), RegistryError> {
        if !matches!(
            self.call_capacity,
            CachedCallCapacity::NoGrowthForThreeIntegers
                | CachedCallCapacity::NoGrowthForFactoryThreeIntegers
                | CachedCallCapacity::NoGrowthForFactorySixIntegers
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
        for key in ["GameEvents", name, arguments.method()] {
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
}

/// One possible physical accounting state; all alternatives share the registry's
/// logical objects. No member is privileged as the actual native history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionAlternative {
    pub execution: CachedExecutionState,
    pub intern: InternExecutionState,
}

/// Explicit bounded knowledge. Unknown intern memberships are enumerated rather
/// than silently interpreted as hits or misses. This is not a general heap VM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedExecutionState {
    alternatives: Vec<ExecutionAlternative>,
    namespace: Option<(u32, u32)>,
}
impl BoundedExecutionState {
    pub fn alternatives(&self) -> &[ExecutionAlternative] {
        &self.alternatives
    }

    /// At most four strictly unknown memberships, ordinary successful allocator,
    /// current intern count/capacity and optional scan knowledge are input facts.
    /// A namespace (capacity, occupied) fact is consumed by a fresh insertion.
    pub fn from_unknown_memberships(
        execution: CachedExecutionState,
        intern: InternExecutionState,
        unknown: &[String],
        namespace: Option<(u32, u32)>,
    ) -> Result<Self, RegistryError> {
        let keys: std::collections::BTreeSet<_> = unknown.iter().collect();
        if unknown.len() > 4
            || keys.len() != unknown.len()
            || unknown
                .iter()
                .any(|s| execution.interned_names.contains(s) || intern.known_absent.contains(s))
        {
            return Err(RegistryError::InvalidSeed(
                "invalid bounded intern memberships",
            ));
        }
        let alternatives = (0..(1usize << unknown.len()))
            .map(|mask| {
                let mut execution = execution.clone();
                let mut intern = intern.clone();
                for (bit, name) in unknown.iter().enumerate() {
                    if mask & (1 << bit) == 0 {
                        execution.interned_names.insert(name.clone());
                    } else {
                        intern.known_absent.insert(name.clone());
                    }
                }
                ExecutionAlternative { execution, intern }
            })
            .collect();
        Ok(Self {
            alternatives,
            namespace,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct BoundedRegistryExecutionPlan {
    name: String,
    arguments: RegistryArguments,
    plans: Vec<RegistryExecutionPlan>,
}

impl BoundedRegistryExecutionPlan {
    pub fn result(&self) -> Option<bool> {
        self.plans[0].result()
    }
}

pub struct PreparedBoundedRegistryCommit<'a> {
    registry: &'a mut ScriptEventRegistry,
    knowledge: &'a mut BoundedExecutionState,
    factory: &'a mut FactoryExecutionState,
    plan: BoundedRegistryExecutionPlan,
}
impl PreparedBoundedRegistryCommit<'_> {
    pub fn commit(self) -> Option<bool> {
        let result = self.plan.plans[0].result();
        let mut plans = self.plan.plans.into_iter();
        let first = plans.next().expect("nonempty bounded alternatives");
        // Only the first delta constructs logical objects; all alternatives have
        // the same delta and differ only in physical intern/accounting state.
        for (alternative, plan) in self.knowledge.alternatives.iter_mut().skip(1).zip(plans) {
            plan.commit_effects(&mut alternative.execution);
            if let Some((count, capacity, added)) = plan.factory.and_then(|f| f.intern_delta) {
                alternative.intern.count = count;
                alternative.intern.capacity = capacity;
                for key in added {
                    alternative.intern.known_absent.remove(&key);
                    alternative.execution.interned_names.insert(key);
                }
            }
        }
        let alternative = &mut self.knowledge.alternatives[0];
        let created = first.factory.is_some();
        self.registry.commit_plan(
            &mut alternative.execution,
            self.factory,
            Some(&mut alternative.intern),
            first,
        );
        if created {
            self.knowledge.namespace = None;
        }
        result
    }
}

impl ScriptEventRegistry {
    /// Source-compute builtin Call with six integer arguments and empty reached
    /// groups. Every alternative must remain inside the supported frontier.
    pub fn plan_call_six(
        &self,
        name: &str,
        arguments: [i32; 6],
        knowledge: &BoundedExecutionState,
        factory: &FactoryExecutionState,
    ) -> Result<BoundedRegistryExecutionPlan, RegistryError> {
        self.plan_bounded(name, RegistryArguments::Call(arguments), knowledge, factory)
    }

    pub fn plan_test_all_alternatives(
        &self,
        name: &str,
        arguments: [i32; 3],
        knowledge: &BoundedExecutionState,
        factory: &FactoryExecutionState,
    ) -> Result<BoundedRegistryExecutionPlan, RegistryError> {
        self.plan_bounded(
            name,
            RegistryArguments::TestAll(arguments),
            knowledge,
            factory,
        )
    }

    fn plan_bounded(
        &self,
        name: &str,
        arguments: RegistryArguments,
        knowledge: &BoundedExecutionState,
        factory: &FactoryExecutionState,
    ) -> Result<BoundedRegistryExecutionPlan, RegistryError> {
        let fresh = self.cached_event(name).is_none();
        let growth = fresh && knowledge.namespace == Some((1, 1));
        if fresh && knowledge.namespace.is_some() && !growth {
            return Err(RegistryError::UnsupportedEffect(
                "unsupported namespace capacity",
            ));
        }
        let plans = knowledge
            .alternatives
            .iter()
            .map(|a| {
                self.plan_invocation(
                    name,
                    arguments,
                    &a.execution,
                    factory,
                    Some(&a.intern),
                    growth,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Every source branch must create identical logical identities/order.
        if plans
            .windows(2)
            .any(|p| match (&p[0].factory, &p[1].factory) {
                (Some(a), Some(b)) => {
                    a.entry != b.entry
                        || a.append_position != b.append_position
                        || a.end != b.end
                        || a.objects != b.objects
                }
                (None, None) => false,
                _ => true,
            })
        {
            return Err(RegistryError::UnsupportedEffect(
                "divergent logical Call alternatives",
            ));
        }
        Ok(BoundedRegistryExecutionPlan {
            name: name.into(),
            arguments,
            plans,
        })
    }

    pub fn prepare_call_six<'a>(
        &'a mut self,
        knowledge: &'a mut BoundedExecutionState,
        factory: &'a mut FactoryExecutionState,
        plan: BoundedRegistryExecutionPlan,
    ) -> Result<PreparedBoundedRegistryCommit<'a>, RegistryError> {
        let current = self.plan_bounded(&plan.name, plan.arguments, knowledge, factory)?;
        if current != plan {
            return Err(RegistryError::UnsupportedEffect("stale bounded Call plan"));
        }
        Ok(PreparedBoundedRegistryCommit {
            registry: self,
            knowledge,
            factory,
            plan,
        })
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
        let plan = self.plan_cached(name, RegistryArguments::TestAll(_arguments), effects)?;
        plan.commit_effects(effects);
        Ok(plan.result())
    }

    fn plan_cached(
        &self,
        name: &str,
        arguments: RegistryArguments,
        effects: &CachedExecutionState,
    ) -> Result<RegistryExecutionPlan, RegistryError> {
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
        if matches!(arguments, RegistryArguments::Call(_))
            && effects.call_capacity != CachedCallCapacity::NoGrowthForFactorySixIntegers
        {
            return Err(RegistryError::UnsupportedEffect(
                "unknown Call6 growth effects",
            ));
        }
        effects.admit(name, arguments)?;
        // Source order: protected cclosure allocation precedes the three
        // intern-hit field lookups and empty TestAll traversal. Phase zero
        // intern hits and the reached empty groups have no further effects.
        Ok(RegistryExecutionPlan {
            name: name.to_owned(),
            arguments,
            namespace_growth: false,
            with_intern: false,
            allocation: AllocationPlan {
                countdown: effects.step_countdown.wrapping_sub(1),
                allocated: effects.allocated_bytes + 48,
                peak: effects
                    .peak_allocated_bytes
                    .max(effects.allocated_bytes + 48),
                threshold: effects.collection_threshold,
                inhibited: effects.collection_inhibited,
            },
            protected_obligations: effects.protected_closure_obligations + 1,
            result: any_nonempty.then_some(true),
            factory: None,
        })
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
        self.test_all_bounded_inner(name, arguments, effects, factory, None)
    }

    /// Extend the bounded source projection with explicitly known intern misses.
    /// Ordinary allocation success is an initial-context requirement, as for
    /// `test_all_bounded`. Cached calls still require all three intern hits.
    /// No collector or unknown intern membership is admitted.
    pub fn test_all_with_intern(
        &mut self,
        name: &str,
        arguments: [i32; 3],
        effects: &mut CachedExecutionState,
        factory: &mut FactoryExecutionState,
        intern: &mut InternExecutionState,
    ) -> Result<Option<bool>, RegistryError> {
        self.test_all_bounded_inner(name, arguments, effects, factory, Some(intern))
    }

    fn test_all_bounded_inner(
        &mut self,
        name: &str,
        arguments: [i32; 3],
        effects: &mut CachedExecutionState,
        factory: &mut FactoryExecutionState,
        intern: Option<&mut InternExecutionState>,
    ) -> Result<Option<bool>, RegistryError> {
        let plan = self.plan_test_all(name, arguments, effects, factory, intern.as_deref())?;
        Ok(self
            .prepare_commit(effects, factory, intern, plan)?
            .commit())
    }

    /// Plan one bounded TestAll without changing registry or execution state.
    /// No whole graph or intern membership set is copied.
    pub fn plan_test_all(
        &self,
        name: &str,
        arguments: [i32; 3],
        effects: &CachedExecutionState,
        factory: &FactoryExecutionState,
        intern: Option<&InternExecutionState>,
    ) -> Result<RegistryExecutionPlan, RegistryError> {
        self.plan_invocation(
            name,
            RegistryArguments::TestAll(arguments),
            effects,
            factory,
            intern,
            false,
        )
    }

    fn plan_invocation(
        &self,
        name: &str,
        arguments: RegistryArguments,
        effects: &CachedExecutionState,
        factory: &FactoryExecutionState,
        intern: Option<&InternExecutionState>,
        namespace_growth: bool,
    ) -> Result<RegistryExecutionPlan, RegistryError> {
        if self.cached_event(name).is_some() {
            if intern.as_ref().is_some_and(|state| {
                ["GameEvents", name, arguments.method()]
                    .iter()
                    .any(|key| state.known_absent.contains(*key))
            }) {
                return Err(RegistryError::UnsupportedEffect(
                    "contradictory cached intern membership",
                ));
            }
            let mut plan = self.plan_cached(name, arguments, effects)?;
            plan.with_intern = intern.is_some();
            return Ok(plan);
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
        if effects.call_capacity
            != match arguments {
                RegistryArguments::TestAll(_) => {
                    CachedCallCapacity::NoGrowthForFactoryThreeIntegers
                }
                RegistryArguments::Call(_) => CachedCallCapacity::NoGrowthForFactorySixIntegers,
            }
        {
            return Err(RegistryError::UnsupportedEffect(
                "unknown factory call growth effects",
            ));
        }
        if effects.collector_phase != 0 {
            return Err(RegistryError::UnsupportedEffect(
                "collector phase outside factory frontier",
            ));
        }
        if !namespace_growth && !factory.game_event_spare_names.contains(name) {
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
        // These existing rooted string keys cannot coherently be intern misses.
        // Native lookup compares intern identity, rather than Rust string bytes.
        if intern.is_some()
            && (!effects.interned_names.contains("GameEvents")
                || (matches!(entry, KnownNameEntry::Shared(_))
                    && !effects.interned_names.contains(name)))
        {
            return Err(RegistryError::UnsupportedEffect(
                "unknown rooted string intern membership",
            ));
        }
        if intern.is_some() && matches!(arguments, RegistryArguments::TestAll(_)) {
            // Known retained event objects root their keys and closure names.
            // A miss cannot create a second native intern identity for them.
            let retained_event = !self.cache.is_empty() || !factory.factory_objects.is_empty();
            let retained_factory = !factory.factory_objects.is_empty();
            let retained_weak = factory
                .factory_objects
                .iter()
                .any(|o| o.weak_metatable.is_some());
            for key in REGISTERED_METHODS.into_iter().chain([
                "func",
                "__metatable",
                "GameEvent::lCreate",
                "__mode",
                "v",
            ]) {
                let rooted = if key == "GameEvent::lCreate" {
                    retained_factory
                } else if matches!(key, "__mode" | "v") {
                    retained_weak && entry == KnownNameEntry::Absent
                } else {
                    retained_event
                };
                let rooted = rooted
                    || matches!(self.names.get(key), Some(KnownNameEntry::Shared(_)))
                    || self.cache.contains_key(key);
                if rooted && !effects.interned_names.contains(key) {
                    return Err(RegistryError::UnsupportedEffect(
                        "unknown retained object intern membership",
                    ));
                }
            }
        }
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
        let mut intern_plan = intern.map(InternPlan::new);
        // Ordered source ledger: each check sees OLD signed countdown and OLD
        // current bytes. Resize allocates before freeing old backing. Intern
        // operations occur at their source positions, including repeated hits.
        allocation.charge(1, &[48], 0)?;
        plan_intern("GameEvents", effects, &mut intern_plan, &mut allocation)?;
        plan_intern(name, effects, &mut intern_plan, &mut allocation)?;
        if entry == KnownNameEntry::Absent {
            allocation.charge(3, &[64], 0)?; // shared table
            allocation.charge(3, &[64, 16], 0)?; // weak metatable
            plan_intern("__mode", effects, &mut intern_plan, &mut allocation)?;
            plan_intern("v", effects, &mut intern_plan, &mut allocation)?;
            allocation.charge(1, &[40], 16)?; // __mode string insertion
        }
        allocation.charge(1, &[64], 0)?; // temporary factory closure
        plan_intern(
            "GameEvent::lCreate",
            effects,
            &mut intern_plan,
            &mut allocation,
        )?;
        allocation.charge(3, &[64], 0)?; // event
        allocation.charge(3, &[64], 0)?; // local array
        if entry == KnownNameEntry::Absent {
            allocation.charge(1, &[16], 0)?; // first shared numeric slot
        }
        // Native registration order differs from REGISTERED_METHODS indexing.
        const METHOD_ORDER: [usize; 9] = [0, 3, 1, 2, 4, 7, 8, 6, 5];
        let mut old_hash_bytes = 0;
        for insertion in 1u64..=9 {
            let method = METHOD_ORDER[insertion as usize - 1];
            plan_intern(
                REGISTERED_METHODS[method],
                effects,
                &mut intern_plan,
                &mut allocation,
            )?;
            allocation.charge(1, &[64], 0)?;
            plan_intern("func", effects, &mut intern_plan, &mut allocation)?;
            if matches!(insertion, 1 | 2 | 3 | 5 | 9) {
                let hash_bytes = 40 * insertion.next_power_of_two();
                allocation.charge(1, &[hash_bytes], old_hash_bytes)?;
                old_hash_bytes = hash_bytes;
            }
        }
        allocation.charge(3, &[64, 160], 0)?; // event metatable
        plan_intern("__newindex", effects, &mut intern_plan, &mut allocation)?;
        plan_intern("__metatable", effects, &mut intern_plan, &mut allocation)?;
        allocation.charge(3, &[64], 0)?; // protected empty metatable
        if namespace_growth {
            allocation.charge(1, &[80], 40)?;
        }
        plan_intern(
            arguments.method(),
            effects,
            &mut intern_plan,
            &mut allocation,
        )?;
        let intern_delta = intern_plan.map(|p| (p.count, p.capacity, p.added));

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

        Ok(RegistryExecutionPlan {
            name: name.to_owned(),
            arguments,
            namespace_growth,
            with_intern: intern.is_some(),
            allocation,
            protected_obligations,
            // The new local group is empty; compute the aggregate over the
            // actual existing shared traversal, whose reached lengths were
            // admitted above. No callback result is supplied by the caller.
            result: match entry {
                KnownNameEntry::Absent => None,
                KnownNameEntry::Shared(id) => self.shared_groups[&id]
                    .entries_in_native_order.iter().any(|entry| {
                        matches!(entry, GroupEntry::Table(local) if self.local_groups[local].objlen != 0)
                    }).then_some(true),
            },
            factory: Some(FactoryPlan {
                entry,
                append_position,
                end,
                objects: FactoryObjects {
                    event: identity,
                    protected_closure,
                    factory_closure,
                    event_metatable,
                    protected_metatable,
                    weak_metatable,
                    swept_construction_order: order,
                },
                intern_delta,
            }),
        })
    }

    /// Recompute the small delta from current source-used facts, then retain
    /// exclusive access so actor and registry commits need no fallible work.
    pub fn prepare_commit<'a>(
        &'a mut self,
        effects: &'a mut CachedExecutionState,
        factory: &'a mut FactoryExecutionState,
        intern: Option<&'a mut InternExecutionState>,
        plan: RegistryExecutionPlan,
    ) -> Result<PreparedRegistryCommit<'a>, RegistryError> {
        let current = self.plan_invocation(
            &plan.name,
            plan.arguments,
            effects,
            factory,
            intern.as_deref(),
            plan.namespace_growth,
        )?;
        if current != plan {
            return Err(RegistryError::UnsupportedEffect(
                "stale registry execution plan",
            ));
        }
        Ok(PreparedRegistryCommit {
            registry: self,
            effects,
            factory,
            intern,
            plan,
        })
    }

    fn commit_plan(
        &mut self,
        effects: &mut CachedExecutionState,
        factory: &mut FactoryExecutionState,
        intern: Option<&mut InternExecutionState>,
        plan: RegistryExecutionPlan,
    ) -> Option<bool> {
        plan.commit_effects(effects);
        let result = plan.result();
        let Some(delta) = plan.factory else {
            return result;
        };
        let name = plan.name;
        let FactoryPlan {
            entry,
            append_position,
            end,
            objects,
            intern_delta,
        } = delta;
        let identity = objects.event;
        let shared_group = identity.shared_group;
        let local_group = identity.local_group;
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
        factory.factory_objects.push(objects);
        if let (Some(intern), Some((count, capacity, added))) = (intern, intern_delta) {
            intern.count = count;
            intern.capacity = capacity;
            for name in added {
                intern.known_absent.remove(&name);
                effects.interned_names.insert(name);
            }
        }
        result
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

    fn intern_state(effects: &mut CachedExecutionState, missing: &[&str]) -> InternExecutionState {
        for name in missing {
            effects.interned_names.remove(*name);
        }
        InternExecutionState {
            count: 64,
            capacity: 128,
            scan_index: Some(0),
            known_absent: missing.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn call_context() -> (
        ScriptEventRegistry,
        BoundedExecutionState,
        FactoryExecutionState,
    ) {
        let (mut registry, mut effects, mut factory) = factory_context(false);
        registry
            .names
            .insert("OnUnitMoved".into(), KnownNameEntry::Absent);
        factory.name_map_spare_names.insert("OnUnitMoved".into());
        effects.call_capacity = CachedCallCapacity::NoGrowthForFactorySixIntegers;
        effects.step_countdown = -1024;
        effects.allocated_bytes = 37_332_465;
        effects.peak_allocated_bytes = 37_334_743;
        effects.collection_threshold = 40_951_825;
        let unknown = ["OnUnitMoved", "GameEvent::lCreate", "func", "__newindex"].map(String::from);
        for key in &unknown {
            effects.interned_names.remove(key);
        }
        let knowledge = BoundedExecutionState::from_unknown_memberships(
            effects,
            InternExecutionState {
                count: 31_054,
                capacity: 65_536,
                scan_index: None,
                known_absent: Default::default(),
            },
            &unknown,
            Some((1, 1)),
        )
        .unwrap();
        (registry, knowledge, factory)
    }

    #[test]
    fn call_six_intern_histories_share_objects_and_cached_reuse() {
        let (mut registry, mut knowledge, mut factory) = call_context();
        let prior = registry.lookup("CanStartOperation").unwrap();
        let initial = knowledge.clone();
        let plan = registry
            .plan_call_six("OnUnitMoved", [0, 7, 1, 2, 3, 4], &knowledge, &factory)
            .unwrap();
        assert_eq!(plan.result(), None); // native builtin Call returns nil
        assert_eq!(knowledge, initial);
        registry
            .prepare_call_six(&mut knowledge, &mut factory, plan)
            .unwrap()
            .commit();
        let event = registry.cached_event("OnUnitMoved").unwrap();
        assert_eq!(
            event
                .methods
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            9
        );
        assert_ne!(event.event, prior.event);
        assert_ne!(event.shared_group, prior.shared_group);
        assert!(event.methods.iter().all(|id| !prior.methods.contains(id)));
        assert_ne!(event.event, event.shared_group);
        assert_ne!(event.shared_group, event.local_group);
        assert_eq!(knowledge.alternatives.len(), 16);
        assert_eq!(
            knowledge
                .alternatives
                .iter()
                .map(|a| a.execution.allocated_bytes)
                .min(),
            Some(37_332_465 + 1968)
        );
        assert_eq!(
            knowledge
                .alternatives
                .iter()
                .map(|a| a.execution.allocated_bytes)
                .max(),
            Some(37_332_465 + 1968 + 151)
        );
        for a in &knowledge.alternatives {
            assert_eq!(a.intern.scan_index, None);
            assert_eq!(a.intern.capacity, 65_536);
            assert_eq!(
                a.execution.step_countdown,
                -1024 - 37 - (a.intern.count - 31_054) as i32
            );
            assert_eq!(a.execution.peak_allocated_bytes, 37_334_743);
            assert!(a.execution.interned_names.contains("OnUnitMoved"));
            assert!(a.intern.known_absent.is_empty());
        }
        let before = knowledge.clone();
        let plan = registry
            .plan_call_six("OnUnitMoved", [9; 6], &knowledge, &factory)
            .unwrap();
        registry
            .prepare_call_six(&mut knowledge, &mut factory, plan)
            .unwrap()
            .commit();
        assert_eq!(registry.cached_event("OnUnitMoved"), Some(event));
        assert_eq!(factory.factory_objects.len(), 1);
        for (a, b) in knowledge.alternatives.iter().zip(&before.alternatives) {
            assert_eq!(
                a.execution.allocated_bytes,
                b.execution.allocated_bytes + 48
            );
            assert_eq!(a.intern, b.intern);
        }
    }

    #[test]
    fn bounded_cached_test_all_then_call_preserves_all_histories() {
        let (mut registry, mut knowledge, mut factory) = call_context();
        registry.lookup("CanStartOperation").unwrap();
        let plan = registry
            .plan_test_all_alternatives("CanStartOperation", [0, 7, 5], &knowledge, &factory)
            .unwrap();
        assert_eq!(plan.result(), None);
        registry
            .prepare_call_six(&mut knowledge, &mut factory, plan)
            .unwrap()
            .commit();
        assert_eq!(knowledge.namespace, Some((1, 1)));
        let plan = registry
            .plan_call_six("OnUnitMoved", [0; 6], &knowledge, &factory)
            .unwrap();
        registry
            .prepare_call_six(&mut knowledge, &mut factory, plan)
            .unwrap()
            .commit();
        assert_eq!(
            knowledge
                .alternatives
                .iter()
                .map(|a| a.execution.allocated_bytes)
                .min(),
            Some(37_332_465 + 2016)
        );
        assert_eq!(
            knowledge
                .alternatives
                .iter()
                .map(|a| a.execution.allocated_bytes)
                .max(),
            Some(37_332_465 + 2167)
        );
    }

    #[test]
    fn bounded_call_refuses_any_later_collector_frontier_atomically() {
        let (mut registry, mut knowledge, mut factory) = call_context();
        let plan = registry
            .plan_call_six("OnUnitMoved", [0; 6], &knowledge, &factory)
            .unwrap();
        registry
            .prepare_call_six(&mut knowledge, &mut factory, plan)
            .unwrap()
            .commit();
        let last = knowledge.alternatives.last_mut().unwrap();
        last.execution.collection_threshold = last.execution.allocated_bytes - 1;
        let before = (registry.clone(), knowledge.clone(), factory.clone());
        assert!(matches!(
            registry.plan_call_six("OnUnitMoved", [0; 6], &knowledge, &factory),
            Err(RegistryError::UnsupportedEffect("collector progression"))
        ));
        assert_eq!((registry, knowledge, factory), before);
    }

    #[test]
    fn deferred_factory_plan_rechecks_changed_public_inputs_without_writes() {
        for changed in 0..9 {
            let (mut registry, mut effects, mut factory) = factory_context(false);
            let mut intern = intern_state(&mut effects, &["func"]);
            let plan = registry
                .plan_test_all(
                    "CanStartOperation",
                    [0, 7, 36],
                    &effects,
                    &factory,
                    Some(&intern),
                )
                .unwrap();
            match changed {
                0 => effects.allocated_bytes += 1,
                1 => effects.step_countdown += 1,
                2 => effects.protected_closure_obligations += 1,
                3 => effects.call_capacity = CachedCallCapacity::Unknown,
                4 => effects
                    .interned_names
                    .remove("GameEvents")
                    .then_some(())
                    .unwrap(),
                5 => {
                    factory.game_event_spare_names.clear();
                }
                6 => intern.count += 1,
                7 => {
                    intern.known_absent.clear();
                }
                8 => effects.collector_phase = 1,
                _ => unreachable!(),
            }
            let before = (
                registry.clone(),
                effects.clone(),
                factory.clone(),
                intern.clone(),
            );
            assert!(
                registry
                    .prepare_commit(&mut effects, &mut factory, Some(&mut intern), plan,)
                    .is_err(),
                "changed input {changed}"
            );
            assert_eq!((registry, effects, factory, intern), before);
        }
    }

    #[test]
    fn prepared_guard_can_be_dropped_and_commit_preserves_unrelated_facts() {
        let (mut registry, mut effects, mut factory) = factory_context(false);
        let mut intern = intern_state(&mut effects, &["func"]);
        let before = (
            registry.clone(),
            effects.clone(),
            factory.clone(),
            intern.clone(),
        );
        let plan = registry
            .plan_test_all(
                "CanStartOperation",
                [0; 3],
                &effects,
                &factory,
                Some(&intern),
            )
            .unwrap();
        assert_eq!(plan.result(), None);
        drop(
            registry
                .prepare_commit(&mut effects, &mut factory, Some(&mut intern), plan)
                .unwrap(),
        );
        assert_eq!(
            (
                registry.clone(),
                effects.clone(),
                factory.clone(),
                intern.clone()
            ),
            before
        );

        let plan = registry
            .plan_test_all(
                "CanStartOperation",
                [0; 3],
                &effects,
                &factory,
                Some(&intern),
            )
            .unwrap();
        // Unrelated membership is preserved, rather than restoring a cloned
        // intern set. Its count already fits the supplied native count.
        effects.interned_names.insert("unrelated".into());
        intern.known_absent.insert("another".into());
        let guard = registry
            .prepare_commit(&mut effects, &mut factory, Some(&mut intern), plan)
            .unwrap();
        assert_eq!(guard.commit(), None);
        assert!(effects.interned_names.contains("unrelated"));
        assert!(intern.known_absent.contains("another"));
        assert!(!intern.known_absent.contains("func"));
        let bytes = effects.allocated_bytes;
        let plan = registry
            .plan_test_all(
                "CanStartOperation",
                [0; 3],
                &effects,
                &factory,
                Some(&intern),
            )
            .unwrap();
        assert_eq!(
            registry
                .prepare_commit(&mut effects, &mut factory, Some(&mut intern), plan)
                .unwrap()
                .commit(),
            None
        );
        assert_eq!(effects.allocated_bytes, bytes + 48);
    }

    #[test]
    fn deferred_plan_rechecks_registry_and_cached_collector_boundary() {
        let (mut registry, mut effects, mut factory) = factory_context(false);
        let plan = registry
            .plan_test_all("CanStartOperation", [0; 3], &effects, &factory, None)
            .unwrap();
        registry.lookup("later").unwrap();
        let before = (registry.clone(), effects.clone(), factory.clone());
        assert!(registry
            .prepare_commit(&mut effects, &mut factory, None, plan)
            .is_err());
        assert_eq!((registry, effects, factory), before);

        let (seed, cache) = cached_seed();
        let mut registry = ScriptEventRegistry::from_seed_with_cache(seed, cache).unwrap();
        let mut effects = cached_effects();
        let mut factory = FactoryExecutionState::default();
        let plan = registry
            .plan_test_all("CanStartOperation", [0; 3], &effects, &factory, None)
            .unwrap();
        registry
            .test_all_cached("CanStartOperation", [0; 3], &mut effects)
            .unwrap();
        let before = effects.clone();
        assert_eq!(
            registry
                .prepare_commit(&mut effects, &mut factory, None, plan)
                .err(),
            Some(RegistryError::UnsupportedEffect("collector progression"))
        );
        assert_eq!(effects, before);
    }

    #[test]
    fn factory_intern_misses_are_charged_once_and_persist_into_cached_calls() {
        for reused in [false, true] {
            let (mut registry, mut effects, mut factory) = factory_context(reused);
            let missing: &[&str] = if reused {
                &["func", "TestAll"]
            } else {
                &["CanStartOperation", "func", "TestAll"]
            };
            let mut intern = intern_state(&mut effects, missing);
            let initial = effects.clone();
            assert_eq!(
                registry.test_all_with_intern(
                    "CanStartOperation",
                    [0; 3],
                    &mut effects,
                    &mut factory,
                    &mut intern
                ),
                Ok(None)
            );
            let bytes: u64 = missing.iter().map(|name| name.len() as u64 + 27).sum();
            assert_eq!(
                effects.allocated_bytes,
                initial.allocated_bytes + if reused { 1744 } else { 1928 } + 1024 + bytes
            );
            assert_eq!(
                effects.step_countdown,
                initial.step_countdown - if reused { 28 } else { 36 } - missing.len() as i32
            );
            assert_eq!(intern.count, 64 + missing.len() as u32);
            assert_eq!(intern.capacity, 256);
            assert_eq!(intern.scan_index, Some(0));
            assert!(intern.known_absent.is_empty());
            for name in missing {
                assert!(effects.interned_names.contains(*name));
            }
            let before_intern = intern.clone();
            let before_bytes = effects.allocated_bytes;
            let before_countdown = effects.step_countdown;
            assert_eq!(
                registry.test_all_with_intern(
                    "CanStartOperation",
                    [0; 3],
                    &mut effects,
                    &mut factory,
                    &mut intern
                ),
                Ok(None)
            );
            assert_eq!(intern, before_intern);
            assert_eq!(effects.allocated_bytes, before_bytes + 48);
            assert_eq!(effects.step_countdown, before_countdown - 1);
        }
    }

    #[test]
    fn intern_growth_has_no_charge_and_guard_sees_net_after_free() {
        let (_, mut effects, _) = factory_context(true);
        let mut initial = intern_state(&mut effects, &["func"]);
        initial.count = 32;
        initial.capacity = 64;
        let mut plan = Some(InternPlan::new(&initial));
        let mut allocation = AllocationPlan {
            countdown: 1,
            allocated: 100,
            peak: 100,
            threshold: 612,
            inhibited: false,
        };
        plan_intern("func", &effects, &mut plan, &mut allocation).unwrap();
        assert_eq!(allocation.countdown, 0);
        assert_eq!(allocation.allocated, 643);
        assert_eq!(allocation.peak, 1124);
        assert_eq!(plan.as_ref().unwrap().capacity, 128);
        // Repeated hit neither charges nor rechecks the now-exceeded threshold.
        plan_intern("func", &effects, &mut plan, &mut allocation).unwrap();
        assert_eq!(allocation.countdown, 0);
        assert_eq!(allocation.allocated, 643);
        let mut plan = Some(InternPlan::new(&initial));
        let mut allocation = AllocationPlan {
            countdown: 1,
            allocated: 100,
            peak: 100,
            threshold: 611,
            inhibited: false,
        };
        assert_eq!(
            plan_intern("func", &effects, &mut plan, &mut allocation),
            Err(RegistryError::UnsupportedEffect("collector progression"))
        );
    }

    #[test]
    fn intern_metadata_is_read_only_when_reached_and_count_guard_is_in_source_order() {
        let (mut registry, mut effects, mut factory) = factory_context(true);
        let mut intern = InternExecutionState {
            count: 0,
            capacity: 0,
            scan_index: Some(42),
            known_absent: Default::default(),
        };
        let before = intern.clone();
        assert_eq!(
            registry.test_all_with_intern(
                "CanStartOperation",
                [0; 3],
                &mut effects,
                &mut factory,
                &mut intern
            ),
            Ok(None)
        );
        assert_eq!(intern, before);

        let (mut registry, mut effects, mut factory) = factory_context(true);
        let mut intern = intern_state(&mut effects, &["Count"]);
        intern.capacity = 256;
        intern.scan_index = Some(42);
        let initial = effects.allocated_bytes;
        assert_eq!(
            registry.test_all_with_intern(
                "CanStartOperation",
                [0; 3],
                &mut effects,
                &mut factory,
                &mut intern
            ),
            Ok(None)
        );
        assert_eq!(intern.scan_index, Some(42));
        assert_eq!(intern.capacity, 256);
        assert_eq!(effects.allocated_bytes, initial + 1744 + 32);

        let (mut registry, mut effects, mut factory) = factory_context(true);
        let mut intern = intern_state(&mut effects, &["Count"]);
        intern.capacity = 256;
        effects.step_countdown = 10;
        // Before Count's intern: protected, factory, event, local, Add closure,
        // hash1 have allocated 344 bytes. Every preceding guard still passes.
        effects.collection_threshold = effects.allocated_bytes + 343;
        let before = (
            registry.clone(),
            effects.clone(),
            factory.clone(),
            intern.clone(),
        );
        assert_eq!(
            registry.test_all_with_intern(
                "CanStartOperation",
                [0; 3],
                &mut effects,
                &mut factory,
                &mut intern
            ),
            Err(RegistryError::UnsupportedEffect("collector progression"))
        );
        assert_eq!((registry, effects, factory, intern), before);
    }

    #[test]
    fn intern_misses_cannot_contradict_reached_rooted_keys() {
        for case in 0..3 {
            let (mut registry, mut effects, mut factory) = factory_context(true);
            let missing = match case {
                0 => "GameEvents",
                1 => "CanStartOperation",
                _ => "Count",
            };
            if case == 2 {
                registry.lookup("CanStartOperation").unwrap();
                factory.game_event_spare_names.insert("later".into());
                factory.name_map_spare_names.insert("later".into());
            }
            let mut intern = intern_state(&mut effects, &[missing]);
            let before = (
                registry.clone(),
                effects.clone(),
                factory.clone(),
                intern.clone(),
            );
            let name = if case == 2 {
                "later"
            } else {
                "CanStartOperation"
            };
            assert!(registry
                .test_all_with_intern(name, [0; 3], &mut effects, &mut factory, &mut intern)
                .is_err());
            assert_eq!((registry, effects, factory, intern), before);
        }
    }

    #[test]
    fn intern_unknown_marks_overflow_and_signed_guards_refuse_atomically() {
        for case in 0..10 {
            let (mut registry, mut effects, mut factory) = factory_context(true);
            let mut intern = intern_state(&mut effects, &["func"]);
            match case {
                0 => {
                    intern.known_absent.remove("func");
                }
                1 => intern.scan_index = Some(1),
                2 => intern.capacity = 127,
                3 => intern.count = u32::MAX,
                4 => {
                    intern.count = 1 << 30;
                    intern.capacity = 1 << 31;
                }
                5 => effects.allocated_bytes = u64::MAX - 100,
                6 => effects.collector_phase = 1,
                7 => {
                    effects.step_countdown = i32::MIN;
                    effects.collection_threshold = effects.allocated_bytes - 1;
                }
                8 => {
                    intern.known_absent.insert("GameEvents".into());
                }
                9 => registry.next_identity = u64::MAX,
                _ => unreachable!(),
            }
            let before = (
                registry.clone(),
                effects.clone(),
                factory.clone(),
                intern.clone(),
            );
            assert!(
                registry
                    .test_all_with_intern(
                        "CanStartOperation",
                        [0; 3],
                        &mut effects,
                        &mut factory,
                        &mut intern
                    )
                    .is_err(),
                "case {case}"
            );
            assert_eq!((registry, effects, factory, intern), before, "case {case}");
        }
        let (mut registry, mut effects, mut factory) = factory_context(true);
        let mut intern = intern_state(&mut effects, &["func"]);
        effects.step_countdown = i32::MIN;
        effects.collection_threshold = u64::MAX;
        assert_eq!(
            registry.test_all_with_intern(
                "CanStartOperation",
                [0; 3],
                &mut effects,
                &mut factory,
                &mut intern
            ),
            Ok(None)
        );
        assert_eq!(effects.step_countdown, i32::MIN.wrapping_sub(29));
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
