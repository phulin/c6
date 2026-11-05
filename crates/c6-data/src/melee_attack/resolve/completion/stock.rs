//! Ordered stock combat subscribers over persistent signal/runtime owners.
//! Native addresses and capture qualification belong to the input adapter.
use super::super::GeneratedMembers;
pub mod unit_district;
use super::{boosts, eras, history, mayhem, stock_guards, targets};
use crate::unit_placement_state;
use c6_core::{
    core_player::initialization::WireBinding,
    game_capabilities::Membership,
    loaded_state::{ActionStamp, LoadedState},
    map::UnitReference,
    melee_attack::signals::{self, Scope as Signal},
    unit_moved_signal::{ListenerRing, RegistryError, Subscription},
};
pub use unit_district::{
    combat_occurred as unit_district_combat_occurred, Effects as UnitDistrictEffects,
};
#[derive(Clone, Copy, Debug)]
pub enum Callback {
    Mayhem,
    Summary,
    GeneralSummary,
    Lua,
    EmergencyCity,
    EmergencyKill,
    Eras,
    History,
    Boost(boosts::Callback),
    Targets(i32),
    FreeCities,
    Barbarian,
    Achievement,
    Attacked(i32),
    Unsupported(u64),
}
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    /// Source stock summary/platform output is presentation-only. History report
    /// cache/translator groups have no synchronous gameplay consumers in this
    /// domain; emit the typed event instead of running native UI cache machinery.
    StockHeadlessNoAddedConsumers,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Output {
    CombatSummary,
    GeneralPlayerSummary,
    CombatAchievementObservation,
    HistoricMoment { id: i32, kind: u32 },
}
#[derive(Debug)]
pub enum BindingError {
    DuplicateReceiver(u64),
    DuplicatePlayer(i32),
    UnknownPlayer(i32),
    MissingPlayerSignal(i32),
    Ring(RegistryError),
    Allocation,
    DuplicateEmergencyScoring,
}
pub struct Bindings {
    origin: ActionStamp,
    binding: WireBinding,
    scope: Scope,
    global: ListenerRing<u64>,
    players: Vec<(i32, ListenerRing<u64>)>,
    callbacks: Vec<(u64, Callback)>,
    history: Vec<history::RecordedListener>,
    outputs: Vec<Output>,
    // Current manager list, bound explicitly by the initialized input adapter.
    // The supported owner has no instances; instance creation is unsupported.
    emergency_scoring: Option<stock_guards::EmptyEmergencyScoring>,
}
impl Bindings {
    pub fn new(
        state: &LoadedState,
        scope: Scope,
        global: Vec<(u64, Callback)>,
        players: Vec<(i32, Vec<(u64, Callback)>)>,
        history: Vec<history::RecordedListener>,
    ) -> Result<Self, BindingError> {
        let mut callbacks = Vec::new();
        let mut make = |rows: Vec<(u64, Callback)>| -> Result<ListenerRing<u64>, BindingError> {
            let mut ordered = Vec::new();
            for (label, callback) in rows {
                if callbacks.iter().any(|r: &(u64, Callback)| r.0 == label) {
                    return Err(BindingError::DuplicateReceiver(label));
                }
                callbacks.push((label, callback));
                ordered.push(label);
            }
            ListenerRing::from_ordered(&ordered).map_err(BindingError::Ring)
        };
        let global = make(global)?;
        let mut player_rings = Vec::new();
        for (owner, rows) in players {
            if player_rings
                .iter()
                .any(|r: &(i32, ListenerRing<u64>)| r.0 == owner)
            {
                return Err(BindingError::DuplicatePlayer(owner));
            }
            player_rings.push((owner, make(rows)?));
        }
        Ok(Self {
            origin: state.action_stamp(),
            binding: state.binding().clone(),
            scope,
            global,
            players: player_rings,
            callbacks,
            history,
            outputs: Vec::new(),
            emergency_scoring: None,
        })
    }
    /// Supply an actually empty current Emergency manager. This owner persists
    /// with the callback runtime; callers must not use it for an unknown list.
    pub fn initialize_empty_emergency_scoring(&mut self) -> Result<(), BindingError> {
        if self.emergency_scoring.is_some() {
            return Err(BindingError::DuplicateEmergencyScoring);
        }
        self.emergency_scoring = Some(stock_guards::EmptyEmergencyScoring);
        Ok(())
    }
    pub fn outputs(&self) -> &[Output] {
        &self.outputs
    }
    pub fn take_outputs(&mut self) -> Vec<Output> {
        std::mem::take(&mut self.outputs)
    }
    pub fn history_listeners_mut(&mut self) -> &mut Vec<history::RecordedListener> {
        &mut self.history
    }
    pub fn listeners_mut(&mut self, scope: Signal) -> Result<&mut ListenerRing<u64>, BindingError> {
        match scope {
            Signal::GameCombatOccurred => Ok(&mut self.global),
            Signal::PlayerAttacked(owner) => self
                .players
                .iter_mut()
                .find(|r| r.0 == owner)
                .map(|r| &mut r.1)
                .ok_or(BindingError::MissingPlayerSignal(owner)),
        }
    }
    /// Register metadata and append to the SAME active ring. Held cursors retain
    /// source append/unsubscribe/reentry behavior through the Core signal kernel.
    pub fn subscribe(
        &mut self,
        scope: Signal,
        label: u64,
        callback: Callback,
    ) -> Result<Subscription, BindingError> {
        if self.callbacks.iter().any(|r| r.0 == label) {
            return Err(BindingError::DuplicateReceiver(label));
        }
        self.callbacks
            .try_reserve(1)
            .map_err(|_| BindingError::Allocation)?;
        let token = self
            .listeners_mut(scope)?
            .subscribe(label)
            .map_err(BindingError::Ring)?;
        self.callbacks.push((label, callback));
        Ok(token)
    }
    pub fn unsubscribe(&mut self, scope: Signal, token: Subscription) -> Result<(), BindingError> {
        self.listeners_mut(scope)?
            .unsubscribe(token)
            .map_err(BindingError::Ring)
    }
    fn matches(&self, state: &LoadedState) -> bool {
        self.origin.same_instance(state.action_stamp()) && self.binding == *state.binding()
    }
}
pub struct Inputs<'a, 'd, 'r> {
    pub placement: &'a unit_placement_state::Definitions<'d, 'r>,
    pub civilization_levels: &'a [(i32, u32)],
    pub capabilities: &'a Membership,
    pub mayhem: &'a mayhem::Definitions,
    pub targets: &'a targets::Definitions,
    pub boosts: &'a boosts::Definitions,
    pub history: &'a history::Definitions,
    pub start_era_hash: Option<u32>,
    pub barbarian_owner: i32,
}
/// Callers forward these operations to their SAME persistent Script, research,
/// notification and operation providers. Only reached continuations execute.
pub trait Effects {
    type Error;
    fn clear_defender_operations(
        &mut self,
        state: &mut LoadedState,
        bindings: &mut Bindings,
        defender: UnitReference,
    ) -> Result<(), Self::Error>;
    fn set_defender_awake(
        &mut self,
        state: &mut LoadedState,
        bindings: &mut Bindings,
        defender: UnitReference,
    ) -> Result<(), Self::Error>;
    fn lua_combat_occurred(
        &mut self,
        state: &mut LoadedState,
        bindings: &mut Bindings,
        members: &GeneratedMembers,
    ) -> Result<(), Self::Error>;
    fn trigger_boost(
        &mut self,
        state: &mut LoadedState,
        bindings: &mut Bindings,
        owner: i32,
        index: usize,
    ) -> Result<(), Self::Error>;
    fn change_era_score(
        &mut self,
        state: &mut LoadedState,
        bindings: &mut Bindings,
        owner: i32,
        amount: i32,
        reason: i32,
    ) -> Result<(), Self::Error>;
    fn era_score_notice(
        &mut self,
        state: &mut LoadedState,
        bindings: &mut Bindings,
        moment: &c6_core::game_history::Moment,
    ) -> Result<(), Self::Error>;
    fn adjust_history_cooldown(
        &mut self,
        _: &LoadedState,
        _: &mut Bindings,
        _: i32,
    ) -> Result<Option<i32>, Self::Error> {
        Ok(None)
    }
}
#[derive(Debug)]
pub enum HistoryError<E> {
    Effect(E),
    Recorded(history::RecordedError),
}
#[derive(Debug)]
pub enum Error<E> {
    DistrictContinuation(&'static str),
    Binding,
    Bindings(BindingError),
    MissingPayload,
    UnknownCallback(u64),
    WrongSignal(u64),
    UnsupportedCallback(u64),
    Mayhem(mayhem::Error),
    Targets(targets::Error),
    Eras(eras::Error),
    Guard(stock_guards::Error),
    Boost(boosts::Error<E>),
    History(history::Error<HistoryError<E>>),
    Effect(E),
    Signal(signals::Error<Box<Error<E>>>),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock combat callbacks: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
struct Boost<'a, E> {
    bindings: &'a mut Bindings,
    effects: &'a mut E,
}
impl<E: Effects> boosts::Trigger for Boost<'_, E> {
    type Error = E::Error;
    fn trigger(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        index: usize,
    ) -> Result<(), Self::Error> {
        self.effects
            .trigger_boost(state, self.bindings, owner, index)
    }
}
struct History<'a, E> {
    bindings: &'a mut Bindings,
    effects: &'a mut E,
}
impl<E: Effects> history::Effects for History<'_, E> {
    type Error = HistoryError<E::Error>;
    fn moment_recorded(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        kind: u32,
        _: i32,
    ) -> Result<(), Self::Error> {
        history::stock_recorded(&self.bindings.history, kind).map_err(HistoryError::Recorded)
    }
    fn report(&mut self, _: &mut LoadedState, id: i32, kind: u32) -> Result<(), Self::Error> {
        match self.bindings.scope {
            Scope::StockHeadlessNoAddedConsumers => self
                .bindings
                .outputs
                .push(Output::HistoricMoment { id, kind }),
        };
        Ok(())
    }
    fn change_era_score(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        amount: i32,
        reason: i32,
    ) -> Result<(), Self::Error> {
        self.effects
            .change_era_score(state, self.bindings, owner, amount, reason)
            .map_err(HistoryError::Effect)
    }
    fn era_score_notice(
        &mut self,
        state: &mut LoadedState,
        moment: &c6_core::game_history::Moment,
    ) -> Result<(), Self::Error> {
        self.effects
            .era_score_notice(state, self.bindings, moment)
            .map_err(HistoryError::Effect)
    }
    fn adjust_cooldown(
        &mut self,
        state: &LoadedState,
        value: i32,
    ) -> Result<Option<i32>, Self::Error> {
        self.effects
            .adjust_history_cooldown(state, self.bindings, value)
            .map_err(HistoryError::Effect)
    }
}
struct Dispatch<'a, 'i, 'd, 'r, E> {
    state: &'a mut LoadedState,
    bindings: &'a mut Bindings,
    inputs: &'a Inputs<'i, 'd, 'r>,
    effects: &'a mut E,
    members: Option<&'a GeneratedMembers>,
    attacker: Option<UnitReference>,
}
impl<E: Effects> Dispatch<'_, '_, '_, '_, E> {
    fn call(&mut self, scope: Signal, id: u64) -> Result<(), Error<E::Error>> {
        let callback = self
            .bindings
            .callbacks
            .iter()
            .find(|r| r.0 == id)
            .ok_or(Error::UnknownCallback(id))?
            .1;
        if let Callback::Unsupported(value) = callback {
            return Err(Error::UnsupportedCallback(value));
        }
        if let Callback::Attacked(owner) = callback {
            if !matches!(scope, Signal::PlayerAttacked(_)) {
                return Err(Error::WrongSignal(id));
            }
            return targets::observer_attacked(
                self.state,
                self.inputs.targets,
                owner,
                self.attacker.ok_or(Error::MissingPayload)?,
            )
            .map_err(Error::Targets);
        }
        if scope != Signal::GameCombatOccurred {
            return Err(Error::WrongSignal(id));
        }
        let members = self.members.ok_or(Error::MissingPayload)?;
        match callback {
            Callback::Mayhem => mayhem::on_combat(
                self.state,
                self.inputs.mayhem,
                self.inputs.placement,
                self.inputs.barbarian_owner,
                members,
            )
            .map_err(Error::Mayhem)?,
            Callback::Summary => self.bindings.outputs.push(Output::CombatSummary),
            Callback::GeneralSummary => self.bindings.outputs.push(Output::GeneralPlayerSummary),
            Callback::Lua => self
                .effects
                .lua_combat_occurred(self.state, self.bindings, members)
                .map_err(Error::Effect)?,
            Callback::EmergencyCity => stock_guards::emergency_attacked_city(members),
            Callback::EmergencyKill => stock_guards::emergency_killed_enemy(
                self.state,
                members,
                self.bindings.emergency_scoring.as_ref(),
            )
            .map_err(Error::Guard)?,
            Callback::Eras => eras::on_combat(
                self.state,
                self.inputs.placement,
                self.inputs.barbarian_owner,
                members,
            )
            .map_err(Error::Eras)?,
            Callback::History => {
                history::on_combat(
                    self.state,
                    self.inputs.history,
                    history::Inputs {
                        capabilities: self.inputs.capabilities,
                        civilization_levels: self.inputs.civilization_levels,
                        start_era_hash: self.inputs.start_era_hash,
                    },
                    members,
                    &mut History {
                        bindings: self.bindings,
                        effects: self.effects,
                    },
                )
                .map_err(Error::History)?;
            }
            Callback::Boost(callback) => boosts::on_combat(
                self.state,
                &mut Boost {
                    bindings: self.bindings,
                    effects: self.effects,
                },
                self.inputs.boosts,
                boosts::Inputs {
                    maximum_damage: self.inputs.placement.movement,
                    civilization_levels: self.inputs.civilization_levels,
                },
                callback,
                members,
            )
            .map_err(Error::Boost)?,
            Callback::Targets(owner) => targets::unit_killed(
                self.state,
                self.inputs.targets,
                self.inputs.placement,
                owner,
                members,
            )
            .map_err(Error::Targets)?,
            Callback::FreeCities => stock_guards::free_cities_unit_combat(members),
            Callback::Barbarian => stock_guards::barbarian(
                self.state,
                self.inputs.placement,
                self.inputs.barbarian_owner,
                members,
            )
            .map_err(Error::Guard)?,
            Callback::Achievement => self
                .bindings
                .outputs
                .push(Output::CombatAchievementObservation),
            Callback::Attacked(_) | Callback::Unsupported(_) => {
                unreachable!("handled before Results query")
            }
        }
        Ok(())
    }
}
impl<E: Effects> signals::Context for Dispatch<'_, '_, '_, '_, E> {
    type Error = Box<Error<E::Error>>;
    fn listeners(&mut self, scope: Signal) -> Result<&mut ListenerRing<u64>, Self::Error> {
        self.bindings
            .listeners_mut(scope)
            .map_err(|e| Box::new(Error::Bindings(e)))
    }
    fn invoke(&mut self, scope: Signal, id: u64) -> Result<(), Self::Error> {
        self.call(scope, id).map_err(Box::new)
    }
}
pub fn combat_occurred<E: Effects>(
    state: &mut LoadedState,
    bindings: &mut Bindings,
    inputs: &Inputs<'_, '_, '_>,
    effects: &mut E,
    members: &GeneratedMembers,
) -> Result<(), Error<E::Error>> {
    if !bindings.matches(state) {
        return Err(Error::Binding);
    }
    signals::dispatch(
        &mut Dispatch {
            state,
            bindings,
            inputs,
            effects,
            members: Some(members),
            attacker: None,
        },
        Signal::GameCombatOccurred,
    )
    .map_err(Error::Signal)
}
pub fn defender_attacked<E: Effects>(
    state: &mut LoadedState,
    bindings: &mut Bindings,
    inputs: &Inputs<'_, '_, '_>,
    effects: &mut E,
    owner: i32,
    attacker: UnitReference,
) -> Result<(), Error<E::Error>> {
    if !bindings.matches(state) {
        return Err(Error::Binding);
    }
    signals::dispatch(
        &mut Dispatch {
            state,
            bindings,
            inputs,
            effects,
            members: None,
            attacker: Some(attacker),
        },
        Signal::PlayerAttacked(owner),
    )
    .map_err(Error::Signal)
}
/// Borrowed completion facade; all runtime objects in Bindings and Effects stay
/// alive across actions. This performs no state import/provider reconstruction.
pub struct Completion<'a, 'i, 'd, 'r, E> {
    pub bindings: &'a mut Bindings,
    pub inputs: &'a Inputs<'i, 'd, 'r>,
    pub effects: &'a mut E,
}
impl<E: Effects> super::Effects for Completion<'_, '_, '_, '_, E> {
    type Error = Error<E::Error>;
    fn clear_defender_operations(
        &mut self,
        state: &mut LoadedState,
        defender: UnitReference,
    ) -> Result<(), Self::Error> {
        self.effects
            .clear_defender_operations(state, self.bindings, defender)
            .map_err(Error::Effect)
    }
    fn set_defender_awake(
        &mut self,
        state: &mut LoadedState,
        defender: UnitReference,
    ) -> Result<(), Self::Error> {
        self.effects
            .set_defender_awake(state, self.bindings, defender)
            .map_err(Error::Effect)
    }
    fn combat_occurred(
        &mut self,
        state: &mut LoadedState,
        members: &GeneratedMembers,
    ) -> Result<(), Self::Error> {
        combat_occurred(state, self.bindings, self.inputs, self.effects, members)
    }
    fn defender_attacked(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        attacker: UnitReference,
    ) -> Result<(), Self::Error> {
        defender_attacked(
            state,
            self.bindings,
            self.inputs,
            self.effects,
            owner,
            attacker,
        )
    }
}

