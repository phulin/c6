//! Immutable, identity-gated rules inputs with explicit native binary profiles.
//! This is a data boundary, not an initialized game or gameplay implementation.
use rusqlite::{types::ValueRef, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fmt, fs, path::Path};

pub mod city_initialization;
pub mod city_power;
pub mod city_movement;
pub mod city_turn;
pub mod city_production_turn;
pub mod combat_defender;
pub mod formation_rules;
pub mod formation_movement_cost;
pub mod formation_move_to;
pub mod formation_path_state;
pub mod game_relations;
pub mod game_parameters;
pub mod loaded_actions;
pub mod loaded_upkeep;
pub mod melee_attack;
pub mod movement;
pub mod movement_placement_state;
pub mod registry;
pub mod research_unit_costs;
pub mod research;
pub mod resource_binding;
pub mod unit_initialization_definitions;
pub mod unit_movement_state;
pub mod unit_moved_requirements;
pub mod unit_operation_manager;
pub mod unit_sight_state;
pub mod unit_moved_achievement;
pub mod world_congress_options;
pub mod world_congress_rules;

const ENCODING: &str = "c6.sqlite-typed-rowid.v1";
// Registered production descriptors retain identity and complete database fingerprints.
// Investigator reports and export histories are not build inputs.
const LINUX_BASE_MANIFEST: &str = include_str!("../profiles/linux-xp2-base.json");
const LINUX_XP2_DIAGNOSTIC_MANIFEST: &str =
    include_str!("../profiles/linux-xp2-activated-diagnostic.json");
const WINDOWS_HISTORICAL_MANIFEST: &str = include_str!("../profiles/windows-xp2-historical.json");

type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self(format!("SQLite: {e}"))
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self(e.to_string())
    }
}
fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(Error(message.into()))
}

/// SQLite row identity only. No mapping to GameCore DefinitionID is established.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RulesRowId(pub i64);

/// REAL stores returned binary64 bits, including signed zero. TEXT remains bytes;
/// decoding to UTF-8 is explicit and fails closed, never replacement-decodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SqlValue {
    Null,
    Integer(i64),
    RealBits(u64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}
