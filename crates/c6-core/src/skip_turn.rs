//! Exact Linux XP2 leaf phases used by SkipTurn, not an accepted action.
//!
//! The caller must implement archive registration and synchronous reporting
//! subscribers through the required hooks. No no-op subscriber implementation is
//! supplied. Manager legality, queue replacement/activation, diplomacy and
//! notification aftermath remain outside this boundary. `Engine::apply` does not
//! route gameplay actions here. See docs/linux-skip-turn-kernel.md.
use crate::unit::{ComponentId, Formation, PlotCoord, Unit, SKIP_TURN_HASH};

pub const HOLD: i32 = 0x047c048b;
pub const ACTIVITY_DIRTY_INDEX: u32 = 11;
pub const FORMATION_PATH_DIRTY_INDEX: u32 = 14;
pub const FORMATION_CACHE_DIRTY_INDEX: u32 = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StartData {
    pub delete: bool,
    pub action: bool,
}

/// The exact Type/ComponentID/int overload; original Send returns int.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityIntReport {
    pub event_hash: u32,
    pub subtype_hash: u32,
    pub identity: ComponentId,
    pub value: i32,
}

/// Required live effects, absent from Unit's wire record.
///
/// `send` runs synchronously at the original call boundary. Subscribers may edit
/// Unit state; subsequent steps read that updated state. These two original
/// callers ignore Send's int result. Implementations must execute all required
/// subscribers before returning; collecting reports alone is a diagnostic use.
pub trait StartEffects {
    /// Register this wrapper in its actual archive's unique dirty-pointer set.
    /// Called before assignment, only when the wrapper has an archive binding.
    fn mark_activity_dirty(&mut self, unit: &Unit, index: u32);
    fn send(&mut self, unit: &mut Unit, report: IdentityIntReport) -> i32;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    IdentityMismatch,
    UnsupportedRequest,
    UnsupportedQueue,
    UnsupportedAutomation,
    UnsupportedInitialActivity,
    FormationMismatch,
    UnsupportedActivation,
    MovementCompletionPending,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

fn check_identity(unit: &Unit, identity: ComponentId) -> Result<(), Error> {
    if i32::from(identity.owner) != unit.e_owner || identity.id != unit.id {
        return Err(Error::IdentityMismatch);
    }
    Ok(())
}

/// Manager::SetActivityType (0x2da1840), including the equality fast path.
///
/// `identity` is the actual live ComponentID; its component-type bits are not
/// inferred from wire IDs. `archive_bound` is the actual activity wrapper's live
/// archive attachment. A mismatching identity rejects before any effect.
pub fn set_activity(
    unit: &mut Unit,
    identity: ComponentId,
    activity: i32,
    archive_bound: bool,
    effects: &mut impl StartEffects,
) -> Result<(), Error> {
    check_identity(unit, identity)?;
    set_activity_checked(unit, identity, activity, archive_bound, effects);
    Ok(())
}

fn set_activity_checked(
    unit: &mut Unit,
    identity: ComponentId,
    activity: i32,
    archive_bound: bool,
    effects: &mut impl StartEffects,
) {
    if unit.e_activity == activity {
        return;
    }
    if archive_bound {
        effects.mark_activity_dirty(unit, ACTIVITY_DIRTY_INDEX);
    }
    unit.e_activity = activity;
    let _ = effects.send(
        unit,
        IdentityIntReport {
            event_hash: 0xe23b112d,
            subtype_hash: 0,
            identity,
            value: activity,
        },
    );
}

/// SkipTurn::Start (0x2dac8e0) with the verified original manager implementation.
///
/// This is a handler phase, not CanStart/Add/Activate or queue deletion. It emits
/// the start report, calls SetActivityType(HOLD), then sets bDelete=true. bAction
/// retains its incoming value. Node contents are unused by this original leaf.
pub fn start(
    unit: &mut Unit,
    identity: ComponentId,
    data: &mut StartData,
    archive_bound: bool,
    effects: &mut impl StartEffects,
) -> Result<(), Error> {
    check_identity(unit, identity)?;
    let _ = effects.send(
        unit,
        IdentityIntReport {
            event_hash: 0x3d4006f2,
            subtype_hash: SKIP_TURN_HASH,
            identity,
            value: 0,
        },
    );
    // Reporting may have changed activity. Preserve the original later read.
    set_activity_checked(unit, identity, HOLD, archive_bound, effects);
    data.delete = true;
    Ok(())
}

/// Required Formation archive effect. Each wrapper's attachment is independent.
pub trait FormationEffects {
    fn mark_dirty(&mut self, formation: &Formation, index: u32);
}

/// Formation::ClearPathCache (0x293d8e0), separate from queue processing.
///
/// Both edits register even if values are already empty/invalid. Bindings are
/// live wrapper archive-presence inputs, not values recoverable from wire state.
pub fn clear_path_cache(
    formation: &mut Formation,
    path_archive_bound: bool,
    cache_archive_bound: bool,
    effects: &mut impl FormationEffects,
) {
    if path_archive_bound {
        effects.mark_dirty(formation, FORMATION_PATH_DIRTY_INDEX);
    }
    formation.last_path.clear();
    if cache_archive_bound {
        effects.mark_dirty(formation, FORMATION_CACHE_DIRTY_INDEX);
    }
    formation.last_path_cache_location = PlotCoord { x: -9999, y: -9999 };
}

/// Request metadata remains distinct from the serialized handler hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplacementInputs {
    pub identity: ComponentId,
    /// Actual numeric SkipTurn registry index in this configuration.
    pub skip_turn_operation_id: i32,
    pub current_turn: i32,
    pub local_player: i32,
    pub directive: i32,
    pub flags: i32,
    pub append: bool,
    pub from_user: bool,
    /// Actual IsBusy result at activation, not inferred from wire movement.
    pub busy: bool,
    /// Actual CanMove result; its complete implementation is outside this phase.
    pub can_move: bool,
    /// Full manager CanStart result for the inserted SkipTurn/current plot,
    /// false/null defaults, evaluated at its actual activation boundary.
    /// The supported phase requires true and excludes scripts/subscriber changes.
    pub activation_can_start: bool,
    pub activity_archive_bound: bool,
    pub timer_archive_bound: bool,
    pub queue_archive_bound: bool,
    pub path_archive_bound: bool,
    pub cache_archive_bound: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    /// Ordered registration requests, not native dirty-tree iteration order.
    UnitDirty(u32),
    FormationDirty(u32),
    /// Exact Type, ComponentID, and zero/one/two int overload inputs.
    Report {
        event_hash: u32,
        subtype_hash: u32,
        identity: ComponentId,
        first: Option<i32>,
        second: Option<i32>,
    },
}

