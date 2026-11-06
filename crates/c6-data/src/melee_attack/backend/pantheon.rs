//! Initiation Rites' persistent global-player attachment graph, in the same
//! EffectRuntime that owns other gameplay modifiers and allocation counters.
use super::{ContextObject, EffectRuntime, Object, QueryError};
use crate::loaded_actions::pantheon_actions::{
    self as action,
    modifiers::{Effect, Recipe},
};
use c6_core::loaded_state::LoadedState;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subject {
    pub owner: i32,
    pub object: u64,
    /// Native Requirement result: 0 is Met, 1 is NotMet.
    pub requirement_result: i32,
    pub child_instance: Option<i32>,
    pub applied: bool,
}
#[derive(Clone, Debug)]
pub struct Attachment {
    pub belief_context: u64,
    pub founder: i32,
    pub belief: u32,
    pub instance: i32,
    pub recipe: Recipe,
    /// GetAlive/IsAlive collection order is retained, including false subjects.
    pub subjects: Vec<Subject>,
}
#[derive(Debug)]
pub enum Error {
    Query(QueryError),
    State(c6_core::loaded_state::Error),
    Missing(&'static str),
}
fn apply(state: &mut LoadedState, owner: i32, recipe: &Recipe, remove: bool) -> Result<(), Error> {
    state
        .change_pantheon_dispersal_effect(
            owner,
            recipe.effect == Effect::FaithFromDispersal,
            if remove {
                recipe.amount.wrapping_neg()
            } else {
                recipe.amount
            },
        )
        .map_err(Error::State)
}
fn test(state: &LoadedState, owner: i32, belief: u32) -> Result<i32, Error> {
    let p = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("PlayerHasPantheon subject"))?;
    Ok(i32::from(p.religion.pantheon_hash != belief))
}
impl EffectRuntime {
    pub fn pantheon_attachments(&self) -> &[Attachment] {
        &self.pantheons
    }
    /// Execute the qualified BeliefChosenForPantheon GME subscriber. All parent
    /// modifiers, global-player subjects, requirement results and active child
    /// instances stay live. No UI, AI selection or RNG substitution occurs.
    pub fn attach_pantheon(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        belief: i32,
        defs: &action::Definitions,
        current: &action::Current<'_>,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let definitions = defs
            .rites
            .as_ref()
            .ok_or(Error::Missing("Initiation Rites definitions"))?;
        if defs.belief_hash(belief) != Some(definitions.belief_hash) {
            return Err(Error::Missing("supported pantheon belief"));
        }
        if self.pantheons.iter().any(|a| a.founder == owner) {
            return Err(Error::Missing("pantheon already attached"));
        }
        let kind = ContextObject::Belief {
            founder: owner,
            religion_native_type: 0,
            belief: definitions.belief_hash,
        };
        let context = match self.objects.iter().find(|r| r.1.kind == kind) {
            Some(r) => r.0,
            None => self
                .append_normal_object(
                    state,
                    Object {
                        kind,
                        instance_id: 0,
                        unit_sort_suffix: None,
                        unit_lifecycle: None,
                    },
                )
                .map_err(Error::Query)?,
        };
        let players = action::alive::<std::convert::Infallible>(state, current)
            .map_err(|_| Error::Missing("pantheon GetAlive"))?;
        for recipe in &definitions.recipes {
            let instance = self.allocate_modifier_id(state).map_err(Error::Query)?;
            self.pantheons.push(Attachment {
                belief_context: context,
                founder: owner,
                belief: definitions.belief_hash,
                instance,
                recipe: recipe.clone(),
                subjects: vec![],
            });
            let index = self.pantheons.len() - 1;
            for &target in &players {
                let p = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == target)
                    .ok_or(Error::Missing("AllPlayers current Player"))?;
                if !p.header.saved_is_alive() {
                    continue;
                }
                let kind = ContextObject::Player(target);
                let object = match self.objects.iter().find(|r| r.1.kind == kind) {
                    Some(r) => r.0,
                    None => self
                        .append_normal_object(
                            state,
                            Object {
                                kind,
                                instance_id: 0,
                                unit_sort_suffix: None,
                                unit_lifecycle: None,
                            },
                        )
                        .map_err(Error::Query)?,
                };
                let requirement_result = test(state, target, definitions.belief_hash)?;
                let child_instance = if requirement_result == 0 {
                    Some(self.allocate_modifier_id(state).map_err(Error::Query)?)
                } else {
                    None
                };
                self.pantheons[index].subjects.push(Subject {
                    owner: target,
                    object,
                    requirement_result,
                    child_instance,
                    applied: false,
                });
                if child_instance.is_some() {
                    apply(state, target, recipe, false)?;
                    self.pantheons[index].subjects.last_mut().unwrap().applied = true;
                }
            }
        }
        Ok(())
    }
    /// Each PlayerHasPantheon listener filters the source City owner, then
    /// repeats Test. A stable result does not apply the child a second time.
    pub fn pantheon_city_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        state.city(owner, city).map_err(Error::State)?;
        for i in 0..self.pantheons.len() {
            let belief = self.pantheons[i].belief;
            let recipe = self.pantheons[i].recipe.clone();
            for j in 0..self.pantheons[i].subjects.len() {
                if self.pantheons[i].subjects[j].owner != owner {
                    continue;
                }
                let result = test(state, owner, belief)?;
                let prior = self.pantheons[i].subjects[j].applied;
                self.pantheons[i].subjects[j].requirement_result = result;
                if result == 0 && !prior {
                    let instance = self.allocate_modifier_id(state).map_err(Error::Query)?;
                    self.pantheons[i].subjects[j].child_instance = Some(instance);
                    apply(state, owner, &recipe, false)?;
                    self.pantheons[i].subjects[j].applied = true;
                } else if result != 0 && prior {
                    apply(state, owner, &recipe, true)?;
                    self.pantheons[i].subjects[j].applied = false;
                    self.pantheons[i].subjects[j].child_instance = None;
                }
            }
        }
        Ok(())
    }
}
