//! Source GenerateCombatResults prefix for AttackInto's noncombat branch.
//! Numeric members are temporary, but proposals, callback effects and RNG use
//! the ongoing canonical owners before TryCaptureUnit is allowed to run.
use crate::{
    loaded_actions::{stock_notifications as stock, IdentityReport},
    loaded_upkeep::{self, OperationReports},
    melee_attack::{backend, execution, experience, target_selection::Defender},
};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::PlotCoord};

pub struct Inputs<'a> {
    pub rules: execution::GenerateRules<'a>,
    pub experience: &'a experience::Definitions,
    /// Moved into the temporary backend and restored after every normal return.
    /// None denotes a missing or already loaned canonical configuration owner.
    pub backend: Option<backend::Inputs>,
}

struct Reports<'a, E>(&'a mut E);
impl<E: OperationReports> OperationReports for Reports<'_, E> {
    type Error = E::Error;
    fn preflight_operation_report(
        &self,
        state: &LoadedState,
        actor: loaded_upkeep::UnitHandle,
        report: IdentityReport,
    ) -> Result<(), Self::Error> {
        self.0.preflight_operation_report(state, actor, report)
    }
    fn preflight_placement_report(
        &self,
        state: &LoadedState,
        actor: loaded_upkeep::UnitHandle,
        event: u32,
    ) -> Result<(), Self::Error> {
        self.0.preflight_placement_report(state, actor, event)
    }
    fn placement_report(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_placement_transition::PlacementReport,
    ) -> Result<(), Self::Error> {
        self.0.placement_report(state, report)
    }
    fn operation_report(
        &mut self,
        state: &mut LoadedState,
        report: IdentityReport,
    ) -> Result<i32, Self::Error> {
        self.0.operation_report(state, report)
    }
}
impl Inputs<'_> {
    /// Continue immediately after the source's fresh target selection. This
    /// clears the defender, generates temporary results and destroys proposals;
    /// the caller still owes the noncombat attacker Clear and TryCaptureUnit.
    pub fn generate<P>(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut stock::Runtime<'_, P>,
        actor: UnitReference,
        defender: UnitReference,
        at: PlotCoord,
    ) -> Result<(), String> {
        // Check the owner slot before moving the immutable backend packet so a
        // rejected loan leaves every input available to its actual owner.
        runtime.modifier_owners().map_err(super::debug)?;
        let inputs = self
            .backend
            .take()
            .ok_or("capture generation backend inputs")?;
        let rules = &self.rules;
        let experience = self.experience;
        let mut restored_inputs = None;
        let result = runtime.with_owned_modifier_owners(|runtime, owners| {
            let stock::unit_creation::ModifierOwners {
                effects,
                requirements,
                collections,
                unit_created,
            } = owners;
            let resumed = backend::Backend::resume_owned(
                state,
                state.action_stamp(),
                Reports(runtime),
                requirements,
                effects,
                inputs,
            );
            let (mut combat, admitted) = match resumed {
                Ok(combat) => (combat, true),
                Err(combat) => (combat, false),
            };
            let result = if admitted {
                (|| {
                    let outcome = execution::begin_selected(
                        state,
                        None,
                        actor,
                        at,
                        Defender::Unit(defender),
                        &mut combat,
                    )
                    .map_err(super::debug)?;
                    let execution::Outcome::RequiresGeneration(mut generation) = outcome else {
                        return Err("capture generation requires selected defender".into());
                    };
                    // Native AttackInto discards these temporary member values
                    // for a successfully captured noncombat Unit. Generation's
                    // actual callbacks/destruction still run in source order.
                    generation
                        .advance_members(rules, experience)
                        .map_err(super::debug)?;
                    Ok(())
                })()
            } else {
                Err("capture generation canonical backend binding".into())
            };
            let (_, requirements, effects, inputs) = combat.into_parts();
            restored_inputs = Some(inputs);
            (
                stock::unit_creation::ModifierOwners {
                    effects,
                    requirements,
                    collections,
                    unit_created,
                },
                result,
            )
        });
        self.backend = restored_inputs;
        result.map_err(super::debug)?
    }
}