impl SqlValue {
    pub fn integer(&self) -> Result<i64> {
        match self {
            Self::Integer(n) => Ok(*n),
            _ => fail("expected SQL INTEGER"),
        }
    }
    pub fn text(&self) -> Result<&str> {
        match self {
            Self::Text(bytes) => {
                std::str::from_utf8(bytes).map_err(|e| Error(format!("invalid UTF-8 TEXT: {e}")))
            }
            _ => fail("expected SQL TEXT"),
        }
    }
    pub fn real_bits(&self) -> Result<u64> {
        match self {
            Self::RealBits(bits) => Ok(*bits),
            _ => fail("expected SQL REAL"),
        }
    }
    fn feed(&self, hash: &mut Sha256) {
        match self {
            Self::Null => hash.update(b"N"),
            Self::Integer(n) => {
                hash.update(b"I");
                hash.update(n.to_be_bytes());
            }
            Self::RealBits(n) => {
                hash.update(b"R");
                hash.update(n.to_be_bytes());
            }
            Self::Text(bytes) | Self::Blob(bytes) => {
                hash.update(if matches!(self, Self::Text(_)) {
                    b"T"
                } else {
                    b"B"
                });
                hash.update((bytes.len() as u64).to_be_bytes());
                hash.update(bytes);
            }
        }
    }
}
fn rows_digest(rows: &[Vec<SqlValue>]) -> String {
    let mut hash = Sha256::new();
    hash.update(ENCODING);
    hash.update([0]);
    for row in rows {
        hash.update(b"A");
        hash.update((row.len() as u64).to_be_bytes());
        for value in row {
            value.feed(&mut hash);
        }
    }
    format!("{:x}", hash.finalize())
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
fn query(connection: &Connection, sql: &str) -> Result<Vec<Vec<SqlValue>>> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|e| Error(format!("{sql}: {e}")))?;
    let count = statement.column_count();
    let mut cursor = statement.query([])?;
    let mut rows = Vec::new();
    while let Some(row) = cursor.next()? {
        let mut values = Vec::with_capacity(count);
        for column in 0..count {
            values.push(match row.get_ref(column)? {
                ValueRef::Null => SqlValue::Null,
                ValueRef::Integer(n) => SqlValue::Integer(n),
                ValueRef::Real(n) => SqlValue::RealBits(n.to_bits()),
                ValueRef::Text(bytes) => SqlValue::Text(bytes.to_vec()),
                ValueRef::Blob(bytes) => SqlValue::Blob(bytes.to_vec()),
            });
        }
        rows.push(values);
    }
    Ok(rows)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum RowIdentity {
    #[serde(rename = "intrinsic_rowid")]
    IntrinsicRowid { alias: String },
    #[serde(rename = "without_rowid_primary_key")]
    WithoutRowidPrimaryKey { columns: Vec<String> },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TableFingerprint {
    pub name: String,
    pub schema_rowid: i64,
    pub rows: usize,
    pub row_identity: RowIdentity,
    pub columns_sha256: String,
    pub typed_rows_sha256: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticFingerprint {
    pub encoding: String,
    pub schema_sha256: String,
    pub schema_objects: usize,
    pub persistent_pragmas: Value,
    pub tables: Vec<TableFingerprint>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseFingerprint {
    pub filename: String,
    pub bytes: u64,
    pub file_sha256: String,
    pub semantic_sha256: String,
    pub semantic: SemanticFingerprint,
    pub table_count: usize,
    pub row_count: usize,
}

/// Full sqlite_schema records, including index/trigger/view DDL and creation rowid.
#[derive(Debug)]
pub struct SchemaObject {
    values: Vec<SqlValue>,
}
impl SchemaObject {
    pub fn values(&self) -> &[SqlValue] {
        &self.values
    }
}
#[derive(Debug)]
pub struct Table {
    fingerprint: TableFingerprint,
    // All table_xinfo records, including defaults, declared types and hidden flags.
    column_metadata: Vec<Vec<SqlValue>>,
    columns: Vec<String>,
    column_index: HashMap<String, usize>,
    // Intrinsic rowid is first only for rowid tables, exactly as canonical stream.
    rows: Vec<Vec<SqlValue>>,
    rowid_index: HashMap<RulesRowId, usize>,
}
impl Table {
    pub fn fingerprint(&self) -> &TableFingerprint {
        &self.fingerprint
    }
    pub fn columns(&self) -> &[String] {
        &self.columns
    }
    pub fn column_metadata(&self) -> &[Vec<SqlValue>] {
        &self.column_metadata
    }
    /// Canonical fingerprint order (intrinsic rowid, or explicit primary key).
    /// This does not promise native unordered-query or DefinitionRepository order.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = Row<'_>> {
        (0..self.rows.len()).map(|index| Row { table: self, index })
    }
    pub fn row_by_id(&self, id: RulesRowId) -> Option<Row<'_>> {
        self.rowid_index
            .get(&id)
            .map(|&index| Row { table: self, index })
    }
    /// Build once, then lookup without SQL or per-lookup allocation. Reject NULL,
    /// non-TEXT, invalid UTF-8 and duplicate keys with table/row/column diagnostics.
    pub fn text_index(&self, column: &str) -> Result<TextIndex<'_>> {
        let mut index = HashMap::with_capacity(self.rows.len());
        for row in self.rows() {
            let key = row.text(column)?;
            if index.insert(key, row.index).is_some() {
                return fail(format!(
                    "table {} row {} column {column}: duplicate TEXT key {key:?}",
                    self.fingerprint.name, row.index
                ));
            }
        }
        Ok(TextIndex { table: self, index })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Row<'a> {
    table: &'a Table,
    index: usize,
}
impl<'a> Row<'a> {
    pub fn row_id(self) -> Option<RulesRowId> {
        match self.table.fingerprint.row_identity {
            RowIdentity::IntrinsicRowid { .. } => match self.table.rows[self.index][0] {
                SqlValue::Integer(id) => Some(RulesRowId(id)),
                _ => unreachable!(),
            },
            RowIdentity::WithoutRowidPrimaryKey { .. } => None,
        }
    }
    /// All SELECT * fields, excluding the prepended intrinsic rowid.
    pub fn values(self) -> &'a [SqlValue] {
        let offset = usize::from(matches!(
            self.table.fingerprint.row_identity,
            RowIdentity::IntrinsicRowid { .. }
        ));
        &self.table.rows[self.index][offset..]
    }
    pub fn get(self, column: &str) -> Result<&'a SqlValue> {
        self.table
            .column_index
            .get(column)
            .map(|&index| &self.values()[index])
            .ok_or_else(|| {
                Error(format!(
                    "table {} row {} column {column}: missing column",
                    self.table.fingerprint.name, self.index
                ))
            })
    }
    pub fn integer(self, column: &str) -> Result<i64> {
        self.get(column)?
            .integer()
            .map_err(|e| self.context(column, e))
    }
    pub fn text(self, column: &str) -> Result<&'a str> {
        self.get(column)?
            .text()
            .map_err(|e| self.context(column, e))
    }
    fn context(self, column: &str, e: Error) -> Error {
        Error(format!(
            "table {} row {} rowid {:?} column {column}: {e}",
            self.table.fingerprint.name,
            self.index,
            self.row_id()
        ))
    }
}
pub struct TextIndex<'a> {
    table: &'a Table,
    index: HashMap<&'a str, usize>,
}
impl<'a> TextIndex<'a> {
    pub fn get(&self, key: &str) -> Option<Row<'a>> {
        self.index.get(key).map(|&index| Row {
            table: self.table,
            index,
        })
    }
}
#[derive(Debug)]
pub struct Database {
    fingerprint: DatabaseFingerprint,
    schema: Vec<SchemaObject>,
    tables: Vec<Table>,
    table_index: HashMap<String, usize>,
}
impl Database {
    pub fn fingerprint(&self) -> &DatabaseFingerprint {
        &self.fingerprint
    }
    pub fn schema(&self) -> &[SchemaObject] {
        &self.schema
    }
    /// Schema creation order, matching the authoritative fingerprint protocol.
    pub fn tables(&self) -> &[Table] {
        &self.tables
    }
    pub fn table(&self, name: &str) -> Option<&Table> {
        self.table_index.get(name).map(|&index| &self.tables[index])
    }
    /// Exact byte count of retained SQL TEXT/BLOB payloads, not total RSS/heap.
    pub fn sql_payload_bytes(&self) -> usize {
        self.schema
            .iter()
            .map(|o| &o.values)
            .chain(
                self.tables
                    .iter()
                    .flat_map(|t| t.column_metadata.iter().chain(&t.rows)),
            )
            .flat_map(|r| r.iter())
            .map(|v| match v {
                SqlValue::Text(b) | SqlValue::Blob(b) => b.len(),
                _ => 0,
            })
            .sum()
    }
}

