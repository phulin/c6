//! Initiation Rites' persistent global-player attachment graph, in the same
//! EffectRuntime that owns other gameplay modifiers and allocation counters.
use super::{ContextObject, EffectRuntime, Object, QueryError};
use crate::loaded_actions::pantheon_actions::{
    self as action,
    modifiers::{Effect, Recipe},
};
use c6_core::{loaded_state::LoadedState, melee_attack::proposals::subject_sets as sets};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subject {
    pub owner: i32,
    pub object: u64,
    /// Shared requirement cache entry; both configured parent modifiers use
    /// one PlayerHasPantheon listener for the same context/subject/definition.
    pub requirement: u64,
    pub requirement_set: u64,
    pub child_instance: Option<i32>,
    pub applied: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requirement {
    pub label: u64,
    pub context: u64,
    pub subject: u64,
    pub owner: i32,
    pub belief: u32,
    pub definition: Vec<u8>,
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
    Requirements(Box<sets::Error<Error>>),
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
    pub fn pantheon_requirements(&self) -> &[Requirement] {
        &self.pantheon_requirements
    }
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
        requirements: &mut super::requirements::Runtime,
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
        let first = definitions
            .recipes
            .first()
            .ok_or(Error::Missing("pantheon recipes"))?;
        let factory = sets::Runtime::from_initial(
            0,
            vec![(
                first.requirement_set.clone(),
                Some(sets::SetDefinition {
                    id: first.requirement_set.clone(),
                    requirements: vec![first.requirement.clone()],
                }),
            )],
            vec![sets::RequirementDefinition {
                id: first.requirement.clone(),
                predicate: sets::Predicate::Unsupported(crate::registry::linux_type_hash(
                    b"REQUIREMENT_PLAYER_HAS_PANTHEON",
                )),
                inverse: false,
                persistent: false,
            }],
        )
        .map_err(Error::Missing)?;
        requirements
            .sets_mut()
            .install_factory_definitions(&factory)
            .map_err(Error::Missing)?;
        let context_instance = self.object(context).map_err(Error::Query)?.instance_id;
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
                let requirement_set = sets::get_or_create_subject(
                    &mut Factory {
                        state,
                        effects: self,
                        requirements,
                    },
                    (1u64 << 62) | u64::from(instance as u32),
                    context_instance,
                    sets::Subject::ModifierObject(object),
                    &recipe.requirement_set,
                )
                .map_err(|e| Error::Requirements(Box::new(e)))?
                .ok_or(Error::Missing("pantheon requirement set"))?;
                let requirement = *requirements
                    .sets()
                    .children(requirement_set)
                    .and_then(|v| v.first())
                    .ok_or(Error::Missing("pantheon shared requirement"))?;
                let child_instance = if requirement_result == 0 {
                    Some(self.allocate_modifier_id(state).map_err(Error::Query)?)
                } else {
                    None
                };
                self.pantheons[index].subjects.push(Subject {
                    owner: target,
                    object,
                    requirement,
                    requirement_set,
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
        requirements: &mut super::requirements::Runtime,
        owner: i32,
        city: u32,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        state.city(owner, city).map_err(Error::State)?;
        for requirement in &self.pantheon_requirements {
            if requirement.owner != owner {
                continue;
            }
            let result = test(state, owner, requirement.belief)?;
            let previous = requirements
                .sets()
                .requirement_state(requirement.label)
                .ok_or(Error::Missing("live pantheon requirement state"))?;
            if result != previous {
                return Err(Error::Missing("changed pantheon requirement continuation"));
            }
        }
        Ok(())
    }
}
struct Factory<'a> {
    state: &'a LoadedState,
    effects: &'a mut EffectRuntime,
    requirements: &'a mut super::requirements::Runtime,
}
impl sets::SubjectContext for Factory<'_> {
    type Error = Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Error> {
        Ok(self.requirements.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Error> {
        Ok(self.requirements.sets_mut())
    }
    fn initialize_requirement(
        &mut self,
        label: u64,
        definition: &sets::RequirementDefinition,
        context: i32,
        subject: sets::Subject,
    ) -> Result<sets::InitializedRequirement, sets::Error<Error>> {
        let sets::Subject::ModifierObject(subject) = subject else {
            return Err(sets::Error::UnsupportedSubject);
        };
        let (context, belief) = self
            .effects
            .objects
            .iter()
            .find_map(|(handle, object)| {
                if object.instance_id != context {
                    return None;
                }
                match object.kind {
                    ContextObject::Belief { belief, .. } => Some((*handle, belief)),
                    _ => None,
                }
            })
            .ok_or(sets::Error::Context(Error::Missing(
                "PlayerHasPantheon context IBelief",
            )))?;
        let object = self
            .effects
            .object(subject)
            .map_err(|e| sets::Error::Context(Error::Query(e)))?;
        let ContextObject::Player(owner) = object.kind else {
            return Err(sets::Error::UnsupportedSubject);
        };
        // Native OnInitialize appends the CityChangedPantheon listener BEFORE
        // SimpleRequirement.Initialize tests. The shared cache invokes this only
        // once for equal definition/context/subject identities.
        self.effects.pantheon_requirements.push(Requirement {
            label,
            context,
            subject,
            owner,
            belief,
            definition: definition.id.clone(),
        });
        let result = test(self.state, owner, belief).map_err(sets::Error::Context)?;
        Ok(sets::InitializedRequirement::Simple(
            c6_core::simple_requirement::Requirement {
                active: true,
                state: result,
                inverse: definition.inverse,
                persistent: definition.persistent,
                triggered: false,
            },
        ))
    }
    fn log_missing_set(&mut self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
    fn log_created_requirement(&mut self, _: u32) -> Result<(), Error> {
        Ok(())
    }
    fn log_created_set(&mut self, _: u32) -> Result<(), Error> {
        Ok(())
    }
}
