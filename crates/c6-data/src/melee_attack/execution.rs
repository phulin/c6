//! Internal AttackInto execution prefix, distinct from TestAttackInto or outer
//! UnitOperation entitlement. Full generation, damage/RNG and Resolve remain
//! mandatory; no AppliedAttack result is provided.
use super::{
    proposals, results,
    target_selection::{self, Defender, Inputs},
};
use crate::{formation_rules, loaded_upkeep};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    melee_attack::{proposals::registry, results as kernel},
    unit::PlotCoord,
};

#[derive(Debug)]
pub enum Error<E> {
    Selection(target_selection::Error),
    Clear(Box<loaded_upkeep::Error<E>>),
    Members(results::Error),
    MissingDistrictDefinitions,
}
pub enum Outcome<'s, 'e, E> {
    NoTarget,
    RequiresGeneration(Generation<'s, 'e, E>),
}
pub struct Generation<'s, 'e, E> {
    state: &'s mut LoadedState,
    actor: UnitReference,
    defender: Defender,
    members: kernel::Members,
    attempted: bool,
    effects: &'e mut E,
}
#[derive(Debug)]
pub enum GenerateError<E> {
    AlreadyAttempted,
    Members(results::Error),
    Unsupported(&'static str),
    Proposal(Box<registry::DispatchError<proposals::Error<E>>>),
}
#[must_use]
pub struct RequiresStrengthModifiers {
    pub proposal_id: u32,
    pub proposal: registry::Handle,
    pub base_strengths: kernel::BaseStrengths,
}
pub struct GenerateRules<'a> {
    pub formation: &'a formation_rules::Definitions,
    pub strength: &'a super::strength::Definitions,
    pub damage: &'a super::damage::Definitions,
    pub maximum_damage: &'a crate::formation_movement_cost::Definitions<'a>,
    pub levels: &'a [(i32, u32)],
    pub players: &'a super::strength::CurrentPlayerCombat<'a>,
}
/// Generated numeric members, after both real draws and temporary-proposal
/// destruction. Experience/member finalization and Resolve are still required.
#[derive(Debug)]
pub struct GeneratedDamage {
    pub attacker: UnitReference,
    pub defender: UnitReference,
    pub attacker_received: i32,
    pub defender_received: i32,
    pub attacker_final_damage: i32,
    pub defender_final_damage: i32,
    pub attacker_modifier_raw: i32,
    pub defender_modifier_raw: i32,
    pub base_strengths: kernel::BaseStrengths,
}
#[derive(Debug)]
pub enum DamageError<E> {
    Generate(GenerateError<E>),
    Attack(Box<c6_core::melee_attack::strength::AttackError<super::strength::Error>>),
    Defense(Box<c6_core::melee_attack::strength::DefenseError<super::strength::Error>>),
    Damage(super::damage::Error),
    Actor(super::Error),
    Combat(crate::loaded_actions::Error),
    Maximum(crate::formation_movement_cost::Error),
    State(c6_core::loaded_state::Error),
    ProposalAccess(c6_core::loaded_state::combat_proposals::AccessError),
    Proposal(registry::Error),
    Cleanup(
        Box<
            c6_core::melee_attack::proposals::collection_modifiers::Error<
                super::proposals::collection_modifiers::Error<E>,
            >,
        >,
    ),
}

