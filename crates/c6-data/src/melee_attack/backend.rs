//! Persistent combat callback owner. Reporting/ScriptState, requirements and
//! effect-owned totals survive every callback; proposal payloads stay in the
//! canonical LoadedState registry. This does not complete melee resolution.
use super::proposals;
pub mod allocation;
pub mod city_creation;
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
    /// Global Game wrapper; never substituted for a Plot or Player interface.
    Game,
    Player(i32),
    /// Objects::Player subtype3, subvalue Policy definition type hash.
    Policy {
        owner: i32,
        policy: u32,
    },
    /// Objects::Player subtype2, subvalue Government definition type hash.
    Government {
        owner: i32,
        government_hash: u32,
    },
    /// Objects.Unit's IPlayer interface uses its stored CID owner, while its
    /// IUnit interface resolves the CURRENT corresponding canonical Unit.
    Unit(UnitReference),
    /// A real City object, distinct from its owner/plot or a proposal handle.
    City(ComponentId),
    /// Objects::District subtype1: this building owns its modifiers independently
    /// of the City and exposes the containing City through ICity.
    Building {
        city: ComponentId,
        district: ComponentId,
        building: u32,
    },
    /// Objects::District subtype0, the primary context of a building wrapper.
    District {
        city: ComponentId,
        district: ComponentId,
    },
    /// A real plot object; interface roles remain source-qualified by services.
    Plot(u32),
    Alliance {
        first_player: i32,
        second_player: i32,
    },
}
impl ContextObject {
    pub(crate) fn city_component(self) -> Option<ComponentId> {
        match self {
            Self::City(city) | Self::Building { city, .. } | Self::District { city, .. } => {
                Some(city)
            }
            _ => None,
        }
    }
}
/// Typed native City effect parameters. The common modifier runtime controls
/// application counts/lifetime; these do not create another subject registry.
#[derive(Clone, Debug)]
pub enum CityEffect {
    /// Retained class identity whose concrete gameplay leaf is unavailable.
    Unsupported {
        native_type: u64,
    },
    FeatureProduction(c6_core::found_city::feature_production::Effect),
    IdentityPerTurn {
        amount: i32,
    },
    /// AdjustCityInnerDefense2347210/2347230 delegates to the add-only RET
    /// setter2420270; no report, query or RNG callback accompanies the write.
    InnerDefense {
        amount: i32,
    },
    YieldChanges {
        yields: Vec<u32>,
        amounts: Vec<i32>,
    },
}
#[derive(Debug)]
pub enum CityEffectError {
    Unsupported { native_type: u64 },
    Query(QueryError),
    FeatureProduction(c6_core::found_city::feature_production::Error),
    Production(crate::city_production_turn::Error),
}
impl CityEffect {
    /// Invoke only after the shared lifecycle resolves the actual ICity subject.
    /// Cloning parameters before this call releases the store borrow, allowing
    /// synchronous reports to reenter the same object/modifier owner.
    pub fn apply(
        &self,
        state: &mut LoadedState,
        city: ComponentId,
        features: &[u32],
        remove: bool,
        effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<bool, CityEffectError> {
        if city.component_type != 2 {
            return Err(CityEffectError::Query(QueryError::Unsupported(
                "source-qualified ICity subject",
            )));
        }
        match self {
            Self::Unsupported { native_type } => Err(CityEffectError::Unsupported {
                native_type: *native_type,
            }),
            Self::FeatureProduction(effect) => c6_core::found_city::feature_production::apply(
                state, city, features, *effect, remove,
            )
            .map_err(CityEffectError::FeatureProduction),
            Self::InnerDefense { amount } => {
                let combat = &mut state
                    .city_mut(i32::from(city.owner), city.id)
                    .map_err(|e| {
                        CityEffectError::Production(crate::city_production_turn::Error::State(e))
                    })?
                    .combat;
                combat.inner_defense_strength_modifier = combat
                    .inner_defense_strength_modifier
                    .wrapping_add(if remove {
                        amount.wrapping_neg()
                    } else {
                        *amount
                    });
                Ok(true)
            }
            Self::IdentityPerTurn { amount } => {
                crate::city_production_turn::modifier_effects::adjust_identity_per_turn(
                    state,
                    i32::from(city.owner),
                    city.id,
                    *amount,
                    remove,
                    effects,
                )
                .map_err(CityEffectError::Production)?;
                Ok(true)
            }
            Self::YieldChanges { yields, amounts } => {
                crate::city_production_turn::modifier_effects::adjust_yields(
                    state,
                    i32::from(city.owner),
                    city.id,
                    yields,
                    amounts,
                    remove,
                    effects,
                )
                .map_err(CityEffectError::Production)?;
                Ok(true)
            }
        }
    }
}
#[derive(Debug)]
pub enum EffectState {
    Unit(crate::unit_kill::effects::Effect),
    City {
        context: u64,
        effect: CityEffect,
    },
    Player {
        context: u64,
        effect: player_strength::Effect,
    },
    Alliance {
        context: u64,
        amount: i32,
    },
    GovernmentBonus {
        context: u64,
        bonus: Option<u32>,
        amount: i32,
    },
}
#[derive(Debug)]
pub struct Object {
    pub kind: ContextObject,
    pub instance_id: i32,
    pub unit_sort_suffix: Option<[i32; 2]>,
    /// Unknown until supplied from the same initial object owner.
    pub unit_lifecycle: Option<c6_core::unit_kill::objects::Object>,
}
pub struct EffectRuntime {
    origin: c6_core::loaded_state::ActionStamp,
    binding: WireBinding,
    witness: ObservedSnapshotWitness,
    effects: Vec<(u64, EffectState)>,
    objects: Vec<(u64, Object)>,
    unit_modifiers: Option<Vec<c6_core::unit_kill::objects::Modifier>>,
    city_creation: Option<city_creation::Creation>,
    allocations: allocation::State,
}
#[derive(Debug)]
pub enum QueryError {
    Binding,
    AlreadyAdvanced,
    DuplicateEffect(u64),
    DuplicateObject(u64),
    DuplicateObjectInstance(i32),
    DuplicateModifierInstance(i32),
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
            origin: state.action_stamp(),
            binding: state.binding().clone(),
            witness: witness.clone(),
            effects,
            objects,
            unit_modifiers: None,
            city_creation: None,
            allocations: allocation::State::from_initial(state),
        })
    }
    fn current_owner(&self, state: &LoadedState) -> Result<(), QueryError> {
        if !self.origin.same_instance(state.action_stamp())
            || self.binding != *state.binding()
            || state.observed_snapshot() != Some(&self.witness)
        {
            return Err(QueryError::Binding);
        }
        Ok(())
    }
    /// Register a source-created live object in the existing owner. The caller
    /// supplies its real logical identity/instance ID from the shared factory;
    /// this method never invents an allocator or reimports an entire fixture.
    pub fn register_current_object(
        &mut self,
        state: &LoadedState,
        identity: u64,
        object: Object,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        if identity == 0 || self.objects.iter().any(|row| row.0 == identity) {
            return Err(QueryError::DuplicateObject(identity));
        }
        match object.kind {
            ContextObject::Unit(unit) => {
                if unit.component_id().component_type != 1 {
                    return Err(QueryError::Unsupported("Unit object component type"));
                }
                let suffix = object
                    .unit_sort_suffix
                    .ok_or(QueryError::Missing("Unit object sort suffix"))?;
                if !matches!(suffix[0], 0 | 1) {
                    return Err(QueryError::Unsupported("new Unit object subtype"));
                }
                let current = crate::melee_attack::actor(state, unit)
                    .map_err(|_| QueryError::Missing("current registered Unit"))?;
                if current.id != unit.id || current.e_owner != unit.player() {
                    return Err(QueryError::Missing("current registered Unit full identity"));
                }
            }
            ContextObject::City(city) => {
                if city.component_type != 2 {
                    return Err(QueryError::Unsupported("City object component type"));
                }
                state
                    .city(i32::from(city.owner), city.id)
                    .map_err(|_| QueryError::Missing("current registered City"))?;
            }
            ContextObject::Building { city, district, .. }
            | ContextObject::District { city, district } => {
                if city.component_type != 2
                    || district.component_type != 3
                    || city.owner != district.owner
                {
                    return Err(QueryError::Unsupported(
                        "Building object component identity",
                    ));
                }
                let c = state
                    .city(i32::from(city.owner), city.id)
                    .map_err(|_| QueryError::Missing("current Building City"))?;
                state
                    .district(i32::from(district.owner), district.id)
                    .map_err(|_| QueryError::Missing("current Building District"))?;
                if !c.districts.district_ids.contains(&district.id) {
                    return Err(QueryError::Missing(
                        "current Building District/City ownership",
                    ));
                }
            }
            ContextObject::Policy { owner, .. } | ContextObject::Government { owner, .. } => {
                state
                    .world()
                    .valid_player(owner)
                    .map_err(QueryError::World)?
                    .ok_or(QueryError::Missing("current Policy wrapper Player"))?;
            }
            ContextObject::Plot(plot) => {
                state
                    .world()
                    .map()
                    .plot_by_index(plot)
                    .ok_or(QueryError::Missing("current registered Plot"))?;
            }
            _ => return Err(QueryError::Unsupported("new object class factory")),
        }
        self.objects
            .try_reserve(1)
            .map_err(|_| QueryError::Missing("object registration allocation"))?;
        self.objects.push((identity, object));
        Ok(())
    }
    pub fn register_unit_effect(
        &mut self,
        state: &LoadedState,
        identity: u64,
        effect: crate::unit_kill::effects::Effect,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        if identity == 0 || self.effects.iter().any(|row| row.0 == identity) {
            return Err(QueryError::DuplicateEffect(identity));
        }
        self.effects.push((identity, EffectState::Unit(effect)));
        Ok(())
    }
    /// AdjustPlayerStrengthModifier created by a Unit ability. Context remains
    /// the ability wrapper; combat uses its IPlayer and IUnit interfaces.
    pub fn register_player_strength_effect(
        &mut self,
        state: &LoadedState,
        identity: u64,
        context: u64,
        effect: player_strength::Effect,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        self.object(context)?;
        if identity == 0 || self.effects.iter().any(|row| row.0 == identity) {
            return Err(QueryError::DuplicateEffect(identity));
        }
        self.effects
            .push((identity, EffectState::Player { context, effect }));
        Ok(())
    }
    pub fn register_unit_modifier(
        &mut self,
        state: &LoadedState,
        modifier: c6_core::unit_kill::objects::Modifier,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        self.object(modifier.context)?;
        self.effect(modifier.effect)?;
        let records = self
            .unit_modifiers
            .as_mut()
            .ok_or(QueryError::Missing("current Unit modifier owner"))?;
        if records
            .iter()
            .any(|row| row.receiver == modifier.receiver || row.instance == modifier.instance)
        {
            return Err(QueryError::DuplicateEffect(modifier.receiver));
        }
        records.push(modifier);
        Ok(())
    }
    pub fn register_city_effect(
        &mut self,
        state: &LoadedState,
        identity: u64,
        context: u64,
        effect: CityEffect,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        if identity == 0 || self.effects.iter().any(|row| row.0 == identity) {
            return Err(QueryError::DuplicateEffect(identity));
        }
        self.object(context)?;
        self.effects
            .try_reserve(1)
            .map_err(|_| QueryError::Missing("effect registration allocation"))?;
        self.effects
            .push((identity, EffectState::City { context, effect }));
        Ok(())
    }
    pub fn city_effect(&self, identity: u64) -> Result<(u64, CityEffect), QueryError> {
        match self.effect(identity)? {
            EffectState::City { context, effect } => Ok((*context, effect.clone())),
            _ => Err(QueryError::Unsupported(
                "non-City effect in City dispatcher",
            )),
        }
    }
    /// Attach current Unit-wrapper cleanup state to the same canonical object
    /// map. Shared handles must have identical immutable identity metadata.
    pub fn with_unit_cleanup(
        mut self,
        objects: Vec<(u64, Object)>,
        modifiers: Vec<c6_core::unit_kill::objects::Modifier>,
        effects: Vec<(u64, crate::unit_kill::effects::Effect)>,
    ) -> Result<Self, QueryError> {
        if self.unit_modifiers.is_some() {
            return Err(QueryError::Unsupported("Unit cleanup already initialized"));
        }
        for (index, modifier) in modifiers.iter().enumerate() {
            if modifiers[..index]
                .iter()
                .any(|m| m.receiver == modifier.receiver)
            {
                return Err(QueryError::DuplicateEffect(modifier.receiver));
            }
        }
        for (handle, object) in objects {
            if let Some((_, current)) = self.objects.iter_mut().find(|r| r.0 == handle) {
                if current.kind != object.kind
                    || current.instance_id != object.instance_id
                    || current.unit_sort_suffix != object.unit_sort_suffix
                    || current.unit_lifecycle.is_some()
                {
                    return Err(QueryError::DuplicateObject(handle));
                }
                current.unit_lifecycle = object.unit_lifecycle;
            } else {
                self.objects.push((handle, object));
            }
        }
        for (handle, effect) in effects {
            if self.effects.iter().any(|row| row.0 == handle) {
                return Err(QueryError::DuplicateEffect(handle));
            }
            self.effects.push((handle, EffectState::Unit(effect)));
        }
        self.unit_modifiers = Some(modifiers);
        Ok(self)
    }
    pub fn objects(&self) -> &[(u64, Object)] {
        &self.objects
    }
    pub fn object_mut(&mut self, identity: u64) -> Result<&mut Object, QueryError> {
        self.objects
            .iter_mut()
            .find(|row| row.0 == identity)
            .map(|row| &mut row.1)
            .ok_or(QueryError::MissingObject(identity))
    }
    pub fn unit_modifier(
        &self,
        receiver: u64,
    ) -> Result<&c6_core::unit_kill::objects::Modifier, QueryError> {
        self.unit_modifiers
            .as_ref()
            .ok_or(QueryError::Missing("current Unit modifier cleanup state"))?
            .iter()
            .find(|m| m.receiver == receiver)
            .ok_or(QueryError::MissingEffect(receiver))
    }
    pub fn unit_modifier_mut(
        &mut self,
        receiver: u64,
    ) -> Result<&mut c6_core::unit_kill::objects::Modifier, QueryError> {
        self.unit_modifiers
            .as_mut()
            .ok_or(QueryError::Missing("current Unit modifier cleanup state"))?
            .iter_mut()
            .find(|m| m.receiver == receiver)
            .ok_or(QueryError::MissingEffect(receiver))
    }
    pub fn remove_unit_modifier_registration(
        &mut self,
        receiver: u64,
    ) -> Result<c6_core::unit_kill::objects::Modifier, QueryError> {
        let modifiers = self
            .unit_modifiers
            .as_mut()
            .ok_or(QueryError::Missing("current Unit modifier cleanup state"))?;
        let index = modifiers
            .iter()
            .position(|m| m.receiver == receiver)
            .ok_or(QueryError::MissingEffect(receiver))?;
        Ok(modifiers.remove(index))
    }
    /// Release a concrete effect after its owning DynamicModifier is destroyed.
    pub fn retire_effect(&mut self, state: &LoadedState, identity: u64) -> Result<(), QueryError> {
        self.current_owner(state)?;
        let at = self
            .effects
            .iter()
            .position(|r| r.0 == identity)
            .ok_or(QueryError::MissingEffect(identity))?;
        if self
            .unit_modifiers
            .as_ref()
            .is_some_and(|v| v.iter().any(|m| m.effect == identity))
        {
            return Err(QueryError::Unsupported(
                "effect retirement before modifier destruction",
            ));
        }
        self.effects.remove(at);
        Ok(())
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
    /// Borrow the cached immutable configuration and calendar definitions.
    pub fn inputs(&self) -> &Inputs {
        &self.inputs
    }
    /// Lend a different reporting view while retaining the exact requirement,
    /// effect and configuration owners. No initialization/admission is replayed.
    pub fn with_reporting<N>(self, reporting: N) -> (O, Backend<N, R>) {
        (
            self.reporting,
            Backend {
                reporting,
                requirements: self.requirements,
                effects: self.effects,
                inputs: self.inputs,
            },
        )
    }
    /// Return the SAME persistent owners after a borrowed reporting scope.
    /// This supports an outer operation lending its provider for one attack;
    /// no requirement/effect/Script state is rebuilt between callbacks.
    pub fn into_parts(self) -> (O, R, EffectRuntime, Inputs) {
        (self.reporting, self.requirements, self.effects, self.inputs)
    }

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
    /// Reborrow already initialized canonical owners after an action. Unlike
    /// `new`, this never refreshes initial inputs or requires a pristine world.
    /// A rejected transfer returns every owner unchanged to its caller.
    #[allow(clippy::too_many_arguments, clippy::result_large_err)]
    pub(crate) fn resume_owned(
        state: &LoadedState,
        origin: c6_core::loaded_state::ActionStamp,
        reporting: O,
        requirements: R,
        effects: EffectRuntime,
        inputs: Inputs,
    ) -> Result<Self, Self> {
        let backend = Self {
            reporting,
            requirements,
            effects,
            inputs,
        };
        if !origin.same_instance(state.action_stamp())
            || backend.effects.binding != *state.binding()
            || state.observed_snapshot() != Some(&backend.effects.witness)
        {
            return Err(backend);
        }
        Ok(backend)
    }

    /// Borrow the persistent provider and lifecycle owners together; no state is
    /// cloned or rebuilt for nested death/ability callbacks.
    pub fn kill_parts(&mut self) -> (&mut O, &mut EffectRuntime, &mut R, &Inputs) {
        (
            &mut self.reporting,
            &mut self.effects,
            &mut self.requirements,
            &self.inputs,
        )
    }
    pub fn effects(&self) -> &EffectRuntime {
        &self.effects
    }
    pub fn effects_mut(&mut self) -> &mut EffectRuntime {
        &mut self.effects
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
            ContextObject::Player(owner)
            | ContextObject::Policy { owner, .. }
            | ContextObject::Government { owner, .. } => owner,
            ContextObject::Unit(u) => i32::from(u.component_id().owner),
            ContextObject::Building { district, .. } | ContextObject::District { district, .. } => {
                i32::from(district.owner)
            }
            ContextObject::Alliance { .. } => return Ok(None),
            ContextObject::Game | ContextObject::City(_) | ContextObject::Plot(_) => {
                return Err(QueryError::Unsupported(
                    "new context IPlayer query requires source-qualified adapter",
                ))
            }
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
        Ok(configured_end_turn(
            self.inputs
                .turn_limit_type
                .ok_or(QueryError::Missing("current TurnLimitType"))?,
            start,
            maximum,
        ))
    }
    fn game_start_turn(&mut self) -> Result<i32, QueryError> {
        self.start_turn()
    }
    fn unit_property(&mut self, _: u32) -> Result<Option<i32>, QueryError> {
        if matches!(
            self.object,
            ContextObject::Game | ContextObject::City(_) | ContextObject::Plot(_)
        ) {
            return Err(QueryError::Unsupported(
                "new context IUnit query requires source-qualified adapter",
            ));
        }
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
        if matches!(
            self.object,
            ContextObject::Game | ContextObject::City(_) | ContextObject::Plot(_)
        ) {
            return Err(QueryError::Unsupported(
                "new context IGame query requires source-qualified adapter",
            ));
        }
        Ok(true)
    }
    fn game_property(&mut self, _: u32) -> Result<Option<i32>, QueryError> {
        Err(QueryError::Unsupported("current Game property conversion"))
    }
    fn ever_alive_majors(&mut self) -> Result<i32, QueryError> {
        Err(QueryError::Unsupported("current ever-alive major roster"))
    }
}
fn configured_end_turn(limit: u32, start: i32, maximum: i32) -> i32 {
    match limit {
        u32::MAX | 0x8099_25de => 0,
        0xdef9_f0af => maximum.wrapping_add(1),
        _ => start.wrapping_add(maximum),
    }
}

