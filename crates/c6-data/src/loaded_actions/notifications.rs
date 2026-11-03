//! Activated definitions and attributed saved notification ownership for upkeep.
//! Live omitted fields, creator authority, subtype effects and complete
//! Manager::Update remain explicit dependencies.
use super::resources::Bound;
use crate::{
    registry::LinuxDefinitionRegistry, unit_initialization_definitions, Rules, RulesRowId, SqlValue,
};
use c6_core::{
    core_player::initialization::WireRole,
    loaded_state::{ComponentSnapshotWitness, Error as StateError, LoadedState},
    notifications::{Limits, PlayerNotifications},
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum Error {
    Resources(unit_initialization_definitions::Error),
    Rules(crate::Error),
    MissingTable,
    Field(&'static str),
    Unsupported(&'static str),
    Attribution,
    Codec(c6_core::notifications::Error),
    State(StateError),
    Owner(i32),
}

const SNAPSHOT: &str = "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884";
const OWNER_TRACE: &str = "528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae";
const OWNER_SPAN: &str = "68c99c43d65778dea5f1d49a7215d9d925b405f639d413b28e2c4f5d6f601094";

/// Install the once-attributed saved owner into the authoritative LoadedState.
/// Source Read and live-runtime authority stay distinct. No locks, subtype
/// bindings, initialized flags or callback absence are inferred from Save.
pub fn install_attributed_snapshot(
    state: &mut LoadedState,
    trace: &[u8],
    limits: Limits,
) -> Result<(), Error> {
    if !state.is_pristine() {
        return Err(Error::State(StateError::NotPristine));
    }
    let (witness, collections) = captured_collections(state, trace, limits)?;
    state
        .install_player_notifications(witness, collections)
        .map_err(Error::State)
}

/// Decode all 64 independently attributed owner frames. The Manager creates
/// 64 slots, including those absent from the loaded 43-Player roster. This is
/// admission work only; no trace scan or source-image hashing belongs on a query
/// or action hot path. Omitted runtime fields are not defaulted by this decode.
pub(crate) fn captured_collections(
    state: &LoadedState,
    trace: &[u8],
    limits: Limits,
) -> Result<(ComponentSnapshotWitness, Vec<PlayerNotifications>), Error> {
    let bytes = state.source_image();
    let matches = |bytes: &[u8], expected: &str| format!("{:x}", Sha256::digest(bytes)) == expected;
    let snapshot_digest = Sha256::digest(bytes);
    let trace_digest = Sha256::digest(trace);
    if bytes.len() != 80_657_462
        || format!("{snapshot_digest:x}") != SNAPSHOT
        || format!("{trace_digest:x}") != OWNER_TRACE
        || state.binding().core_sha256 != <[u8; 32]>::from(snapshot_digest)
    {
        return Err(Error::Attribution);
    }
    let mut frames = Vec::with_capacity(64);
    for line in trace.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let row: serde_json::Value =
            serde_json::from_slice(line).map_err(|_| Error::Attribution)?;
        if row["event"] == "owner"
            && row["save"] == 1
            && row["owner"] == "GameCore::Reporting::Notification::PlayerNotifications"
        {
            let index = frames.len();
            if index >= 64
                || row["depth"] != 1
                || row["parent"] != 1
                || row["call"] != 201876 + index
            {
                return Err(Error::Attribution);
            }
            let offset = |name: &str| {
                row[name]
                    .as_u64()
                    .and_then(|x| usize::try_from(x).ok())
                    .ok_or(Error::Attribution)
            };
            let start = offset("start")?;
            let end = offset("end")?;
            if end <= start
                || frames
                    .last()
                    .is_some_and(|previous: &[usize; 2]| previous[1] != start)
            {
                return Err(Error::Attribution);
            }
            frames.push([start, end]);
        }
    }
    if frames.len() != 64 || frames[0][0] != 76_400_879 || frames[63][1] != 76_405_580 {
        return Err(Error::Attribution);
    }
    let range = [frames[0][0] - 8, frames[63][1]];
    let span = bytes.get(range[0]..range[1]).ok_or(Error::Attribution)?;
    if span.len() > limits.max_bytes {
        return Err(Error::Codec(c6_core::notifications::Error::ResourceLimit));
    }
    if !matches(span, OWNER_SPAN) || span[..8] != [1, 0, 0, 0, 64, 0, 0, 0] {
        return Err(Error::Attribution);
    }
    let mut collections = Vec::with_capacity(64);
    let mut records = 0usize;
    for (slot, frame) in frames.into_iter().enumerate() {
        let owner_bytes = bytes.get(frame[0]..frame[1]).ok_or(Error::Attribution)?;
        let owner = PlayerNotifications::decode(owner_bytes, limits).map_err(Error::Codec)?;
        records = records
            .checked_add(owner.records.len())
            .ok_or(Error::Attribution)?;
        if records > limits.max_records {
            return Err(Error::Codec(c6_core::notifications::Error::ResourceLimit));
        }
        if owner.owner != slot as i32 {
            return Err(Error::Attribution);
        }
        collections.push(owner);
    }
    Ok((
        ComponentSnapshotWitness {
            source_role: WireRole::SavedOutputCore,
            snapshot_sha256: snapshot_digest.into(),
            snapshot_bytes: bytes.len(),
            owner_trace_sha256: trace_digest.into(),
            owner_range: range,
            decoded_prefix_bytes: span.len(),
        },
        collections,
    ))
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loaded notification inputs: {self:?}")
    }
}
impl std::error::Error for Error {}

/// The native builder's fourteen SQL columns, preserving nullable strings.
/// Database row identity is distinct from the sequential collection index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    pub row_id: RulesRowId,
    pub type_hash: u32,
    pub notification_type: String,
    pub auto_activate: bool,
    pub auto_notify: bool,
    pub expires_end_of_next_turn: bool,
    pub expires_end_of_turn: bool,
    pub group_type: Option<String>,
    pub icon: Option<String>,
    pub message: Option<String>,
    pub severity_type: Option<String>,
    pub show_icon_single_player: bool,
    pub sub_type: Option<String>,
    pub summary: Option<String>,
    pub visible_in_ui: bool,
}
pub struct Definitions {
    ordered: Vec<Definition>,
    registry: LinuxDefinitionRegistry,
}
impl Definitions {
    /// Source: NotificationCollectionBuilder::GetBuilderSql 0x2204e70:
    /// SELECT rowid, ... FROM Notifications ORDER BY rowid. BuildDefinition
    /// 0x2204bb0 reads AutoActivate at column1 and AutoNotify at column2.
    /// This uses the activated DB after actual static-resource equivalence;
    /// base XML does not establish the enabled collection or iteration order.
    pub fn from_rules(rules: &Rules, bound: &Bound<'_>, report: &[u8]) -> Result<Self, Error> {
        unit_initialization_definitions::Definitions::validate_resources(rules, bound, report)
            .map_err(Error::Resources)?;
        let registry =
            unit_initialization_definitions::collection(rules, "Notifications", "NotificationType")
                .map_err(Error::Resources)?;
        let table = rules
            .gameplay()
            .table("Notifications")
            .ok_or(Error::MissingTable)?;
        let mut ordered = Vec::with_capacity(table.rows().len());
        for row in table.rows() {
            let get = |name| row.get(name).map_err(Error::Rules);
            let boolean = |name| match get(name)? {
                SqlValue::Integer(0) => Ok(false),
                SqlValue::Integer(1) => Ok(true),
                _ => Err(Error::Field(name)),
            };
            let text = |name| match get(name)? {
                SqlValue::Null => Ok(None),
                SqlValue::Text(value) if !value.contains(&0) => String::from_utf8(value.clone())
                    .map(Some)
                    .map_err(|_| Error::Field(name)),
                _ => Err(Error::Field(name)),
            };
            let notification_type =
                text("NotificationType")?.ok_or(Error::Field("NotificationType"))?;
            ordered.push(Definition {
                row_id: row.row_id().ok_or(Error::Field("rowid"))?,
                type_hash: crate::registry::linux_type_hash(notification_type.as_bytes()),
                notification_type,
                auto_activate: boolean("AutoActivate")?,
                auto_notify: boolean("AutoNotify")?,
                expires_end_of_next_turn: boolean("ExpiresEndOfNextTurn")?,
                expires_end_of_turn: boolean("ExpiresEndOfTurn")?,
                group_type: text("GroupType")?,
                icon: text("Icon")?,
                message: text("Message")?,
                severity_type: text("SeverityType")?,
                show_icon_single_player: boolean("ShowIconSinglePlayer")?,
                sub_type: text("SubType")?,
                summary: text("Summary")?,
                visible_in_ui: boolean("VisibleInUI")?,
            });
        }
        Ok(Self { ordered, registry })
    }
    pub fn ordered(&self) -> &[Definition] {
        &self.ordered
    }
    /// Native direct-index priority and duplicate-hash precedence retained.
    pub fn find(&self, hash: u32) -> Option<&Definition> {
        self.registry
            .find_index(hash)
            .and_then(|i| self.ordered.get(i))
    }
    /// SendAutoNotify reads the flag before reading the hash. Consumers should
    /// iterate ordered(), checking auto_notify at its actual short-circuit point.
    pub fn auto_notify_count(&self) -> usize {
        self.ordered.iter().filter(|d| d.auto_notify).count()
    }
}

