//! Movement.IsOccupiedByEnemy(owner, district plot, true). Only current layer
//! zero stack members are candidates; noncombat units never query diplomacy.
use super::{city_range_attack, Definitions, Error};
use c6_core::{loaded_state::LoadedState, unit::PlotCoord};

pub(super) fn occupied_by_enemy(
    state: &LoadedState,
    definitions: &Definitions,
    owner: i32,
    location: (i32, i32),
) -> Result<bool, Error> {
    state
        .world()
        .player(owner)
        .map_err(|_| Error::Missing("current occupation player"))?;
    let (index, _) = c6_core::movement_world::plot_at(
        state.world().map(),
        PlotCoord {
            x: location.0,
            y: location.1,
        },
    )
    .map_err(|_| Error::Missing("current occupation district plot"))?;
    // GetUnits(location, 0) returns an empty vector for an absent layer.
    let map = state.world().map();
    let Some(layer) = map.units.layers.iter().find(|l| l.layer_id == 0) else {
        return Ok(false);
    };
    if index >= layer.maximum as usize {
        return Ok(false);
    }
    for &reference in state
        .world()
        .plot_units(0, index)
        .map_err(|_| Error::Missing("current occupation plot stack"))?
    {
        let Some(found) = state
            .world()
            .unit(reference)
            .map_err(|_| Error::Missing("current occupation unit"))?
        else {
            continue;
        };
        let unit = found.record;
        if unit.b_delayed_death {
            continue;
        }
        // Native IsCombat short circuits GetCombat(false), GetRangedCombat,
        // GetBombardCombat in that order, testing each signed result > 0.
        if !(city_range_attack::unit_combat(state, unit, &definitions.combat)
            .map_err(Error::Combat)?
            > 0
            || city_range_attack::unit_ranged_combat(state, unit, &definitions.combat)
                .map_err(Error::Combat)?
                > 0
            || city_range_attack::unit_bombard_combat(state, unit, &definitions.combat)
                .map_err(Error::Combat)?
                > 0)
        {
            continue;
        }
        // The resolved unit already established GetValidPlayer. Diplomacy's
        // IsAtWarWith next tests the target player's current IsAlive value.
        if !state
            .player_alive(unit.e_owner)
            .map_err(|_| Error::Missing("current occupation target alive"))?
        {
            continue;
        }
        if !state
            .game_relations()
            .map_err(|_| Error::Missing("current occupation diplomacy"))?
            .is_at_war(owner, unit.e_owner)
            .map_err(|_| Error::Missing("current occupation war relationship"))?
        {
            continue;
        }
        // The caller passes true. Native only returns true for an owner whose
        // current GetLevel() is CIVILIZATION_LEVEL_FULL_CIV (0x253718b0).
        // That unsaved classification is not yet represented here. Preserve
        // this reached branch as unsupported rather than guessing from slot ID.
        return Err(Error::Missing(
            "current occupation owner civilization level",
        ));
    }
    Ok(false)
}
