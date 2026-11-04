//! In-place Linux movement/Formation upkeep over the actual loaded World.
//!
//! Exact MaxMoves and initialized domain/type queries use existing state/rules
//! implementations. Unimplemented effective getters and synchronous subscribers
//! are mandatory providers, never invented values or silent no-ops. This is not
//! native RestoreMovement acceptance, complete action execution, or a turn.
use crate::unit_movement_state::{self, Inputs};
use c6_core::{
    formation_upkeep::{self, BooleanQuery, FormationUpkeepContext, IntegerQuery},
    loaded_state::LoadedState,
    unit::{Formation, Unit},
    unit_turn_start::{self, FormationRestoreContext, FormationRestoredReport},
    unit_turn_upkeep::{
        self, AttacksRemainingReport, MovementReport, RestoreAttacksContext, RestoreMovementContext,
    },
};

/// Identity of a retained collection slot, independent of mutable Unit ID/owner.
/// Native Find resolves generation bits through the low sixteen slot bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitHandle {
    owner: i32,
    id: u32,
}
impl UnitHandle {
    pub fn collection_owner(self) -> i32 {
        self.owner
    }
    pub fn requested_id(self) -> u32 {
        self.id
    }
    pub fn get(self, state: &LoadedState) -> Result<&Unit, c6_core::world::MissingInput> {
        state
            .world()
            .player(self.owner)?
            .unit(self.id)?
            .map(|found| found.record)
            .ok_or(c6_core::world::MissingInput::InvalidSlot)
    }
    pub fn get_mut(
        self,
        state: &mut LoadedState,
    ) -> Result<&mut Unit, c6_core::loaded_state::Error> {
        state.unit_mut(self.owner, self.id)
    }
}

#[derive(Debug)]
pub enum Error<E> {
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    Movement(unit_movement_state::Error),
    Rules(crate::loaded_actions::Error),
    Provider(E),
    Formation(Box<formation_upkeep::FormationUpkeepError<Error<E>>>),
}

/// Required live continuations. Admission is pure and must cover every reachable
/// getter, subscriber, RNG effect, owner lookup and callback-introduced formation.
/// Getter methods run lazily in Core's native order. Implementations must reuse
/// existing placement/combat implementations when supported, or return an error.
/// They must not install per-call native answers as if these were a simulator.
///
/// Callbacks may edit authoritative fields through LoadedState. They must retain
/// object/collection storage and valid member cursors; replacing the loaded World
/// or erasing retained slots during a call violates the admitted lifetime.
pub trait Effects {
    type Error;
    type Definitions: Copy;
    /// Obtain the actual admitted definitions collection at this reached point.
    fn definitions(&mut self, state: &mut LoadedState) -> Result<Self::Definitions, Self::Error>;
    fn preflight_movement(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
        movement_branch: bool,
    ) -> Result<(), Self::Error>;
    fn preflight_formation(
        &self,
        state: &LoadedState,
        owner: i32,
        id: u32,
    ) -> Result<(), Self::Error>;
    fn integer(
        &mut self,
        state: &mut LoadedState,
        actor: UnitHandle,
        query: IntegerQuery,
    ) -> Result<i32, Self::Error>;
    fn boolean(
        &mut self,
        state: &mut LoadedState,
        actor: UnitHandle,
        query: BooleanQuery,
    ) -> Result<bool, Self::Error>;
    /// Resolve actual At(runtime_index)->IsCanCapture; actual null is None.
    fn definition_can_capture(
        &mut self,
        state: &mut LoadedState,
        definitions: Self::Definitions,
        runtime_index: u32,
    ) -> Result<Option<bool>, Self::Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        report: MovementReport,
    ) -> Result<(), Self::Error>;
    /// Current Player::IsBarbarian, after callbacks; no saved-roster inference.
    fn owner_is_barbarian(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error>;
}

