//! Source-derived Linux XP2 continuation after UnitOperation's manager Add.
//!
//! This is control flow over required, typed providers, not an accepted action.
//! Subtype rules, statement processing, reporting and complete state/RNG remain
//! required. No default no-op provider or native policy implementation is supplied.
//! See docs/linux-unit-operation-aftermath.md for the isolation contract.

pub const BLOCKING_CHANGED: u32 = 0x55f15a7b;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Unsupported(&'static str),
    /// A genuine player choice must be supplied by external policy. This error
    /// is not a native resumable continuation or a successful action.
    ExternalDecisionRequired(&'static str),
    DiagnosticBudgetExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pending<S> {
    pub from: i32,
    pub to: i32,
    /// Typed statement identity/payload owned by the actual statement provider.
    pub statement: S,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session<S> {
    pub from: i32,
    pub to: i32,
    pub pending: Vec<Pending<S>>,
}

/// Original outgoing removal at 0x2917270: first matching source, then optional
/// destination, erase before returning. This never processes/destroys a statement.
pub fn remove_next<S>(session: &mut Session<S>, player: i32, other: i32) -> Option<S> {
    let index = session
        .pending
        .iter()
        .position(|p| p.from == player && (other == -1 || p.to == other))?;
    Some(session.pending.remove(index).statement)
}

pub trait Diplomacy {
    type Player: Copy;
    type Statement;
    /// Copy original ever-alive *handles*, not player fields. Later IsAI reads
    /// must see changes caused by processing an earlier player's statements.
    fn ever_alive(&self) -> Vec<Self::Player>;
    fn player_id(&self, player: Self::Player) -> i32;
    fn is_ai(&self, player: Self::Player) -> bool;
    fn open_sessions(&mut self) -> &mut Vec<Session<Self::Statement>>;
    /// Full Manager::ProcessStatement, including session routing, ownership,
    /// history, callbacks, RNG and external decisions. No policy may run here.
    fn process_statement(&mut self, statement: Self::Statement) -> Result<(), Error>;
}

/// 0x2913510 / 0x2911900, with actual pending-list mutation before the provider.
/// Each successful processing restarts the session scan from the current head.
pub fn process_pending<W: Diplomacy>(world: &mut W, budget: &mut usize) -> Result<(), Error> {
    let players = world.ever_alive();
    for player in players {
        let id = world.player_id(player);
        // Native IsAI is tested once, not on every iteration of this drain.
        if !world.is_ai(player) {
            continue;
        }
        loop {
            let statement = world
                .open_sessions()
                .iter_mut()
                .filter(|s| s.from == id || s.to == id)
                .find_map(|s| remove_next(s, id, -1));
            let Some(statement) = statement else { break };
            spend(budget)?;
            world.process_statement(statement)?;
        }
    }
    Ok(())
}

fn spend(budget: &mut usize) -> Result<(), Error> {
    *budget = budget
        .checked_sub(1)
        .ok_or(Error::DiagnosticBudgetExceeded)?;
    Ok(())
}

/// The full live notification provider. Handles identify objects, not serialized
/// ComponentIDs (which may duplicate). All virtual methods must bind the actual
/// target subtype. Getter/callback effects must be synchronous and stateful.
/// Cursor methods follow the current native list links; no snapshot list is used.
/// The definition array's allocation/end must stay stable during SendAutoNotify,
/// as required by the original captured begin/end pointer iteration.
pub trait Notifications {
    type Object: Copy;
    fn accepts_notifications(&mut self, owner: i32) -> Result<bool, Error>;
    fn definition_count(&mut self) -> Result<usize, Error>;
    /// Read each flag/hash at its actual short-circuit point. Non-auto
    /// definitions never invoke GetTypeHash.
    fn definition_is_auto(&mut self, index: usize) -> Result<bool, Error>;
    fn definition_hash(&mut self, index: usize) -> Result<u32, Error>;
    /// Exact Create(type, owner, true, null), including registered factory effects.
    fn create_auto(&mut self, hash: u32, owner: i32) -> Result<Option<Self::Object>, Error>;
    fn redundant(&mut self, owner: i32, object: Self::Object) -> Result<bool, Error>;
    fn release(&mut self, object: Self::Object) -> Result<(), Error>;
    fn send(&mut self, object: Self::Object) -> Result<(), Error>;
    fn add(&mut self, owner: i32, object: Self::Object) -> Result<(), Error>;
    fn play_by_cloud(&mut self) -> Result<bool, Error>;
    /// None is a null real manager game pointer. Called only in cloud mode.
    fn manager_game_local_player(&mut self) -> Result<Option<i32>, Error>;
    fn owner_turn_active(&mut self, owner: i32) -> Result<bool, Error>;
    fn network_multiplayer(&mut self) -> Result<bool, Error>;
    /// Exact selection; existing notification_lifecycle::find_auto_activate is
    /// available for its verified base profile, including live signed locks.
    fn find_auto_activate(
        &mut self,
        owner: i32,
        threshold: i32,
    ) -> Result<Option<Self::Object>, Error>;
    fn locked(&mut self, object: Self::Object) -> Result<bool, Error>;
    fn activate(&mut self, object: Self::Object, argument: bool) -> Result<(), Error>;
    fn first(&mut self, owner: i32) -> Result<Option<Self::Object>, Error>;
    fn next(&mut self, owner: i32, current: Self::Object) -> Result<Option<Self::Object>, Error>;
    fn dismissed(&mut self, object: Self::Object) -> Result<bool, Error>;
    fn expired(&mut self, object: Self::Object) -> Result<bool, Error>;
    /// Actual Dismiss, including synchronous event 9ad1c020 after setting198.
    fn dismiss(&mut self, object: Self::Object) -> Result<(), Error>;
    /// Full ordered Release-before-erase, with concrete destructor/pool effects.
    /// notification_lifecycle::cleanup_in_place supports its verified profile.
    fn cleanup(&mut self, owner: i32) -> Result<(), Error>;
    fn needed(&mut self, object: Self::Object) -> Result<bool, Error>;
    fn blocking(&mut self, object: Self::Object) -> Result<i32, Error>;
    fn severity(&mut self, object: Self::Object) -> Result<i32, Error>;
}

pub fn send_auto_notify<W: Notifications>(w: &mut W, owner: i32) -> Result<(), Error> {
    if !w.accepts_notifications(owner)? {
        return Ok(());
    }
    let end = w.definition_count()?;
    for index in 0..end {
        if !w.definition_is_auto(index)? {
            continue;
        }
        let hash = w.definition_hash(index)?;
        let Some(object) = w.create_auto(hash, owner)? else {
            continue;
        };
        if w.redundant(owner, object)? {
            w.release(object)?;
        } else {
            w.send(object)?;
            w.add(owner, object)?;
        }
    }
    Ok(())
}

pub fn process_auto_activate<W: Notifications>(
    w: &mut W,
    owner: i32,
    budget: &mut usize,
) -> Result<(), Error> {
    if !w.accepts_notifications(owner)? || !w.owner_turn_active(owner)? {
        return Ok(());
    }
    let threshold = if w.network_multiplayer()? { 3 } else { 0 };
    while let Some(object) = w.find_auto_activate(owner, threshold)? {
        if w.locked(object)? {
            break;
        }
        spend(budget)?;
        w.activate(object, false)?;
    }
    Ok(())
}

pub fn dismiss_expired<W: Notifications>(w: &mut W, owner: i32) -> Result<(), Error> {
    let mut current = w.first(owner)?;
    while let Some(object) = current {
        if !w.dismissed(object)? && w.expired(object)? {
            w.dismiss(object)?
        }
        // Read the next link after the virtual callback, as the original does.
        current = w.next(owner, object)?;
    }
    Ok(())
}

/// 0x2d2f360: cloud gates are read *after* all auto-notification callbacks.
pub fn update_notifications<W: Notifications>(
    w: &mut W,
    owner: i32,
    budget: &mut usize,
) -> Result<(), Error> {
    send_auto_notify(w, owner)?;
    if !w.play_by_cloud()? || w.manager_game_local_player()? == Some(owner) {
        process_auto_activate(w, owner, budget)?;
    }
    dismiss_expired(w, owner)?;
    w.cleanup(owner)
}

/// 0x2d31060: nonzero blocking among needed records, first unsigned-severity
/// maximum. Read both severities at comparison time and re-read chosen blocking.
pub fn notification_blocking<W: Notifications>(w: &mut W, owner: i32) -> Result<i32, Error> {
    let mut current = w.first(owner)?;
    let mut selected = None;
    while let Some(object) = current {
        if w.needed(object)? && w.blocking(object)? != 0 {
            let replace = match selected {
                None => true,
                Some(previous) => (w.severity(object)? as u32) > (w.severity(previous)? as u32),
            };
            if replace {
                selected = Some(object)
            }
        }
        current = w.next(owner, object)?;
    }
    match selected {
        Some(object) => w.blocking(object),
        None => Ok(0),
    }
}

/// Only the verified base AutoExpires/IsExpired binding; subtype overrides must
/// use their own provider. Native19c is deliberately not an input.
pub fn base_expired(added: i32, delta: i32, current_turn: i32) -> bool {
    delta > 0 && current_turn >= added.wrapping_add(delta)
}

pub trait Blocking: Notifications {
    fn turn_complete(&mut self, owner: i32) -> Result<bool, Error>;
    fn owner_blocking(&mut self, owner: i32) -> Result<i32, Error>;
    fn store_owner_blocking(&mut self, owner: i32, blocking: i32) -> Result<(), Error>;
    fn local_player(&mut self) -> Result<i32, Error>;
    /// Exact Type,int,int overload; store precedes local check and Send.
    fn blocking_changed(&mut self, event: u32, old: i32, new: i32) -> Result<(), Error>;
}

pub fn set_owner_blocking<W: Blocking>(w: &mut W, owner: i32, value: i32) -> Result<(), Error> {
    let old = w.owner_blocking(owner)?;
    if old != value {
        w.store_owner_blocking(owner, value)?;
        if owner == w.local_player()? {
            w.blocking_changed(BLOCKING_CHANGED, old, value)?
        }
    }
    Ok(())
}

pub fn update_owner_blocking<W: Blocking>(
    w: &mut W,
    owner: i32,
    budget: &mut usize,
) -> Result<(), Error> {
    let mut value = notification_blocking(w, owner)?;
    if !w.turn_complete(owner)? {
        update_notifications(w, owner, budget)?;
        value = notification_blocking(w, owner)?;
    }
    set_owner_blocking(w, owner, value)
}

/// Execute only the post-Add continuation in an isolated owned candidate.
///
/// `Clone` MUST isolate all mutable provider/state effects; shared interior
/// mutation, native callbacks, I/O and external effects violate this API's
/// contract. Unsupported/decision/budget errors discard the candidate. Lower
/// level functions mutate in place and do not promise rollback after errors.
/// Call even when Add rejected the operation, provided Execute found its unit.
/// This does not include request TurnUnready, Add, or a resumable policy yield.
pub fn after_add<W: Diplomacy + Blocking + Clone>(
    world: &W,
    owner: i32,
    diagnostic_budget: usize,
) -> Result<W, Error> {
    let mut candidate = world.clone();
    let mut budget = diagnostic_budget;
    process_pending(&mut candidate, &mut budget)?;
    update_owner_blocking(&mut candidate, owner, &mut budget)?;
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Record {
        severity: i32,
        blocking: i32,
        needed: bool,
        expired: bool,
        dismissed: bool,
        activated: bool,
    }
    #[derive(Clone, Debug, Default, PartialEq)]
    struct World {
        sessions: Vec<Session<u32>>,
        ai: [bool; 3],
        log: Vec<String>,
        records: Vec<Record>,
        definitions: Vec<(bool, u32)>,
        cloud: bool,
        local: i32,
        complete: bool,
        blocking: i32,
        reject_statement: bool,
        reject_expiry: bool,
        inject: bool,
        send_changes_cloud: bool,
        blocking_calls: usize,
        change_final_blocking: bool,
    }
    impl World {
        fn event(&mut self, text: impl Into<String>) {
            self.log.push(text.into())
        }
        fn record(severity: i32, blocking: i32) -> Record {
            Record {
                severity,
                blocking,
                needed: true,
                expired: false,
                dismissed: false,
                activated: true,
            }
        }
    }
    impl Diplomacy for World {
        type Player = usize;
        type Statement = u32;
        fn ever_alive(&self) -> Vec<usize> {
            vec![0, 1, 2]
        }
        fn player_id(&self, player: usize) -> i32 {
            player as i32
        }
        fn is_ai(&self, player: usize) -> bool {
            self.ai[player]
        }
        fn open_sessions(&mut self) -> &mut Vec<Session<u32>> {
            &mut self.sessions
        }
        fn process_statement(&mut self, statement: u32) -> Result<(), Error> {
            self.event(format!("statement{statement}"));
            if self.reject_statement {
                return Err(Error::ExternalDecisionRequired("statement response"));
            }
            if self.inject {
                self.inject = false;
                // Earlier session becomes eligible; processing must restart at head.
                self.sessions[0].pending.push(Pending {
                    from: 0,
                    to: 2,
                    statement: 9,
                });
                self.ai[1] = true;
            }
            Ok(())
        }
    }
    impl Notifications for World {
        type Object = usize;
        fn accepts_notifications(&mut self, _: i32) -> Result<bool, Error> {
            self.event("accepts");
            Ok(true)
        }
        fn definition_count(&mut self) -> Result<usize, Error> {
            Ok(self.definitions.len())
        }
        fn definition_is_auto(&mut self, i: usize) -> Result<bool, Error> {
            Ok(self.definitions[i].0)
        }
        fn definition_hash(&mut self, i: usize) -> Result<u32, Error> {
            assert!(self.definitions[i].0);
            Ok(self.definitions[i].1)
        }
        fn create_auto(&mut self, hash: u32, _: i32) -> Result<Option<usize>, Error> {
            self.event(format!("create{hash}"));
            Ok(if hash == 2 { None } else { Some(hash as usize) })
        }
        fn redundant(&mut self, _: i32, object: usize) -> Result<bool, Error> {
            Ok(object == 1)
        }
        fn release(&mut self, object: usize) -> Result<(), Error> {
            self.event(format!("release{object}"));
            Ok(())
        }
        fn send(&mut self, object: usize) -> Result<(), Error> {
            self.event(format!("send{object}"));
            if self.send_changes_cloud {
                self.cloud = true;
            }
            Ok(())
        }
        fn add(&mut self, _: i32, object: usize) -> Result<(), Error> {
            self.event(format!("add{object}"));
            Ok(())
        }
        fn play_by_cloud(&mut self) -> Result<bool, Error> {
            self.event("cloud");
            Ok(self.cloud)
        }
        fn manager_game_local_player(&mut self) -> Result<Option<i32>, Error> {
            self.event("manager_local");
            Ok(Some(self.local))
        }
        fn owner_turn_active(&mut self, _: i32) -> Result<bool, Error> {
            self.event("active");
            Ok(true)
        }
        fn network_multiplayer(&mut self) -> Result<bool, Error> {
            self.event("network");
            Ok(false)
        }
        fn find_auto_activate(&mut self, _: i32, threshold: i32) -> Result<Option<usize>, Error> {
            self.event(format!("find{threshold}"));
            Ok(self.records.iter().position(|r| !r.activated))
        }
        fn locked(&mut self, _: usize) -> Result<bool, Error> {
            Ok(false)
        }
        fn activate(&mut self, object: usize, argument: bool) -> Result<(), Error> {
            assert!(!argument);
            self.records[object].activated = true;
            self.event(format!("activate{object}"));
            Ok(())
        }
        fn first(&mut self, _: i32) -> Result<Option<usize>, Error> {
            Ok((!self.records.is_empty()).then_some(0))
        }
        fn next(&mut self, _: i32, current: usize) -> Result<Option<usize>, Error> {
            Ok((current + 1 < self.records.len()).then_some(current + 1))
        }
        fn dismissed(&mut self, object: usize) -> Result<bool, Error> {
            Ok(self.records[object].dismissed)
        }
        fn expired(&mut self, object: usize) -> Result<bool, Error> {
            self.event(format!("expiry{object}"));
            if self.reject_expiry {
                return Err(Error::Unsupported("subtype expiry"));
            }
            Ok(self.records[object].expired)
        }
        fn dismiss(&mut self, object: usize) -> Result<(), Error> {
            self.records[object].dismissed = true;
            self.event(format!("dismiss{object}"));
            // Append during callback: original iterator reaches the new tail.
            let mut record = Self::record(0, 0);
            record.dismissed = true;
            self.records.push(record);
            Ok(())
        }
        fn cleanup(&mut self, _: i32) -> Result<(), Error> {
            self.event("cleanup");
            self.records.retain(|r| !r.dismissed);
            Ok(())
        }
        fn needed(&mut self, object: usize) -> Result<bool, Error> {
            Ok(self.records[object].needed)
        }
        fn blocking(&mut self, object: usize) -> Result<i32, Error> {
            self.blocking_calls += 1;
            self.event(format!("block{object}"));
            if self.change_final_blocking && self.blocking_calls > self.records.len() {
                return Ok(77);
            }
            Ok(self.records[object].blocking)
        }
        fn severity(&mut self, object: usize) -> Result<i32, Error> {
            self.event(format!("severity{object}"));
            Ok(self.records[object].severity)
        }
    }
    impl Blocking for World {
        fn turn_complete(&mut self, _: i32) -> Result<bool, Error> {
            self.event("complete");
            Ok(self.complete)
        }
        fn owner_blocking(&mut self, _: i32) -> Result<i32, Error> {
            Ok(self.blocking)
        }
        fn store_owner_blocking(&mut self, _: i32, value: i32) -> Result<(), Error> {
            self.blocking = value;
            self.event(format!("store{value}"));
            Ok(())
        }
        fn local_player(&mut self) -> Result<i32, Error> {
            self.event("local");
            Ok(self.local)
        }
        fn blocking_changed(&mut self, event: u32, old: i32, new: i32) -> Result<(), Error> {
            assert_eq!(event, BLOCKING_CHANGED);
            assert_eq!(self.blocking, new);
            self.event(format!("changed{old}:{new}"));
            Ok(())
        }
    }

    #[test]
    fn pending_removal_filters_endpoints_and_preserves_order() {
        let mut s = Session {
            from: 1,
            to: 2,
            pending: vec![
                Pending {
                    from: 2,
                    to: 1,
                    statement: 1,
                },
                Pending {
                    from: 1,
                    to: 3,
                    statement: 2,
                },
                Pending {
                    from: 1,
                    to: 2,
                    statement: 3,
                },
            ],
        };
        assert_eq!(remove_next(&mut s, 1, 2), Some(3));
        assert_eq!(remove_next(&mut s, 1, -1), Some(2));
        assert_eq!(remove_next(&mut s, 1, -1), None);
        assert_eq!(s.pending[0].statement, 1);
    }
    #[test]
    fn pending_drain_restarts_sessions_and_reads_later_player_ai_live() {
        let mut w = World {
            ai: [true, false, false],
            inject: true,
            sessions: vec![
                Session {
                    from: 0,
                    to: 2,
                    pending: vec![],
                },
                Session {
                    from: 0,
                    to: 2,
                    pending: vec![Pending {
                        from: 0,
                        to: 2,
                        statement: 1,
                    }],
                },
                Session {
                    from: 1,
                    to: 2,
                    pending: vec![Pending {
                        from: 1,
                        to: 2,
                        statement: 2,
                    }],
                },
            ],
            ..World::default()
        };
        process_pending(&mut w, &mut 10).unwrap();
        assert_eq!(w.log, ["statement1", "statement9", "statement2"]);
        assert!(w.sessions.iter().all(|s| s.pending.is_empty()));
    }
    #[test]
    fn update_factory_branches_and_callback_cloud_reread() {
        let mut w = World {
            definitions: vec![(false, 0), (true, 1), (true, 2), (true, 3)],
            send_changes_cloud: true,
            local: 2,
            records: vec![World::record(0, 0)],
            ..World::default()
        };
        w.records[0].expired = true;
        w.records[0].activated = false;
        update_notifications(&mut w, 0, &mut 10).unwrap();
        assert_eq!(
            w.log,
            [
                "accepts",
                "create1",
                "release1",
                "create2",
                "create3",
                "send3",
                "add3",
                "cloud",
                "manager_local",
                "expiry0",
                "dismiss0",
                "cleanup"
            ]
        );
        assert!(w.records.is_empty());
    }
    #[test]
    fn activation_precedes_expiry_then_cleanup() {
        let mut w = World {
            records: vec![World::record(0, 0)],
            ..World::default()
        };
        w.records[0].activated = false;
        update_notifications(&mut w, 0, &mut 10).unwrap();
        assert_eq!(
            w.log,
            [
                "accepts",
                "cloud",
                "accepts",
                "active",
                "network",
                "find0",
                "activate0",
                "find0",
                "expiry0",
                "cleanup"
            ]
        );
    }
    #[test]
    fn blocking_unsigned_ties_and_final_virtual_reread() {
        let mut w = World {
            records: vec![
                World::record(3, 1),
                World::record(-1, 2),
                World::record(-1, 3),
            ],
            change_final_blocking: true,
            ..World::default()
        };
        assert_eq!(notification_blocking(&mut w, 0), Ok(77));
        assert_eq!(w.log.last().unwrap(), "block1");
        assert_eq!(w.log.iter().filter(|s| *s == "severity0").count(), 1);
    }
    #[test]
    fn complete_skips_update_equal_skips_local_and_changed_stores_before_report() {
        let mut w = World {
            complete: true,
            records: vec![World::record(1, 4)],
            ..World::default()
        };
        update_owner_blocking(&mut w, 0, &mut 0).unwrap();
        assert_eq!(
            w.log,
            [
                "block0",
                "block0",
                "complete",
                "store4",
                "local",
                "changed0:4"
            ]
        );
        w.log.clear();
        update_owner_blocking(&mut w, 0, &mut 0).unwrap();
        assert_eq!(w.log, ["block0", "block0", "complete"]);
    }
    #[test]
    fn isolated_candidate_rejects_decision_and_late_unsupported_without_input_mutation() {
        let mut w = World {
            ai: [true, false, false],
            reject_statement: true,
            sessions: vec![Session {
                from: 0,
                to: 1,
                pending: vec![Pending {
                    from: 0,
                    to: 1,
                    statement: 1,
                }],
            }],
            ..World::default()
        };
        let before = w.clone();
        assert_eq!(
            after_add(&w, 0, 10),
            Err(Error::ExternalDecisionRequired("statement response"))
        );
        assert_eq!(w, before);
        w.reject_statement = false;
        w.reject_expiry = true;
        w.records.push(World::record(0, 0));
        let before = w.clone();
        assert_eq!(
            after_add(&w, 0, 10),
            Err(Error::Unsupported("subtype expiry"))
        );
        assert_eq!(w, before);
        assert_eq!(after_add(&w, 0, 0), Err(Error::DiagnosticBudgetExceeded));
        assert_eq!(w, before);
    }
    #[test]
    fn expiry_signed_wrap_boundaries() {
        assert!(!base_expired(0, 0, i32::MAX));
        assert!(!base_expired(0, -1, i32::MAX));
        assert!(!base_expired(3, 2, 4));
        assert!(base_expired(3, 2, 5));
        assert!(base_expired(i32::MAX, 1, i32::MIN));
    }
}
