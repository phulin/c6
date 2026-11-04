//! Fresh post-kill proposal and current post-combat yields. This continuation
//! still requires retreat, weariness, combat signals and outer resolution.
use super::{kill_awards, post_kill::RequiresPostCombatProposal, unit_era, GeneratedMembers};
use crate::loaded_actions::configuration::Configuration;
use crate::melee_attack::proposals::{self, collection_modifiers};
use c6_core::{loaded_state::LoadedState, map::UnitReference, melee_attack::proposals::registry};

#[derive(Debug)]
pub enum Error<E> {
    Proposal(Box<registry::DispatchError<proposals::Error<E>>>),
    Forget(
        Box<
            c6_core::melee_attack::proposals::collection_modifiers::Error<
                collection_modifiers::Error<E>,
            >,
        >,
    ),
    Access(c6_core::loaded_state::combat_proposals::AccessError),
    Runtime(registry::Error),
    Actor(super::super::Error),
    Era(unit_era::Error),
    Configuration(crate::loaded_actions::configuration::Error),
    Effects(E),
    Missing(&'static str),
}
#[must_use = "retreat, weariness and combat signals remain before action completion"]
pub struct RequiresCombatAftermath {
    pub members: GeneratedMembers,
}
pub struct Inputs<'a, 'c> {
    pub eras: &'a unit_era::Definitions,
    pub awards: &'a kill_awards::Definitions,
    pub configuration: &'a Configuration<'c>,
}
/// Credit the real current Player component, including its mandatory callbacks,
/// then perform the source visibility-gated reward feedback. Never a DTO stub.
pub trait Effects:
    proposals::Effects + collection_modifiers::Services<Error = <Self as proposals::Effects>::Error>
{
    fn credit_post_combat_yield(
        &mut self,
        state: &mut LoadedState,
        winner: UnitReference,
        yield_type: i32,
        amount: i32,
    ) -> Result<(), <Self as proposals::Effects>::Error>;
}
fn raw_amount(
    base: &mut i32,
    religious: i32,
    row: &registry::PostCombatYield,
    winner_era: i32,
    loser_era: i32,
) -> Option<i32> {
    if row.option_2 {
        *base = religious;
    }
    if row.option_1 && winner_era <= loser_era {
        return None;
    }
    Some(row.amount.wrapping_mul(*base) / 100)
}
pub fn apply<B: Effects>(
    state: &mut LoadedState,
    backend: &mut B,
    inputs: &Inputs<'_, '_>,
    pending: RequiresPostCombatProposal,
) -> Result<RequiresCombatAftermath, Error<<B as proposals::Effects>::Error>> {
    if !pending.needs_proposal {
        return Ok(RequiresCombatAftermath {
            members: pending.members,
        });
    }
    let kill = pending
        .kill
        .ok_or(Error::Missing("post-combat kill participant"))?;
    let location = pending.members.location;
    if state
        .world()
        .map()
        .plot_at(location.x, location.y)
        .is_none()
    {
        return Err(Error::Missing("post-combat Results plot"));
    }
    let (id, _) = proposals::Context::new(state, backend)
        .propose_units(
            pending.members.damage.attacker,
            pending.members.damage.defender,
            location,
        )
        .map_err(|e| Error::Proposal(Box::new(e)))?;
    // ObjectCreated may remove or replace the current registry entry. The
    // returned local shared handle is not a substitute for GetCombatInfo(id).
    let active = state
        .combat_proposals()
        .map_err(Error::Access)?
        .active()
        .iter()
        .find(|entry| entry.0 == id)
        .map(|entry| entry.1);
    if let Some(handle) = active {
        // Both era queries occur even when the current yield vector is empty.
        let winner_era = inputs
            .eras
            .chronology(state, kill.winner)
            .map_err(Error::Era)?;
        let loser_era = inputs
            .eras
            .chronology(state, kill.loser)
            .map_err(Error::Era)?;
        let winner_owner = super::super::actor(state, kill.winner)
            .map_err(Error::Actor)?
            .e_owner;
        let attacker = state
            .combat_proposals()
            .map_err(Error::Access)?
            .proposal(handle)
            .map_err(Error::Runtime)?
            .attacker;
        let attacker_owner = match attacker {
            registry::Participant::Unit(unit) => {
                super::super::actor(state, unit)
                    .map_err(Error::Actor)?
                    .e_owner
            }
            registry::Participant::District(_) => {
                return Err(Error::Missing("post-combat district participant"))
            }
        };
        let attacker_side = attacker_owner == winner_owner;
        let mut base = kill.captured_combat;
        let religious = if kill.religious_strength > 0 {
            kill.religious_strength
        } else {
            -100
        };
        let mut index = 0;
        loop {
            let row = {
                let proposal = state
                    .combat_proposals()
                    .map_err(Error::Access)?
                    .proposal(handle)
                    .map_err(Error::Runtime)?;
                let rows = if attacker_side {
                    &proposal.attacker_yields
                } else {
                    &proposal.defender_yields
                };
                rows.get(index).cloned()
            };
            let Some(row) = row else {
                break;
            };
            index += 1;
            let Some(whole) = raw_amount(&mut base, religious, &row, winner_era, loser_era) else {
                continue;
            };
            inputs
                .configuration
                .require_binding(state.binding())
                .map_err(Error::Configuration)?;
            let speed = inputs
                .configuration
                .game_fields()
                .game_speed_type
                .ok_or(Error::Missing("current post-combat game speed"))?;
            let amount = kill_awards::adjust_speed(whole, inputs.awards.cost(speed));
            if amount > 0 {
                backend
                    .credit_post_combat_yield(state, kill.winner, row.yield_type, amount)
                    .map_err(Error::Effects)?;
            }
        }
    }
    collection_modifiers::forget_combat(state, backend, id)
        .map_err(|e| Error::Forget(Box::new(e)))?;
    Ok(RequiresCombatAftermath {
        members: pending.members,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skipped_religious_row_changes_base_for_following_ordinary_row() {
        let mut base = 85;
        let mut row = registry::PostCombatYield {
            yield_type: 0,
            amount: 25,
            option_1: true,
            option_2: true,
        };
        assert_eq!(raw_amount(&mut base, -100, &row, 1, 1), None);
        row.option_1 = false;
        row.option_2 = false;
        assert_eq!(raw_amount(&mut base, -100, &row, 1, 1), Some(-25));
    }
    #[test]
    fn wrapping_product_precedes_signed_division() {
        let mut base = i32::MAX;
        let row = registry::PostCombatYield {
            yield_type: 0,
            amount: 200,
            option_1: false,
            option_2: false,
        };
        assert_eq!(raw_amount(&mut base, 0, &row, 0, 0), Some(-2));
    }
}