/// Effectful dependencies after the computed rules/world getters are bound.
/// Implement CanEmbark with the existing placement provider, and deliver every
/// admitted synchronous subscriber. Unknown dependencies must return an error.
pub trait Continuations {
    type Error;
    fn preflight_movement(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
        branch: bool,
    ) -> Result<(), Self::Error>;
    fn preflight_formation(
        &self,
        state: &LoadedState,
        owner: i32,
        id: u32,
    ) -> Result<(), Self::Error>;
    fn can_embark(
        &mut self,
        state: &mut LoadedState,
        actor: UnitHandle,
    ) -> Result<bool, Self::Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        report: MovementReport,
    ) -> Result<(), Self::Error>;
    fn owner_is_barbarian(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error>;
}

/// Additional synchronous effects reached by Formation::RestoreMovement.
/// These callbacks receive the same authoritative state as member movement
/// reports. They must execute every reached gameplay/RNG effect before return;
/// collecting a transcript does not satisfy this contract.
pub trait FormationRestoreEffects: Effects {
    fn formation_restored(
        &mut self,
        state: &mut LoadedState,
        report: FormationRestoredReport,
    ) -> Result<(), Self::Error>;
}

/// Formation restoration extends the existing rules-backed provider with its
/// two reached report overloads, without supplying default subscribers.
pub trait FormationRestoreContinuations: Continuations {
    fn formation_restored(
        &mut self,
        state: &mut LoadedState,
        report: FormationRestoredReport,
    ) -> Result<(), Self::Error>;
}

/// Required changed-attacks subscribers on the authoritative loaded World.
/// Unknown subscribers reject during the pure preflight, before assignment.
/// Delivery must preserve every gameplay/RNG effect and callback mutation.
pub trait AttacksReports {
    type Error;
    fn preflight_attacks_report(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
    ) -> Result<(), Self::Error>;
    fn attacks_report(
        &mut self,
        state: &mut LoadedState,
        report: AttacksRemainingReport,
    ) -> Result<(), Self::Error>;
}

struct Attacks<'s, 'd, E> {
    state: &'s mut LoadedState,
    definitions: &'d crate::formation_rules::Definitions,
    reports: &'s mut E,
    actor: UnitHandle,
}
impl<E: AttacksReports> RestoreAttacksContext for Attacks<'_, '_, E> {
    type Error = Error<E::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor.get(self.state).map_err(Error::World)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.actor.get_mut(self.state).map_err(Error::State)
    }
    fn combat_max_num_attacks(&self) -> Result<i32, Self::Error> {
        self.definitions
            .combat_max_num_attacks(self.state)
            .map_err(Error::Rules)
    }
    fn preflight_attacks_report(&self) -> Result<(), Self::Error> {
        self.reports
            .preflight_attacks_report(self.state, self.actor)
            .map_err(Error::Provider)
    }
    fn send_attacks_report(&mut self, report: AttacksRemainingReport) -> Result<(), Self::Error> {
        self.reports
            .attacks_report(self.state, report)
            .map_err(Error::Provider)
    }
}

/// Restore the actual retained Unit's attacks from identity-gated rules and
/// its current bonus. The core implementation owns zero fallback, wrapping
/// arithmetic, equality, assignment and synchronous reporting order. Equal
/// values require no reporting provider capability. Errors after delivery starts
/// retain the assignment and subscriber effects; this is a leaf, not a turn.
pub fn restore_attacks_remaining<E: AttacksReports>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    definitions: &crate::formation_rules::Definitions,
    reports: &mut E,
) -> Result<
    unit_turn_upkeep::RestoreAttacksOutcome,
    unit_turn_upkeep::RestoreAttacksError<Error<E::Error>>,
> {
    unit_turn_upkeep::restore_attacks_remaining(&mut Attacks {
        state,
        definitions,
        reports,
        actor: UnitHandle { owner, id },
    })
}

#[derive(Debug)]
pub enum RulesEffectError<E> {
    World(c6_core::world::MissingInput),
    Rules(crate::loaded_actions::Error),
    Continuation(E),
}

