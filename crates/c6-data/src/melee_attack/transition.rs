//! Source-ordered ordinary AttackInto composition. Persistent owners are lent
//! to the existing operation provider for one attack and restored on errors.
use super::{backend, execution, experience, resolve, target_selection};
use crate::{loaded_upkeep, unit_placement_state as placement, unit_sight_state as sight};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::PlotCoord};

type ProviderError<E> = <E as sight::Effects>::Error;
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    Reentry,
    Backend,
    Begin,
    Generate,
    AttackerCompletion,
    Damage,
    Experience,
    DeathReport,
    SurvivingAttacker,
    DefenderDeath,
    CaptureProposal,
    Budget,
    KillAwards,
    PostKill,
    PostYields,
    Completion,
}
#[derive(Debug)]
pub struct Error {
    pub stage: Stage,
    pub detail: String,
}
fn failure(stage: Stage, error: impl std::fmt::Debug) -> Error {
    Error {
        stage,
        detail: format!("{error:?}"),
    }
}
/// A no-target return is the native false bool-out. Resolved includes every
/// supported mandatory combat continuation; the containing operation continues.
pub enum Outcome {
    NoTarget,
    Resolved(resolve::completion::ResolvedCombat),
}
pub struct Session<R> {
    backend: Option<backend::Backend<(), R>>,
    origin: c6_core::loaded_state::ActionStamp,
}
impl<R> Session<R> {
    pub fn from_backend<O>(state: &LoadedState, backend: backend::Backend<O, R>) -> (O, Self) {
        let (reporting, backend) = backend.with_reporting(());
        (
            reporting,
            Self {
                backend: Some(backend),
                origin: state.action_stamp(),
            },
        )
    }
    pub fn backend(&self) -> Option<&backend::Backend<(), R>> {
        self.backend.as_ref()
    }
    pub fn backend_mut(&mut self) -> Option<&mut backend::Backend<(), R>> {
        self.backend.as_mut()
    }
    pub fn into_backend(self) -> Option<backend::Backend<(), R>> {
        self.backend
    }
}
/// Bind the actual death continuation to the same requirement/effect owners.
/// Implementations construct only their typed callback/provider view and invoke
/// resolve::apply_damage_with_continuations; they must preserve partial failures.
pub trait Damage<E: placement::Effects, R> {
    type Error: std::fmt::Debug;
    #[allow(clippy::too_many_arguments)]
    fn apply(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut E,
        runtime: &mut backend::EffectRuntime,
        requirements: &mut R,
        definitions: &placement::Definitions<'_, '_>,
        queries: &placement::Queries<'_>,
        formation: &crate::formation_rules::Definitions,
        pending: execution::RequiresResolve,
    ) -> Result<resolve::RequiresExperience, Self::Error>;
}
pub struct Inputs<'a, 'd, 'r> {
    pub generation: &'a execution::GenerateRules<'a>,
    pub experience: &'a experience::Definitions,
    pub placement: &'a placement::Definitions<'d, 'r>,
    pub queries: &'a placement::Queries<'a>,
    pub movement: &'a crate::unit_movement_state::Inputs<'a>,
    pub cached_majors: &'a [i32],
    pub awards: &'a resolve::kill_awards::Inputs<'a, 'd, 'r>,
    pub post_kill: &'a resolve::post_kill::Inputs<'a, 'd, 'r>,
    pub post_yields: &'a resolve::post_yields::Inputs<'a, 'a>,
    pub callbacks: &'a resolve::completion::stock::Inputs<'a, 'd, 'r>,
    pub death_report_status: u32,
}
pub(crate) struct Reports<'a, E>(pub(crate) &'a mut E);
impl<E: loaded_upkeep::OperationReports> loaded_upkeep::OperationReports for Reports<'_, E> {
    type Error = E::Error;
    fn preflight_operation_report(
        &self,
        s: &LoadedState,
        u: loaded_upkeep::UnitHandle,
        r: crate::loaded_actions::IdentityReport,
    ) -> Result<(), Self::Error> {
        self.0.preflight_operation_report(s, u, r)
    }
    fn preflight_placement_report(
        &self,
        s: &LoadedState,
        u: loaded_upkeep::UnitHandle,
        e: u32,
    ) -> Result<(), Self::Error> {
        self.0.preflight_placement_report(s, u, e)
    }
    fn placement_report(
        &mut self,
        s: &mut LoadedState,
        r: c6_core::unit_placement_transition::PlacementReport,
    ) -> Result<(), Self::Error> {
        self.0.placement_report(s, r)
    }
    fn operation_report(
        &mut self,
        s: &mut LoadedState,
        r: crate::loaded_actions::IdentityReport,
    ) -> Result<i32, Self::Error> {
        self.0.operation_report(s, r)
    }
}
impl<E: backend::PostCombatYields> backend::PostCombatYields for Reports<'_, E> {
    fn credit_post_combat_yield(
        &mut self,
        s: &mut LoadedState,
        u: UnitReference,
        y: i32,
        n: i32,
    ) -> Result<(), Self::Error> {
        self.0.credit_post_combat_yield(s, u, y, n)
    }
}
impl<R: backend::Requirements> Session<R> {
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub fn execute<E, D>(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut E,
        damage: &mut D,
        inputs: &Inputs<'_, '_, '_>,
        bindings: &mut resolve::completion::stock::Bindings,
        actor: UnitReference,
        destination: PlotCoord,
    ) -> Result<Outcome, Error>
    where
        E: placement::Effects
            + loaded_upkeep::Effects<Error = ProviderError<E>>
            + resolve::DeathReports<Error = ProviderError<E>>
            + resolve::surviving::Effects
            + resolve::winning::Effects
            + experience::Reports<Error = ProviderError<E>>
            + resolve::kill_awards::Effects
            + resolve::post_kill::Effects
            + backend::PostCombatYields
            + resolve::completion::stock::Effects,
        ProviderError<E>: std::fmt::Debug,
        R::Error: std::fmt::Debug,
        <E as resolve::completion::stock::Effects>::Error: std::fmt::Debug,
        D: Damage<E, R>,
    {
        if !self.origin.same_instance(state.action_stamp()) {
            return Err(failure(
                Stage::Backend,
                "combat session belongs to another World",
            ));
        }
        if self.backend.is_none() {
            return Err(failure(Stage::Reentry, "attack already active"));
        }
        let selected = {
            let service =
                crate::unit_placement_services::Services::placement_service_runtime(effects)
                    .map_err(|e| failure(Stage::Begin, e))?;
            let target = target_selection::Inputs {
                combat: inputs.generation.formation,
                movement: inputs.movement.definitions,
                sight,
                services: service,
                districts: Some(inputs.generation.maximum_damage),
            };
            target_selection::select_execution_target(state, &target, actor, destination)
                .map_err(|e| failure(Stage::Begin, e))?
        };
        let Some(selected) = selected else {
            return Ok(Outcome::NoTarget);
        };
        let backend = self
            .backend
            .take()
            .ok_or_else(|| failure(Stage::Reentry, "attack already active"))?;
        let (_, mut backend) = backend.with_reporting(Reports(effects));
        let result = (|| {
            let pending = match execution::begin_selected(
                state,
                Some(inputs.generation.maximum_damage),
                actor,
                destination,
                selected,
                &mut backend,
            )
            .map_err(|e| failure(Stage::Begin, e))?
            {
                execution::Outcome::NoTarget => return Ok(Outcome::NoTarget),
                execution::Outcome::RequiresGeneration(mut generation) => {
                    let next = generation
                        .advance_members(inputs.generation, inputs.experience)
                        .map_err(|e| failure(Stage::Generate, e))?;
                    generation
                        .complete_attacker(next)
                        .map_err(|e| failure(Stage::AttackerCompletion, e))?
                }
            };
            let (reports, runtime, requirements, _) = backend.kill_parts();
            let pending = damage
                .apply(
                    state,
                    sight,
                    reports.0,
                    runtime,
                    requirements,
                    inputs.placement,
                    inputs.queries,
                    inputs.generation.formation,
                    pending,
                )
                .map_err(|e| failure(Stage::Damage, e))?;
            let pending = resolve::apply_experience(
                state,
                inputs.experience,
                inputs.generation.maximum_damage,
                inputs.generation.levels,
                inputs.cached_majors,
                reports.0,
                pending,
            )
            .map_err(|e| failure(Stage::Experience, e))?;
            let current = super::actor(state, pending.members.damage.attacker)
                .map_err(|e| failure(Stage::DeathReport, e))?;
            let dead = current.i_damage
                >= inputs
                    .generation
                    .maximum_damage
                    .maximum_damage(current)
                    .map_err(|e| failure(Stage::DeathReport, e))?;
            let defender_dead = if dead {
                false
            } else {
                let defender = super::actor(state, pending.members.damage.defender)
                    .map_err(|e| failure(Stage::DefenderDeath, e))?;
                defender.i_damage
                    >= inputs
                        .generation
                        .maximum_damage
                        .maximum_damage(defender)
                        .map_err(|e| failure(Stage::DefenderDeath, e))?
            };
            let pending = if dead {
                resolve::finish_dead_attacker(
                    state,
                    inputs.generation.maximum_damage,
                    reports.0,
                    pending,
                    inputs.death_report_status,
                )
                .map_err(|e| failure(Stage::DeathReport, e))?
            } else if defender_dead {
                resolve::winning::report(
                    state,
                    inputs.generation.maximum_damage,
                    backend.reporting.0,
                    &pending,
                )
                .map_err(|e| failure(Stage::DefenderDeath, e))?;
                let (proposal_id, proposal) = super::proposals::Context::new(state, &mut backend)
                    .propose_units(
                        pending.members.damage.attacker,
                        pending.members.damage.defender,
                        pending.members.location,
                    )
                    .map_err(|e| failure(Stage::CaptureProposal, e))?;
                let capture = state
                    .combat_proposals()
                    .map_err(|e| failure(Stage::CaptureProposal, e))?
                    .proposal(proposal)
                    .map_err(|e| failure(Stage::CaptureProposal, e))?
                    .capture_combat_units;
                if capture {
                    return Err(failure(
                        Stage::CaptureProposal,
                        "positive combat-unit capture continuation",
                    ));
                }
                resolve::winning::notify(
                    state,
                    backend.reporting.0,
                    &pending,
                    inputs.death_report_status,
                )
                .map_err(|e| failure(Stage::DefenderDeath, e))?;
                super::proposals::collection_modifiers::forget_combat(
                    state,
                    &mut backend,
                    proposal_id,
                )
                .map_err(|e| failure(Stage::CaptureProposal, e))?;
                resolve::surviving::finish_winner(
                    state,
                    inputs.generation.maximum_damage,
                    inputs.movement,
                    inputs.placement,
                    inputs.queries,
                    sight,
                    backend.reporting.0,
                    pending,
                )
                .map_err(|e| failure(Stage::SurvivingAttacker, e))?
            } else {
                resolve::surviving::finish_both_alive(
                    state,
                    inputs.generation.maximum_damage,
                    inputs.movement,
                    sight,
                    reports.0,
                    pending,
                    inputs.death_report_status,
                )
                .map_err(|e| failure(Stage::SurvivingAttacker, e))?
            };
            let reports = &mut backend.reporting;
            let pending =
                resolve::budget::consume_attack(state, inputs.movement, reports.0, pending)
                    .map_err(|e| failure(Stage::Budget, e))?;
            let pending =
                resolve::kill_awards::apply(state, sight, reports.0, inputs.awards, pending)
                    .map_err(|e| failure(Stage::KillAwards, e))?;
            let pending =
                resolve::post_kill::apply(state, sight, reports.0, inputs.post_kill, pending)
                    .map_err(|e| failure(Stage::PostKill, e))?;
            let pending =
                resolve::post_yields::apply(state, &mut backend, inputs.post_yields, pending)
                    .map_err(|e| failure(Stage::PostYields, e))?;
            let mut completion = resolve::completion::stock::Completion {
                bindings,
                inputs: inputs.callbacks,
                effects: backend.reporting.0,
            };
            resolve::completion::finish(state, &mut completion, inputs.generation.levels, pending)
                .map(Outcome::Resolved)
                .map_err(|e| failure(Stage::Completion, e))
        })();
        let (_, backend) = backend.with_reporting(());
        self.backend = Some(backend);
        result
    }
}
