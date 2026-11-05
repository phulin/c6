//! Owned notification Update composition for a conditional stock XP2 profile.
//!
//! The core owns ordering; this adapter supplies actual typed records and subtype
//! queries. Runtime services remain required, synchronous and effectful. A failed
//! call retains attached writes, backend effects and any pending allocation.
use super::{lifecycle, Created, Definitions, LogicalAllocation};
use crate::loaded_actions::{
    choose_tech, city_range_attack, command_units, configuration, culture_notifications as culture,
    reporting,
};
use c6_core::{
    loaded_state::LoadedState,
    notification_lifecycle as live,
    notifications::Notification,
    player_notifications_owner::{ObjectId, OccurrenceLimits},
    unit_operation_aftermath::{self as aftermath, Blocking, Notifications},
};

#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Configuration(crate::loaded_actions::configuration::Error),
    Owner(c6_core::player_notifications_owner::Error),
    Definitions(super::Error),
    Lifecycle(lifecycle::Error),
    Tech(choose_tech::Error),
    Culture(culture::Error),
    Gameplay(crate::loaded_actions::Error),
    Unsupported(&'static str),
    Orchestration(aftermath::Error),
    Text(super::text::Error<std::convert::Infallible>),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "owned notification Update: {self:?}")
    }
}
impl std::error::Error for Error {}
macro_rules! conversions { ($($variant:ident: $ty:ty),* $(,)?) => {$(
    impl From<$ty> for Error { fn from(e: $ty) -> Self { Self::$variant(e) } }
)*}; }
conversions! { Configuration:crate::loaded_actions::configuration::Error, State:c6_core::loaded_state::Error, Owner:c6_core::player_notifications_owner::Error,
Definitions:super::Error, Lifecycle:lifecycle::Error, Tech:choose_tech::Error,
Culture:culture::Error, Gameplay:crate::loaded_actions::Error }

