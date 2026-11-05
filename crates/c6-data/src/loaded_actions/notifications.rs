//! Activated definitions and attributed saved notification ownership for upkeep.
//! Live omitted fields, creator authority, subtype effects and complete
//! Manager::Update remain explicit dependencies.
pub mod allocation;
mod borrowed;
pub mod initialization;
pub mod lifecycle;
pub mod processing;
pub mod text;

use crate::resource_binding::ResourceAuthority;
use crate::{
    registry::LinuxDefinitionRegistry, unit_initialization_definitions, Rules, RulesRowId, SqlValue,
};
use c6_core::loaded_state::{Error as StateError, LoadedState};

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
    pub fn from_rules(rules: &Rules, bound: &ResourceAuthority<'_>) -> Result<Self, Error> {
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
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
    Ok(!state.turn_active(owner).map_err(Error::State)?)
}

/// Source leaf used by the inherited duplicate slot of ChooseTech,
/// ChooseCivic, FillCivicSlot and ConsiderGovernmentChange. This is only the
/// parameters=null creation paths (manual or auto): IsNeeded and the receiver's actual
/// virtual binding must be established by the caller before invoking it.
/// It does not inspect either object's own ID, so a recycled pre-Add ID cannot
/// affect this supported duplicate comparison.
pub fn base_auto_duplicate(
    candidate: &c6_core::notifications::Notification,
    existing: &c6_core::notifications::Notification,
) -> Result<bool, Error> {
    if !candidate.parameters.entries.is_empty() {
        return Err(Error::Unsupported(
            "base duplicate requires null creation parameters",
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

use c6_core::notification_lifecycle::{self as core_lifecycle, ObjectId, PoolId};
use c6_core::notifications::{ComponentId, LocalizedString, Notification};

pub enum LogicalAllocation<'a> {
    Fresh(ObjectId),
    Reused {
        object: ObjectId,
        prior: &'a Notification,
    },
    Failed,
}
pub struct Created {
    pub record: Notification,
    pub live: core_lifecycle::LiveNotification,
    pub default_blocking: i32,
    /// Native +0x18 is not present in the wire Notification record.
    pub group_hash: u32,
    pub vtable_addresspoint: u64,
}
/// Exact base Initialize for auto=true/null parameters. Manager lifecycle and
/// creator dispatch remain the caller's source-derived responsibility. Subtype
/// stores happen separately so a failing query retains its partially initialized
/// authoritative candidate. Returned runtime describes source initialization,
/// not an observation of a native live object.
pub fn initialize_auto_base(
    definitions: &Definitions,
    type_hash: u32,
    vtable_addresspoint: u64,
    allocation: LogicalAllocation<'_>,
    pool: PoolId,
    owner: i32,
) -> Result<Created, Error> {
    let definition = definitions
        .find(type_hash)
        .ok_or(Error::Unsupported("auto notification definition"))?;
    if definition.expires_end_of_turn || definition.expires_end_of_next_turn {
        return Err(Error::Unsupported(
            "base Initialize PreviouslyActiveThisTurn dependency",
        ));
    }
    let (object, mut id, component_type) = match allocation {
        LogicalAllocation::Fresh(object) => (
            object,
            ComponentId {
                owner: -1,
                component_type: 0x26,
                id: u32::MAX,
            },
            0,
        ),
        LogicalAllocation::Reused { object, prior } => {
            if prior.factory_type != type_hash
                || prior.type_hash != type_hash
                || !prior.parameters.entries.is_empty()
            {
                return Err(Error::Unsupported(
                    "reused notification identity/parameter destruction",
                ));
            }
            (object, prior.id, prior.component.component_type)
        }
        LogicalAllocation::Failed => {
            return Err(Error::Unsupported("notification allocation failure"))
        }
    };
    if owner != -1 {
        id.owner = owner as i16;
    }
    let severity = definition
        .severity_type
        .as_deref()
        .map(|name| severity_type(crate::registry::linux_type_hash(name.as_bytes())))
        .filter(|v| *v != -1)
        .unwrap_or(0);
    let record = Notification {
        factory_type: type_hash,
        id,
        type_hash,
        severity,
        message: LocalizedString::default(),
        summary: LocalizedString::default(),
        location: [-9999; 2],
        component: ComponentId {
            owner: -1,
            component_type,
            id: u32::MAX,
        },
        native_188: -1,
        native_18c: 0,
        is_dismissed: false,
        native_199: true,
        native_19a: false,
        native_190: 1,
        native_19b: true,
        native_19c: false,
        parameters: Default::default(),
        native_19d: definition.auto_activate,
        has_been_activated: false,
    };
    Ok(Created {
        record,
        live: core_lifecycle::LiveNotification {
            object,
            lock_count: 0,
            pool_binding: Some(pool),
            definition_icon: core_lifecycle::DefinitionIcon::ShowInSinglePlayer(
                definition.show_icon_single_player,
            ),
            activation: core_lifecycle::ActivationBinding::Base,
            release: core_lifecycle::ReleaseBinding::Base,
            destruction: core_lifecycle::DestructionBinding::BaseResources,
        },
        default_blocking: 0,
        group_hash: definition
            .group_type
            .as_deref()
            .map(|name| crate::registry::linux_type_hash(name.as_bytes()))
            .unwrap_or(0xb6fa_1284),
        vtable_addresspoint,
    })
}
fn severity_type(hash: u32) -> i32 {
    match hash {
        0xfbd0_c5d4 => 0,
        0x28f6_49bb => 1,
        0x386f_f61e => 2,
        0xba9f_3062 => 3,
        0x6e89_7878 => 4,
        0..5 => hash as i32,
        _ => -1,
    }
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
