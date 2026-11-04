//! Stock Alert.Update2d95920. Getters stay ordered and lazy; this never selects
//! a new order or draws policy randomness.
pub trait Context {
    type Error;
    fn maximum(&mut self) -> Result<i32, Self::Error>;
    fn fortifyable(&mut self) -> Result<bool, Self::Error>;
    fn turns(&mut self) -> Result<i32, Self::Error>;
    fn set_turns(&mut self, turns: i32) -> Result<(), Self::Error>;
    fn fortify_activity(&mut self) -> Result<(), Self::Error>;
    fn clear_delete(&mut self);
}
pub fn update<C: Context>(c: &mut C) -> Result<(), C::Error> {
    let max = c.maximum()?;
    if c.fortifyable()? {
        if c.turns()? < max {
            let turns = c.turns()?.wrapping_add(1).max(0);
            c.set_turns(turns)?;
        } else {
            c.fortify_activity()?;
        }
    }
    c.clear_delete();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        log: Vec<&'static str>,
        turns: i32,
        eligible: bool,
        fail: bool,
    }
    impl Context for C {
        type Error = ();
        fn maximum(&mut self) -> Result<i32, ()> {
            self.log.push("max");
            Ok(2)
        }
        fn fortifyable(&mut self) -> Result<bool, ()> {
            self.log.push("eligible");
            Ok(self.eligible)
        }
        fn turns(&mut self) -> Result<i32, ()> {
            self.log.push("turns");
            Ok(self.turns)
        }
        fn set_turns(&mut self, n: i32) -> Result<(), ()> {
            self.log.push("set");
            self.turns = n;
            Ok(())
        }
        fn fortify_activity(&mut self) -> Result<(), ()> {
            self.log.push("activity");
            if self.fail {
                Err(())
            } else {
                Ok(())
            }
        }
        fn clear_delete(&mut self) {
            self.log.push("delete=false");
        }
    }
    #[test]
    fn lazy_queries_and_repeated_current_turn_read() {
        let mut c = C {
            log: vec![],
            turns: 0,
            eligible: false,
            fail: false,
        };
        update(&mut c).unwrap();
        assert_eq!(c.log, ["max", "eligible", "delete=false"]);
        c.log.clear();
        c.eligible = true;
        update(&mut c).unwrap();
        assert_eq!(
            c.log,
            ["max", "eligible", "turns", "turns", "set", "delete=false"]
        );
        assert_eq!(c.turns, 1);
    }
    #[test]
    fn activity_failure_does_not_clear_delete() {
        let mut c = C {
            log: vec![],
            turns: 2,
            eligible: true,
            fail: true,
        };
        assert_eq!(update(&mut c), Err(()));
        assert_eq!(c.log, ["max", "eligible", "turns", "activity"]);
    }
}
