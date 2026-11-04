//! Bounded Linux XP2 city combat activation and its player-loop join.
//! This is source-derived upkeep, not complete gameplay or native parity.

use crate::player_districts::District;
use crate::unit_turn_start::{
    activate_units_and_formations_suffix, ActivationSuffixContext, ActivationSuffixError,
};

/// Live identities, not enclosing saved-table ownership. `district_city` must
/// resolve the native District.GetCity path; absent and unavailable are distinct.
pub trait CityCombatActivationContext {
    type Error;
    type DistrictHandle: Copy;
    type CityHandle: Copy;
    /// The retained original City/Districts receiver, resolved authoritatively.
    fn city_district_ids(&mut self) -> Result<&[u32], Self::Error>;
    fn invalidated_district_cursor(&mut self, index: usize) -> Self::Error;
    /// Actual City.header.project_production storage, not a temporary.
    fn project_production(&mut self) -> Result<&mut bool, Self::Error>;
    /// Resolve and retain the city owner's actual Player before Begin.
    fn retain_city_owner(&mut self) -> Result<(), Self::Error>;
    fn find_owner_district(&mut self, id: u32)
        -> Result<Option<Self::DistrictHandle>, Self::Error>;
    fn district(&mut self, district: Self::DistrictHandle) -> Result<&District, Self::Error>;
    fn district_mut(
        &mut self,
        district: Self::DistrictHandle,
    ) -> Result<&mut District, Self::Error>;
    /// Exact GetMaxDamage(0x6da56a3d), including definition validity, positive
    /// inner HP gate, linked City combat/building effects. Unknown support fails.
    fn outer_defense_max_damage(
        &mut self,
        district: Self::DistrictHandle,
    ) -> Result<i32, Self::Error>;
    /// Retain actual owner Stats receiver before its parameter query.
    fn retain_district_owner_stats(
        &mut self,
        district: Self::DistrictHandle,
    ) -> Result<(), Self::Error>;
    /// Read the retained Stats receiver; do not resolve another owner here.
    fn district_owner_max_trained_strength(
        &mut self,
        district: Self::DistrictHandle,
    ) -> Result<&i32, Self::Error>;
    fn minimum_city_strike_strength(&mut self) -> Result<i32, Self::Error>;
    fn district_city(
        &mut self,
        district: Self::DistrictHandle,
    ) -> Result<Option<Self::CityHandle>, Self::Error>;
    /// Actual linked City.combat.ranged_strike_strength_modifier storage.
    fn linked_city_ranged_modifier(&mut self, city: Self::CityHandle) -> Result<&i32, Self::Error>;
    /// Actual linked City.header.attacks_per_turn storage.
    fn linked_city_attacks_per_turn(&mut self, city: Self::CityHandle)
        -> Result<&i32, Self::Error>;
    /// Null City during GetAttackStrength is a dereference failure, unlike the
    /// later Restore null branch. Implementations must not manufacture a City.
    fn null_attack_city_error(&mut self) -> Self::Error;
    /// Execute the mandatory Power.DoTurnActivate continuation completely,
    /// preserving synchronous gameplay/RNG effects or returning unsupported.
    fn activate_city_power(&mut self) -> Result<(), Self::Error>;
}

/// CanAttack uses raw Q8 > 0 after *two* wrapping integer-to-Q8 conversions.
/// Do not clamp or compare the unshifted mathematical sum.
pub fn district_attack_strength_q8(max_trained: i32, minimum: i32, city_modifier: i32) -> i32 {
    max_trained
        .max(minimum)
        .wrapping_shl(8)
        .wrapping_add(city_modifier.wrapping_shl(8))
}

