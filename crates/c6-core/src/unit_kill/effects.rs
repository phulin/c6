//! Source-bound Unit effect Remove leaves reached by modifier destruction.
//! Effect gating, tracking and synchronous ability-loss dispatch are separate.
use crate::unit::Unit;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    ExperienceModifier(i32),
    DiploVisibilityCombat { amount: i32, higher: bool },
    SeaMovement(i32),
    ValidTerrain { terrain_hash: u32 },
    GrantAbility { ability_hash: u32 },
    ExtractSeaArtifacts(bool),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    MissingTerrainCell,
    MissingAbilityCell,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Continuation {
    None,
    AbilityLost,
}

/// Apply the real leaf's field changes. A returned AbilityLost requires the
/// caller to execute signal1578 and report7274f04b after the count write.
/// Domain is read only by the SeaMovement branch. Sparse missing vector cells
/// remain unsupported rather than assigning invented initialization values.
pub fn remove(unit: &mut Unit, effect: Effect, domain: i32) -> Result<Continuation, Error> {
    match effect {
        Effect::ExperienceModifier(amount) => {
            // ChangeGeneralExperienceModifier converts the wrapping negative
            // integer through binary32 percent then the shared native Q8 ctor.
            let percent = (amount.wrapping_neg() as f32) * f32::from_bits(0x3c23d70a);
            let change = crate::movement::MovementPoints::route_float_bits(percent.to_bits()).0;
            unit.k_experience.x_general_experience_modifier = unit
                .k_experience
                .x_general_experience_modifier
                .wrapping_add(change);
        }
        Effect::DiploVisibilityCombat { amount, higher } => {
            let field = if higher {
                &mut unit.k_combat_modifers.i_higher_diplo_viz_combat_mod
            } else {
                &mut unit.k_combat_modifers.i_each_diplo_viz_combat_mod
            };
            *field = field.wrapping_sub(amount).clamp(0, 100);
        }
        Effect::SeaMovement(amount) => {
            if domain == 0 {
                unit.k_movement_modifiers.i_extra_moves =
                    unit.k_movement_modifiers.i_extra_moves.wrapping_sub(amount);
            }
        }
        Effect::ValidTerrain { terrain_hash } => {
            let field = unit
                .ab_invalid_terrain_types
                .iter_mut()
                .find(|r| r.definition_hash == terrain_hash)
                .ok_or(Error::MissingTerrainCell)?;
            // Remove does not restore the effect's configured bool; it toggles
            // the CURRENT invalid flag via GetInvalidTerrain/Set/RemoveInvalid.
            field.value = !field.value;
        }
        Effect::GrantAbility { ability_hash } => {
            let field = unit
                .ai_abilities_possessed
                .iter_mut()
                .find(|r| r.definition_hash == ability_hash)
                .ok_or(Error::MissingAbilityCell)?;
            let sum = field.value.wrapping_sub(1);
            let lost = field.value > 0 && sum <= 0;
            field.value = sum.max(0);
            if lost {
                return Ok(Continuation::AbilityLost);
            }
        }
        Effect::ExtractSeaArtifacts(enabled) => {
            unit.k_archaeology.b_can_extract_sea_artifacts = !enabled;
        }
    }
    Ok(Continuation::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::HashValue;
    fn unit() -> Unit {
        crate::unit_turn_upkeep::tests::source_only_unit()
    }
    #[test]
    fn negative_experience_percent_uses_native_floor_fraction_constructor() {
        let mut u = unit();
        u.k_experience.x_general_experience_modifier = 281;
        remove(&mut u, Effect::ExperienceModifier(10), 2).unwrap();
        assert_eq!(u.k_experience.x_general_experience_modifier, 255);
    }
    #[test]
    fn diplo_visibility_wraps_before_clamping() {
        let mut u = unit();
        u.k_combat_modifers.i_higher_diplo_viz_combat_mod = i32::MIN;
        remove(
            &mut u,
            Effect::DiploVisibilityCombat {
                amount: 1,
                higher: true,
            },
            2,
        )
        .unwrap();
        assert_eq!(u.k_combat_modifers.i_higher_diplo_viz_combat_mod, 100);
    }
    #[test]
    fn sea_movement_skips_land_and_wraps_sea() {
        let mut u = unit();
        u.k_movement_modifiers.i_extra_moves = i32::MIN;
        remove(&mut u, Effect::SeaMovement(1), 2).unwrap();
        assert_eq!(u.k_movement_modifiers.i_extra_moves, i32::MIN);
        remove(&mut u, Effect::SeaMovement(1), 0).unwrap();
        assert_eq!(u.k_movement_modifiers.i_extra_moves, i32::MAX);
    }
    #[test]
    fn terrain_remove_toggles_current_cell_and_missing_is_unknown() {
        let mut u = unit();
        u.ab_invalid_terrain_types = vec![HashValue {
            definition_hash: 7,
            value: false,
        }];
        remove(&mut u, Effect::ValidTerrain { terrain_hash: 7 }, 2).unwrap();
        assert!(u.ab_invalid_terrain_types[0].value);
        remove(&mut u, Effect::ValidTerrain { terrain_hash: 7 }, 2).unwrap();
        assert!(!u.ab_invalid_terrain_types[0].value);
        assert_eq!(
            remove(&mut u, Effect::ValidTerrain { terrain_hash: 8 }, 2),
            Err(Error::MissingTerrainCell)
        );
    }
    #[test]
    fn ability_lost_only_on_positive_to_nonpositive_transition() {
        let mut u = unit();
        u.ai_abilities_possessed = vec![HashValue {
            definition_hash: 7,
            value: 2,
        }];
        let effect = Effect::GrantAbility { ability_hash: 7 };
        assert_eq!(remove(&mut u, effect, 2), Ok(Continuation::None));
        assert_eq!(remove(&mut u, effect, 2), Ok(Continuation::AbilityLost));
        assert_eq!(u.ai_abilities_possessed[0].value, 0);
        assert_eq!(remove(&mut u, effect, 2), Ok(Continuation::None));
    }
}
