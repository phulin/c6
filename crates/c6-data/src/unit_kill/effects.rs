//! Current Unit effect Remove leaves, using existing configured definition maps.
use super::{EffectError, Effects, Error};
use crate::{unit_initialization_definitions::Definitions, unit_sight_state as sight};
use c6_core::{
    loaded_state::LoadedState,
    movement_world::DefinitionLookup,
    unit_kill::effects as kernel,
    unit_placement_transition::{PlacementPayload, PlacementReport},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    Unsupported(u64),
    Common(c6_core::unit_modifier_effect::Effect),
    ExperienceModifier(i32),
    DiploVisibilityCombat {
        amount: i32,
        higher: bool,
    },
    SeaMovement(i32),
    ValidTerrain(u32),
    GrantAbility(u32),
    ExtractSeaArtifacts(bool),
    CombatStrength {
        amount: i32,
        kind: i32,
    },
    SeeHidden,
    /// AdjustUnitIgnoreTerrainCost with the HILLS selector. Native Apply writes
    /// the stored boolean; Remove writes its inverse, irrespective of overlap.
    IgnoreHills {
        ignore: bool,
    },
}
/// The definitions must be the existing cached maps built for this state's
/// admitted resources. No table is rebuilt during a modifier callback.
pub struct FieldRemoval {
    pub success: bool,
    pub ability_lost: Option<i32>,
}
/// AdjustUnitCombatStrength23560a0 uses wrapping native integers and selects
/// exactly one scalar. It does not require the subject to be a combat Unit.
pub fn adjust_combat_strength(unit: &mut c6_core::unit::Unit, kind: i32, amount: i32) {
    let field = match kind {
        1 => &mut unit.k_combat_modifers.i_combat_strength_modifier,
        2 => &mut unit.k_combat_modifers.i_ranged_strength_modifier,
        3 => &mut unit.k_combat_modifers.i_bombard_strength_modifier,
        4 => &mut unit.k_combat_modifers.i_anti_air_strength_modifier,
        _ => return,
    };
    *field = field.wrapping_add(amount);
}
pub fn remove_fields<C, P>(
    state: &mut LoadedState,
    definitions: &Definitions,
    actor: (i32, u32),
    effect: Effect,
) -> Result<FieldRemoval, Error<C, P>> {
    if let Effect::IgnoreHills { ignore } = effect {
        return set_ignore_hills(state, definitions, actor, !ignore)
            .map(|success| FieldRemoval {
                success,
                ability_lost: None,
            })
            .map_err(Error::State);
    }
    if let Effect::SeeHidden = effect {
        return crate::unit_creation::collections::see_hidden::apply(state, actor, true)
            .map(|success| FieldRemoval {
                success,
                ability_lost: None,
            })
            .map_err(|_| Error::Missing("SeeHidden removal Sight.UpdateMap continuation"));
    }
    if let Effect::CombatStrength { amount, kind } = effect {
        adjust_combat_strength(
            state.unit_mut(actor.0, actor.1).map_err(Error::State)?,
            kind,
            amount.wrapping_neg(),
        );
        return Ok(FieldRemoval {
            success: true,
            ability_lost: None,
        });
    }
    if let Effect::Common(effect) = effect {
        let reference = c6_core::map::UnitReference::from_component_id(super::identity(
            super::unit(state, actor)?,
        ));
        return crate::unit_modifier_effect::remove(state, Some(reference), effect)
            .map(|success| FieldRemoval {
                success,
                ability_lost: None,
            })
            .map_err(Error::CommonEffect);
    }
    let resolved = match effect {
        Effect::Unsupported(_) => {
            return Err(Error::Missing("unimplemented Unit effect Remove class"))
        }
        Effect::Common(_) => unreachable!("common UnitEffect returned above"),
        Effect::CombatStrength { .. } => unreachable!("strength effect returned above"),
        Effect::IgnoreHills { .. } => unreachable!("terrain effect returned above"),
        Effect::SeeHidden => unreachable!("SeeHidden effect returned above"),
        Effect::ExperienceModifier(n) => kernel::Effect::ExperienceModifier(n),
        Effect::DiploVisibilityCombat { amount, higher } => {
            kernel::Effect::DiploVisibilityCombat { amount, higher }
        }
        Effect::SeaMovement(n) => kernel::Effect::SeaMovement(n),
        Effect::ExtractSeaArtifacts(b) => kernel::Effect::ExtractSeaArtifacts(b),
        Effect::ValidTerrain(value) => {
            let index = definitions
                .terrains()
                .find_index(value)
                .ok_or(Error::Missing("valid-terrain effect index"))?;
            let terrain_hash = definitions
                .terrains()
                .type_hash(index)
                .ok_or(Error::Missing("valid-terrain effect hash"))?;
            let old = *state
                .unit_runtime(actor.0, actor.1)
                .map_err(Error::State)?
                .invalid_terrain
                .get(index)
                .ok_or(Error::Missing("valid-terrain current index"))?;
            state
                .set_unit_invalid_terrain(actor.0, actor.1, index, terrain_hash, !old)
                .map_err(Error::State)?;
            return Ok(FieldRemoval {
                success: true,
                ability_lost: None,
            });
        }
        Effect::GrantAbility(value) => {
            // Native ChangeAbilityCount returns before a vector write for
            // GetIndex==-1 or an index outside the current vector size.
            let Some(index) = definitions.abilities().find_index(value) else {
                return Ok(FieldRemoval {
                    success: true,
                    ability_lost: None,
                });
            };
            let runtime = state.unit_runtime(actor.0, actor.1).map_err(Error::State)?;
            let Some(old) = runtime.abilities.get(index).copied() else {
                return Ok(FieldRemoval {
                    success: true,
                    ability_lost: None,
                });
            };
            let ability_hash = definitions
                .abilities()
                .type_hash(index)
                .ok_or(Error::Missing("grant ability effect hash"))?;
            let sum = old.wrapping_sub(1);
            state
                .set_unit_ability_count(actor.0, actor.1, index, ability_hash, sum.max(0))
                .map_err(Error::State)?;
            return Ok(FieldRemoval {
                success: true,
                ability_lost: if old > 0 && sum <= 0 {
                    Some(i32::try_from(index).map_err(|_| Error::Missing("ability native index"))?)
                } else {
                    None
                },
            });
        }
    };
    let domain = if matches!(effect, Effect::SeaMovement(_)) {
        let unit = super::unit(state, actor)?;
        state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .domain
    } else {
        0
    };
    let continuation = kernel::remove(super::unit_mut(state, actor)?, resolved, domain)
        .map_err(Error::EffectFields)?;
    debug_assert_eq!(continuation, kernel::Continuation::None);
    Ok(FieldRemoval {
        success: true,
        ability_lost: None,
    })
}
/// AdjustUnitSeeHidden.Remove ignores its initialization boolean and calls
/// SetSeeHiddenType(NO_HIDDEN). The setter stores first, then performs the
/// existing complete UpdateMap(location,true) for a currently placed Unit.
pub(crate) fn remove_see_hidden<P: super::Providers, C>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    providers: &mut P,
    definitions: &crate::loaded_actions::city_range_attack::Definitions,
    actor: (i32, u32),
) -> Result<bool, Error<C, EffectError<P>>> {
    let unit = super::unit(state, actor)?;
    if unit.k_sight_modifiers.e_can_see_hidden_type == -1 {
        return Ok(true);
    }
    let location = unit.k_location;
    let placed = c6_core::skip_turn::placement::coordinate_valid(location);
    state
        .unit_mut(actor.0, actor.1)
        .map_err(Error::State)?
        .k_sight_modifiers
        .e_can_see_hidden_type = -1;
    if placed {
        sight::Context {
            state,
            runtime,
            effects: providers,
            sight_definitions: definitions,
            actor,
        }
        .update_map(location, true)
        .map_err(Error::Sight)?;
    }
    Ok(true)
}

