//! Linux GetBestDefender(location,-1,-1,null,0) selection only.
//! The caller supplies CURRENT layer0 order. Other flags/attackers are not implied.

pub trait Context {
    type Error;
    type Unit: Copy;
    /// Native vector End is read initially and after every visited reference.
    fn candidate_count(&mut self) -> Result<usize, Self::Error>;
    /// CManager.EditUnit: unavailable identity/player is absent, not a candidate.
    fn resolve(&mut self, index: usize) -> Result<Option<Self::Unit>, Self::Error>;
    fn delayed_death(&mut self, unit: Self::Unit) -> Result<bool, Self::Error>;
    fn owner(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
    fn combat_false(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
    fn ranged_combat(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
    fn bombard_combat(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
}

/// Unit.IsCombat20dee90 short-circuits in melee, ranged, bombard order.
pub fn is_combat<C: Context>(c: &mut C, unit: C::Unit) -> Result<bool, C::Error> {
    Ok(c.combat_false(unit)? > 0 || c.ranged_combat(unit)? > 0 || c.bombard_combat(unit)? > 0)
}

/// 28c9e70 with null attacker and flags0: no visibility, war, domain, movement,
/// cargo, religious-combat or CanDefend query. Rules.IsBetterDefender2bdfdb0
/// accepts the first eligible unit without an EXTRA GetCombat call. Thereafter
/// it reads candidate GetCombat(false), then incumbent GetCombat(false), each
/// time; signed strict-greater keeps the first unit on ties. No RNG selection.
pub fn select_default<C: Context>(c: &mut C) -> Result<Option<C::Unit>, C::Error> {
    let mut best = None;
    let mut index = 0;
    while index < c.candidate_count()? {
        if let Some(unit) = c.resolve(index)? {
            if !c.delayed_death(unit)? && c.owner(unit)? != -1 && is_combat(c, unit)? {
                let replace = match best {
                    None => true,
                    Some(old) => c.combat_false(unit)? > c.combat_false(old)?,
                };
                if replace {
                    best = Some(unit);
                }
            }
        }
        index += 1;
    }
    Ok(best)
}

/// GetBestDefender flags0x9/0x19 with a nonnull attacker. These profiles examine
/// all eligible units (mask0x8), require enemies (mask0x1), and use actual visibility.
pub trait MeleeContext: Context {
    fn is_enemy(&mut self, candidate_owner: i32, actor_owner: i32) -> Result<bool, Self::Error>;
    fn visible(&mut self, unit: Self::Unit) -> Result<bool, Self::Error>;
    fn domain(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
    fn attacker_religious_strength(&mut self) -> Result<i32, Self::Error>;
    fn religious_strength(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
}
pub fn select_melee<C: MeleeContext>(
    c: &mut C,
    actor_owner: i32,
    exclude_air: bool,
) -> Result<Option<C::Unit>, C::Error> {
    let mut best = None;
    let mut index = 0;
    while index < c.candidate_count()? {
        if let Some(unit) = c.resolve(index)? {
            let mut eligible = false;
            if !c.delayed_death(unit)? && c.owner(unit)? != actor_owner {
                let owner = c.owner(unit)?;
                eligible = c.is_enemy(owner, actor_owner)?
                    && c.visible(unit)?
                    && (!exclude_air || c.domain(unit)? != 1);
            }
            if eligible {
                if c.attacker_religious_strength()? > 0 && c.religious_strength(unit)? > 0 {
                    return Ok(Some(unit));
                }
                let replace = match best {
                    None => true,
                    Some(old) => c.combat_false(unit)? > c.combat_false(old)?,
                };
                if replace {
                    best = Some(unit)
                }
            }
        }
        index += 1;
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Copy)]
    struct Unit {
        owner: i32,
        delayed: bool,
        strengths: [i32; 3],
    }
    struct C {
        units: Vec<Option<Unit>>,
        calls: Vec<(usize, &'static str)>,
        fail: Option<(usize, &'static str)>,
        visible: Vec<bool>,
        domain: Vec<i32>,
        religion: Vec<i32>,
        attacker_religion: i32,
    }
    impl C {
        fn new(strengths: &[[i32; 3]]) -> Self {
            Self {
                units: strengths
                    .iter()
                    .map(|&strengths| {
                        Some(Unit {
                            owner: 0,
                            delayed: false,
                            strengths,
                        })
                    })
                    .collect(),
                calls: vec![],
                fail: None,
                visible: vec![true; strengths.len()],
                domain: vec![2; strengths.len()],
                religion: vec![0; strengths.len()],
                attacker_religion: 0,
            }
        }
        fn query(&mut self, index: usize, name: &'static str) -> Result<Unit, &'static str> {
            self.calls.push((index, name));
            if self.fail == Some((index, name)) {
                return Err("missing getter input");
            }
            Ok(self.units[index].unwrap())
        }
    }
    impl MeleeContext for C {
        fn is_enemy(&mut self, a: i32, _: i32) -> Result<bool, Self::Error> {
            self.calls.push((a as usize, "enemy"));
            Ok(a == 1)
        }
        fn visible(&mut self, u: usize) -> Result<bool, Self::Error> {
            self.query(u, "visible")?;
            Ok(self.visible[u])
        }
        fn domain(&mut self, u: usize) -> Result<i32, Self::Error> {
            self.query(u, "domain")?;
            Ok(self.domain[u])
        }
        fn attacker_religious_strength(&mut self) -> Result<i32, Self::Error> {
            self.calls.push((usize::MAX, "actorReligion"));
            Ok(self.attacker_religion)
        }
        fn religious_strength(&mut self, u: usize) -> Result<i32, Self::Error> {
            self.query(u, "religion")?;
            Ok(self.religion[u])
        }
    }
    #[test]
    fn melee_filters_owner_enemy_visibility_air_before_any_strength_query() {
        let mut c = C::new(&[[0, 0, 0]; 4]);
        for u in &mut c.units {
            u.as_mut().unwrap().owner = 1;
        }
        c.units[0].as_mut().unwrap().owner = 0;
        c.units[1].as_mut().unwrap().owner = 2;
        c.visible[2] = false;
        c.domain[3] = 1;
        assert_eq!(select_melee(&mut c, 0, true), Ok(None));
        assert!(!c.calls.iter().any(|(_, q)| *q == "combat"));
        assert!(!c.calls.contains(&(1, "visible")));
        assert!(!c.calls.contains(&(2, "domain")));
        c.calls.clear();
        c.domain[3] = 2;
        c.fail = Some((3, "combat"));
        assert_eq!(select_melee(&mut c, 0, true), Ok(Some(3))); // mask0x8 has no IsCombat filter
    }
    #[test]
    fn melee_profile9_allows_air_and_keeps_first_tie_without_iscombat_queries() {
        let mut c = C::new(&[[10, 0, 0]; 2]);
        for u in &mut c.units {
            u.as_mut().unwrap().owner = 1;
        }
        c.domain[0] = 1;
        assert_eq!(select_melee(&mut c, 0, false), Ok(Some(0)));
        let combat: Vec<_> = c
            .calls
            .iter()
            .filter(|(_, q)| *q == "combat")
            .copied()
            .collect();
        assert_eq!(combat, [(1, "combat"), (0, "combat")]);
        assert!(!c.calls.iter().any(|(_, q)| *q == "domain"));
        c.calls.clear();
        assert_eq!(select_melee(&mut c, 0, true), Ok(Some(1)));
        assert!(!c.calls.iter().any(|(_, q)| *q == "combat"));
    }
    #[test]
    fn melee_first_religious_match_returns_before_strength_and_later_candidates() {
        let mut c = C::new(&[[0, 0, 0]; 3]);
        for u in &mut c.units {
            u.as_mut().unwrap().owner = 1;
        }
        c.attacker_religion = 5;
        c.religion[1] = 8;
        assert_eq!(select_melee(&mut c, 0, false), Ok(Some(1)));
        assert!(!c.calls.iter().any(|(_, q)| *q == "combat"));
        assert!(!c.calls.contains(&(2, "resolve")));
    }
    impl Context for C {
        type Error = &'static str;
        type Unit = usize;
        fn candidate_count(&mut self) -> Result<usize, Self::Error> {
            self.calls.push((usize::MAX, "end"));
            Ok(self.units.len())
        }
        fn resolve(&mut self, i: usize) -> Result<Option<usize>, Self::Error> {
            self.calls.push((i, "resolve"));
            Ok(self.units[i].map(|_| i))
        }
        fn delayed_death(&mut self, i: usize) -> Result<bool, Self::Error> {
            Ok(self.query(i, "delayed")?.delayed)
        }
        fn owner(&mut self, i: usize) -> Result<i32, Self::Error> {
            Ok(self.query(i, "owner")?.owner)
        }
        fn combat_false(&mut self, i: usize) -> Result<i32, Self::Error> {
            Ok(self.query(i, "combat")?.strengths[0])
        }
        fn ranged_combat(&mut self, i: usize) -> Result<i32, Self::Error> {
            Ok(self.query(i, "ranged")?.strengths[1])
        }
        fn bombard_combat(&mut self, i: usize) -> Result<i32, Self::Error> {
            Ok(self.query(i, "bombard")?.strengths[2])
        }
    }
    #[test]
    fn ties_keep_first_and_comparison_rereads_candidate_then_incumbent() {
        let mut c = C::new(&[[10, 0, 0], [10, 0, 0], [11, 0, 0]]);
        assert_eq!(select_default(&mut c), Ok(Some(2)));
        let combat: Vec<_> = c
            .calls
            .iter()
            .filter(|(_, name)| *name == "combat")
            .copied()
            .collect();
        assert_eq!(
            combat,
            [
                (0, "combat"),
                (1, "combat"),
                (1, "combat"),
                (0, "combat"),
                (2, "combat"),
                (2, "combat"),
                (0, "combat")
            ]
        );
        assert_eq!(c.calls.iter().filter(|(_, name)| *name == "end").count(), 4);
    }
    #[test]
    fn eligibility_uses_all_three_but_comparison_uses_only_melee() {
        let mut c = C::new(&[[0, 100, 0], [1, 0, 0], [0, 0, 30]]);
        assert_eq!(select_default(&mut c), Ok(Some(1)));
        assert!(c.calls.contains(&(0, "ranged")));
        assert!(!c.calls.contains(&(0, "bombard")));
        assert!(!c.calls.contains(&(1, "ranged")));
        assert!(c.calls.contains(&(2, "bombard")));
    }
    #[test]
    fn missing_delayed_and_owner_minus_one_skip_later_getters() {
        let mut c = C::new(&[[1, 0, 0]; 4]);
        c.units[0] = None;
        c.units[1].as_mut().unwrap().delayed = true;
        c.units[2].as_mut().unwrap().owner = -1;
        assert_eq!(select_default(&mut c), Ok(Some(3)));
        assert!(!c.calls.contains(&(0, "delayed")));
        assert!(!c.calls.contains(&(1, "owner")));
        assert!(!c.calls.contains(&(2, "combat")));
    }
    #[test]
    fn unsupported_incumbent_requery_propagates_after_candidate_prefix() {
        let c = C::new(&[[1, 0, 0], [2, 0, 0]]);
        // First eligibility succeeds; make the later incumbent read fail by
        // using a provider that changes availability after its first read.
        struct F(C, usize);
        impl Context for F {
            type Error = &'static str;
            type Unit = usize;
            fn candidate_count(&mut self) -> Result<usize, Self::Error> {
                self.0.candidate_count()
            }
            fn resolve(&mut self, i: usize) -> Result<Option<usize>, Self::Error> {
                self.0.resolve(i)
            }
            fn delayed_death(&mut self, i: usize) -> Result<bool, Self::Error> {
                self.0.delayed_death(i)
            }
            fn owner(&mut self, i: usize) -> Result<i32, Self::Error> {
                self.0.owner(i)
            }
            fn combat_false(&mut self, i: usize) -> Result<i32, Self::Error> {
                if i == 0 {
                    self.1 += 1;
                    if self.1 == 2 {
                        self.0.fail = Some((0, "combat"));
                    }
                }
                self.0.combat_false(i)
            }
            fn ranged_combat(&mut self, i: usize) -> Result<i32, Self::Error> {
                self.0.ranged_combat(i)
            }
            fn bombard_combat(&mut self, i: usize) -> Result<i32, Self::Error> {
                self.0.bombard_combat(i)
            }
        }
        let mut f = F(c, 0);
        assert_eq!(select_default(&mut f), Err("missing getter input"));
        assert_eq!(f.0.calls.last(), Some(&(0, "combat")));
    }
}