impl alliance_strength::Context for EffectContext<'_> {
    fn context_alliance_interface(&mut self) -> Result<bool, QueryError> {
        if matches!(
            self.object,
            ContextObject::Game | ContextObject::City(_) | ContextObject::Plot(_)
        ) {
            return Err(QueryError::Unsupported(
                "new context IAlliance query requires source-qualified adapter",
            ));
        }
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
                .object_id(h)
                .map(|id| id as i32)
                .map_err(|e| Error::Query(QueryError::Proposal(e)))
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
            EffectState::City { .. } | EffectState::GovernmentBonus { .. } => {
                return Err(Error::Query(QueryError::Unsupported(
                    "City effect in strength dispatcher",
                )))
            }
            EffectState::Unit(_) => {
                return Err(Error::Query(QueryError::Unsupported(
                    "Unit effect in strength Apply dispatcher",
                )))
            }
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
            EffectState::City { .. } | EffectState::GovernmentBonus { .. } => Err(Error::Query(
                QueryError::Unsupported("City effect in strength dispatcher"),
            )),
            EffectState::Unit(_) => Err(Error::Query(QueryError::Unsupported(
                "Unit effect in strength Apply dispatcher",
            ))),
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

/// Positive post-combat credits use the same reporting/gameplay owner that
/// executed damage and death. There is no second provider or default credit.
pub trait PostCombatYields: loaded_upkeep::OperationReports {
    fn credit_post_combat_yield(
        &mut self,
        state: &mut LoadedState,
        winner: UnitReference,
        yield_type: i32,
        amount: i32,
    ) -> Result<(), Self::Error>;
}
impl<O: PostCombatYields, R: Requirements> super::resolve::post_yields::Effects for Backend<O, R> {
    fn credit_post_combat_yield(
        &mut self,
        state: &mut LoadedState,
        winner: UnitReference,
        yield_type: i32,
        amount: i32,
    ) -> Result<(), <Self as proposals::Effects>::Error> {
        self.reporting
            .credit_post_combat_yield(state, winner, yield_type, amount)
            .map_err(Error::Reporting)
    }
}

#[cfg(test)]
mod tests {
    use super::configured_end_turn;
    #[test]
    fn end_turn_distinguishes_score_limit_from_enabled_turn_limit() {
        assert_eq!(configured_end_turn(0x8099_25de, 50, 0), 0);
        assert_eq!(configured_end_turn(u32::MAX, 50, 500), 0);
        assert_eq!(configured_end_turn(0xdef9_f0af, 50, 500), 501);
        assert_eq!(configured_end_turn(123, 50, 500), 550);
        assert_eq!(configured_end_turn(0xdef9_f0af, 0, i32::MAX), i32::MIN);
        assert_eq!(configured_end_turn(123, i32::MAX, 1), i32::MIN);
    }
}