/// Concrete getter provider over the admitted shared combat definitions and
/// current loaded Unit/player fields. Callers supply reached external effects,
/// not combat/attack/disabled/capture answers.
pub struct RulesEffects<'a, C> {
    pub definitions: &'a crate::formation_rules::Definitions,
    pub continuations: C,
}
impl<'a, C: Continuations> Effects for RulesEffects<'a, C> {
    type Error = RulesEffectError<C::Error>;
    type Definitions = &'a crate::formation_rules::Definitions;
    fn definitions(&mut self, state: &mut LoadedState) -> Result<Self::Definitions, Self::Error> {
        self.definitions
            .check(state)
            .map_err(RulesEffectError::Rules)?;
        Ok(self.definitions)
    }
    fn preflight_movement(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
        branch: bool,
    ) -> Result<(), Self::Error> {
        self.continuations
            .preflight_movement(state, actor, branch)
            .map_err(RulesEffectError::Continuation)
    }
    fn preflight_formation(
        &self,
        state: &LoadedState,
        owner: i32,
        id: u32,
    ) -> Result<(), Self::Error> {
        self.continuations
            .preflight_formation(state, owner, id)
            .map_err(RulesEffectError::Continuation)
    }
    fn integer(
        &mut self,
        state: &mut LoadedState,
        actor: UnitHandle,
        query: IntegerQuery,
    ) -> Result<i32, Self::Error> {
        let unit = actor.get(state).map_err(RulesEffectError::World)?;
        match query {
            IntegerQuery::Combat => self.definitions.combat(state, unit),
            IntegerQuery::RangedCombat => self.definitions.ranged(state, unit),
            IntegerQuery::BombardCombat => self.definitions.bombard(state, unit),
            // Core context owns these current runtime/rules queries directly.
            IntegerQuery::Domain | IntegerQuery::MaxMoves => {
                return Err(RulesEffectError::Rules(
                    crate::loaded_actions::Error::Unsupported("upkeep internal integer routing"),
                ))
            }
        }
        .map_err(RulesEffectError::Rules)
    }
    fn boolean(
        &mut self,
        state: &mut LoadedState,
        actor: UnitHandle,
        query: BooleanQuery,
    ) -> Result<bool, Self::Error> {
        if query == BooleanQuery::CanEmbark {
            return self
                .continuations
                .can_embark(state, actor)
                .map_err(RulesEffectError::Continuation);
        }
        let unit = actor.get(state).map_err(RulesEffectError::World)?;
        match query {
            BooleanQuery::CanAttack => self.definitions.can_attack(state, unit),
            BooleanQuery::MovementDisabledThisTurn => {
                self.definitions.movement_disabled(state, unit)
            }
            BooleanQuery::CanEmbark => {
                return self
                    .continuations
                    .can_embark(state, actor)
                    .map_err(RulesEffectError::Continuation)
            }
        }
        .map_err(RulesEffectError::Rules)
    }
    fn definition_can_capture(
        &mut self,
        state: &mut LoadedState,
        definitions: Self::Definitions,
        index: u32,
    ) -> Result<Option<bool>, Self::Error> {
        definitions
            .definition_can_capture(state, index)
            .map_err(RulesEffectError::Rules)
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        report: MovementReport,
    ) -> Result<(), Self::Error> {
        self.continuations
            .report(state, report)
            .map_err(RulesEffectError::Continuation)
    }
    fn owner_is_barbarian(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error> {
        self.continuations
            .owner_is_barbarian(state, owner)
            .map_err(RulesEffectError::Continuation)
    }
}

impl<C: FormationRestoreContinuations> FormationRestoreEffects for RulesEffects<'_, C> {
    fn formation_restored(
        &mut self,
        state: &mut LoadedState,
        report: FormationRestoredReport,
    ) -> Result<(), Self::Error> {
        self.continuations
            .formation_restored(state, report)
            .map_err(RulesEffectError::Continuation)
    }
}

