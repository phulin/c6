//! Persistent combat callback owner. Reporting/ScriptState, requirements and
//! effect-owned totals survive every callback; proposal payloads stay in the
//! canonical LoadedState registry. This does not complete melee resolution.
use super::proposals;
pub mod requirements;
use crate::{
    loaded_actions::{configuration::RawGameConfiguration, culture_notifications as calendar},
    loaded_upkeep,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::{LoadedState, ObservedSnapshotWitness},
    map::UnitReference,
    melee_attack::proposals::{alliance_strength, player_strength, registry},
    unit::ComponentId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextObject {
    Player(i32),
    /// Objects.Unit's IPlayer interface uses its stored CID owner, while its
    /// IUnit interface resolves the CURRENT corresponding canonical Unit.
    Unit(UnitReference),
    Alliance {
        first_player: i32,
        second_player: i32,
    },
}
#[derive(Debug)]
pub enum EffectState {
    Player {
        context: u64,
        effect: player_strength::Effect,
    },
    Alliance {
        context: u64,
        amount: i32,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Object {
    pub kind: ContextObject,
    pub instance_id: i32,
    pub unit_sort_suffix: Option<[i32; 2]>,
}
pub struct EffectRuntime {
    binding: WireBinding,
    witness: ObservedSnapshotWitness,
    effects: Vec<(u64, EffectState)>,
    objects: Vec<(u64, Object)>,
}
#[derive(Debug)]
pub enum QueryError {
    Binding,
    AlreadyAdvanced,
    DuplicateEffect(u64),
    DuplicateObject(u64),
    MissingObject(u64),
    MissingEffect(u64),
    Missing(&'static str),
    Actor(Box<super::Error>),
    World(c6_core::world::MissingInput),
    Access(c6_core::loaded_state::combat_proposals::AccessError),
    Proposal(registry::Error),
    Calendar(Box<calendar::Error>),
    Unsupported(&'static str),
}
impl EffectRuntime {
    pub fn from_initial(
        state: &LoadedState,
        witness: &ObservedSnapshotWitness,
        effects: Vec<(u64, EffectState)>,
        objects: Vec<(u64, Object)>,
    ) -> Result<Self, QueryError> {
        if !state.is_pristine() {
            return Err(QueryError::AlreadyAdvanced);
        }
        if state.observed_snapshot() != Some(witness) {
            return Err(QueryError::Binding);
        }
        for (i, effect) in effects.iter().enumerate() {
            if effects[..i].iter().any(|old| old.0 == effect.0) {
                return Err(QueryError::DuplicateEffect(effect.0));
            }
        }
        for (i, object) in objects.iter().enumerate() {
            if objects[..i].iter().any(|old| old.0 == object.0) {
                return Err(QueryError::DuplicateObject(object.0));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            witness: witness.clone(),
            effects,
            objects,
        })
    }
    pub fn effect(&self, identity: u64) -> Result<&EffectState, QueryError> {
        self.effects
            .iter()
            .find(|r| r.0 == identity)
            .map(|r| &r.1)
            .ok_or(QueryError::MissingEffect(identity))
    }
    pub fn object(&self, identity: u64) -> Result<&Object, QueryError> {
        self.objects
            .iter()
            .find(|r| r.0 == identity)
            .map(|r| &r.1)
            .ok_or(QueryError::MissingObject(identity))
    }
}
/// One current RequirementSystem owner, including captured existing sets and
/// computed newly constructed subject sets. No native Test answers are inputs.
pub trait Requirements {
    type Error;
    fn state(&self, state: &LoadedState, set: u64) -> Result<i32, Self::Error>;
    fn create_subject_set(
        &mut self,
        state: &mut LoadedState,
        modifier: u64,
        proposal: registry::Handle,
        objects: &EffectRuntime,
    ) -> Result<Option<u64>, Self::Error>;
    fn unsubscribe_subject_set(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        proposal: registry::Handle,
        set: u64,
    ) -> Result<(), Self::Error>;
    fn release_subject_set(&mut self, state: &mut LoadedState, set: u64)
        -> Result<(), Self::Error>;
}
pub struct Inputs {
    pub calendar: calendar::Definitions,
    pub configuration: RawGameConfiguration,
    /// Current raw Configuration.TurnLimitType. None remains unknown.
    pub turn_limit_type: Option<u32>,
    /// Current GameEffects logging scalar, not a guessed constructor default.
    pub logging_priority: Option<i32>,
}
pub struct Backend<O, R> {
    pub reporting: O,
    pub requirements: R,
    effects: EffectRuntime,
    inputs: Inputs,
}
#[derive(Debug)]
pub enum Error<O, R> {
    Query(QueryError),
    Reporting(O),
    Requirements(R),
    Player(Box<player_strength::Error<QueryError>>),
    Alliance(Box<alliance_strength::Error<QueryError>>),
    Modifier(
        Box<
            c6_core::melee_attack::proposals::collection_modifiers::Error<
                proposals::collection_modifiers::Error<Error<O, R>>,
            >,
        >,
    ),
}
impl<O, R> Backend<O, R> {
    pub fn new(
        state: &LoadedState,
        reporting: O,
        requirements: R,
        effects: EffectRuntime,
        inputs: Inputs,
    ) -> Result<Self, QueryError> {
        if !state.is_pristine() {
            return Err(QueryError::AlreadyAdvanced);
        }
        if effects.binding != *state.binding()
            || state.observed_snapshot() != Some(&effects.witness)
        {
            return Err(QueryError::Binding);
        }
        Ok(Self {
            reporting,
            requirements,
            effects,
            inputs,
        })
    }
    pub fn effects(&self) -> &EffectRuntime {
        &self.effects
    }
}

struct EffectContext<'a> {
    state: &'a mut LoadedState,
    object: ContextObject,
    proposal: registry::Handle,
    inputs: &'a Inputs,
    amount: Option<&'a i32>,
}
impl EffectContext<'_> {
    fn start_turn(&self) -> Result<i32, QueryError> {
        let g = self.inputs.configuration;
        self.inputs
            .calendar
            .starting_context_from_raw_configuration(
                self.state,
                g.start_era,
                g.start_year,
                g.calendar_type,
                g.game_speed_type,
                None,
            )
            .map_err(|e| QueryError::Calendar(Box::new(e)))?
            .game_start_turn
            .ok_or(QueryError::Missing("computed start turn"))
    }
}
impl registry::Context for EffectContext<'_> {
    type Error = QueryError;
    fn runtime(&self) -> Result<&registry::Runtime, QueryError> {
        self.state.combat_proposals().map_err(QueryError::Access)
    }
    fn runtime_mut(&mut self) -> Result<&mut registry::Runtime, QueryError> {
        self.state
            .combat_proposals_mut()
            .map_err(QueryError::Access)
    }
    fn owner(&mut self, p: registry::Participant) -> Result<i32, QueryError> {
        match p {
            registry::Participant::Unit(u) => Ok(super::actor(self.state, u)
                .map_err(|e| QueryError::Actor(Box::new(e)))?
                .e_owner),
            registry::Participant::District(_) => Err(QueryError::Unsupported(
                "retained District owner in strength effect",
            )),
        }
    }
    fn unit_id(&mut self, u: UnitReference) -> Result<ComponentId, QueryError> {
        let u = super::actor(self.state, u).map_err(|e| QueryError::Actor(Box::new(e)))?;
        Ok(ComponentId {
            owner: u.e_owner as i16,
            component_type: 1,
            id: u.id,
        })
    }
    fn callback(&mut self, _: u64, _: registry::Handle) -> Result<(), QueryError> {
        Err(QueryError::Unsupported(
            "unexpected nested proposal signal in effect getter",
        ))
    }
}
impl player_strength::Context for EffectContext<'_> {
    fn context_player(&mut self) -> Result<Option<i32>, QueryError> {
        let owner = match self.object {
            ContextObject::Player(owner) => owner,
            ContextObject::Unit(u) => i32::from(u.component_id().owner),
            ContextObject::Alliance { .. } => return Ok(None),
        };
        let player = self
            .state
            .world()
            .valid_player(owner)
            .map_err(QueryError::World)?
            .ok_or(QueryError::Missing("current effect context Player"))?;
        Ok(Some(player.owner()))
    }
    fn subject_proposal(&mut self) -> Result<Option<registry::Handle>, QueryError> {
        self.state
            .combat_proposals()
            .map_err(QueryError::Access)?
            .proposal(self.proposal)
            .map_err(QueryError::Proposal)?;
        Ok(Some(self.proposal))
    }
    fn affected_by_count(&mut self) -> Result<i32, QueryError> {
        Err(QueryError::Unsupported(
            "effect stack definition projection",
        ))
    }
    fn stack_power(&mut self, _: f32, _: f32) -> Result<f32, QueryError> {
        Err(QueryError::Unsupported("target Linux powf"))
    }
    fn game_end_turn(&mut self) -> Result<i32, QueryError> {
        let start = self.start_turn()?;
        let maximum = self.state.retained().core.max_turns;
        Ok(
            match self
                .inputs
                .turn_limit_type
                .ok_or(QueryError::Missing("current TurnLimitType"))?
            {
                u32::MAX => 0,
                0xdef9_f0af | 0x8099_25de => maximum.wrapping_add(1),
                _ => start.wrapping_add(maximum),
            },
        )
    }
    fn game_start_turn(&mut self) -> Result<i32, QueryError> {
        self.start_turn()
    }
    fn unit_property(&mut self, _: u32) -> Result<Option<i32>, QueryError> {
        if matches!(self.object, ContextObject::Unit(_)) {
            Err(QueryError::Unsupported("current Unit property conversion"))
        } else {
            Ok(None)
        }
    }
    fn player_property(&mut self, _: u32) -> Result<Option<i32>, QueryError> {
        Err(QueryError::Unsupported(
            "current Player property conversion",
        ))
    }
    fn has_game_interface(&mut self) -> Result<bool, QueryError> {
        Ok(true)
    }
    fn game_property(&mut self, _: u32) -> Result<Option<i32>, QueryError> {
        Err(QueryError::Unsupported("current Game property conversion"))
    }
    fn ever_alive_majors(&mut self) -> Result<i32, QueryError> {
        Err(QueryError::Unsupported("current ever-alive major roster"))
    }
}
impl alliance_strength::Context for EffectContext<'_> {
    fn context_alliance_interface(&mut self) -> Result<bool, QueryError> {
        Ok(matches!(self.object, ContextObject::Alliance { .. }))
    }
    fn subject_proposal(&mut self) -> Result<Option<registry::Handle>, QueryError> {
        <Self as player_strength::Context>::subject_proposal(self)
    }
    fn alliance_players(&mut self) -> Result<(i32, i32), QueryError> {
        match self.object {
            ContextObject::Alliance {
                first_player,
                second_player,
            } => Ok((first_player, second_player)),
            _ => Err(QueryError::Missing("effect IAlliance")),
        }
    }
    fn amount(&mut self) -> Result<i32, QueryError> {
        self.amount
            .copied()
            .ok_or(QueryError::Missing("alliance amount"))
    }
}