/// Allocation transfers logical ownership to this provider. Native addresses,
/// vector capacities and allocator layout are not inputs. `prior` must be the
/// actual reused object removed from the authoritative logical pool.
pub struct Allocation {
    pub object: live::ObjectId,
    pub pool: live::PoolId,
    pub prior: Option<c6_core::player_notifications_owner::PooledNotification>,
}
/// Each method is an actual runtime query/effect, not an observed answer replay.
/// There are no successful defaults. Unknown script consumers, text effects,
/// allocator failure and pool history must be reported by the implementation.
pub trait Backend: lifecycle::Backend {
    /// ChoosePantheon::IsExpired2d29f70 repeats current human/religion queries.
    fn pantheon_needed(&self, _state: &LoadedState, _owner: i32) -> Result<bool, Error> {
        Err(Error::Unsupported("current pantheon eligibility binding"))
    }
    fn manager_initialized(&mut self, state: &mut LoadedState) -> Result<bool, Error>;
    fn game_uninitializing(&mut self, state: &mut LoadedState) -> Result<bool, Error>;
    fn allocate(&mut self, state: &mut LoadedState, type_hash: u32) -> Result<Allocation, Error>;
    /// Validation at the first positive IsNeeded identity consumer. Exact
    /// allocators validate their retained/fresh target identity here. A
    /// presentation projection may omit pool history on the redundant branch,
    /// but must reject here if a retained target type could change IsDuplicate.
    /// This is an invariant check, not an additional native gameplay query.
    fn validate_duplicate_identity(
        &mut self,
        state: &mut LoadedState,
        candidate: &Created,
        existing_target: c6_core::notifications::ComponentId,
    ) -> Result<(), Error>;
    fn release_pending(
        &mut self,
        state: &mut LoadedState,
        candidate: &mut Option<Created>,
    ) -> Result<(), Error>;
    fn initialize_text(
        &mut self,
        state: &mut LoadedState,
        candidate: &mut Created,
    ) -> Result<(), Error>;
    fn manager_game_local_player(&mut self, state: &mut LoadedState) -> Result<Option<i32>, Error>;
    fn blocking_changed(
        &mut self,
        state: &mut LoadedState,
        event: u32,
        old: i32,
        new: i32,
    ) -> Result<(), Error>;
}
/// These are admitted immutable definitions/source projections, never scalar
/// precomputed IsNeeded answers. They are built once before the first action.
#[derive(Clone, Copy)]
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub configuration: &'a configuration::Configuration<'a>,
    pub tech: &'a choose_tech::Definitions,
    pub tech_graph: Option<&'a choose_tech::GameTechs>,
    pub culture: &'a culture::Definitions,
    pub culture_graph: Option<&'a culture::GameCulture>,
    pub starting: Option<&'a culture::StartingContext>,
    pub readiness: &'a reporting::Provider<'a>,
    pub cities: &'a city_range_attack::Queries<'a>,
    pub text_scope: Option<super::text::Scope>,
}
/// Conditional source registration authority, not a captured native registry.
/// Added/modded creators and later load/runtime overrides remain unsupported.
pub enum Scope {
    SourceInitializedStockXp2,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Object {
    Pending(usize),
    Attached(ObjectId),
}
struct Pending {
    allocation: Allocation,
    created: Option<Created>,
}
pub struct Provider<'a, B> {
    pub state: &'a mut LoadedState,
    pub backend: B,
    inputs: Inputs<'a>,
    limits: OccurrenceLimits,
    pending: Vec<Pending>,
    failure: Option<Error>,
}
impl<'a, B: Backend> Provider<'a, B> {
    pub fn new(
        state: &'a mut LoadedState,
        inputs: Inputs<'a>,
        backend: B,
        limits: OccurrenceLimits,
        _: Scope,
    ) -> Self {
        Self {
            state,
            backend,
            inputs,
            limits,
            pending: Vec::new(),
            failure: None,
        }
    }
    pub fn failure(&self) -> Option<&Error> {
        self.failure.as_ref()
    }
    /// Detached, partially initialized allocations stay owned after failure.
    pub fn pending_candidate(&self, index: usize) -> Option<&Created> {
        self.pending.get(index).and_then(|p| p.created.as_ref())
    }
    pub fn pending_allocation(&self, index: usize) -> Option<&Allocation> {
        self.pending.get(index).map(|p| &p.allocation)
    }
    pub fn update(&mut self, owner: i32, budget: &mut usize) -> Result<(), aftermath::Error> {
        if self.failure.is_some() {
            return Err(aftermath::Error::Unsupported(
                "failed diagnostic provider cannot be retried as unchanged",
            ));
        }
        let result = aftermath::update_owner_blocking(self, owner, budget);
        if let Err(error) = &result {
            if self.failure.is_none() {
                self.failure = Some(Error::Orchestration(error.clone()));
            }
        }
        result
    }
    /// HasEndTurnBlocking scans the current owner list in order and executes
    /// each existing effectful blocking getter, stopping at the first match.
    pub fn has_end_turn_blocking(
        &mut self,
        owner: i32,
        kind: u32,
    ) -> Result<bool, aftermath::Error> {
        if owner == -1 {
            return Ok(false);
        }
        let mut current = self.step(|s| Ok(s.state.player_notifications()?.first(owner)?))?;
        while let Some(object) = current {
            if Notifications::blocking(self, Object::Attached(object))? as u32 == kind {
                return Ok(true);
            }
            current = self.step(|s| Ok(s.state.player_notifications()?.next(object)?))?;
        }
        Ok(false)
    }
    fn step<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, aftermath::Error> {
        if self.failure.is_some() {
            return Err(aftermath::Error::Unsupported(
                "failed diagnostic provider cannot execute another effect",
            ));
        }
        f(self).map_err(|e| {
            self.failure = Some(e);
            aftermath::Error::Unsupported(
                "owned notification provider failure; inspect failure() and retained state",
            )
        })
    }
    fn record(&self, object: Object) -> Result<&Notification, Error> {
        match object {
            Object::Attached(o) => Ok(self.state.player_notifications()?.record_by_object(o)?),
            Object::Pending(i) => self
                .pending
                .get(i)
                .and_then(|p| p.created.as_ref())
                .map(|p| &p.record)
                .ok_or(Error::Unsupported(
                    "pending notification no longer owns an initialized record",
                )),
        }
    }
    fn attached(object: Object) -> Result<ObjectId, Error> {
        match object {
            Object::Attached(o) => Ok(o),
            _ => Err(Error::Unsupported("attached notification required")),
        }
    }
    fn expiry(&self, object: Object, needed: bool) -> Result<bool, Error> {
        let record = self.record(object)?;
        let vtable = match object {
            Object::Attached(o) => {
                self.state
                    .player_notifications()?
                    .runtime(o)?
                    .vtable_addresspoint
            }
            Object::Pending(i) => self
                .pending
                .get(i)
                .and_then(|p| p.created.as_ref())
                .map(|p| p.vtable_addresspoint),
        }
        .ok_or(Error::Unsupported("notification expiry virtual binding"))?;
        let expected = match record.type_hash {
            choose_tech::TYPE_HASH => Some(choose_tech::VTABLE_ADDRESSPOINT),
            0x40fb81ed => Some(0x3af4b80),
            0x9456ed4a => Some(0x3af4ec0),
            command_units::TYPE_HASH => Some(command_units::VTABLE_ADDRESSPOINT),
            city_range_attack::TYPE_HASH => Some(city_range_attack::VTABLE_ADDRESSPOINT),
            hash => culture::Kind::from_hash(hash).map(|k| k.vtable()),
        };
        let base = matches!(
            vtable,
            super::initialization::BASE_VTABLE | super::initialization::PRIDE_VTABLE
        );
        if !base && expected != Some(vtable) {
            return Err(Error::Unsupported(
                "actual notification expiry virtual binding",
            ));
        }
        if needed && !record.native_19a {
            return Ok(true);
        }
        if base {
            let expired = aftermath::base_expired(
                record.native_188,
                record.native_18c,
                self.state.retained().core.current_turn,
            );
            return Ok(if needed { !expired } else { expired });
        }
        let owner = i32::from(record.id.owner);
        let survives = match record.type_hash {
            0x9456ed4a => self.backend.pantheon_needed(self.state, owner)?,
            // ChooseCityProduction::IsExpired2d29800. This only queries the
            // current City; the external policy retains its production choice.
            0x40fb81ed => self
                .state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .and_then(|p| {
                    p.cities
                        .records
                        .entries
                        .iter()
                        .find(|c| c.value.header.id == record.component.id)
                })
                .is_some_and(|c| {
                    !c.value.header.delayed_death && c.value.build_queue.entries.is_empty()
                }),
            choose_tech::TYPE_HASH => self
                .inputs
                .tech
                .player(self.state, owner, self.inputs.tech_graph)?
                .is_needed(true)?,
            command_units::TYPE_HASH => {
                !command_units::is_expired(self.state, record, self.inputs.readiness)?
            }
            city_range_attack::TYPE_HASH => {
                !city_range_attack::is_expired(self.state, self.inputs.cities, record)?
            }
            hash => {
                if let Some(kind) = culture::Kind::from_hash(hash) {
                    let research =
                        self.inputs
                            .tech
                            .player(self.state, owner, self.inputs.tech_graph)?;
                    !self
                        .inputs
                        .culture
                        .player(
                            self.state,
                            owner,
                            self.inputs.culture_graph,
                            Some(&research),
                            self.inputs.starting,
                        )?
                        .is_expired(kind)?
                } else {
                    return Err(Error::Unsupported(
                        "actual notification subtype expiry binding",
                    ));
                }
            }
        };
        Ok(if needed { survives } else { !survives })
    }
    fn create(&mut self, hash: u32, owner: i32) -> Result<Option<Object>, Error> {
        if !self.backend.manager_initialized(self.state)?
            || self.backend.game_uninitializing(self.state)?
        {
            return Ok(None);
        }
        let vtable = match hash {
            choose_tech::TYPE_HASH => choose_tech::VTABLE_ADDRESSPOINT,
            command_units::TYPE_HASH => command_units::VTABLE_ADDRESSPOINT,
            city_range_attack::TYPE_HASH => city_range_attack::VTABLE_ADDRESSPOINT,
            _ => culture::Kind::from_hash(hash)
                .ok_or(Error::Unsupported("stock auto creator binding"))?
                .vtable(),
        };
        let allocation = self.backend.allocate(self.state, hash)?;
        let index = self.pending.len();
        self.pending.push(Pending {
            allocation,
            created: None,
        });
        let allocation = &self.pending[index].allocation;
        let logical = match &allocation.prior {
            Some(prior) => LogicalAllocation::Reused {
                object: allocation.object,
                prior: &prior.record,
            },
            None => LogicalAllocation::Fresh(allocation.object),
        };
        let created = super::initialize_auto_base(
            self.inputs.definitions,
            hash,
            vtable,
            logical,
            allocation.pool,
            owner,
        )?;
        self.pending[index].created = Some(created);
        let candidate = self.pending[index]
            .created
            .as_mut()
            .expect("just installed");
        match hash {
            choose_tech::TYPE_HASH => {
                candidate.record.native_199 = false;
                candidate.record.native_19a = true;
                candidate.record.native_18c = 0;
                candidate.default_blocking = choose_tech::DEFAULT_BLOCKING;
            }
            command_units::TYPE_HASH => {
                command_units::initialize_auto(self.state, self.inputs.readiness, candidate)?
            }
            city_range_attack::TYPE_HASH => {
                city_range_attack::initialize_auto(self.state, self.inputs.readiness, candidate)?
            }
            _ => culture::initialize_in_place(
                culture::Kind::from_hash(hash).expect("validated"),
                candidate,
            )?,
        }
        Ok(Some(Object::Pending(index)))
    }
}
impl<B: Backend> Notifications for Provider<'_, B> {
    type Object = Object;
    type Cursor = Option<ObjectId>;
    type Manager = ();
    type Configuration = ();
    type Collection = i32;
    type Definitions = ();
    fn configuration(&mut self) -> Result<(), aftermath::Error> {
        Ok(())
    }
    fn definitions(&mut self) -> Result<(), aftermath::Error> {
        Ok(())
    }
    fn collection(&mut self, _: (), owner: i32, _: bool) -> Result<i32, aftermath::Error> {
        self.step(|s| {
            s.state.player_notifications()?.collection(owner)?;
            Ok(owner)
        })
    }
    fn accepts_notifications(&mut self, _: (), owner: i32) -> Result<bool, aftermath::Error> {
        self.step(|s| {
            if !s.backend.manager_initialized(s.state)? || s.backend.game_uninitializing(s.state)? {
                return Ok(false);
            }
            Ok(s.state.player_slot_status(owner)? == 3)
        })
    }
    fn definition_count(&mut self, _: ()) -> Result<usize, aftermath::Error> {
        Ok(self.inputs.definitions.ordered().len())
    }
    fn definition_is_auto(&mut self, _: (), i: usize) -> Result<bool, aftermath::Error> {
        self.step(|s| {
            s.inputs
                .definitions
                .ordered()
                .get(i)
                .map(|d| d.auto_notify)
                .ok_or(Error::Unsupported("definition index"))
        })
    }
    fn definition_hash(&mut self, _: (), i: usize) -> Result<u32, aftermath::Error> {
        self.step(|s| {
            s.inputs
                .definitions
                .ordered()
                .get(i)
                .map(|d| d.type_hash)
                .ok_or(Error::Unsupported("definition index"))
        })
    }
    fn create_auto(
        &mut self,
        _: (),
        hash: u32,
        owner: i32,
    ) -> Result<Option<Object>, aftermath::Error> {
        self.step(|s| s.create(hash, owner))
    }
    fn redundant(&mut self, slot: i32, object: Object) -> Result<bool, aftermath::Error> {
        self.step(|s| {
            if !s.expiry(object, true)? {
                return Ok(true);
            }
            let candidate_prefix = {
                let record = s.record(object)?;
                (record.type_hash, record.location)
            };
            let mut cursor = s.state.player_notifications()?.first(slot)?;
            while let Some(current) = cursor {
                let existing = s.state.player_notifications()?.record_by_object(current)?;
                if !existing.is_dismissed {
                    // Base IsDuplicate returns before reading target identity
                    // when type/location differ. Preserve that lazy boundary.
                    if candidate_prefix.0 == existing.type_hash
                        && candidate_prefix.1 == existing.location
                    {
                        let existing_target = existing.component;
                        if let Object::Pending(i) = object {
                            let created =
                                s.pending.get(i).and_then(|p| p.created.as_ref()).ok_or(
                                    Error::Unsupported("positive IsNeeded pending candidate"),
                                )?;
                            s.backend.validate_duplicate_identity(
                                s.state,
                                created,
                                existing_target,
                            )?;
                        }
                    }
                    let candidate = s.record(object)?;
                    let existing = s.state.player_notifications()?.record_by_object(current)?;
                    let duplicate = match candidate.type_hash {
                        command_units::TYPE_HASH => {
                            command_units::is_duplicate(candidate, existing)?
                        }
                        city_range_attack::TYPE_HASH => {
                            city_range_attack::is_duplicate(candidate, existing)?
                        }
                        _ => super::base_auto_duplicate(candidate, existing)?,
                    };
                    if duplicate {
                        return Ok(true);
                    }
                }
                cursor = s.state.player_notifications()?.next(current)?;
            }
            Ok(false)
        })
    }
    fn release(&mut self, object: Object) -> Result<(), aftermath::Error> {
        self.step(|s| match object {
            Object::Pending(i) => {
                let p = s
                    .pending
                    .get_mut(i)
                    .ok_or(Error::Unsupported("pending allocation index"))?;
                s.backend.release_pending(s.state, &mut p.created)
            }
            Object::Attached(o) => {
                lifecycle::release(s.state, o, &mut s.backend)?;
                Ok(())
            }
        })
    }
    fn send(&mut self, object: Object) -> Result<(), aftermath::Error> {
        self.step(|s| {
            let Object::Pending(i) = object else {
                return Err(Error::Unsupported("InitializeText detached candidate"));
            };
            let candidate = s
                .pending
                .get_mut(i)
                .and_then(|p| p.created.as_mut())
                .ok_or(Error::Unsupported("InitializeText candidate consumed"))?;
            if let Some(scope) = s.inputs.text_scope {
                super::text::initialize_headless(s.inputs.definitions, candidate, scope)
                    .map_err(Error::Text)
            } else {
                s.backend.initialize_text(s.state, candidate)
            }
        })
    }
    fn add(&mut self, slot: i32, object: Object) -> Result<(), aftermath::Error> {
        self.step(|s| {
            let Object::Pending(i) = object else {
                return Err(Error::Unsupported("Add detached candidate"));
            };
            let candidate = &mut s
                .pending
                .get_mut(i)
                .ok_or(Error::Unsupported("Add candidate index"))?
                .created;
            lifecycle::add(s.state, slot, candidate, s.limits, &mut s.backend)?;
            Ok(())
        })
    }
    fn play_by_cloud(&mut self, _: ()) -> Result<bool, aftermath::Error> {
        self.step(|s| Ok(s.inputs.configuration.is_play_by_cloud(s.state.binding())?))
    }
    fn manager_game_local_player(&mut self, _: ()) -> Result<Option<i32>, aftermath::Error> {
        self.step(|s| s.backend.manager_game_local_player(s.state))
    }
    fn owner_turn_active(&mut self, owner: i32) -> Result<bool, aftermath::Error> {
        self.step(|s| Ok(s.state.turn_active(owner)?))
    }
    fn network_multiplayer(&mut self, _: ()) -> Result<bool, aftermath::Error> {
        self.step(|s| {
            Ok(s.inputs
                .configuration
                .is_network_multiplayer(s.state.binding())?)
        })
    }
    fn find_auto_activate(
        &mut self,
        slot: i32,
        threshold: i32,
    ) -> Result<Option<Object>, aftermath::Error> {
        self.step(|s| {
            let network = s
                .inputs
                .configuration
                .is_network_multiplayer(s.state.binding())?;
            let mut cursor = s.state.player_notifications()?.first(slot)?;
            let mut candidate = None;
            while let Some(object) = cursor {
                let owner = s.state.player_notifications()?;
                let record = owner.record_by_object(object)?;
                let runtime = owner
                    .runtime(object)?
                    .live
                    .as_ref()
                    .ok_or(Error::Unsupported("FindAutoActivate live runtime"))?;
                let eligible = if record.has_been_activated {
                    true
                } else if !network {
                    match runtime.definition_icon {
                        live::DefinitionIcon::Unobserved => {
                            return Err(Error::Unsupported("FindAutoActivate definition icon"))
                        }
                        live::DefinitionIcon::ShowInSinglePlayer(false) => true,
                        _ => record.native_19d,
                    }
                } else {
                    record.native_19d
                };
                if eligible && (severity_override(record)? || record.severity >= threshold) {
                    if runtime.lock_count > 0 {
                        return Ok(Some(Object::Attached(object)));
                    }
                    if candidate.is_none() && !record.has_been_activated {
                        candidate = Some(Object::Attached(object))
                    }
                }
                cursor = owner.next(object)?;
            }
            Ok(candidate)
        })
    }
    fn locked(&mut self, object: Object) -> Result<bool, aftermath::Error> {
        self.step(|s| {
            let o = Self::attached(object)?;
            Ok(s.state
                .player_notifications()?
                .runtime(o)?
                .live
                .as_ref()
                .ok_or(Error::Unsupported("notification lock runtime"))?
                .lock_count
                > 0)
        })
    }
    fn activate(&mut self, object: Object, argument: bool) -> Result<(), aftermath::Error> {
        self.step(|s| {
            lifecycle::activate(s.state, Self::attached(object)?, argument, &mut s.backend)?;
            Ok(())
        })
    }
    fn begin(&mut self, slot: i32) -> Result<Option<ObjectId>, aftermath::Error> {
        self.step(|s| Ok(s.state.player_notifications()?.first(slot)?))
    }
    fn end(&mut self, _: i32) -> Result<Option<ObjectId>, aftermath::Error> {
        Ok(None)
    }
    fn object(&mut self, cursor: Option<ObjectId>) -> Result<Object, aftermath::Error> {
        self.step(|_| {
            cursor
                .map(Object::Attached)
                .ok_or(Error::Unsupported("dereference end notification cursor"))
        })
    }
    fn next(&mut self, current: Option<ObjectId>) -> Result<Option<ObjectId>, aftermath::Error> {
        self.step(|s| {
            Ok(s.state
                .player_notifications()?
                .next(current.ok_or(Error::Unsupported("advance end notification cursor"))?)?)
        })
    }
    fn dismissed(&mut self, object: Object) -> Result<bool, aftermath::Error> {
        self.step(|s| Ok(s.record(object)?.is_dismissed))
    }
    fn expired(&mut self, object: Object) -> Result<bool, aftermath::Error> {
        self.step(|s| s.expiry(object, false))
    }
    fn dismiss(&mut self, object: Object) -> Result<(), aftermath::Error> {
        self.step(|s| {
            lifecycle::dismiss(s.state, Self::attached(object)?, &mut s.backend)?;
            Ok(())
        })
    }
    fn cleanup(&mut self, slot: i32) -> Result<(), aftermath::Error> {
        self.step(|s| {
            lifecycle::cleanup(s.state, slot, &mut s.backend)?;
            Ok(())
        })
    }
    fn valid_for_phase(&mut self, object: Object) -> Result<bool, aftermath::Error> {
        self.step(|s| {
            let object = Self::attached(object)?;
            let owner = s.state.player_notifications()?;
            let record = owner.record_by_object(object)?;
            let vtable = owner
                .runtime(object)?
                .vtable_addresspoint
                .ok_or(Error::Unsupported("notification phase virtual binding"))?;
            match vtable {
                super::initialization::BASE_VTABLE | super::initialization::PRIDE_VTABLE => {
                    Ok(true)
                }
                command_units::VTABLE_ADDRESSPOINT => s
                    .inputs
                    .cities
                    .phases
                    .from_processor(
                        &s.state.retained().core.phase_processor,
                        i32::from(record.id.owner),
                    )
                    .map_err(|_| Error::Unsupported("actual tactical notification phase")),
                // ChooseCityProduction::IsValidForPhase2d29870 forwards to
                // the same strategic-command phase query.
                // ChoosePantheon::IsValidForPhase2d29fc0 uses this query too.
                0x3af4ec0
                | 0x3af4b80
                | choose_tech::VTABLE_ADDRESSPOINT
                | city_range_attack::VTABLE_ADDRESSPOINT => {
                    Ok(s.inputs.culture.valid_for_phase(s.state)?)
                }
                value
                    if [
                        culture::Kind::ChooseCivic,
                        culture::Kind::ConsiderGovernmentChange,
                        culture::Kind::FillCivicSlot,
                    ]
                    .iter()
                    .any(|kind| kind.vtable() == value) =>
                {
                    Ok(s.inputs.culture.valid_for_phase(s.state)?)
                }
                _ => Err(Error::Unsupported(
                    "actual notification phase virtual binding",
                )),
            }
        })
    }
    fn blocking(&mut self, object: Object) -> Result<i32, aftermath::Error> {
        self.step(|s| {
            if s.record(object)?.is_dismissed {
                return Ok(0);
            }
            if s.expiry(object, false)? {
                return Ok(0);
            }
            s.state
                .player_notifications()?
                .runtime(Self::attached(object)?)?
                .default_blocking
                .ok_or(Error::Unsupported("actual notification blocking runtime"))
        })
    }
    fn severity(&mut self, object: Object) -> Result<i32, aftermath::Error> {
        self.step(|s| Ok(s.record(object)?.severity))
    }
}
impl<B: Backend> Blocking for Provider<'_, B> {
    type Owner = i32;
    fn notification_manager(&mut self, _: bool) -> Result<(), aftermath::Error> {
        Ok(())
    }
    fn owner_id(&mut self, owner: i32) -> Result<i32, aftermath::Error> {
        self.step(|s| {
            s.state
                .retained()
                .players
                .iter()
                .find(|p| p.requested_owner == owner)
                .map(|p| p.header.owner)
                .ok_or(Error::Unsupported("retained notification Player receiver"))
        })
    }
    fn turn_complete(&mut self, owner: i32) -> Result<bool, aftermath::Error> {
        self.step(|s| Ok(super::turn_complete(s.state, owner)?))
    }
    fn owner_blocking(&mut self, owner: i32) -> Result<i32, aftermath::Error> {
        self.step(|s| Ok(super::owner_blocking(s.state, owner)?))
    }
    fn store_owner_blocking(&mut self, owner: i32, value: i32) -> Result<(), aftermath::Error> {
        self.step(|s| {
            s.state.store_player_end_turn_blocking(owner, value)?;
            Ok(())
        })
    }
    fn local_player(&mut self) -> Result<i32, aftermath::Error> {
        Ok(self.state.retained().core.local_player)
    }
    fn blocking_changed(&mut self, event: u32, old: i32, new: i32) -> Result<(), aftermath::Error> {
        self.step(|s| s.backend.blocking_changed(s.state, event, old, new))
    }
}
/// Same bounded scalar conversion as the source-closed core selector. Unknown
/// Variant conversions are rejected exactly where this getter is required.
fn severity_override(record: &Notification) -> Result<bool, Error> {
    use c6_core::unit::variants::VariantPayload;
    let Some(entry) = record
        .parameters
        .entries
        .iter()
        .find(|e| e.key == live::SEVERITY_OVERRIDE_KEY)
    else {
        return Ok(false);
    };
    let value = &entry.value;
    if value.type_bits != 1
        || value.modifier_bits != 0
        || value.value.count_flags != 0
        || value.value.element_size != 0
    {
        return Err(Error::Unsupported("severity override Variant conversion"));
    }
    match value.value.payload {
        VariantPayload::Inline32(bits) if bits as u8 <= 1 => Ok(bits as u8 != 0),
        _ => Err(Error::Unsupported("severity override Variant payload")),
    }
}
