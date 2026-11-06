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
/// Source-qualified pending decisions can retain their blocking effect without
/// constructing a native presentation object. They follow retained list entries
/// in tie order; callers must preserve every reached gameplay/RNG callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticBlocking {
    pub kind: i32,
    pub severity: i32,
}

pub trait Notifications {
    type Object: Copy;
    type Cursor: Copy + Eq;
    type Manager: Copy;
    type Configuration: Copy;
    type Collection: Copy;
    type Definitions: Copy;
    fn semantic_blocking(
        &mut self,
        _collection: Self::Collection,
    ) -> Result<Option<SemanticBlocking>, Error> {
        Ok(None)
    }
    fn configuration(&mut self) -> Result<Self::Configuration, Error>;
    fn definitions(&mut self) -> Result<Self::Definitions, Error>;
    /// `edit=false` is GetPlayerNotifications, `true` is EditPlayerNotifications.
    fn collection(
        &mut self,
        manager: Self::Manager,
        owner: i32,
        edit: bool,
    ) -> Result<Self::Collection, Error>;
    fn accepts_notifications(&mut self, manager: Self::Manager, owner: i32) -> Result<bool, Error>;
    fn definition_count(&mut self, definitions: Self::Definitions) -> Result<usize, Error>;
    /// Read each flag/hash at its actual short-circuit point. Non-auto
    /// definitions never invoke GetTypeHash.
    fn definition_is_auto(
        &mut self,
        definitions: Self::Definitions,
        index: usize,
    ) -> Result<bool, Error>;
    fn definition_hash(
        &mut self,
        definitions: Self::Definitions,
        index: usize,
    ) -> Result<u32, Error>;
    /// Exact Create(type, owner, true, null), including registered factory effects.
    fn create_auto(
        &mut self,
        manager: Self::Manager,
        hash: u32,
        owner: i32,
    ) -> Result<Option<Self::Object>, Error>;
    fn redundant(
        &mut self,
        collection: Self::Collection,
        object: Self::Object,
    ) -> Result<bool, Error>;
    fn release(&mut self, object: Self::Object) -> Result<(), Error>;
    /// Compatibility name for the actual virtual +0x80 InitializeText call,
    /// not reporting Send. The six stock auto-notify subtypes bind base
    /// InitializeText0x2d2dd10. Preserve subtype effects and synchronous order;
    /// the real notification-added report occurs later inside Add.
    fn send(&mut self, object: Self::Object) -> Result<(), Error>;
    fn add(&mut self, collection: Self::Collection, object: Self::Object) -> Result<(), Error>;
    fn play_by_cloud(&mut self, configuration: Self::Configuration) -> Result<bool, Error>;
    /// None is a null real manager game pointer. Called only in cloud mode.
    fn manager_game_local_player(&mut self, manager: Self::Manager) -> Result<Option<i32>, Error>;
    fn owner_turn_active(&mut self, owner: i32) -> Result<bool, Error>;
    fn network_multiplayer(&mut self, configuration: Self::Configuration) -> Result<bool, Error>;
    /// Exact selection; existing notification_lifecycle::find_auto_activate is
    /// available for its verified base profile, including live signed locks.
    fn find_auto_activate(
        &mut self,
        collection: Self::Collection,
        threshold: i32,
    ) -> Result<Option<Self::Object>, Error>;
    fn locked(&mut self, object: Self::Object) -> Result<bool, Error>;
    fn activate(&mut self, object: Self::Object, argument: bool) -> Result<(), Error>;
    /// A cursor is a native list-node identity, distinct from Object. Capture
    /// end once; next follows the current node link after synchronous effects.
    fn begin(&mut self, collection: Self::Collection) -> Result<Self::Cursor, Error>;
    fn end(&mut self, collection: Self::Collection) -> Result<Self::Cursor, Error>;
    fn object(&mut self, cursor: Self::Cursor) -> Result<Self::Object, Error>;
    fn next(&mut self, current: Self::Cursor) -> Result<Self::Cursor, Error>;
    fn dismissed(&mut self, object: Self::Object) -> Result<bool, Error>;
    fn expired(&mut self, object: Self::Object) -> Result<bool, Error>;
    /// Actual Dismiss, including synchronous event 9ad1c020 after setting198.
    fn dismiss(&mut self, object: Self::Object) -> Result<(), Error>;
    /// Full ordered Release-before-erase, with concrete destructor/pool effects.
    /// notification_lifecycle::cleanup_in_place supports its verified profile.
    fn cleanup(&mut self, collection: Self::Collection) -> Result<(), Error>;
    fn valid_for_phase(&mut self, object: Self::Object) -> Result<bool, Error>;
    fn blocking(&mut self, object: Self::Object) -> Result<i32, Error>;
    fn severity(&mut self, object: Self::Object) -> Result<i32, Error>;
}