fn inspect(path: &Path) -> Result<Database> {
    for suffix in ["-wal", "-journal"] {
        let mut sidecar = path.as_os_str().to_owned();
        sidecar.push(suffix);
        if Path::new(&sidecar).exists() {
            return fail(format!(
                "{}: external {suffix} state is unsupported",
                path.display()
            ));
        }
    }
    let before = fs::read(path)?;
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.execute_batch("PRAGMA query_only=ON;")?;
    if query(&connection, "PRAGMA integrity_check")? != vec![vec![SqlValue::Text(b"ok".to_vec())]] {
        return fail(format!("{}: integrity check failed", path.display()));
    }
    connection.execute_batch("BEGIN")?;
    let schema = query(
        &connection,
        "SELECT rowid,type,name,tbl_name,sql FROM sqlite_schema ORDER BY rowid",
    )?;
    let flags = query(&connection, "PRAGMA table_list")?;
    let mut tables = Vec::new();
    for object in &schema {
        if object[1].text()? != "table" {
            continue;
        }
        let name = object[2].text()?;
        let ddl = match &object[4] {
            SqlValue::Null => "",
            value => value.text()?,
        };
        if ddl.to_uppercase().starts_with("CREATE VIRTUAL TABLE") {
            return fail(format!(
                "table {name}: virtual table extension state unsupported"
            ));
        }
        let columns = query(&connection, &format!("PRAGMA table_xinfo({})", quote(name)))?;
        let flag = flags
            .iter()
            .find(|r| r[0].text().ok() == Some("main") && r[1].text().ok() == Some(name))
            .ok_or_else(|| {
                Error(format!(
                    "table {name}: missing required table_list metadata"
                ))
            })?;
        if columns.is_empty() {
            return fail(format!("table {name}: missing column metadata"));
        }
        if matches!(flag[2].text()?, "virtual" | "shadow") {
            return fail(format!(
                "table {name}: virtual/shadow extension state unsupported"
            ));
        }
        let without_rowid = flag[4].integer()? != 0;
        let (selected, order, row_identity) = if without_rowid {
            let mut primary = columns
                .iter()
                .filter_map(|r| match r[5].integer() {
                    Ok(n) if n != 0 => Some((n, r[1].text().map(str::to_owned))),
                    _ => None,
                })
                .collect::<Vec<_>>();
            primary.sort_by_key(|p| p.0);
            let names = primary
                .into_iter()
                .map(|(_, n)| n)
                .collect::<Result<Vec<_>>>()?;
            if names.is_empty() {
                return fail(format!("table {name}: WITHOUT ROWID lacks primary key"));
            }
            (
                "*".to_owned(),
                names.iter().map(|n| quote(n)).collect::<Vec<_>>().join(","),
                RowIdentity::WithoutRowidPrimaryKey { columns: names },
            )
        } else {
            let names = columns
                .iter()
                .map(|r| r[1].text().map(str::to_ascii_lowercase))
                .collect::<Result<Vec<_>>>()?;
            let alias = ["rowid", "_rowid_", "oid"]
                .into_iter()
                .find(|a| !names.iter().any(|n| n == a))
                .ok_or_else(|| {
                    Error(format!(
                        "table {name}: all intrinsic rowid aliases shadowed"
                    ))
                })?;
            (
                format!("{},*", quote(alias)),
                quote(alias),
                RowIdentity::IntrinsicRowid {
                    alias: alias.to_owned(),
                },
            )
        };
        let rows = query(
            &connection,
            &format!("SELECT {selected} FROM {} ORDER BY {order}", quote(name)),
        )?;
        let fingerprint = TableFingerprint {
            name: name.to_owned(),
            schema_rowid: object[0].integer()?,
            rows: rows.len(),
            row_identity,
            columns_sha256: rows_digest(&columns),
            typed_rows_sha256: rows_digest(&rows),
        };
        // table_xinfo hidden=1 is omitted by SELECT *; generated columns (2/3) remain.
        let selected_columns = columns
            .iter()
            .filter(|r| r[6] != SqlValue::Integer(1))
            .map(|r| r[1].text().map(str::to_owned))
            .collect::<Result<Vec<_>>>()?;
        let column_index = selected_columns
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i))
            .collect();
        let mut rowid_index = HashMap::new();
        if !without_rowid {
            for (index, row) in rows.iter().enumerate() {
                rowid_index.insert(RulesRowId(row[0].integer()?), index);
            }
        }
        tables.push(Table {
            fingerprint,
            columns: selected_columns,
            column_metadata: columns,
            column_index,
            rows,
            rowid_index,
        });
    }
    let mut persistent = serde_json::Map::new();
    for pragma in ["application_id", "user_version", "encoding"] {
        let rows = query(&connection, &format!("PRAGMA {pragma}"))?;
        persistent.insert(
            pragma.into(),
            match &rows[0][0] {
                SqlValue::Integer(n) => json!(n),
                value => json!(value.text()?),
            },
        );
    }
    connection.execute_batch("ROLLBACK")?;
    drop(connection);
    if fs::read(path)? != before {
        return fail(format!("{}: changed during read-only load", path.display()));
    }
    let semantic = SemanticFingerprint {
        encoding: ENCODING.into(),
        schema_sha256: rows_digest(&schema),
        schema_objects: schema.len(),
        persistent_pragmas: Value::Object(persistent),
        tables: tables.iter().map(|t| t.fingerprint.clone()).collect(),
    };
    // Conversion through Value sorts all object keys, matching Python canonical JSON.
    let semantic_sha256 = sha(&serde_json::to_vec(&serde_json::to_value(&semantic)?)?);
    let fingerprint = DatabaseFingerprint {
        filename: path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error("invalid UTF-8 filename".into()))?
            .into(),
        bytes: before.len() as u64,
        file_sha256: sha(&before),
        semantic_sha256,
        table_count: tables.len(),
        row_count: tables.iter().map(|t| t.rows.len()).sum(),
        semantic,
    };
    let table_index = tables
        .iter()
        .enumerate()
        .map(|(i, t)| (t.fingerprint.name.clone(), i))
        .collect();
    Ok(Database {
        fingerprint,
        schema: schema
            .into_iter()
            .map(|values| SchemaObject { values })
            .collect(),
        tables,
        table_index,
    })
}

