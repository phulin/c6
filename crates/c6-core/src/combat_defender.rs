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