pub fn send_auto_notify<W: Notifications>(
    w: &mut W,
    manager: W::Manager,
    owner: i32,
) -> Result<(), Error> {
    if !w.accepts_notifications(manager, owner)? {
        return Ok(());
    }
    let definitions = w.definitions()?;
    let end = w.definition_count(definitions)?;
    for index in 0..end {
        if !w.definition_is_auto(definitions, index)? {
            continue;
        }
        let hash = w.definition_hash(definitions, index)?;
        let Some(object) = w.create_auto(manager, hash, owner)? else {
            continue;
        };
        let collection = w.collection(manager, owner, true)?;
        if w.redundant(collection, object)? {
            w.release(object)?;
        } else {
            w.send(object)?;
            w.add(collection, object)?;
        }
    }
    Ok(())
}

pub fn process_auto_activate<W: Notifications>(
    w: &mut W,
    manager: W::Manager,
    owner: i32,
    budget: &mut usize,
) -> Result<(), Error> {
    let configuration = w.configuration()?;
    if !w.accepts_notifications(manager, owner)? || !w.owner_turn_active(owner)? {
        return Ok(());
    }
    let collection = w.collection(manager, owner, true)?;
    let threshold = if w.network_multiplayer(configuration)? {
        3
    } else {
        0
    };
    while let Some(object) = w.find_auto_activate(collection, threshold)? {
        if w.locked(object)? {
            break;
        }
        spend(budget)?;
        w.activate(object, false)?;
    }
    Ok(())
}

pub fn dismiss_expired<W: Notifications>(
    w: &mut W,
    collection: W::Collection,
) -> Result<(), Error> {
    let mut current = w.begin(collection)?;
    let end = w.end(collection)?;
    while current != end {
        let object = w.object(current)?;
        if !w.dismissed(object)? && w.expired(object)? {
            w.dismiss(object)?
        }
        // Read the next link after the virtual callback, as the original does.
        current = w.next(current)?;
    }
    Ok(())
}

/// 0x2d2f360: cloud gates are read *after* all auto-notification callbacks.
pub fn update_notifications<W: Notifications>(
    w: &mut W,
    manager: W::Manager,
    owner: i32,
    budget: &mut usize,
) -> Result<(), Error> {
    send_auto_notify(w, manager, owner)?;
    let configuration = w.configuration()?;
    if !w.play_by_cloud(configuration)? || w.manager_game_local_player(manager)? == Some(owner) {
        process_auto_activate(w, manager, owner, budget)?;
    }
    let collection = w.collection(manager, owner, true)?;
    dismiss_expired(w, collection)?;
    let collection = w.collection(manager, owner, true)?;
    w.cleanup(collection)
}

