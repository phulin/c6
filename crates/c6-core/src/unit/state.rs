//! Named Linux Unit17/28 wire/default projection. No native object reconstruction.
//! Field ordering and versions follow evidence/linux/unit-wire-contracts.json.
//! Definition enums use raw u32 hashes; plain enums use i32 bits. Fixedpoint
//! values retain raw i32 storage. Byte strings are not assumed UTF-8. Ordered
//! hash pairs preserve definition identity and serialization order without live
//! index remapping. Only the explicitly supported nested codecs are accepted.
use super::variants::{self, TypedVariantMap, VariantMap};
use super::{Error, Limits, OperationQueue, PlotCoord, Reader};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentId {
    pub owner: i16,
    pub component_type: i16,
    pub id: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashValue<T> {
    pub definition_hash: u32,
    pub value: T,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldRange {
    pub name: &'static str,
    pub start: usize,
    pub end: usize,
}

trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
struct Writer {
    bytes: Vec<u8>,
    budget: usize,
    max_bytes: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::ResourceLimit)?
            > self.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn claim(&mut self, n: usize) -> Result<(), Error> {
        self.budget = self.budget.checked_sub(n).ok_or(Error::ResourceLimit)?;
        Ok(())
    }
    fn limits(&self) -> Limits {
        Limits {
            max_bytes: self.max_bytes - self.bytes.len(),
            max_items: self.budget,
        }
    }
}
macro_rules! scalar {
    ($t:ty,$n:expr) => {
        impl Wire for $t {
            const MIN: usize = $n;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                Ok(<$t>::from_le_bytes(r.take()?))
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                w.put(&self.to_le_bytes())
            }
        }
    };
}
scalar!(i32, 4);
scalar!(u32, 4);
scalar!(i16, 2);
scalar!(u8, 1);
impl Wire for bool {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.boolean()
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&[u8::from(*self)])
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = r.count(T::MIN)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(T::read(r)?);
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.claim(self.len())?;
        let n = u32::try_from(self.len()).map_err(|_| Error::ResourceLimit)?;
        n.write(w)?;
        for v in self {
            v.write(w)?;
        }
        Ok(())
    }
}
impl<T: Wire> Wire for HashValue<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            definition_hash: u32::read(r)?,
            value: T::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.definition_hash.write(w)?;
        self.value.write(w)
    }
}
impl Wire for ComponentId {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            owner: i16::read(r)?,
            component_type: i16::read(r)?,
            id: u32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.owner.write(w)?;
        self.component_type.write(w)?;
        self.id.write(w)
    }
}
impl Wire for PlotCoord {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.coord()
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.x.write(w)?;
        self.y.write(w)
    }
}
impl Wire for [i32; 5] {
    const MIN: usize = 20;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok([
            i32::read(r)?,
            i32::read(r)?,
            i32::read(r)?,
            i32::read(r)?,
            i32::read(r)?,
        ])
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        for v in self {
            v.write(w)?;
        }
        Ok(())
    }
}
impl Wire for OperationQueue {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Self::read(r)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let bytes = self.encode(w.limits())?;
        w.claim(self.nodes.len())?;
        w.put(&bytes)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthModifiers {
    /// Original m_iExtraEnemyHeal at 0x0.
    pub i_extra_enemy_heal: i32,
    /// Original m_iExtraNeutralHeal at 0x4.
    pub i_extra_neutral_heal: i32,
    /// Original m_iExtraFriendlyHeal at 0x8.
    pub i_extra_friendly_heal: i32,
    /// Original m_bCanHealAfterAction at 0xc.
    pub b_can_heal_after_action: bool,
}
impl Wire for HealthModifiers {
    const MIN: usize = 17;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(2)?;
        Ok(Self {
            i_extra_enemy_heal: <i32>::read(r)?,
            i_extra_neutral_heal: <i32>::read(r)?,
            i_extra_friendly_heal: <i32>::read(r)?,
            b_can_heal_after_action: <bool>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (2i32).write(w)?;
        self.i_extra_enemy_heal.write(w)?;
        self.i_extra_neutral_heal.write(w)?;
        self.i_extra_friendly_heal.write(w)?;
        self.b_can_heal_after_action.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthReligionModifiers {
    /// Original m_iExtraEnemyHealReligion at 0x0.
    pub i_extra_enemy_heal_religion: i32,
    /// Original m_iExtraNeutralHealReligion at 0x4.
    pub i_extra_neutral_heal_religion: i32,
    /// Original m_iExtraFriendlyHealReligion at 0x8.
    pub i_extra_friendly_heal_religion: i32,
    /// Original m_bCanHealAfterActionReligion at 0xc.
    pub b_can_heal_after_action_religion: bool,
}
impl Wire for HealthReligionModifiers {
    const MIN: usize = 17;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            i_extra_enemy_heal_religion: <i32>::read(r)?,
            i_extra_neutral_heal_religion: <i32>::read(r)?,
            i_extra_friendly_heal_religion: <i32>::read(r)?,
            b_can_heal_after_action_religion: <bool>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (1i32).write(w)?;
        self.i_extra_enemy_heal_religion.write(w)?;
        self.i_extra_neutral_heal_religion.write(w)?;
        self.i_extra_friendly_heal_religion.write(w)?;
        self.b_can_heal_after_action_religion.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatModifiers {
    pub wire_version: i32,
    /// Original m_iVersusBarbarians at 0x4.
    pub i_versus_barbarians: i32,
    /// Original m_iFriendlyTerritoryCombatMod at 0x8.
    pub i_friendly_territory_combat_mod: i32,
    /// Original m_bBypassWalls at 0x78.
    pub b_bypass_walls: bool,
    /// Original m_bEnableWallAttack at 0x79.
    pub b_enable_wall_attack: bool,
    /// Original m_bCannotAttack at 0x7a.
    pub b_cannot_attack: bool,
    /// Original m_iFlankingBonusModifier at 0xc.
    pub i_flanking_bonus_modifier: i32,
    /// Original m_iSupportBonusModifier at 0x10.
    pub i_support_bonus_modifier: i32,
    /// Original m_iPerLuxuryAttackModifier at 0x14.
    pub i_per_luxury_attack_modifier: i32,
    /// Original m_bAdvancedPillaging at 0x7b.
    pub b_advanced_pillaging: bool,
    /// Original m_iPlunderYieldModifier at 0x18.
    pub i_plunder_yield_modifier: i32,
    /// Original m_bBypassCombatUnits at 0x7c.
    pub b_bypass_combat_units: bool,
    /// Original m_bNoReductionForDamage at 0x7e.
    pub b_no_reduction_for_damage: bool,
    /// Original m_bCaptureCombatUnits at 0x7f.
    pub b_capture_combat_units: bool,
    /// Original m_eCapturedCombatUnitType at 0x24.
    pub e_captured_combat_unit_type: u32,
    /// Original m_iBonusAttacks at 0x0.
    pub i_bonus_attacks: i32,
    /// Original m_bConductRaid at 0x7d.
    pub b_conduct_raid: bool,
    /// Original m_iRaidBonus at 0x1c.
    pub i_raid_bonus: i32,
    /// Original m_iStrengthReductionForDamageModifier at 0x28.
    pub i_strength_reduction_for_damage_modifier: i32,
    /// Original m_iPostCombatHeal at 0x20.
    pub i_post_combat_heal: i32,
    /// Original m_bCanForceRetreat at 0x80.
    pub b_can_force_retreat: bool,
    /// Original m_iHigherDiploVizCombatMod at 0x2c.
    pub i_higher_diplo_viz_combat_mod: i32,
    /// Original m_iEachDiploVizCombatMod at 0x30.
    pub i_each_diplo_viz_combat_mod: i32,
    /// Original m_iAdjacentLevyCombatModifier at 0x34.
    pub i_adjacent_levy_combat_modifier: i32,
    /// Original m_iNumberAlliesCombatModifier at 0x38.
    pub i_number_allies_combat_modifier: i32,
    /// Original m_iStrengthReductionInsufficientFuel at 0x3c.
    pub i_strength_reduction_insufficient_fuel: i32,
    /// Original m_iPerUnusedMovementCombatModifier at 0x40.
    pub i_per_unused_movement_combat_modifier: i32,
    /// Original m_iAdvancedCoastalRaid at 0x44.
    pub i_advanced_coastal_raid: i32,
    /// Original m_iCombatBonusAgainstDistrict at 0x48.
    pub i_combat_bonus_against_district: i32,
    /// Original m_bIgnoreRangedVsDistrictPenalty at 0x81.
    pub b_ignore_ranged_vs_district_penalty: bool,
    /// Original m_iWMDBlastResistance at 0x4c.
    pub i_wmd_blast_resistance: i32,
    /// Original m_iWMDFalloutResistance at 0x50.
    pub i_wmd_fallout_resistance: i32,
    /// Original m_iWaterDamageResistance at 0x54.
    pub i_water_damage_resistance: i32,
    /// Original m_iNumberHolyCitiesCombatModifier at 0x58.
    pub i_number_holy_cities_combat_modifier: i32,
    /// Original m_iNumberMilitaryPoliciesCombatModifier at 0x5c.
    pub i_number_military_policies_combat_modifier: i32,
    /// Original m_iNumberNeighborCombatBonus at 0x60.
    pub i_number_neighbor_combat_bonus: i32,
    /// Original m_bFightWhileEmbarked at 0x82.
    pub b_fight_while_embarked: bool,
    /// Original m_iAntiAirStrengthModifier at 0x74.
    pub i_anti_air_strength_modifier: i32,
    /// Original m_iCombatStrengthModifier at 0x68.
    pub i_combat_strength_modifier: i32,
    /// Original m_iRangedStrengthModifier at 0x6c.
    pub i_ranged_strength_modifier: i32,
    /// Original m_iBombardStrengthModifier at 0x70.
    pub i_bombard_strength_modifier: i32,
    /// Original m_iDirectLoyaltyDamage at 0x64.
    pub i_direct_loyalty_damage: i32,
}
impl Wire for CombatModifiers {
    const MIN: usize = 98;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.i32()?;
        if wire_version != 20 && wire_version != 28 {
            return Err(Error::Version {
                expected: 28,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            i_versus_barbarians: <i32>::read(r)?,
            i_friendly_territory_combat_mod: <i32>::read(r)?,
            b_bypass_walls: <bool>::read(r)?,
            b_enable_wall_attack: <bool>::read(r)?,
            b_cannot_attack: <bool>::read(r)?,
            i_flanking_bonus_modifier: <i32>::read(r)?,
            i_support_bonus_modifier: <i32>::read(r)?,
            i_per_luxury_attack_modifier: <i32>::read(r)?,
            b_advanced_pillaging: <bool>::read(r)?,
            i_plunder_yield_modifier: <i32>::read(r)?,
            b_bypass_combat_units: <bool>::read(r)?,
            b_no_reduction_for_damage: <bool>::read(r)?,
            b_capture_combat_units: <bool>::read(r)?,
            e_captured_combat_unit_type: <u32>::read(r)?,
            i_bonus_attacks: <i32>::read(r)?,
            b_conduct_raid: <bool>::read(r)?,
            i_raid_bonus: <i32>::read(r)?,
            i_strength_reduction_for_damage_modifier: <i32>::read(r)?,
            i_post_combat_heal: <i32>::read(r)?,
            b_can_force_retreat: <bool>::read(r)?,
            i_higher_diplo_viz_combat_mod: <i32>::read(r)?,
            i_each_diplo_viz_combat_mod: <i32>::read(r)?,
            i_adjacent_levy_combat_modifier: <i32>::read(r)?,
            i_number_allies_combat_modifier: <i32>::read(r)?,
            i_strength_reduction_insufficient_fuel: <i32>::read(r)?,
            i_per_unused_movement_combat_modifier: <i32>::read(r)?,
            i_advanced_coastal_raid: <i32>::read(r)?,
            i_combat_bonus_against_district: <i32>::read(r)?,
            b_ignore_ranged_vs_district_penalty: <bool>::read(r)?,
            i_wmd_blast_resistance: <i32>::read(r)?,
            i_wmd_fallout_resistance: <i32>::read(r)?,
            i_water_damage_resistance: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
            i_number_holy_cities_combat_modifier: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
            i_number_military_policies_combat_modifier: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
            i_number_neighbor_combat_bonus: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
            b_fight_while_embarked: if wire_version == 20 {
                false
            } else {
                <bool>::read(r)?
            },
            i_anti_air_strength_modifier: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
            i_combat_strength_modifier: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
            i_ranged_strength_modifier: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
            i_bombard_strength_modifier: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
            i_direct_loyalty_damage: if wire_version == 20 {
                0
            } else {
                <i32>::read(r)?
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 28 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        (28i32).write(w)?;
        self.i_versus_barbarians.write(w)?;
        self.i_friendly_territory_combat_mod.write(w)?;
        self.b_bypass_walls.write(w)?;
        self.b_enable_wall_attack.write(w)?;
        self.b_cannot_attack.write(w)?;
        self.i_flanking_bonus_modifier.write(w)?;
        self.i_support_bonus_modifier.write(w)?;
        self.i_per_luxury_attack_modifier.write(w)?;
        self.b_advanced_pillaging.write(w)?;
        self.i_plunder_yield_modifier.write(w)?;
        self.b_bypass_combat_units.write(w)?;
        self.b_no_reduction_for_damage.write(w)?;
        self.b_capture_combat_units.write(w)?;
        self.e_captured_combat_unit_type.write(w)?;
        self.i_bonus_attacks.write(w)?;
        self.b_conduct_raid.write(w)?;
        self.i_raid_bonus.write(w)?;
        self.i_strength_reduction_for_damage_modifier.write(w)?;
        self.i_post_combat_heal.write(w)?;
        self.b_can_force_retreat.write(w)?;
        self.i_higher_diplo_viz_combat_mod.write(w)?;
        self.i_each_diplo_viz_combat_mod.write(w)?;
        self.i_adjacent_levy_combat_modifier.write(w)?;
        self.i_number_allies_combat_modifier.write(w)?;
        self.i_strength_reduction_insufficient_fuel.write(w)?;
        self.i_per_unused_movement_combat_modifier.write(w)?;
        self.i_advanced_coastal_raid.write(w)?;
        self.i_combat_bonus_against_district.write(w)?;
        self.b_ignore_ranged_vs_district_penalty.write(w)?;
        self.i_wmd_blast_resistance.write(w)?;
        self.i_wmd_fallout_resistance.write(w)?;
        self.i_water_damage_resistance.write(w)?;
        self.i_number_holy_cities_combat_modifier.write(w)?;
        self.i_number_military_policies_combat_modifier.write(w)?;
        self.i_number_neighbor_combat_bonus.write(w)?;
        self.b_fight_while_embarked.write(w)?;
        self.i_anti_air_strength_modifier.write(w)?;
        self.i_combat_strength_modifier.write(w)?;
        self.i_ranged_strength_modifier.write(w)?;
        self.i_bombard_strength_modifier.write(w)?;
        self.i_direct_loyalty_damage.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementModifiers {
    /// Original m_iExtraMoves at 0x0.
    pub i_extra_moves: i32,
    /// Original m_iClearTerrainStartMovement at 0x4.
    pub i_clear_terrain_start_movement: i32,
    /// Original m_bEnterForeignLands at 0x14.
    pub b_enter_foreign_lands: bool,
    /// Original m_bIgnoreRivers at 0x15.
    pub b_ignore_rivers: bool,
    /// Original m_bIgnoreCliffs at 0x16.
    pub b_ignore_cliffs: bool,
    /// Original m_bIgnoreShores at 0x17.
    pub b_ignore_shores: bool,
    /// Original m_bEscortMobility at 0x18.
    pub b_escort_mobility: bool,
    /// Original m_iFriendlyTerritoryStartMovement at 0x8.
    pub i_friendly_territory_start_movement: i32,
    /// Original m_iEnemyTerritoryStartMovement at 0xc.
    pub i_enemy_territory_start_movement: i32,
    /// Original m_bCanParadrop at 0x19.
    pub b_can_paradrop: bool,
    /// Original m_iJumpRange at 0x10.
    pub i_jump_range: i32,
}
impl Wire for MovementModifiers {
    const MIN: usize = 30;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(8)?;
        Ok(Self {
            i_extra_moves: <i32>::read(r)?,
            i_clear_terrain_start_movement: <i32>::read(r)?,
            b_enter_foreign_lands: <bool>::read(r)?,
            b_ignore_rivers: <bool>::read(r)?,
            b_ignore_cliffs: <bool>::read(r)?,
            b_ignore_shores: <bool>::read(r)?,
            b_escort_mobility: <bool>::read(r)?,
            i_friendly_territory_start_movement: <i32>::read(r)?,
            i_enemy_territory_start_movement: <i32>::read(r)?,
            b_can_paradrop: <bool>::read(r)?,
            i_jump_range: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (8i32).write(w)?;
        self.i_extra_moves.write(w)?;
        self.i_clear_terrain_start_movement.write(w)?;
        self.b_enter_foreign_lands.write(w)?;
        self.b_ignore_rivers.write(w)?;
        self.b_ignore_cliffs.write(w)?;
        self.b_ignore_shores.write(w)?;
        self.b_escort_mobility.write(w)?;
        self.i_friendly_territory_start_movement.write(w)?;
        self.i_enemy_territory_start_movement.write(w)?;
        self.b_can_paradrop.write(w)?;
        self.i_jump_range.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SightModifiers {
    /// Original m_iSightRange at 0x0.
    pub i_sight_range: i32,
    /// Original m_eCanSeeHiddenType at 0x4.
    pub e_can_see_hidden_type: i32,
}
impl Wire for SightModifiers {
    const MIN: usize = 12;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            i_sight_range: <i32>::read(r)?,
            e_can_see_hidden_type: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (1i32).write(w)?;
        self.i_sight_range.write(w)?;
        self.e_can_see_hidden_type.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttackRangeModifiers {
    /// Original m_iAttackRange at 0x0.
    pub i_attack_range: i32,
}
impl Wire for AttackRangeModifiers {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            i_attack_range: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (1i32).write(w)?;
        self.i_attack_range.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Experience {
    pub wire_version: i32,
    /// Original m_bCanPromote at 0x8.
    pub b_can_promote: bool,
    /// Original m_bExperienceLocked at 0xa.
    pub b_experience_locked: bool,
    /// Original m_iExperience at 0xc.
    pub i_experience: i32,
    /// Original m_iLevel at 0x10.
    pub i_level: i32,
    /// Original m_iStoredPromotions at 0x14.
    pub i_stored_promotions: i32,
    /// Original m_iPromoteNoFinishMoves at 0x18.
    pub i_promote_no_finish_moves: i32,
    /// Original m_xGeneralExperienceModifier at 0x28.
    pub x_general_experience_modifier: i32,
    /// Original m_abPromotionsEarned at 0x30.
    pub ab_promotions_earned: Vec<HashValue<bool>>,
    /// Original m_sVeteranName at 0x20.
    pub s_veteran_name: Vec<u8>,
    /// Original m_bCanPromoteNotifyNeeded at 0x9.
    pub b_can_promote_notify_needed: bool,
    /// Original m_xAttackingExperienceModifierPercent at 0x2c.
    pub x_attacking_experience_modifier_percent: i32,
}
impl Wire for Experience {
    const MIN: usize = 35;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.i32()?;
        if wire_version != 6 && wire_version != 7 {
            return Err(Error::Version {
                expected: 7,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            b_can_promote: <bool>::read(r)?,
            b_experience_locked: <bool>::read(r)?,
            i_experience: <i32>::read(r)?,
            i_level: <i32>::read(r)?,
            i_stored_promotions: <i32>::read(r)?,
            i_promote_no_finish_moves: if wire_version == 6 {
                0
            } else {
                <i32>::read(r)?
            },
            x_general_experience_modifier: <i32>::read(r)?,
            ab_promotions_earned: <Vec<HashValue<bool>>>::read(r)?,
            s_veteran_name: <Vec<u8>>::read(r)?,
            b_can_promote_notify_needed: <bool>::read(r)?,
            x_attacking_experience_modifier_percent: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 7 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        (7i32).write(w)?;
        self.b_can_promote.write(w)?;
        self.b_experience_locked.write(w)?;
        self.i_experience.write(w)?;
        self.i_level.write(w)?;
        self.i_stored_promotions.write(w)?;
        self.i_promote_no_finish_moves.write(w)?;
        self.x_general_experience_modifier.write(w)?;
        self.ab_promotions_earned.write(w)?;
        self.s_veteran_name.write(w)?;
        self.b_can_promote_notify_needed.write(w)?;
        self.x_attacking_experience_modifier_percent.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreatPerson {
    /// Original m_eIndividual at 0x8.
    pub e_individual: u32,
    /// Original m_eClass at 0xc.
    pub e_class: u32,
    /// Original m_iActionCharges at 0x10.
    pub i_action_charges: i32,
}
impl Wire for GreatPerson {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            e_individual: <u32>::read(r)?,
            e_class: <u32>::read(r)?,
            i_action_charges: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (1i32).write(w)?;
        self.e_individual.write(w)?;
        self.e_class.write(w)?;
        self.i_action_charges.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Religion {
    pub wire_version: i32,
    /// Original m_iReligiousStrength at 0x8.
    pub i_religious_strength: i32,
    /// Original m_eReligionType at 0xc.
    pub e_religion_type: u32,
    /// Original m_iSpreadCharges at 0x10.
    pub i_spread_charges: i32,
    /// Original m_iReligiousHealCharges at 0x14.
    pub i_religious_heal_charges: i32,
    /// Original m_iNWDeferredCharges at 0x18.
    pub i_nw_deferred_charges: i32,
    /// Original m_eInitiationYield at 0x1c.
    pub e_initiation_yield: i32,
    /// Original m_iInitiationAmount at 0x20.
    pub i_initiation_amount: i32,
    /// Original m_iEvictPercent at 0x24.
    pub i_evict_percent: i32,
    /// Original m_iForeignSpreadModifier at 0x28.
    pub i_foreign_spread_modifier: i32,
    /// Original m_bConvertsBarbarians at 0x2c.
    pub b_converts_barbarians: bool,
    /// Original m_bNoForeignSpread at 0x2d.
    pub b_no_foreign_spread: bool,
    /// Original m_bRelicUponDeath at 0x2e.
    pub b_relic_upon_death: bool,
    /// Original m_bLandVictorySpread at 0x2f.
    pub b_land_victory_spread: bool,
    /// Original m_iLandVictorySpread at 0x30.
    pub i_land_victory_spread: i32,
    /// Original m_iPlunderImprovementFaithYield at 0x34.
    pub i_plunder_improvement_faith_yield: i32,
    /// Original m_iPlunderDistrictFaithYield at 0x38.
    pub i_plunder_district_faith_yield: i32,
    /// Original m_aInitiationYieldPopulation at 0x40.
    pub a_initiation_yield_population: Vec<InitiationYieldPopulation>,
    /// Original m_eRelicSource at 0x3c.
    pub e_relic_source: i32,
}
impl Wire for Religion {
    const MIN: usize = 56;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.i32()?;
        if wire_version != 6 && wire_version != 8 {
            return Err(Error::Version {
                expected: 8,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            i_religious_strength: <i32>::read(r)?,
            e_religion_type: <u32>::read(r)?,
            i_spread_charges: <i32>::read(r)?,
            i_religious_heal_charges: <i32>::read(r)?,
            i_nw_deferred_charges: <i32>::read(r)?,
            e_initiation_yield: <i32>::read(r)?,
            i_initiation_amount: <i32>::read(r)?,
            i_evict_percent: <i32>::read(r)?,
            i_foreign_spread_modifier: <i32>::read(r)?,
            b_converts_barbarians: <bool>::read(r)?,
            b_no_foreign_spread: <bool>::read(r)?,
            b_relic_upon_death: <bool>::read(r)?,
            b_land_victory_spread: <bool>::read(r)?,
            i_land_victory_spread: if wire_version == 6 {
                0
            } else {
                <i32>::read(r)?
            },
            i_plunder_improvement_faith_yield: <i32>::read(r)?,
            i_plunder_district_faith_yield: <i32>::read(r)?,
            a_initiation_yield_population: <Vec<InitiationYieldPopulation>>::read(r)?,
            e_relic_source: if wire_version == 6 {
                -1
            } else {
                <i32>::read(r)?
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 8 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        (8i32).write(w)?;
        self.i_religious_strength.write(w)?;
        self.e_religion_type.write(w)?;
        self.i_spread_charges.write(w)?;
        self.i_religious_heal_charges.write(w)?;
        self.i_nw_deferred_charges.write(w)?;
        self.e_initiation_yield.write(w)?;
        self.i_initiation_amount.write(w)?;
        self.i_evict_percent.write(w)?;
        self.i_foreign_spread_modifier.write(w)?;
        self.b_converts_barbarians.write(w)?;
        self.b_no_foreign_spread.write(w)?;
        self.b_relic_upon_death.write(w)?;
        self.b_land_victory_spread.write(w)?;
        self.i_land_victory_spread.write(w)?;
        self.i_plunder_improvement_faith_yield.write(w)?;
        self.i_plunder_district_faith_yield.write(w)?;
        self.a_initiation_yield_population.write(w)?;
        self.e_relic_source.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RockBand {
    pub wire_version: i32,
    /// Original m_bMaxLevel at 0x8.
    pub b_max_level: bool,
    /// Original m_bMaxPromotions at 0x9.
    pub b_max_promotions: bool,
    /// Original m_iLevel at 0xc.
    pub i_level: i32,
    /// Original m_iAlbumSales at 0x10.
    pub i_album_sales: i32,
    /// Original m_iPostTourismBombForeignLoyalty at 0x14.
    pub i_post_tourism_bomb_foreign_loyalty: i32,
    /// Original m_iModifiedRockbandLevelNationalPark at 0x18.
    pub i_modified_rockband_level_national_park: i32,
    /// Original m_iModifiedTourismBombValueNationalPark at 0x1c.
    pub i_modified_tourism_bomb_value_national_park: i32,
    /// Original m_iModifiedRockbandLevelNaturalWonder at 0x20.
    pub i_modified_rockband_level_natural_wonder: i32,
    /// Original m_iModifiedTourismBombValueNaturalWonder at 0x24.
    pub i_modified_tourism_bomb_value_natural_wonder: i32,
    /// Original m_iModifiedTourismBombPlayerRange at 0x28.
    pub i_modified_tourism_bomb_player_range: i32,
    /// Original m_iModifiedTourismBombValueAtRange at 0x2c.
    pub i_modified_tourism_bomb_value_at_range: i32,
    /// Original m_iModifiedTourismBombConvertCity at 0x30.
    pub i_modified_tourism_bomb_convert_city: i32,
    /// Original m_iModifiedTourismBombValueTargetPeace at 0x34.
    pub i_modified_tourism_bomb_value_target_peace: i32,
    /// Original m_aModifiedYieldPerTourismBomb at 0xb0.
    pub a_modified_yield_per_tourism_bomb: Vec<HashValue<i32>>,
    /// Original m_aiConcertPlots at 0x98. At6 no list is serialized; the
    /// empty projection relies on parent Unit.Reset clearing this vector.
    pub ai_concert_plots: Vec<i32>,
    /// Original m_aiModifiedRockbandLevelDistrict at 0x38.
    pub ai_modified_rockband_level_district: Vec<HashValue<i32>>,
    /// Original m_aiModifiedRockbandLevelImprovement at 0x50.
    pub ai_modified_rockband_level_improvement: Vec<HashValue<i32>>,
    /// Original m_aiModifiedTourismBombValueDistrict at 0x68.
    pub ai_modified_tourism_bomb_value_district: Vec<HashValue<i32>>,
    /// Original m_aiModifiedTourismBombValueImprovement at 0x80.
    pub ai_modified_tourism_bomb_value_improvement: Vec<HashValue<i32>>,
}
impl Wire for RockBand {
    const MIN: usize = 56;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.i32()?;
        if wire_version != 6 && wire_version != 10 {
            return Err(Error::Version {
                expected: 10,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            b_max_level: if wire_version == 6 {
                false
            } else {
                <bool>::read(r)?
            },
            b_max_promotions: if wire_version == 6 {
                false
            } else {
                <bool>::read(r)?
            },
            i_level: <i32>::read(r)?,
            i_album_sales: <i32>::read(r)?,
            i_post_tourism_bomb_foreign_loyalty: <i32>::read(r)?,
            i_modified_rockband_level_national_park: <i32>::read(r)?,
            i_modified_tourism_bomb_value_national_park: <i32>::read(r)?,
            i_modified_rockband_level_natural_wonder: if wire_version == 6 {
                0
            } else {
                <i32>::read(r)?
            },
            i_modified_tourism_bomb_value_natural_wonder: if wire_version == 6 {
                0
            } else {
                <i32>::read(r)?
            },
            i_modified_tourism_bomb_player_range: <i32>::read(r)?,
            i_modified_tourism_bomb_value_at_range: <i32>::read(r)?,
            i_modified_tourism_bomb_convert_city: <i32>::read(r)?,
            i_modified_tourism_bomb_value_target_peace: if wire_version == 6 {
                0
            } else {
                <i32>::read(r)?
            },
            a_modified_yield_per_tourism_bomb: <Vec<HashValue<i32>>>::read(r)?,
            ai_concert_plots: if wire_version == 6 {
                Vec::new()
            } else {
                <Vec<i32>>::read(r)?
            },
            ai_modified_rockband_level_district: <Vec<HashValue<i32>>>::read(r)?,
            ai_modified_rockband_level_improvement: <Vec<HashValue<i32>>>::read(r)?,
            ai_modified_tourism_bomb_value_district: <Vec<HashValue<i32>>>::read(r)?,
            ai_modified_tourism_bomb_value_improvement: <Vec<HashValue<i32>>>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 10 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        (10i32).write(w)?;
        self.b_max_level.write(w)?;
        self.b_max_promotions.write(w)?;
        self.i_level.write(w)?;
        self.i_album_sales.write(w)?;
        self.i_post_tourism_bomb_foreign_loyalty.write(w)?;
        self.i_modified_rockband_level_national_park.write(w)?;
        self.i_modified_tourism_bomb_value_national_park.write(w)?;
        self.i_modified_rockband_level_natural_wonder.write(w)?;
        self.i_modified_tourism_bomb_value_natural_wonder.write(w)?;
        self.i_modified_tourism_bomb_player_range.write(w)?;
        self.i_modified_tourism_bomb_value_at_range.write(w)?;
        self.i_modified_tourism_bomb_convert_city.write(w)?;
        self.i_modified_tourism_bomb_value_target_peace.write(w)?;
        self.a_modified_yield_per_tourism_bomb.write(w)?;
        self.ai_concert_plots.write(w)?;
        self.ai_modified_rockband_level_district.write(w)?;
        self.ai_modified_rockband_level_improvement.write(w)?;
        self.ai_modified_tourism_bomb_value_district.write(w)?;
        self.ai_modified_tourism_bomb_value_improvement.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    /// Original m_kOriginTradeCity at 0x8.
    pub k_origin_trade_city: ComponentId,
    /// Original m_kDestinationTradeCity at 0x10.
    pub k_destination_trade_city: ComponentId,
    /// Original m_vDomainPlunderImmunities at 0x18.
    pub v_domain_plunder_immunities: [i32; 5],
}
impl Wire for Trade {
    const MIN: usize = 40;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(2)?;
        Ok(Self {
            k_origin_trade_city: <ComponentId>::read(r)?,
            k_destination_trade_city: <ComponentId>::read(r)?,
            v_domain_plunder_immunities: <[i32; 5]>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (2i32).write(w)?;
        self.k_origin_trade_city.write(w)?;
        self.k_destination_trade_city.write(w)?;
        self.v_domain_plunder_immunities.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Archaeology {
    /// Original m_kVariants.m_kHomeCityID at 0x8.
    pub k_home_city_id: u32,
    /// Original m_kVariants.m_eHomeBuilding at 0xc.
    pub e_home_building: u32,
    /// Original m_kVariants.m_iArtifactIndex at 0x10.
    pub i_artifact_index: i32,
    /// Original m_kVariants.m_iGreatWorkIndex at 0x14.
    pub i_great_work_index: i32,
    /// Original m_kVariants.m_bCanExtractSeaArtifacts at 0x18.
    pub b_can_extract_sea_artifacts: bool,
    /// Original m_kVariants.m_bChoosingArtifact at 0x19.
    pub b_choosing_artifact: bool,
}
impl Wire for Archaeology {
    const MIN: usize = 22;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(4)?;
        Ok(Self {
            k_home_city_id: <u32>::read(r)?,
            e_home_building: <u32>::read(r)?,
            i_artifact_index: <i32>::read(r)?,
            i_great_work_index: <i32>::read(r)?,
            b_can_extract_sea_artifacts: <bool>::read(r)?,
            b_choosing_artifact: <bool>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (4i32).write(w)?;
        self.k_home_city_id.write(w)?;
        self.e_home_building.write(w)?;
        self.i_artifact_index.write(w)?;
        self.i_great_work_index.write(w)?;
        self.b_can_extract_sea_artifacts.write(w)?;
        self.b_choosing_artifact.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Espionage {
    pub wire_version: i32,
    /// Original m_aEspionageBonuses at 0x8.
    pub a_espionage_bonuses: Vec<EspionageBonus>,
    /// Original m_aEspionageTimeReductions at 0x28.
    pub a_espionage_time_reductions: Vec<EspionageTimeReduction>,
    /// Original m_eOperation at 0x48.
    pub e_operation: u32,
    /// Original m_iOperationEndTurn at 0x4c.
    pub i_operation_end_turn: i32,
    /// Original m_kOperationEndDestination at 0x50.
    pub k_operation_end_destination: PlotCoord,
    /// Original m_iEstablishInCityTimeReduction at 0x58.
    pub i_establish_in_city_time_reduction: i32,
    /// Original m_iEscapeBoost at 0x5c.
    pub i_escape_boost: i32,
    /// Original m_iBoostAllSpies at 0x60.
    pub i_boost_all_spies: i32,
    /// Original m_iNerfAllEnemySpies at 0x64.
    pub i_nerf_all_enemy_spies: i32,
    /// Original m_iVisibilityReductionAllCities at 0x68.
    pub i_visibility_reduction_all_cities: i32,
    /// Original m_bCounterSpyEntireCity at 0x80.
    pub b_counter_spy_entire_city: bool,
    /// Original m_iCounterSpyAdjacentLevelBoost at 0x6c.
    pub i_counter_spy_adjacent_level_boost: i32,
    /// Original m_eEscapeRoute at 0x70.
    pub e_escape_route: u32,
    /// Original m_bEscaping at 0x81.
    pub b_escaping: bool,
    /// Original m_eStolenBoost at 0x74.
    pub e_stolen_boost: u32,
    /// Original m_iSiphonedGold at 0x78.
    pub i_siphoned_gold: i32,
    /// Original m_iStolenGreatWorkIndex at 0x7c.
    pub i_stolen_great_work_index: i32,
}
impl Wire for Espionage {
    const MIN: usize = 62;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.i32()?;
        if wire_version != 6 && wire_version != 7 {
            return Err(Error::Version {
                expected: 7,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            a_espionage_bonuses: <Vec<EspionageBonus>>::read(r)?,
            a_espionage_time_reductions: <Vec<EspionageTimeReduction>>::read(r)?,
            e_operation: <u32>::read(r)?,
            i_operation_end_turn: <i32>::read(r)?,
            k_operation_end_destination: <PlotCoord>::read(r)?,
            i_establish_in_city_time_reduction: <i32>::read(r)?,
            i_escape_boost: <i32>::read(r)?,
            i_boost_all_spies: <i32>::read(r)?,
            i_nerf_all_enemy_spies: if wire_version == 6 {
                0
            } else {
                <i32>::read(r)?
            },
            i_visibility_reduction_all_cities: <i32>::read(r)?,
            b_counter_spy_entire_city: <bool>::read(r)?,
            i_counter_spy_adjacent_level_boost: if wire_version == 6 {
                0
            } else {
                <i32>::read(r)?
            },
            e_escape_route: <u32>::read(r)?,
            b_escaping: <bool>::read(r)?,
            e_stolen_boost: <u32>::read(r)?,
            i_siphoned_gold: <i32>::read(r)?,
            i_stolen_great_work_index: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 7 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        (7i32).write(w)?;
        self.a_espionage_bonuses.write(w)?;
        self.a_espionage_time_reductions.write(w)?;
        self.e_operation.write(w)?;
        self.i_operation_end_turn.write(w)?;
        self.k_operation_end_destination.write(w)?;
        self.i_establish_in_city_time_reduction.write(w)?;
        self.i_escape_boost.write(w)?;
        self.i_boost_all_spies.write(w)?;
        self.i_nerf_all_enemy_spies.write(w)?;
        self.i_visibility_reduction_all_cities.write(w)?;
        self.b_counter_spy_entire_city.write(w)?;
        self.i_counter_spy_adjacent_level_boost.write(w)?;
        self.e_escape_route.write(w)?;
        self.b_escaping.write(w)?;
        self.e_stolen_boost.write(w)?;
        self.i_siphoned_gold.write(w)?;
        self.i_stolen_great_work_index.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hero {
    /// Original m_eClass at 0x8.
    pub e_class: u32,
    /// Original m_kOriginCityID at 0x10.
    pub k_origin_city_id: ComponentId,
}
impl Wire for Hero {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(5)?;
        Ok(Self {
            e_class: <u32>::read(r)?,
            k_origin_city_id: <ComponentId>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (5i32).write(w)?;
        self.e_class.write(w)?;
        self.k_origin_city_id.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SightPlotEntry {
    /// Original m_iRing at 0x4.
    pub i_ring: i32,
    /// Original m_uiIndex at 0x0.
    pub ui_index: u32,
}
impl Wire for SightPlotEntry {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            i_ring: <i32>::read(r)?,
            ui_index: <u32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.i_ring.write(w)?;
        self.ui_index.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspionageBonus {
    pub wire_version: i32,
    /// Original m_eOperationType at 0x0.
    pub e_operation_type: u32,
    /// Original m_bOffensive at 0x4.
    pub b_offensive: bool,
    /// Original m_iAmount at 0x8.
    pub i_amount: i32,
}
impl Wire for EspionageBonus {
    const MIN: usize = 13;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.i32()?;
        if wire_version != 6 && wire_version != 7 {
            return Err(Error::Version {
                expected: 7,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            e_operation_type: <u32>::read(r)?,
            b_offensive: <bool>::read(r)?,
            i_amount: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 7 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        (7i32).write(w)?;
        self.e_operation_type.write(w)?;
        self.b_offensive.write(w)?;
        self.i_amount.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspionageTimeReduction {
    pub wire_version: i32,
    /// Original m_eOperationType at 0x0.
    pub e_operation_type: u32,
    /// Original m_iReductionPercent at 0x4.
    pub i_reduction_percent: i32,
}
impl Wire for EspionageTimeReduction {
    const MIN: usize = 12;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.i32()?;
        if wire_version != 6 && wire_version != 7 {
            return Err(Error::Version {
                expected: 7,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            e_operation_type: <u32>::read(r)?,
            i_reduction_percent: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 7 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        (7i32).write(w)?;
        self.e_operation_type.write(w)?;
        self.i_reduction_percent.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitiationYieldPopulation {
    /// Original m_eYieldType at 0x0.
    pub e_yield_type: i32,
    /// Original m_iPopulationAmount at 0x4.
    pub i_population_amount: i32,
}
impl Wire for InitiationYieldPopulation {
    const MIN: usize = 12;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            e_yield_type: <i32>::read(r)?,
            i_population_amount: <i32>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (1i32).write(w)?;
        self.e_yield_type.write(w)?;
        self.i_population_amount.write(w)?;
        Ok(())
    }
}
/// Exact outer field cursor when a Unit reader refuses. A nested error is
/// attributed to that field; it is not relabeled as the outer Unit version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitReadBoundary {
    pub field: &'static str,
    pub field_start: usize,
    pub consumed: usize,
    pub error: Error,
}
/// Current28 fields and a bounded17 wire/default projection for supported nested codecs.
/// Missing prior scalar state and omitted definition/configuration Read effects
/// stay explicit. Domain, pointers, dirty history and runtime fixups are absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unit {
    pub wire_version: i32,
    /// Original m_id.
    pub id: u32,
    /// Original m_eTypeIndex.
    pub e_type_index: u32,
    /// Original m_eAIType.
    pub e_ai_type: i32,
    /// Original m_aiController.
    pub ai_controller: u32,
    /// Original m_aiGoal.
    pub ai_goal: PlotCoord,
    /// Original m_kLocation.
    pub k_location: PlotCoord,
    /// Original m_eFacingDirection.
    pub e_facing_direction: i32,
    /// Original m_eOwner.
    pub e_owner: i32,
    /// Original m_eOriginalOwner.
    pub e_original_owner: i32,
    /// Original m_bIsLevied.
    pub b_is_levied: bool,
    /// Original m_formationID.
    pub formation_id: u32,
    /// Original m_eMilitaryFormation.
    pub e_military_formation: i32,
    /// Original m_eAutomation.
    pub e_automation: i32,
    /// Original m_eActivity.
    pub e_activity: i32,
    /// Original m_iDamage.
    pub i_damage: i32,
    /// Original m_iFortifyTurns.
    pub i_fortify_turns: i32,
    /// Original m_iUpgradeDiscount.
    pub i_upgrade_discount: i32,
    /// Original m_xMovesRemaining.
    pub x_moves_remaining: i32,
    /// Original m_iAttacksRemaining.
    pub i_attacks_remaining: i32,
    /// Original m_iOperationTimer.
    pub i_operation_timer: i32,
    /// Original m_iBuildCharges.
    pub i_build_charges: i32,
    /// Original m_iParkCharges.
    pub i_park_charges: i32,
    /// Original m_iActionCharges.
    pub i_action_charges: i32,
    /// Original m_iAirSlotModifier.
    pub i_air_slot_modifier: i32,
    /// Original m_bDominantConquer.
    pub b_dominant_conquer: bool,
    /// Original m_iBarbarianTribeIndex.
    pub i_barbarian_tribe_index: i32,
    /// Original m_eBarbarianPlayerTarget.
    pub e_barbarian_player_target: i32,
    /// Original m_iDisasterCharges.
    /// None at17: native Read retains a prior scalar not supplied by the wire.
    pub i_disaster_charges: Option<i32>,
    /// Original m_iEraStrengthModifier.
    pub i_era_strength_modifier: i32,
    /// Original m_bEmbarked.
    pub b_embarked: bool,
    /// Original m_bDelayedDeath.
    pub b_delayed_death: bool,
    /// Original m_bImmobile.
    pub b_immobile: bool,
    /// Original m_bProcessedTurn.
    pub b_processed_turn: bool,
    /// Original m_bNeedsMovementCompletion.
    pub b_needs_movement_completion: bool,
    /// Original m_bEligibleToHeal.
    pub b_eligible_to_heal: bool,
    /// Original m_bHasHiddenVisibility.
    pub b_has_hidden_visibility: bool,
    /// Original m_bHiddenVisibilityForfeited.
    pub b_hidden_visibility_forfeited: bool,
    /// Original m_bIgnoreTerrainCost.
    pub b_ignore_terrain_cost: bool,
    /// Original m_bIgnoreZOC.
    pub b_ignore_zoc: bool,
    /// Original m_bExertZOC.
    pub b_exert_zoc: bool,
    /// Original m_bCanSeeThroughFeatures.
    pub b_can_see_through_features: bool,
    /// Original m_bCanSeeThroughTerrain.
    pub b_can_see_through_terrain: bool,
    /// Original m_bAttackAfterMoving.
    pub b_attack_after_moving: bool,
    /// Original m_iMoveAfterAttacking.
    pub i_move_after_attacking: i32,
    /// Original m_bNeedsAttention.
    pub b_needs_attention: bool,
    /// Original m_bIgnoreResourceMaintenance.
    pub b_ignore_resource_maintenance: bool,
    /// Original m_kHealthModifiers.
    pub k_health_modifiers: HealthModifiers,
    /// Original m_kHealthReligionModifiers.
    pub k_health_religion_modifiers: HealthReligionModifiers,
    /// Original m_kCombatModifers.
    pub k_combat_modifers: CombatModifiers,
    /// Original m_kMovementModifiers.
    pub k_movement_modifiers: MovementModifiers,
    /// Original m_kSightModifiers.
    pub k_sight_modifiers: SightModifiers,
    /// Original m_kAttackRangeModifiers.
    pub k_attack_range_modifiers: AttackRangeModifiers,
    /// Original m_kSightPlots.
    pub k_sight_plots: Vec<SightPlotEntry>,
    /// Original m_aiAbilitiesPossessed.
    pub ai_abilities_possessed: Vec<HashValue<i32>>,
    /// Original m_abInvalidTerrainTypes.
    pub ab_invalid_terrain_types: Vec<HashValue<bool>>,
    /// Original m_abIgnoreTerrainTypes.
    pub ab_ignore_terrain_types: Vec<HashValue<bool>>,
    /// Original m_abIgnoreFeatureTypes.
    pub ab_ignore_feature_types: Vec<HashValue<bool>>,
    /// Original m_aiBonusPointsPerKill.
    pub ai_bonus_points_per_kill: Vec<HashValue<i32>>,
    /// Original m_aiBonusPointsPerKillStrength.
    pub ai_bonus_points_per_kill_strength: Vec<HashValue<i32>>,
    /// Original m_aiBypassWallsPromotionClass.
    pub ai_bypass_walls_promotion_class: Vec<HashValue<u8>>,
    /// Original m_aiEnableWallAttackPromotionClass.
    pub ai_enable_wall_attack_promotion_class: Vec<HashValue<u8>>,
    /// Original m_kExperience.
    pub k_experience: Experience,
    /// Original m_kGreatPerson.
    pub k_great_person: GreatPerson,
    /// Original m_kReligion.
    pub k_religion: Religion,
    /// Original m_kRockBand.
    pub k_rock_band: RockBand,
    /// Original m_kTrade.
    pub k_trade: Trade,
    /// Original m_kArchaeology.
    pub k_archaeology: Archaeology,
    /// Original m_kEspionage.
    pub k_espionage: Espionage,
    /// Original m_kAttributes.
    pub k_attributes: VariantMap,
    /// Original m_kOperations.
    pub k_operations: OperationQueue,
    /// Original m_kOperationsDisabled.
    pub k_operations_disabled: Vec<HashValue<bool>>,
    /// Original m_kHero.
    pub k_hero: Hero,
    /// Original m_kProperties.
    pub k_properties: TypedVariantMap,
}
impl Unit {
    /// These absent17 arrays require actual-definition native Read application:
    /// GreatPersonClass zero defaults, old promotion-class temporary rebuild,
    /// and Operation false defaults plus XP2.IsDisabled overlays. Empty wire
    /// vectors do not assert initialized empty/all-false native destinations.
    pub fn legacy_definition_read_effects_unapplied(&self) -> bool {
        self.wire_version == 17
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        Ok(Self::decode_with_ranges(bytes, limits)?.0)
    }
    pub fn decode_with_ranges(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, Vec<FieldRange>), Error> {
        let mut r = Reader::new(bytes, limits)?;
        let value = Self::read_with_ranges(&mut r)?;
        r.finish()?;
        Ok(value)
    }
    /// Decode one current Unit at a structural cursor, leaving the caller's tail
    /// untouched. Returns consumed bytes and the aggregate dynamic-item charge.
    /// Only the consumed record is subject to the byte limit.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_prefix_with_boundary(bytes, limits).map_err(|boundary| boundary.error)
    }
    pub fn decode_prefix_with_boundary(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), UnitReadBoundary> {
        let clipped = bytes.len() > limits.max_bytes;
        let input = &bytes[..bytes.len().min(limits.max_bytes)];
        let mut r = Reader::new(input, limits).map_err(|error| UnitReadBoundary {
            field: "Unit",
            field_start: 0,
            consumed: 0,
            error,
        })?;
        let (value, _) = Self::read_with_ranges(&mut r).map_err(|error| {
            let error = if clipped
                && matches!(
                    error,
                    Error::Truncated | Error::Variant(variants::VariantError::Truncated)
                ) {
                Error::ResourceLimit
            } else {
                error
            };
            UnitReadBoundary {
                field: r.field,
                field_start: r.field_start,
                consumed: r.pos,
                error,
            }
        })?;
        Ok((value, r.pos, limits.max_items - r.budget))
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(bytes, limits).map(|(value, consumed, _)| (value, consumed))
    }
    fn read_with_ranges(r: &mut Reader<'_>) -> Result<(Self, Vec<FieldRange>), Error> {
        let wire_version = r.i32()?;
        if wire_version != 17 && wire_version != 28 {
            return Err(Error::Version {
                expected: 28,
                actual: wire_version,
            });
        }
        let mut ranges = Vec::new();
        let start = r.pos;
        r.field = "m_id";
        r.field_start = start;
        let id = <u32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_id",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eTypeIndex";
        r.field_start = start;
        let e_type_index = <u32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eTypeIndex",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eAIType";
        r.field_start = start;
        let e_ai_type = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eAIType",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_aiController";
        r.field_start = start;
        let ai_controller = <u32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_aiController",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_aiGoal";
        r.field_start = start;
        let ai_goal = <PlotCoord>::read(r)?;
        ranges.push(FieldRange {
            name: "m_aiGoal",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kLocation";
        r.field_start = start;
        let k_location = <PlotCoord>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kLocation",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eFacingDirection";
        r.field_start = start;
        let e_facing_direction = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eFacingDirection",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eOwner";
        r.field_start = start;
        let e_owner = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eOwner",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eOriginalOwner";
        r.field_start = start;
        let e_original_owner = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eOriginalOwner",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bIsLevied";
        r.field_start = start;
        let b_is_levied = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bIsLevied",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_formationID";
        r.field_start = start;
        let formation_id = <u32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_formationID",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eMilitaryFormation";
        r.field_start = start;
        let e_military_formation = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eMilitaryFormation",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eAutomation";
        r.field_start = start;
        let e_automation = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eAutomation",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eActivity";
        r.field_start = start;
        let e_activity = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eActivity",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iDamage";
        r.field_start = start;
        let i_damage = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iDamage",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iFortifyTurns";
        r.field_start = start;
        let i_fortify_turns = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iFortifyTurns",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iUpgradeDiscount";
        r.field_start = start;
        let i_upgrade_discount = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iUpgradeDiscount",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_xMovesRemaining";
        r.field_start = start;
        let x_moves_remaining = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_xMovesRemaining",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iAttacksRemaining";
        r.field_start = start;
        let i_attacks_remaining = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iAttacksRemaining",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iOperationTimer";
        r.field_start = start;
        let i_operation_timer = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iOperationTimer",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iBuildCharges";
        r.field_start = start;
        let i_build_charges = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iBuildCharges",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iParkCharges";
        r.field_start = start;
        let i_park_charges = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iParkCharges",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iActionCharges";
        r.field_start = start;
        let i_action_charges = if wire_version == 17 {
            0
        } else {
            <i32>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_iActionCharges",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iAirSlotModifier";
        r.field_start = start;
        let i_air_slot_modifier = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iAirSlotModifier",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bDominantConquer";
        r.field_start = start;
        let b_dominant_conquer = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bDominantConquer",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iBarbarianTribeIndex";
        r.field_start = start;
        let i_barbarian_tribe_index = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_iBarbarianTribeIndex",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_eBarbarianPlayerTarget";
        r.field_start = start;
        let e_barbarian_player_target = <i32>::read(r)?;
        ranges.push(FieldRange {
            name: "m_eBarbarianPlayerTarget",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iDisasterCharges";
        r.field_start = start;
        let i_disaster_charges = if wire_version == 17 {
            None
        } else {
            Some(<i32>::read(r)?)
        };
        ranges.push(FieldRange {
            name: "m_iDisasterCharges",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iEraStrengthModifier";
        r.field_start = start;
        let i_era_strength_modifier = if wire_version == 17 {
            0
        } else {
            <i32>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_iEraStrengthModifier",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bEmbarked";
        r.field_start = start;
        let b_embarked = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bEmbarked",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bDelayedDeath";
        r.field_start = start;
        let b_delayed_death = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bDelayedDeath",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bImmobile";
        r.field_start = start;
        let b_immobile = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bImmobile",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bProcessedTurn";
        r.field_start = start;
        let b_processed_turn = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bProcessedTurn",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bNeedsMovementCompletion";
        r.field_start = start;
        let b_needs_movement_completion = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bNeedsMovementCompletion",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bEligibleToHeal";
        r.field_start = start;
        let b_eligible_to_heal = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bEligibleToHeal",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bHasHiddenVisibility";
        r.field_start = start;
        let b_has_hidden_visibility = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bHasHiddenVisibility",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bHiddenVisibilityForfeited";
        r.field_start = start;
        let b_hidden_visibility_forfeited = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bHiddenVisibilityForfeited",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bIgnoreTerrainCost";
        r.field_start = start;
        let b_ignore_terrain_cost = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bIgnoreTerrainCost",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bIgnoreZOC";
        r.field_start = start;
        let b_ignore_zoc = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bIgnoreZOC",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bExertZOC";
        r.field_start = start;
        let b_exert_zoc = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bExertZOC",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bCanSeeThroughFeatures";
        r.field_start = start;
        let b_can_see_through_features = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bCanSeeThroughFeatures",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bCanSeeThroughTerrain";
        r.field_start = start;
        let b_can_see_through_terrain = if wire_version == 17 {
            false
        } else {
            <bool>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_bCanSeeThroughTerrain",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bAttackAfterMoving";
        r.field_start = start;
        let b_attack_after_moving = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bAttackAfterMoving",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_iMoveAfterAttacking";
        r.field_start = start;
        let i_move_after_attacking = if wire_version == 17 {
            i32::from(<bool>::read(r)?)
        } else {
            <i32>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_iMoveAfterAttacking",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bNeedsAttention";
        r.field_start = start;
        let b_needs_attention = <bool>::read(r)?;
        ranges.push(FieldRange {
            name: "m_bNeedsAttention",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_bIgnoreResourceMaintenance";
        r.field_start = start;
        let b_ignore_resource_maintenance = if wire_version == 17 {
            false
        } else {
            <bool>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_bIgnoreResourceMaintenance",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kHealthModifiers";
        r.field_start = start;
        let k_health_modifiers = <HealthModifiers>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kHealthModifiers",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kHealthReligionModifiers";
        r.field_start = start;
        let k_health_religion_modifiers = <HealthReligionModifiers>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kHealthReligionModifiers",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kCombatModifers";
        r.field_start = start;
        let k_combat_modifers = <CombatModifiers>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kCombatModifers",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kMovementModifiers";
        r.field_start = start;
        let k_movement_modifiers = <MovementModifiers>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kMovementModifiers",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kSightModifiers";
        r.field_start = start;
        let k_sight_modifiers = <SightModifiers>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kSightModifiers",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kAttackRangeModifiers";
        r.field_start = start;
        let k_attack_range_modifiers = <AttackRangeModifiers>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kAttackRangeModifiers",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kSightPlots";
        r.field_start = start;
        let k_sight_plots = <Vec<SightPlotEntry>>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kSightPlots",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_aiAbilitiesPossessed";
        r.field_start = start;
        let ai_abilities_possessed = <Vec<HashValue<i32>>>::read(r)?;
        ranges.push(FieldRange {
            name: "m_aiAbilitiesPossessed",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_abInvalidTerrainTypes";
        r.field_start = start;
        let ab_invalid_terrain_types = <Vec<HashValue<bool>>>::read(r)?;
        ranges.push(FieldRange {
            name: "m_abInvalidTerrainTypes",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_abIgnoreTerrainTypes";
        r.field_start = start;
        let ab_ignore_terrain_types = <Vec<HashValue<bool>>>::read(r)?;
        ranges.push(FieldRange {
            name: "m_abIgnoreTerrainTypes",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_abIgnoreFeatureTypes";
        r.field_start = start;
        let ab_ignore_feature_types = <Vec<HashValue<bool>>>::read(r)?;
        ranges.push(FieldRange {
            name: "m_abIgnoreFeatureTypes",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_aiBonusPointsPerKill";
        r.field_start = start;
        let ai_bonus_points_per_kill = <Vec<HashValue<i32>>>::read(r)?;
        ranges.push(FieldRange {
            name: "m_aiBonusPointsPerKill",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_aiBonusPointsPerKillStrength";
        r.field_start = start;
        let ai_bonus_points_per_kill_strength = if wire_version == 17 {
            Vec::new()
        } else {
            <Vec<HashValue<i32>>>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_aiBonusPointsPerKillStrength",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_aiBypassWallsPromotionClass";
        r.field_start = start;
        let ai_bypass_walls_promotion_class = if wire_version == 17 {
            Vec::new()
        } else {
            <Vec<HashValue<u8>>>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_aiBypassWallsPromotionClass",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_aiEnableWallAttackPromotionClass";
        r.field_start = start;
        let ai_enable_wall_attack_promotion_class = if wire_version == 17 {
            Vec::new()
        } else {
            <Vec<HashValue<u8>>>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_aiEnableWallAttackPromotionClass",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kExperience";
        r.field_start = start;
        let k_experience = <Experience>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kExperience",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kGreatPerson";
        r.field_start = start;
        let k_great_person = <GreatPerson>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kGreatPerson",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kReligion";
        r.field_start = start;
        let k_religion = <Religion>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kReligion",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kRockBand";
        r.field_start = start;
        let k_rock_band = <RockBand>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kRockBand",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kTrade";
        r.field_start = start;
        let k_trade = <Trade>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kTrade",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kArchaeology";
        r.field_start = start;
        let k_archaeology = <Archaeology>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kArchaeology",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kEspionage";
        r.field_start = start;
        let k_espionage = <Espionage>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kEspionage",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kAttributes";
        r.field_start = start;
        let k_attributes = <VariantMap>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kAttributes",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kOperations";
        r.field_start = start;
        let k_operations = <OperationQueue>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kOperations",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kOperationsDisabled";
        r.field_start = start;
        let k_operations_disabled = if wire_version == 17 {
            Vec::new()
        } else {
            <Vec<HashValue<bool>>>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_kOperationsDisabled",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kHero";
        r.field_start = start;
        let k_hero = if wire_version == 17 {
            Hero {
                e_class: u32::MAX,
                k_origin_city_id: ComponentId {
                    owner: -1,
                    component_type: 2,
                    id: u32::MAX,
                },
            }
        } else {
            <Hero>::read(r)?
        };
        ranges.push(FieldRange {
            name: "m_kHero",
            start,
            end: r.pos,
        });
        let start = r.pos;
        r.field = "m_kProperties";
        r.field_start = start;
        let k_properties = <TypedVariantMap>::read(r)?;
        ranges.push(FieldRange {
            name: "m_kProperties",
            start,
            end: r.pos,
        });
        Ok((
            Self {
                wire_version,
                id,
                e_type_index,
                e_ai_type,
                ai_controller,
                ai_goal,
                k_location,
                e_facing_direction,
                e_owner,
                e_original_owner,
                b_is_levied,
                formation_id,
                e_military_formation,
                e_automation,
                e_activity,
                i_damage,
                i_fortify_turns,
                i_upgrade_discount,
                x_moves_remaining,
                i_attacks_remaining,
                i_operation_timer,
                i_build_charges,
                i_park_charges,
                i_action_charges,
                i_air_slot_modifier,
                b_dominant_conquer,
                i_barbarian_tribe_index,
                e_barbarian_player_target,
                i_disaster_charges,
                i_era_strength_modifier,
                b_embarked,
                b_delayed_death,
                b_immobile,
                b_processed_turn,
                b_needs_movement_completion,
                b_eligible_to_heal,
                b_has_hidden_visibility,
                b_hidden_visibility_forfeited,
                b_ignore_terrain_cost,
                b_ignore_zoc,
                b_exert_zoc,
                b_can_see_through_features,
                b_can_see_through_terrain,
                b_attack_after_moving,
                i_move_after_attacking,
                b_needs_attention,
                b_ignore_resource_maintenance,
                k_health_modifiers,
                k_health_religion_modifiers,
                k_combat_modifers,
                k_movement_modifiers,
                k_sight_modifiers,
                k_attack_range_modifiers,
                k_sight_plots,
                ai_abilities_possessed,
                ab_invalid_terrain_types,
                ab_ignore_terrain_types,
                ab_ignore_feature_types,
                ai_bonus_points_per_kill,
                ai_bonus_points_per_kill_strength,
                ai_bypass_walls_promotion_class,
                ai_enable_wall_attack_promotion_class,
                k_experience,
                k_great_person,
                k_religion,
                k_rock_band,
                k_trade,
                k_archaeology,
                k_espionage,
                k_attributes,
                k_operations,
                k_operations_disabled,
                k_hero,
                k_properties,
            },
            ranges,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(bytes, _)| bytes)
    }
    /// Encode the supported record and report its aggregate dynamic-item charge.
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if self.wire_version != 28 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let disaster_charges = self
            .i_disaster_charges
            .ok_or(Error::MissingRetainedValue("m_iDisasterCharges"))?;
        let mut w = Writer {
            bytes: Vec::new(),
            budget: limits.max_items,
            max_bytes: limits.max_bytes,
        };
        (28i32).write(&mut w)?;
        self.id.write(&mut w)?;
        self.e_type_index.write(&mut w)?;
        self.e_ai_type.write(&mut w)?;
        self.ai_controller.write(&mut w)?;
        self.ai_goal.write(&mut w)?;
        self.k_location.write(&mut w)?;
        self.e_facing_direction.write(&mut w)?;
        self.e_owner.write(&mut w)?;
        self.e_original_owner.write(&mut w)?;
        self.b_is_levied.write(&mut w)?;
        self.formation_id.write(&mut w)?;
        self.e_military_formation.write(&mut w)?;
        self.e_automation.write(&mut w)?;
        self.e_activity.write(&mut w)?;
        self.i_damage.write(&mut w)?;
        self.i_fortify_turns.write(&mut w)?;
        self.i_upgrade_discount.write(&mut w)?;
        self.x_moves_remaining.write(&mut w)?;
        self.i_attacks_remaining.write(&mut w)?;
        self.i_operation_timer.write(&mut w)?;
        self.i_build_charges.write(&mut w)?;
        self.i_park_charges.write(&mut w)?;
        self.i_action_charges.write(&mut w)?;
        self.i_air_slot_modifier.write(&mut w)?;
        self.b_dominant_conquer.write(&mut w)?;
        self.i_barbarian_tribe_index.write(&mut w)?;
        self.e_barbarian_player_target.write(&mut w)?;
        disaster_charges.write(&mut w)?;
        self.i_era_strength_modifier.write(&mut w)?;
        self.b_embarked.write(&mut w)?;
        self.b_delayed_death.write(&mut w)?;
        self.b_immobile.write(&mut w)?;
        self.b_processed_turn.write(&mut w)?;
        self.b_needs_movement_completion.write(&mut w)?;
        self.b_eligible_to_heal.write(&mut w)?;
        self.b_has_hidden_visibility.write(&mut w)?;
        self.b_hidden_visibility_forfeited.write(&mut w)?;
        self.b_ignore_terrain_cost.write(&mut w)?;
        self.b_ignore_zoc.write(&mut w)?;
        self.b_exert_zoc.write(&mut w)?;
        self.b_can_see_through_features.write(&mut w)?;
        self.b_can_see_through_terrain.write(&mut w)?;
        self.b_attack_after_moving.write(&mut w)?;
        self.i_move_after_attacking.write(&mut w)?;
        self.b_needs_attention.write(&mut w)?;
        self.b_ignore_resource_maintenance.write(&mut w)?;
        self.k_health_modifiers.write(&mut w)?;
        self.k_health_religion_modifiers.write(&mut w)?;
        self.k_combat_modifers.write(&mut w)?;
        self.k_movement_modifiers.write(&mut w)?;
        self.k_sight_modifiers.write(&mut w)?;
        self.k_attack_range_modifiers.write(&mut w)?;
        self.k_sight_plots.write(&mut w)?;
        self.ai_abilities_possessed.write(&mut w)?;
        self.ab_invalid_terrain_types.write(&mut w)?;
        self.ab_ignore_terrain_types.write(&mut w)?;
        self.ab_ignore_feature_types.write(&mut w)?;
        self.ai_bonus_points_per_kill.write(&mut w)?;
        self.ai_bonus_points_per_kill_strength.write(&mut w)?;
        self.ai_bypass_walls_promotion_class.write(&mut w)?;
        self.ai_enable_wall_attack_promotion_class.write(&mut w)?;
        self.k_experience.write(&mut w)?;
        self.k_great_person.write(&mut w)?;
        self.k_religion.write(&mut w)?;
        self.k_rock_band.write(&mut w)?;
        self.k_trade.write(&mut w)?;
        self.k_archaeology.write(&mut w)?;
        self.k_espionage.write(&mut w)?;
        self.k_attributes.write(&mut w)?;
        self.k_operations.write(&mut w)?;
        self.k_operations_disabled.write(&mut w)?;
        self.k_hero.write(&mut w)?;
        self.k_properties.write(&mut w)?;
        Ok((w.bytes, limits.max_items - w.budget))
    }
}

macro_rules! variant_wire {
    ($t:ty) => {
        impl Wire for $t {
            const MIN: usize = 4;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                let mut budget = variants::DecodeBudget::new(variants::Limits {
                    max_bytes: r.bytes.len() - r.pos,
                    max_items: r.budget,
                    max_depth: 64,
                });
                let (value, consumed) =
                    <$t>::decode_prefix(&r.bytes[r.pos..], &mut budget).map_err(variant_error)?;
                r.pos += consumed;
                r.budget = budget.remaining_items();
                Ok(value)
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                let (bytes, items) = self
                    .encode(variants::Limits {
                        max_bytes: w.max_bytes - w.bytes.len(),
                        max_items: w.budget,
                        max_depth: 64,
                    })
                    .map_err(variant_error)?;
                w.claim(items)?;
                w.put(&bytes)
            }
        }
    };
}
variant_wire!(VariantMap);
variant_wire!(TypedVariantMap);

fn variant_error(error: variants::VariantError) -> Error {
    if error == variants::VariantError::Limit {
        Error::ResourceLimit
    } else {
        Error::Variant(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn writer() -> Writer {
        Writer {
            bytes: Vec::new(),
            budget: 100,
            max_bytes: 1000,
        }
    }
    #[test]
    fn prefix_preserves_version_errors_and_distinguishes_clipping() {
        let limits = Limits {
            max_bytes: 4,
            max_items: 0,
        };
        assert_eq!(
            Unit::decode_prefix(&[28, 0, 0], limits),
            Err(Error::Truncated)
        );
        assert_eq!(
            Unit::decode_prefix(&[28, 0, 0, 0, 0], limits),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Unit::decode(&[28, 0, 0, 0, 0], limits),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Unit::decode_prefix(&[27, 0, 0, 0, 0], limits),
            Err(Error::Version {
                expected: 28,
                actual: 27
            })
        );
        assert_eq!(
            Unit::decode_prefix(
                &[28, 0, 0, 0],
                Limits {
                    max_bytes: 0,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn independent_health_and_component_id_wire_boundaries() {
        let mut bytes: Vec<u8> = [2u32, 0x80000000, 0x7fffffff, 0xffffffff]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        bytes.push(1);
        let mut r = Reader::new(&bytes, Limits::default()).unwrap();
        let health = HealthModifiers::read(&mut r).unwrap();
        r.finish().unwrap();
        assert_eq!(health.i_extra_enemy_heal, i32::MIN);
        assert_eq!(health.i_extra_neutral_heal, i32::MAX);
        assert_eq!(health.i_extra_friendly_heal, -1);
        assert!(health.b_can_heal_after_action);
        let mut w = writer();
        health.write(&mut w).unwrap();
        assert_eq!(w.bytes, bytes);
        let bytes: Vec<u8> = [0x7fff8000u32, 0xffffffff]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        let mut r = Reader::new(&bytes, Limits::default()).unwrap();
        let id = ComponentId::read(&mut r).unwrap();
        r.finish().unwrap();
        assert_eq!(
            id,
            ComponentId {
                owner: i16::MIN,
                component_type: i16::MAX,
                id: u32::MAX
            }
        );
        let mut w = writer();
        id.write(&mut w).unwrap();
        assert_eq!(w.bytes, bytes);
    }
    #[test]
    fn definition_pairs_preserve_nonbool_byte_values_order_and_duplicate_hashes() {
        let bytes = [
            2u8, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 2, 0xff, 0xff, 0xff, 0xff, 255,
        ];
        let mut r = Reader::new(&bytes, Limits::default()).unwrap();
        let pairs = Vec::<HashValue<u8>>::read(&mut r).unwrap();
        r.finish().unwrap();
        assert_eq!(
            pairs,
            [
                HashValue {
                    definition_hash: u32::MAX,
                    value: 2
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: 255
                }
            ]
        );
        let mut w = writer();
        pairs.write(&mut w).unwrap();
        assert_eq!(w.bytes, bytes);
        assert_eq!(
            Vec::<HashValue<bool>>::read(&mut Reader::new(&bytes, Limits::default()).unwrap()),
            Err(Error::NoncanonicalBool(2))
        );
        let bytes = [3u8, 0, 0, 0, 255, 0, 128];
        let mut r = Reader::new(&bytes, Limits::default()).unwrap();
        let raw = Vec::<u8>::read(&mut r).unwrap();
        r.finish().unwrap();
        assert_eq!(raw, [255, 0, 128]);
        let mut w = writer();
        raw.write(&mut w).unwrap();
        assert_eq!(w.bytes, bytes);
    }
    fn legacy17_fixture() -> (Vec<u8>, Unit) {
        let mut current = crate::unit_turn_upkeep::tests::source_only_unit();
        current.i_action_charges = 7;
        current.i_disaster_charges = Some(9);
        current.i_era_strength_modifier = 11;
        current.b_can_see_through_terrain = true;
        current.b_ignore_resource_maintenance = true;
        current.i_move_after_attacking = 1;
        current.ai_abilities_possessed = vec![HashValue {
            definition_hash: 0x80000001,
            value: 4,
        }];
        current.ai_bonus_points_per_kill_strength = vec![HashValue {
            definition_hash: 0x80000002,
            value: -257,
        }];
        current.ai_bypass_walls_promotion_class = vec![HashValue {
            definition_hash: 0x80000003,
            value: 7,
        }];
        current.ai_enable_wall_attack_promotion_class = vec![HashValue {
            definition_hash: 0x80000004,
            value: 8,
        }];
        current.k_operations_disabled = vec![HashValue {
            definition_hash: 0x80000005,
            value: true,
        }];
        let bytes = current.encode(Limits::default()).unwrap();
        let (_, ranges) = Unit::decode_with_ranges(&bytes, Limits::default()).unwrap();
        let mut legacy = 17i32.to_le_bytes().to_vec();
        for range in ranges {
            match range.name {
                "m_iActionCharges"
                | "m_iDisasterCharges"
                | "m_iEraStrengthModifier"
                | "m_bCanSeeThroughTerrain"
                | "m_bIgnoreResourceMaintenance"
                | "m_aiBonusPointsPerKillStrength"
                | "m_aiBypassWallsPromotionClass"
                | "m_aiEnableWallAttackPromotionClass"
                | "m_kOperationsDisabled"
                | "m_kHero" => {}
                "m_iMoveAfterAttacking" => legacy.push(1),
                _ => legacy.extend_from_slice(&bytes[range.start..range.end]),
            }
        }
        current.wire_version = 17;
        current.i_action_charges = 0;
        current.i_disaster_charges = None;
        current.i_era_strength_modifier = 0;
        current.b_can_see_through_terrain = false;
        current.b_ignore_resource_maintenance = false;
        current.ai_bonus_points_per_kill_strength.clear();
        current.ai_bypass_walls_promotion_class.clear();
        current.ai_enable_wall_attack_promotion_class.clear();
        current.k_operations_disabled.clear();
        current.k_hero = Hero {
            e_class: u32::MAX,
            k_origin_city_id: ComponentId {
                owner: -1,
                component_type: 2,
                id: u32::MAX,
            },
        };
        (legacy, current)
    }
    #[test]
    fn legacy17_presence_defaults_bool_migration_and_nested_independence() {
        let (b, expected) = legacy17_fixture();
        let (value, ranges) = Unit::decode_with_ranges(&b, Limits::default()).unwrap();
        assert_eq!(value, expected);
        assert!(value.legacy_definition_read_effects_unapplied());
        assert_eq!(
            value.encode(Limits::default()),
            Err(Error::UnsupportedLegacyWrite)
        );
        assert_eq!(
            ranges
                .iter()
                .find(|r| r.name == "m_iDisasterCharges")
                .map(|r| r.end - r.start),
            Some(0)
        );
        assert_eq!(
            ranges
                .iter()
                .find(|r| r.name == "m_iMoveAfterAttacking")
                .map(|r| r.end - r.start),
            Some(1)
        );
        let combat = ranges
            .iter()
            .find(|r| r.name == "m_kCombatModifiers" || r.name == "m_kCombatModifers")
            .unwrap();
        let mut bad = b.clone();
        bad[combat.start..combat.start + 4].copy_from_slice(&17i32.to_le_bytes());
        let boundary = Unit::decode_prefix_with_boundary(&bad, Limits::default()).unwrap_err();
        assert_eq!(
            (boundary.field, boundary.field_start),
            (combat.name, combat.start)
        );
        assert_eq!(
            boundary.error,
            Error::Version {
                expected: 28,
                actual: 17
            }
        );
        let mut current = crate::unit_turn_upkeep::tests::source_only_unit();
        current.i_disaster_charges = None;
        assert_eq!(
            current.encode(Limits::default()),
            Err(Error::MissingRetainedValue("m_iDisasterCharges"))
        );
    }
    #[test]
    fn legacy17_exact_budgets_truncation_and_unsupported_neighbor() {
        let (b, _) = legacy17_fixture();
        let (_, n, items) = Unit::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!(n, b.len());
        assert!(items > 0);
        assert!(Unit::decode_prefix_with_items(
            &b,
            Limits {
                max_bytes: b.len(),
                max_items: items
            }
        )
        .is_ok());
        assert_eq!(
            Unit::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: b.len() - 1,
                    max_items: items
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Unit::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: b.len(),
                    max_items: items - 1
                }
            ),
            Err(Error::ResourceLimit)
        );
        for end in 0..b.len() {
            assert!(
                Unit::decode(&b[..end], Limits::default()).is_err(),
                "prefix {end}"
            );
        }
        let mut bad = b.clone();
        bad[..4].copy_from_slice(&18i32.to_le_bytes());
        assert_eq!(
            Unit::decode(&bad, Limits::default()),
            Err(Error::Version {
                expected: 28,
                actual: 18
            })
        );
        let mut trailing = b;
        trailing.push(0);
        assert_eq!(
            Unit::decode(&trailing, Limits::default()),
            Err(Error::TrailingBytes)
        );
    }
    fn combat20_fixture() -> (Vec<u8>, CombatModifiers) {
        let mut current = crate::unit_turn_upkeep::tests::source_only_unit().k_combat_modifers;
        current.i_versus_barbarians = i32::MIN;
        current.e_captured_combat_unit_type = 0x80000001;
        current.b_bypass_walls = true;
        current.i_water_damage_resistance = 37;
        current.i_number_holy_cities_combat_modifier = 37;
        current.i_number_military_policies_combat_modifier = 37;
        current.i_number_neighbor_combat_bonus = 37;
        current.i_anti_air_strength_modifier = 37;
        current.i_combat_strength_modifier = 37;
        current.i_ranged_strength_modifier = 37;
        current.i_bombard_strength_modifier = 37;
        current.i_direct_loyalty_damage = 37;
        current.b_fight_while_embarked = true;
        let mut w = writer();
        current.write(&mut w).unwrap();
        assert_eq!(w.bytes.len(), 135);
        let mut bytes = w.bytes;
        bytes[..4].copy_from_slice(&20i32.to_le_bytes());
        bytes.truncate(98);
        current.wire_version = 20;
        current.i_water_damage_resistance = 0;
        current.i_number_holy_cities_combat_modifier = 0;
        current.i_number_military_policies_combat_modifier = 0;
        current.i_number_neighbor_combat_bonus = 0;
        current.i_anti_air_strength_modifier = 0;
        current.i_combat_strength_modifier = 0;
        current.i_ranged_strength_modifier = 0;
        current.i_bombard_strength_modifier = 0;
        current.i_direct_loyalty_damage = 0;
        current.b_fight_while_embarked = false;
        (bytes, current)
    }
    #[test]
    fn combat20_explicit_defaults_and_independent_own_tag() {
        let (bytes, expected) = combat20_fixture();
        let mut r = Reader::new(
            &bytes,
            Limits {
                max_bytes: 98,
                max_items: 0,
            },
        )
        .unwrap();
        let value = CombatModifiers::read(&mut r).unwrap();
        r.finish().unwrap();
        assert_eq!(value, expected);
        assert_eq!(
            value.write(&mut writer()),
            Err(Error::UnsupportedLegacyWrite)
        );
        // A current outer Unit can independently contain older Combat20.
        let unit = crate::unit_turn_upkeep::tests::source_only_unit();
        let original = unit.encode(Limits::default()).unwrap();
        let (_, ranges) = Unit::decode_with_ranges(&original, Limits::default()).unwrap();
        let range = ranges
            .iter()
            .find(|r| r.name == "m_kCombatModifers")
            .unwrap();
        let mut mixed = original[..range.start].to_vec();
        mixed.extend_from_slice(&bytes);
        mixed.extend_from_slice(&original[range.end..]);
        let decoded = Unit::decode(&mixed, Limits::default()).unwrap();
        assert_eq!(decoded.wire_version, 28);
        assert_eq!(decoded.k_combat_modifers, expected);
    }
    #[test]
    fn combat20_truncation_bool_and_neighbor_refusals() {
        let (bytes, _) = combat20_fixture();
        for end in 0..bytes.len() {
            let mut r = Reader::new(&bytes[..end], Limits::default()).unwrap();
            assert!(CombatModifiers::read(&mut r).is_err(), "prefix {end}");
        }
        assert!(matches!(
            Reader::new(
                &bytes,
                Limits {
                    max_bytes: 97,
                    max_items: 0
                }
            ),
            Err(Error::ResourceLimit)
        ));
        let mut bad = bytes.clone();
        bad[12] = 2;
        assert_eq!(
            CombatModifiers::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::NoncanonicalBool(2))
        );
        bad[..4].copy_from_slice(&21i32.to_le_bytes());
        assert_eq!(
            CombatModifiers::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::Version {
                expected: 28,
                actual: 21
            })
        );
        let mut trailing = bytes;
        trailing.push(0);
        let mut r = Reader::new(&trailing, Limits::default()).unwrap();
        CombatModifiers::read(&mut r).unwrap();
        assert_eq!(r.finish(), Err(Error::TrailingBytes));
    }
    #[test]
    fn experience6_omits_only_promote_no_finish_moves() {
        let mut value = crate::unit_turn_upkeep::tests::source_only_unit().k_experience;
        value.b_can_promote = true;
        value.b_experience_locked = true;
        value.i_experience = -257;
        value.i_level = 3;
        value.i_stored_promotions = 5;
        value.i_promote_no_finish_moves = 37;
        value.x_general_experience_modifier = -513;
        value.ab_promotions_earned = vec![
            HashValue {
                definition_hash: 0x80000001,
                value: true,
            },
            HashValue {
                definition_hash: 0x80000001,
                value: false,
            },
        ];
        value.s_veteran_name = vec![0, 255, b'x'];
        value.b_can_promote_notify_needed = true;
        value.x_attacking_experience_modifier_percent = 257;
        let mut w = writer();
        value.write(&mut w).unwrap();
        let mut current_reader = Reader::new(&w.bytes, Limits::default()).unwrap();
        assert_eq!(Experience::read(&mut current_reader).unwrap(), value);
        current_reader.finish().unwrap();
        // Header + two bools + experience/level/stored scalar words precede
        // the since7 word. Preserve both counted vectors and trailing fields.
        let mut b = 6i32.to_le_bytes().to_vec();
        b.extend_from_slice(&w.bytes[4..18]);
        b.extend_from_slice(&w.bytes[22..]);
        value.wire_version = 6;
        value.i_promote_no_finish_moves = 0;
        let mut r = Reader::new(
            &b,
            Limits {
                max_bytes: b.len(),
                max_items: 5,
            },
        )
        .unwrap();
        assert_eq!(Experience::read(&mut r).unwrap(), value);
        assert_eq!(r.budget, 0);
        r.finish().unwrap();
        assert_eq!(
            value.write(&mut writer()),
            Err(Error::UnsupportedLegacyWrite)
        );
        for end in 0..b.len() {
            let mut r = Reader::new(&b[..end], Limits::default()).unwrap();
            assert!(Experience::read(&mut r).is_err(), "prefix {end}");
        }
        let mut r = Reader::new(
            &b,
            Limits {
                max_bytes: b.len(),
                max_items: 4,
            },
        )
        .unwrap();
        assert_eq!(Experience::read(&mut r), Err(Error::ResourceLimit));
        let mut bad = b.clone();
        bad[4] = 2;
        assert_eq!(
            Experience::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::NoncanonicalBool(2))
        );
        bad[..4].copy_from_slice(&5i32.to_le_bytes());
        assert_eq!(
            Experience::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::Version {
                expected: 7,
                actual: 5
            })
        );
    }
    #[test]
    fn unit_religion6_keeps_population_rows_and_exact_scalar_defaults() {
        let mut value = crate::unit_turn_upkeep::tests::source_only_unit().k_religion;
        value.i_religious_strength = -257;
        value.e_religion_type = 0x80000001;
        value.i_foreign_spread_modifier = -7;
        value.b_land_victory_spread = true;
        value.i_land_victory_spread = 37;
        value.e_relic_source = 5;
        value.a_initiation_yield_population = vec![InitiationYieldPopulation {
            e_yield_type: i32::MIN,
            i_population_amount: -9,
        }];
        let mut w = writer();
        value.write(&mut w).unwrap();
        let mut r = Reader::new(&w.bytes, Limits::default()).unwrap();
        assert_eq!(Religion::read(&mut r).unwrap(), value);
        r.finish().unwrap();
        let mut bytes = 6i32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&w.bytes[4..44]);
        bytes.extend_from_slice(&w.bytes[48..w.bytes.len() - 4]);
        value.wire_version = 6;
        value.i_land_victory_spread = 0;
        value.e_relic_source = -1;
        assert_eq!(bytes.len(), 68);
        let mut r = Reader::new(
            &bytes,
            Limits {
                max_bytes: bytes.len(),
                max_items: 1,
            },
        )
        .unwrap();
        assert_eq!(Religion::read(&mut r).unwrap(), value);
        assert_eq!(r.budget, 0);
        r.finish().unwrap();
        assert_eq!(
            value.write(&mut writer()),
            Err(Error::UnsupportedLegacyWrite)
        );
        for end in 0..bytes.len() {
            let mut r = Reader::new(&bytes[..end], Limits::default()).unwrap();
            assert!(Religion::read(&mut r).is_err(), "prefix {end}");
        }
        let mut r = Reader::new(
            &bytes,
            Limits {
                max_bytes: bytes.len(),
                max_items: 0,
            },
        )
        .unwrap();
        assert_eq!(Religion::read(&mut r), Err(Error::ResourceLimit));
        let mut bad = bytes.clone();
        bad[40] = 2;
        assert_eq!(
            Religion::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::NoncanonicalBool(2))
        );
        bad = bytes.clone();
        bad[56..60].copy_from_slice(&2i32.to_le_bytes());
        assert_eq!(
            Religion::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::Version {
                expected: 1,
                actual: 2
            })
        );
        bad[..4].copy_from_slice(&7i32.to_le_bytes());
        assert_eq!(
            Religion::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::Version {
                expected: 8,
                actual: 7
            })
        );
    }
    #[test]
    fn rockband6_keeps_all_five_arrays_and_parent_reset_concert_baseline() {
        let mut value = crate::unit_turn_upkeep::tests::source_only_unit().k_rock_band;
        value.b_max_level = true;
        value.b_max_promotions = true;
        value.i_level = 6;
        value.i_album_sales = i32::MIN;
        value.i_modified_rockband_level_natural_wonder = 13;
        value.i_modified_tourism_bomb_value_natural_wonder = 14;
        value.i_modified_tourism_bomb_value_target_peace = 15;
        value.a_modified_yield_per_tourism_bomb = vec![
            HashValue {
                definition_hash: 0x80000001,
                value: -257,
            },
            HashValue {
                definition_hash: 0x80000001,
                value: 513,
            },
        ];
        value.ai_concert_plots = vec![17, -1];
        value.ai_modified_rockband_level_district = vec![HashValue {
            definition_hash: 0x80000002,
            value: -1,
        }];
        value.ai_modified_rockband_level_improvement = vec![HashValue {
            definition_hash: 0x80000003,
            value: 2,
        }];
        value.ai_modified_tourism_bomb_value_district = vec![HashValue {
            definition_hash: 0x80000004,
            value: 3,
        }];
        value.ai_modified_tourism_bomb_value_improvement = vec![HashValue {
            definition_hash: 0x80000005,
            value: 4,
        }];
        let mut current = writer();
        value.write(&mut current).unwrap();
        let mut r = Reader::new(&current.bytes, Limits::default()).unwrap();
        assert_eq!(RockBand::read(&mut r).unwrap(), value);
        r.finish().unwrap();
        let mut old = writer();
        6i32.write(&mut old).unwrap();
        value.i_level.write(&mut old).unwrap();
        value.i_album_sales.write(&mut old).unwrap();
        value
            .i_post_tourism_bomb_foreign_loyalty
            .write(&mut old)
            .unwrap();
        value
            .i_modified_rockband_level_national_park
            .write(&mut old)
            .unwrap();
        value
            .i_modified_tourism_bomb_value_national_park
            .write(&mut old)
            .unwrap();
        value
            .i_modified_tourism_bomb_player_range
            .write(&mut old)
            .unwrap();
        value
            .i_modified_tourism_bomb_value_at_range
            .write(&mut old)
            .unwrap();
        value
            .i_modified_tourism_bomb_convert_city
            .write(&mut old)
            .unwrap();
        value
            .a_modified_yield_per_tourism_bomb
            .write(&mut old)
            .unwrap();
        value
            .ai_modified_rockband_level_district
            .write(&mut old)
            .unwrap();
        value
            .ai_modified_rockband_level_improvement
            .write(&mut old)
            .unwrap();
        value
            .ai_modified_tourism_bomb_value_district
            .write(&mut old)
            .unwrap();
        value
            .ai_modified_tourism_bomb_value_improvement
            .write(&mut old)
            .unwrap();
        value.wire_version = 6;
        value.b_max_level = false;
        value.b_max_promotions = false;
        value.i_modified_rockband_level_natural_wonder = 0;
        value.i_modified_tourism_bomb_value_natural_wonder = 0;
        value.i_modified_tourism_bomb_value_target_peace = 0;
        value.ai_concert_plots.clear();
        let bytes = old.bytes;
        assert_eq!(bytes.len(), 104);
        let mut r = Reader::new(
            &bytes,
            Limits {
                max_bytes: bytes.len(),
                max_items: 6,
            },
        )
        .unwrap();
        assert_eq!(RockBand::read(&mut r).unwrap(), value);
        assert_eq!(r.budget, 0);
        r.finish().unwrap();
        assert_eq!(
            value.write(&mut writer()),
            Err(Error::UnsupportedLegacyWrite)
        );
        for end in 0..bytes.len() {
            let mut r = Reader::new(&bytes[..end], Limits::default()).unwrap();
            assert!(RockBand::read(&mut r).is_err(), "prefix {end}");
        }
        let mut r = Reader::new(
            &bytes,
            Limits {
                max_bytes: bytes.len(),
                max_items: 5,
            },
        )
        .unwrap();
        assert_eq!(RockBand::read(&mut r), Err(Error::ResourceLimit));
        let mut bad = bytes.clone();
        bad[36..40].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            RockBand::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::ResourceLimit)
        );
        bad[..4].copy_from_slice(&7i32.to_le_bytes());
        assert_eq!(
            RockBand::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::Version {
                expected: 10,
                actual: 7
            })
        );
    }
    #[test]
    fn unit_espionage6_two_defaults_and_independent_nested_records() {
        let mut value = crate::unit_turn_upkeep::tests::source_only_unit().k_espionage;
        value.a_espionage_bonuses = vec![EspionageBonus {
            wire_version: 7,
            e_operation_type: 0x80000001,
            b_offensive: true,
            i_amount: -5,
        }];
        value.a_espionage_time_reductions = vec![EspionageTimeReduction {
            wire_version: 7,
            e_operation_type: 0x80000002,
            i_reduction_percent: -3,
        }];
        value.e_operation = 0x80000003;
        value.i_operation_end_turn = i32::MIN;
        value.k_operation_end_destination = PlotCoord { x: -7, y: i32::MAX };
        value.i_nerf_all_enemy_spies = 37;
        value.i_counter_spy_adjacent_level_boost = 38;
        value.i_visibility_reduction_all_cities = -11;
        value.b_counter_spy_entire_city = true;
        value.e_escape_route = 0x80000004;
        value.b_escaping = true;
        value.e_stolen_boost = 0x80000005;
        value.i_siphoned_gold = -257;
        value.i_stolen_great_work_index = -1;
        let mut w = writer();
        value.write(&mut w).unwrap();
        assert_eq!(w.bytes.len(), 95);
        let mut r = Reader::new(&w.bytes, Limits::default()).unwrap();
        assert_eq!(Espionage::read(&mut r).unwrap(), value);
        r.finish().unwrap();
        // Two counted vectors contain a13-byte Bonus7 and12-byte Reduction7.
        // Nerf occupies65..69; visibility i32+counterbool precede Adj74..78.
        let mut bytes = 6i32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&w.bytes[4..65]);
        bytes.extend_from_slice(&w.bytes[69..74]);
        bytes.extend_from_slice(&w.bytes[78..]);
        value.wire_version = 6;
        value.i_nerf_all_enemy_spies = 0;
        value.i_counter_spy_adjacent_level_boost = 0;
        assert_eq!(bytes.len(), 87);
        let mut r = Reader::new(
            &bytes,
            Limits {
                max_bytes: bytes.len(),
                max_items: 2,
            },
        )
        .unwrap();
        assert_eq!(Espionage::read(&mut r).unwrap(), value);
        assert_eq!(r.budget, 0);
        r.finish().unwrap();
        assert_eq!(
            value.write(&mut writer()),
            Err(Error::UnsupportedLegacyWrite)
        );
        for end in 0..bytes.len() {
            let mut r = Reader::new(&bytes[..end], Limits::default()).unwrap();
            assert!(Espionage::read(&mut r).is_err(), "prefix {end}");
        }
        let mut r = Reader::new(
            &bytes,
            Limits {
                max_bytes: bytes.len(),
                max_items: 1,
            },
        )
        .unwrap();
        assert_eq!(Espionage::read(&mut r), Err(Error::ResourceLimit));
        let mut bad = bytes.clone();
        bad[8..12].copy_from_slice(&5i32.to_le_bytes());
        assert_eq!(
            Espionage::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::Version {
                expected: 7,
                actual: 5
            })
        );
        bad = bytes.clone();
        bad[16] = 2;
        assert_eq!(
            Espionage::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::NoncanonicalBool(2))
        );
        bad[..4].copy_from_slice(&5i32.to_le_bytes());
        assert_eq!(
            Espionage::read(&mut Reader::new(&bad, Limits::default()).unwrap()),
            Err(Error::Version {
                expected: 7,
                actual: 5
            })
        );
    }
    #[test]
    fn independently_tagged_espionage_records6_preserve_fields() {
        let mut value = crate::unit_turn_upkeep::tests::source_only_unit().k_espionage;
        value.a_espionage_bonuses = vec![EspionageBonus {
            wire_version: 7,
            e_operation_type: 0xf1234567,
            b_offensive: true,
            i_amount: i32::MIN,
        }];
        value.a_espionage_time_reductions = vec![EspionageTimeReduction {
            wire_version: 7,
            e_operation_type: 0xfedcba98,
            i_reduction_percent: -257,
        }];
        let mut w = writer();
        value.write(&mut w).unwrap();
        // Each nested tag belongs to its record, independently of outer7.
        w.bytes[8..12].copy_from_slice(&6i32.to_le_bytes());
        w.bytes[25..29].copy_from_slice(&6i32.to_le_bytes());
        value.a_espionage_bonuses[0].wire_version = 6;
        value.a_espionage_time_reductions[0].wire_version = 6;
        let limits = Limits {
            max_bytes: w.bytes.len(),
            max_items: 2,
        };
        let mut r = Reader::new(&w.bytes, limits).unwrap();
        assert_eq!(Espionage::read(&mut r).unwrap(), value);
        r.finish().unwrap();
        assert_eq!(
            value.write(&mut writer()),
            Err(Error::UnsupportedLegacyWrite)
        );
        for n in 0..w.bytes.len() {
            assert!(Espionage::read(&mut Reader::new(&w.bytes[..n], limits).unwrap()).is_err());
        }
        let mut bad = w.bytes.clone();
        bad[25..29].copy_from_slice(&5i32.to_le_bytes());
        assert_eq!(
            Espionage::read(&mut Reader::new(&bad, limits).unwrap()),
            Err(Error::Version {
                expected: 7,
                actual: 5
            })
        );
        bad = w.bytes.clone();
        bad[16] = 2;
        assert_eq!(
            Espionage::read(&mut Reader::new(&bad, limits).unwrap()),
            Err(Error::NoncanonicalBool(2))
        );
        assert_eq!(
            Espionage::read(
                &mut Reader::new(
                    &w.bytes,
                    Limits {
                        max_items: 1,
                        ..limits
                    }
                )
                .unwrap()
            ),
            Err(Error::ResourceLimit)
        );
    }
}