struct Movement<'s, 'i, 'd, E> {
    state: &'s mut LoadedState,
    inputs: &'i Inputs<'d>,
    effects: &'s mut E,
    actor: UnitHandle,
}
impl<E: Effects> RestoreMovementContext for Movement<'_, '_, '_, E> {
    type Error = Error<E::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor.get(self.state).map_err(Error::World)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.actor.get_mut(self.state).map_err(Error::State)
    }
    fn max_moves(&self) -> Result<i32, Self::Error> {
        unit_movement_state::unit_max_moves(self.state, self.unit()?, self.inputs)
            .map_err(Error::Movement)
    }
    fn preflight_restore_movement(&self, branch: bool) -> Result<(), Self::Error> {
        self.effects
            .preflight_movement(self.state, self.actor, branch)
            .map_err(Error::Provider)
    }
    fn send_movement_report(&mut self, report: MovementReport) -> Result<(), Self::Error> {
        self.effects
            .report(self.state, report)
            .map_err(Error::Provider)
    }
    fn sync_current_formation(&mut self) -> Result<(), Self::Error> {
        // Validate initialized EditFormation against the CURRENT attachment. A
        // stale saved formation ID is never enough to establish that pointer.
        let actor = self.unit()?;
        let attachment = self
            .state
            .unit_runtime(actor.e_owner, actor.id)
            .map_err(Error::State)?
            .formation;
        if attachment.is_some() {
            // Reread just as the native nonnull branch does.
            let actor = self.unit()?;
            let current = self
                .state
                .unit_runtime(actor.e_owner, actor.id)
                .map_err(Error::State)?
                .formation;
            let f = current.ok_or(Error::State(
                c6_core::loaded_state::Error::RuntimeDependenciesChanged,
            ))?;
            update_formation(
                self.state,
                f.owner,
                f.requested_id,
                self.inputs,
                self.effects,
            )
            .map_err(|e| Error::Formation(Box::new(e)))?;
        }
        Ok(())
    }
    fn owner_is_barbarian(&self) -> Result<bool, Self::Error> {
        self.effects
            .owner_is_barbarian(self.state, self.unit()?.e_owner)
            .map_err(Error::Provider)
    }
}

/// Restore an actual retained Unit in place. Completed callback effects survive
/// errors; no World clone or scratch record is copied over synchronous changes.
pub fn restore_movement<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    inputs: &Inputs<'_>,
    effects: &mut E,
) -> Result<
    unit_turn_upkeep::RestoreMovementOutcome,
    unit_turn_upkeep::RestoreMovementError<Error<E::Error>>,
> {
    let actor = UnitHandle { owner, id };
    actor
        .get(state)
        .map_err(|e| unit_turn_upkeep::RestoreMovementError::EffectsUnavailable(Error::World(e)))?;
    unit_turn_upkeep::restore_movement(Some(&mut Movement {
        state,
        inputs,
        effects,
        actor,
    }))
}

