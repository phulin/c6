//! Rules.Appeal.UpdateOnly2bd37f0: calculate, compare current cache, store,
//! synchronous gameplay signal, then reporting. Numeric narrowing is native i16.
pub trait Context {
    type Error;
    type Plot: Copy;
    fn calculate(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn cached(&mut self, plot: Self::Plot) -> Result<i16, Self::Error>;
    fn store(&mut self, plot: Self::Plot, value: i16) -> Result<(), Self::Error>;
    fn changed(&mut self, plot: Self::Plot, value: i32) -> Result<(), Self::Error>;
    fn report(&mut self, plot: Self::Plot) -> Result<(), Self::Error>;
}
/// None follows the original null-plot return. A signal error retains the stored
/// prefix and suppresses the later report; callers must not accept that as done.
pub fn update_only<C: Context>(c: &mut C, plot: Option<C::Plot>) -> Result<bool, C::Error> {
    let Some(plot) = plot else { return Ok(false) };
    let value = c.calculate(plot)?;
    if value == i32::from(c.cached(plot)?) {
        return Ok(false);
    }
    c.store(plot, value as i16)?;
    c.changed(plot, value)?;
    c.report(plot)?;
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    struct World {
        cached: i16,
        calculated: i32,
        during_calculate: Option<i16>,
        fail_signal: bool,
        events: Vec<(char, i32)>,
    }
    impl Context for World {
        type Error = &'static str;
        type Plot = ();
        fn calculate(&mut self, _: ()) -> Result<i32, Self::Error> {
            if let Some(v) = self.during_calculate {
                self.cached = v;
            }
            Ok(self.calculated)
        }
        fn cached(&mut self, _: ()) -> Result<i16, Self::Error> {
            Ok(self.cached)
        }
        fn store(&mut self, _: (), v: i16) -> Result<(), Self::Error> {
            self.cached = v;
            self.events.push(('w', i32::from(v)));
            Ok(())
        }
        fn changed(&mut self, _: (), v: i32) -> Result<(), Self::Error> {
            self.events.push(('s', v));
            if self.fail_signal {
                Err("signal")
            } else {
                self.cached = 9;
                Ok(())
            }
        }
        fn report(&mut self, _: ()) -> Result<(), Self::Error> {
            self.events.push(('r', i32::from(self.cached)));
            Ok(())
        }
    }
    fn world() -> World {
        World {
            cached: 0,
            calculated: 2,
            during_calculate: None,
            fail_signal: false,
            events: vec![],
        }
    }
    #[test]
    fn stores_before_signal_and_report_observes_callback_prefix() {
        let mut w = world();
        assert_eq!(update_only(&mut w, Some(())), Ok(true));
        assert_eq!(w.events, [('w', 2), ('s', 2), ('r', 9)]);
    }
    #[test]
    fn cache_is_read_after_calculation_and_signal_failure_does_not_report() {
        let mut w = world();
        w.during_calculate = Some(2);
        assert_eq!(update_only(&mut w, Some(())), Ok(false));
        assert!(w.events.is_empty());
        w.during_calculate = None;
        w.cached = 0;
        w.fail_signal = true;
        assert_eq!(update_only(&mut w, Some(())), Err("signal"));
        assert_eq!(w.cached, 2);
        assert_eq!(w.events, [('w', 2), ('s', 2)]);
    }
    #[test]
    fn compares_full_result_but_signal_preserves_untruncated_value() {
        let mut w = world();
        w.calculated = 65536;
        assert_eq!(update_only(&mut w, Some(())), Ok(true));
        assert_eq!(w.events, [('w', 0), ('s', 65536), ('r', 9)]);
        assert_eq!(update_only(&mut w, None), Ok(false));
    }
}