/// Fresh source selection: religion, enemy defensible District, otherwise
/// CanAttackDomain/GetBestDefender flags9/19. No readiness or CanAttackTarget
/// query is repeated and no previously tested defender is accepted as input.
pub fn begin<'s, 'e, E: loaded_upkeep::OperationReports>(
    state: &'s mut LoadedState,
    inputs: &Inputs<'_>,
    actor: UnitReference,
    destination: PlotCoord,
    effects: &'e mut E,
) -> Result<Outcome<'s, 'e, E>, Error<E::Error>> {
    let Some(defender) =
        target_selection::select_execution_target(state, inputs, actor, destination)
            .map_err(Error::Selection)?
    else {
        return Ok(Outcome::NoTarget);
    };
    let mut members = kernel::Members::new(c6_core::formation_path_cache::INVALID_CACHE_LOCATION);
    match defender {
        Defender::Unit(unit) => {
            let id = unit.component_id();
            // Source28c9800: clear the freshly selected DEFENDER before member
            // construction/Generate. Callbacks may mutate this same live state.
            loaded_upkeep::clear_unit_operations(state, i32::from(id.owner), id.id, effects)
                .map_err(|e| Error::Clear(Box::new(e)))?;
            let _next = results::prepare_units(state, &mut members, actor, unit, 0)
                .map_err(Error::Members)?;
        }
        Defender::District(_) => {
            // District branch has no pre-Generate Unit Clear.
            let definitions = inputs.districts.ok_or(Error::MissingDistrictDefinitions)?;
            let _next =
                results::prepare_district(state, definitions, &mut members, actor, destination, 0)
                    .map_err(Error::Members)?;
        }
    }
    Ok(Outcome::RequiresGeneration(Generation {
        state,
        actor,
        defender,
        members,
        attempted: false,
        effects,
    }))
}
impl<E> Generation<'_, '_, E> {
    pub fn state(&self) -> &LoadedState {
        self.state
    }
    pub fn members(&self) -> &kernel::Members {
        &self.members
    }
    pub fn selected_defender(&self) -> Defender {
        self.defender
    }
    /// Continue source-ordered base strengths and actual proposal registration.
    /// Failures are not replayable: the defender Clear, registration and earlier
    /// callback effects remain in the authoritative state for diagnosis.
    pub fn advance_proposal(
        &mut self,
        definitions: &formation_rules::Definitions,
    ) -> Result<RequiresStrengthModifiers, GenerateError<<E as proposals::Effects>::Error>>
    where
        E: proposals::Effects,
    {
        if self.attempted {
            return Err(GenerateError::AlreadyAttempted);
        }
        self.attempted = true;
        let Defender::Unit(defender) = self.defender else {
            return Err(GenerateError::Unsupported("Unit/District generation"));
        };
        let strengths = results::current_unit_strengths(
            self.state,
            definitions,
            self.actor,
            defender,
            self.members.location,
        )
        .map_err(GenerateError::Members)?;
        let kernel::UnitGeneration::RequiresProposeCombat(base_strengths) = strengths else {
            return Err(GenerateError::Unsupported(
                "support FormationClass generation",
            ));
        };
        let (proposal_id, proposal) = proposals::Context::new(self.state, self.effects)
            .propose_units(self.actor, defender, self.members.location)
            .map_err(|e| GenerateError::Proposal(Box::new(e)))?;
        // Attacker fortify reset, attacker Clear and CompleteMovement occur
        // only AFTER successful complete Generate, not after this prefix.
        Ok(RequiresStrengthModifiers {
            proposal_id,
            proposal,
            base_strengths,
        })
    }
    /// Source ordinary Unit/Unit path: proposal callbacks, attack then defense
    /// modifiers, forward then retaliation damage, mutual-death correction,
    /// then actual ForgetCombat/destruction before XP/member result fields.
    pub fn advance_damage(
        &mut self,
        rules: &GenerateRules<'_>,
    ) -> Result<GeneratedDamage, DamageError<<E as proposals::Effects>::Error>>
    where
        E: proposals::Effects
            + proposals::collection_modifiers::Services<Error = <E as proposals::Effects>::Error>,
    {
        let next = self
            .advance_proposal(rules.formation)
            .map_err(DamageError::Generate)?;
        let Defender::Unit(defender) = self.defender else {
            unreachable!("advance_proposal checks Unit");
        };
        let attacker_modifier_raw = super::strength::ordinary_attack_modifier(
            self.state,
            rules.strength,
            rules.formation,
            rules.levels,
            rules.players,
            self.actor,
            defender,
            next.proposal_id,
        )
        .map_err(|e| DamageError::Attack(Box::new(e)))?;
        let defender_modifier_raw = super::strength::ordinary_defense_modifier(
            self.state,
            rules.strength,
            rules.formation,
            rules.levels,
            rules.players,
            self.actor,
            defender,
            next.proposal_id,
        )
        .map_err(|e| DamageError::Defense(Box::new(e)))?;
        let mut exchange = super::damage::exchange(
            self.state,
            rules.damage,
            defender,
            c6_core::melee_attack::damage::Strength {
                base_raw: next.base_strengths.attacker_raw,
                modifier_raw: attacker_modifier_raw,
            },
            c6_core::melee_attack::damage::Strength {
                base_raw: next.base_strengths.defender_raw,
                modifier_raw: defender_modifier_raw,
            },
            false,
        )
        .map_err(DamageError::Damage)?;
        if rules
            .formation
            .formation_class(
                self.state,
                super::actor(self.state, defender).map_err(DamageError::Actor)?,
            )
            .map_err(DamageError::Combat)?
            == 1
        {
            exchange.defender_received = rules.damage.maximum_hit_points();
        }
        let mut defender_final_damage = super::actor(self.state, defender)
            .map_err(DamageError::Actor)?
            .i_damage;
        let mut attacker_final_damage = super::actor(self.state, self.actor)
            .map_err(DamageError::Actor)?
            .i_damage;
        let defender_maximum = rules
            .maximum_damage
            .maximum_damage(super::actor(self.state, defender).map_err(DamageError::Actor)?)
            .map_err(DamageError::Maximum)?;
        defender_final_damage = defender_final_damage.wrapping_add(exchange.defender_received);
        attacker_final_damage = attacker_final_damage.wrapping_add(exchange.attacker_received);
        if defender_final_damage >= defender_maximum {
            let captures = self
                .state
                .combat_proposals()
                .map_err(DamageError::ProposalAccess)?
                .proposal(next.proposal)
                .map_err(DamageError::Proposal)?
                .capture_combat_units;
            if captures {
                // Source still reads both domains before skipping preview text
                // in this native no-Results-preview profile.
                self.state
                    .unit_runtime(i32::from(defender.component_id().owner), defender.id)
                    .map_err(DamageError::State)?;
                self.state
                    .unit_runtime(i32::from(self.actor.component_id().owner), self.actor.id)
                    .map_err(DamageError::State)?;
            }
            let attacker_maximum = rules
                .maximum_damage
                .maximum_damage(super::actor(self.state, self.actor).map_err(DamageError::Actor)?)
                .map_err(DamageError::Maximum)?;
            if attacker_final_damage >= attacker_maximum {
                let a_max = rules
                    .maximum_damage
                    .maximum_damage(
                        super::actor(self.state, self.actor).map_err(DamageError::Actor)?,
                    )
                    .map_err(DamageError::Maximum)?;
                let d_max = rules
                    .maximum_damage
                    .maximum_damage(super::actor(self.state, defender).map_err(DamageError::Actor)?)
                    .map_err(DamageError::Maximum)?;
                if attacker_final_damage.wrapping_sub(a_max)
                    > defender_final_damage.wrapping_sub(d_max)
                {
                    let maximum = rules
                        .maximum_damage
                        .maximum_damage(
                            super::actor(self.state, defender).map_err(DamageError::Actor)?,
                        )
                        .map_err(DamageError::Maximum)?;
                    let remaining = maximum.wrapping_sub(
                        super::actor(self.state, defender)
                            .map_err(DamageError::Actor)?
                            .i_damage,
                    );
                    exchange.defender_received = remaining.wrapping_sub(1);
                    defender_final_damage = remaining
                        .wrapping_add(
                            super::actor(self.state, defender)
                                .map_err(DamageError::Actor)?
                                .i_damage,
                        )
                        .wrapping_sub(1);
                } else {
                    let maximum = rules
                        .maximum_damage
                        .maximum_damage(
                            super::actor(self.state, self.actor).map_err(DamageError::Actor)?,
                        )
                        .map_err(DamageError::Maximum)?;
                    let remaining = maximum.wrapping_sub(
                        super::actor(self.state, self.actor)
                            .map_err(DamageError::Actor)?
                            .i_damage,
                    );
                    exchange.attacker_received = remaining.wrapping_sub(1);
                    attacker_final_damage = remaining
                        .wrapping_add(
                            super::actor(self.state, self.actor)
                                .map_err(DamageError::Actor)?
                                .i_damage,
                        )
                        .wrapping_sub(1);
                }
            }
        }
        proposals::collection_modifiers::forget_combat(self.state, self.effects, next.proposal_id)
            .map_err(|e| DamageError::Cleanup(Box::new(e)))?;
        Ok(GeneratedDamage {
            attacker: self.actor,
            defender,
            attacker_received: exchange.attacker_received,
            defender_received: exchange.defender_received,
            attacker_final_damage,
            defender_final_damage,
            attacker_modifier_raw,
            defender_modifier_raw,
            base_strengths: next.base_strengths,
        })
    }
}
