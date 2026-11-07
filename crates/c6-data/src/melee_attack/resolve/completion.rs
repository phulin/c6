//! Outer Resolve tail after kill awards, loyalty/religion and fresh proposal.
//! Callback providers must execute real current subscribers before returning.
pub mod army_knowledge;
pub mod boosts;
pub mod eras;
pub mod history;
pub mod mayhem;
pub mod stock;
pub mod stock_guards;
pub mod targets;
pub mod weariness;
use super::{post_yields::RequiresCombatAftermath, GeneratedMembers};
use c6_core::{loaded_state::LoadedState, map::UnitReference};
#[derive(Debug)]
pub enum Error<E> {
    Actor(super::super::Error),
    Statistics(c6_core::loaded_state::game_statistics::AccessError),
    Effect(E),
    MissingLevel(i32),
    MissingWarWeariness,
    WarWeariness(crate::melee_attack::city::Error),
    RetreatWhenKilled,
}
/// Resolved gameplay results. The containing operation/queued action still has
/// its own continuation and must not equate this value with action completion.
#[must_use]
pub struct ResolvedCombat {
    pub members: GeneratedMembers,
}
pub trait Effects {
    type Error;
    fn clear_defender_operations(
        &mut self,
        state: &mut LoadedState,
        defender: UnitReference,
    ) -> Result<(), Self::Error>;
    fn set_defender_awake(
        &mut self,
        state: &mut LoadedState,
        defender: UnitReference,
    ) -> Result<(), Self::Error>;
    /// Game.Signals+4c8, after current Game.Stats.OnCombatOccuredBeforeSignal.
    fn combat_occurred(
        &mut self,
        state: &mut LoadedState,
        members: &GeneratedMembers,
    ) -> Result<(), Self::Error>;
    /// CURRENT defender owner Player.Signals+78, payload retained attacker.
    fn defender_attacked(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        attacker: UnitReference,
    ) -> Result<(), Self::Error>;
}
fn major_pair<E>(
    attacker: i32,
    defender: i32,
    mut major: impl FnMut(i32) -> Result<bool, E>,
) -> Result<bool, E> {
    if attacker == -1 {
        return Ok(false);
    }
    let attacker_major = major(attacker)?;
    if defender == -1 || !attacker_major {
        return Ok(false);
    }
    major(defender)
}
/// Ordinary unit/unit combat, including current major-pair war weariness.
pub fn finish<E: Effects>(
    state: &mut LoadedState,
    effects: &mut E,
    levels: &[(i32, u32)],
    weariness: Option<&weariness::Definitions>,
    maximum: &crate::formation_movement_cost::Definitions<'_>,
    pending: RequiresCombatAftermath,
) -> Result<ResolvedCombat, Error<E::Error>> {
    let members = pending.members;
    if members.attacker_retreats_when_killed || members.defender_retreats_when_killed {
        return Err(Error::RetreatWhenKilled);
    }
    let attacker = members.damage.attacker;
    let defender = members.damage.defender;
    let attacker_owner = super::super::actor(state, attacker)
        .map_err(Error::Actor)?
        .e_owner;
    let defender_owner = super::super::actor(state, defender)
        .map_err(Error::Actor)?
        .e_owner;
    if major_pair(attacker_owner, defender_owner, |owner| {
        levels
            .iter()
            .find(|row| row.0 == owner)
            .map(|row| row.1 == 0x253718b0)
            .ok_or(Error::MissingLevel(owner))
    })? {
        weariness::apply(
            state,
            weariness.ok_or(Error::MissingWarWeariness)?,
            maximum,
            &members,
        )
        .map_err(Error::WarWeariness)?;
    }
    if super::super::actor(state, defender)
        .map_err(Error::Actor)?
        .e_activity as u32
        == 0xd0aed661
    {
        effects
            .clear_defender_operations(state, defender)
            .map_err(Error::Effect)?;
        effects
            .set_defender_awake(state, defender)
            .map_err(Error::Effect)?;
    }
    // The native statistics hook queries the defender only if the attacker is
    // not delayed-dead, and increments once even when both are delayed-dead.
    let attacker_dead = super::super::actor(state, attacker)
        .map_err(Error::Actor)?
        .b_delayed_death;
    let defender_dead = if attacker_dead {
        false
    } else {
        super::super::actor(state, defender)
            .map_err(Error::Actor)?
            .b_delayed_death
    };
    state
        .record_game_combat_death(attacker_dead, || Ok(defender_dead))
        .map_err(Error::Statistics)?;
    effects
        .combat_occurred(state, &members)
        .map_err(Error::Effect)?;
    let current_defender_owner = super::super::actor(state, defender)
        .map_err(Error::Actor)?
        .e_owner;
    effects
        .defender_attacked(state, current_defender_owner, attacker)
        .map_err(Error::Effect)?;
    Ok(ResolvedCombat { members })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weariness_queries_attacker_before_no_defender_guard_and_skips_nonmajor() {
        let mut seen = vec![];
        assert!(!major_pair(0, -1, |owner| {
            seen.push(owner);
            Ok::<_, ()>(true)
        })
        .unwrap());
        assert_eq!(seen, [0]);
        seen.clear();
        assert!(!major_pair(0, 63, |owner| {
            seen.push(owner);
            Ok::<_, ()>(owner == 0)
        })
        .unwrap());
        assert_eq!(seen, [0, 63]);
        assert!(!major_pair::<()>(-1, 63, |_| panic!("native skips lookup")).unwrap_or(false));
    }
}
