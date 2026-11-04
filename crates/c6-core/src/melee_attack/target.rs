//! Rules.Combat.CanAttackTarget(Unit,Unit,MELEE), source2be0340.
//! Other CombatTypes do not share this branch and are not inferred here.
pub trait Context {
    type Error;
    fn attack_after_moving(&mut self) -> Result<bool, Self::Error>;
    fn has_moved(&mut self) -> Result<bool, Self::Error>;
    fn attacker_religious_strength(&mut self) -> Result<i32, Self::Error>;
    fn is_combat(&mut self) -> Result<bool, Self::Error>;
    fn can_target_air(&mut self) -> Result<bool, Self::Error>;
    fn defender_domain(&mut self) -> Result<i32, Self::Error>;
    fn ranged(&mut self) -> Result<i32, Self::Error>;
    fn combat_false(&mut self) -> Result<i32, Self::Error>;
    fn bombard(&mut self) -> Result<i32, Self::Error>;
    fn attacker_domain(&mut self) -> Result<i32, Self::Error>;
    fn defender_embarked(&mut self) -> Result<bool, Self::Error>;
    fn can_attack_player(&mut self) -> Result<bool, Self::Error>;
}
pub fn can_attack_unit<C: Context>(c: &mut C) -> Result<bool, C::Error> {
    if !c.attack_after_moving()? && c.has_moved()? {
        return Ok(false);
    }
    let _ = c.attacker_religious_strength()?; // still read for nonreligious combat
    if !c.is_combat()? {
        return Ok(false);
    }
    if !c.can_target_air()? && c.defender_domain()? == 1 {
        return Ok(false);
    }
    let ranged = c.ranged()?;
    let combat = c.combat_false()?;
    if ranged >= combat {
        return Ok(false);
    }
    let bombard = c.bombard()?;
    let combat = c.combat_false()?;
    if bombard >= combat {
        return Ok(false);
    }
    if c.attacker_domain()? == 2 && c.defender_embarked()? {
        return Ok(false);
    }
    let domain = c.attacker_domain()?;
    if domain != c.defender_domain()? && !c.defender_embarked()? {
        return Ok(false);
    }
    c.can_attack_player()
}

