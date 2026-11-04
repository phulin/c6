//! Linux SimpleRequirement invalidation/SetState, shared by concrete predicates.
use crate::city_power_activation::stock_signals::inverse_state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    pub active: bool,
    pub state: i32,
    pub inverse: bool,
    pub persistent: bool,
    pub triggered: bool,
}

pub trait Context {
    type Error;
    fn active(&mut self) -> Result<bool, Self::Error>;
    fn requirement(&mut self) -> Result<&mut Requirement, Self::Error>;
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Self::Error>;
}
pub trait TestContext: Context {
    fn test(&mut self) -> Result<i32, Self::Error>;
}

/// 0x233b060: IsTriggered, distinct from IsPersistent, promotes satisfied to4.
/// Assignment precedes notification; repeated4 still notifies. Active is reread
/// after virtual Test by this function, including effects of synchronous queries.
pub fn set_state<C: Context>(c: &mut C, tested: i32, notify: bool) -> Result<(), C::Error> {
    if !c.active()? {
        return Ok(());
    }
    let (old, new) = {
        let r = c.requirement()?;
        let value = if r.inverse {
            inverse_state(tested)
        } else {
            tested
        };
        (r.state, if r.triggered && value == 0 { 4 } else { value })
    };
    c.requirement()?.state = new;
    if notify && (old != new || new == 4) {
        c.changed(old, new)?;
    }
    Ok(())
}

/// 0x233a290 -> 0x233ad40: first active guard, virtual Test, second active guard.
pub fn invalidate<C: TestContext>(c: &mut C) -> Result<(), C::Error> {
    if !c.active()? {
        return Ok(());
    }
    let tested = c.test()?;
    set_state(c, tested, true)
}
