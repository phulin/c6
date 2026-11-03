//! In-place source-ordered Linux XP2 notification lifecycle leaves.
//! Required synchronous reporting executes against the authoritative LoadedState.
//! These source contracts are not a native comparison or complete action support.
use super::Created;
use c6_core::{
    loaded_state::{Error as StateError, LoadedState},
    notification_lifecycle::{ActivationBinding, ReleaseBinding},
    notifications::{ComponentId, Notification},
    player_notifications_owner::{Error as OwnerError, ObjectId, OccurrenceLimits, Runtime},
};

#[derive(Debug)]
pub enum Error {
    State(StateError),
    Owner(OwnerError),
    MissingRuntime(ObjectId),
    UnknownVtable(ObjectId),
    UnsupportedRelease(ObjectId),
    UnsupportedActivation(ObjectId),
    Backend(&'static str),
    MissingCandidate,
    AfterAppend { object: ObjectId, cause: Box<Error> },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loaded notification lifecycle: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<StateError> for Error {
    fn from(error: StateError) -> Self {
        Self::State(error)
    }
}
impl From<OwnerError> for Error {
    fn from(error: OwnerError) -> Self {
        Self::Owner(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    /// 025f6cfd: eab69e5a identity, 78a2f946 actual activation argument.
    Activated {
        identity: ComponentId,
        argument: bool,
    },
    /// 9ad1c020: actual ID captured after dismissed=true, including repeat calls.
    Dismissed(ComponentId),
    /// f26f3acf: ID captured after virtual OnAdd, retained across reporting.
    Added(ComponentId),
}
impl Report {
    pub fn event_hash(self) -> u32 {
        match self {
            Self::Activated { .. } => 0x025f6cfd,
            Self::Dismissed(_) => 0x9ad1c020,
            Self::Added(_) => 0xf26f3acf,
        }
    }
    pub fn identity(self) -> ComponentId {
        match self {
            Self::Activated { identity, .. } => identity,
            Self::Dismissed(identity) | Self::Added(identity) => identity,
        }
    }
}
/// The bound global reporting/subtype environment. There is deliberately no
/// successful default or fake Unit reporting context. Errors retain all earlier
/// state writes and backend effects. Overrides must execute their actual body or
/// reject precisely, including unknown payload/subtype destruction paths.
pub trait Backend {
    fn current_turn(&mut self, state: &mut LoadedState) -> Result<i32, Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        object: ObjectId,
        report: Report,
    ) -> Result<(), Error>;
    fn overridden_activate(
        &mut self,
        state: &mut LoadedState,
        object: ObjectId,
        argument: bool,
    ) -> Result<(), Error>;
    fn overridden_on_add(&mut self, state: &mut LoadedState, object: ObjectId)
        -> Result<(), Error>;
    fn overridden_supersede(
        &mut self,
        state: &mut LoadedState,
        receiver: ObjectId,
        candidate: ObjectId,
    ) -> Result<(), Error>;
    /// Actual deleting destructor, including parameter payload destruction.
    /// This must not erase the owner node: Cleanup does that after return.
    fn deleting_destructor(
        &mut self,
        state: &mut LoadedState,
        object: ObjectId,
    ) -> Result<(), Error>;
}
fn record(state: &LoadedState, object: ObjectId) -> Result<&Notification, Error> {
    Ok(state.player_notifications()?.record_by_object(object)?)
}
fn record_mut(state: &mut LoadedState, object: ObjectId) -> Result<&mut Notification, Error> {
    let owner = state.player_notifications()?;
    let ordinal = owner.ordinal_of(object)?;
    let id = owner.record_by_object(object)?.id;
    Ok(state.notification_record_mut(object.slot(), ordinal, id)?)
}
fn runtime(state: &LoadedState, object: ObjectId) -> Result<&Runtime, Error> {
    Ok(state.player_notifications()?.runtime(object)?)
}
// Addresspoints, not vtable starts. The companion static audit checks all slots.
fn known_vtable(state: &LoadedState, object: ObjectId) -> Result<u64, Error> {
    let vtable = runtime(state, object)?
        .vtable_addresspoint
        .ok_or(Error::UnknownVtable(object))?;
    if VTABLES.contains(&vtable) {
        Ok(vtable)
    } else {
        Err(Error::UnknownVtable(object))
    }
}

/// Base Activate writes before sending, even if already activated.
pub fn activate(
    state: &mut LoadedState,
    object: ObjectId,
    argument: bool,
    backend: &mut impl Backend,
) -> Result<(), Error> {
    let binding = runtime(state, object)?
        .live
        .as_ref()
        .ok_or(Error::MissingRuntime(object))?
        .activation;
    if binding != ActivationBinding::Base {
        return backend.overridden_activate(state, object, argument);
    }
    if !BASE_ACTIVATION_VTABLES.contains(&known_vtable(state, object)?) {
        return Err(Error::UnsupportedActivation(object));
    }
    record_mut(state, object)?.has_been_activated = true;
    let identity = record(state, object)?.id;
    backend.report(state, object, Report::Activated { identity, argument })
}
/// All 39 pinned vtables use Base Dismiss; equality does not suppress its event.
pub fn dismiss(
    state: &mut LoadedState,
    object: ObjectId,
    backend: &mut impl Backend,
) -> Result<(), Error> {
    known_vtable(state, object)?;
    record_mut(state, object)?.is_dismissed = true;
    let identity = record(state, object)?.id;
    backend.report(state, object, Report::Dismissed(identity))
}

/// Add an initialized candidate. Before append, `candidate` retains ordered turn
/// and ID writes on failure. Successful append consumes the candidate; all later
/// errors identify its stable attached occurrence in AfterAppend.
/// Captured return ID survives subsequent callback changes to the actual ID.
pub fn add(
    state: &mut LoadedState,
    slot: i32,
    candidate: &mut Option<Created>,
    limits: OccurrenceLimits,
    backend: &mut impl Backend,
) -> Result<ComponentId, Error> {
    let supplied = candidate.as_mut().ok_or(Error::MissingCandidate)?;
    if supplied.record.native_188 == -1 {
        supplied.record.native_188 = backend.current_turn(state)?;
    }
    let collection = state.player_notifications()?.collection(slot)?;
    let owner = collection.owner;
    let next_id = collection.next_id;
    state.set_notification_next_id(slot, next_id.wrapping_add(1))?;
    // SetID truncates stored owner to i16 and preserves component_type.
    supplied.record.id.owner = owner as i16;
    supplied.record.id.id = next_id;
    let object = state.append_initialized_notification(
        slot,
        supplied.record.clone(),
        Runtime {
            live: Some(supplied.live.clone()),
            default_blocking: Some(supplied.default_blocking),
            group_hash: Some(supplied.group_hash),
            vtable_addresspoint: Some(supplied.vtable_addresspoint),
        },
        limits,
    )?;
    *candidate = None;
    let mut continuation = || -> Result<ComponentId, Error> {
        // DiplomacySession overrides OnAdd. Other known target bindings are RET.
        if !matches!(known_vtable(state, object), Ok(vtable) if vtable != DIPLOMACY_VTABLE) {
            backend.overridden_on_add(state, object)?;
        }
        let captured = record(state, object)?.id;
        backend.report(state, object, Report::Added(captured))?;
        expire_all_superseded_by(state, slot, object, backend)?;
        Ok(captured)
    };
    continuation().map_err(|cause| Error::AfterAppend {
        object,
        cause: Box::new(cause),
    })
}

/// Retains the actual collection and candidate, scans current order, skips only
/// the same logical object, and advances after each virtual call.
pub fn expire_all_superseded_by(
    state: &mut LoadedState,
    slot: i32,
    candidate: ObjectId,
    backend: &mut impl Backend,
) -> Result<(), Error> {
    let logical = runtime(state, candidate)?
        .live
        .as_ref()
        .ok_or(Error::MissingRuntime(candidate))?
        .object;
    let mut cursor = state.player_notifications()?.first(slot)?;
    while let Some(receiver) = cursor {
        let receiver_logical = runtime(state, receiver)?
            .live
            .as_ref()
            .ok_or(Error::MissingRuntime(receiver))?
            .object;
        if receiver_logical != logical {
            // DiplomacySession alone overrides +60; unknown bindings reject via
            // required backend rather than inheriting Base by assumption.
            match known_vtable(state, receiver) {
                Ok(vtable) if vtable != DIPLOMACY_VTABLE => {}
                _ => backend.overridden_supersede(state, receiver, candidate)?,
            }
        }
        cursor = state.player_notifications()?.next(receiver)?;
    }
    Ok(())
}

/// Release retains the exact logical object and all fields. Pool append precedes
/// clearing parent; no-pool dispatches the required actual deleting destructor.
/// The list node remains linked until Cleanup erases it after successful return.
pub fn release(
    state: &mut LoadedState,
    object: ObjectId,
    backend: &mut impl Backend,
) -> Result<(), Error> {
    known_vtable(state, object)?;
    let live = runtime(state, object)?
        .live
        .as_ref()
        .ok_or(Error::MissingRuntime(object))?;
    if live.release != ReleaseBinding::Base {
        return Err(Error::UnsupportedRelease(object));
    }
    if let Some(pool) = live.pool_binding {
        state.pool_notification(object, pool)?;
        Ok(())
    } else {
        backend.deleting_destructor(state, object)
    }
}

/// Source Cleanup visits current list order and calls Release before erasing each
/// dismissed node. A release error retains that node and all earlier effects.
pub fn cleanup(
    state: &mut LoadedState,
    slot: i32,
    backend: &mut impl Backend,
) -> Result<(), Error> {
    let mut cursor = state.player_notifications()?.first(slot)?;
    while let Some(object) = cursor {
        if record(state, object)?.is_dismissed {
            release(state, object, backend)?;
            cursor = state.erase_released_notification(object)?;
        } else {
            cursor = state.player_notifications()?.next(object)?;
        }
    }
    Ok(())
}

const VTABLES: &[u64] = &[
    0x3af4840, 0x3af5060, 0x3af4c50, 0x3af6600, 0x3af4ab0, 0x3af53a0, 0x3af5c90, 0x3af4ec0,
    0x3af4f90, 0x3af6460, 0x3af67a0, 0x3af5130, 0x3af5200, 0x3af5610, 0x3af5a20, 0x3af52d0,
    0x3af56e0, 0x3af5950, 0x3af5af0, 0x3af66d0, 0x3af4df0, 0x3af5540, 0x3af5e30, 0x3af5bc0,
    0x3af5f00, 0x3af60a0, 0x3af6530, 0x3af4770, 0x3af49e0, 0x3af4b80, 0x3af57b0, 0x3af4d20,
    0x3af5470, 0x3af5880, 0x3af5fd0, 0x3af5d60, 0x3af6170, 0x3af6238, 0x3af4910,
];
const DIPLOMACY_VTABLE: u64 = 0x3af5af0;

const BASE_ACTIVATION_VTABLES: &[u64] = &[
    0x3af4840, 0x3af5060, 0x3af4c50, 0x3af6600, 0x3af4ab0, 0x3af53a0, 0x3af5c90, 0x3af4ec0,
    0x3af4f90, 0x3af67a0, 0x3af5130, 0x3af5200, 0x3af5610, 0x3af5a20, 0x3af52d0, 0x3af56e0,
    0x3af5950, 0x3af66d0, 0x3af4df0, 0x3af5540, 0x3af5e30, 0x3af5bc0, 0x3af5f00, 0x3af60a0,
    0x3af6530, 0x3af4770, 0x3af49e0, 0x3af4b80, 0x3af57b0, 0x3af4d20, 0x3af5470, 0x3af5880,
    0x3af5fd0, 0x3af5d60, 0x3af6170, 0x3af6238, 0x3af4910,
];
