//! Real Unit/District payload dispatch on the retained stock CombatOccurred ring.
use super::*;
use crate::melee_attack::city::Generated;

pub trait Effects: super::Effects {
    fn lua_unit_district_combat_occurred(
        &mut self,
        state: &mut LoadedState,
        bindings: &mut Bindings,
        members: &Generated,
    ) -> Result<(), Self::Error>;
}
struct Dispatch<'a, 'i, 'd, 'r, E> {
    state: &'a mut LoadedState,
    bindings: &'a mut Bindings,
    inputs: &'a Inputs<'i, 'd, 'r>,
    effects: &'a mut E,
    members: &'a Generated,
    retired: Option<&'a crate::city_capture::retired_district::RetiredDistrictScalars>,
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
impl<E: Effects> Dispatch<'_, '_, '_, '_, E> {
    fn alive(&self) -> Result<bool, Error<E::Error>> {
        let unit = crate::melee_attack::actor(self.state, self.members.attacker)
            .map_err(|e| Error::Targets(targets::Error::Actor(e)))?;
        Ok(unit.i_damage
            < self
                .inputs
                .placement
                .movement
                .maximum_damage(unit)
                .map_err(|e| Error::Targets(targets::Error::Maximum(e)))?)
    }
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
            Callback::Mayhem => mayhem::on_unit_district_combat(
                self.state,
                self.inputs.mayhem,
                self.inputs.placement,
                self.inputs.barbarian_owner,
                self.members,
                self.retired,
            )
            .map_err(Error::Mayhem)?,
            Callback::Summary => self.bindings.outputs.push(Output::CombatSummary),
            Callback::GeneralSummary => self.bindings.outputs.push(Output::GeneralPlayerSummary),
            Callback::Lua => self
                .effects
                .lua_unit_district_combat_occurred(self.state, self.bindings, self.members)
                .map_err(Error::Effect)?,
            Callback::EmergencyCity => {
                if self.bindings.emergency_scoring.is_none() {
                    return Err(Error::DistrictContinuation(
                        "current emergency city scoring manager",
                    ));
                }
            }
            // ScoreOnKilledEnemy tests the absent defending Unit first.
            Callback::EmergencyKill | Callback::Barbarian => {}
            Callback::FreeCities => {
                let level = self
                    .inputs
                    .civilization_levels
                    .iter()
                    .find(|r| r.0 == i32::from(self.members.defender.owner))
                    .ok_or(Error::DistrictContinuation("District defender level"))?
                    .1;
                if level == 0x04e6b0a0 {
                    return Err(Error::DistrictContinuation("FreeCities retaliation"));
                }
            }
            Callback::Eras | Callback::Boost(_) => {
                if !self.alive()? {
                    return Err(Error::DistrictContinuation(
                        "Unit/District attacker death subscriber",
                    ));
                }
            }
            Callback::Targets(observer) => {
                if let Some(observation) = targets::unit_district_combat(
                    self.state,
                    self.inputs.targets,
                    self.inputs.placement,
                    observer,
                    self.members,
                    self.retired,
                )
                .map_err(Error::Targets)?
                {
                    self.bindings
                        .outputs
                        .push(Output::CityAttackPlanningObservation(observation));
                }
            }
            Callback::History => {
                history::general_battle(
                    self.state,
                    self.inputs.history,
                    history::Inputs {
                        capabilities: self.inputs.capabilities,
                        civilization_levels: self.inputs.civilization_levels,
                        start_era_hash: self.inputs.start_era_hash,
                    },
                    self.members.attacker.player(),
                    i32::from(self.members.defender.owner),
                    self.members.location,
                    &mut History {
                        bindings: self.bindings,
                        effects: self.effects,
                    },
                )
                .map_err(Error::History)?;
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
pub fn combat_occurred<E: Effects>(
    state: &mut LoadedState,
    bindings: &mut Bindings,
    inputs: &Inputs<'_, '_, '_>,
    effects: &mut E,
    members: &Generated,
    retired: Option<&crate::city_capture::retired_district::RetiredDistrictScalars>,
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
            members,
            retired,
        },
        Signal::GameCombatOccurred,
    )
    .map_err(Error::Signal)
}
