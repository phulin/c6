//! Ordinary ranged Unit/Unit combat over the current canonical state.
//! The supported branch requires both units to survive. Other reached branches
//! fail explicitly and retain completed mutations and consumed gameplay RNG.
pub mod query;
use crate::{
    loaded_upkeep as upkeep, melee_attack as melee, unit_placement_state as placement,
    unit_sight_state as sight,
};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::PlotCoord};
use melee::{backend, execution, experience, proposals, resolve};
pub const RANGED: u32 = 0x2ec4ce4d;
#[derive(Debug)]
pub struct Error {
    pub stage: &'static str,
    pub detail: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ranged combat {}: {}", self.stage, self.detail)
    }
}
impl std::error::Error for Error {}
fn error(stage: &'static str, detail: impl std::fmt::Debug) -> Error {
    Error {
        stage,
        detail: format!("{detail:?}"),
    }
}
type ProviderError<E> = <E as sight::Effects>::Error;
/// Ranged statuses are distinct semantic outputs from the melee statuses.
/// Subscribers must retain all current gameplay effects and RNG.
pub trait Effects: placement::Effects {
    fn ranged_surviving_statuses(
        &mut self,
        state: &mut LoadedState,
        attacker: UnitReference,
        defender: UnitReference,
        damage: i32,
        status: u32,
    ) -> Result<(), ProviderError<Self>>;
}
pub struct Session<R> {
    backend: Option<backend::Backend<(), R>>,
    origin: c6_core::loaded_state::ActionStamp,
}
impl<R> Session<R> {
    pub fn from_backend<O>(state: &LoadedState, backend: backend::Backend<O, R>) -> (O, Self) {
        let (reports, backend) = backend.with_reporting(());
        (
            reports,
            Self {
                backend: Some(backend),
                origin: state.action_stamp(),
            },
        )
    }
    pub fn into_backend(self) -> Option<backend::Backend<(), R>> {
        self.backend
    }
    pub fn backend(&self) -> Option<&backend::Backend<(), R>> {
        self.backend.as_ref()
    }
    pub fn backend_mut(&mut self) -> Option<&mut backend::Backend<(), R>> {
        self.backend.as_mut()
    }
}
impl<R: backend::Requirements> Session<R> {
    /// Complete the ordinary ranged combat call. The containing queued
    /// operation and MessagesProcessed continuation remain mandatory.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub fn execute<E>(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut E,
        inputs: &melee::transition::Inputs<'_, '_, '_>,
        bindings: &mut resolve::completion::stock::Bindings,
        actor: UnitReference,
        destination: PlotCoord,
    ) -> Result<Option<resolve::completion::ResolvedCombat>, Error>
    where
        E: Effects
            + upkeep::Effects<Error = ProviderError<E>>
            + experience::Reports<Error = ProviderError<E>>
            + resolve::kill_awards::Effects
            + resolve::post_kill::Effects
            + backend::PostCombatYields
            + resolve::completion::stock::Effects,
        ProviderError<E>: std::fmt::Debug,
        R::Error: std::fmt::Debug,
        <E as resolve::completion::stock::Effects>::Error: std::fmt::Debug,
    {
        if !self.origin.same_instance(state.action_stamp()) {
            return Err(error("owner", "different World"));
        }
        let attacker = melee::actor(state, actor).map_err(|e| error("attacker", e))?;
        let bombard = inputs
            .generation
            .formation
            .bombard(state, attacker)
            .map_err(|e| error("type", e))?;
        let ranged = inputs
            .generation
            .formation
            .ranged(state, attacker)
            .map_err(|e| error("type", e))?;
        if bombard > ranged {
            return Err(error("type", "bombard combat unsupported"));
        }
        let defender = {
            let services =
                crate::unit_placement_services::Services::placement_service_runtime(effects)
                    .map_err(|e| error("target", e))?;
            let target = melee::target_selection::Inputs {
                combat: inputs.generation.formation,
                movement: inputs.movement.definitions,
                sight,
                services,
                districts: Some(inputs.generation.maximum_damage),
            };
            if melee::target_selection::enemy_defensible_district_with_definitions(
                state,
                target.districts,
                attacker.e_owner,
                destination,
            )
            .map_err(|e| error("target", e))?
            .is_some()
            {
                return Err(error("target", "ranged District combat unsupported"));
            }
            melee::target_selection::ranged_defender(state, &target, actor, destination)
                .map_err(|e| error("target", e))?
        };
        let Some(defender) = defender else {
            return Ok(None);
        };
        let backend = self
            .backend
            .take()
            .ok_or_else(|| error("reentry", "ranged combat already active"))?;
        let (_, mut backend) = backend.with_reporting(melee::transition::Reports(effects));
        let result = (|| {
            let members = generate(state, &mut backend, inputs, actor, defender, destination)?;
            // RangedAttackInto clears the retained defender AFTER Generate.
            upkeep::clear_unit_operations(
                state,
                defender.player(),
                defender.id,
                backend.reporting.0,
            )
            .map_err(|e| error("defender clear", e))?;
            state
                .unit_mut(actor.player(), actor.id)
                .map_err(|e| error("attacker complete", e))?
                .i_fortify_turns = 0;
            // SetForfeitHiddenVisibility is a real reached setter. Its pending
            // reveal calculation belongs to the existing current sight owner.
            crate::ranged_attack::forfeit_hidden_visibility(state, actor)?;
            upkeep::clear_unit_operations(state, actor.player(), actor.id, backend.reporting.0)
                .map_err(|e| error("attacker clear", e))?;
            upkeep::complete_unit_movement(state, actor.player(), actor.id, backend.reporting.0)
                .map_err(|e| error("attacker complete", e))?;
            placement::change_damage(
                state,
                inputs.placement,
                inputs.queries,
                sight,
                backend.reporting.0,
                (defender.player(), defender.id),
                members.damage.defender_received,
            )
            .map_err(|e| error("damage", e))?;
            ensure_alive(state, inputs, actor, defender)?;
            // Ranged Resolve awards attacker before defender; melee is reverse.
            for (unit, amount) in [
                (actor, members.attacker_experience),
                (defender, members.defender_experience),
            ] {
                experience::change(
                    state,
                    inputs.experience,
                    inputs.generation.maximum_damage,
                    inputs.generation.levels,
                    unit,
                    amount,
                    backend.reporting.0,
                )
                .map_err(|e| error("experience", e))?;
            }
            for enemy in [actor.player(), defender.player()] {
                let partners = experience::joint_war_partners(
                    state,
                    inputs.generation.levels,
                    inputs.cached_majors,
                    enemy,
                )
                .map_err(|e| error("joint war", e))?;
                if !partners.owners.is_empty() {
                    return Err(error("joint war", partners));
                }
            }
            backend
                .reporting
                .0
                .ranged_surviving_statuses(
                    state,
                    actor,
                    defender,
                    members.damage.defender_received,
                    inputs.death_report_status,
                )
                .map_err(|e| error("statuses", e))?;
            upkeep::clear_unit_operations(state, actor.player(), actor.id, backend.reporting.0)
                .map_err(|e| error("resolve clear", e))?;
            upkeep::clear_unit_operations(
                state,
                defender.player(),
                defender.id,
                backend.reporting.0,
            )
            .map_err(|e| error("resolve clear", e))?;
            let heal = melee::actor(state, actor)
                .map_err(|e| error("heal", e))?
                .k_health_modifiers
                .b_can_heal_after_action;
            state
                .unit_mut(actor.player(), actor.id)
                .map_err(|e| error("heal", e))?
                .b_eligible_to_heal = heal;
            let pending = resolve::budget::consume_attack(
                state,
                inputs.movement,
                backend.reporting.0,
                resolve::RequiresOuterResolution { members },
            )
            .map_err(|e| error("budget", e))?;
            let pending = resolve::kill_awards::apply(
                state,
                sight,
                backend.reporting.0,
                inputs.awards,
                pending,
            )
            .map_err(|e| error("awards", e))?;
            let pending = resolve::post_kill::apply(
                state,
                sight,
                backend.reporting.0,
                inputs.post_kill,
                pending,
            )
            .map_err(|e| error("post combat", e))?;
            let pending =
                resolve::post_yields::apply(state, &mut backend, inputs.post_yields, pending)
                    .map_err(|e| error("yields", e))?;
            let mut completion = resolve::completion::stock::Completion {
                bindings,
                inputs: inputs.callbacks,
                effects: backend.reporting.0,
            };
            resolve::completion::finish(state, &mut completion, inputs.generation.levels, pending)
                .map(Some)
                .map_err(|e| error("completion", e))
        })();
        let (_, backend) = backend.with_reporting(());
        self.backend = Some(backend);
        result
    }
}
fn ensure_alive(
    state: &LoadedState,
    inputs: &melee::transition::Inputs<'_, '_, '_>,
    actor: UnitReference,
    defender: UnitReference,
) -> Result<(), Error> {
    for id in [actor, defender] {
        let unit = melee::actor(state, id).map_err(|e| error("survival", e))?;
        let max = inputs
            .generation
            .maximum_damage
            .maximum_damage(unit)
            .map_err(|e| error("survival", e))?;
        if unit.i_damage >= max || unit.b_delayed_death {
            return Err(error("survival", "ranged death continuation unsupported"));
        }
    }
    Ok(())
}
fn forfeit_hidden_visibility(state: &mut LoadedState, actor: UnitReference) -> Result<(), Error> {
    state
        .unit_mut(actor.player(), actor.id)
        .map_err(|e| error("hidden visibility", e))?
        .b_hidden_visibility_forfeited = true;
    Ok(())
}
fn generate<B>(
    state: &mut LoadedState,
    backend: &mut B,
    inputs: &melee::transition::Inputs<'_, '_, '_>,
    actor: UnitReference,
    defender: UnitReference,
    location: PlotCoord,
) -> Result<execution::GeneratedMembers, Error>
where
    B: proposals::Effects
        + proposals::collection_modifiers::Services<Error = <B as proposals::Effects>::Error>,
    <B as proposals::Effects>::Error: std::fmt::Debug,
{
    let a = melee::actor(state, actor).map_err(|e| error("base", e))?;
    let attack = inputs
        .generation
        .formation
        .ranged(state, a)
        .map_err(|e| error("base", e))?;
    let d = melee::actor(state, defender).map_err(|e| error("base", e))?;
    let defense = inputs
        .generation
        .formation
        .combat(state, d)
        .map_err(|e| error("base", e))?;
    let (id, _) = proposals::Context::new(state, backend)
        .propose_units_with_type(actor, defender, location, RANGED)
        .map_err(|e| error("proposal", e))?;
    let attack_modifier = melee::strength::ranged_attack_modifier(
        state,
        inputs.generation.strength,
        inputs.generation.formation,
        inputs.generation.levels,
        inputs.generation.players,
        actor,
        defender,
        id,
    )
    .map_err(|e| error("attack strength", e))?;
    let defense_modifier = melee::strength::ranged_defense_modifier(
        state,
        inputs.generation.strength,
        inputs.generation.formation,
        inputs.generation.levels,
        inputs.generation.players,
        actor,
        defender,
        id,
    )
    .map_err(|e| error("defense strength", e))?;
    let (damage, draws) = melee::damage::single_with_draws(
        state,
        inputs.generation.damage,
        defender,
        c6_core::melee_attack::damage::Strength {
            base_raw: attack.wrapping_shl(8),
            modifier_raw: attack_modifier,
        },
        c6_core::melee_attack::damage::Strength {
            base_raw: defense.wrapping_shl(8),
            modifier_raw: defense_modifier,
        },
    )
    .map_err(|e| error("damage generation", e))?;
    let defender_final = melee::actor(state, defender)
        .map_err(|e| error("final damage", e))?
        .i_damage
        .wrapping_add(damage);
    let attacker_max = inputs
        .generation
        .maximum_damage
        .maximum_damage(melee::actor(state, actor).map_err(|e| error("maximum", e))?)
        .map_err(|e| error("maximum", e))?;
    let defender_max = inputs
        .generation
        .maximum_damage
        .maximum_damage(melee::actor(state, defender).map_err(|e| error("maximum", e))?)
        .map_err(|e| error("maximum", e))?;
    proposals::collection_modifiers::forget_combat(state, backend, id)
        .map_err(|e| error("forget", e))?;
    if defender_final >= defender_max {
        return Err(error(
            "generation",
            "lethal ranged continuation unsupported",
        ));
    }
    let (attacker_xp, defender_xp) = experience::awards(
        state,
        inputs.experience,
        inputs.generation.formation,
        inputs.generation.levels,
        actor,
        defender,
        false,
        true,
    )
    .map_err(|e| error("experience generation", e))?;
    ensure_alive(state, inputs, actor, defender)?;
    Ok(execution::GeneratedMembers {
        location,
        damage: execution::GeneratedDamage {
            attacker: actor,
            defender,
            attacker_received: 0,
            defender_received: damage,
            attacker_final_damage: 0,
            defender_final_damage: defender_final,
            attacker_modifier_raw: attack_modifier,
            defender_modifier_raw: defense_modifier,
            base_strengths: c6_core::melee_attack::results::BaseStrengths {
                attacker_raw: attack.wrapping_shl(8),
                defender_raw: defense.wrapping_shl(8),
            },
            game_sync_draws: draws,
        },
        attacker_experience: attacker_xp,
        defender_experience: defender_xp,
        attacker_maximum_damage: attacker_max,
        defender_maximum_damage: defender_max,
        attacker_advances: false,
        attacker_retreats_when_killed: false,
        defender_retreats_when_killed: false,
    })
}
