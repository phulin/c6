//! Linux TestAll and synchronous SimpleRequirementSet state-change ordering.
//! Child identities and states are current runtime inputs, not native Test answers.

#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    Context(E),
    Allocation,
}

pub trait Context {
    type Error;
    fn children(&self) -> Result<&[u64], Self::Error>;
    fn child_state(&self, child: u64) -> Result<i32, Self::Error>;
    fn child_persistent(&self, child: u64) -> Result<bool, Self::Error>;
    fn remove_child(&mut self, child: u64) -> Result<(), Self::Error>;
    fn remove_all_children(&mut self) -> Result<(), Self::Error>;
    fn state(&self) -> Result<i32, Self::Error>;
    fn store_state(&mut self, state: i32) -> Result<(), Self::Error>;
    /// Dispatch actual current set listeners, after the state assignment.
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Self::Error>;
    /// Actual SimpleRequirementSet.ResetTrigger continuation, required if reached.
    fn reset_trigger(&mut self) -> Result<(), Self::Error>;
}

/// 0x2337850. State3 removes all children immediately; state2 collects only
/// nonpersistent children and removes them after the complete second scan.
pub fn test_all<C: Context>(c: &mut C) -> Result<i32, Error<C::Error>> {
    let count = c.children().map_err(Error::Context)?.len();
    let mut result = 0;
    let mut has_two = false;
    for index in 0..count {
        let child = c.children().map_err(Error::Context)?[index];
        match c.child_state(child).map_err(Error::Context)? {
            1 => result = 1,
            2 => has_two = true,
            3 => {
                c.remove_all_children().map_err(Error::Context)?;
                return Ok(3);
            }
            _ => {}
        }
    }
    if has_two {
        let count = c.children().map_err(Error::Context)?.len();
        let mut removed = Vec::new();
        removed.try_reserve(count).map_err(|_| Error::Allocation)?;
        for index in 0..count {
            let child = c.children().map_err(Error::Context)?[index];
            if c.child_state(child).map_err(Error::Context)? == 2
                && !c.child_persistent(child).map_err(Error::Context)?
            {
                removed.push(child);
            }
        }
        for child in removed {
            c.remove_child(child).map_err(Error::Context)?;
        }
        if c.children().map_err(Error::Context)?.is_empty() {
            result = 2;
        }
    }
    Ok(result)
}

/// 0x2337000 stores before dispatch and emits only if the state changed.
pub fn set_state<C: Context>(c: &mut C, new: i32, notify: bool) -> Result<(), Error<C::Error>> {
    let old = c.state().map_err(Error::Context)?;
    c.store_state(new).map_err(Error::Context)?;
    if notify && old != new {
        let current = c.state().map_err(Error::Context)?;
        c.changed(old, current).map_err(Error::Context)?;
    }
    Ok(())
}

/// 0x2337510: recompute first. Incoming4 can force a4 notification even when
/// already4; ResetTrigger follows the complete synchronous listener dispatch.
pub fn requirement_changed<C: Context>(c: &mut C, incoming: i32) -> Result<(), Error<C::Error>> {
    let old = c.state().map_err(Error::Context)?;
    let tested = test_all(c)?;
    if incoming == 4 && (tested | 2) != 3 {
        set_state(c, 4, false)?;
        let current = c.state().map_err(Error::Context)?;
        c.changed(old, current).map_err(Error::Context)?;
        c.reset_trigger().map_err(Error::Context)?;
    } else {
        set_state(c, tested, true)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Test {
        children: Vec<u64>,
        states: Vec<(i32, bool)>,
        state: i32,
        events: Vec<(i32, i32)>,
        removed: Vec<u64>,
    }
    impl Context for Test {
        type Error = ();
        fn children(&self) -> Result<&[u64], ()> {
            Ok(&self.children)
        }
        fn child_state(&self, child: u64) -> Result<i32, ()> {
            Ok(self.states[child as usize].0)
        }
        fn child_persistent(&self, child: u64) -> Result<bool, ()> {
            Ok(self.states[child as usize].1)
        }
        fn remove_child(&mut self, child: u64) -> Result<(), ()> {
            self.removed.push(child);
            self.children.retain(|id| *id != child);
            Ok(())
        }
        fn remove_all_children(&mut self) -> Result<(), ()> {
            self.removed.append(&mut self.children);
            Ok(())
        }
        fn state(&self) -> Result<i32, ()> {
            Ok(self.state)
        }
        fn store_state(&mut self, state: i32) -> Result<(), ()> {
            self.state = state;
            Ok(())
        }
        fn changed(&mut self, old: i32, new: i32) -> Result<(), ()> {
            assert_eq!(self.state, new);
            self.events.push((old, new));
            Ok(())
        }
        fn reset_trigger(&mut self) -> Result<(), ()> {
            self.events.push((self.state, -1));
            Ok(())
        }
    }
    fn fixture(states: &[(i32, bool)]) -> Test {
        Test {
            children: (0..states.len() as u64).collect(),
            states: states.to_vec(),
            state: 1,
            events: vec![],
            removed: vec![],
        }
    }
    #[test]
    fn current_single_child_unchanged_skips_listener_then_changed_stores_first() {
        let mut c = fixture(&[(1, false)]);
        requirement_changed(&mut c, 1).unwrap();
        assert!(c.events.is_empty());
        c.states[0].0 = 0;
        requirement_changed(&mut c, 0).unwrap();
        assert_eq!(c.events, [(1, 0)]);
    }
    #[test]
    fn terminal_child_cleanup_and_persistent_two_are_distinct() {
        let mut c = fixture(&[(2, false), (2, true)]);
        assert_eq!(test_all(&mut c), Ok(0));
        assert_eq!(c.removed, [0]);
        assert_eq!(c.children, [1]);
        c.states[1].0 = 3;
        assert_eq!(test_all(&mut c), Ok(3));
        assert!(c.children.is_empty());
        let mut c = fixture(&[(2, false)]);
        assert_eq!(test_all(&mut c), Ok(2));
    }
    #[test]
    fn incoming_trigger_dispatches_equal_four_before_reset() {
        let mut c = fixture(&[(4, false)]);
        c.state = 4;
        requirement_changed(&mut c, 4).unwrap();
        assert_eq!(c.events, [(4, 4), (4, -1)]);
    }
}
