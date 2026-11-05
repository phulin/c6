//! Linux scalar UnitEffect leaves, separate from DynamicModifier gating,
//! subject callbacks and affected-subject bookkeeping.
use crate::unit::{HealthModifiers, MovementModifiers, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Stored initialized effect fields +8 amount and +c TerritoryTypes.
    Healing { amount: i32, territory: i32 },
    /// Stored initialized effect field +8, in whole extra moves.
    Movement { amount: i32 },
    /// AdjustUnitBarbarianCombat2355650/2355670; ChangeVersusBarbarian2422530.
    BarbarianCombat { amount: i32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Apply,
    Remove,
}
impl Direction {
    fn delta(self, amount: i32) -> i32 {
        match self {
            Self::Apply => amount,
            Self::Remove => amount.wrapping_neg(),
        }
    }
}
impl Effect {
    /// Invalid territory selectors return without editing the native wrapper.
    pub const fn writes_unit(self) -> bool {
        match self {
            Self::Healing { territory, .. } => territory >= 0 && territory <= 3,
            Self::Movement { .. } | Self::BarbarianCombat { .. } => true,
        }
    }
}
/// ChangeTerritoryHealing2c27640. Universal(0) writes enemy, neutral, friendly
/// in that order; unknown signed selectors make no write. All adds wrap i32.
pub fn change_territory_healing(modifiers: &mut HealthModifiers, territory: i32, amount: i32) {
    match territory {
        0 => {
            modifiers.i_extra_enemy_heal = modifiers.i_extra_enemy_heal.wrapping_add(amount);
            modifiers.i_extra_neutral_heal = modifiers.i_extra_neutral_heal.wrapping_add(amount);
            modifiers.i_extra_friendly_heal = modifiers.i_extra_friendly_heal.wrapping_add(amount);
        }
        1 => modifiers.i_extra_friendly_heal = modifiers.i_extra_friendly_heal.wrapping_add(amount),
        2 => modifiers.i_extra_neutral_heal = modifiers.i_extra_neutral_heal.wrapping_add(amount),
        3 => modifiers.i_extra_enemy_heal = modifiers.i_extra_enemy_heal.wrapping_add(amount),
        _ => {}
    }
}
/// MovementModifiers.ChangeExtraMoves2422cc0. Remaining movement and other
/// modifiers are untouched; no immediate healing, movement, callback or RNG.
pub fn change_extra_moves(modifiers: &mut MovementModifiers, amount: i32) {
    modifiers.i_extra_moves = modifiers.i_extra_moves.wrapping_add(amount);
}
/// Apply23583b0/2359a30 and Remove23583d0/2359a50 after a nonnull Unit
/// has been resolved by the common UnitEffect wrapper. Every leaf returns true.
pub fn update(unit: &mut Unit, effect: Effect, direction: Direction) -> bool {
    match effect {
        Effect::Healing { amount, territory } => change_territory_healing(
            &mut unit.k_health_modifiers,
            territory,
            direction.delta(amount),
        ),
        Effect::BarbarianCombat { amount } => {
            unit.k_combat_modifers.i_versus_barbarians = unit
                .k_combat_modifers
                .i_versus_barbarians
                .wrapping_add(direction.delta(amount));
        }
        Effect::Movement { amount } => {
            change_extra_moves(&mut unit.k_movement_modifiers, direction.delta(amount))
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn barbarian_combat_wraps_and_remove_applies_wrapping_negative() {
        let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
        unit.k_combat_modifers.i_versus_barbarians = i32::MAX;
        let before = unit.k_combat_modifers.i_combat_strength_modifier;
        update(
            &mut unit,
            Effect::BarbarianCombat { amount: 5 },
            Direction::Apply,
        );
        assert_eq!(unit.k_combat_modifers.i_versus_barbarians, i32::MIN + 4);
        update(
            &mut unit,
            Effect::BarbarianCombat { amount: 5 },
            Direction::Remove,
        );
        assert_eq!(unit.k_combat_modifers.i_versus_barbarians, i32::MAX);
        assert_eq!(unit.k_combat_modifers.i_combat_strength_modifier, before);
    }
    fn health() -> HealthModifiers {
        HealthModifiers {
            i_extra_enemy_heal: i32::MAX,
            i_extra_neutral_heal: -7,
            i_extra_friendly_heal: i32::MIN,
            b_can_heal_after_action: true,
        }
    }
    #[test]
    fn universal_and_individual_territories_preserve_wrapping_and_other_fields() {
        let mut h = health();
        change_territory_healing(&mut h, 0, 20);
        assert_eq!(
            (
                h.i_extra_enemy_heal,
                h.i_extra_neutral_heal,
                h.i_extra_friendly_heal
            ),
            (i32::MIN + 19, 13, i32::MIN + 20)
        );
        assert!(h.b_can_heal_after_action);
        change_territory_healing(&mut h, 0, Direction::Remove.delta(20));
        assert_eq!(h, health());
        for (territory, expected) in [
            (1, (i32::MAX, -7, i32::MIN + 1)),
            (2, (i32::MAX, -6, i32::MIN)),
            (3, (i32::MIN, -7, i32::MIN)),
        ] {
            let mut h = health();
            change_territory_healing(&mut h, territory, 1);
            assert_eq!(
                (
                    h.i_extra_enemy_heal,
                    h.i_extra_neutral_heal,
                    h.i_extra_friendly_heal
                ),
                expected
            );
        }
    }
    #[test]
    fn unknown_territory_and_minimum_removal_have_native_semantics() {
        for territory in [-1, 4, i32::MIN, i32::MAX] {
            let mut h = health();
            change_territory_healing(&mut h, territory, 123);
            assert_eq!(h, health());
            assert!(!Effect::Healing {
                amount: 123,
                territory
            }
            .writes_unit());
        }
        assert_eq!(Direction::Remove.delta(i32::MIN), i32::MIN);
        let mut h = health();
        change_territory_healing(&mut h, 3, Direction::Remove.delta(i32::MIN));
        assert_eq!(h.i_extra_enemy_heal, -1);
    }
    #[test]
    fn movement_changes_only_extra_moves_and_restores_on_remove() {
        let mut m = MovementModifiers {
            i_extra_moves: i32::MAX,
            i_clear_terrain_start_movement: -3,
            b_enter_foreign_lands: true,
            b_ignore_rivers: true,
            b_ignore_cliffs: false,
            b_ignore_shores: true,
            b_escort_mobility: false,
            i_friendly_territory_start_movement: 9,
            i_enemy_territory_start_movement: -11,
            b_can_paradrop: true,
            i_jump_range: 15,
        };
        let before = m.clone();
        change_extra_moves(&mut m, 1);
        assert_eq!(m.i_extra_moves, i32::MIN);
        change_extra_moves(&mut m, Direction::Remove.delta(1));
        assert_eq!(m, before);
    }
}