/// Separate Unit/District MELEE overload2bd91a0 -> player/District2be0240.
/// It deliberately has different strength/domain rules from the Unit overload.
pub trait DistrictContext {
    type Error;
    fn is_combat(&mut self) -> Result<bool, Self::Error>;
    fn attacker_owner(&mut self) -> Result<i32, Self::Error>;
    fn district_owner(&mut self) -> Result<i32, Self::Error>;
    fn embarked(&mut self) -> Result<bool, Self::Error>;
    fn fight_while_embarked(&mut self) -> Result<bool, Self::Error>;
    fn domain(&mut self) -> Result<i32, Self::Error>;
    fn attack_after_moving(&mut self) -> Result<bool, Self::Error>;
    fn has_moved(&mut self) -> Result<bool, Self::Error>;
    fn district_defensible(&mut self) -> Result<bool, Self::Error>;
    fn can_attack_player(&mut self, a: i32, b: i32) -> Result<bool, Self::Error>;
}
pub fn can_attack_district<C: DistrictContext>(c: &mut C) -> Result<bool, C::Error> {
    if !c.is_combat()? {
        return Ok(false);
    }
    let owner = c.attacker_owner()?;
    if owner == c.district_owner()? {
        return Ok(false);
    }
    if c.embarked()? && !c.fight_while_embarked()? {
        let _ = c.domain()?; // Native getter precedes MELEE enum guard, which permits it.
    }
    if !c.attack_after_moving()? && c.has_moved()? {
        return Ok(false);
    }
    let owner = c.attacker_owner()?;
    if owner == -1 || !c.district_defensible()? {
        return Ok(false);
    }
    let other = c.district_owner()?;
    c.can_attack_player(owner, other)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        after: bool,
        moved: bool,
        air: bool,
        embarked: bool,
        domain: i32,
        other_domain: i32,
        ranged: i32,
        melee: [i32; 2],
        melee_index: usize,
        bombard: i32,
        calls: Vec<&'static str>,
    }
    impl Default for C {
        fn default() -> Self {
            Self {
                after: true,
                moved: false,
                air: false,
                embarked: false,
                domain: 2,
                other_domain: 2,
                ranged: 0,
                melee: [10, 10],
                melee_index: 0,
                bombard: 0,
                calls: vec![],
            }
        }
    }
    impl C {
        fn read<T>(&mut self, name: &'static str, value: T) -> Result<T, &'static str> {
            self.calls.push(name);
            Ok(value)
        }
    }
    impl Context for C {
        type Error = &'static str;
        fn attack_after_moving(&mut self) -> Result<bool, Self::Error> {
            self.read("after", self.after)
        }
        fn has_moved(&mut self) -> Result<bool, Self::Error> {
            self.read("moved", self.moved)
        }
        fn attacker_religious_strength(&mut self) -> Result<i32, Self::Error> {
            self.read("religion", 0)
        }
        fn is_combat(&mut self) -> Result<bool, Self::Error> {
            self.read("combat?", true)
        }
        fn can_target_air(&mut self) -> Result<bool, Self::Error> {
            self.read("targetAir", self.air)
        }
        fn defender_domain(&mut self) -> Result<i32, Self::Error> {
            self.read("otherDomain", self.other_domain)
        }
        fn ranged(&mut self) -> Result<i32, Self::Error> {
            self.read("ranged", self.ranged)
        }
        fn combat_false(&mut self) -> Result<i32, Self::Error> {
            let v = self.melee[self.melee_index];
            self.melee_index += 1;
            self.read("melee", v)
        }
        fn bombard(&mut self) -> Result<i32, Self::Error> {
            self.read("bombard", self.bombard)
        }
        fn attacker_domain(&mut self) -> Result<i32, Self::Error> {
            self.read("domain", self.domain)
        }
        fn defender_embarked(&mut self) -> Result<bool, Self::Error> {
            self.read("embarked", self.embarked)
        }
        fn can_attack_player(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("war");
            Err("unknown diplomacy")
        }
    }
    #[test]
    fn ordinary_path_reaches_diplomacy_last_and_does_not_cache_melee_or_domains() {
        let mut c = C::default();
        assert_eq!(can_attack_unit(&mut c), Err("unknown diplomacy"));
        assert_eq!(
            c.calls,
            [
                "after",
                "religion",
                "combat?",
                "targetAir",
                "otherDomain",
                "ranged",
                "melee",
                "bombard",
                "melee",
                "domain",
                "embarked",
                "domain",
                "otherDomain",
                "war"
            ]
        );
    }
    #[test]
    fn moved_denial_and_air_denial_skip_later_strength_and_diplomacy() {
        let mut c = C {
            after: false,
            moved: true,
            ..C::default()
        };
        assert_eq!(can_attack_unit(&mut c), Ok(false));
        assert_eq!(c.calls, ["after", "moved"]);
        let mut c = C {
            other_domain: 1,
            ..C::default()
        };
        assert_eq!(can_attack_unit(&mut c), Ok(false));
        assert_eq!(
            c.calls,
            ["after", "religion", "combat?", "targetAir", "otherDomain"]
        );
    }
    #[test]
    fn strict_melee_comparisons_and_land_embarked_rejection_are_distinct() {
        let mut c = C {
            ranged: 10,
            ..C::default()
        };
        assert_eq!(can_attack_unit(&mut c), Ok(false));
        assert!(!c.calls.contains(&"bombard"));
        let mut c = C {
            melee: [10, 5],
            bombard: 6,
            ..C::default()
        };
        assert_eq!(can_attack_unit(&mut c), Ok(false));
        assert!(!c.calls.contains(&"domain"));
        let mut c = C {
            embarked: true,
            ..C::default()
        };
        assert_eq!(can_attack_unit(&mut c), Ok(false));
        assert_eq!(c.calls.last(), Some(&"embarked"));
        let mut c = C {
            domain: 0,
            other_domain: 2,
            embarked: true,
            ..C::default()
        };
        assert_eq!(can_attack_unit(&mut c), Err("unknown diplomacy"));
    }
    #[test]
    fn district_melee_keeps_embarked_domain_read_and_rechecks_actual_owner_defensibility() {
        struct D {
            calls: Vec<&'static str>,
        }
        impl DistrictContext for D {
            type Error = &'static str;
            fn is_combat(&mut self) -> Result<bool, Self::Error> {
                self.calls.push("combat?");
                Ok(true)
            }
            fn attacker_owner(&mut self) -> Result<i32, Self::Error> {
                self.calls.push("owner");
                Ok(12)
            }
            fn district_owner(&mut self) -> Result<i32, Self::Error> {
                self.calls.push("plotOwner");
                Ok(62)
            }
            fn embarked(&mut self) -> Result<bool, Self::Error> {
                self.calls.push("embarked");
                Ok(true)
            }
            fn fight_while_embarked(&mut self) -> Result<bool, Self::Error> {
                self.calls.push("fight");
                Ok(false)
            }
            fn domain(&mut self) -> Result<i32, Self::Error> {
                self.calls.push("domain");
                Ok(2)
            }
            fn attack_after_moving(&mut self) -> Result<bool, Self::Error> {
                self.calls.push("after");
                Ok(true)
            }
            fn has_moved(&mut self) -> Result<bool, Self::Error> {
                panic!("short circuit")
            }
            fn district_defensible(&mut self) -> Result<bool, Self::Error> {
                self.calls.push("defensible");
                Ok(true)
            }
            fn can_attack_player(&mut self, a: i32, b: i32) -> Result<bool, Self::Error> {
                self.calls.push("war");
                assert_eq!((a, b), (12, 62));
                Ok(true)
            }
        }
        let mut d = D { calls: vec![] };
        assert_eq!(can_attack_district(&mut d), Ok(true));
        assert_eq!(
            d.calls,
            [
                "combat?",
                "owner",
                "plotOwner",
                "embarked",
                "fight",
                "domain",
                "after",
                "owner",
                "defensible",
                "plotOwner",
                "war"
            ]
        );
    }
}
