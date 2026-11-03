//! Concrete Linux notification predicates over wire state plus explicit live inputs.
//! No full Manager::Update or gameplay action is admitted by this adapter.
use crate::notification_lifecycle::{self as lifecycle, ObjectId};
use crate::unit_operation_aftermath::{self as aftermath, Error, Notifications};
use std::collections::BTreeSet;

/// Actual target vtable addresspoint, expiration, phase-valid, needed slots.
/// Preserve subtype identity even where several inherited implementations match.
const PROFILES: &[(u64, u64, u64, u64)] = &[
    (0x3af4840, 0x2d2e250, 0x2d29200, 0x2d2e290),
    (0x3af5060, 0x2d2a320, 0x2d2a3a0, 0x2d2e290),
    (0x3af4c50, 0x2d299d0, 0x2d29a40, 0x2d2e290),
    (0x3af6600, 0x2d31890, 0x2d318b0, 0x2d2e290),
    (0x3af4ab0, 0x2d29630, 0x2d29690, 0x2d2e290),
    (0x3af53a0, 0x2d2aab0, 0x2d2aae0, 0x2d2e290),
    (0x3af5c90, 0x2d2c940, 0x2d2c9e0, 0x2d2c9f0),
    (0x3af4ec0, 0x2d29f70, 0x2d29fc0, 0x2d2e290),
    (0x3af4f90, 0x2d2a120, 0x2d2a1c0, 0x2d2e290),
    (0x3af6460, 0x2d31420, 0x2d31430, 0x2d2e290),
    (0x3af67a0, 0x2d2e250, 0x2d29200, 0x2d2e290),
    (0x3af5130, 0x2d2a540, 0x2d2a570, 0x2d2e290),
    (0x3af5200, 0x2d2e250, 0x2d29200, 0x2d2e290),
    (0x3af5610, 0x2d2aff0, 0x2d2b000, 0x2d2e290),
    (0x3af5a20, 0x2d2e250, 0x2d29200, 0x2d2e290),
    (0x3af52d0, 0x2d2a860, 0x2d2a8b0, 0x2d2e290),
    (0x3af56e0, 0x2d2b100, 0x2d2b1e0, 0x2d2e290),
    (0x3af5950, 0x2d2b760, 0x2d2b820, 0x2d2e290),
    (0x3af5af0, 0x2d2bbc0, 0x2d2be30, 0x2d2be40),
    (0x3af66d0, 0x2d2e250, 0x2d29200, 0x2d2e290),
    (0x3af4df0, 0x2d29dd0, 0x2d29e10, 0x2d2e290),
    (0x3af5540, 0x2d2ae50, 0x2d2ae90, 0x2d2e290),
    (0x3af5e30, 0x2d2cd20, 0x2d2cd80, 0x2d2e290),
    (0x3af5bc0, 0x2d2e250, 0x2d29200, 0x2d2e290),
    (0x3af5f00, 0x2d2cee0, 0x2d2cf40, 0x2d2e290),
    (0x3af60a0, 0x2d2d250, 0x2d2d2b0, 0x2d2e290),
    (0x3af6530, 0x2d2e250, 0x2d29200, 0x2d2e290),
    (0x3af4770, 0x2d28e60, 0x2d28ea0, 0x2d2e290),
    (0x3af49e0, 0x2d293f0, 0x2d294d0, 0x2d2e290),
    (0x3af4b80, 0x2d29800, 0x2d29870, 0x2d2e290),
    (0x3af57b0, 0x2d2b350, 0x2d2b410, 0x2d2e290),
    (0x3af4d20, 0x2d29c40, 0x2d29c70, 0x2d2e290),
    (0x3af5470, 0x2d2ac40, 0x2d2ac90, 0x2d2e290),
    (0x3af5880, 0x2d2b570, 0x2d2b5f0, 0x2d2b600),
    (0x3af5fd0, 0x2d2d0a0, 0x2d2d0f0, 0x2d2e290),
    (0x3af5d60, 0x2d2cb40, 0x2d29200, 0x2d2e290),
    (0x3af6170, 0x2d2d410, 0x2d2d470, 0x2d2e290),
    (0x3af6238, 0x2d2e250, 0x2d29200, 0x2d2e290),
    (0x3af4910, 0x2d2e250, 0x2d29200, 0x2d2e290),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivePredicates {
    pub object: ObjectId,
    pub vtable_addresspoint: u64,
    /// Actual omitted Instance +0x130, not a definition lookup or wire default.
    pub default_blocking: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedOwner {
    state: lifecycle::State,
    predicates: Vec<LivePredicates>,
    /// Actual game turn. None preserves an absent capture; short circuits may
    /// avoid this input but a positive expiry delta requires it.
    pub current_turn: Option<i32>,
}

impl CapturedOwner {
    pub fn state(&self) -> &lifecycle::State {
        &self.state
    }
    pub fn predicates(&self) -> &[LivePredicates] {
        &self.predicates
    }
    pub fn new(
        state: lifecycle::State,
        predicates: Vec<LivePredicates>,
        current_turn: Option<i32>,
    ) -> Result<Self, Error> {
        if state.notifications.records.len() != state.live.len()
            || predicates.len() != state.live.len()
        {
            return Err(Error::Unsupported("live predicate cardinality"));
        }
        let mut objects = BTreeSet::new();
        for (live, predicate) in state.live.iter().zip(&predicates) {
            if live.object != predicate.object || !objects.insert(live.object) {
                return Err(Error::Unsupported("live predicate object correspondence"));
            }
            if !PROFILES
                .iter()
                .any(|p| p.0 == predicate.vtable_addresspoint)
            {
                return Err(Error::Unsupported("unknown target notification subtype"));
            }
        }
        Ok(Self {
            state,
            predicates,
            current_turn,
        })
    }
    fn index(&self, object: ObjectId) -> Result<usize, Error> {
        self.state
            .live
            .iter()
            .position(|r| r.object == object)
            .ok_or(Error::Unsupported(
                "notification object not in captured owner",
            ))
    }
    fn profile(&self, index: usize) -> Result<(u64, u64, u64, u64), Error> {
        let address = self
            .predicates
            .get(index)
            .ok_or(Error::Unsupported("missing live predicate record"))?
            .vtable_addresspoint;
        PROFILES
            .iter()
            .find(|p| p.0 == address)
            .copied()
            .ok_or(Error::Unsupported("unknown target notification subtype"))
    }
    pub fn expired(&self, object: ObjectId) -> Result<bool, Error> {
        let i = self.index(object)?;
        if self.profile(i)?.1 != 0x2d2e250 {
            return Err(Error::Unsupported("concrete subtype expiration"));
        }
        // All 39 actual AutoExpires slots bind base positive signed delta.
        let record = &self.state.notifications.records[i];
        if record.native_18c <= 0 {
            return Ok(false);
        }
        let turn = self
            .current_turn
            .ok_or(Error::Unsupported("actual current game turn"))?;
        Ok(aftermath::base_expired(
            record.native_188,
            record.native_18c,
            turn,
        ))
    }
    pub fn valid_for_phase(&self, object: ObjectId) -> Result<bool, Error> {
        let i = self.index(object)?;
        if self.profile(i)?.2 != 0x2d29200 {
            return Err(Error::Unsupported("concrete subtype phase predicate"));
        }
        Ok(true)
    }
    /// Actual +0x38 IsNeeded; NOT the blocking reducer's +0x30 predicate.
    pub fn is_needed(&self, object: ObjectId) -> Result<bool, Error> {
        let i = self.index(object)?;
        if self.profile(i)?.3 != 0x2d2e290 {
            return Err(Error::Unsupported("concrete subtype IsNeeded"));
        }
        if !self.state.notifications.records[i].native_19a {
            return Ok(true);
        }
        Ok(!self.expired(object)?)
    }
    pub fn blocking(&self, object: ObjectId) -> Result<i32, Error> {
        let i = self.index(object)?;
        // All 39 actual +0x50 slots use base getter, but it dispatches expiry.
        if self.state.notifications.records[i].is_dismissed || self.expired(object)? {
            return Ok(0);
        }
        self.predicates[i]
            .default_blocking
            .ok_or(Error::Unsupported("actual live default blocking +0x130"))
    }
    pub fn blocking_reduction(&mut self) -> Result<i32, Error> {
        aftermath::notification_blocking(self, ())
    }
}

macro_rules! unsupported {
    ($name:ident ($($arg:ident : $ty:ty),*) -> $ret:ty, $reason:literal) => {
        fn $name(&mut self, $($arg: $ty),*) -> Result<$ret, Error> {
            $(let _ = $arg;)*
            Err(Error::Unsupported($reason))
        }
    };
}

/// Concrete read-only provider for the verified blocking/predicate boundary.
/// Uncaptured manager/factory/effect inputs reject explicitly; no no-op is used.
/// The owned vector models this immutable captured list's node order, while
/// ObjectId keeps duplicate ComponentIDs distinct. Mutation/creation are rejected.
impl Notifications for CapturedOwner {
    type Object = ObjectId;
    type Cursor = usize;
    type Manager = ();
    type Configuration = ();
    type Collection = ();
    type Definitions = ();
    unsupported!(configuration() -> (), "actual manager/configuration capture");
    unsupported!(definitions() -> (), "actual ordered definition inventory");
    unsupported!(collection(manager: (), owner: i32, edit: bool) -> (), "actual manager/owner collection binding");
    unsupported!(accepts_notifications(manager: (), owner: i32) -> bool, "actual manager/owner gates");
    unsupported!(definition_count(definitions: ()) -> usize, "actual ordered definition inventory");
    unsupported!(definition_is_auto(definitions: (), index: usize) -> bool, "actual auto-notify definition");
    unsupported!(definition_hash(definitions: (), index: usize) -> u32, "actual notification definition type");
    unsupported!(create_auto(manager: (), hash: u32, owner: i32) -> Option<ObjectId>, "actual registered factory and subtype Create");
    unsupported!(redundant(collection: (), object: ObjectId) -> bool, "actual redundancy/subtype duplicate rules");
    unsupported!(release(object: ObjectId) -> (), "unlisted factory temporary ownership");
    unsupported!(send(object: ObjectId) -> (), "actual notification send/subscribers");
    unsupported!(add(collection: (), object: ObjectId) -> (), "actual notification Add/identity assignment");
    unsupported!(play_by_cloud(configuration: ()) -> bool, "actual captured configuration");
    unsupported!(manager_game_local_player(manager: ()) -> Option<i32>, "actual manager game/local player");
    unsupported!(owner_turn_active(owner: i32) -> bool, "actual owner turn-active state");
    unsupported!(network_multiplayer(configuration: ()) -> bool, "actual retained configuration");
    unsupported!(find_auto_activate(collection: (), threshold: i32) -> Option<ObjectId>, "actual network/definition selection inputs");
    fn locked(&mut self, object: ObjectId) -> Result<bool, Error> {
        Ok(self.state.live[self.index(object)?].lock_count > 0)
    }
    unsupported!(activate(object: ObjectId, argument: bool) -> (), "bound synchronous reporting and subtype activation");
    fn begin(&mut self, _: ()) -> Result<usize, Error> {
        Ok(0)
    }
    fn end(&mut self, _: ()) -> Result<usize, Error> {
        Ok(self.state.live.len())
    }
    fn object(&mut self, cursor: usize) -> Result<ObjectId, Error> {
        self.state
            .live
            .get(cursor)
            .map(|r| r.object)
            .ok_or(Error::Unsupported("captured node cursor"))
    }
    fn next(&mut self, cursor: usize) -> Result<usize, Error> {
        if cursor >= self.state.live.len() {
            return Err(Error::Unsupported("captured end increment"));
        }
        Ok(cursor + 1)
    }
    fn dismissed(&mut self, object: ObjectId) -> Result<bool, Error> {
        Ok(self.state.notifications.records[self.index(object)?].is_dismissed)
    }
    fn expired(&mut self, object: ObjectId) -> Result<bool, Error> {
        Self::expired(self, object)
    }
    unsupported!(dismiss(object: ObjectId) -> (), "bound synchronous dismissal report 9ad1c020");
    unsupported!(cleanup(collection: ()) -> (), "predicate adapter is read-only; use lifecycle cleanup with its ownership contract");
    fn valid_for_phase(&mut self, object: ObjectId) -> Result<bool, Error> {
        Self::valid_for_phase(self, object)
    }
    fn blocking(&mut self, object: ObjectId) -> Result<i32, Error> {
        Self::blocking(self, object)
    }
    fn severity(&mut self, object: ObjectId) -> Result<i32, Error> {
        Ok(self.state.notifications.records[self.index(object)?].severity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notification_lifecycle::{
        ActivationBinding, DefinitionIcon, DestructionBinding, LiveNotification, ReleaseBinding,
    };
    use crate::notifications::{Notification, PlayerNotifications};

    fn owner(
        table: u64,
        record: Notification,
        blocking: Option<i32>,
        turn: Option<i32>,
    ) -> CapturedOwner {
        CapturedOwner::new(
            lifecycle::State {
                notifications: PlayerNotifications {
                    owner: 0,
                    next_id: 1,
                    records: vec![record],
                },
                live: vec![LiveNotification {
                    object: ObjectId(1),
                    lock_count: 0,
                    pool_binding: None,
                    definition_icon: DefinitionIcon::Unobserved,
                    activation: ActivationBinding::Unsupported,
                    release: ReleaseBinding::Unsupported,
                    destruction: DestructionBinding::Unsupported,
                }],
                pools: vec![],
            },
            vec![LivePredicates {
                object: ObjectId(1),
                vtable_addresspoint: table,
                default_blocking: blocking,
            }],
            turn,
        )
        .unwrap()
    }

    #[test]
    fn captured_base_and_pride_profiles_preserve_subtype_identity() {
        for address in [0x3af6238, 0x3af6530] {
            let mut p = owner(address, Notification::default(), Some(4), None);
            assert_eq!(p.valid_for_phase(ObjectId(1)), Ok(true));
            assert_eq!(p.is_needed(ObjectId(1)), Ok(true));
            assert_eq!(p.expired(ObjectId(1)), Ok(false));
            assert_eq!(p.blocking_reduction(), Ok(4));
            assert_eq!(p.predicates()[0].vtable_addresspoint, address);
        }
    }

    #[test]
    fn actual_needed_false_does_not_invoke_phase_validity() {
        let record = Notification {
            native_19a: true,
            native_188: 7,
            native_18c: 2,
            ..Notification::default()
        };
        let mut p = owner(0x3af6238, record, Some(4), Some(9));
        assert_eq!(p.valid_for_phase(ObjectId(1)), Ok(true));
        assert_eq!(p.is_needed(ObjectId(1)), Ok(false));
        assert_eq!(p.blocking(ObjectId(1)), Ok(0));
        assert_eq!(p.blocking_reduction(), Ok(0));
    }

    #[test]
    fn expiry_requires_observed_turn_only_after_auto_expires_short_circuit() {
        let mut p = owner(
            0x3af6238,
            Notification {
                native_18c: 1,
                native_188: i32::MAX,
                ..Notification::default()
            },
            Some(0),
            None,
        );
        assert_eq!(
            p.expired(ObjectId(1)),
            Err(Error::Unsupported("actual current game turn"))
        );
        // Even a zero blocking enum must evaluate expiry first.
        assert_eq!(
            p.blocking(ObjectId(1)),
            Err(Error::Unsupported("actual current game turn"))
        );
        // IsNeeded's flag false avoids expiration, independently of phase-valid.
        assert_eq!(p.is_needed(ObjectId(1)), Ok(true));
        p.current_turn = Some(i32::MIN);
        assert_eq!(p.expired(ObjectId(1)), Ok(true));
    }

    #[test]
    fn dismissed_short_circuit_avoids_unknown_subtype_expiration_and_blocking_input() {
        let p = owner(
            0x3af6600,
            Notification {
                is_dismissed: true,
                ..Notification::default()
            },
            None,
            None,
        );
        assert_eq!(p.blocking(ObjectId(1)), Ok(0));
        assert_eq!(
            p.expired(ObjectId(1)),
            Err(Error::Unsupported("concrete subtype expiration"))
        );
        assert_eq!(
            p.valid_for_phase(ObjectId(1)),
            Err(Error::Unsupported("concrete subtype phase predicate"))
        );
    }

    #[test]
    fn missing_live_blocking_is_not_defaulted_and_unsupported_effects_do_not_mutate() {
        let mut p = owner(0x3af6238, Notification::default(), None, None);
        assert_eq!(
            p.blocking(ObjectId(1)),
            Err(Error::Unsupported("actual live default blocking +0x130"))
        );
        let before = p.clone();
        assert!(Notifications::dismiss(&mut p, ObjectId(1)).is_err());
        assert!(Notifications::activate(&mut p, ObjectId(1), false).is_err());
        assert!(aftermath::update_notifications(&mut p, (), 0, &mut 10).is_err());
        assert_eq!(p, before);
    }

    #[test]
    fn subtype_override_is_never_replaced_with_base_expiry() {
        let p = owner(
            0x3af6600,
            Notification {
                native_18c: 0,
                ..Notification::default()
            },
            Some(0),
            Some(9),
        );
        assert_eq!(
            p.expired(ObjectId(1)),
            Err(Error::Unsupported("concrete subtype expiration"))
        );
        assert_eq!(p.is_needed(ObjectId(1)), Ok(true));
    }
}
