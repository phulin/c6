//! Stock Policy GME subscriber, borrowing the ongoing canonical modifier owners.
use super::{unit_creation::CollectionEffects, Runtime};
use crate::{
    loaded_actions::policy_actions::modifiers::{Definitions, Recipe},
    unit_creation::collections,
};
use c6_core::{loaded_state::LoadedState, map::UnitReference};
#[derive(Debug)]
pub enum Error {
    Definitions(crate::loaded_actions::policy_actions::modifiers::Error),
    City(crate::melee_attack::backend::city_creation::Error),
    Unit(collections::Error),
    Missing(&'static str),
}
impl<P> Runtime<'_, P> {
    /// Called at the actual OnPolicyEnacted/OnPolicyRetracted signal position.
    /// `policy` is the native definition index, `hash` its configured type hash.
    /// City report effects are the caller's existing qualified report services.
    #[allow(clippy::too_many_arguments)]
    pub fn policy_modifier_event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        policy: i32,
        hash: u32,
        added: bool,
        definitions: &Definitions,
        reports: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), Error> {
        let recipes = definitions
            .policy(state, hash)
            .map_err(Error::Definitions)?;
        if added && recipes.is_empty() {
            return Ok(());
        }
        if policy < 0 {
            return Err(Error::Missing("native Policy index"));
        }
        self.with_modifier_owners(|runtime, modifier| {
            let context = modifier
                .effects
                .make_policy_object(state, owner, policy as u32)
                .map_err(Error::City)?;
            if !added {
                let mut owners = collections::Owners {
                    state,
                    effects: &mut modifier.effects,
                    requirements: &mut modifier.requirements,
                    collections: &mut modifier.collections,
                };
                if recipes.iter().any(|r| matches!(r, Recipe::Units { .. })) {
                    collections::policy::retract(&mut owners, &mut modifier.unit_created, context)
                        .map_err(Error::Unit)?;
                }
                if recipes.iter().any(|r| matches!(r, Recipe::City(_))) {
                    modifier
                        .effects
                        .remove_policy_city_modifiers(
                            state,
                            &mut modifier.requirements,
                            context,
                            reports,
                        )
                        .map_err(Error::City)?;
                }
                modifier
                    .effects
                    .retire_policy_object(state, context)
                    .map_err(Error::City)?;
                return Ok(());
            }
            for recipe in recipes {
                match recipe {
                    Recipe::City(definition) => {
                        modifier
                            .effects
                            .attach_city_owner(
                                state,
                                &mut modifier.requirements,
                                context,
                                definition,
                                None,
                                reports,
                            )
                            .map_err(Error::City)?;
                    }
                    Recipe::Units { id, amount } => {
                        let input = runtime
                            .unit_creation
                            .as_ref()
                            .ok_or(Error::Missing("current Unit creation definition services"))?;
                        let movement = runtime
                            .movement
                            .as_mut()
                            .ok_or(Error::Missing("current Unit collection services"))?;
                        let mut effects = CollectionEffects {
                            definitions: input.definitions,
                            tags: input.tags,
                            moved: &mut movement.signal,
                            reporting: &mut runtime.reporting,
                            reporting_suspended: movement.reporting_suspended,
                        };
                        let mut owners = collections::Owners {
                            state,
                            effects: &mut modifier.effects,
                            requirements: &mut modifier.requirements,
                            collections: &mut modifier.collections,
                        };
                        collections::policy::enact(
                            &mut owners,
                            &mut modifier.unit_created,
                            &mut effects,
                            input.definitions,
                            context,
                            id,
                            *amount,
                        )
                        .map_err(Error::Unit)?;
                    }
                }
            }
            Ok(())
        })
        .map_err(|_| Error::Missing("canonical modifier owner loan"))?
    }
    /// Newly registered PlayerUnits listeners append to the source UnitCreated
    /// ring. The signal dispatcher invokes this route at that actual position.
    pub fn policy_unit_created(
        &mut self,
        state: &mut LoadedState,
        route: collections::policy::Subscription,
        unit: UnitReference,
    ) -> Result<(), Error> {
        self.with_modifier_owners(|runtime, modifier| {
            let input = runtime
                .unit_creation
                .as_ref()
                .ok_or(Error::Missing("current Unit collection definitions"))?;
            let movement = runtime
                .movement
                .as_mut()
                .ok_or(Error::Missing("current Unit collection movement"))?;
            let mut effects = CollectionEffects {
                definitions: input.definitions,
                tags: input.tags,
                moved: &mut movement.signal,
                reporting: &mut runtime.reporting,
                reporting_suspended: movement.reporting_suspended,
            };
            let mut owners = collections::Owners {
                state,
                effects: &mut modifier.effects,
                requirements: &mut modifier.requirements,
                collections: &mut modifier.collections,
            };
            collections::policy::unit_created(
                &mut owners,
                &mut modifier.unit_created,
                &mut effects,
                input.definitions,
                route,
                unit,
            )
            .map_err(Error::Unit)
        })
        .map_err(|_| Error::Missing("canonical modifier owner loan"))?
    }
}