/// Caller declares the complete source/profile before loading. Only registered
/// pinned native exports are accepted; profile selection never falls back.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub native_configuration_sha256: String,
    pub profile: Value,
    pub module_identities: Value,
    pub source_sequence_sha256: String,
    pub language_identity: Value,
}
/// Registered exact rules fixture for a native binary and complete input context.
/// Several profiles may share the same GameCore hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulesProfile {
    /// Historical Windows-native export coverage; never a Linux fallback.
    WindowsXp2Historical,
    /// Supplied Linux GameCore with original-native base inputs and no DLC/mod activation.
    LinuxXp2Base,
    /// Native activated XP2 export with LoadGameViewState constructor/requestNone
    /// callback policy. This is not a saved Begin or initialized gameplay session.
    LinuxXp2ActivatedDiagnostic,
}
impl RulesProfile {
    pub fn gamecore_sha256(self) -> &'static str {
        match self {
            Self::WindowsXp2Historical => {
                "324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c"
            }
            Self::LinuxXp2Base | Self::LinuxXp2ActivatedDiagnostic => {
                "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
            }
        }
    }
    /// Obtaining a descriptor does not load or validate files. Each descriptor is
    /// pinned to its own native export; no other profile's data is substituted.
    pub fn source_identity(self) -> Result<SourceIdentity> {
        Ok(profile_manifest(self)?.source)
    }
}
#[derive(Deserialize)]
struct Manifest {
    #[serde(flatten)]
    source: SourceIdentity,
    databases: Vec<DatabaseFingerprint>,
}
fn profile_manifest(profile: RulesProfile) -> Result<Manifest> {
    let text = match profile {
        RulesProfile::WindowsXp2Historical => WINDOWS_HISTORICAL_MANIFEST,
        RulesProfile::LinuxXp2Base => LINUX_BASE_MANIFEST,
        RulesProfile::LinuxXp2ActivatedDiagnostic => LINUX_XP2_DIAGNOSTIC_MANIFEST,
    };
    let manifest: Manifest = serde_json::from_str(text)?;
    if manifest.source.profile["target_sha256"].as_str() != Some(profile.gamecore_sha256()) {
        return fail("registered manifest GameCore hash differs from selected binary profile");
    }
    Ok(manifest)
}
/// Historical Windows-only identity. New callers should select a RulesProfile.
/// This function supplies no Linux coverage and is never used as a fallback.
pub fn xp2_source_identity() -> SourceIdentity {
    RulesProfile::WindowsXp2Historical
        .source_identity()
        .expect("checked-in historical manifest")
}

