//! Explicit Linux XP2 pantheon founding. Policies supply the belief; callbacks
//! remain synchronous and source ordered. Only Initiation Rites effects are
//! implemented; other legal beliefs are reported with unsupported effects.
pub mod history;
pub mod modifiers;
pub mod stock;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    Rules,
};
use c6_core::movement_world::DefinitionLookup;
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error<E = std::convert::Infallible> {
    State(c6_core::loaded_state::Error),
    Unsupported(&'static str),
    Rules(crate::Error),
    Effects(E),
    Binding,
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pantheon action: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Choice {
    pub native_type: i32,
    pub hash: u32,
    pub effects_supported: bool,
}
pub struct Definitions {
    binding: WireBinding,
    registry: LinuxDefinitionRegistry,
    choices: Vec<Choice>,
    pub(crate) rites: Option<modifiers::Definitions>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let registry =
            crate::unit_initialization_definitions::collection(rules, "Beliefs", "BeliefType")
                .map_err(|_| Error::Unsupported("Belief registry"))?;
        let rows = rules
            .gameplay()
            .table("Beliefs")
            .ok_or(Error::Unsupported("Beliefs table"))?;
        let rites = modifiers::Definitions::from_rules(rules)?;
        let mut choices = vec![];
        for (index, entry) in registry.entries().iter().enumerate() {
            let row = rows
                .rows()
                .find(|r| {
                    r.text("BeliefType")
                        .ok()
                        .is_some_and(|s| s.as_bytes() == entry.type_name)
                })
                .ok_or(Error::Unsupported("Belief row"))?;
            if row.text("BeliefClassType").map_err(Error::Rules)? == "BELIEF_CLASS_PANTHEON" {
                let hash = linux_type_hash(&entry.type_name);
                choices.push(Choice {
                    native_type: index as i32,
                    hash,
                    effects_supported: rites.as_ref().is_some_and(|d| d.belief_hash == hash),
                });
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            registry,
            choices,
            rites,
        })
    }
    fn check<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if &self.binding != state.binding() {
            return Err(Error::Binding);
        }
        Ok(())
    }
    pub fn belief_hash(&self, native_type: i32) -> Option<u32> {
        usize::try_from(native_type)
            .ok()
            .and_then(|i| self.registry.type_hash(i))
    }
}
pub struct Current<'a> {
    pub capabilities: &'a c6_core::game_capabilities::Membership,
    pub players: &'a crate::unit_placement_services::Runtime,
}
fn player<E>(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error<E>> {
    state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("current pantheon Player"))?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("current Player.Religion"))
}
pub(crate) fn alive<E>(state: &LoadedState, current: &Current<'_>) -> Result<Vec<i32>, Error<E>> {
    if &current.players.binding != state.binding() {
        return Err(Error::Binding);
    }
    let (list, count) = current
        .players
        .alive_begin::<()>()
        .map_err(|_| Error::Unsupported("current GetAlive"))?;
    (0..count)
        .map(|i| {
            current
                .players
                .alive_at::<()>(list, i)
                .map_err(|_| Error::Unsupported("GetAlive order"))
        })
        .collect()
}
/// Same eligibility as CanCreatePantheon(true), without restricting decisions
/// to humans. A created religion is conservatively explicit until its identity
/// resolves; no unknown record is interpreted as no religion.
pub fn can_create<E>(
    state: &LoadedState,
    defs: &Definitions,
    current: &Current<'_>,
    owner: i32,
) -> Result<bool, Error<E>> {
    defs.check(state)?;
    let p = player(state, owner)?;
    if !current.capabilities.has(0xb9fc347c) || p.religion.pantheon_hash != u32::MAX {
        return Ok(false);
    }
    if state
        .retained()
        .religion
        .records
        .iter()
        .any(|r| r.founder == owner)
    {
        return Err(Error::Unsupported("created religion eligibility"));
    }
    Ok(state.retained().religion.next_pantheon_founder == owner
        && p.religion.faith_balance_q8
            >= state
                .retained()
                .religion
                .minimum_faith_for_next_pantheon
                .wrapping_shl(8))
}
pub fn available<E>(
    state: &LoadedState,
    defs: &Definitions,
    current: &Current<'_>,
    owner: i32,
) -> Result<Vec<Choice>, Error<E>> {
    if !can_create(state, defs, current, owner)? {
        return Ok(vec![]);
    }
    let taken = alive(state, current)?
        .into_iter()
        .map(|p| player(state, p).map(|p| p.religion.pantheon_hash))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(defs
        .choices
        .iter()
        .copied()
        .filter(|c| !taken.contains(&c.hash))
        .collect())
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    BeliefChosen { belief: i32 },
    FaithChanged { balance_q8: i32 },
    CityPantheonChanged { city: u32, old: u32, new: u32 },
    ReportFounded { belief: i32 },
    PantheonFounded { belief: i32 },
    ReportCity { city: u32 },
}
pub trait Effects {
    type Error: std::fmt::Debug;
    /// Must execute the reached gameplay/RNG consumers, including attaching
    /// modifiers on BeliefChosen. Presentation publication may be omitted only
    /// by a qualified concrete provider, never by a successful default.
    fn event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        event: Event,
        defs: &Definitions,
    ) -> Result<(), Self::Error>;
}
/// Validates before mutation. Callback errors preserve the actual changed
/// prefix; callers must retain failure rather than issue another action.
pub fn found<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    current: &Current<'_>,
    owner: i32,
    belief: i32,
    effects: &mut E,
) -> Result<bool, Error<E::Error>> {
    let Some(choice) = available(state, defs, current, owner)?
        .into_iter()
        .find(|c| c.native_type == belief)
    else {
        return Ok(false);
    };
    if !choice.effects_supported {
        return Err(Error::Unsupported("selected pantheon effects"));
    }
    state
        .social_components_mut(owner)
        .map_err(Error::State)?
        .religion
        .pantheon_hash = choice.hash;
    effects
        .event(state, owner, Event::BeliefChosen { belief }, defs)
        .map_err(Error::Effects)?;
    let delta = state
        .retained()
        .religion
        .minimum_faith_for_next_pantheon
        .wrapping_neg()
        .wrapping_shl(8);
    let old = player::<E::Error>(state, owner)?.religion.faith_balance_q8;
    let new = old.saturating_add(delta);
    if old != new {
        state
            .social_components_mut(owner)
            .map_err(Error::State)?
            .religion
            .faith_balance_q8 = new;
        effects
            .event(state, owner, Event::FaithChanged { balance_q8: new }, defs)
            .map_err(Error::Effects)?;
    }
    let cities = player::<E::Error>(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .map(|r| r.value.header.id)
        .collect::<Vec<_>>();
    for &city in &cities {
        effects
            .event(
                state,
                owner,
                Event::CityPantheonChanged {
                    city,
                    old: u32::MAX,
                    new: choice.hash,
                },
                defs,
            )
            .map_err(Error::Effects)?;
    }
    effects
        .event(state, owner, Event::ReportFounded { belief }, defs)
        .map_err(Error::Effects)?;
    effects
        .event(state, owner, Event::PantheonFounded { belief }, defs)
        .map_err(Error::Effects)?;
    for city in cities {
        effects
            .event(state, owner, Event::ReportCity { city }, defs)
            .map_err(Error::Effects)?;
    }
    Ok(true)
}
