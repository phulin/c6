//! Dynamic meeting requirements use the shared live requirement/set owner.
//! Only meeting delivery is covered here; turn and war signals have other owners.
use crate::loaded_actions::{
    declare_war::requirements as war, government_actions::requirements::Continuations,
};
use crate::melee_attack::backend::requirements::Runtime;
use c6_core::{
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    HasMet,
    MetXTurnsAgo,
    FriendlyMeeting,
    AtPeace,
}
#[derive(Clone, Copy, Debug)]
pub struct Receiver {
    /// Identity in the existing canonical requirement owner, not a new allocation.
    pub label: u64,
    pub kind: Kind,
    pub context: Option<i32>,
    pub subject: Option<i32>,
    /// PlayerMetXTurnsAgo's signed turn count / PlayerIsAtPeace minimum turns.
    pub minimum_turns: i32,
}
#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Query(war::CanonicalError),
    Unsupported(&'static str),
    Effects(E),
    Changed(Box<sets::Error<Error<E>>>),
}
fn filtered(r: Receiver, first: i32, second: i32) -> bool {
    match r.kind {
        Kind::FriendlyMeeting => r.context == Some(first) || r.context == Some(second),
        Kind::HasMet => {
            (r.context == Some(first) && r.subject == Some(second))
                || (r.context == Some(second) && r.subject == Some(first))
        }
        Kind::MetXTurnsAgo | Kind::AtPeace => [r.context, r.subject]
            .iter()
            .all(|p| *p == Some(first) || *p == Some(second)),
    }
}
fn has_met_result(met: bool) -> i32 {
    if met {
        2
    } else {
        1
    }
}
fn turns_result(met: bool, met_turn: i32, turns: i32, now: i32) -> i32 {
    has_met_result(met && met_turn.wrapping_add(turns) <= now)
}
/// Native Test result before inverse/persistent processing. This reads current
/// gameplay getters; a missing interface has the native failed Test value 1.
pub fn test<E>(state: &LoadedState, r: Receiver) -> Result<i32, Error<E>> {
    let (Some(a), Some(b)) = (r.context, r.subject) else {
        return Ok(1);
    };
    use war::Queries;
    let mut queries = war::CanonicalQueries;
    let mut query = |q| queries.query(state, q).map_err(Error::Query);
    match r.kind {
        Kind::HasMet => Ok(has_met_result(query(war::Query::HasMet(a, b))? != 0)),
        Kind::MetXTurnsAgo => {
            // GetCurrentGameTurn precedes HasMet, even on the failed branch.
            let now = query(war::Query::CurrentTurn)?;
            if query(war::Query::HasMet(a, b))? == 0 {
                return Ok(1);
            }
            let met_turn = state
                .game_relations()
                .map_err(Error::State)?
                .met_turn(a, b)
                .map_err(Error::Relations)?;
            Ok(turns_result(true, met_turn, r.minimum_turns, now))
        }
        Kind::FriendlyMeeting => {
            // Native getter directly indexes the array; it has no HasMet/self guard.
            let index = usize::try_from(b)
                .map_err(|_| Error::Unsupported("friendly meeting subject index"))?;
            let player = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == a)
                .ok_or(Error::Unsupported("current friendly meeting Player"))?;
            let friendly = player
                .diplomacy
                .had_friendly_meeting
                .get(index)
                .ok_or(Error::Unsupported("friendly meeting subject index"))?;
            Ok(i32::from(!*friendly))
        }
        Kind::AtPeace => {
            if query(war::Query::AtWar(a, b))? != 0 || query(war::Query::HasMet(a, b))? == 0 {
                return Ok(1);
            }
            if r.minimum_turns >= 0
                && query(war::Query::CurrentTurn)?
                    .wrapping_sub(query(war::Query::AtWarChangeTurn(a, b))?)
                    < r.minimum_turns
            {
                return Ok(1);
            }
            Ok(0)
        }
    }
}
struct Context<'a, C> {
    state: &'a mut LoadedState,
    runtime: &'a mut Runtime,
    receiver: Receiver,
    continuations: &'a mut C,
}
impl<C: Continuations> changes::Context for Context<'_, C> {
    type Error = Error<C::Error>;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.runtime.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.runtime.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<Self::Error>> {
        if label != self.receiver.label {
            return Err(sets::Error::Context(Error::Unsupported(
                "meeting requirement receiver identity",
            )));
        }
        test(self.state, self.receiver).map_err(sets::Error::Context)
    }
    fn modifier_changed(
        &mut self,
        l: sets::SetListener,
        s: u64,
        o: i32,
        n: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .modifier_changed(self.state, self.runtime, l, s, o, n)
            .map_err(|e| sets::Error::Context(Error::Effects(e)))
    }
    fn destroy_requirement(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .destroy_requirement(self.state, self.runtime, l)
            .map_err(|e| sets::Error::Context(Error::Effects(e)))
    }
    fn destroy_set(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .destroy_set(self.state, self.runtime, l)
            .map_err(|e| sets::Error::Context(Error::Effects(e)))
    }
    fn reset_trigger(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .reset_trigger(self.state, self.runtime, l)
            .map_err(|e| sets::Error::Context(Error::Effects(e)))
    }
}
/// Dispatch in the current signal subscriber order. Import the actual requirement,
/// cached SimpleRequirement, child/set rings and modifier listeners beforehand.
/// All changed/destruction/trigger callbacks continue through the supplied owner.
pub fn changed<C: Continuations>(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    receiver: Receiver,
    first: i32,
    second: i32,
    continuations: &mut C,
) -> Result<(), Error<C::Error>> {
    if !filtered(receiver, first, second) {
        return Ok(());
    }
    let name: &[u8] = match receiver.kind {
        Kind::HasMet => b"REQUIREMENT_PLAYER_HAS_MET",
        Kind::MetXTurnsAgo => b"REQUIREMENT_PLAYER_MET_X_TURNS_AGO",
        Kind::FriendlyMeeting => b"REQUIREMENT_HAD_FRIENDLY_MEETING",
        Kind::AtPeace => b"REQUIREMENT_PLAYER_IS_AT_PEACE",
    };
    if runtime
        .sets()
        .requirement_definition(receiver.label)
        .map(|d| &d.predicate)
        != Some(&sets::Predicate::Unsupported(
            crate::registry::linux_type_hash(name),
        ))
    {
        return Err(Error::Unsupported(
            "meeting requirement definition identity",
        ));
    }
    // Captured initial routes borrow their registration list. A source-derived
    // destructor removes the native handles before base Deactivate; the shared
    // zero-reference inactive record is the same logical unsubscription here.
    if runtime
        .sets()
        .simple_requirement_for_destruction(receiver.label)
        .is_some_and(|r| !r.active)
    {
        return Ok(());
    }
    changes::invalidate(
        &mut Context {
            state,
            runtime,
            receiver,
            continuations,
        },
        receiver.label,
    )
    .map_err(|e| Error::Changed(Box::new(e)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_pair_guards_distinguish_same_endpoint_from_unordered_pair() {
        let r = Receiver {
            label: 1,
            kind: Kind::HasMet,
            context: Some(2),
            subject: Some(4),
            minimum_turns: 0,
        };
        assert!(filtered(r, 2, 4));
        assert!(filtered(r, 4, 2));
        assert!(!filtered(r, 2, 3));
        let same = Receiver {
            subject: Some(2),
            ..r
        };
        assert!(!filtered(same, 2, 4));
        for kind in [Kind::MetXTurnsAgo, Kind::AtPeace] {
            assert!(filtered(Receiver { kind, ..same }, 2, 4));
        }
        assert!(filtered(
            Receiver {
                kind: Kind::FriendlyMeeting,
                subject: None,
                ..r
            },
            4,
            2
        ));
        for kind in [
            Kind::HasMet,
            Kind::MetXTurnsAgo,
            Kind::AtPeace,
            Kind::FriendlyMeeting,
        ] {
            assert!(!filtered(
                Receiver {
                    kind,
                    context: None,
                    ..r
                },
                2,
                4
            ));
        }
    }
    #[test]
    fn native_terminal_success_and_wrapping_turn_addition() {
        assert_eq!(has_met_result(true), 2);
        assert_eq!(has_met_result(false), 1);
        assert_eq!(turns_result(true, 10, 3, 12), 1);
        assert_eq!(turns_result(true, 10, 3, 13), 2);
        assert_eq!(turns_result(false, 10, 3, 99), 1);
        assert_eq!(turns_result(true, i32::MAX, 1, 0), 2);
        // Subtracting met_turn from now would overflow in the opposite direction.
        assert_eq!(turns_result(true, i32::MIN, -1, 0), 1);
    }
}
