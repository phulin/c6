//! Linux XP2 PlotAdjacentToOwner::Test and SimpleRequirement invalidation.
//! Plot interfaces are resolved from live typed objects by the data adapter;
//! no observed Test answer is accepted. Changed-state consumers are synchronous.
use crate::{
    city_power_activation::stock_signals::inverse_state,
    geometry::plot_valid,
    unit::PlotCoord,
};

/// Distinct definition flags: IsTriggered controls state four; IsPersistent does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    pub active: bool,
    pub state: i32,
    pub inverse: bool,
    pub persistent: bool,
    pub triggered: bool,
}

pub trait PlotContext {
    type Error;
    type Plot: Copy;
    fn distance_bounds(&mut self) -> Result<(i32, i32), Self::Error>;
    fn context_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error>;
    fn subject_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error>;
    fn location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error>;
    fn fixed_location(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn distance(&mut self, a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error>;
}
pub trait Context: PlotContext {
    fn active(&mut self) -> Result<bool, Self::Error>;
    fn requirement(&mut self) -> Result<&mut Requirement, Self::Error>;
    /// Actual synchronous modifier/requirement-set consumers, after assignment.
    /// Errors retain the completed native prefix; no default no-op is supplied.
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Self::Error>;
}

/// 0x238c1d0. Both interfaces and both locations are queried before null/valid
/// branches respectively. Only the second fixed-location query short-circuits.
pub fn test<C: PlotContext>(c: &mut C) -> Result<i32, C::Error> {
    let context = c.context_plot()?;
    let subject = c.subject_plot()?;
    let (Some(context), Some(subject)) = (context, subject) else {
        return Ok(3);
    };
    let a = c.location(context)?;
    let b = c.location(subject)?;
    if !plot_valid(a.x, a.y) || !plot_valid(b.x, b.y) {
        return Ok(
            if c.fixed_location(context)? && c.fixed_location(subject)? {
                3
            } else {
                1
            },
        );
    }
    let distance = c.distance(a, b)?;
    let fixed = c.fixed_location(context)? && c.fixed_location(subject)?;
    let (minimum, maximum) = c.distance_bounds()?;
    let within = distance >= minimum && distance <= maximum;
    Ok(match (within, fixed) {
        (true, false) => 0,
        (false, false) => 1,
        (true, true) => 2,
        (false, true) => 3,
    })
}

/// The scalar portion of SetState(enum,notify), shared with the existing
/// CityIsPowered inverse mapping, with its own definition flags. Active status is checked by callers.
fn next_state(requirement: &Requirement, tested: i32) -> i32 {
    let state = if requirement.inverse {
        inverse_state(tested)
    } else {
        tested
    };
    if requirement.triggered && state == 0 {
        4
    } else {
        state
    }
}

/// 0x233b060. The second active guard occurs after the virtual Test. Equal state
/// four still dispatches; notify=false still assigns but suppresses dispatch.
pub fn set_state<C: Context>(c: &mut C, tested: i32, notify: bool) -> Result<(), C::Error> {
    if !c.active()? {
        return Ok(());
    }
    let (old, new) = {
        let requirement = c.requirement()?;
        (requirement.state, next_state(requirement, tested))
    };
    let signal = notify && (old != new || new == 4);
    c.requirement()?.state = new;
    if signal {
        c.changed(old, new)?
    }
    Ok(())
}

/// Invalidate0x233a290 -> SetState(bool=true)0x233ad40 -> virtual Test ->
/// SetState(enum,true). This callback ignores incoming Unit/old/new arguments.
pub fn invalidate<C: Context>(c: &mut C) -> Result<(), C::Error> {
    if !c.active()? {
        return Ok(());
    }
    let tested = test(c)?;
    set_state(c, tested, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        r: Requirement,
        interfaces: [bool; 2],
        locations: [PlotCoord; 2],
        fixed: [bool; 2],
        range: (i32, i32),
        distance: i32,
        calls: Vec<&'static str>,
        changes: Vec<(i32, i32)>,
        deactivate_on_distance: bool,
        available: bool,
    }
    impl C {
        fn new() -> Self {
            Self {
                r: Requirement {
                    active: true,
                    state: 1,
                    inverse: false,
                    persistent: false,
                    triggered: false,
                },
                interfaces: [true; 2],
                locations: [PlotCoord { x: 1, y: 1 }, PlotCoord { x: 2, y: 1 }],
                fixed: [false; 2],
                range: (0, 1),
                distance: 1,
                calls: vec![],
                changes: vec![],
                deactivate_on_distance: false,
                available: true,
            }
        }
    }
    impl PlotContext for C {
        type Error = &'static str;
        type Plot = usize;
        fn distance_bounds(&mut self) -> Result<(i32, i32), Self::Error> {
            self.calls.push("bounds");
            Ok(self.range)
        }
        fn context_plot(&mut self) -> Result<Option<usize>, Self::Error> {
            self.calls.push("context");
            Ok(self.interfaces[0].then_some(0))
        }
        fn subject_plot(&mut self) -> Result<Option<usize>, Self::Error> {
            self.calls.push("subject");
            Ok(self.interfaces[1].then_some(1))
        }
        fn location(&mut self, p: usize) -> Result<PlotCoord, Self::Error> {
            self.calls.push(if p == 0 {
                "context location"
            } else {
                "subject location"
            });
            Ok(self.locations[p])
        }
        fn fixed_location(&mut self, p: usize) -> Result<bool, Self::Error> {
            self.calls.push(if p == 0 {
                "context fixed"
            } else {
                "subject fixed"
            });
            Ok(self.fixed[p])
        }
        fn distance(&mut self, _: PlotCoord, _: PlotCoord) -> Result<i32, Self::Error> {
            self.calls.push("distance");
            if self.deactivate_on_distance {
                self.r.active = false
            }
            Ok(self.distance)
        }
    }
    impl Context for C {
        fn active(&mut self) -> Result<bool, Self::Error> {
            Ok(self.r.active)
        }
        fn requirement(&mut self) -> Result<&mut Requirement, Self::Error> {
            Ok(&mut self.r)
        }
        fn changed(&mut self, old: i32, new: i32) -> Result<(), Self::Error> {
            if !self.available {
                return Err("unknown consumers");
            }
            assert_eq!(self.r.state, new);
            self.changes.push((old, new));
            Ok(())
        }
    }
    #[test]
    fn null_context_still_queries_subject_but_not_locations() {
        let mut c = C::new();
        c.interfaces[0] = false;
        assert_eq!(test(&mut c), Ok(3));
        assert_eq!(c.calls, ["context", "subject"]);
    }
    #[test]
    fn valid_range_and_fixed_location_states_are_distinct() {
        for (d, fixed, wanted) in [
            (0, [false, false], 0),
            (1, [true, false], 0),
            (2, [true, false], 1),
            (1, [true, true], 2),
            (2, [true, true], 3),
        ] {
            let mut c = C::new();
            c.distance = d;
            c.fixed = fixed;
            assert_eq!(test(&mut c), Ok(wanted));
        }
    }
    #[test]
    fn invalid_coords_skip_distance_bounds_but_fetch_both_locations() {
        let mut c = C::new();
        c.locations[0].x = -9999;
        c.fixed = [true, true];
        assert_eq!(test(&mut c), Ok(3));
        assert_eq!(
            c.calls,
            [
                "context",
                "subject",
                "context location",
                "subject location",
                "context fixed",
                "subject fixed"
            ]
        );
    }
    #[test]
    fn inactive_and_second_active_guard_skip_mutation() {
        let mut c = C::new();
        c.r.active = false;
        invalidate(&mut c).unwrap();
        assert!(c.calls.is_empty());
        c.r.active = true;
        c.deactivate_on_distance = true;
        invalidate(&mut c).unwrap();
        assert_eq!(c.r.state, 1);
        assert!(c.changes.is_empty());
    }
    #[test]
    fn inverse_triggered_equal_four_and_notify_false_follow_native() {
        let mut c = C::new();
        c.r.inverse = true;
        set_state(&mut c, 1, true).unwrap();
        assert_eq!(c.changes, [(1, 0)]);
        c.r.inverse = false;
        c.r.triggered = true;
        set_state(&mut c, 0, true).unwrap();
        set_state(&mut c, 0, true).unwrap();
        assert_eq!(c.changes, [(1, 0), (0, 4), (4, 4)]);
        set_state(&mut c, 1, false).unwrap();
        assert_eq!(c.r.state, 1);
        assert_eq!(c.changes.len(), 3);
    }
    #[test]
    fn persistent_and_triggered_flags_are_independent() {
        for (persistent, triggered, expected) in [(true, false, 0), (false, true, 4)] {
            let mut c = C::new();
            c.r.persistent = persistent;
            c.r.triggered = triggered;
            set_state(&mut c, 0, true).unwrap();
            assert_eq!(c.r.state, expected);
            assert_eq!(c.changes, [(1, expected)]);
        }
    }
    #[test]
    fn unknown_consumers_fail_after_native_state_write() {
        let mut c = C::new();
        c.available = false;
        assert_eq!(invalidate(&mut c), Err("unknown consumers"));
        assert_eq!(c.r.state, 0);
        assert!(c.changes.is_empty());
    }
}