/// City.Combat.Restore (0x2871ba0) with its actual District.Combat.Restore
/// (0x291cb60) computation. No healing, damage clearing, reports or RNG in these
/// direct bodies. Required leaf providers remain exact, not blanket UI exclusions.
pub fn restore_city_combat<C: CityCombatActivationContext>(c: &mut C) -> Result<(), C::Error> {
    c.retain_city_owner()?;
    let mut index = 0;
    loop {
        // Native End is refreshed on every visit; duplicate IDs are visited.
        let ids = c.city_district_ids()?;
        if index == ids.len() {
            break;
        }
        let id = ids.get(index).copied();
        let id = id.ok_or_else(|| c.invalidated_district_cursor(index))?;
        if let Some(handle) = c.find_owner_district(id)? {
            // HasWalls reads damage BEFORE computing maximum damage.
            let damage = c.district(handle)?.outer_defenses_damage;
            let maximum = c.outer_defense_max_damage(handle)?;
            if damage < maximum {
                // Stats getter obtains the minimum parameter before reading the
                // retained Stats scalar; keep that order across live queries.
                c.retain_district_owner_stats(handle)?;
                let minimum = c.minimum_city_strike_strength()?;
                let trained = *c.district_owner_max_trained_strength(handle)?;
                let city = c
                    .district_city(handle)?
                    .ok_or_else(|| c.null_attack_city_error())?;
                let modifier = *c.linked_city_ranged_modifier(city)?;
                if district_attack_strength_q8(trained, minimum, modifier) > 0 {
                    // Restore refetches GetCity, rather than retaining the one
                    // used by GetAttackStrength. Null here preserves old count.
                    if let Some(city) = c.district_city(handle)? {
                        let attacks = *c.linked_city_attacks_per_turn(city)?;
                        c.district_mut(handle)?.remaining_attacks = attacks;
                    }
                }
            }
        }
        index += 1;
    }
    Ok(())
}

/// Enter only after native BuildQueue, growth, border expansion, cultural
/// identity and citizen assignment have completed. External policies own any
/// choices in those stages; failure is not permission to skip them.
pub fn finish_city_activation<C: CityCombatActivationContext>(c: &mut C) -> Result<(), C::Error> {
    restore_city_combat(c)?;
    *c.project_production()? = false;
    c.activate_city_power()
}