/// Typed original rules values, keyed by SQL UnitType. Values are not mechanics:
/// modifiers, player/state effects and native definition indexing remain unported.
#[derive(Debug)]
pub struct UnitDefinition {
    pub rules_row_id: RulesRowId,
    pub unit_type: String,
    pub cost: i64,
    pub base_moves: i64,
    pub combat: i64,
    pub ranged_combat: i64,
    pub range: i64,
    pub maintenance: i64,
}
#[derive(Debug)]
pub struct Rules {
    source: SourceIdentity,
    profile: RulesProfile,
    configuration: Database,
    gameplay: Database,
    units: Vec<UnitDefinition>,
    unit_index: HashMap<String, usize>,
}
impl Rules {
    pub fn load(directory: impl AsRef<Path>, declared: &SourceIdentity) -> Result<Self> {
        // A module hash alone cannot distinguish base from activated content.
        // Resolve only an exact complete registered descriptor before any file I/O.
        let mut selected = None;
        for profile in [
            RulesProfile::LinuxXp2Base,
            RulesProfile::LinuxXp2ActivatedDiagnostic,
            RulesProfile::WindowsXp2Historical,
        ] {
            if declared == &profile.source_identity()? && selected.replace(profile).is_some() {
                return fail("declared source/profile matches ambiguous registered identities");
            }
        }
        let profile = selected.ok_or_else(|| {
            Error("declared source/profile differs from every registered pinned original-native identity".into())
        })?;
        Self::load_profile(profile, directory, declared)
    }
    /// Explicit profile selection precedes filesystem access. Declared input must
    /// equal the complete authoritative manifest identity for that exact profile.
    pub fn load_profile(
        profile: RulesProfile,
        directory: impl AsRef<Path>,
        declared: &SourceIdentity,
    ) -> Result<Self> {
        let manifest = profile_manifest(profile)?;
        if declared != &manifest.source {
            return fail(
                "declared source/profile differs from selected pinned original-native identity",
            );
        }
        // Name-based selection avoids coupling profile extension to manifest array
        // position. Only registered exact filenames are ever opened.
        let expected_configuration = manifest
            .databases
            .iter()
            .find(|db| db.filename == "configuration.sqlite")
            .ok_or_else(|| Error("registered manifest lacks configuration.sqlite".into()))?;
        let expected_gameplay = manifest
            .databases
            .iter()
            .find(|db| db.filename == "gameplay.sqlite")
            .ok_or_else(|| Error("registered manifest lacks gameplay.sqlite".into()))?;
        if manifest.databases.len() != 2 {
            return fail("registered rules manifest must contain exactly configuration and gameplay databases");
        }
        let configuration = inspect(&directory.as_ref().join(&expected_configuration.filename))?;
        validate(&configuration, expected_configuration)?;
        let gameplay = inspect(&directory.as_ref().join(&expected_gameplay.filename))?;
        validate(&gameplay, expected_gameplay)?;
        let table = gameplay
            .table("Units")
            .ok_or_else(|| Error("missing table Units".into()))?;
        let keys = table.text_index("UnitType")?;
        drop(keys);
        let mut units = Vec::new();
        let mut unit_index = HashMap::new();
        for row in table.rows() {
            let unit_type = row.text("UnitType")?.to_owned();
            unit_index.insert(unit_type.clone(), units.len());
            units.push(UnitDefinition {
                rules_row_id: row
                    .row_id()
                    .ok_or_else(|| Error("Units lacks intrinsic rowid".into()))?,
                unit_type,
                cost: row.integer("Cost")?,
                base_moves: row.integer("BaseMoves")?,
                combat: row.integer("Combat")?,
                ranged_combat: row.integer("RangedCombat")?,
                range: row.integer("Range")?,
                maintenance: row.integer("Maintenance")?,
            });
        }
        Ok(Self {
            source: declared.clone(),
            profile,
            configuration,
            gameplay,
            units,
            unit_index,
        })
    }
    pub fn profile(&self) -> RulesProfile {
        self.profile
    }
    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }
    pub fn configuration(&self) -> &Database {
        &self.configuration
    }
    pub fn gameplay(&self) -> &Database {
        &self.gameplay
    }
    pub fn units(&self) -> &[UnitDefinition] {
        &self.units
    }
    pub fn unit(&self, unit_type: &str) -> Option<&UnitDefinition> {
        self.unit_index.get(unit_type).map(|&i| &self.units[i])
    }
}
fn validate(actual: &Database, expected: &DatabaseFingerprint) -> Result<()> {
    for table in &expected.semantic.tables {
        let found = actual.table(&table.name).ok_or_else(|| {
            Error(format!(
                "{} table {}: missing required table",
                expected.filename, table.name
            ))
        })?;
        let got = found.fingerprint();
        if got.columns_sha256 != table.columns_sha256 {
            return fail(format!(
                "{} table {}: column metadata digest mismatch ({} columns)",
                expected.filename,
                table.name,
                found.columns.len()
            ));
        }
        if got != table {
            return fail(format!("{} table {}: schema/row identity or typed rows mismatch; expected {} rows, got {}; expected {}, got {}", expected.filename, table.name, table.rows, got.rows, table.typed_rows_sha256, got.typed_rows_sha256));
        }
    }
    if actual.fingerprint != *expected {
        return fail(format!("{}: complete schema/file/semantic identity mismatch; semantic expected {}, got {}; file expected {}, got {}", expected.filename, expected.semantic_sha256, actual.fingerprint.semantic_sha256, expected.file_sha256, actual.fingerprint.file_sha256));
    }
    Ok(())
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn production_profiles_preserve_registered_descriptors() {
        // Canonical identities cover every previously consumed descriptor and
        // database field, including nested arrays and raw JSON-valued identities.
        let expected = [
            (
                RulesProfile::LinuxXp2Base,
                "da3394ed155bb25e13e9a32380356abbf017a6d97daf55d1ddb28aabb1dc2a44",
                "f0137e4c5203f0a16c5e5ae5e0ad7a4cfb6481e8e338db22b48a3f053a635cc9",
            ),
            (
                RulesProfile::LinuxXp2ActivatedDiagnostic,
                "a70562229841296b982cfb1d9e435c75a9730656ac6657fb5e4d123c24d0330d",
                "ac6d4e34b22dd3f154c8ca069cf1fa2f9be1cf0b8de3fba673a65571418954b4",
            ),
            (
                RulesProfile::WindowsXp2Historical,
                "04c4a71e37c6e0fa5d72903cc50c58be1e396a95d04caa8491441cbae872f150",
                "36c6b2987a3cdacf5fbfc6c5129700d7299a4b286ad48fde0515121bfdfe1faf",
            ),
        ];
        for (profile, source_digest, databases_digest) in expected {
            let manifest = profile_manifest(profile).unwrap();
            let source = serde_json::to_value(&manifest.source).unwrap();
            let databases = serde_json::to_value(&manifest.databases).unwrap();
            assert_eq!(sha(&serde_json::to_vec(&source).unwrap()), source_digest);
            assert_eq!(
                sha(&serde_json::to_vec(&databases).unwrap()),
                databases_digest
            );
            assert_eq!(manifest.source, profile.source_identity().unwrap());
            assert_eq!(manifest.databases.len(), 2);
            assert_eq!(
                manifest.source.profile["target_sha256"],
                profile.gamecore_sha256()
            );

            let text = match profile {
                RulesProfile::LinuxXp2Base => LINUX_BASE_MANIFEST,
                RulesProfile::LinuxXp2ActivatedDiagnostic => LINUX_XP2_DIAGNOSTIC_MANIFEST,
                RulesProfile::WindowsXp2Historical => WINDOWS_HISTORICAL_MANIFEST,
            };
            let mut consumed = source;
            consumed
                .as_object_mut()
                .unwrap()
                .insert("databases".into(), databases);
            // A production descriptor contains exactly the deserialized contract,
            // with no ignored report sections or database annotations.
            assert_eq!(serde_json::from_str::<Value>(text).unwrap(), consumed);
        }
    }
}

pub mod random_event_definitions;

#[cfg(test)]
mod tests;

pub mod unit_placement_state;

pub mod unit_placement_services;

pub mod plot_adjacent_friendly_unit_tag_requirement;
pub mod plot_adjacent_owner_requirement;


pub mod game_update_state;
pub mod unit_near_religious_city_requirement;
pub mod unit_modifier_effect;

pub mod found_city;
pub mod district_initialization;
pub mod unit_kill;

pub mod city_territory;
pub mod influence_map_update;

pub mod found_city_requirement_queries;
