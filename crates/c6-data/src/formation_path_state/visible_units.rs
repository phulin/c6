//! GetVisibleOpposingUnits and HasEnemy/HasRival for Formation node caches.
use super::*;
use c6_core::map::UnitReference;

pub(super) fn enemy(c: &Context<'_, '_, '_>, at: PlotCoord) -> Result<(bool, bool), Error> {
    has_visible_enemy_unit(
        c.state,
        &c.definitions.formation,
        c.runtime.visibility,
        c.record(),
        c.layer,
        at,
    )
}
/// The same ordered HasVisibleEnemyUnit query used by pathing and post-combat
/// advance checks. The returned tuple contains the bool result and combat out.
pub fn has_visible_enemy_unit(
    state: &LoadedState,
    definitions: &formation_rules::Definitions,
    visibility: &crate::unit_sight_state::Runtime,
    formation: &Formation,
    layer: u32,
    at: PlotCoord,
) -> Result<(bool, bool), Error> {
    let owner = formation.owner;
    let world = state.world();
    if !world.player_is_valid(owner).map_err(Error::World)? {
        return Ok((false, false));
    }
    let candidates =
        unit_placement::units_at(world, Some(layer), at).map_err(Error::PlacementQuery)?;
    let mut opposing = Vec::new();
    for &reference in candidates {
        let Some(found) = world.unit(reference).map_err(Error::World)? else {
            continue;
        };
        let unit = found.record;
        if unit.e_owner == owner {
            continue;
        }
        // This list does not apply the normal visibility, delayed-death or
        // combat predicates. Only hidden units require a hidden-layer count.
        if unit.b_has_hidden_visibility && !unit.b_hidden_visibility_forfeited {
            let hash = visibility
                .hidden_layer_hashes
                .first()
                .copied()
                .ok_or(Error::Missing("hidden visibility layer0 ID"))?;
            let layers = visibility.shared(owner).map_err(Error::Missing)?;
            let visible = layers
                .0
                .iter()
                .find(|layer| layer.type_hash == hash)
                .is_some_and(|layer| {
                    let index = layer
                        .grid_width
                        .wrapping_mul(unit.k_location.y as u32)
                        .wrapping_add(unit.k_location.x as u32)
                        as usize;
                    layer.values.get(index).is_some_and(|count| *count > 0)
                });
            if !visible {
                continue;
            }
        }
        opposing.push(reference);
    }
    if opposing.is_empty() {
        return Ok((false, false));
    }
    let relations = state.game_relations().ok();
    let war = relations.and_then(|r| {
        r.war().map(|w| unit_placement::WarRelations {
            player_count: r.player_count(),
            war_types: w.values(),
        })
    });
    let inputs = unit_placement::Inputs {
        world,
        definitions: None,
        relations: war.as_ref(),
        plot_unit_limit: None,
    };
    let mut enemy = false;
    let mut combat = false;
    if world.valid_player(owner).map_err(Error::World)?.is_some() {
        // Native HasEnemyUnit2be1780 repeats the complete inner traversal once
        // for every element in this vector; it does not stop at the first war.
        for _ in &opposing {
            for &reference in &opposing {
                let unit = resolve(state, reference)?;
                let at_war = inputs
                    .at_war_with(owner, unit.e_owner)
                    .map_err(Error::PlacementQuery)?;
                enemy |= at_war;
                if at_war {
                    combat |= crate::combat_defender::is_combat(state, definitions, unit)
                        .map_err(|_| Error::Missing("visible enemy combat getter"))?;
                }
            }
        }
    }
    if enemy {
        return Ok((true, combat));
    }
    // HasRivalUnit resets its out parameter and tests current Formation members
    // in member order against every opposing unit. Null Find results are skipped.
    let player = world.player(owner).map_err(Error::World)?;
    let mut rival = false;
    for &id in &formation.unit_ids {
        let Some(member) = player.unit(id).map_err(Error::World)? else {
            continue;
        };
        for &reference in &opposing {
            rival |= inputs
                .rivals(member.record, resolve(state, reference)?)
                .map_err(Error::PlacementQuery)?;
        }
    }
    Ok((rival, rival))
}
fn resolve(state: &LoadedState, reference: UnitReference) -> Result<&c6_core::unit::Unit, Error> {
    state
        .world()
        .unit(reference)
        .map_err(Error::World)?
        .map(|unit| unit.record)
        .ok_or(Error::Missing("visible opposing Unit invalidated"))
}