impl<O: loaded_upkeep::OperationReports, R: Requirements> loaded_upkeep::OperationReports
    for Backend<O, R>
{
    type Error = Error<O::Error, R::Error>;
    fn preflight_operation_report(
        &self,
        state: &LoadedState,
        actor: loaded_upkeep::UnitHandle,
        report: crate::loaded_actions::IdentityReport,
    ) -> Result<(), Self::Error> {
        self.reporting
            .preflight_operation_report(state, actor, report)
            .map_err(Error::Reporting)
    }
    fn preflight_placement_report(
        &self,
        state: &LoadedState,
        actor: loaded_upkeep::UnitHandle,
        event: u32,
    ) -> Result<(), Self::Error> {
        self.reporting
            .preflight_placement_report(state, actor, event)
            .map_err(Error::Reporting)
    }
    fn placement_report(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_placement_transition::PlacementReport,
    ) -> Result<(), Self::Error> {
        self.reporting
            .placement_report(state, report)
            .map_err(Error::Reporting)
    }
    fn operation_report(
        &mut self,
        state: &mut LoadedState,
        report: crate::loaded_actions::IdentityReport,
    ) -> Result<i32, Self::Error> {
        self.reporting
            .operation_report(state, report)
            .map_err(Error::Reporting)
    }
}
impl<O: loaded_upkeep::OperationReports, R: Requirements> proposals::collection_modifiers::Services
    for Backend<O, R>
{
    type Error = Error<O::Error, R::Error>;
    fn proposal_order(
        &self,
        state: &LoadedState,
        a: registry::Handle,
        b: registry::Handle,
    ) -> Result<std::cmp::Ordering, Self::Error> {
        if a == b {
            return Ok(std::cmp::Ordering::Equal);
        }
        let runtime = state
            .combat_proposals()
            .map_err(|e| Error::Query(QueryError::Access(e)))?;
        let id = |h| {
            runtime
                .active()
                .iter()
                .find(|r| r.1 == h)
                .map(|r| r.0 as i32)
                .ok_or(Error::Query(QueryError::Missing("active proposal ID")))
        };
        Ok(id(a)?.cmp(&id(b)?))
    }
    fn requirement_set_state(&self, state: &LoadedState, set: u64) -> Result<i32, Self::Error> {
        self.requirements
            .state(state, set)
            .map_err(Error::Requirements)
    }
    fn create_subject_requirements(
        &mut self,
        state: &mut LoadedState,
        modifier: u64,
        proposal: registry::Handle,
    ) -> Result<Option<u64>, Self::Error> {
        self.requirements
            .create_subject_set(state, modifier, proposal, &self.effects)
            .map_err(Error::Requirements)
    }
    fn apply_strength_effect(
        &mut self,
        state: &mut LoadedState,
        effect: u64,
        proposal: registry::Handle,
    ) -> Result<bool, Self::Error> {
        let context = match self.effects.effect(effect).map_err(Error::Query)? {
            EffectState::Player { context, .. } | EffectState::Alliance { context, .. } => *context,
        };
        let object = self.effects.object(context).map_err(Error::Query)?.kind;
        let entry = self
            .effects
            .effects
            .iter_mut()
            .find(|r| r.0 == effect)
            .map(|r| &mut r.1)
            .ok_or(Error::Query(QueryError::MissingEffect(effect)))?;
        match entry {
            EffectState::Player { effect, .. } => player_strength::apply(
                &mut EffectContext {
                    state,
                    object,
                    proposal,
                    inputs: &self.inputs,
                    amount: None,
                },
                effect,
            )
            .map_err(|e| Error::Player(Box::new(e))),
            EffectState::Alliance { amount, .. } => alliance_strength::apply(&mut EffectContext {
                state,
                object,
                proposal,
                inputs: &self.inputs,
                amount: Some(amount),
            })
            .map_err(|e| Error::Alliance(Box::new(e))),
        }
    }
    fn unsubscribe_subject_requirements(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        proposal: registry::Handle,
        set: u64,
    ) -> Result<(), Self::Error> {
        self.requirements
            .unsubscribe_subject_set(state, receiver, proposal, set)
            .map_err(Error::Requirements)
    }
    fn release_subject_requirements(
        &mut self,
        state: &mut LoadedState,
        set: u64,
    ) -> Result<(), Self::Error> {
        self.requirements
            .release_subject_set(state, set)
            .map_err(Error::Requirements)
    }
    fn log_applying(
        &mut self,
        _: &mut LoadedState,
        _: u64,
        _: registry::Handle,
    ) -> Result<(), Self::Error> {
        if self
            .inputs
            .logging_priority
            .ok_or(Error::Query(QueryError::Missing(
                "current logging priority",
            )))?
            < 3
        {
            Ok(())
        } else {
            Err(Error::Query(QueryError::Unsupported(
                "enabled modifier text logger",
            )))
        }
    }
    fn log_failed(
        &mut self,
        state: &mut LoadedState,
        modifier: u64,
        proposal: registry::Handle,
    ) -> Result<(), Self::Error> {
        <Self as proposals::collection_modifiers::Services>::log_applying(
            self, state, modifier, proposal,
        )
    }
}
impl<O: loaded_upkeep::OperationReports, R: Requirements> proposals::Effects for Backend<O, R> {
    type Error = Error<O::Error, R::Error>;
    fn collection_item_changed(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        proposal: &mut registry::Handle,
        added: bool,
    ) -> Result<(), Self::Error> {
        proposals::collection_modifiers::item_changed(state, self, receiver, *proposal, added)
            .map_err(|e| Error::Modifier(Box::new(e)))
    }
}