/// District Results carry no attacking Unit. Keep this callback explicit rather
/// than manufacturing a Unit participant for the Lua argument encoder.
pub trait DistrictEffects: Effects {
    fn lua_district_combat_occurred(
        &mut self,
        state: &mut LoadedState,
        bindings: &mut Bindings,
        members: &crate::city_strike::Generated,
    ) -> Result<(), Self::Error>;
}
struct DistrictDispatch<'a, 'i, 'd, 'r, E> {
    state: &'a mut LoadedState,
    bindings: &'a mut Bindings,
    inputs: &'a Inputs<'i, 'd, 'r>,
    effects: &'a mut E,
    members: &'a crate::city_strike::Generated,
}
impl<E: DistrictEffects> signals::Context for DistrictDispatch<'_, '_, '_, '_, E> {
    type Error = Box<Error<E::Error>>;
    fn listeners(&mut self, scope: Signal) -> Result<&mut ListenerRing<u64>, Self::Error> {
        self.bindings
            .listeners_mut(scope)
            .map_err(|e| Box::new(Error::Bindings(e)))
    }
    fn invoke(&mut self, scope: Signal, id: u64) -> Result<(), Self::Error> {
        self.call(scope, id).map_err(Box::new)
    }
}
impl<E: DistrictEffects> DistrictDispatch<'_, '_, '_, '_, E> {
    fn call(&mut self, scope: Signal, id: u64) -> Result<(), Error<E::Error>> {
        if scope != Signal::GameCombatOccurred {
            return Err(Error::WrongSignal(id));
        }
        let callback = self
            .bindings
            .callbacks
            .iter()
            .find(|r| r.0 == id)
            .ok_or(Error::UnknownCallback(id))?
            .1;
        match callback {
            Callback::Mayhem => mayhem::on_district_combat(
                self.state,
                self.inputs.mayhem,
                self.inputs.placement,
                self.inputs.barbarian_owner,
                self.members,
            )
            .map_err(Error::Mayhem)?,
            Callback::Summary => self.bindings.outputs.push(Output::CombatSummary),
            Callback::GeneralSummary => self.bindings.outputs.push(Output::GeneralPlayerSummary),
            Callback::Lua => self
                .effects
                .lua_district_combat_occurred(self.state, self.bindings, self.members)
                .map_err(Error::Effect)?,
            // Emergency attacked-city and FreeCities require a District DEFENDER.
            // Barbarian manager requires BOTH Units before any owner/death query.
            Callback::EmergencyCity | Callback::FreeCities | Callback::Barbarian => {}
            Callback::EmergencyKill => {
                if crate::melee_attack::actor(self.state, self.members.defender)
                    .map_err(|e| Error::Guard(stock_guards::Error::Actor(e)))?
                    .b_delayed_death
                    && self.bindings.emergency_scoring.is_none()
                {
                    return Err(Error::Guard(stock_guards::Error::EmergencyKilledEnemy));
                }
            }
            Callback::Eras => {
                // Null attacking Unit skips the first half; live defender skips
                // its second half before any era bonus lookup or mutation.
                let defender = crate::melee_attack::actor(self.state, self.members.defender)
                    .map_err(|e| Error::Eras(eras::Error::Actor(e)))?;
                if defender.i_damage
                    >= self
                        .inputs
                        .placement
                        .movement
                        .maximum_damage(defender)
                        .map_err(|e| Error::Eras(eras::Error::Maximum(e)))?
                {
                    return Err(Error::DistrictContinuation("Eras defender death"));
                }
            }
            Callback::History => {
                history::on_district_combat(
                    self.state,
                    self.inputs.history,
                    history::Inputs {
                        capabilities: self.inputs.capabilities,
                        civilization_levels: self.inputs.civilization_levels,
                        start_era_hash: self.inputs.start_era_hash,
                    },
                    self.members,
                    &mut History {
                        bindings: self.bindings,
                        effects: self.effects,
                    },
                )
                .map_err(Error::History)?;
            }
            // KillSpecificUnit requires an attacking Unit. KillWith's defender
            // half requires a killed attacking Unit; NumberBarbarianKills checks
            // live defender death after skipping its null-attacker half.
            Callback::Boost(boosts::Callback {
                check: boosts::Check::KillSpecificUnit | boosts::Check::KillWith,
                ..
            }) => {}
            Callback::Boost(_) | Callback::Targets(_) => {
                let defender = crate::melee_attack::actor(self.state, self.members.defender)
                    .map_err(|e| Error::Targets(targets::Error::Actor(e)))?;
                if defender.i_damage
                    >= self
                        .inputs
                        .placement
                        .movement
                        .maximum_damage(defender)
                        .map_err(|e| Error::Targets(targets::Error::Maximum(e)))?
                {
                    return Err(Error::DistrictContinuation("combat kill subscriber"));
                }
            }
            Callback::Achievement => self
                .bindings
                .outputs
                .push(Output::CombatAchievementObservation),
            Callback::Attacked(_) => return Err(Error::WrongSignal(id)),
            Callback::Unsupported(value) => return Err(Error::UnsupportedCallback(value)),
        }
        Ok(())
    }
}
/// Dispatch on the original persistent ring, preserving mutation/reentry rules.
pub fn district_combat_occurred<E: DistrictEffects>(
    state: &mut LoadedState,
    bindings: &mut Bindings,
    inputs: &Inputs<'_, '_, '_>,
    effects: &mut E,
    members: &crate::city_strike::Generated,
) -> Result<(), Error<E::Error>> {
    if !bindings.matches(state) {
        return Err(Error::Binding);
    }
    signals::dispatch(
        &mut DistrictDispatch {
            state,
            bindings,
            inputs,
            effects,
            members,
        },
        Signal::GameCombatOccurred,
    )
    .map_err(Error::Signal)
}
