//! Synchronous reunion of combat's existing canonical modifier owner bundle.
//! No replacement stores, source replay, or world clone is constructed.
use super::{movement, unit_creation::ModifierOwners, Runtime};
use crate::melee_attack::backend::{requirements, EffectRuntime};
use c6_core::loaded_state::LoadedState;

/// The two owners which combat queries do not borrow. They stay beside the
/// combat backend until a capture needs the full founding/destruction scope.
pub(crate) struct CombatLifecycleOwners {
    pub collections: c6_core::melee_attack::proposals::collections::Runtime,
    pub unit_created: crate::unit_creation::collections::Runtime,
}

impl<P> Runtime<'_, P> {
    /// Loan the SAME four owners to a nested capture lifecycle. The caller
    /// consumes its backend before this entry and rebuilds it from the returned
    /// effect/requirement owners before propagating either result.
    ///
    /// `apply` may itself use the existing with_modifier_owners scopes. Every
    /// ordinary error result restores the mutated bundle and its sidecar; no
    /// failed gameplay prefix is reset or replaced with initial inputs.
    pub fn with_combat_modifier_owners<T>(
        &mut self,
        state: &mut LoadedState,
        effects: EffectRuntime,
        requirements: requirements::Runtime,
        apply: impl FnOnce(&mut Self, &mut LoadedState) -> T,
    ) -> (
        EffectRuntime,
        requirements::Runtime,
        Result<T, movement::Error>,
    ) {
        let rejected = self.check(state).is_err()
            || self.modifier_owners.is_some()
            || self.combat.as_ref().is_none_or(|combat| {
                !combat.origin.same_instance(state.action_stamp())
                    || combat.capture_owners.is_none()
            });
        if rejected {
            return (
                effects,
                requirements,
                Err(movement::Error::Missing("capture canonical owner loan")),
            );
        }
        let sidecar = self
            .combat
            .as_mut()
            .expect("checked combat owner")
            .capture_owners
            .take()
            .expect("checked lifecycle sidecar");
        self.modifier_owners = Some(ModifierOwners {
            effects,
            requirements,
            collections: sidecar.collections,
            unit_created: sidecar.unit_created,
        });
        let result = apply(self, state);
        let ModifierOwners {
            effects,
            requirements,
            collections,
            unit_created,
        } = self
            .modifier_owners
            .take()
            .expect("nested lifecycle scopes restore canonical owners");
        self.combat
            .as_mut()
            .expect("nested lifecycle retains combat output owner")
            .capture_owners = Some(CombatLifecycleOwners {
            collections,
            unit_created,
        });
        (effects, requirements, Ok(result))
    }
}
