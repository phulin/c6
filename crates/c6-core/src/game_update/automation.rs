//! Operations.Manager.UpdateAutomation2da18b0, called for EVERY current unit
//! during Player.DoAutomation. It does not choose new player orders.
pub mod alert;
pub mod diplomacy;
use crate::skip_turn::StartData;

pub trait Context {
    type Error;
    fn reset_timer(&mut self) -> Result<(), Self::Error>;
    fn front_hash(&mut self) -> Result<Option<u32>, Self::Error>;
    fn delayed_death(&mut self) -> Result<bool, Self::Error>;
    fn check_auto_start(&mut self, hash: u32, data: &mut StartData) -> Result<bool, Self::Error>;
    fn activate(&mut self) -> Result<(), Self::Error>;
    fn clear(&mut self) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Empty,
    DelayedDeath,
    Retained,
    Activated,
    Cleared,
}

pub fn update<C: Context>(c: &mut C) -> Result<Outcome, C::Error> {
    c.reset_timer()?;
    let Some(hash) = c.front_hash()? else {
        return Ok(Outcome::Empty);
    };
    if c.delayed_death()? {
        return Ok(Outcome::DelayedDeath);
    }
    let mut data = StartData {
        delete: false,
        action: false,
    };
    if c.check_auto_start(hash, &mut data)? {
        c.activate()?;
        Ok(Outcome::Activated)
    } else if data.delete {
        c.clear()?;
        Ok(Outcome::Cleared)
    } else {
        Ok(Outcome::Retained)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Current {
        timer: i32,
        front: Option<u32>,
        dead: bool,
        start: Result<bool, &'static str>,
        delete: bool,
        trace: Vec<&'static str>,
    }
    impl Context for Current {
        type Error = &'static str;
        fn reset_timer(&mut self) -> Result<(), Self::Error> {
            self.trace.push("timer");
            self.timer = 0;
            Ok(())
        }
        fn front_hash(&mut self) -> Result<Option<u32>, Self::Error> {
            self.trace.push("front");
            Ok(self.front)
        }
        fn delayed_death(&mut self) -> Result<bool, Self::Error> {
            self.trace.push("death");
            Ok(self.dead)
        }
        fn check_auto_start(
            &mut self,
            hash: u32,
            data: &mut StartData,
        ) -> Result<bool, Self::Error> {
            self.trace.push("check");
            assert_eq!(hash, 7);
            assert_eq!(
                *data,
                StartData {
                    delete: false,
                    action: false
                }
            );
            data.delete = self.delete;
            self.front = Some(9);
            self.start
        }
        fn activate(&mut self) -> Result<(), Self::Error> {
            self.trace.push("activate");
            assert_eq!(self.front, Some(9));
            Ok(())
        }
        fn clear(&mut self) -> Result<(), Self::Error> {
            self.trace.push("clear");
            assert_eq!(self.front, Some(9));
            Ok(())
        }
    }
    fn current() -> Current {
        Current {
            timer: 9,
            front: Some(7),
            dead: false,
            start: Ok(true),
            delete: false,
            trace: vec![],
        }
    }
    #[test]
    fn empty_and_death_reset_timer_before_any_handler_resolution() {
        let mut c = current();
        c.front = None;
        assert_eq!(update(&mut c), Ok(Outcome::Empty));
        assert_eq!(c.timer, 0);
        assert_eq!(c.trace, ["timer", "front"]);
        let mut c = current();
        c.dead = true;
        assert_eq!(update(&mut c), Ok(Outcome::DelayedDeath));
        assert_eq!(c.trace, ["timer", "front", "death"]);
    }
    #[test]
    fn auto_start_activates_current_queue_and_true_ignores_delete() {
        let mut c = current();
        c.delete = true;
        assert_eq!(update(&mut c), Ok(Outcome::Activated));
        assert_eq!(c.trace, ["timer", "front", "death", "check", "activate"]);
    }
    #[test]
    fn false_delete_clears_current_queue_and_failure_retains_reset() {
        let mut c = current();
        c.start = Ok(false);
        c.delete = true;
        assert_eq!(update(&mut c), Ok(Outcome::Cleared));
        assert_eq!(c.trace.last(), Some(&"clear"));
        let mut c = current();
        c.start = Err("handler");
        assert_eq!(update(&mut c), Err("handler"));
        assert_eq!(c.timer, 0);
        assert_eq!(c.trace, ["timer", "front", "death", "check"]);
    }
}
