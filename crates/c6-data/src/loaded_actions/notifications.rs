//! Actual activated notification definitions required by post-Add upkeep.
//! This closes the ordered definition input, not creator registration, live
//! notification locks/pools, subtype effects or a complete Manager::Update.
use super::resources::Bound;
use crate::{
    registry::LinuxDefinitionRegistry, unit_initialization_definitions, Rules, RulesRowId, SqlValue,
};

#[derive(Debug)]
pub enum Error {
    Resources(unit_initialization_definitions::Error),
    Rules(crate::Error),
    MissingTable,
    Field(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "notification definitions: {self:?}")
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