/// Candidate state at Manager::Add return, plus ordered effects to deliver.
/// This does not execute a reporting provider or its consumers. The caller must
/// discharge those obligations before using this as live gameplay state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Replacement {
    pub unit: Unit,
    pub formation: Formation,
    pub effects: Vec<Effect>,
}
struct Collector {
    effects: Vec<Effect>,
}
impl Collector {
    fn report(
        &mut self,
        identity: ComponentId,
        event_hash: u32,
        subtype_hash: u32,
        first: Option<i32>,
        second: Option<i32>,
    ) {
        self.effects.push(Effect::Report {
            event_hash,
            subtype_hash,
            identity,
            first,
            second,
        });
    }
}
impl StartEffects for Collector {
    fn mark_activity_dirty(&mut self, _: &Unit, index: u32) {
        self.effects.push(Effect::UnitDirty(index));
    }
    fn send(&mut self, _: &mut Unit, report: IdentityIntReport) -> i32 {
        self.report(
            report.identity,
            report.event_hash,
            report.subtype_hash,
            Some(report.value),
            None,
        );
        0 // Diagnostic collection only; original callers discard this result.
    }
}
impl FormationEffects for Collector {
    fn mark_dirty(&mut self, _: &Formation, index: u32) {
        self.effects.push(Effect::FormationDirty(index));
    }
}
const OPERATION: i32 = 0x490cc8e1;
const AWAKE: i32 = 0x6b25012e;

fn deactivate(unit: &mut Unit, i: ReplacementInputs, e: &mut Collector) {
    if unit.k_operations.nodes.is_empty() {
        return;
    }
    if unit.e_activity == OPERATION {
        set_activity_checked(unit, i.identity, AWAKE, i.activity_archive_bound, e);
    }
    if i.timer_archive_bound {
        e.effects.push(Effect::UnitDirty(32));
    }
    // Native tracked assignment edits even when the old value is already zero.
    unit.i_operation_timer = 0;
}
fn clear_formation(f: &mut Formation, i: ReplacementInputs, e: &mut Collector) {
    clear_path_cache(f, i.path_archive_bound, i.cache_archive_bound, e);
}
fn pop_front(unit: &mut Unit, f: &mut Formation, i: ReplacementInputs, e: &mut Collector) {
    let hash = unit.k_operations.nodes[0].operation_hash;
    let ready = unit.e_owner == i.local_player
        && unit.e_automation == -1
        && i.can_move
        && unit.k_operations.nodes.len() <= 1
        && [OPERATION, AWAKE].contains(&unit.e_activity);
    e.report(
        i.identity,
        0xa60328c5,
        0,
        Some(hash as i32),
        Some(i32::from(ready)),
    );
    // MoveTo first calls CompleteMovement; its false-predicate branch was
    // required before any mutation. Both then use BaseHandler's false return.
    if i.queue_archive_bound {
        e.effects.push(Effect::UnitDirty(72));
    }
    deactivate(unit, i, e);
    unit.k_operations.nodes.remove(0);
    if unit.k_operations.nodes.is_empty() {
        clear_formation(f, i, e);
    }
    if unit.k_operations.nodes.is_empty() {
        e.report(i.identity, 0x383669f1, 0, None, None);
    }
}

