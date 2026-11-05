//! Outer District/Unit Resolve tail, after the genuine district resolver.
//! No attacking Unit exists: unit-only aftermath and PlayerAttacked are guarded
//! out by the original code. Supported survivor callbacks use canonical owners.
use super::{error, Error, Generated};
use crate::melee_attack::{self as melee, resolve::completion::stock};
use c6_core::loaded_state::LoadedState;

pub fn finish<E: stock::DistrictEffects>(
    state: &mut LoadedState,
    bindings: &mut stock::Bindings,
    inputs: &stock::Inputs<'_, '_, '_>,
    effects: &mut E,
    members: &Generated,
) -> Result<(), Error>
where
    E::Error: std::fmt::Debug,
{
    // AwardPostCombatEffects requires Unit/Unit versus. Weariness supports the
    // actual District/Unit pair and then checks the two current major flags.
    let attacker = i32::from(members.attacker.owner);
    let defender = melee::actor(state, members.defender)
        .map_err(|e| error("completion defender", e))?
        .e_owner;
    let major = |owner| {
        inputs
            .civilization_levels
            .iter()
            .find(|r| r.0 == owner)
            .map(|r| r.1 == 0x253718b0)
            .ok_or_else(|| error("war weariness", ("missing level", owner)))
    };
    if attacker != -1 {
        let attacker_major = major(attacker)?;
        if defender != -1 && attacker_major && major(defender)? {
            return Err(error(
                "war weariness",
                "major-major District combat continuation",
            ));
        }
    }
    if melee::actor(state, members.defender)
        .map_err(|e| error("completion defender", e))?
        .e_activity as u32
        == 0xd0aed661
    {
        effects
            .clear_defender_operations(state, bindings, members.defender)
            .map_err(|e| error("defender operations", e))?;
        effects
            .set_defender_awake(state, bindings, members.defender)
            .map_err(|e| error("defender awake", e))?;
    }
    let defender_dead = melee::actor(state, members.defender)
        .map_err(|e| error("combat statistics", e))?
        .b_delayed_death;
    state
        .record_game_combat_death(false, || Ok(defender_dead))
        .map_err(|e| error("combat statistics", e))?;
    stock::district_combat_occurred(state, bindings, inputs, effects, members)
        .map_err(|e| error("combat occurred", e))?;
    // ResolveCombat tests both Unit pointers before Player.Signals+78.
    Ok(())
}
impl<P> stock::DistrictEffects for crate::loaded_actions::stock_notifications::Runtime<'_, P> {
    fn lua_district_combat_occurred(
        &mut self,
        state: &mut LoadedState,
        _: &mut stock::Bindings,
        members: &Generated,
    ) -> Result<(), Self::Error> {
        self.script
            .on_combat_occurred(state, None, Some(members.defender))
            .map_err(crate::loaded_actions::stock_notifications::movement::Error::Script)
    }
}