fn player_header(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::core_player::PlayerHeader, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|player| player.header.owner == owner)
        .map(|player| &player.header)
        .ok_or(Error::Owner(owner))
}

/// Exact saved/current cache used by Processor::SetEndTurnBlockingState.
/// Write0x2b4ef15 and Read0x2b4f860 both address Player+0x7a8: this is the
/// fifteenth ordered header scalar, not an inferred initialized zero.
pub fn owner_blocking(state: &LoadedState, owner: i32) -> Result<i32, Error> {
    state.player_end_turn_blocking(owner).map_err(Error::State)
}

/// Source SetEndTurnBlockingState ordering over the actual owned field. The
/// required global reporting backend sees the already-stored value and this
/// same LoadedState. A backend error preserves that store and any callback
/// effects; no rollback or callback absence is implied. No event is delivered
/// for equality or a different current local player.
pub fn set_owner_blocking<F>(
    state: &mut LoadedState,
    owner: i32,
    value: i32,
    reporting: &mut F,
) -> Result<(), Error>
where
    F: FnMut(&mut LoadedState, u32, i32, i32) -> Result<(), Error>,
{
    c6_core::unit_operation_aftermath::set_owner_blocking_with(
        state,
        owner,
        value,
        |state, owner| owner_blocking(state, owner),
        |state, owner, value| {
            state
                .store_player_end_turn_blocking(owner, value)
                .map(|_| ())
                .map_err(Error::State)
        },
        |state, owner| Ok(player_header(state, owner)?.owner),
        |state| Ok(state.retained().core.local_player),
        reporting,
    )
}

