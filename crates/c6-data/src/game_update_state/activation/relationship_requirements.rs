//! Live relationship predicates on the PlayerTurnStarted signal.
//! SimpleRequirement state/Changed delivery belongs to the shared requirement
//! owner; these functions compute source Tests and preserve event gates.
use crate::{resource_binding::ResourceAuthority, unit_initialization_definitions, Rules};
use c6_core::{loaded_components::RetainedPlayer, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Relations(c6_core::game_diplomatic_relations::Error),
    Definitions(unit_initialization_definitions::Error),
    Placement(crate::movement_placement_state::Error),
    Missing(&'static str),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    TurnStarted,
    Ally,
    DeclaredFriend,
    FriendlyCityOccupied,
    NearCultureBorder { lookback: i32, threshold: i32 },
    MetXTurns { turns: i32 },
    AtPeace { turns: i32 },
    OpenBorders { grace_turns: i32 },
}
#[derive(Clone, Copy, Debug)]
pub struct Receiver {
    pub kind: Kind,
    pub context: i32,
    pub subject: i32,
}
pub struct Definitions {
    units: crate::registry::LinuxDefinitionRegistry,
}
impl Definitions {
    pub fn from_rules(rules: &Rules, authority: &ResourceAuthority<'_>) -> Result<Self, Error> {
        unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(Error::Definitions)?;
        Ok(Self {
            units: unit_initialization_definitions::collection(rules, "Units", "UnitType")
                .map_err(Error::Definitions)?,
        })
    }
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state.world().player(owner).map_err(Error::World)?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current requirement player"))
}
/// Callback gates precede SimpleRequirement's active guard and virtual Test.
/// In particular MetX does not query HasMet for another reported owner.
pub fn should_invalidate(state: &LoadedState, r: Receiver, reported: i32) -> Result<bool, Error> {
    event_gate(r, reported, || {
        state
            .game_relations()
            .map_err(Error::State)?
            .has_met(r.context, r.subject)
            .map_err(Error::Relations)
    })
}
fn event_gate(
    r: Receiver,
    reported: i32,
    has_met: impl FnOnce() -> Result<bool, Error>,
) -> Result<bool, Error> {
    match r.kind {
        Kind::TurnStarted | Kind::AtPeace { .. } => Ok(r.context == reported),
        Kind::Ally
        | Kind::DeclaredFriend
        | Kind::FriendlyCityOccupied
        | Kind::NearCultureBorder { .. } => Ok(r.subject == reported),
        Kind::MetXTurns { .. } => {
            if r.context != reported {
                return Ok(false);
            }
            has_met()
        }
        Kind::OpenBorders { .. } => Ok(true),
    }
}
/// Actual native result convention: 0 satisfied, 1 unsatisfied, 3 invalid.
/// Nonempty border armies and positive occupation/promise changes remain
/// explicit until their gameplay continuation is supplied.
pub fn test(
    state: &LoadedState,
    defs: &Definitions,
    alive_majors: &[i32],
    r: Receiver,
) -> Result<i32, Error> {
    let context = player(state, r.context)?;
    let subject = player(state, r.subject)?;
    match r.kind {
        Kind::TurnStarted => Ok(0),
        Kind::Ally => Ok(i32::from(
            !state
                .game_relations()
                .map_err(Error::State)?
                .is_allied(r.subject, r.context)
                .map_err(Error::Relations)?,
        )),
        Kind::DeclaredFriend => Ok(i32::from(
            !state
                .game_relations()
                .map_err(Error::State)?
                .has_declared_friendship(r.subject, r.context)
                .map_err(Error::Relations)?,
        )),
        Kind::FriendlyCityOccupied => {
            let relations = state.game_relations().map_err(Error::State)?;
            for &major in alive_majors {
                player(state, major)?;
                if (relations
                    .has_declared_friendship(r.context, major)
                    .map_err(Error::Relations)?
                    || relations
                        .is_allied(r.context, major)
                        .map_err(Error::Relations)?)
                    && !subject.cities.records.entries.is_empty()
                {
                    return Err(Error::Missing("current original-city occupation test"));
                }
            }
            Ok(1)
        }
        Kind::NearCultureBorder {
            lookback,
            threshold,
        } => {
            if r.context == r.subject {
                return Ok(3);
            }
            let _since = state
                .retained()
                .core
                .current_turn
                .wrapping_sub(lookback)
                .max(0);
            if let Some((_, types)) = context
                .diplomacy
                .incursions_made
                .iter()
                .find(|(owner, _)| *owner == r.subject)
            {
                if let Some((_, turns)) = types.entries.iter().find(|(kind, _)| *kind == 0x626c7a34)
                {
                    if !turns.is_empty() {
                        return Err(Error::Missing("current border incursion filtering"));
                    }
                }
            }
            let army = &context
                .ai_targets
                .as_ref()
                .ok_or(Error::Missing("current border army knowledge"))?
                .knowledge_base
                .player_armies
                .0
                .get(
                    usize::try_from(r.subject)
                        .map_err(|_| Error::Missing("border subject index"))?,
                )
                .ok_or(Error::Missing("border subject index"))?;
            for group in army
                .army_size
                .value
                .as_ref()
                .ok_or(Error::Missing("current border army contents"))?
            {
                if defs.units.find_index(group.hash).is_none() {
                    return Err(Error::Missing("current army Unit definition"));
                }
                if !group.value.is_empty() {
                    return Err(Error::Missing("current border army neighborhood traversal"));
                }
            }
            if threshold <= 0 {
                return Err(Error::Missing("current border promise update"));
            }
            Ok(1)
        }
        Kind::OpenBorders { grace_turns } => {
            if crate::movement_placement_state::player_open_borders(state, r.context, r.subject)
                .map_err(Error::Placement)?
            {
                return Ok(0);
            }
            if grace_turns > 0 {
                let ended = *context
                    .diplomacy
                    .open_borders_ended_turn
                    .get(
                        usize::try_from(r.subject)
                            .map_err(|_| Error::Missing("open border subject index"))?,
                    )
                    .ok_or(Error::Missing("open border ending turn"))?;
                // GetTurnsSinceOpenBordersEnded returns -1 for a negative ending turn
                // before subtracting from the current turn.
                let elapsed = if ended < 0 {
                    -1
                } else {
                    state.retained().core.current_turn.wrapping_sub(ended)
                };
                if elapsed >= 0 && elapsed < grace_turns {
                    return Ok(0);
                }
            }
            Ok(1)
        }
        Kind::MetXTurns { .. } | Kind::AtPeace { .. } => {
            Err(Error::Missing("current elapsed diplomacy predicate"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subject_and_context_callbacks_have_distinct_native_filters() {
        let mut r = Receiver {
            kind: Kind::Ally,
            context: 0,
            subject: 1,
        };
        assert!(matches!(
            event_gate(r, 0, || panic!("unexpected contact query")),
            Ok(false)
        ));
        assert!(matches!(
            event_gate(r, 1, || panic!("unexpected contact query")),
            Ok(true)
        ));
        r.kind = Kind::TurnStarted;
        assert!(matches!(
            event_gate(r, 0, || panic!("unexpected contact query")),
            Ok(true)
        ));
        assert!(matches!(
            event_gate(r, 1, || panic!("unexpected contact query")),
            Ok(false)
        ));
    }
    #[test]
    fn elapsed_contact_callback_does_not_resolve_unreached_contact_storage() {
        let r = Receiver {
            kind: Kind::MetXTurns { turns: 10 },
            context: 0,
            subject: 1,
        };
        assert!(matches!(
            event_gate(r, 1, || panic!("other player cannot reach HasMet")),
            Ok(false)
        ));
        assert!(matches!(
            event_gate(r, 0, || Err(Error::Missing("contact"))),
            Err(Error::Missing("contact"))
        ));
    }
}

#[derive(Debug)]
pub enum DispatchError<E> {
    Binding,
    Missing(u64),
    Predicate(Error),
    Change(super::requirement_changes::Error<E>),
}
#[derive(Clone, Copy, Debug)]
pub struct Invocation {
    pub receiver: u64,
    pub descriptor: Receiver,
    pub reported: i32,
}
/// One actual signal subscriber. The shared owner supplies the first active
/// read and performs the second read, inverse/trigger store and Changed fanout.
pub fn dispatch<S: super::requirement_changes::Services>(
    state: &mut LoadedState,
    defs: &Definitions,
    alive_majors: &[i32],
    runtime: &mut super::requirement_changes::Runtime,
    backend: &mut S,
    invocation: Invocation,
) -> Result<(), DispatchError<S::Error>> {
    let Invocation {
        receiver,
        descriptor,
        reported,
    } = invocation;
    if runtime.binding() != state.binding() {
        return Err(DispatchError::Binding);
    }
    if !should_invalidate(state, descriptor, reported).map_err(DispatchError::Predicate)? {
        return Ok(());
    }
    if !runtime
        .state(receiver)
        .ok_or(DispatchError::Missing(receiver))?
        .active
    {
        return Ok(());
    }
    let tested = test(state, defs, alive_majors, descriptor).map_err(DispatchError::Predicate)?;
    runtime
        .set_state(state, backend, receiver, tested, true)
        .map_err(DispatchError::Change)
}