/// Native player city-loop context. Earlier scoring/player-resource/era,
/// placement and improvement-capture work must already have completed. The city
/// prefix and later maintenance remain explicit required barriers, not defaults.
pub trait PlayerCityActivationContext: ActivationSuffixContext {
    type Cities: Copy;
    type CityCursor;
    type CityHandle: Copy;
    type CityActivation: CityCombatActivationContext<Error = Self::Error>;
    fn transfer_allocation_error(&mut self) -> Self::Error;
    fn player_cities(&mut self) -> Result<Self::Cities, Self::Error>;
    fn cities_begin(&mut self, cities: Self::Cities) -> Result<Self::CityCursor, Self::Error>;
    fn cities_end(&mut self, cities: Self::Cities) -> Result<Self::CityCursor, Self::Error>;
    fn city_cursors_equal(
        &mut self,
        a: &Self::CityCursor,
        b: &Self::CityCursor,
    ) -> Result<bool, Self::Error>;
    fn city_cursor_id(&mut self, cursor: &Self::CityCursor) -> Result<u32, Self::Error>;
    fn find_city(
        &mut self,
        cities: Self::Cities,
        id: u32,
    ) -> Result<Option<Self::CityHandle>, Self::Error>;
    fn city_before_combat(&mut self, city: Self::CityHandle) -> Result<(), Self::Error>;
    fn city_activation(
        &mut self,
        city: Self::CityHandle,
    ) -> Result<&mut Self::CityActivation, Self::Error>;
    fn current_city_id(&mut self, city: Self::CityHandle) -> Result<u32, Self::Error>;
    fn city_transfer_player(&mut self, city: Self::CityHandle) -> Result<i32, Self::Error>;
    fn city_owner(&mut self, city: Self::CityHandle) -> Result<i32, Self::Error>;
    fn increment_city_cursor(&mut self, cursor: &mut Self::CityCursor) -> Result<(), Self::Error>;
    fn transfer_city_cultural_identity(
        &mut self,
        city: Self::CityHandle,
        target: i32,
    ) -> Result<(), Self::Error>;
    /// Native nonbarbarian resource maintenance, resource notifications, then
    /// trade-capacity notification; must retain original resource receiver.
    fn resource_maintenance_and_notifications(&mut self) -> Result<(), Self::Error>;
    fn player_is_barbarian(&mut self) -> Result<bool, Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerCityActivationError<E> {
    CityOrResource(E),
    Units(ActivationSuffixError<E>),
}

/// Actual 0x2ba0d38..0x2ba1027 sequence joined to existing executable unit and
/// formation upkeep. Transfer IDs are gathered after each city, then re-found
/// and current transfer target re-read in a separate pass. A null Find is
/// skipped. Errors preserve earlier effects; this is not an accepted arena turn.
pub fn activate_city_resource_unit_suffix<C: PlayerCityActivationContext>(
    c: &mut C,
    first_activation: bool,
) -> Result<(), PlayerCityActivationError<C::Error>> {
    if !first_activation {
        return Ok(());
    }
    let access = PlayerCityActivationError::CityOrResource;
    let cities = c.player_cities().map_err(access)?;
    let mut cursor = c.cities_begin(cities).map_err(access)?;
    let mut transfers = Vec::new();
    loop {
        let end = c.cities_end(cities).map_err(access)?;
        if c.city_cursors_equal(&cursor, &end).map_err(access)? {
            break;
        }
        let id = c.city_cursor_id(&cursor).map_err(access)?;
        if let Some(city) = c.find_city(cities, id).map_err(access)? {
            c.city_before_combat(city).map_err(access)?;
            finish_city_activation(c.city_activation(city).map_err(access)?).map_err(access)?;
            if c.city_transfer_player(city).map_err(access)? != -1 {
                let id = c.current_city_id(city).map_err(access)?;
                transfers
                    .try_reserve(1)
                    .map_err(|_| access(c.transfer_allocation_error()))?;
                transfers.push(id);
            }
        }
        c.increment_city_cursor(&mut cursor).map_err(access)?;
    }
    for id in transfers {
        if let Some(city) = c.find_city(cities, id).map_err(access)? {
            if c.city_owner(city).map_err(access)? != -1 {
                let target = c.city_transfer_player(city).map_err(access)?;
                c.transfer_city_cultural_identity(city, target)
                    .map_err(access)?;
            }
        }
    }
    if !c.player_is_barbarian().map_err(access)? {
        c.resource_maintenance_and_notifications().map_err(access)?;
    }
    activate_units_and_formations_suffix(c, true).map_err(PlayerCityActivationError::Units)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn attack_strength_preserves_floor_fraction_scale_and_wrapping() {
        assert_eq!(district_attack_strength_q8(5, 10, -9), 256);
        assert_eq!(district_attack_strength_q8(5, 10, -10), 0);
        assert_eq!(district_attack_strength_q8(20, 10, -21), -256);
        assert_eq!(district_attack_strength_q8(0x7fffff, 0, 1), i32::MIN);
        assert_eq!(district_attack_strength_q8(i32::MAX, 0, 1), 0);
    }

    // Explicit synthetic storage and query effects, not an initialized game or
    // original-engine fixture. Only the reached stage fields carry test values.
    struct Context {
        ids: Vec<u32>,
        district: District,
        project: bool,
        maximum: Result<i32, &'static str>,
        trained: i32,
        minimum: i32,
        modifier: i32,
        attacks: i32,
        links: std::collections::VecDeque<Option<u32>>,
        shrink: bool,
        power_error: bool,
        calls: Vec<&'static str>,
        visited: Vec<u32>,
    }
    fn context(ids: Vec<u32>) -> Context {
        Context {
            ids,
            district: District {
                wire_version: 15,
                id: 7,
                location: (2, 3),
                city_id: 9,
                district_hash: 123,
                garrison_damage: 13,
                outer_defenses_damage: 17,
                last_turn_damaged: 4,
                remaining_attacks: 99,
                cost: 50,
                is_complete: true,
                properties: crate::unit::variants::TypedVariantMap::default(),
                yield_changes: vec![],
                base_yield_changes: vec![],
                yield_adjacency_modifiers: vec![],
                extra_regional_range: 0,
                extra_regional_entertainment: 0,
                extra_entertainment: 0,
                extra_regional_yields: vec![],
                tourism_adjacency_yield_modifier: vec![],
                tourism_change: 0,
                yield_adjacency_mirrors: vec![],
                appeal_yield_changes: None,
                great_person_point_changes_q8: vec![],
                great_person_point_multiplier: 0,
                great_person_point_adder: 0,
                air_slots_modifier: 0,
                within_one_hex_espionage_defense_bonus: 0,
                under_siege: false,
                pillaged: false,
            },
            project: true,
            maximum: Ok(100),
            trained: 5,
            minimum: 10,
            modifier: 0,
            attacks: -3,
            links: [].into(),
            shrink: false,
            power_error: false,
            calls: vec![],
            visited: vec![],
        }
    }
    impl CityCombatActivationContext for Context {
        type Error = &'static str;
        type DistrictHandle = u32;
        type CityHandle = u32;
        fn city_district_ids(&mut self) -> Result<&[u32], Self::Error> {
            Ok(&self.ids)
        }
        fn invalidated_district_cursor(&mut self, _: usize) -> Self::Error {
            "invalidated cursor"
        }
        fn project_production(&mut self) -> Result<&mut bool, Self::Error> {
            Ok(&mut self.project)
        }
        fn retain_city_owner(&mut self) -> Result<(), Self::Error> {
            self.calls.push("owner");
            Ok(())
        }
        fn find_owner_district(&mut self, id: u32) -> Result<Option<u32>, Self::Error> {
            self.visited.push(id);
            Ok((id == 7).then_some(id))
        }
        fn district(&mut self, _: u32) -> Result<&District, Self::Error> {
            Ok(&self.district)
        }
        fn district_mut(&mut self, _: u32) -> Result<&mut District, Self::Error> {
            self.calls.push("assign");
            Ok(&mut self.district)
        }
        fn outer_defense_max_damage(&mut self, _: u32) -> Result<i32, Self::Error> {
            self.calls.push("maximum");
            if self.shrink {
                self.ids.clear();
            }
            self.maximum
        }
        fn retain_district_owner_stats(&mut self, _: u32) -> Result<(), Self::Error> {
            self.calls.push("stats");
            Ok(())
        }
        fn district_owner_max_trained_strength(&mut self, _: u32) -> Result<&i32, Self::Error> {
            self.calls.push("trained");
            Ok(&self.trained)
        }
        fn minimum_city_strike_strength(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("minimum");
            Ok(self.minimum)
        }
        fn district_city(&mut self, _: u32) -> Result<Option<u32>, Self::Error> {
            self.calls.push("linked");
            Ok(self.links.pop_front().unwrap_or(Some(9)))
        }
        fn linked_city_ranged_modifier(&mut self, _: u32) -> Result<&i32, Self::Error> {
            Ok(&self.modifier)
        }
        fn linked_city_attacks_per_turn(&mut self, _: u32) -> Result<&i32, Self::Error> {
            Ok(&self.attacks)
        }
        fn null_attack_city_error(&mut self) -> Self::Error {
            "null attack city"
        }
        fn activate_city_power(&mut self) -> Result<(), Self::Error> {
            self.calls.push("power");
            assert!(!self.project);
            if self.power_error {
                Err("power unsupported")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn duplicate_districts_null_find_and_negative_attacks_copy_in_order() {
        let mut c = context(vec![7, 8, 7]);
        let old = c.district.clone();
        finish_city_activation(&mut c).unwrap();
        assert_eq!(c.visited, [7, 8, 7]);
        assert_eq!(c.district.remaining_attacks, -3);
        let mut expected = old;
        expected.remaining_attacks = -3;
        assert_eq!(c.district, expected);
        assert_eq!(
            c.calls,
            [
                "owner", "maximum", "stats", "minimum", "trained", "linked", "linked", "assign",
                "maximum", "stats", "minimum", "trained", "linked", "linked", "assign", "power"
            ]
        );
    }
    #[test]
    fn walls_and_signed_q8_gates_preserve_old_attack_count() {
        for (damage, modifier) in [(100, 0), (17, -10), (17, -11)] {
            let mut c = context(vec![7]);
            c.district.outer_defenses_damage = damage;
            c.modifier = modifier;
            finish_city_activation(&mut c).unwrap();
            assert_eq!(c.district.remaining_attacks, 99);
            assert!(!c.project);
        }
    }
    #[test]
    fn null_link_has_distinct_attack_dereference_and_restore_skip() {
        let mut c = context(vec![7]);
        c.links = [None].into();
        assert_eq!(finish_city_activation(&mut c), Err("null attack city"));
        assert_eq!(c.district.remaining_attacks, 99);
        assert!(c.project);
        let mut c = context(vec![7]);
        c.links = [Some(9), None].into();
        finish_city_activation(&mut c).unwrap();
        assert_eq!(c.district.remaining_attacks, 99);
        assert!(!c.project);
    }
    #[test]
    fn unknown_leaf_and_shrunk_cursor_refuse_without_finishing_stage() {
        let mut c = context(vec![7]);
        c.maximum = Err("maximum unsupported");
        assert_eq!(finish_city_activation(&mut c), Err("maximum unsupported"));
        assert_eq!(c.district.remaining_attacks, 99);
        assert!(c.project);
        let mut c = context(vec![7, 7]);
        c.shrink = true;
        assert_eq!(finish_city_activation(&mut c), Err("invalidated cursor"));
        assert_eq!(c.district.remaining_attacks, -3);
        assert!(c.project);
        assert!(!c.calls.contains(&"power"));
    }
    #[test]
    fn missing_power_keeps_completed_native_preceding_writes() {
        let mut c = context(vec![7]);
        c.power_error = true;
        assert_eq!(finish_city_activation(&mut c), Err("power unsupported"));
        assert_eq!(c.district.remaining_attacks, -3);
        assert!(!c.project);
    }
}
