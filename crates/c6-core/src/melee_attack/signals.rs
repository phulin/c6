//! Game.CombatOccurred and Player.Attacked signal traversal. The caller retains
//! the current Results payload and the same listener owner across reentry.
use crate::unit_moved_signal::{ListenerRing, RegistryError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    GameCombatOccurred,
    PlayerAttacked(i32),
}
pub trait Context {
    type Error;
    fn listeners(&mut self, scope: Scope) -> Result<&mut ListenerRing<u64>, Self::Error>;
    fn invoke(&mut self, scope: Scope, receiver: u64) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Ring(RegistryError),
}
pub fn dispatch<C: Context>(c: &mut C, scope: Scope) -> Result<(), Error<C::Error>> {
    let mut cursor = c
        .listeners(scope)
        .map_err(Error::Context)?
        .cursor_start()
        .map_err(Error::Ring)?;
    loop {
        let receiver = c
            .listeners(scope)
            .map_err(Error::Context)?
            .cursor_route(cursor)
            .map_err(Error::Ring)?;
        if let Some(receiver) = receiver {
            if let Err(error) = c.invoke(scope, receiver) {
                c.listeners(scope)
                    .map_err(Error::Context)?
                    .cursor_release(cursor)
                    .map_err(Error::Ring)?;
                return Err(Error::Context(error));
            }
        }
        match c
            .listeners(scope)
            .map_err(Error::Context)?
            .cursor_next(cursor)
            .map_err(Error::Ring)?
        {
            Some(next) => cursor = next,
            None => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        ring: ListenerRing<u64>,
        seen: Vec<u64>,
        fail: bool,
    }
    impl Context for C {
        type Error = ();
        fn listeners(&mut self, _: Scope) -> Result<&mut ListenerRing<u64>, ()> {
            Ok(&mut self.ring)
        }
        fn invoke(&mut self, _: Scope, receiver: u64) -> Result<(), ()> {
            self.seen.push(receiver);
            if receiver == 1 && self.seen.len() == 1 {
                self.ring.subscribe(3).unwrap();
            }
            if self.fail && receiver == 2 {
                return Err(());
            }
            Ok(())
        }
    }
    #[test]
    fn callback_append_is_seen_and_failure_leaves_ring_reusable() {
        let mut c = C {
            ring: ListenerRing::from_ordered(&[1, 2]).unwrap(),
            seen: vec![],
            fail: true,
        };
        assert!(dispatch(&mut c, Scope::GameCombatOccurred).is_err());
        assert_eq!(c.seen, [1, 2]);
        c.seen.clear();
        c.fail = false;
        dispatch(&mut c, Scope::GameCombatOccurred).unwrap();
        assert_eq!(c.seen, [1, 2, 3, 3]);
    }
}
