//! Current ordered pending-deal pairs and the reached Player.Diplomacy fallback.
//! This is a query, not deal negotiation/execution. A reached unlocked live
//! DiplomacySession still requires its expiry and statement reconstruction.
use c6_core::loaded_state::{ActionStamp, LoadedState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingDeal {
    pub from: i32,
    pub to: i32,
}

/// Owns semantic current pairs, never a captured HasPendingDeal answer. The
/// producer must authenticate its source epoch before constructing this owner.
/// Future deal mutation must update these same pairs, not a second query cache.
pub struct CurrentPendingDeals {
    instance: ActionStamp,
    ordered: Vec<PendingDeal>,
}
impl CurrentPendingDeals {
    pub fn new(state: &LoadedState, ordered: Vec<PendingDeal>) -> Self {
        Self {
            instance: state.action_stamp(),
            ordered,
        }
    }
    pub fn ordered(&self) -> &[PendingDeal] {
        &self.ordered
    }
    pub fn ordered_mut(&mut self) -> &mut Vec<PendingDeal> {
        &mut self.ordered
    }
}

#[derive(Debug)]
pub enum Error {
    WrongWorld,
    State(c6_core::loaded_state::Error),
    Notifications(c6_core::player_notifications_owner::Error),
    MissingLock(c6_core::player_notifications_owner::ObjectId),
    UnsupportedSession(c6_core::player_notifications_owner::ObjectId),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pending deals: {self:?}")
    }
}
impl std::error::Error for Error {}

fn query<E>(
    pairs: &[PendingDeal],
    from: i32,
    to: i32,
    mut alive: impl FnMut(i32) -> Result<bool, E>,
    mut notification: impl FnMut(i32, i32, u32) -> Result<bool, E>,
) -> Result<bool, E> {
    // Linux 2c757c0: scan actual ordered pairs BEFORE looking up either player.
    for pair in pairs {
        if pair.from == from && (to == -1 || pair.to == to) {
            return Ok(true);
        }
    }
    let (owner, other) = if alive(from)? {
        (from, to)
    } else if alive(to)? {
        (to, from)
    } else {
        return Ok(false);
    };
    if notification(owner, other, 0x00eb1858)? {
        return Ok(true);
    }
    notification(owner, other, 0x03d12c08)
}

/// 2913290 ignores DiplomacyDirection and calls 2c757c0(owner, NO_PLAYER).
/// An early pair match needs no player/notification data. Otherwise reads actual
/// current controls and the canonical notification owner, never wire Diplomacy
/// defaults or an initial cached boolean. Unknown reached session work is an
/// error and this query never mutates gameplay state.
pub fn has_pending_deal(
    state: &LoadedState,
    owner: i32,
    runtime: &CurrentPendingDeals,
) -> Result<bool, Error> {
    if !runtime.instance.same_instance(state.action_stamp()) {
        return Err(Error::WrongWorld);
    }
    query(
        &runtime.ordered,
        owner,
        -1,
        |p| {
            if !(0..64).contains(&p) {
                Ok(false)
            } else {
                state.player_alive(p).map_err(Error::State)
            }
        },
        |p, _, _| {
            let notifications = state.player_notifications().map_err(Error::State)?;
            let mut cursor = notifications.first(p).map_err(Error::Notifications)?;
            while let Some(object) = cursor {
                let record = notifications
                    .record_by_object(object)
                    .map_err(Error::Notifications)?;
                if record.type_hash == 0xe661da67 {
                    // 2aa8e00 tests IsLocked before IsDismissed; missing runtime
                    // cannot be excused by a later dismissed flag.
                    let live = notifications
                        .runtime(object)
                        .map_err(Error::Notifications)?
                        .live
                        .as_ref()
                        .ok_or(Error::MissingLock(object))?;
                    if live.lock_count <= 0 && !record.is_dismissed {
                        // Actual virtual expiry and CreateStatement invoke the
                        // current session/factory, not saved Diplomacy fields.
                        return Err(Error::UnsupportedSession(object));
                    }
                }
                cursor = notifications.next(object).map_err(Error::Notifications)?;
            }
            Ok(false)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pair_match_precedes_missing_players_and_filters_only_from() {
        let pairs = [
            PendingDeal { from: 3, to: 7 },
            PendingDeal { from: 3, to: 9 },
        ];
        assert_eq!(
            query(&pairs, 3, -1, |_| Err("unread"), |_, _, _| Err("unread")),
            Ok(true)
        );
        assert_eq!(
            query(&pairs, 3, 9, |_| Err("unread"), |_, _, _| Err("unread")),
            Ok(true)
        );
        assert_eq!(
            query(&pairs, 7, -1, |_| Ok::<_, ()>(false), |_, _, _| panic!()),
            Ok(false)
        );
    }
    #[test]
    fn alive_from_short_circuits_to_and_queries_actions_in_native_order() {
        let mut calls = Vec::new();
        let value = query(
            &[],
            3,
            9,
            |p| {
                assert_eq!(p, 3);
                Ok::<_, ()>(true)
            },
            |p, o, a| {
                calls.push((p, o, a));
                Ok(a == 0x03d12c08)
            },
        );
        assert_eq!(value, Ok(true));
        assert_eq!(calls, [(3, 9, 0x00eb1858), (3, 9, 0x03d12c08)]);
    }
    #[test]
    fn dead_from_uses_alive_to_and_first_action_short_circuits() {
        let mut alive = Vec::new();
        let mut actions = 0;
        assert_eq!(
            query(
                &[],
                3,
                9,
                |p| {
                    alive.push(p);
                    Ok::<_, ()>(p == 9)
                },
                |p, o, a| {
                    assert_eq!((p, o, a), (9, 3, 0x00eb1858));
                    actions += 1;
                    Ok(true)
                }
            ),
            Ok(true)
        );
        assert_eq!(alive, [3, 9]);
        assert_eq!(actions, 1);
    }
    #[test]
    fn reached_missing_fallback_remains_error() {
        assert_eq!(
            query(
                &[],
                3,
                -1,
                |_| Ok(true),
                |_, _, _| Err("session factory unavailable")
            ),
            Err("session factory unavailable")
        );
        assert_eq!(
            query(
                &[],
                3,
                -1,
                |_| Err("current alive unavailable"),
                |_, _, _| Ok(false)
            ),
            Err("current alive unavailable")
        );
    }
}
