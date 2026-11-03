//! Bounded Linux XP2 notification selection, base activation and logical cleanup.
//!
//! This composes two original phases, not Manager::Update or an accepted action.
//! Wire records omit required live locks, definition inputs and pool ownership.
//! Reporting is synchronous through an explicit caller-supplied delivery model.
//! Complete gameplay state/action/turn and native callback parity remain open.
use crate::notifications::{ComponentId, Notification, PlayerNotifications};
use crate::unit::variants::VariantPayload;
use std::collections::BTreeSet;

pub const ACTIVATION_EVENT: u32 = 0x025f6cfd;
pub const IDENTITY_KEY: u32 = 0xeab69e5a;
pub const ACTIVATION_BOOL_KEY: u32 = 0x78a2f946;
pub const SEVERITY_OVERRIDE_KEY: u32 = 0xc362976f;

/// Caller-assigned logical object identity, independent of possibly duplicate IDs.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObjectId(pub u64);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PoolId(pub u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationBinding {
    /// Actual vtable slot 0x88 binds the original base Activate implementation.
    Base,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseBinding {
    Base,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestructionBinding {
    /// Base logical resources only; excludes subtype-owned extra resources.
    BaseResources,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionIcon {
    /// Getter was skipped by the native short circuit; no value is invented.
    Unobserved,
    MissingDefinition,
    ShowInSinglePlayer(bool),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveNotification {
    pub object: ObjectId,
    /// Actual signed native +0x194; only strictly positive means locked.
    pub lock_count: i32,
    /// Actual native pool pointer association, represented without addresses.
    pub pool_binding: Option<PoolId>,
    pub definition_icon: DefinitionIcon,
    pub activation: ActivationBinding,
    pub release: ReleaseBinding,
    pub destruction: DestructionBinding,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PooledNotification {
    pub notification: Notification,
    /// Release clears pool_binding before erasing the owner-list entry.
    pub live: LiveNotification,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pool {
    pub id: PoolId,
    /// Ordered existing pool pointers as logical handles. Their unobserved object
    /// fields are outside this phase and are never interpreted or reused.
    pub retained_objects: Vec<ObjectId>,
    /// Appended after retained_objects, in native vector order; Release appends. Allocation/reinitialization is not
    /// implemented here. The native Create path later takes back/pop (LIFO).
    pub records: Vec<PooledNotification>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub notifications: PlayerNotifications,
    /// Exact parallel correspondence to ordered nonnull owner-list entries.
    pub live: Vec<LiveNotification>,
    pub pools: Vec<Pool>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Gates {
    pub manager_initialized: bool,
    pub game_uninitializing: bool,
    pub human_configuration: bool,
    pub owner_turn_active: bool,
    pub network_multiplayer: bool,
    pub play_by_cloud: bool,
    /// None means the actual manager's game pointer is null.
    pub manager_game_local_player: Option<i32>,
}
impl Gates {
    pub fn accepts_notifications(self) -> bool {
        self.manager_initialized && !self.game_uninitializing && self.human_configuration
    }
    pub fn runs_auto_activation(self, owner: i32) -> bool {
        self.accepts_notifications()
            && self.owner_turn_active
            && (!self.play_by_cloud || self.manager_game_local_player == Some(owner))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    LiveRecordCount,
    MissingRecord(usize),
    DuplicateObject,
    DuplicatePool,
    MissingPool(PoolId),
    InvalidPooledBinding,
    UnsupportedOverrideValue,
    UnobservedDefinitionInput(ObjectId),
    UnsupportedActivation(ObjectId),
    UnsupportedDestruction(ObjectId),
    UnsupportedPayloadDestruction(ObjectId),
    UnsupportedRelease(ObjectId),
    ReportingUnbound,
    /// A caller-supplied subscriber invalidated the supported model.
    SubscriberChangedSupportedProfile,
    /// Explicit diagnostic resource bound; never a successful native result.
    ActivationBudgetExceeded,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivationReport {
    pub event_hash: u32,
    pub identity_key: u32,
    pub identity: ComponentId,
    pub bool_key: u32,
    pub activation_argument: bool,
}
/// A bound simulation of the actual synchronous reporting environment.
///
/// Unsupported delivery must reject in preflight. `send` runs after setting the
/// activation bit and before the next selection scan, and may update modeled
/// state just as original subscribers can. It must preserve the declared
/// supported ownership/virtual-binding profile. Clone must isolate all modeled
/// effects: external I/O/shared mutable references are outside this contract.
/// External game/configuration gates and owner-container identity must remain
/// fixed during delivery; native IsAutoActivate reads network configuration on
/// each scan. Record locks, definitions, ordering and flags may change in State.
/// Merely recording a report is explicitly a projection, not bound delivery.
pub trait ActivationDelivery: Clone {
    fn preflight(&self, state: &State) -> Result<(), Error>;
    fn send(&mut self, state: &mut State, report: ActivationReport);
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnershipEffect {
    ReturnedToPool { object: ObjectId, pool: PoolId },
    DestroyedBaseResources { object: ObjectId },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Phase<D> {
    pub state: State,
    pub delivery: D,
    pub ownership_effects: Vec<OwnershipEffect>,
    pub activations: usize,
}

fn override_flag(record: &Notification) -> Result<bool, Error> {
    // Native FindEntry walks the vector and returns its first matching key.
    let Some(entry) = record
        .parameters
        .entries
        .iter()
        .find(|e| e.key == SEVERITY_OVERRIDE_KEY)
    else {
        return Ok(false);
    };
    let value = &entry.value;
    if value.type_bits != 1
        || value.modifier_bits != 0
        || value.value.count_flags != 0
        || value.value.element_size != 0
    {
        return Err(Error::UnsupportedOverrideValue);
    }
    match value.value.payload {
        VariantPayload::Inline32(bits) if bits as u8 <= 1 => Ok(bits as u8 != 0),
        _ => Err(Error::UnsupportedOverrideValue),
    }
}
fn validate(state: &State) -> Result<(), Error> {
    if state.notifications.records.len() != state.live.len() {
        return Err(Error::LiveRecordCount);
    }
    let mut pools = BTreeSet::new();
    let mut objects = BTreeSet::new();
    for pool in &state.pools {
        if !pools.insert(pool.id) {
            return Err(Error::DuplicatePool);
        }
        for object in &pool.retained_objects {
            if !objects.insert(*object) {
                return Err(Error::DuplicateObject);
            }
        }
        for record in &pool.records {
            if record.live.pool_binding.is_some() {
                return Err(Error::InvalidPooledBinding);
            }
            if !objects.insert(record.live.object) {
                return Err(Error::DuplicateObject);
            }
        }
    }
    for (record, live) in state.notifications.records.iter().zip(&state.live) {
        if !objects.insert(live.object) {
            return Err(Error::DuplicateObject);
        }
        if let Some(pool) = live.pool_binding {
            if !pools.contains(&pool) {
                return Err(Error::MissingPool(pool));
            }
        }
        if record.is_dismissed && live.release != ReleaseBinding::Base {
            return Err(Error::UnsupportedRelease(live.object));
        }
        // Conservatively preflight all override values before any mutation.
        override_flag(record)?;
    }
    Ok(())
}
fn auto_activate(
    record: &Notification,
    live: &LiveNotification,
    network: bool,
) -> Result<bool, Error> {
    if record.has_been_activated {
        return Ok(true);
    }
    if !network {
        match live.definition_icon {
            DefinitionIcon::Unobserved => {
                return Err(Error::UnobservedDefinitionInput(live.object))
            }
            DefinitionIcon::ShowInSinglePlayer(false) => return Ok(true),
            _ => (),
        }
    }
    Ok(record.native_19d)
}
fn find_checked(state: &State, threshold: i32, network: bool) -> Result<Option<usize>, Error> {
    let mut candidate = None;
    for (index, (record, live)) in state
        .notifications
        .records
        .iter()
        .zip(&state.live)
        .enumerate()
    {
        if !auto_activate(record, live, network)?
            || (!override_flag(record)? && record.severity < threshold)
        {
            continue;
        }
        // No dismissed filter, and a later locked item preempts an earlier one.
        if live.lock_count > 0 {
            return Ok(Some(index));
        }
        if candidate.is_none() && !record.has_been_activated {
            candidate = Some(index);
        }
    }
    Ok(candidate)
}
/// Original FindAutoActivate; returns list position, not a notification ID.
pub fn find_auto_activate(
    state: &State,
    severity_threshold: i32,
    network: bool,
) -> Result<Option<usize>, Error> {
    validate(state)?;
    find_checked(state, severity_threshold, network)
}
fn preflight_destruction(record: &Notification, live: &LiveNotification) -> Result<(), Error> {
    if live.pool_binding.is_some() {
        return Ok(());
    }
    if live.destruction != DestructionBinding::BaseResources {
        return Err(Error::UnsupportedDestruction(live.object));
    }
    // Native DestroySelf can dispatch owned payloads through VariantManager.
    // Bound destruction to canonical nonowned scalar32 branches; the codec's
    // broader transport support does not establish those destructor effects.
    for entry in &record.parameters.entries {
        let value = &entry.value;
        if value.modifier_bits != 0
            || value.value.count_flags != 0
            || value.value.element_size != 0
            || !matches!(
                (value.type_bits, &value.value.payload),
                (0, VariantPayload::Inline32(0)) | (1..=4, VariantPayload::Inline32(_))
            )
        {
            return Err(Error::UnsupportedPayloadDestruction(live.object));
        }
    }
    Ok(())
}
fn preflight(state: &State, gates: Gates) -> Result<(), Error> {
    validate(state)?;
    for (record, live) in state.notifications.records.iter().zip(&state.live) {
        if gates.runs_auto_activation(state.notifications.owner) {
            auto_activate(record, live, gates.network_multiplayer)?;
            if live.release != ReleaseBinding::Base {
                return Err(Error::UnsupportedRelease(live.object));
            }
        }
        // This conservative supported profile permits callbacks to change which
        // existing records are selected/dismissed without reaching unknown code.
        if gates.runs_auto_activation(state.notifications.owner)
            && live.activation != ActivationBinding::Base
        {
            return Err(Error::UnsupportedActivation(live.object));
        }
        if record.is_dismissed || gates.runs_auto_activation(state.notifications.owner) {
            preflight_destruction(record, live)?;
        }
    }
    Ok(())
}
fn cleanup_checked(state: &mut State) -> Vec<OwnershipEffect> {
    let mut effects = Vec::new();
    let mut index = 0;
    while index < state.notifications.records.len() {
        if !state.notifications.records[index].is_dismissed {
            index += 1;
            continue;
        }
        let mut live = state.live[index].clone();
        let object = live.object;
        // Virtual Release happens before erasing the original list node.
        if let Some(pool_id) = live.pool_binding.take() {
            let pool = state
                .pools
                .iter_mut()
                .find(|p| p.id == pool_id)
                .expect("preflighted pool");
            pool.records.push(PooledNotification {
                notification: state.notifications.records[index].clone(),
                live,
            });
            effects.push(OwnershipEffect::ReturnedToPool {
                object,
                pool: pool_id,
            });
        } else {
            effects.push(OwnershipEffect::DestroyedBaseResources { object });
        }
        state.notifications.records.remove(index);
        state.live.remove(index);
    }
    effects
}
/// Original cleanup's logical ownership/list erase phase, with local mutation.
/// Unsupported ownership/destructor profiles reject before mutation.
pub fn cleanup_in_place(state: &mut State) -> Result<Vec<OwnershipEffect>, Error> {
    validate(state)?;
    for (record, live) in state.notifications.records.iter().zip(&state.live) {
        if record.is_dismissed {
            preflight_destruction(record, live)?;
        }
    }
    Ok(cleanup_checked(state))
}
/// Isolated component-copy adapter; production can use cleanup_in_place.
pub fn cleanup(state: &State) -> Result<(State, Vec<OwnershipEffect>), Error> {
    // Preflight before copying, and preserve caller state on all errors.
    validate(state)?;
    for (record, live) in state.notifications.records.iter().zip(&state.live) {
        if record.is_dismissed {
            preflight_destruction(record, live)?;
        }
    }
    let mut next = state.clone();
    let effects = cleanup_checked(&mut next);
    Ok((next, effects))
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InPlacePhase {
    pub ownership_effects: Vec<OwnershipEffect>,
    pub activations: usize,
}
fn activate_base_checked<D: ActivationDelivery>(
    state: &mut State,
    delivery: &mut D,
    index: usize,
    argument: bool,
) {
    let record = &mut state.notifications.records[index];
    // This store and report are unconditional, including an already-true bit.
    record.has_been_activated = true;
    let report = ActivationReport {
        event_hash: ACTIVATION_EVENT,
        identity_key: IDENTITY_KEY,
        identity: record.id,
        bool_key: ACTIVATION_BOOL_KEY,
        activation_argument: argument,
    };
    delivery.send(state, report);
}
/// Original BaseActivate(bool), distinct from the automatic false-argument pass.
/// Unsupported binding/delivery/index rejects before mutation. Reporting runs
/// synchronously after the unconditional activation-bit store.
pub fn activate_base_in_place<D: ActivationDelivery>(
    state: &mut State,
    index: usize,
    argument: bool,
    delivery: &mut D,
) -> Result<(), Error> {
    validate(state)?;
    let live = state.live.get(index).ok_or(Error::MissingRecord(index))?;
    if live.activation != ActivationBinding::Base {
        return Err(Error::UnsupportedActivation(live.object));
    }
    delivery.preflight(state)?;
    activate_base_checked(state, delivery, index, argument);
    validate(state).map_err(|_| Error::SubscriberChangedSupportedProfile)
}
/// ProcessAutoActivate then Cleanup, excluding SendAutoNotify/DismissExpired.
///
/// No state copy is made. Initial unsupported profiles and unbound reporting
/// reject before mutation. A hook must preserve its preflighted supported
/// contract; a violated hook contract or exhausted diagnostic budget can leave
/// a partial prefix, and the error must never count as native completion.
pub fn activation_and_cleanup_in_place<D: ActivationDelivery>(
    state: &mut State,
    gates: Gates,
    delivery: &mut D,
    activation_budget: usize,
) -> Result<InPlacePhase, Error> {
    preflight(state, gates)?;
    if gates.runs_auto_activation(state.notifications.owner) {
        delivery.preflight(state)?;
    }
    if activation_budget == 0 && gates.runs_auto_activation(state.notifications.owner) {
        if let Some(index) = find_checked(
            state,
            if gates.network_multiplayer { 3 } else { 0 },
            gates.network_multiplayer,
        )? {
            if state.live[index].lock_count <= 0 {
                return Err(Error::ActivationBudgetExceeded);
            }
        }
    }
    let mut activations = 0;
    if gates.runs_auto_activation(state.notifications.owner) {
        let threshold = if gates.network_multiplayer { 3 } else { 0 };
        while let Some(index) = find_checked(state, threshold, gates.network_multiplayer)? {
            if state.live[index].lock_count > 0 {
                break;
            }
            if activations == activation_budget {
                return Err(Error::ActivationBudgetExceeded);
            }
            activate_base_checked(state, delivery, index, false);
            activations += 1;
            preflight(state, gates).map_err(|_| Error::SubscriberChangedSupportedProfile)?;
        }
    }
    let ownership_effects = cleanup_checked(state);
    Ok(InPlacePhase {
        ownership_effects,
        activations,
    })
}
/// Isolated component-copy adapter for differential checking, not a world copy.
/// All modeled mutations occur on cloned component/delivery state. Production
/// callers can use the separately preflighted in-place entry point.
pub fn activation_and_cleanup<D: ActivationDelivery>(
    state: &State,
    gates: Gates,
    delivery: &D,
    activation_budget: usize,
) -> Result<Phase<D>, Error> {
    preflight(state, gates)?;
    if gates.runs_auto_activation(state.notifications.owner) {
        delivery.preflight(state)?;
    }
    let mut next = state.clone();
    let mut delivery = delivery.clone();
    let phase =
        activation_and_cleanup_in_place(&mut next, gates, &mut delivery, activation_budget)?;
    Ok(Phase {
        state: next,
        delivery,
        ownership_effects: phase.ownership_effects,
        activations: phase.activations,
    })
}
/// Diagnostic projection only: records undelivered reports. It deliberately
/// makes no claim that native synchronous subscribers were executed or absent.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UndeliveredProjection {
    pub reports: Vec<ActivationReport>,
}
impl ActivationDelivery for UndeliveredProjection {
    fn preflight(&self, _: &State) -> Result<(), Error> {
        Ok(())
    }
    fn send(&mut self, _: &mut State, report: ActivationReport) {
        self.reports.push(report);
    }
}
/// Use when an actual synchronous provider environment has not been bound.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnboundDelivery;
impl ActivationDelivery for UnboundDelivery {
    fn preflight(&self, _: &State) -> Result<(), Error> {
        Err(Error::ReportingUnbound)
    }
    fn send(&mut self, _: &mut State, _: ActivationReport) {
        unreachable!("unbound delivery rejected before mutation")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::variants::{ArrayVariant, TypedEntry, TypedVariant};
    fn gates() -> Gates {
        Gates {
            manager_initialized: true,
            game_uninitializing: false,
            human_configuration: true,
            owner_turn_active: true,
            network_multiplayer: false,
            play_by_cloud: false,
            manager_game_local_player: None,
        }
    }
    fn fixture(count: usize) -> State {
        State {
            notifications: PlayerNotifications {
                owner: 7,
                next_id: 900,
                records: (0..count)
                    .map(|n| Notification {
                        id: ComponentId {
                            owner: 7,
                            component_type: 3,
                            id: 100 + n as u32,
                        },
                        severity: 1,
                        native_19d: true,
                        ..Default::default()
                    })
                    .collect(),
            },
            live: (0..count)
                .map(|n| LiveNotification {
                    object: ObjectId(n as u64),
                    lock_count: 0,
                    pool_binding: None,
                    definition_icon: DefinitionIcon::ShowInSinglePlayer(true),
                    activation: ActivationBinding::Base,
                    release: ReleaseBinding::Base,
                    destruction: DestructionBinding::BaseResources,
                })
                .collect(),
            pools: vec![],
        }
    }
    fn override_entry(bits: u32) -> TypedEntry {
        TypedEntry {
            key: SEVERITY_OVERRIDE_KEY,
            name: None,
            value: TypedVariant {
                type_bits: 1,
                modifier_bits: 0,
                value: ArrayVariant {
                    count_flags: 0,
                    element_size: 0,
                    payload: VariantPayload::Inline32(bits),
                },
            },
        }
    }
    #[test]
    fn locked_preempts_retained_candidate_even_activated_or_dismissed() {
        let mut s = fixture(3);
        s.live[2].lock_count = 1;
        s.notifications.records[2].has_been_activated = true;
        s.notifications.records[2].is_dismissed = true;
        assert_eq!(find_auto_activate(&s, 0, false), Ok(Some(2)));
        s.live[2].lock_count = i32::MIN;
        assert_eq!(find_auto_activate(&s, 0, false), Ok(Some(0)));
        s.live[0].lock_count = -1;
        assert_eq!(find_auto_activate(&s, 0, false), Ok(Some(0)));
        let r = activation_and_cleanup(&s, gates(), &UndeliveredProjection::default(), 10).unwrap();
        assert_eq!(r.activations, 2);
        assert_eq!(r.state.notifications.records.len(), 2);
        assert_eq!(r.state.notifications.next_id, 900);
    }
    #[test]
    fn signed_severity_first_override_and_definition_network_predicates() {
        let mut s = fixture(1);
        s.notifications.records[0].severity = i32::MIN;
        assert_eq!(find_auto_activate(&s, 0, false), Ok(None));
        s.notifications.records[0].parameters.entries = vec![override_entry(0xabcd0001)];
        assert_eq!(find_auto_activate(&s, 0, false), Ok(Some(0)));
        s.notifications.records[0]
            .parameters
            .entries
            .insert(0, override_entry(0));
        assert_eq!(find_auto_activate(&s, 0, false), Ok(None));
        s.notifications.records[0].parameters.entries.clear();
        s.notifications.records[0].severity = 1;
        s.notifications.records[0].native_19d = false;
        s.live[0].definition_icon = DefinitionIcon::ShowInSinglePlayer(false);
        assert_eq!(find_auto_activate(&s, 0, false), Ok(Some(0)));
        assert_eq!(find_auto_activate(&s, 0, true), Ok(None));
        s.notifications.records[0].native_19d = true;
        assert_eq!(find_auto_activate(&s, 3, true), Ok(None));
        s.notifications.records[0].severity = 3;
        assert_eq!(find_auto_activate(&s, 3, true), Ok(Some(0)));
        s.notifications.records[0].native_19d = false;
        s.live[0].definition_icon = DefinitionIcon::MissingDefinition;
        assert_eq!(find_auto_activate(&s, 0, false), Ok(None));
    }
    #[test]
    fn release_preserves_pool_and_survivor_order_without_id_filter() {
        let mut s = fixture(4);
        s.pools.push(Pool {
            id: PoolId(22),
            retained_objects: vec![],
            records: vec![],
        });
        for index in [0, 2, 3] {
            s.notifications.records[index].is_dismissed = true;
        }
        s.live[0].pool_binding = Some(PoolId(22));
        s.live[2].pool_binding = Some(PoolId(22));
        // Duplicate notification IDs are fine; logical ownership must be unique.
        s.notifications.records[2].id = s.notifications.records[0].id;
        let mut live = s.clone();
        let effects = cleanup_in_place(&mut live).unwrap();
        assert_eq!(
            effects,
            [
                OwnershipEffect::ReturnedToPool {
                    object: ObjectId(0),
                    pool: PoolId(22)
                },
                OwnershipEffect::ReturnedToPool {
                    object: ObjectId(2),
                    pool: PoolId(22)
                },
                OwnershipEffect::DestroyedBaseResources {
                    object: ObjectId(3)
                }
            ]
        );
        assert_eq!(
            live.notifications.records,
            [s.notifications.records[1].clone()]
        );
        assert_eq!(live.notifications.next_id, 900);
        assert_eq!(
            live.pools[0]
                .records
                .iter()
                .map(|r| r.live.object)
                .collect::<Vec<_>>(),
            [ObjectId(0), ObjectId(2)]
        );
        assert!(live.pools[0]
            .records
            .iter()
            .all(|r| r.live.pool_binding.is_none()));
    }
    #[test]
    fn unsupported_ownership_virtuals_and_delivery_fail_before_mutation() {
        let base = fixture(2);
        for mode in 0..8 {
            let mut s = base.clone();
            match mode {
                0 => s.live[1].object = s.live[0].object,
                1 => s.live[0].pool_binding = Some(PoolId(4)),
                2 => s.live[0].activation = ActivationBinding::Unsupported,
                3 => {
                    s.notifications.records[0].is_dismissed = true;
                    s.live[0].destruction = DestructionBinding::Unsupported;
                }
                4 => {
                    s.notifications.records[0].is_dismissed = true;
                    s.live[0].release = ReleaseBinding::Unsupported;
                }
                5 => {
                    let mut bad = override_entry(1);
                    bad.value.type_bits = 2;
                    s.notifications.records[0].parameters.entries.push(bad);
                }
                6 => {
                    s.live.pop();
                }
                _ => {
                    s.live[0].release = ReleaseBinding::Unsupported;
                }
            }
            let original = s.clone();
            let mut sink = UndeliveredProjection::default();
            assert!(activation_and_cleanup_in_place(&mut s, gates(), &mut sink, 10).is_err());
            assert_eq!(s, original);
            assert!(sink.reports.is_empty());
        }
        let mut s = base.clone();
        let mut unbound = UnboundDelivery;
        assert_eq!(
            activation_and_cleanup_in_place(&mut s, gates(), &mut unbound, 10),
            Err(Error::ReportingUnbound)
        );
        assert_eq!(s, base);
        let mut sink = UndeliveredProjection::default();
        assert_eq!(
            activation_and_cleanup_in_place(&mut s, gates(), &mut sink, 0),
            Err(Error::ActivationBudgetExceeded)
        );
        assert_eq!(s, base);
        assert!(sink.reports.is_empty());
    }
    #[test]
    fn unpooled_owned_payload_destruction_rejects_but_pool_release_preserves_it() {
        let mut state = fixture(1);
        state.notifications.records[0].is_dismissed = true;
        state.notifications.records[0]
            .parameters
            .entries
            .push(TypedEntry {
                key: 17,
                name: None,
                value: TypedVariant {
                    type_bits: 9,
                    modifier_bits: 0,
                    value: ArrayVariant {
                        count_flags: 0x08000000,
                        element_size: 0,
                        payload: VariantPayload::EmptyVariantVector,
                    },
                },
            });
        let original = state.clone();
        assert_eq!(
            cleanup_in_place(&mut state),
            Err(Error::UnsupportedPayloadDestruction(ObjectId(0)))
        );
        assert_eq!(state, original);
        let mut delivery = UndeliveredProjection::default();
        assert_eq!(
            activation_and_cleanup_in_place(&mut state, gates(), &mut delivery, 4),
            Err(Error::UnsupportedPayloadDestruction(ObjectId(0)))
        );
        assert_eq!(state, original);
        assert!(delivery.reports.is_empty());
        state.live[0].pool_binding = Some(PoolId(44));
        state.pools.push(Pool {
            id: PoolId(44),
            retained_objects: vec![],
            records: vec![],
        });
        cleanup_in_place(&mut state).unwrap();
        assert_eq!(
            state.pools[0].records[0].notification.parameters,
            original.notifications.records[0].parameters
        );
    }

    #[test]
    fn manager_cloud_gates_skip_activation_but_still_cleanup() {
        let mut s = fixture(2);
        s.notifications.records[0].is_dismissed = true;
        let mut scenarios = vec![];
        let mut g = gates();
        g.manager_initialized = false;
        scenarios.push(g);
        let mut g = gates();
        g.game_uninitializing = true;
        scenarios.push(g);
        let mut g = gates();
        g.human_configuration = false;
        scenarios.push(g);
        let mut g = gates();
        g.owner_turn_active = false;
        scenarios.push(g);
        let mut g = gates();
        g.play_by_cloud = true;
        scenarios.push(g);
        let mut g = gates();
        g.play_by_cloud = true;
        g.manager_game_local_player = Some(8);
        scenarios.push(g);
        for g in scenarios {
            let r = activation_and_cleanup(&s, g, &UnboundDelivery, 10).unwrap();
            assert_eq!(r.activations, 0);
            assert_eq!(r.state.notifications.records.len(), 1);
            assert!(!r.state.notifications.records[0].has_been_activated);
        }
        let mut g = gates();
        g.play_by_cloud = true;
        g.manager_game_local_player = Some(7);
        assert_eq!(
            activation_and_cleanup(&s, g, &UndeliveredProjection::default(), 10)
                .unwrap()
                .activations,
            2
        );
    }
    #[test]
    fn direct_base_activation_is_unconditional_and_preserves_actual_bool_argument() {
        let mut state = fixture(1);
        state.notifications.records[0].has_been_activated = true;
        let mut projection = UndeliveredProjection::default();
        activate_base_in_place(&mut state, 0, true, &mut projection).unwrap();
        activate_base_in_place(&mut state, 0, false, &mut projection).unwrap();
        assert_eq!(
            projection
                .reports
                .iter()
                .map(|r| r.activation_argument)
                .collect::<Vec<_>>(),
            [true, false]
        );
        let original = state.clone();
        assert_eq!(
            activate_base_in_place(&mut state, 4, false, &mut projection),
            Err(Error::MissingRecord(4))
        );
        assert_eq!(state, original);
        state.notifications.records[0].has_been_activated = false;
        state.live[0].definition_icon = DefinitionIcon::Unobserved;
        let original = state.clone();
        assert_eq!(
            activation_and_cleanup_in_place(&mut state, gates(), &mut projection, 3),
            Err(Error::UnobservedDefinitionInput(ObjectId(0)))
        );
        assert_eq!(state, original);
        state.notifications.records[0].has_been_activated = true;
        assert_eq!(find_auto_activate(&state, 0, false), Ok(None));
    }

    #[derive(Clone, Default)]
    struct Subscriber {
        seen: Vec<ComponentId>,
    }
    impl ActivationDelivery for Subscriber {
        fn preflight(&self, _: &State) -> Result<(), Error> {
            Ok(())
        }
        fn send(&mut self, s: &mut State, r: ActivationReport) {
            assert_eq!(
                (
                    r.event_hash,
                    r.identity_key,
                    r.bool_key,
                    r.activation_argument
                ),
                (ACTIVATION_EVENT, IDENTITY_KEY, ACTIVATION_BOOL_KEY, false)
            );
            assert!(
                s.notifications
                    .records
                    .iter()
                    .find(|n| n.id == r.identity)
                    .unwrap()
                    .has_been_activated
            );
            self.seen.push(r.identity);
            s.notifications.records[1].native_19d = true;
        }
    }
    #[test]
    fn synchronous_subscriber_runs_before_rescan_and_isolated_adapter_matches_local() {
        let mut s = fixture(2);
        s.notifications.records[1].native_19d = false;
        let original = s.clone();
        let subscriber = Subscriber::default();
        let isolated = activation_and_cleanup(&s, gates(), &subscriber, 10).unwrap();
        assert_eq!(isolated.activations, 2);
        assert_eq!(isolated.delivery.seen.len(), 2);
        assert_eq!(s, original);
        let mut delivery = subscriber.clone();
        let local = activation_and_cleanup_in_place(&mut s, gates(), &mut delivery, 10).unwrap();
        assert_eq!(s, isolated.state);
        assert_eq!(local.activations, isolated.activations);
        assert_eq!(delivery.seen, isolated.delivery.seen);
    }
}