/// Bounded original queue replacement/activation state phase, not a full action.
///
/// Supports the public empty-map replacement metadata (USE_REQUEST, append=false,
/// from_user=false, directive=-1, flags=0), an empty queue or one MoveTo node,
/// no pending movement completion, no automation, a matching singleton Formation,
/// and a legal nonbusy activation.
/// Actual omitted live predicate results and archive bindings are required inputs.
/// Unsupported inputs return before any output state/effects are constructed.
///
/// Reporting delivery is an explicit returned obligation. This phase assumes it
/// does not synchronously change core inputs; no such general provider contract
/// or complete live-state/action parity is claimed. Full legality, provider
/// delivery/subscribers, pending diplomacy and player/notification aftermath must
/// compose outside this function. Inputs are borrowed and never mutated.
pub fn replace_operation(
    unit: &Unit,
    formation: &Formation,
    i: ReplacementInputs,
) -> Result<Replacement, Error> {
    check_identity(unit, i.identity)?;
    if i.skip_turn_operation_id < 0 || i.directive != -1 || i.flags != 0 || i.append || i.from_user
    {
        return Err(Error::UnsupportedRequest);
    }
    // MoveTo::Deactivate calls CompleteMovement first. Its false predicate
    // returns without effects; true requires the unmodeled IdentityPlotCoord path.
    if !unit.k_operations.nodes.is_empty() && unit.b_needs_movement_completion {
        return Err(Error::MovementCompletionPending);
    }
    if unit.e_automation != -1 {
        return Err(Error::UnsupportedAutomation);
    }
    if formation.id != unit.formation_id
        || formation.owner != unit.e_owner
        || formation.unit_ids != [unit.id]
    {
        return Err(Error::FormationMismatch);
    }
    if unit.k_operations.nodes.len() > 1
        || unit.k_operations.nodes.first().is_some_and(|n| {
            n.operation_hash != crate::unit::MOVE_TO_HASH
                || !matches!(
                    n.parameters,
                    crate::unit::OperationParameters::MoveTo { .. }
                )
        })
    {
        return Err(Error::UnsupportedQueue);
    }
    if ![OPERATION, AWAKE].contains(&unit.e_activity)
        || (unit.k_operations.nodes.is_empty() && unit.e_activity == OPERATION)
    {
        return Err(Error::UnsupportedInitialActivity);
    }
    if i.busy || !i.activation_can_start {
        return Err(Error::UnsupportedActivation);
    }
    let mut unit = unit.clone();
    let mut formation = formation.clone();
    let mut e = Collector {
        effects: Vec::new(),
    };
    // Manager::Clear does nothing when the original queue is empty.
    if !unit.k_operations.nodes.is_empty() {
        deactivate(&mut unit, i, &mut e);
        pop_front(&mut unit, &mut formation, i, &mut e);
        clear_formation(&mut formation, i, &mut e);
    }
    let node = crate::unit::OperationNode {
        operation_hash: SKIP_TURN_HASH,
        directive: i.directive,
        flags: i.flags,
        push_turn: i.current_turn,
        previous_activity: unit.e_activity,
        previous_automation: unit.e_automation,
        parameters: crate::unit::OperationParameters::None,
    };
    if i.queue_archive_bound {
        e.effects.push(Effect::UnitDirty(72));
    }
    unit.k_operations.nodes.push(node);
    e.report(
        i.identity,
        0x15c35863,
        0,
        Some(i.skip_turn_operation_id),
        None,
    );
    set_activity_checked(
        &mut unit,
        i.identity,
        if i.can_move { OPERATION } else { HOLD },
        i.activity_archive_bound,
        &mut e,
    );
    let mut data = StartData {
        delete: false,
        action: false,
    };
    start(
        &mut unit,
        i.identity,
        &mut data,
        i.activity_archive_bound,
        &mut e,
    )?;
    if data.delete {
        pop_front(&mut unit, &mut formation, i, &mut e);
    }
    Ok(Replacement {
        unit,
        formation,
        effects: e.effects,
    })
}