pub fn remove_with_continuations<P: super::Providers, C: super::Continuations<P>>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    providers: &mut P,
    continuations: &mut C,
    definitions: &Definitions,
    actor: (i32, u32),
    effect: Effect,
) -> Result<bool, Error<C::Error, EffectError<P>>> {
    let result = remove_fields(state, definitions, actor, effect)?;
    if let Some(ability) = result.ability_lost {
        continuations.ability_lost(state, runtime, providers, actor, ability)?;
        let id = super::identity(super::unit(state, actor)?);
        super::send(
            state,
            providers,
            PlacementReport {
                event_hash: 0x7274f04b,
                identity: id,
                payload: PlacementPayload::Integer(ability),
            },
        )?;
    }
    Ok(result.success)
}
pub fn remove<E: Effects>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    definitions: &Definitions,
    actor: (i32, u32),
    effect: Effect,
) -> Result<bool, Error<EffectError<E>>> {
    remove_with_continuations(
        state,
        runtime,
        effects,
        &mut super::Legacy,
        definitions,
        actor,
        effect,
    )
}

/// AdjustUnitIgnoreTerrainCost2358f10/23590d0, selector -3. Iterate the current
/// terrain definitions in collection order; SetIgnoreTerrain2422af0 performs
/// a direct boolean store without callbacks, counters or RNG.
pub fn set_ignore_hills(
    state: &mut LoadedState,
    definitions: &Definitions,
    actor: (i32, u32),
    ignore: bool,
) -> Result<bool, c6_core::loaded_state::Error> {
    for (index, &hills) in definitions.hills().iter().enumerate() {
        if hills {
            let hash = definitions
                .terrains()
                .type_hash(index)
                .expect("hills metadata shares current terrain registry order");
            state.set_unit_ignore_terrain(actor.0, actor.1, index, hash, ignore)?;
        }
    }
    Ok(true)
}
