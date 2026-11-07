//! Current Unit/Unit war weariness. Ordinary melee has no WMD contribution.
//! Each side uses its own land/death predicate; attacker mutation precedes the
//! defender queries. The diplomacy change publishes only a logging record.
pub use crate::melee_attack::city::weariness::Definitions;
use crate::melee_attack::{
    city::{
        error,
        weariness::{delta, player},
        Error,
    },
    execution::GeneratedMembers,
};
use c6_core::loaded_state::LoadedState;

pub fn apply(
    state: &mut LoadedState,
    definitions: &Definitions,
    maximum: &crate::formation_movement_cost::Definitions<'_>,
    members: &GeneratedMembers,
) -> Result<(), Error> {
    if state.binding() != &definitions.binding {
        return Err(error("weariness", "rules identity"));
    }
    let attacker = crate::melee_attack::actor(state, members.damage.attacker)
        .map_err(|e| error("weariness attacker", e))?
        .e_owner;
    let defender = crate::melee_attack::actor(state, members.damage.defender)
        .map_err(|e| error("weariness defender", e))?
        .e_owner;
    if !(0..64).contains(&attacker) || !(0..64).contains(&defender) {
        return Err(error("weariness owners", "current player slots"));
    }
    for (unit, owner, other, attacking) in [
        (members.damage.attacker, attacker, defender, true),
        (members.damage.defender, defender, attacker, false),
    ] {
        let (_, plot) = c6_core::movement_world::plot_at(state.world().map(), members.location)
            .map_err(|e| error("weariness plot", e))?;
        let plot_owner = crate::movement_placement_state::plot_owner(plot)
            .map_err(|e| error("weariness plot", e))?;
        let allied = plot_owner == owner
            || (plot_owner != -1
                && state
                    .game_relations()
                    .map_err(|e| error("weariness relations", e))?
                    .is_allied(owner, plot_owner)
                    .map_err(|e| error("weariness ally", e))?);
        let mut combat = if allied {
            definitions.allied
        } else {
            definitions.foreign
        };
        let unit =
            crate::melee_attack::actor(state, unit).map_err(|e| error("weariness unit", e))?;
        if unit.i_damage
            >= maximum
                .maximum_damage(unit)
                .map_err(|e| error("weariness maximum", e))?
        {
            combat = combat.wrapping_add(definitions.killed);
        }
        let dow = definitions.dow(state, owner, other)?;
        let factor = if attacking {
            let era = state
                .game_eras()
                .map_err(|e| error("weariness era", e))?
                .current_era()
                .ok_or_else(|| error("weariness era", "unknown current era"))?;
            let era = usize::try_from(era)
                .ok()
                .and_then(|i| definitions.eras.get(i))
                .copied()
                .ok_or_else(|| error("weariness era", "current definition index"))?;
            (era.wrapping_mul(dow) / 100).wrapping_add(definitions.base)
        } else {
            definitions.base.wrapping_sub(dow)
        };
        let own = &player(state, owner)?.diplomacy.war_and_alliance_modifiers;
        let enemy = &player(state, other)?.diplomacy.war_and_alliance_modifiers;
        let modifier = own[2]
            .wrapping_add(enemy[0])
            .wrapping_add(if allied { own[1] } else { 0 });
        let amount = delta(modifier, factor, combat);
        if amount > 0 {
            let diplomacy = state
                .war_diplomacy_mut(owner)
                .map_err(|e| error("weariness mutation", e))?;
            diplomacy.war_weariness[other as usize] = diplomacy.war_weariness[other as usize]
                .wrapping_add(amount)
                .max(0);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::delta;
    #[test]
    fn combat_amount_preserves_native_threshold_and_unsigned_division() {
        assert_eq!(delta(0, 16, 2), 32);
        assert_eq!(delta(-99, 16, 2), 0);
        assert_eq!(delta(-100, 16, 2), 0);
        assert_eq!(delta(-99, 100, 1), 1);
        assert_eq!(delta(-99, i32::MAX, 1), 21_474_836);
        assert_eq!(delta(0, i32::MAX, 1), 0);
    }
}
