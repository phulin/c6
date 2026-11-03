//! Internal Linux XP2 PlayerTurnComplete prefix, not an accepted EndTurn action.
//!
//! Target a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa.
//! Required live accessors/reporting/archive providers have no no-op defaults.
//! An error may follow internal mutation; Engine must reject before entering
//! until the whole action and its affected state/RNG reach an external boundary.
use crate::city_production::BuildQueue;

pub const NONE: u32 = 0xb6fa1284;
pub const USER_FORCED: u32 = 0x720b6468;
pub const TURN_TIMER_EXPIRED: u32 = 0x4601eac9;
pub const PAUSED_HUMAN_REPORT: u32 = 0xa7b85261;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Unsupported(&'static str),
    ExternalDecisionRequired(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogBranch {
    PausedHuman,
    AlreadyComplete,
}

/// Handles identify actual objects, not cached copies of mutable fields.
/// Implementations must bind the supplied target's actual managers/containers.
/// Fallible services are explicit gaps, never permission to skip native effects.
pub trait World {
    type Player: Copy;
    type Cities: Copy;
    type City: Copy;
    type Cursor: Copy;

    fn resolve_player(&mut self, encoded: i32) -> Result<Self::Player, Error>;
    fn player_id(&mut self, player: Self::Player) -> Result<usize, Error>;
    fn is_human(&mut self, player: Self::Player) -> Result<bool, Error>;
    fn is_alive(&mut self, player: Self::Player) -> Result<bool, Error>;
    fn is_turn_active(&mut self, player: Self::Player) -> Result<bool, Error>;
    fn is_paused(&mut self) -> Result<bool, Error>;
    /// Actual Player::ms_abIsTurnComplete, not a field of a serialized player.
    fn completion_flags(&mut self) -> &mut [bool; 64];
    fn is_valid_slot(&mut self, slot: i32) -> Result<bool, Error>;
    /// Exact logging-options/format/service branch; callbacks cannot be omitted.
    fn log(&mut self, branch: LogBranch, player: Self::Player) -> Result<(), Error>;
    /// Synchronous original reporting event and every registered subscriber.
    fn report(&mut self, event: u32, player: Self::Player) -> Result<(), Error>;
    /// EditTurnManager then its owner->EditCities. Both accessors are pointer
    /// reads in this target; return the actual hook receiver's Cities object.
    fn completion_cities(&mut self, player: Self::Player) -> Result<Self::Cities, Error>;
    fn cities_player(&mut self, cities: Self::Cities) -> Result<Self::Player, Error>;
    fn cities_begin(&mut self, cities: Self::Cities) -> Result<Self::Cursor, Error>;
    fn cities_end(&mut self, cities: Self::Cities) -> Result<Self::Cursor, Error>;
    fn cursor_ne(&mut self, a: Self::Cursor, b: Self::Cursor) -> Result<bool, Error>;
    /// Dereference cursor, GetID, then Cities::Find(id), in that order.
    fn find_cursor_city(
        &mut self,
        cities: Self::Cities,
        cursor: Self::Cursor,
    ) -> Result<Self::City, Error>;
    fn next_city(&mut self, cursor: Self::Cursor) -> Result<Self::Cursor, Error>;
    /// Complete typed queue state; entries must model actual GetQueue/Front.
    fn build_queue(&mut self, city: Self::City) -> Result<&BuildQueue, Error>;
    /// MUST execute actual FAutoVariable<BuildQueue,City>::edit archive/dirty
    /// effects before exposing the typed queue. The wire codec alone lacks them.
    fn edit_build_queue(&mut self, city: Self::City) -> Result<&mut BuildQueue, Error>;
}

/// Original ClearOverflow (0x2850fc0) fixed-point integer-zero assignment.
/// The caller owns City::EditBuildQueue archive/dirty effects separately.
pub fn clear_overflow(queue: &mut BuildQueue) {
    queue.production_overflow = 0;
}

/// 0x2a5b570: original recycled-container order, with End reread after each
/// increment. Do not sort city IDs, snapshot the container, or edit full queues.
pub fn remove_unassigned_production_overflow<W: World>(
    world: &mut W,
    cities: W::Cities,
) -> Result<(), Error> {
    let mut cursor = world.cities_begin(cities)?;
    let mut end = world.cities_end(cities)?;
    while world.cursor_ne(cursor, end)? {
        let city = world.find_cursor_city(cities, cursor)?;
        if world.build_queue(city)?.entries.is_empty() {
            clear_overflow(world.edit_build_queue(city)?);
        }
        cursor = world.next_city(cursor)?;
        end = world.cities_end(cities)?;
    }
    Ok(())
}

/// 0x2ba17b0 -> 0x2a5b640. Even NONE enters the requested player's hook/accessors.
fn completion_hook<W: World>(world: &mut W, player: W::Player, reason: u32) -> Result<(), Error> {
    let cities = world.completion_cities(player)?;
    if reason == USER_FORCED || reason == TURN_TIMER_EXPIRED {
        let owner = world.cities_player(cities)?;
        if world.is_human(owner)? {
            remove_unassigned_production_overflow(world, cities)?;
        }
    }
    Ok(())
}

/// Original SetTurnComplete 0x2b5b840 returns true even for false assignment.
/// Repeated hooks target the SAME requested player once per valid numeric slot;
/// IsAlive is reread after validity on every iteration, including callback changes.
pub fn set_turn_complete<W: World>(
    world: &mut W,
    player: W::Player,
    complete: bool,
    reason: u32,
) -> Result<bool, Error> {
    let id = world.player_id(player)?;
    let flag = world
        .completion_flags()
        .get_mut(id)
        .ok_or(Error::Unsupported(
            "requested player ID outside global completion array",
        ))?;
    *flag = complete;
    if complete {
        for slot in 0..64 {
            if world.is_valid_slot(slot)? && world.is_alive(player)? {
                completion_hook(world, player, reason)?;
            }
        }
    }
    Ok(true)
}

#[derive(Debug, Eq, PartialEq)]
#[must_use = "accepted prefixes require the complete Game.Update continuation"]
pub enum Prefix<P> {
    InvalidEncodedPlayer,
    PausedHuman,
    AlreadyComplete,
    RequiresGameUpdate(RequiresGameUpdate<P>),
}

/// Internal dispatch continuation, never an external decision or native stack
/// token. Cannot be constructed outside this module or cloned for double resume.
#[derive(Debug, Eq, PartialEq)]
#[must_use = "Game.Update is mandatory, including its conditional second call"]
pub struct RequiresGameUpdate<P> {
    player: P,
    first: bool,
}

/// Complete Instance::Update, including processor, pending diplomacy and cache
/// profiling; yielding inside it requires its own evidenced typed continuation.
/// A provider must not return Ok after skipping unfinished upkeep or decisions.
pub trait GameUpdate: World {
    fn game_update(&mut self) -> Result<(), Error>;
}

impl<P: Copy> RequiresGameUpdate<P> {
    /// Run the required update. After the first returns, reread the SAME player
    /// object's active flag; if true require precisely one more whole Update.
    /// None means this dispatch body returned, not completed EndTurn acceptance.
    pub fn run<W: GameUpdate<Player = P>>(self, world: &mut W) -> Result<Option<Self>, Error> {
        world.game_update()?;
        if self.first && world.is_turn_active(self.player)? {
            Ok(Some(Self {
                player: self.player,
                first: false,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Original PlayerTurnComplete::Execute prefix (0x2d27b50). Reason stays raw:
/// unknown values follow the native hook's non-forced path without a whitelist.
pub fn execute_prefix<W: World>(
    world: &mut W,
    encoded_player: i32,
    reason: u32,
) -> Result<Prefix<W::Player>, Error> {
    if encoded_player as u32 > 63 {
        return Ok(Prefix::InvalidEncodedPlayer);
    }
    let player = world.resolve_player(encoded_player)?;
    if world.is_human(player)? && world.is_paused()? {
        world.log(LogBranch::PausedHuman, player)?;
        world.report(PAUSED_HUMAN_REPORT, player)?;
        return Ok(Prefix::PausedHuman);
    }
    let id = world.player_id(player)?;
    let complete = *world.completion_flags().get(id).ok_or(Error::Unsupported(
        "requested player ID outside global completion array",
    ))?;
    if complete {
        world.log(LogBranch::AlreadyComplete, player)?;
        return Ok(Prefix::AlreadyComplete);
    }
    set_turn_complete(world, player, true, reason)?;
    Ok(Prefix::RequiresGameUpdate(RequiresGameUpdate {
        player,
        first: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city_production::{Order, QueueEntry};
    use crate::unit::Limits;

    struct Fixture {
        flags: [bool; 64],
        valid: [bool; 64],
        human: bool,
        alive: bool,
        paused: bool,
        active: bool,
        hooks: usize,
        die_after_hook: bool,
        deactivate_on_update: bool,
        fail_edit: bool,
        append_on_edit: bool,
        queues: Vec<BuildQueue>,
        events: Vec<String>,
    }
    fn queue(overflow: i32) -> BuildQueue {
        let mut bytes = vec![0; 133];
        bytes[..4].copy_from_slice(&16i32.to_le_bytes());
        let mut queue = BuildQueue::decode(&bytes, Limits::default()).unwrap();
        queue.production_overflow = overflow;
        queue.production_salvage = 91;
        queue
    }
    impl Default for Fixture {
        fn default() -> Self {
            let mut valid = [false; 64];
            valid[0] = true;
            valid[63] = true;
            Self {
                flags: [false; 64],
                valid,
                human: true,
                alive: true,
                paused: false,
                active: true,
                hooks: 0,
                die_after_hook: false,
                deactivate_on_update: false,
                fail_edit: false,
                append_on_edit: false,
                queues: vec![queue(i32::MIN), queue(i32::MAX)],
                events: vec![],
            }
        }
    }
    impl World for Fixture {
        type Player = usize;
        type Cities = usize;
        type City = usize;
        type Cursor = usize;
        fn resolve_player(&mut self, id: i32) -> Result<usize, Error> {
            self.events.push(format!("resolve:{id}"));
            Ok(id as usize)
        }
        fn player_id(&mut self, player: usize) -> Result<usize, Error> {
            Ok(player)
        }
        fn is_human(&mut self, _: usize) -> Result<bool, Error> {
            self.events.push("human".into());
            Ok(self.human)
        }
        fn is_alive(&mut self, player: usize) -> Result<bool, Error> {
            self.events.push(format!("alive:{player}"));
            Ok(self.alive)
        }
        fn is_turn_active(&mut self, player: usize) -> Result<bool, Error> {
            self.events.push(format!("active:{player}"));
            Ok(self.active)
        }
        fn is_paused(&mut self) -> Result<bool, Error> {
            self.events.push("paused".into());
            Ok(self.paused)
        }
        fn completion_flags(&mut self) -> &mut [bool; 64] {
            &mut self.flags
        }
        fn is_valid_slot(&mut self, slot: i32) -> Result<bool, Error> {
            self.events.push(format!("valid:{slot}"));
            Ok(self.valid[slot as usize])
        }
        fn log(&mut self, branch: LogBranch, _: usize) -> Result<(), Error> {
            self.events.push(format!("log:{branch:?}"));
            Ok(())
        }
        fn report(&mut self, event: u32, _: usize) -> Result<(), Error> {
            assert_eq!(event, PAUSED_HUMAN_REPORT);
            self.events.push("report".into());
            self.active = false; // Demonstrate synchronous subscriber mutation.
            Ok(())
        }
        fn completion_cities(&mut self, player: usize) -> Result<usize, Error> {
            assert!(self.flags[player]);
            self.events.push(format!("hook:{player}"));
            self.hooks += 1;
            if self.die_after_hook {
                self.alive = false;
            }
            Ok(player)
        }
        fn cities_player(&mut self, cities: usize) -> Result<usize, Error> {
            Ok(cities)
        }
        fn cities_begin(&mut self, _: usize) -> Result<usize, Error> {
            Ok(0)
        }
        fn cities_end(&mut self, _: usize) -> Result<usize, Error> {
            Ok(self.queues.len())
        }
        fn cursor_ne(&mut self, a: usize, b: usize) -> Result<bool, Error> {
            Ok(a != b)
        }
        fn find_cursor_city(&mut self, _: usize, cursor: usize) -> Result<usize, Error> {
            self.events.push(format!("city:{cursor}"));
            Ok(cursor)
        }
        fn next_city(&mut self, cursor: usize) -> Result<usize, Error> {
            Ok(cursor + 1)
        }
        fn build_queue(&mut self, city: usize) -> Result<&BuildQueue, Error> {
            Ok(&self.queues[city])
        }
        fn edit_build_queue(&mut self, city: usize) -> Result<&mut BuildQueue, Error> {
            self.events.push(format!("archive:{city}"));
            if self.fail_edit {
                return Err(Error::Unsupported("archive provider unavailable"));
            }
            if self.append_on_edit {
                self.append_on_edit = false;
                self.queues.push(queue(999));
            }
            Ok(&mut self.queues[city])
        }
    }
    impl GameUpdate for Fixture {
        fn game_update(&mut self) -> Result<(), Error> {
            self.events.push("update".into());
            if self.deactivate_on_update {
                self.active = false;
            }
            Ok(())
        }
    }
    fn token(f: &mut Fixture, reason: u32) -> RequiresGameUpdate<usize> {
        let Prefix::RequiresGameUpdate(token) = execute_prefix(f, 7, reason).unwrap() else {
            panic!("expected mandatory continuation")
        };
        token
    }
    #[test]
    fn encoded_unsigned_boundaries_do_not_resolve_or_mutate() {
        for id in [i32::MIN, -1, 64, i32::MAX] {
            let mut f = Fixture::default();
            assert_eq!(
                execute_prefix(&mut f, id, NONE),
                Ok(Prefix::InvalidEncodedPlayer)
            );
            assert!(f.events.is_empty());
            assert!(!f.flags.iter().any(|v| *v));
        }
        for id in [0, 63] {
            let mut f = Fixture::default();
            assert!(matches!(
                execute_prefix(&mut f, id, NONE),
                Ok(Prefix::RequiresGameUpdate(_))
            ));
            assert!(f.flags[id as usize]);
        }
    }
    #[test]
    fn paused_human_precedes_duplicate_and_runs_reporting() {
        let mut f = Fixture {
            paused: true,
            ..Fixture::default()
        };
        f.flags[7] = true;
        assert_eq!(execute_prefix(&mut f, 7, NONE), Ok(Prefix::PausedHuman));
        assert_eq!(
            f.events,
            ["resolve:7", "human", "paused", "log:PausedHuman", "report"]
        );
        assert!(!f.active);
        assert_eq!(f.hooks, 0);
    }
    #[test]
    fn duplicate_logs_without_reporting_or_update_and_nonhuman_skips_pause_read() {
        let mut f = Fixture {
            human: false,
            paused: true,
            ..Fixture::default()
        };
        f.flags[7] = true;
        assert_eq!(execute_prefix(&mut f, 7, NONE), Ok(Prefix::AlreadyComplete));
        assert_eq!(f.events, ["resolve:7", "human", "log:AlreadyComplete"]);
    }
    #[test]
    fn same_requested_player_hook_repeats_per_valid_slot_and_none_preserves_queues() {
        let mut f = Fixture::default();
        let before = f.queues.clone();
        let _ = token(&mut f, NONE);
        assert_eq!(f.hooks, 2);
        assert_eq!(f.queues, before);
        assert_eq!(
            f.events
                .iter()
                .filter(|s| s.starts_with("hook:"))
                .collect::<Vec<_>>(),
            [&"hook:7", &"hook:7"]
        );
        assert_eq!(
            f.events.iter().filter(|s| s.starts_with("valid:")).count(),
            64
        );
        assert!(!f.events.iter().any(|s| s == "update"));
    }
    #[test]
    fn hook_mutation_is_seen_by_later_alive_checks() {
        let mut f = Fixture {
            die_after_hook: true,
            ..Fixture::default()
        };
        let _ = token(&mut f, USER_FORCED);
        assert_eq!(f.hooks, 1);
        assert_eq!(
            f.events.iter().filter(|s| s.starts_with("alive:")).count(),
            2
        );
    }
    #[test]
    fn forced_and_timer_clear_only_empty_human_queues_preserving_other_fields() {
        for reason in [USER_FORCED, TURN_TIMER_EXPIRED] {
            let mut f = Fixture::default();
            f.queues[1].entries.push(QueueEntry {
                type_hash: 5,
                node_type_hash: 6,
                order: Order::Other(-1),
                flags: 7,
                request_id: 8,
                location: [9, 10],
            });
            let before = f.queues[1].clone();
            let _ = token(&mut f, reason);
            assert_eq!(f.queues[0].production_overflow, 0);
            assert_eq!(f.queues[0].production_salvage, 91);
            assert_eq!(f.queues[1], before);
            assert_eq!(
                f.events
                    .iter()
                    .filter(|s| s.as_str() == "archive:0")
                    .count(),
                2
            );
            assert!(!f.events.iter().any(|s| s == "archive:1"));
        }
        for (human, reason) in [(false, USER_FORCED), (false, TURN_TIMER_EXPIRED), (true, 0)] {
            let mut f = Fixture {
                human,
                ..Fixture::default()
            };
            let before = f.queues.clone();
            let _ = token(&mut f, reason);
            assert_eq!(f.queues, before);
        }
    }
    #[test]
    fn city_end_is_reread_and_archive_failure_is_explicit_after_global_write() {
        let mut f = Fixture {
            append_on_edit: true,
            ..Fixture::default()
        };
        let _ = token(&mut f, USER_FORCED);
        assert_eq!(f.queues.len(), 3);
        assert!(f.queues.iter().all(|q| q.production_overflow == 0));
        let mut f = Fixture {
            fail_edit: true,
            ..Fixture::default()
        };
        assert_eq!(
            execute_prefix(&mut f, 7, USER_FORCED),
            Err(Error::Unsupported("archive provider unavailable"))
        );
        assert!(f.flags[7]);
        assert_eq!(f.queues[0].production_overflow, i32::MIN);
    }
    #[test]
    fn false_set_has_no_hook_and_dead_or_inactive_players_have_no_dispatch_gate() {
        let mut f = Fixture::default();
        f.flags[7] = true;
        assert_eq!(set_turn_complete(&mut f, 7, false, USER_FORCED), Ok(true));
        assert!(!f.flags[7]);
        assert!(f.events.is_empty());
        let mut f = Fixture {
            alive: false,
            active: false,
            ..Fixture::default()
        };
        let _ = token(&mut f, NONE);
        assert!(f.flags[7]);
        assert_eq!(f.hooks, 0);
    }
    #[test]
    fn mandatory_first_update_and_at_most_one_second_update_use_current_active_flag() {
        let mut f = Fixture::default();
        let first = token(&mut f, NONE);
        let second = first.run(&mut f).unwrap().unwrap();
        assert!(second.run(&mut f).unwrap().is_none());
        assert_eq!(
            &f.events[f.events.len() - 3..],
            ["update", "active:7", "update"]
        );
        let mut f = Fixture {
            deactivate_on_update: true,
            ..Fixture::default()
        };
        let first = token(&mut f, NONE);
        assert!(first.run(&mut f).unwrap().is_none());
        assert_eq!(&f.events[f.events.len() - 2..], ["update", "active:7"]);
    }
}