/// Processor::IsTurnComplete0x2b5bb60 reads the fifth serialized processor
/// boolean array, ms_abIsTurnComplete. It does not read a Player header flag.
pub fn turn_complete(state: &LoadedState, owner: i32) -> Result<bool, Error> {
    let owner = player_header(state, owner)?.owner;
    let index = usize::try_from(owner).map_err(|_| Error::Owner(owner))?;
    state
        .retained()
        .core
        .player_processor
        .player_complete
        .get(index)
        .copied()
        .ok_or(Error::Owner(owner))
}

/// Processor::PreviouslyActiveThisTurn0x2b5bbc0 checks the activation-event
/// array first. The actual current IsTurnActive input is required only when
/// that flag is false, preserving the native short circuit.
pub fn previously_active_this_turn(state: &LoadedState, owner: i32) -> Result<bool, Error> {
    let owner = player_header(state, owner)?.owner;
    let index = usize::try_from(owner).map_err(|_| Error::Owner(owner))?;
    let event = state
        .retained()
        .core
        .player_processor
        .activation_event
        .get(index)
        .copied()
        .ok_or(Error::Owner(owner))?;
    if event {
        return Ok(false);
    }
    Ok(!state
        .player_controls(owner)
        .map_err(Error::State)?
        .turn_active)
}

/// Source leaf used by the inherited duplicate slot of ChooseTech,
/// ChooseCivic, FillCivicSlot and ConsiderGovernmentChange. This is only the
/// auto=true/parameters=null creation path: IsNeeded and the receiver's actual
/// virtual binding must be established by the caller before invoking it.
/// It does not inspect either object's own ID, so a recycled pre-Add ID cannot
/// affect this supported duplicate comparison.
pub fn base_auto_duplicate(
    candidate: &c6_core::notifications::Notification,
    existing: &c6_core::notifications::Notification,
) -> Result<bool, Error> {
    if !candidate.parameters.entries.is_empty() {
        return Err(Error::Unsupported(
            "base auto duplicate requires null creation parameters",
        ));
    }
    if candidate.type_hash != existing.type_hash
        || candidate.location != existing.location
        || candidate.component != existing.component
    {
        return Ok(false);
    }
    // Instance::IsDuplicate0x2d2e2d0: the empty candidate has neither
    // 0x42fc7f19 nor 0x55c7a3f8. The latter matches only when both are absent;
    // native never converts the existing value if just one is present.
    Ok(!existing
        .parameters
        .entries
        .iter()
        .any(|entry| entry.key == 0x55c7a3f8))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supported_duplicate_ignores_recycled_own_id_but_preserves_target_identity() {
        let candidate = c6_core::notifications::Notification {
            type_hash: 0x592df4c8,
            ..Default::default()
        };
        let mut existing = candidate.clone();
        existing.id.id = 123;
        existing.id.owner = 11;
        assert!(base_auto_duplicate(&candidate, &existing).unwrap());
        existing.component.id = 123;
        assert!(!base_auto_duplicate(&candidate, &existing).unwrap());
        existing.component = candidate.component;
        existing.location[0] = 1;
        assert!(!base_auto_duplicate(&candidate, &existing).unwrap());
    }
}