/// 0x2d31060: nonzero blocking among phase-valid records, first unsigned-severity
/// maximum. Read both severities at comparison time and re-read chosen blocking.
pub fn notification_blocking<W: Notifications>(
    w: &mut W,
    collection: W::Collection,
) -> Result<i32, Error> {
    let mut current = w.begin(collection)?;
    let end = w.end(collection)?;
    let mut selected = None;
    while current != end {
        let object = w.object(current)?;
        if w.valid_for_phase(object)? && w.blocking(object)? != 0 {
            let replace = match selected {
                None => true,
                Some(previous) => (w.severity(object)? as u32) > (w.severity(previous)? as u32),
            };
            if replace {
                selected = Some(object)
            }
        }
        current = w.next(current)?;
    }
    if let Some(candidate) = w.semantic_blocking(collection)? {
        if candidate.kind != 0 {
            let replace = match selected {
                None => true,
                Some(previous) => (candidate.severity as u32) > (w.severity(previous)? as u32),
            };
            if replace {
                return Ok(candidate.kind);
            }
        }
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
    type Owner: Copy;
    /// Original global Manager Get/Edit; each call returns a captured receiver.
    fn notification_manager(&mut self, edit: bool) -> Result<Self::Manager, Error>;
    fn owner_id(&mut self, owner: Self::Owner) -> Result<i32, Error>;
    fn turn_complete(&mut self, owner_id: i32) -> Result<bool, Error>;
    fn owner_blocking(&mut self, owner: Self::Owner) -> Result<i32, Error>;
    fn store_owner_blocking(&mut self, owner: Self::Owner, blocking: i32) -> Result<(), Error>;
    fn local_player(&mut self) -> Result<i32, Error>;
    fn blocking_changed(&mut self, event: u32, old: i32, new: i32) -> Result<(), Error>;
}

pub fn set_owner_blocking<W: Blocking>(
    w: &mut W,
    owner: W::Owner,
    value: i32,
) -> Result<(), Error> {
    set_owner_blocking_with(
        w,
        owner,
        value,
        |w, owner| w.owner_blocking(owner),
        |w, owner, value| w.store_owner_blocking(owner, value),
        |w, owner| w.owner_id(owner),
        |w| w.local_player(),
        |w, event, old, value| w.blocking_changed(event, old, value),
    )
}

/// Source setter control flow shared by complete notification providers and
/// concrete owned-field adapters. Closures receive the same mutable context;
/// no unrelated factory/list methods or precomputed acceptance bit is needed.
/// Reporting failure retains the preceding field store and callback effects.
#[allow(clippy::too_many_arguments)]
pub fn set_owner_blocking_with<C, O: Copy, E>(
    context: &mut C,
    owner: O,
    value: i32,
    mut read: impl FnMut(&mut C, O) -> Result<i32, E>,
    mut store: impl FnMut(&mut C, O, i32) -> Result<(), E>,
    mut owner_id: impl FnMut(&mut C, O) -> Result<i32, E>,
    mut local_player: impl FnMut(&mut C) -> Result<i32, E>,
    mut report: impl FnMut(&mut C, u32, i32, i32) -> Result<(), E>,
) -> Result<(), E> {
    let old = read(context, owner)?;
    if old != value {
        store(context, owner, value)?;
        let id = owner_id(context, owner)?;
        if id == local_player(context)? {
            report(context, BLOCKING_CHANGED, old, value)?
        }
    }
    Ok(())
}

pub fn update_owner_blocking<W: Blocking>(
    w: &mut W,
    owner: W::Owner,
    budget: &mut usize,
) -> Result<(), Error> {
    let manager = w.notification_manager(false)?;
    let id = w.owner_id(owner)?;
    let collection = w.collection(manager, id, false)?;
    let mut value = notification_blocking(w, collection)?;
    let id = w.owner_id(owner)?;
    if !w.turn_complete(id)? {
        let manager = w.notification_manager(true)?;
        let id = w.owner_id(owner)?;
        update_notifications(w, manager, id, budget)?;
        let manager = w.notification_manager(false)?;
        let id = w.owner_id(owner)?;
        let collection = w.collection(manager, id, false)?;
        value = notification_blocking(w, collection)?;
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
    owner: W::Owner,
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

    #[test]
    fn shared_blocking_setter_preserves_store_and_callback_effects_on_failure() {
        let mut context = (1i32, Vec::new());
        let result = set_owner_blocking_with(
            &mut context,
            7,
            2,
            |c, owner| {
                assert_eq!(owner, 7);
                c.1.push("read");
                Ok(c.0)
            },
            |c, owner, value| {
                assert_eq!(owner, 7);
                c.1.push("store");
                c.0 = value;
                Ok(())
            },
            |c, owner| {
                c.1.push("id");
                Ok(owner)
            },
            |c| {
                c.1.push("local");
                Ok(7)
            },
            |c, event, old, value| {
                c.1.push("report");
                assert_eq!((event, old, value, c.0), (BLOCKING_CHANGED, 1, 2, 2));
                c.0 = 3;
                Err(Error::Unsupported("required reporting continuation"))
            },
        );
        assert_eq!(
            result,
            Err(Error::Unsupported("required reporting continuation"))
        );
        assert_eq!(context, (3, vec!["read", "store", "id", "local", "report"]));
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Record {
        severity: i32,
        blocking: i32,
        phase_valid: bool,
        actual_needed: bool,
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
        trace_handles: bool,
        configuration_global: usize,
        collection_global: usize,
        replace_on_accepts: bool,
        replace_on_network: bool,
        replace_on_activation: bool,
        replace_on_send: bool,
        owner_id_reads: usize,
        manager_reads: usize,
        object_aliases: Vec<usize>,
    }
    impl World {
        fn event(&mut self, text: impl Into<String>) {
            self.log.push(text.into())
        }
        fn record(severity: i32, blocking: i32) -> Record {
            Record {
                severity,
                blocking,
                phase_valid: true,
                actual_needed: true,
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
        type Cursor = Option<usize>;
        type Manager = usize;
        type Configuration = usize;
        type Collection = usize;
        type Definitions = usize;
        fn configuration(&mut self) -> Result<usize, Error> {
            if self.trace_handles {
                self.event(format!("configuration{}", self.configuration_global));
            }
            Ok(self.configuration_global)
        }
        fn definitions(&mut self) -> Result<usize, Error> {
            Ok(0)
        }
        fn collection(&mut self, manager: usize, owner: i32, edit: bool) -> Result<usize, Error> {
            if self.trace_handles {
                self.event(format!(
                    "collection{}:{manager}:{owner}:{edit}",
                    self.collection_global
                ));
            }
            Ok(self.collection_global)
        }
        fn accepts_notifications(&mut self, _: usize, _: i32) -> Result<bool, Error> {
            self.event("accepts");
            if self.replace_on_accepts {
                self.configuration_global += 1;
            }
            Ok(true)
        }
        fn definition_count(&mut self, _: Self::Definitions) -> Result<usize, Error> {
            Ok(self.definitions.len())
        }
        fn definition_is_auto(&mut self, _: usize, i: usize) -> Result<bool, Error> {
            Ok(self.definitions[i].0)
        }
        fn definition_hash(&mut self, _: usize, i: usize) -> Result<u32, Error> {
            assert!(self.definitions[i].0);
            Ok(self.definitions[i].1)
        }
        fn create_auto(&mut self, _: usize, hash: u32, _: i32) -> Result<Option<usize>, Error> {
            self.event(format!("create{hash}"));
            Ok(if hash == 2 { None } else { Some(hash as usize) })
        }
        fn redundant(&mut self, collection: usize, object: usize) -> Result<bool, Error> {
            if self.trace_handles {
                self.event(format!("redundant_collection{collection}"));
            }
            Ok(object == 1)
        }
        fn release(&mut self, object: usize) -> Result<(), Error> {
            self.event(format!("release{object}"));
            Ok(())
        }
        fn send(&mut self, object: usize) -> Result<(), Error> {
            self.event(format!("send{object}"));
            if self.replace_on_send {
                self.collection_global += 1;
            }
            if self.send_changes_cloud {
                self.cloud = true;
            }
            Ok(())
        }
        fn add(&mut self, collection: usize, object: usize) -> Result<(), Error> {
            if self.trace_handles {
                self.event(format!("add_collection{collection}"));
            }
            self.event(format!("add{object}"));
            Ok(())
        }
        fn play_by_cloud(&mut self, configuration: Self::Configuration) -> Result<bool, Error> {
            if self.trace_handles {
                self.event(format!("cloud_configuration{configuration}"));
            }
            self.event("cloud");
            Ok(self.cloud)
        }
        fn manager_game_local_player(&mut self, _: Self::Manager) -> Result<Option<i32>, Error> {
            self.event("manager_local");
            Ok(Some(self.local))
        }
        fn owner_turn_active(&mut self, _: i32) -> Result<bool, Error> {
            self.event("active");
            Ok(true)
        }
        fn network_multiplayer(
            &mut self,
            configuration: Self::Configuration,
        ) -> Result<bool, Error> {
            if self.trace_handles {
                self.event(format!("network_configuration{configuration}"));
            }
            if self.replace_on_network {
                self.collection_global += 1;
            }
            self.event("network");
            Ok(configuration == 1)
        }
        fn find_auto_activate(
            &mut self,
            collection: usize,
            threshold: i32,
        ) -> Result<Option<usize>, Error> {
            if self.trace_handles {
                self.event(format!("find_collection{collection}"));
            }
            self.event(format!("find{threshold}"));
            Ok(self.records.iter().position(|r| !r.activated))
        }
        fn locked(&mut self, _: usize) -> Result<bool, Error> {
            Ok(false)
        }
        fn activate(&mut self, object: usize, argument: bool) -> Result<(), Error> {
            if self.replace_on_activation {
                self.collection_global += 1;
            }
            assert!(!argument);
            self.records[object].activated = true;
            self.event(format!("activate{object}"));
            Ok(())
        }
        fn begin(&mut self, _: usize) -> Result<Option<usize>, Error> {
            Ok((!self.records.is_empty()).then_some(0))
        }
        fn end(&mut self, _: usize) -> Result<Option<usize>, Error> {
            Ok(None)
        }
        fn object(&mut self, cursor: Option<usize>) -> Result<usize, Error> {
            let index = cursor.ok_or(Error::Unsupported("end cursor dereference"))?;
            Ok(self.object_aliases.get(index).copied().unwrap_or(index))
        }
        fn next(&mut self, current: Option<usize>) -> Result<Option<usize>, Error> {
            let current = current.ok_or(Error::Unsupported("end cursor increment"))?;
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
        fn cleanup(&mut self, _: usize) -> Result<(), Error> {
            self.event("cleanup");
            self.records.retain(|r| !r.dismissed);
            Ok(())
        }
        fn valid_for_phase(&mut self, object: usize) -> Result<bool, Error> {
            Ok(self.records[object].phase_valid)
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
        type Owner = usize;
        fn notification_manager(&mut self, edit: bool) -> Result<usize, Error> {
            self.manager_reads += 1;
            if self.trace_handles {
                self.event(format!("manager{}:{edit}", self.manager_reads));
            }
            Ok(self.manager_reads)
        }
        fn owner_id(&mut self, owner: usize) -> Result<i32, Error> {
            self.owner_id_reads += 1;
            if self.trace_handles {
                self.event(format!("owner_id{owner}:{}", self.owner_id_reads));
                Ok(self.owner_id_reads as i32)
            } else {
                Ok(owner as i32)
            }
        }

        fn turn_complete(&mut self, _: i32) -> Result<bool, Error> {
            self.event("complete");
            Ok(self.complete)
        }
        fn owner_blocking(&mut self, _: usize) -> Result<i32, Error> {
            Ok(self.blocking)
        }
        fn store_owner_blocking(&mut self, owner: usize, value: i32) -> Result<(), Error> {
            if self.trace_handles {
                self.event(format!("store_owner{owner}"));
            }
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
        update_notifications(&mut w, 0, 0, &mut 10).unwrap();
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
        update_notifications(&mut w, 0, 0, &mut 10).unwrap();
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
    fn node_cursors_distinguish_repeated_object_entries() {
        let mut w = World {
            records: vec![World::record(3, 2), World::record(0, 0)],
            object_aliases: vec![0, 0],
            ..World::default()
        };
        assert_eq!(notification_blocking(&mut w, 0), Ok(2));
        // Two distinct nodes read the same object, then the selected getter.
        assert_eq!(w.log.iter().filter(|s| *s == "block0").count(), 3);
    }
    #[test]
    fn reducer_uses_phase_validity_separately_from_actual_is_needed() {
        let mut w = World {
            records: vec![World::record(0, 4)],
            ..World::default()
        };
        w.records[0].actual_needed = false;
        assert_eq!(notification_blocking(&mut w, 0), Ok(4));
        assert_eq!(w.log, ["block0", "block0"]);
        w.records[0].actual_needed = true;
        w.records[0].phase_valid = false;
        w.log.clear();
        assert_eq!(notification_blocking(&mut w, 0), Ok(0));
        assert!(w.log.is_empty());
    }
    #[test]
    fn expiry_signed_wrap_boundaries() {
        assert!(!base_expired(0, 0, i32::MAX));
        assert!(!base_expired(0, -1, i32::MAX));
        assert!(!base_expired(3, 2, 4));
        assert!(base_expired(3, 2, 5));
        assert!(base_expired(i32::MAX, 1, i32::MIN));
    }

    #[test]
    fn captures_configuration_before_accepts_and_collection_before_network_and_activation() {
        let mut w = World {
            trace_handles: true,
            replace_on_accepts: true,
            replace_on_network: true,
            replace_on_activation: true,
            records: vec![World::record(0, 0)],
            ..World::default()
        };
        w.records[0].activated = false;
        process_auto_activate(&mut w, 7, 2, &mut 5).unwrap();
        assert_eq!(
            w.log,
            [
                "configuration0",
                "accepts",
                "active",
                "collection0:7:2:true",
                "network_configuration0",
                "network",
                "find_collection0",
                "find0",
                "activate0",
                "find_collection0",
                "find0"
            ]
        );
        assert_eq!(w.configuration_global, 1);
        assert_eq!(w.collection_global, 2);
    }

    #[test]
    fn send_retains_collection_for_add_then_next_definition_reacquires() {
        let mut w = World {
            trace_handles: true,
            replace_on_send: true,
            definitions: vec![(true, 3), (true, 4)],
            ..World::default()
        };
        send_auto_notify(&mut w, 7, 2).unwrap();
        assert_eq!(
            w.log,
            [
                "accepts",
                "create3",
                "collection0:7:2:true",
                "redundant_collection0",
                "send3",
                "add_collection0",
                "add3",
                "create4",
                "collection1:7:2:true",
                "redundant_collection1",
                "send4",
                "add_collection1",
                "add4"
            ]
        );
        assert_eq!(w.collection_global, 2);
    }

    #[test]
    fn owner_receiver_survives_repeated_id_reads_and_manager_lookup_precedes_id() {
        let mut w = World {
            trace_handles: true,
            records: vec![World::record(0, 4)],
            local: 5,
            ..World::default()
        };
        update_owner_blocking(&mut w, 99, &mut 5).unwrap();
        let observed: Vec<&str> = w
            .log
            .iter()
            .filter(|s| {
                s.starts_with("manager")
                    || s.starts_with("owner_id")
                    || s.starts_with("collection")
                    || s.starts_with("store")
                    || s.starts_with("changed")
            })
            .map(String::as_str)
            .collect();
        assert_eq!(
            observed,
            [
                "manager1:false",
                "owner_id99:1",
                "collection0:1:1:false",
                "owner_id99:2",
                "manager2:true",
                "owner_id99:3",
                "collection0:2:3:true",
                "collection0:2:3:true",
                "collection0:2:3:true",
                "manager3:false",
                "owner_id99:4",
                "collection0:3:4:false",
                "store_owner99",
                "store4",
                "owner_id99:5",
                "changed0:4"
            ]
        );
    }
}
