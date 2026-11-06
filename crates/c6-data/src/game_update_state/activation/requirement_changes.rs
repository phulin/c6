//! Turn-start invalidation over the existing current requirement/set owner.
//! Predicates and modifier leaves borrow the same state/backend as the action.
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
    simple_requirement::Requirement,
};
pub use changes::{CurrentRequirement, CurrentSet};
pub use sets::{
    InitializedRequirement, RequirementDefinition, SetDefinition, SetListener, Subject,
};

#[derive(Debug)]
pub enum Error<E> {
    Binding,
    Missing(u64),
    Kernel(sets::Error<E>),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "activation requirement change: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
/// This is the sole owner of imported child states and set subscription rings.
/// It can also borrow an already-used shared owner through `from_current`.
/// These effect families update native diplomatic gameplay opinion. The
/// human-owner path cannot send statements. Nonhuman owners additionally need
/// a source-proven no-statement guard; reachable statements remain unsupported.
/// No formal relation, visibility, favor or gameplay RNG effect is omitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpinionKind {
    SettledCities,
    TradeRelations,
    ThirdParty,
    Archaeology,
    BrokenPledge,
    ForceIncursion,
    KeptPledge,
    NoPledge,
    ThirdPartyWarmonger,
    Warmonger,
    Bushido,
    Exploitative,
}
/// Supported current opinion-only subject tests contain no script/random leaf.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpinionRequirements {
    KnownMajor,
    KnownMajor10Turns,
    KnownMajor30Turns,
    KnownWithoutOpenBorders,
    NewWarmonger,
}
#[derive(Debug)]
pub struct OwnerModifier {
    pub receiver: u64,
    pub owner: i32,
    pub instance: i32,
    pub enabled: bool,
    pub suppressed: bool,
    pub has_activated: bool,
    pub owner_requirement_set: Option<u64>,
    /// None remains unsupported, rather than assigning an opinion family.
    pub opinion: Option<(OpinionKind, OpinionRequirements)>,
    /// Actual SimpleEffect promise/agenda statement enable flags (+0x4a/+0x64).
    pub statement_flags: [bool; 2],
    pub current_update_instance: Option<i32>,
}
#[derive(Debug)]
pub struct Runtime {
    binding: WireBinding,
    sets: sets::Runtime,
    owner_modifiers: Vec<OwnerModifier>,
    pub omitted_opinion_updates: Vec<u64>,
}
impl Runtime {
    pub fn from_current(binding: WireBinding, sets: sets::Runtime) -> Self {
        Self {
            binding,
            sets,
            owner_modifiers: Vec::new(),
            omitted_opinion_updates: Vec::new(),
        }
    }
    pub fn bind_owner_modifiers(&mut self, rows: Vec<OwnerModifier>) -> Result<(), &'static str> {
        if !self.owner_modifiers.is_empty()
            || rows
                .iter()
                .enumerate()
                .any(|(i, r)| rows[..i].iter().any(|p| p.receiver == r.receiver))
        {
            return Err("duplicate owner-modifier runtime");
        }
        self.owner_modifiers = rows;
        Ok(())
    }
    pub fn owner_modifiers(&self) -> &[OwnerModifier] {
        &self.owner_modifiers
    }
    fn opinion_owner_changed(
        &mut self,
        state: &LoadedState,
        receiver: u64,
    ) -> Result<bool, &'static str> {
        let Some(row) = self
            .owner_modifiers
            .iter_mut()
            .find(|r| r.receiver == receiver)
        else {
            return Ok(false);
        };
        // Native suppression returns before owner requirements/effect queries.
        if row.suppressed {
            return Ok(true);
        }
        if row.opinion.is_none() {
            return Ok(false);
        }
        if state
            .player_slot_status(row.owner)
            .map_err(|_| "current opinion owner controls")?
            != 3
            && !no_opinion_statement(state, row)?
        {
            return Err("nonhuman opinion statements require concrete effects");
        }
        let met = match row.owner_requirement_set {
            None => true,
            Some(id) => matches!(
                self.sets.state(id).ok_or("current modifier owner set")?,
                0 | 2 | 4
            ),
        };
        if met {
            if !row.enabled {
                row.has_activated = true;
            }
            row.enabled = true;
            // Source Attach/Detach are literal returns for these families.
            // BeforeUpdate retains instance; policy target enumeration and
            // opinion/explanation tables have no player-decision authority.
            row.current_update_instance = Some(row.instance);
        } else {
            row.enabled = false;
        }
        self.omitted_opinion_updates.push(receiver);
        Ok(true)
    }
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
    pub fn state(&self, receiver: u64) -> Option<&Requirement> {
        self.sets.simple_requirement(receiver)
    }
    pub fn state_mut(&mut self, receiver: u64) -> Option<&mut Requirement> {
        self.sets.simple_requirement_mut(receiver)
    }
    pub fn sets(&self) -> &sets::Runtime {
        &self.sets
    }
    pub fn sets_mut(&mut self) -> &mut sets::Runtime {
        &mut self.sets
    }
    fn check<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if self.binding != *state.binding() {
            Err(Error::Binding)
        } else {
            Ok(())
        }
    }
    /// Exact SimpleRequirement.Invalidate against this SAME live store and
    /// current backend: active guard, virtual Test, store, ordered listeners.
    pub fn invalidate<S: Services>(
        &mut self,
        state: &mut LoadedState,
        backend: &mut S,
        receiver: u64,
    ) -> Result<(), Error<S::Error>> {
        self.check(state)?;
        changes::invalidate(
            &mut Context {
                state,
                backend,
                runtime: self,
            },
            receiver,
        )
        .map_err(Error::Kernel)
    }
    /// Called by SimpleRequirement only AFTER its state store. The old/new pair
    /// is not a predicate answer and never replaces current child reads.
    pub fn changed<S: Services>(
        &mut self,
        state: &mut LoadedState,
        backend: &mut S,
        receiver: u64,
        old: i32,
        new: i32,
    ) -> Result<(), Error<S::Error>> {
        self.check(state)?;
        if self.state(receiver).is_none_or(|r| r.state != new) {
            return Err(Error::Missing(receiver));
        }
        changes::dispatch_requirement_changed(
            &mut Context {
                state,
                backend,
                runtime: self,
            },
            receiver,
            old,
            new,
        )
        .map_err(Error::Kernel)
    }
    /// Prefer this entry when the caller has already computed the current Test
    /// result: inverse/trigger4/active guards remain in the existing kernel.
    pub fn set_state<S: Services>(
        &mut self,
        state: &mut LoadedState,
        backend: &mut S,
        receiver: u64,
        tested: i32,
        notify: bool,
    ) -> Result<(), Error<S::Error>> {
        self.check(state)?;
        changes::set_requirement_state(
            &mut Context {
                state,
                backend,
                runtime: self,
            },
            receiver,
            tested,
            notify,
        )
        .map_err(Error::Kernel)
    }
}
/// Restrict the existing opinion-table policy omission to source branches that
/// cannot construct a diplomatic statement. Positive branches remain required.
fn no_opinion_statement(state: &LoadedState, row: &OwnerModifier) -> Result<bool, &'static str> {
    // SimpleEffect.Apply235f5c0 requires either flag before constructing a
    // statement. Its base opinion-table update is the existing policy omission.
    if row.statement_flags == [false, false] {
        return Ok(true);
    }
    let (kind, requirements) = row.opinion.ok_or("current opinion family")?;
    match kind {
        OpinionKind::SettledCities | OpinionKind::Archaeology => {
            // HasNewIncursions2367a50/2365c90 calls GetIncursionsSince2aa89a0
            // on the OWNER's retained diplomacy map. An empty correct-type
            // vector cannot pass, regardless of contact/throttle/old opinion.
            let promise = if kind == OpinionKind::SettledCities {
                0xfadf5055u32 as i32
            } else {
                0x1347b09f
            };
            let owner = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == row.owner)
                .ok_or("current opinion owner diplomacy")?;
            Ok(owner.diplomacy.incursions_made.iter().all(|(_, types)| {
                types
                    .entries
                    .iter()
                    .all(|(kind, turns)| *kind != promise || turns.is_empty())
            }))
        }
        OpinionKind::Bushido | OpinionKind::Exploitative => {
            // Subject requirements gate Apply, before score/statement queries.
            let turns = match requirements {
                OpinionRequirements::KnownMajor10Turns => 10,
                OpinionRequirements::KnownMajor30Turns => 30,
                _ => return Ok(false),
            };
            let relations = state
                .game_relations()
                .map_err(|_| "current opinion relations")?;
            // Manager validity is independent of which player records the
            // bounded importer retained. Visit every native relation slot.
            for other in 0..relations.player_count() {
                if other == row.owner
                    || !state
                        .world()
                        .player_is_valid(other)
                        .map_err(|_| "current opinion subject validity")?
                {
                    continue;
                }
                if relations
                    .has_met(row.owner, other)
                    .map_err(|_| "current opinion contact")?
                    && relations
                        .met_turn(row.owner, other)
                        .map_err(|_| "current opinion met turn")?
                        .wrapping_add(turns)
                        <= state.retained().core.current_turn
                {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}
/// All real reached continuation effects are mandatory. In particular a
/// modifier callback cannot disappear just because its owner is AI or UI named.
/// `sets` is the same owner after the source set-state assignment.
pub trait Services {
    type Error;
    /// Borrow the SAME current modifier graph used by other gameplay callbacks.
    /// The concrete owner update must enumerate subjects, use `sets` for every
    /// child/set lifetime, and apply TradeRelations to the canonical Player
    /// blackboard. Mirror resulting owner-control fields into `row` only after
    /// that update; this row is the transport projection, not another graph.
    /// False means unbound and is an explicit unsupported continuation.
    fn trade_relations_changed(
        &mut self,
        _state: &mut LoadedState,
        _sets: &mut sets::Runtime,
        _row: &mut OwnerModifier,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn test_requirement(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
    ) -> Result<i32, Self::Error>;
    fn modifier_changed(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        listener: SetListener,
        set: u64,
        old: i32,
        new: i32,
    ) -> Result<(), Self::Error>;
    fn destroy_requirement(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
    ) -> Result<(), Self::Error>;
    fn destroy_set(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
    ) -> Result<(), Self::Error>;
}
struct Context<'a, S> {
    state: &'a mut LoadedState,
    backend: &'a mut S,
    runtime: &'a mut Runtime,
}
impl<S: Services> changes::Context for Context<'_, S> {
    type Error = S::Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(&self.runtime.sets)
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(&mut self.runtime.sets)
    }
    fn test_requirement(&mut self, receiver: u64) -> Result<i32, sets::Error<Self::Error>> {
        self.backend
            .test_requirement(self.state, &mut self.runtime.sets, receiver)
            .map_err(sets::Error::Context)
    }
    fn modifier_changed(
        &mut self,
        listener: SetListener,
        set: u64,
        old: i32,
        new: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        if let SetListener::Owner(receiver) = listener {
            if let Some(row) = self.runtime.owner_modifiers.iter_mut().find(|row| {
                row.receiver == receiver
                    && row
                        .opinion
                        .is_some_and(|(kind, _)| kind == OpinionKind::TradeRelations)
            }) {
                if row.suppressed {
                    return Ok(());
                }
                if !self
                    .backend
                    .trade_relations_changed(self.state, &mut self.runtime.sets, row)
                    .map_err(sets::Error::Context)?
                {
                    return Err(sets::Error::CurrentInput(
                        "current TradeRelations modifier graph",
                    ));
                }
                return Ok(());
            }
            if self
                .runtime
                .opinion_owner_changed(self.state, receiver)
                .map_err(sets::Error::CurrentInput)?
            {
                return Ok(());
            }
        }
        self.backend
            .modifier_changed(self.state, &mut self.runtime.sets, listener, set, old, new)
            .map_err(sets::Error::Context)
    }
    fn destroy_requirement(&mut self, receiver: u64) -> Result<(), sets::Error<Self::Error>> {
        self.backend
            .destroy_requirement(self.state, &mut self.runtime.sets, receiver)
            .map_err(sets::Error::Context)
    }
    fn destroy_set(&mut self, receiver: u64) -> Result<(), sets::Error<Self::Error>> {
        self.backend
            .destroy_set(self.state, &mut self.runtime.sets, receiver)
            .map_err(sets::Error::Context)
    }
    fn reset_trigger(&mut self, set: u64) -> Result<(), sets::Error<Self::Error>> {
        changes::reset_trigger_state(&mut self.runtime.sets, set)
    }
}