struct Members<'s, 'i, 'd, E> {
    state: &'s mut LoadedState,
    inputs: &'i Inputs<'d>,
    effects: &'s mut E,
    owner: i32,
    id: u32,
}
impl<E: Effects> FormationUpkeepContext for Members<'_, '_, '_, E> {
    type Error = Error<E::Error>;
    type UnitHandle = UnitHandle;
    type Definitions = E::Definitions;
    type Units = i32;
    fn formation(&self) -> Result<&Formation, Self::Error> {
        self.state
            .world()
            .player(self.owner)
            .map_err(Error::World)?
            .formation(self.id)
            .map_err(Error::World)?
            .map(|f| f.record)
            .ok_or(Error::World(c6_core::world::MissingInput::InvalidSlot))
    }
    fn formation_mut(&mut self) -> Result<&mut Formation, Self::Error> {
        self.state
            .formation_mut(self.owner, self.id)
            .map_err(Error::State)
    }
    fn preflight_update_attributes(&self) -> Result<(), Self::Error> {
        self.effects
            .preflight_formation(self.state, self.owner, self.id)
            .map_err(Error::Provider)
    }
    fn definitions(&mut self) -> Result<Self::Definitions, Self::Error> {
        self.effects
            .definitions(self.state)
            .map_err(Error::Provider)
    }
    fn units(&mut self) -> Result<i32, Self::Error> {
        let owner = self.formation()?.owner;
        self.state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .unit_records()
            .map_err(Error::World)?;
        Ok(owner)
    }
    fn find_unit(&mut self, owner: i32, id: u32) -> Result<Option<UnitHandle>, Self::Error> {
        Ok(self
            .state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .unit(id)
            .map_err(Error::World)?
            .map(|_| UnitHandle { owner, id }))
    }
    fn unit(&self, handle: UnitHandle) -> Result<&Unit, Self::Error> {
        handle.get(self.state).map_err(Error::World)
    }
    fn integer(&mut self, handle: UnitHandle, query: IntegerQuery) -> Result<i32, Self::Error> {
        let unit = self.unit(handle)?;
        match query {
            IntegerQuery::MaxMoves => {
                unit_movement_state::unit_max_moves(self.state, unit, self.inputs)
                    .map_err(Error::Movement)
            }
            IntegerQuery::Domain => Ok(self
                .state
                .unit_runtime(unit.e_owner, unit.id)
                .map_err(Error::State)?
                .domain),
            _ => self
                .effects
                .integer(self.state, handle, query)
                .map_err(Error::Provider),
        }
    }
    fn boolean(&mut self, handle: UnitHandle, query: BooleanQuery) -> Result<bool, Self::Error> {
        self.effects
            .boolean(self.state, handle, query)
            .map_err(Error::Provider)
    }
    fn native_type_index(&mut self, handle: UnitHandle) -> Result<u32, Self::Error> {
        let unit = self.unit(handle)?;
        Ok(self
            .state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .definition_index as u32)
    }
    fn definition_can_capture(
        &mut self,
        definitions: Self::Definitions,
        index: u32,
    ) -> Result<Option<bool>, Self::Error> {
        self.effects
            .definition_can_capture(self.state, definitions, index)
            .map_err(Error::Provider)
    }
    fn send_member_movement(&mut self, report: MovementReport) -> Result<(), Self::Error> {
        self.effects
            .report(self.state, report)
            .map_err(Error::Provider)
    }
}

impl<E: FormationRestoreEffects> FormationRestoreContext for Members<'_, '_, '_, E> {
    fn unit_mut(&mut self, handle: UnitHandle) -> Result<&mut Unit, Self::Error> {
        handle.get_mut(self.state).map_err(Error::State)
    }
    fn send_formation_restored(
        &mut self,
        report: FormationRestoredReport,
    ) -> Result<(), Self::Error> {
        self.effects
            .formation_restored(self.state, report)
            .map_err(Error::Provider)
    }
}

/// Execute the source-derived Linux Formation::RestoreMovement continuation on
/// current owned records, including member attention resets, attribute updates
/// and the ordered Formation reports. Existing rules-backed MaxMoves and domain
/// queries are reused; no actor/World copyback can overwrite subscriber changes.
///
/// Errors retain completed writes and callback effects. The caller must admit
/// the required subscriber/getter profile and valid retained collections; this
/// does not execute earlier activation stages or establish native turn parity.
pub fn restore_formation_movement<E: FormationRestoreEffects>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    inputs: &Inputs<'_>,
    effects: &mut E,
) -> Result<(), formation_upkeep::FormationUpkeepError<Error<E::Error>>> {
    state
        .world()
        .player(owner)
        .and_then(|p| p.formation(id))
        .and_then(|f| f.ok_or(c6_core::world::MissingInput::InvalidSlot))
        .map_err(|e| formation_upkeep::FormationUpkeepError::EffectsUnavailable(Error::World(e)))?;
    unit_turn_start::restore_formation_movement(&mut Members {
        state,
        inputs,
        effects,
        owner,
        id,
    })
}

/// Compute actual current Formation attributes, domain, movement and member
/// reports using the Core continuation and current live Units collections.
pub fn update_formation<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    inputs: &Inputs<'_>,
    effects: &mut E,
) -> Result<(), formation_upkeep::FormationUpkeepError<Error<E::Error>>> {
    state
        .world()
        .player(owner)
        .and_then(|p| p.formation(id))
        .and_then(|f| f.ok_or(c6_core::world::MissingInput::InvalidSlot))
        .map_err(|e| formation_upkeep::FormationUpkeepError::EffectsUnavailable(Error::World(e)))?;
    formation_upkeep::update_attributes(&mut Members {
        state,
        inputs,
        effects,
        owner,
        id,
    })
}
