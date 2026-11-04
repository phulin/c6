//! Linux Player.Units delayed destruction. A delayed Unit remains in its roster
//! for subsequent combat XP/results; placement and sight are removed separately.
use crate::unit::Unit;

pub trait Context {
    type Error;
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;
    /// Complete current GameSignals+1470 dispatch, after the delayed-death write.
    fn signal_removed(&mut self) -> Result<(), Self::Error>;
    fn formation_unit_count(&self) -> Result<Option<usize>, Self::Error>;
    /// Native RemoveUnit then Player.Formations.Create(unit ID), only for >=2.
    fn split_dying_unit(&mut self) -> Result<(), Self::Error>;
    /// Complete Placement.Remove (membership -> sight -> invalid location -> reports).
    fn remove_placement(&mut self) -> Result<(), Self::Error>;
}

/// Player.Units.DelayedDestroy2ba54b0. Native-prefix errors are retained.
/// Already-delayed units return without dispatching or removing placement again.
pub fn delayed_destroy<C: Context>(c: &mut C) -> Result<(), C::Error> {
    if c.unit()?.b_delayed_death {
        return Ok(());
    }
    c.unit_mut()?.b_delayed_death = true;
    c.signal_removed()?;
    if c.formation_unit_count()?.is_some_and(|count| count >= 2) {
        c.split_dying_unit()?;
    }
    c.remove_placement()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        unit: Unit,
        count: Option<usize>,
        fail_signal: bool,
        calls: Vec<&'static str>,
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn signal_removed(&mut self) -> Result<(), Self::Error> {
            assert!(self.unit.b_delayed_death);
            self.calls.push("signal");
            if self.fail_signal {
                Err("signal consumer")
            } else {
                Ok(())
            }
        }
        fn formation_unit_count(&self) -> Result<Option<usize>, Self::Error> {
            Ok(self.count)
        }
        fn split_dying_unit(&mut self) -> Result<(), Self::Error> {
            self.calls.push("split");
            Ok(())
        }
        fn remove_placement(&mut self) -> Result<(), Self::Error> {
            self.calls.push("placement");
            self.unit.k_location.x = -1;
            self.unit.k_location.y = -1;
            Ok(())
        }
    }
    fn fixture(count: Option<usize>) -> Fixture {
        Fixture {
            unit: crate::unit_turn_upkeep::tests::source_only_unit(),
            count,
            fail_signal: false,
            calls: vec![],
        }
    }
    #[test]
    fn singleton_retains_unit_and_removes_placement_after_signal() {
        let mut f = fixture(Some(1));
        let id = f.unit.id;
        delayed_destroy(&mut f).unwrap();
        assert_eq!(f.unit.id, id);
        assert_eq!(f.calls, ["signal", "placement"]);
        delayed_destroy(&mut f).unwrap();
        assert_eq!(f.calls.len(), 2);
    }
    #[test]
    fn multiple_members_split_before_placement_removal() {
        let mut f = fixture(Some(2));
        delayed_destroy(&mut f).unwrap();
        assert_eq!(f.calls, ["signal", "split", "placement"]);
    }
    #[test]
    fn unsupported_consumer_preserves_flag_and_stops_before_placement() {
        let mut f = fixture(None);
        f.fail_signal = true;
        assert_eq!(delayed_destroy(&mut f), Err("signal consumer"));
        assert!(f.unit.b_delayed_death);
        assert_eq!(f.calls, ["signal"]);
    }
}
