//! Production ingestion of bounded passive-observer artifacts. Consistency is
//! checked here; authenticity and native/gameplay acceptance are separate.
use super::*;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct NamedArtifact<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
}
/// Exact bytes, not caller-supplied success/acceptance flags. The raw Save
/// container and actual incoming Core remain distinct artifacts.
#[derive(Clone, Copy)]
pub struct CaptureArtifacts<'a> {
    pub observations_jsonl: &'a [u8],
    pub runtime_plan_json: &'a [u8],
    pub static_plan_json: &'a [u8],
    pub prepared_report_json: &'a [u8],
    pub exit_json: &'a [u8],
    pub target_module: &'a [u8],
    pub target_host: &'a [u8],
    pub input_save: &'a [u8],
    pub input_core: &'a [u8],
    pub outputs: &'a [NamedArtifact<'a>],
    pub source_files: &'a [NamedArtifact<'a>],
}
#[derive(Clone, Copy, Debug)]
pub struct CaptureLimits {
    pub max_log_bytes: usize,
    pub max_line_bytes: usize,
    pub max_events: usize,
    pub max_artifact_bytes: usize,
    pub max_total_artifact_bytes: usize,
    pub max_vector_values: usize,
    pub max_artifacts: usize,
    pub max_identity_bytes: usize,
}
impl Default for CaptureLimits {
    fn default() -> Self {
        Self {
            max_log_bytes: 32 * 1024 * 1024,
            max_line_bytes: 2 * 1024 * 1024,
            max_events: 100_000,
            max_artifact_bytes: 1024 * 1024 * 1024,
            max_total_artifact_bytes: 2 * 1024 * 1024 * 1024,
            max_vector_values: 65_536,
            max_artifacts: 1024,
            max_identity_bytes: 4096,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureError {
    ResourceLimit,
    Json,
    MissingField(&'static str),
    Shape,
    ArtifactIdentity,
    TargetIdentity,
    Provenance,
    Incomplete,
    EventOrder,
    Frame,
    Stream,
    Owner,
    Vector,
    Definitions,
    Initialization,
    Query,
    MilitaryWire,
}
impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Military capture admission: {self:?}")
    }
}
impl std::error::Error for CaptureError {}
type Result<T> = std::result::Result<T, CaptureError>;
fn require(ok: bool, e: CaptureError) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(e)
    }
}
fn field<'a>(v: &'a Value, k: &'static str) -> Result<&'a Value> {
    v.get(k).ok_or(CaptureError::MissingField(k))
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or(CaptureError::Shape)
}
fn number(v: &Value) -> Result<u64> {
    if let Some(n) = v.as_u64() {
        return Ok(n);
    }
    if let Some(s) = v.as_str() {
        return u64::from_str_radix(s.strip_prefix("0x").ok_or(CaptureError::Shape)?, 16)
            .map_err(|_| CaptureError::Shape);
    }
    Err(CaptureError::Shape)
}
fn n(v: &Value, k: &'static str) -> Result<u64> {
    number(field(v, k)?)
}
fn s<'a>(v: &'a Value, k: &'static str) -> Result<&'a str> {
    text(field(v, k)?)
}
fn flag(v: &Value, k: &'static str) -> Result<bool> {
    field(v, k)?.as_bool().ok_or(CaptureError::Shape)
}
fn signed(v: &Value) -> Result<i32> {
    i32::try_from(v.as_i64().ok_or(CaptureError::Shape)?).map_err(|_| CaptureError::Shape)
}
fn array(v: &Value) -> Result<&[Value]> {
    v.as_array().map(Vec::as_slice).ok_or(CaptureError::Shape)
}
fn hash(bytes: &[u8]) -> Digest {
    Sha256::digest(bytes).into()
}
fn digest(v: &Value) -> Result<Digest> {
    let raw = text(v)?;
    require(raw.len() == 64, CaptureError::Shape)?;
    let mut result = [0; 32];
    for (i, pair) in raw.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let nybble = |b| match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            _ => Err(CaptureError::Shape),
        };
        result[i] = (nybble(pair[0])? << 4) | nybble(pair[1])?;
    }
    Ok(result)
}
fn json(raw: &[u8], limit: usize) -> Result<Value> {
    require(raw.len() <= limit, CaptureError::ResourceLimit)?;
    serde_json::from_slice(raw).map_err(|_| CaptureError::Json)
}
/// Python's existing observer/report canonical encoding uses sorted keys and
/// ASCII Unicode escapes. serde_json's default map ordering is sorted; expand
/// non-ASCII scalars to the same UTF-16 escape spelling before hashing.
fn canonical(value: &Value) -> Result<Vec<u8>> {
    fn write(value: &Value, out: &mut Vec<u8>) -> Result<()> {
        match value {
            Value::Array(values) => {
                out.push(b'[');
                for (i, v) in values.iter().enumerate() {
                    if i != 0 {
                        out.push(b',')
                    }
                    write(v, out)?;
                }
                out.push(b']');
            }
            Value::Object(values) => {
                out.push(b'{');
                let mut keys: Vec<_> = values.keys().collect();
                keys.sort();
                for (i, k) in keys.iter().enumerate() {
                    if i != 0 {
                        out.push(b',')
                    }
                    write(&Value::String((*k).clone()), out)?;
                    out.push(b':');
                    write(&values[*k], out)?;
                }
                out.push(b'}');
            }
            Value::Number(v) if v.is_f64() => {
                let raw = format!("{:?}", v.as_f64().ok_or(CaptureError::Json)?);
                if let Some((base, exponent)) = raw.split_once('e') {
                    let exponent: i32 = exponent.parse().map_err(|_| CaptureError::Json)?;
                    out.extend_from_slice(
                        format!(
                            "{base}e{}{:02}",
                            if exponent < 0 { "-" } else { "+" },
                            exponent.unsigned_abs()
                        )
                        .as_bytes(),
                    );
                } else {
                    out.extend_from_slice(raw.as_bytes());
                }
            }
            _ => {
                let raw = serde_json::to_string(value).map_err(|_| CaptureError::Json)?;
                for c in raw.chars() {
                    if c < '\u{7f}' {
                        out.push(c as u8)
                    } else {
                        let mut buf = [0u16; 2];
                        for word in c.encode_utf16(&mut buf) {
                            out.extend_from_slice(format!("\\u{word:04x}").as_bytes());
                        }
                    }
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    write(value, &mut out)?;
    Ok(out)
}

fn canonical_hash(value: &Value) -> Result<Digest> {
    Ok(hash(&canonical(value)?))
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureIdentity {
    pub raw_observation_sha256: Digest,
    pub runtime_plan_sha256: Digest,
    pub runtime_plan_artifact_sha256: Digest,
    pub static_plan_artifact_sha256: Digest,
    pub exit_artifact_sha256: Digest,
    pub prepared_report_sha256: Digest,
    pub configuration_sha256: Digest,
    pub input_save_sha256: Digest,
    pub input_core_sha256: Digest,
    /// This is supplied-capture consistency, never execution authenticity.
    pub live_database_snapshot_unavailable: bool,
}
struct ParsedArtifacts<'a> {
    identity: Arc<CaptureIdentity>,
    plan: Value,
    #[cfg(test)]
    report: Value,
    rows: Vec<Value>,
    input_core: &'a [u8],
    inputs: InputIdentities,
    runner_source_sha256: Digest,
    outputs: Vec<(&'a str, &'a [u8], Digest)>,
}
fn parse_artifacts<'a>(
    a: CaptureArtifacts<'a>,
    limits: CaptureLimits,
) -> Result<ParsedArtifacts<'a>> {
    parse_artifacts_with_images(a, limits, TargetImages::Original)
}
enum TargetImages {
    Original,
    #[cfg(test)]
    SourceOnly,
}
fn parse_artifacts_with_images<'a>(
    a: CaptureArtifacts<'a>,
    limits: CaptureLimits,
    images: TargetImages,
) -> Result<ParsedArtifacts<'a>> {
    require(
        a.outputs
            .len()
            .checked_add(a.source_files.len())
            .is_some_and(|n| n <= limits.max_artifacts),
        CaptureError::ResourceLimit,
    )?;
    let mut total = 0usize;
    for raw in [
        a.observations_jsonl,
        a.runtime_plan_json,
        a.static_plan_json,
        a.prepared_report_json,
        a.exit_json,
        a.target_module,
        a.target_host,
        a.input_save,
        a.input_core,
    ]
    .into_iter()
    .chain(a.outputs.iter().map(|a| a.bytes))
    .chain(a.source_files.iter().map(|a| a.bytes))
    {
        require(
            raw.len() <= limits.max_artifact_bytes,
            CaptureError::ResourceLimit,
        )?;
        total = total
            .checked_add(raw.len())
            .ok_or(CaptureError::ResourceLimit)?;
    }
    require(
        total <= limits.max_total_artifact_bytes
            && a.observations_jsonl.len() <= limits.max_log_bytes,
        CaptureError::ResourceLimit,
    )?;
    match images {
        TargetImages::Original => require(
            hash(a.target_module) == CORE_IDENTITY && hash(a.target_host) == HOST_IDENTITY,
            CaptureError::TargetIdentity,
        )?,
        #[cfg(test)]
        TargetImages::SourceOnly => {}
    }
    let plan = json(a.runtime_plan_json, limits.max_log_bytes)?;
    let report = json(a.prepared_report_json, limits.max_log_bytes)?;
    let exit = json(a.exit_json, limits.max_line_bytes)?;
    require(!flag(&exit, "observer_failed")?, CaptureError::Incomplete)?;
    require(
        s(&plan, "schema")? == "c6-linux-military-observer-plan-v1",
        CaptureError::Shape,
    )?;
    require(
        digest(field(&plan, "gamecore_sha256")?)? == CORE_IDENTITY
            && digest(field(&plan, "host_sha256")?)? == HOST_IDENTITY,
        CaptureError::TargetIdentity,
    )?;
    let gate = field(&plan, "runtime_gate")?;
    let configuration_sha256 = canonical_hash(field(&report, "configuration")?)?;
    require(
        digest(field(&report, "configuration_sha256")?)? == configuration_sha256
            && digest(field(gate, "configuration_sha256")?)? == configuration_sha256,
        CaptureError::Provenance,
    )?;
    let prepared_report_sha256 = hash(a.prepared_report_json);
    require(
        digest(field(gate, "report_sha256")?)? == prepared_report_sha256,
        CaptureError::ArtifactIdentity,
    )?;
    let sources = field(gate, "source_files")?
        .as_object()
        .ok_or(CaptureError::Shape)?;
    require(
        sources.len() == a.source_files.len(),
        CaptureError::Provenance,
    )?;
    for (path, expected) in sources {
        let matching: Vec<_> = a.source_files.iter().filter(|a| a.name == path).collect();
        require(matching.len() == 1, CaptureError::Provenance)?;
        require(
            hash(matching[0].bytes) == digest(expected)?,
            CaptureError::ArtifactIdentity,
        )?;
    }
    let input_save_sha256 = hash(a.input_save);
    let input_core_sha256 = hash(a.input_core);
    let cfg = field(&report, "configuration")?;
    let observer_cfg = field(cfg, "lower_core_military_initialization_observer")?;
    let static_plan = json(a.static_plan_json, limits.max_log_bytes)?;
    let static_plan_artifact_sha256 = hash(a.static_plan_json);
    let mut stripped = plan.clone();
    stripped
        .as_object_mut()
        .ok_or(CaptureError::Shape)?
        .remove("runtime_gate");
    require(
        static_plan == stripped
            && field(observer_cfg, "static_plan")? == &static_plan
            && digest(field(observer_cfg, "static_plan_sha256")?)? == static_plan_artifact_sha256,
        CaptureError::Provenance,
    )?;
    let observer_sources: Vec<_> = a
        .source_files
        .iter()
        .filter(|source| {
            source.name.rsplit('/').next() == Some("military_initialization_observer.py")
        })
        .collect();
    require(
        observer_sources.len() == 1
            && digest(
                sources
                    .get(observer_sources[0].name)
                    .ok_or(CaptureError::Provenance)?,
            )? == digest(field(&plan, "observer_sha256")?)?,
        CaptureError::Provenance,
    )?;

    require(
        s(observer_cfg, "mode")? == "ordinary_load_save_passive",
        CaptureError::Provenance,
    )?;
    require(
        digest(field(
            field(observer_cfg, "input_save_artifact")?,
            "sha256",
        )?)? == input_save_sha256,
        CaptureError::ArtifactIdentity,
    )?;
    require(
        digest(field(field(cfg, "lower_core_stream")?, "sha256")?)? == input_core_sha256,
        CaptureError::ArtifactIdentity,
    )?;
    let mut rows = Vec::new();
    for line in a.observations_jsonl.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        require(rows.len() < limits.max_events, CaptureError::ResourceLimit)?;
        rows.try_reserve(1)
            .map_err(|_| CaptureError::ResourceLimit)?;
        rows.push(json(line, limits.max_line_bytes)?);
    }
    let mut outputs = Vec::new();
    for output in a.outputs {
        require(
            !outputs.iter().any(|(name, _, _)| name == &output.name),
            CaptureError::Provenance,
        )?;
        outputs.push((output.name, output.bytes, hash(output.bytes)));
    }
    let identity = Arc::new(CaptureIdentity {
        raw_observation_sha256: hash(a.observations_jsonl),
        runtime_plan_sha256: canonical_hash(&plan)?,
        runtime_plan_artifact_sha256: hash(a.runtime_plan_json),
        static_plan_artifact_sha256,
        exit_artifact_sha256: hash(a.exit_json),
        prepared_report_sha256,
        configuration_sha256,
        input_save_sha256,
        input_core_sha256,
        live_database_snapshot_unavailable: true,
    });
    let inputs = expected_inputs(cfg, configuration_sha256)?;
    let runner_source_sha256 = canonical_hash(field(gate, "source_files")?)?;
    Ok(ParsedArtifacts {
        identity,
        plan,
        #[cfg(test)]
        report,
        rows,
        input_core: a.input_core,
        inputs,
        runner_source_sha256,
        outputs,
    })
}

fn u32_field(v: &Value, k: &'static str) -> Result<u32> {
    u32::try_from(n(v, k)?).map_err(|_| CaptureError::Shape)
}
fn vector(v: &Value, limits: CaptureLimits) -> Result<VectorState> {
    let size = u32_field(v, "size_u32")?;
    let capacity = u32_field(v, "capacity_u32")?;
    let values = array(field(v, "values_i32")?)?;
    require(
        size <= capacity
            && capacity as usize <= limits.max_vector_values
            && values.len() == size as usize,
        CaptureError::Vector,
    )?;
    let data = n(v, "data_pointer")?;
    require(capacity == 0 || data != 0, CaptureError::Vector)?;
    require(
        u64::from(capacity)
            .checked_mul(4)
            .and_then(|bytes| data.checked_add(bytes))
            .is_some(),
        CaptureError::Vector,
    )?;
    let mut typed = Vec::new();
    typed
        .try_reserve_exact(values.len())
        .map_err(|_| CaptureError::ResourceLimit)?;
    for value in values {
        typed.push(signed(value)?);
    }
    Ok(VectorState {
        data_pointer: data,
        capacity_u32: capacity,
        size_u32: size,
        padding_u32: u32_field(v, "padding_u32")?,
        tail_padding_u32: u32_field(v, "tail_padding_u32")?,
        values_i32: typed,
    })
}
fn vectors(v: &Value, limits: CaptureLimits) -> Result<Vectors> {
    Ok(Vectors {
        allowed: vector(field(v, "allowed")?, limits)?,
        per_war: vector(field(v, "per_war")?, limits)?,
        current: vector(field(v, "current")?, limits)?,
    })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedState {
    pub military_pointer: u64,
    pub player: PlayerObservation,
    pub vectors: Vectors,
}
fn state(v: &Value, limits: CaptureLimits) -> Result<CapturedState> {
    Ok(CapturedState {
        military_pointer: n(v, "military_pointer")?,
        player: PlayerObservation {
            player_pointer: n(v, "player_pointer")?,
            player_id_i32: signed(field(v, "player_id_i32")?)?,
            was_ever_alive_raw: u8::from(flag(v, "was_ever_alive")?),
            handicap_i32: signed(field(v, "handicap_i32")?)?,
        },
        vectors: vectors(field(v, "vectors")?, limits)?,
    })
}
fn definitions(v: &Value, limits: CaptureLimits) -> Result<DefinitionCollection> {
    let begin = n(v, "begin_pointer")?;
    let end = n(v, "end_pointer")?;
    let capacity = n(v, "capacity_pointer")?;
    let rows = array(field(v, "ordered")?)?;
    require(
        begin <= end
            && end <= capacity
            && (end - begin) % 8 == 0
            && (capacity - begin) % 8 == 0
            && rows.len() as u64 == (end - begin) / 8
            && ((capacity - begin) / 8) as usize <= limits.max_vector_values,
        CaptureError::Definitions,
    )?;
    let mut ordered = Vec::new();
    ordered
        .try_reserve_exact(rows.len())
        .map_err(|_| CaptureError::ResourceLimit)?;
    for row in rows {
        ordered.push(DefinitionEntry {
            pointer: n(row, "pointer")?,
            database_row_i32: signed(field(row, "database_row_i32")?)?,
            collection_index_u32: u32_field(row, "collection_index_u32")?,
            operation_type_hash_u32: u32_field(row, "operation_type_hash_u32")?,
            default_value_i32: signed(field(row, "default_value_i32")?)?,
        });
    }
    Ok(DefinitionCollection {
        collection_pointer: n(v, "collection_pointer")?,
        begin_pointer: begin,
        end_pointer: end,
        capacity_pointer: capacity,
        ordered,
    })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryObservation {
    pub handler_pointer: u64,
    pub query_pointer: u64,
    pub default_value_i32: i32,
    pub civilization_i32: i32,
    pub handicap_i32: i32,
    pub captured_definition_pointer: Option<u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoleStates {
    Read {
        prior_owner_pointer: u64,
        prior: Box<Vectors>,
        initialize_prior: Box<CapturedState>,
        initialized: Box<CapturedState>,
        read_initialized: Box<CapturedState>,
        read_return: Box<CapturedState>,
        has_ai_personality: bool,
        leader: Option<i32>,
        query: Option<QueryObservation>,
    },
    Write {
        entry: Box<CapturedState>,
        guard: Box<CapturedState>,
        returned: Box<CapturedState>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventWitness {
    pub row_index: usize,
    pub checkpoint_row_index: Option<usize>,
    pub engine_sequence: Option<u64>,
    pub event: String,
    pub frame_id: String,
}
fn event_witness(row: &Value, row_index: usize, cp: Option<&Checkpoint>) -> Result<EventWitness> {
    Ok(EventWitness {
        row_index,
        checkpoint_row_index: cp.map(|cp| cp.row_index),
        engine_sequence: row.get("sequence").map(number).transpose()?,
        event: s(row, "event")?.to_owned(),
        frame_id: row
            .get("frame_id")
            .map(text)
            .transpose()?
            .unwrap_or("")
            .to_owned(),
    })
}
/// Compact retained facts, not the original JSONL analysis log or a mutable World.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleSnapshot {
    pub capture: Arc<CaptureIdentity>,
    pub binding: WireBinding,
    pub military_occurrence_id: String,
    pub frame_id: String,
    pub thread_id: [u64; 3],
    pub events: Vec<EventWitness>,
    pub lifecycle_id: String,
    pub owner_lifetime_id: String,
    pub stream_occurrence: u64,
    pub stream_pointer: u64,
    pub core_range: [usize; 2],
    pub states: RoleStates,
    pub active_definitions: Option<DefinitionCollection>,
}
impl RoleSnapshot {
    pub fn player_id(&self) -> i32 {
        match &self.states {
            RoleStates::Read {
                read_initialized, ..
            } => read_initialized.player.player_id_i32,
            RoleStates::Write { entry, .. } => entry.player.player_id_i32,
        }
    }
    pub(super) fn context_for(
        &self,
        player: PlayerIdentity,
        binding: &WireBinding,
    ) -> std::result::Result<player_ai_military::Context, super::Error> {
        let owner = match &self.states {
            RoleStates::Read {
                read_initialized, ..
            } => &read_initialized.player,
            RoleStates::Write { entry, .. } => &entry.player,
        };
        if owner.was_ever_alive_raw == 0
            || player.record_owner != self.player_id()
            || player.requested_owner != self.player_id()
        {
            return Err(super::Error::OwnerBinding);
        }
        if binding != &self.binding {
            return Err(super::Error::InputBinding);
        }
        let size = match &self.states {
            RoleStates::Read {
                read_initialized, ..
            } => read_initialized.vectors.allowed.size_u32,
            RoleStates::Write { guard, .. } => guard.vectors.allowed.size_u32,
        };
        Ok(player_ai_military::Context {
            operation_type_arrays_present: size != 0,
        })
    }
}
#[derive(Clone)]
struct Checkpoint {
    row_index: usize,
    name: String,
    registers: Value,
    frame: String,
    thread: Value,
    return_pc: Option<u64>,
}
impl Checkpoint {
    fn register(&self, key: &'static str) -> Result<u64> {
        n(&self.registers, key)
    }
}
#[derive(Clone)]
struct NativeFrame {
    id: String,
    thread: Value,
    entry_rsp: u64,
    return_pc: u64,
}
impl NativeFrame {
    fn entry(cp: &Checkpoint) -> Result<Self> {
        Ok(Self {
            id: cp.frame.clone(),
            thread: cp.thread.clone(),
            entry_rsp: cp.register("rsp")?,
            return_pc: cp.return_pc.ok_or(CaptureError::Frame)?,
        })
    }
    fn returned(&self, cp: &Checkpoint) -> Result<()> {
        require(
            cp.frame == self.id
                && cp.thread == self.thread
                && cp.register("pc")? == self.return_pc
                && cp.register("rsp")?
                    == self.entry_rsp.checked_add(8).ok_or(CaptureError::Frame)?,
            CaptureError::Frame,
        )
    }
    fn same(&self, cp: &Checkpoint) -> Result<()> {
        require(
            cp.frame == self.id && cp.thread == self.thread,
            CaptureError::Frame,
        )
    }
}
struct ReadProgress {
    prior_owner_pointer: u64,
    prior: Vectors,
    initialize_prior: Option<CapturedState>,
    initialized: Option<CapturedState>,
    ready: Option<CapturedState>,
    returned: Option<CapturedState>,
    personality: Option<bool>,
    leader: Option<i32>,
    query: Option<QueryObservation>,
    definitions: Option<DefinitionCollection>,
    count: Option<u32>,
    initialize_frame: Option<NativeFrame>,
    count_frame: Option<NativeFrame>,
    phase: u8,
    owner_installation: bool,
}
struct WriteProgress {
    entry: CapturedState,
    guard: Option<CapturedState>,
    returned: Option<CapturedState>,
    definitions: Option<DefinitionCollection>,
    arrays: usize,
}
enum Progress {
    Read(Box<ReadProgress>),
    Write(Box<WriteProgress>),
}
struct Record {
    binding: Value,
    frame: NativeFrame,
    military: u64,
    stream: u64,
    stream_occurrence: u64,
    start: usize,
    end: Option<usize>,
    progress: Progress,
    events: Vec<EventWitness>,
}
fn same_state_owner(a: &CapturedState, b: &CapturedState) -> Result<()> {
    require(
        a.military_pointer == b.military_pointer && a.player == b.player,
        CaptureError::Owner,
    )
}
fn zero(v: &VectorState, size: usize) -> bool {
    v.values_i32.len() == size && v.values_i32.iter().all(|v| *v == 0)
}
fn validate_initialization(read: &ReadProgress) -> Result<()> {
    let prior = read
        .initialize_prior
        .as_ref()
        .ok_or(CaptureError::Incomplete)?;
    let initialized = read.initialized.as_ref().ok_or(CaptureError::Incomplete)?;
    let ready = read.ready.as_ref().ok_or(CaptureError::Incomplete)?;
    same_state_owner(prior, initialized)?;
    same_state_owner(initialized, ready)?;
    require(
        read.prior == prior.vectors && initialized == ready,
        CaptureError::Initialization,
    )?;
    let personality = read.personality.ok_or(CaptureError::Incomplete)?;
    require(
        prior.player.was_ever_alive_raw == 0 || personality,
        CaptureError::Initialization,
    )?;
    if !personality {
        require(
            read.leader.is_none()
                && read.query.is_none()
                && read.count.is_none()
                && prior.vectors == initialized.vectors,
            CaptureError::Initialization,
        )?;
        return Ok(());
    }
    let leader = read.leader.ok_or(CaptureError::Incomplete)?;
    let allowed = &initialized.vectors.allowed;
    if leader == -1 {
        let count = read.count.ok_or(CaptureError::Incomplete)? as usize;
        require(
            read.query.is_none()
                && zero(allowed, count)
                && zero(&initialized.vectors.per_war, count),
            CaptureError::Initialization,
        )?;
    } else {
        let query = read.query.as_ref().ok_or(CaptureError::Incomplete)?;
        require(
            query.handicap_i32 == prior.player.handicap_i32,
            CaptureError::Query,
        )?;
        if query.query_pointer == 0 {
            require(
                read.count.is_none() && allowed == &prior.vectors.allowed,
                CaptureError::Initialization,
            )?;
        } else {
            require(
                allowed.size_u32 == read.count.ok_or(CaptureError::Incomplete)?,
                CaptureError::Initialization,
            )?;
        }
        // Per-war uses a separate query. Its complete actual return is retained;
        // allowed-query observations do not reconstruct its SQL row semantics.
    }
    require(
        zero(&initialized.vectors.current, allowed.size_u32 as usize),
        CaptureError::Initialization,
    )
}

fn expected_inputs(cfg: &Value, configuration_sha256: Digest) -> Result<InputIdentities> {
    let content = field(
        field(cfg, "original_filesystem_profile")?,
        "full_content_input_gate_sha256",
    )?;
    let mods = serde_json::json!({"full_content_input_gate_sha256":content,"saved_configuration_sha256":field(cfg,"saved_configuration_sha256")?});
    Ok(InputIdentities {
        configuration_sha256,
        rules_inputs_sha256: digest(content)?,
        database: DatabaseIdentity::LiveSnapshotUnavailable,
        dlc_inputs_sha256: canonical_hash(field(cfg, "dlc_descriptors")?)?,
        mods_inputs_sha256: canonical_hash(&mods)?,
        content_inputs_sha256: digest(content)?,
    })
}
fn input_identities(parsed: &ParsedArtifacts<'_>, binding: &Value) -> Result<InputIdentities> {
    let expected = &parsed.inputs;
    require(
        digest(field(binding, "configuration_sha256")?)? == expected.configuration_sha256
            && digest(field(binding, "runtime_profile_sha256")?)? == expected.configuration_sha256
            && digest(field(binding, "rules_inputs_sha256")?)? == expected.rules_inputs_sha256
            && digest(field(binding, "dlc_sha256")?)? == expected.dlc_inputs_sha256
            && digest(field(binding, "mods_sha256")?)? == expected.mods_inputs_sha256
            && digest(field(binding, "input_save_sha256")?)? == parsed.identity.input_save_sha256
            && digest(field(binding, "input_core_sha256")?)? == parsed.identity.input_core_sha256
            && field(binding, "rules_database_sha256")?.is_null()
            && flag(binding, "live_database_snapshot_unavailable")?,
        CaptureError::Provenance,
    )?;
    require(
        digest(field(binding, "runner_source_sha256")?)? == parsed.runner_source_sha256,
        CaptureError::Provenance,
    )?;
    require(
        s(binding, "owner_lifetime_kind")? == "selected_original_Read_or_Write_frame_only",
        CaptureError::Provenance,
    )?;
    Ok(expected.clone())
}
fn checkpoint(row: &Value, row_index: usize) -> Result<Checkpoint> {
    Ok(Checkpoint {
        row_index,
        name: s(row, "checkpoint")?.to_owned(),
        registers: field(row, "registers")?.clone(),
        frame: s(row, "frame_id")?.to_owned(),
        thread: field(row, "thread")?.clone(),
        return_pc: match field(row, "actual_entry_return_pc")? {
            Value::Null => None,
            v => Some(number(v)?),
        },
    })
}
fn site(name: &str) -> Option<u64> {
    Some(match name {
        "read_entry" => 0x2145620,
        "write_entry" => 0x21453f0,
        "initialize_entry" => 0x2145be0,
        "personality_return" => 0x2145c07,
        "leader_return" => 0x2145c17,
        "query_entry" => 0x203ac30,
        "count_entry" => 0x2114220,
        "hash_array_entry" => 0x214d7b0,
        "write_guard" => 0x2145504,
        "initialize_return" => 0x2145669,
        _ => return None,
    })
}

#[derive(Clone)]
struct MarkerDescriptor {
    words: [u64; 10],
}
impl MarkerDescriptor {
    fn read(row: &Value) -> Result<Self> {
        let raw = s(row, "descriptor_raw80")?;
        require(
            raw.len() == 160 && n(field(row, "registers")?, "rdi")? > 0,
            CaptureError::Stream,
        )?;
        let mut words = [0u64; 10];
        for (i, chunk) in raw.as_bytes().as_chunks::<16>().0.iter().enumerate() {
            let mut bytes = [0u8; 8];
            for (j, pair) in chunk.as_chunks::<2>().0.iter().enumerate() {
                let text = std::str::from_utf8(pair).map_err(|_| CaptureError::Shape)?;
                bytes[j] = u8::from_str_radix(text, 16).map_err(|_| CaptureError::Shape)?;
            }
            words[i] = u64::from_le_bytes(bytes);
        }
        require(words[0] == 1, CaptureError::Stream)?;
        Ok(Self { words })
    }
    fn matches(&self, row: &Value, destroyed: bool) -> Result<()> {
        let keys = [
            "schema",
            if destroyed { "event" } else { "marker_event" },
            "occurrence",
            "context",
            "stream",
            "buffer",
            "size",
            "path",
            "result",
            "offset",
        ];
        for (i, key) in keys.into_iter().enumerate() {
            if i == 7 && !destroyed {
                continue;
            }
            require(n(row, key)? == self.words[i], CaptureError::Stream)?;
        }
        Ok(())
    }
}
#[derive(Clone)]
struct Stream {
    occurrence: u64,
    context: u64,
    pointer: u64,
    buffer: u64,
    path: String,
    size: u64,
    capacity: u64,
    path_pointer: u64,
    kind: WireRole,
    next: u64,
    finished: bool,
}
fn stream_marker(
    row: &Value,
    streams: &mut Vec<Stream>,
    active: &mut Option<usize>,
    marker: &MarkerDescriptor,
) -> Result<()> {
    let event = n(row, "marker_event")?;
    let occurrence = n(row, "occurrence")?;
    require(
        n(row, "schema")? == 1 && occurrence != 0,
        CaptureError::Stream,
    )?;
    if event == 1 || event == 11 {
        let buffer = n(row, "buffer")?;
        let capacity = n(row, "size")?;
        require(
            buffer > 0 && buffer.checked_add(capacity).is_some(),
            CaptureError::Stream,
        )?;
        require(
            active.is_none() && !streams.iter().any(|s| s.occurrence == occurrence),
            CaptureError::Stream,
        )?;
        streams.push(Stream {
            occurrence,
            context: n(row, "context")?,
            pointer: n(row, "stream")?,
            buffer: n(row, "buffer")?,
            path: s(row, "path")?.to_owned(),
            size: n(row, "size")?,
            capacity: n(row, "size")?,
            path_pointer: marker.words[7],
            kind: if event == 1 {
                WireRole::ReadInputCore
            } else {
                WireRole::SavedOutputCore
            },
            next: if event == 1 { 2 } else { 12 },
            finished: false,
        });
        *active = Some(streams.len() - 1);
        return Ok(());
    }
    let index = active.ok_or(CaptureError::Stream)?;
    let stream = &mut streams[index];
    require(
        stream.occurrence == occurrence
            && stream.next == event
            && stream.context == n(row, "context")?
            && stream.pointer == n(row, "stream")?
            && stream.buffer == n(row, "buffer")?
            && stream.path == s(row, "path")?,
        CaptureError::Stream,
    )?;
    require(stream.path_pointer == marker.words[7], CaptureError::Stream)?;
    if event == 2 || event == 12 {
        require(
            n(row, "size")? <= stream.capacity
                && (event != 2 || n(row, "size")? == stream.capacity),
            CaptureError::Stream,
        )?;
        require(
            n(row, "result")? == 0 && n(row, "offset")? == n(row, "size")?,
            CaptureError::Incomplete,
        )?;
        stream.size = n(row, "size")?;
        stream.next = event + 1;
    } else if event == 13 {
        require(n(row, "result")? == 0, CaptureError::Incomplete)?;
        stream.next = 14;
    } else if event == 3 || event == 14 {
        stream.next = event + 1;
        *active = None;
    } else {
        return Err(CaptureError::Stream);
    }
    Ok(())
}
fn military_arrays(
    value: &player_ai_military::Military,
    state: &CapturedState,
    defs: &DefinitionCollection,
) -> Result<()> {
    let present = state.vectors.allowed.size_u32 != 0;
    require(
        value.operation_types.is_some() == present,
        CaptureError::MilitaryWire,
    )?;
    let Some(arrays) = &value.operation_types else {
        return Ok(());
    };
    for (saved, vector) in [
        (&arrays.allowed, &state.vectors.allowed),
        (&arrays.per_war, &state.vectors.per_war),
        (&arrays.current, &state.vectors.current),
    ] {
        require(saved.len() == defs.ordered.len(), CaptureError::Definitions)?;
        for (index, (entry, definition)) in saved.iter().zip(&defs.ordered).enumerate() {
            // WriteIndexHashArray advances the value pointer sequentially;
            // the saved collection-index field is not the writer subscript.
            let actual = vector
                .values_i32
                .get(index)
                .ok_or(CaptureError::Definitions)?;
            require(
                entry.hash == definition.operation_type_hash_u32 && entry.value == *actual,
                CaptureError::MilitaryWire,
            )?;
        }
    }
    Ok(())
}
fn validate_read_vectors(
    value: &player_ai_military::Military,
    read: &ReadProgress,
    defs: Option<&DefinitionCollection>,
) -> Result<()> {
    let ready = read.ready.as_ref().ok_or(CaptureError::Incomplete)?;
    let returned = read.returned.as_ref().ok_or(CaptureError::Incomplete)?;
    same_state_owner(ready, returned)?;
    require(
        ready.vectors.allowed == returned.vectors.allowed
            && ready.vectors.per_war == returned.vectors.per_war,
        CaptureError::Vector,
    )?;
    require(
        value.operation_types.is_some() == (ready.vectors.allowed.size_u32 != 0),
        CaptureError::MilitaryWire,
    )?;
    let mut expected = ready.vectors.current.clone();
    if let Some(arrays) = &value.operation_types {
        let defs = defs.ok_or(CaptureError::Definitions)?;
        let mut by_hash = std::collections::BTreeMap::new();
        for entry in &defs.ordered {
            require(
                by_hash
                    .insert(entry.operation_type_hash_u32, entry)
                    .is_none(),
                CaptureError::Definitions,
            )?;
        }
        // Read14 consumes allowed/per-war into a temporary D-sized vector and
        // destroys it. Only current writes the live Military vector. Unknown
        // hashes are consumed into a scalar temporary; duplicate saved hashes
        // retain source order and therefore the final store wins.
        for saved in [&arrays.allowed, &arrays.per_war] {
            for pair in saved {
                if let Some(def) = by_hash.get(&pair.hash).copied() {
                    let index = def.collection_index_u32 as i32;
                    require(
                        index < 0 || (index as usize) < defs.ordered.len(),
                        CaptureError::Definitions,
                    )?;
                }
            }
        }
        for pair in &arrays.current {
            if let Some(def) = by_hash.get(&pair.hash).copied() {
                let index = def.collection_index_u32 as i32;
                if index >= 0 {
                    *expected
                        .values_i32
                        .get_mut(index as usize)
                        .ok_or(CaptureError::Definitions)? = pair.value;
                }
            }
        }
    }
    require(expected == returned.vectors.current, CaptureError::Vector)
}
fn query_from(row: &Value) -> Result<QueryObservation> {
    let q = field(field(row, "observations")?, "leader_query")?;
    Ok(QueryObservation {
        handler_pointer: n(q, "handler_pointer")?,
        query_pointer: n(q, "query_pointer")?,
        default_value_i32: signed(field(q, "default_value_i32")?)?,
        civilization_i32: signed(field(q, "civilization_i32")?)?,
        handicap_i32: signed(field(q, "handicap_i32")?)?,
        captured_definition_pointer: match field(q, "captured_definition_pointer")? {
            Value::Null => None,
            v => Some(number(v)?),
        },
    })
}
fn target_address(bias: u64, rva: u64) -> Result<u64> {
    bias.checked_add(rva).ok_or(CaptureError::Frame)
}
fn process_record(
    record: &mut Record,
    event: &str,
    row: &Value,
    cp: Option<&Checkpoint>,
    bias: u64,
    limits: CaptureLimits,
) -> Result<()> {
    if let Some(cp) = cp {
        require(cp.thread == record.frame.thread, CaptureError::Frame)?;
    }
    require(
        field(row, "thread")? == &record.frame.thread,
        CaptureError::Frame,
    )?;
    if let Some(binding) = row.get("binding") {
        require(binding == &record.binding, CaptureError::Provenance)?;
    }
    let root_id = record.frame.id.clone();
    let military = record.military;
    let allowed_address = military.checked_add(0x158).ok_or(CaptureError::Owner)?;
    let stream = record.stream;
    match &mut record.progress {
        Progress::Read(read) => match event {
            "initialize_entry" => {
                require(
                    read.phase == 0 && read.owner_installation,
                    CaptureError::EventOrder,
                )?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                let value = state(field(row, "snapshot")?, limits)?;
                require(
                    cp.register("rdi")? == military
                        && cp.register("rsi")? == value.player.player_pointer
                        && value.military_pointer == military
                        && cp.return_pc == Some(target_address(bias, 0x2145669)?),
                    CaptureError::Frame,
                )?;
                require(
                    cp.register("rsp")? < record.frame.entry_rsp,
                    CaptureError::Frame,
                )?;
                read.initialize_frame = Some(NativeFrame::entry(cp)?);
                read.initialize_prior = Some(value);
                read.phase = 1;
            }
            "personality_return" => {
                require(read.phase == 1, CaptureError::EventOrder)?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                read.initialize_frame
                    .as_ref()
                    .ok_or(CaptureError::Frame)?
                    .same(cp)?;
                let value = flag(field(row, "observations")?, "has_ai_personality")?;
                require(
                    (cp.register("rax")? & 255) == u64::from(value),
                    CaptureError::Initialization,
                )?;
                read.personality = Some(value);
                read.phase = 2;
            }
            "leader_return" => {
                require(
                    read.phase == 2 && read.personality == Some(true),
                    CaptureError::EventOrder,
                )?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                read.initialize_frame
                    .as_ref()
                    .ok_or(CaptureError::Frame)?
                    .same(cp)?;
                let leader = signed(field(field(row, "observations")?, "leader_i32")?)?;
                require(
                    cp.register("rax")? as u32 as i32 == leader,
                    CaptureError::Query,
                )?;
                read.leader = Some(leader);
                read.phase = 3;
            }
            "query_entry" => {
                require(
                    read.phase == 3 && read.leader.is_some_and(|l| l != -1),
                    CaptureError::EventOrder,
                )?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                read.initialize_frame
                    .as_ref()
                    .ok_or(CaptureError::Frame)?
                    .same(cp)?;
                require(
                    cp.register("rsp")?
                        < read
                            .initialize_frame
                            .as_ref()
                            .ok_or(CaptureError::Frame)?
                            .entry_rsp,
                    CaptureError::Frame,
                )?;
                if let Some(actual) = cp.return_pc {
                    require(
                        actual == target_address(bias, 0x2145c9b)?,
                        CaptureError::Frame,
                    )?;
                }
                let query = query_from(row)?;
                require(
                    cp.register("rdi")? == query.handler_pointer
                        && cp.register("rsi")? == allowed_address
                        && cp.register("rcx")? as u32 as i32 == query.civilization_i32
                        && cp.register("r8")? as u32 as i32 == query.handicap_i32,
                    CaptureError::Query,
                )?;
                let raw = field(field(row, "observations")?, "leader_query")?;
                if query.query_pointer != 0 {
                    require(
                        n(raw, "count_callback_pointer")? != 0
                            && n(raw, "count_callback_vtable_pointer")? != 0
                            && query.captured_definition_pointer.is_some(),
                        CaptureError::Query,
                    )?;
                }
                read.query = Some(query);
                read.phase = 4;
            }
            "count_entry" => {
                require(
                    (read.phase == 3 && read.leader == Some(-1))
                        || (read.phase == 4
                            && read.query.as_ref().is_some_and(|q| q.query_pointer != 0)),
                    CaptureError::EventOrder,
                )?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                let defs = definitions(field(row, "definitions")?, limits)?;
                require(
                    cp.register("rdi")? == defs.collection_pointer,
                    CaptureError::Definitions,
                )?;
                if read.leader == Some(-1) {
                    require(
                        cp.return_pc == Some(target_address(bias, 0x2145dd2)?)
                            && cp.register("rbx")? == allowed_address,
                        CaptureError::Frame,
                    )?;
                } else {
                    let query = read.query.as_ref().ok_or(CaptureError::Query)?;
                    require(
                        cp.return_pc == Some(target_address(bias, 0x203ac8b)?)
                            && cp.register("rbx")? == query.handler_pointer
                            && cp.register("rbp")? == allowed_address
                            && query.captured_definition_pointer == Some(defs.collection_pointer),
                        CaptureError::Query,
                    )?;
                }
                require(
                    cp.register("rsp")?
                        < read
                            .initialize_frame
                            .as_ref()
                            .ok_or(CaptureError::Frame)?
                            .entry_rsp,
                    CaptureError::Frame,
                )?;
                read.count_frame = Some(NativeFrame::entry(cp)?);
                read.definitions = Some(defs);
                read.phase = 5;
            }
            "count_return" => {
                require(read.phase == 5, CaptureError::EventOrder)?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                read.count_frame
                    .as_ref()
                    .ok_or(CaptureError::Frame)?
                    .returned(cp)?;
                let defs = definitions(field(row, "definitions")?, limits)?;
                let count = u32_field(row, "count_u32")?;
                require(
                    read.definitions.as_ref() == Some(&defs)
                        && count as usize == defs.ordered.len()
                        && cp.register("rax")? as u32 == count,
                    CaptureError::Definitions,
                )?;
                read.count = Some(count);
                read.phase = 6;
            }
            "initialize_return" => {
                let expected = if read.personality == Some(false) {
                    2
                } else if read.leader == Some(-1)
                    || read.query.as_ref().is_some_and(|q| q.query_pointer != 0)
                {
                    6
                } else {
                    4
                };
                require(read.phase == expected, CaptureError::EventOrder)?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                read.initialize_frame
                    .as_ref()
                    .ok_or(CaptureError::Frame)?
                    .returned(cp)?;
                require(flag(row, "normal_native_return")?, CaptureError::Frame)?;
                let observations = field(row, "observations")?;
                require(
                    Some(flag(observations, "has_ai_personality")?) == read.personality,
                    CaptureError::Initialization,
                )?;
                if let Some(leader) = read.leader {
                    require(
                        signed(field(observations, "leader_i32")?)? == leader,
                        CaptureError::Query,
                    )?;
                }
                if let Some(query) = &read.query {
                    require(&query_from(row)? == query, CaptureError::Query)?;
                }
                if let Some(defs) = &read.definitions {
                    require(
                        &definitions(field(observations, "active_definitions")?, limits)? == defs,
                        CaptureError::Definitions,
                    )?;
                }
                read.initialized = Some(state(field(row, "snapshot")?, limits)?);
                read.phase = 7;
            }
            "read_initialized" => {
                require(
                    read.phase == 7 && s(row, "frame_id")? == root_id,
                    CaptureError::EventOrder,
                )?;
                read.ready = Some(state(field(row, "snapshot")?, limits)?);
                read.phase = 8;
                validate_initialization(read)?;
            }
            "read_return" => {
                require(read.phase == 8, CaptureError::EventOrder)?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                record.frame.returned(cp)?;
                require(flag(row, "normal_native_return")?, CaptureError::Frame)?;
                read.returned = Some(state(field(row, "snapshot")?, limits)?);
                read.phase = 9;
            }
            _ => return Err(CaptureError::EventOrder),
        },
        Progress::Write(write) => match event {
            "write_guard" => {
                require(
                    write.guard.is_none() && write.arrays == 0,
                    CaptureError::EventOrder,
                )?;
                let cp = cp.ok_or(CaptureError::Frame)?;
                record.frame.same(cp)?;
                let guard = state(field(row, "snapshot")?, limits)?;
                same_state_owner(&write.entry, &guard)?;
                require(
                    write.entry == guard
                        && cp.register("rbx")? == military
                        && cp.register("r14")? == stream
                        && cp.register("rax")? as u32 == guard.vectors.allowed.size_u32
                        && flag(row, "operation_type_arrays_present")?
                            == (guard.vectors.allowed.size_u32 != 0),
                    CaptureError::Vector,
                )?;
                write.guard = Some(guard);
            }
            "hash_array_entry" => {
                let cp = cp.ok_or(CaptureError::Frame)?;
                record.frame.same(cp)?;
                let guard = write.guard.as_ref().ok_or(CaptureError::EventOrder)?;
                require(
                    guard.vectors.allowed.size_u32 != 0 && write.arrays < 3,
                    CaptureError::EventOrder,
                )?;
                require(
                    cp.register("rsp")? < record.frame.entry_rsp,
                    CaptureError::Frame,
                )?;
                if let Some(actual) = cp.return_pc {
                    require(
                        actual
                            == target_address(
                                bias,
                                [0x2145526, 0x2145548, 0x214556a][write.arrays],
                            )?,
                        CaptureError::Frame,
                    )?;
                }
                let (name, expected) = match write.arrays {
                    0 => ("allowed", &guard.vectors.allowed),
                    1 => ("per_war", &guard.vectors.per_war),
                    _ => ("current", &guard.vectors.current),
                };
                let actual = vector(field(row, "vector")?, limits)?;
                let defs = definitions(field(row, "definitions")?, limits)?;
                require(
                    s(row, "name")? == name
                        && &actual == expected
                        && cp.register("rdi")? == stream
                        && cp.register("rsi")? == expected.data_pointer
                        && cp.register("rdx")? == defs.collection_pointer
                        && expected.values_i32.len() >= defs.ordered.len(),
                    CaptureError::Definitions,
                )?;
                if let Some(previous) = &write.definitions {
                    require(previous == &defs, CaptureError::Definitions)?;
                } else {
                    write.definitions = Some(defs);
                }
                write.arrays += 1;
            }
            "write_return" => {
                let cp = cp.ok_or(CaptureError::Frame)?;
                record.frame.returned(cp)?;
                let guard = write.guard.as_ref().ok_or(CaptureError::EventOrder)?;
                require(
                    write.returned.is_none()
                        && write.arrays
                            == if guard.vectors.allowed.size_u32 == 0 {
                                0
                            } else {
                                3
                            }
                        && flag(row, "normal_native_return")?,
                    CaptureError::EventOrder,
                )?;
                let returned = state(field(row, "snapshot")?, limits)?;
                require(returned == write.entry, CaptureError::Vector)?;
                write.returned = Some(returned);
            }
            _ => return Err(CaptureError::EventOrder),
        },
    }
    Ok(())
}

/// Validated consistency of supplied finalized capture artifacts. This does not
/// authenticate execution or reconstruct initialization/gameplay semantics.
#[derive(Debug)]
pub struct AdmittedCapture {
    pub identity: Arc<CaptureIdentity>,
    snapshots: Vec<Arc<InitializationSnapshot>>,
}
impl AdmittedCapture {
    pub fn snapshots(&self) -> &[Arc<InitializationSnapshot>] {
        &self.snapshots
    }
    pub const fn native_acceptance(&self) -> bool {
        false
    }
}
/// Parse/check all capture evidence once. No benchmark identities or trusted
/// booleans select a framing branch; raw native checkpoints and vector states do.
pub fn admit_capture(
    artifacts: CaptureArtifacts<'_>,
    limits: CaptureLimits,
) -> Result<AdmittedCapture> {
    let parsed = parse_artifacts(artifacts, limits)?;
    admit_parsed(parsed, limits)
}
fn admit_parsed(parsed: ParsedArtifacts<'_>, limits: CaptureLimits) -> Result<AdmittedCapture> {
    admit_parsed_with_origin(parsed, limits, Origin::AttributedObservation)
}
fn admit_parsed_with_origin(
    parsed: ParsedArtifacts<'_>,
    limits: CaptureLimits,
    origin: Origin,
) -> Result<AdmittedCapture> {
    let mut sequence = 0u64;
    let mut begun = false;
    let mut ended = false;
    let mut final_gate = false;
    let mut pending: Option<Checkpoint> = None;
    let mut current: Option<Record> = None;
    let mut records = Vec::new();
    let mut frames = std::collections::BTreeSet::new();
    let mut bias: Option<u64> = None;
    let mut streams = Vec::new();
    let mut active = None;
    let mut pending_marker: Option<MarkerDescriptor> = None;
    let mut marker_pc = None;
    let mut capture_thread: Option<Value> = None;
    let mut bound_outputs = std::collections::BTreeMap::new();
    for (row_index, row) in parsed.rows.iter().enumerate() {
        let event = s(row, "event")?;
        require(
            !ended && (!final_gate || event == "capture_end"),
            CaptureError::EventOrder,
        )?;
        if event == "installed" {
            require(
                marker_pc.is_none()
                    && digest(field(row, "profile_sha256")?)?
                        == parsed.identity.configuration_sha256
                    && n(row, "hardware_target_lanes")? == 3
                    && n(row, "provenance_lanes")? == 1
                    && !flag(row, "software_game_breakpoints")?,
                CaptureError::Provenance,
            )?;
            let address = n(row, "marker_symbol_address")?;
            let pc = n(row, "marker_stop_pc")?;
            require(
                address > 0 && (pc == address || address.checked_add(4) == Some(pc)),
                CaptureError::Provenance,
            )?;
            marker_pc = Some(pc);
            continue;
        }
        if event == "marker_checkpoint" {
            require(
                current.is_none()
                    && pending.is_none()
                    && pending_marker.is_none()
                    && marker_pc == Some(n(field(row, "registers")?, "pc")?),
                CaptureError::Stream,
            )?;
            let thread = field(row, "thread")?;
            if let Some(expected) = &capture_thread {
                require(thread == expected, CaptureError::Frame)?;
            } else {
                capture_thread = Some(thread.clone());
            }
            pending_marker = Some(MarkerDescriptor::read(row)?);
            continue;
        }
        if event == "checkpoint" {
            require(pending.is_none(), CaptureError::EventOrder)?;
            let cp = checkpoint(row, row_index)?;
            require(pending_marker.is_none(), CaptureError::Stream)?;
            if let Some(expected) = &capture_thread {
                require(&cp.thread == expected, CaptureError::Frame)?;
            } else {
                capture_thread = Some(cp.thread.clone());
            }
            require(
                !cp.frame.is_empty() && cp.frame.len() <= limits.max_identity_bytes,
                CaptureError::Frame,
            )?;
            let tid = array(&cp.thread)?;
            require(
                tid.len() == 3 && number(&tid[0])? > 0 && number(&tid[1])? > 0,
                CaptureError::Frame,
            )?;
            number(&tid[2])?;
            if let Some(site) = site(&cp.name) {
                let actual = cp
                    .register("pc")?
                    .checked_sub(site)
                    .ok_or(CaptureError::Frame)?;
                require(actual % 4096 == 0, CaptureError::Frame)?;
                if let Some(expected) = bias {
                    require(actual == expected, CaptureError::Frame)?;
                } else {
                    bias = Some(actual);
                }
            }
            if matches!(
                cp.name.as_str(),
                "read_entry" | "write_entry" | "initialize_entry" | "count_entry"
            ) {
                require(frames.insert(cp.frame.clone()), CaptureError::Frame)?;
            }
            pending = Some(cp);
            continue;
        }
        if event == "owner_installation_witness" {
            let record = current.as_mut().ok_or(CaptureError::Frame)?;
            let cp = pending.as_ref().ok_or(CaptureError::Frame)?;
            let Progress::Read(read) = &mut record.progress else {
                return Err(CaptureError::Frame);
            };
            require(
                read.phase == 0
                    && !read.owner_installation
                    && cp.name == "initialize_entry"
                    && s(row, "read_frame")? == record.frame.id
                    && s(row, "initialize_frame")? == cp.frame
                    && n(row, "military_pointer")? == record.military
                    && cp.register("rdi")? == record.military
                    && n(row, "actual_Player_argument")? == cp.register("rsi")?
                    && n(row, "stored_Player_pointer")? == cp.register("rsi")?,
                CaptureError::Owner,
            )?;
            read.owner_installation = true;
            let mut witness = event_witness(row, row_index, Some(cp))?;
            witness.frame_id = cp.frame.clone();
            record.events.push(witness);
            continue;
        }
        if event == "runner_refused" || event == "capture_refused" {
            return Err(CaptureError::Incomplete);
        }
        if event == "stream_marker" {
            require(
                current.is_none() && pending.is_none(),
                CaptureError::EventOrder,
            )?;
            let marker = pending_marker.take().ok_or(CaptureError::Stream)?;
            marker.matches(row, false)?;
            if n(row, "marker_event")? == 1 {
                require(
                    n(row, "size")? as usize == parsed.input_core.len() && n(row, "offset")? == 0,
                    CaptureError::Stream,
                )?;
            }
            if n(row, "marker_event")? == 11 {
                require(
                    n(row, "size")? > 0 && n(row, "offset")? == 0,
                    CaptureError::Stream,
                )?;
            }
            stream_marker(row, &mut streams, &mut active, &marker)?;
            continue;
        }
        if event == "stream_destroyed" {
            require(current.is_none() && active.is_none(), CaptureError::Stream)?;
            let marker = field(row, "marker")?;
            pending_marker
                .take()
                .ok_or(CaptureError::Stream)?
                .matches(marker, true)?;
            let occurrence = n(marker, "occurrence")?;
            let stream = streams
                .iter_mut()
                .find(|s| s.occurrence == occurrence)
                .ok_or(CaptureError::Stream)?;
            require(
                !stream.finished
                    && stream.next == n(marker, "event")?
                    && stream.pointer == n(marker, "stream")?
                    && stream.context == n(marker, "context")?
                    && stream.buffer == n(marker, "buffer")?
                    && stream.path_pointer == n(marker, "path")?
                    && stream.size == n(marker, "size")?
                    && n(marker, "offset")? == stream.size
                    && n(marker, "result")? == 0,
                CaptureError::Stream,
            )?;
            stream.finished = true;
            continue;
        }
        if event == "final_live_original_method_fingerprints_equal" {
            // Source-closed runner checks 29 Initialize dependencies +5 wire methods.
            require(
                begun
                    && !final_gate
                    && !records.is_empty()
                    && !streams.is_empty()
                    && current.is_none()
                    && pending.is_none()
                    && pending_marker.is_none()
                    && active.is_none()
                    && n(row, "method_count")? == 34
                    && streams.iter().all(|s| s.finished),
                CaptureError::Incomplete,
            )?;
            final_gate = true;
            continue;
        }
        if event == "original_stream_range" {
            require(pending.is_none(), CaptureError::EventOrder)?;
            let mut record = current.take().ok_or(CaptureError::EventOrder)?;
            require(
                field(row, "binding")? == &record.binding && s(row, "frame_id")? == record.frame.id,
                CaptureError::Provenance,
            )?;
            let range = array(field(row, "core_range")?)?;
            require(range.len() == 2, CaptureError::Stream)?;
            let start =
                usize::try_from(number(&range[0])?).map_err(|_| CaptureError::ResourceLimit)?;
            let end =
                usize::try_from(number(&range[1])?).map_err(|_| CaptureError::ResourceLimit)?;
            let returned = match &record.progress {
                Progress::Read(r) => r.phase == 9,
                Progress::Write(w) => w.returned.is_some(),
            };
            require(
                returned && start == record.start && start < end,
                CaptureError::Stream,
            )?;
            let kind = match &record.progress {
                Progress::Read(_) => "read",
                Progress::Write(_) => "write",
            };
            require(s(row, "kind")? == kind, CaptureError::Stream)?;
            record.end = Some(end);
            record.events.push(event_witness(row, row_index, None)?);
            records
                .try_reserve(1)
                .map_err(|_| CaptureError::ResourceLimit)?;
            records.push(record);
            continue;
        }
        // Non-observer runner diagnostics are retained in the hashed original
        // log but cannot supply a missing checkpoint or observation.
        if row.get("schema").and_then(Value::as_str) != Some("c6-linux-military-observation-v1") {
            continue;
        }
        sequence = sequence.checked_add(1).ok_or(CaptureError::ResourceLimit)?;
        require(n(row, "sequence")? == sequence, CaptureError::EventOrder)?;
        if event == "observer_begin" {
            require(
                !begun && sequence == 1 && pending.is_none(),
                CaptureError::EventOrder,
            )?;
            begun = true;
            require(
                digest(field(row, "gamecore_sha256")?)? == CORE_IDENTITY
                    && digest(field(row, "host_sha256")?)? == HOST_IDENTITY,
                CaptureError::TargetIdentity,
            )?;
            require(
                digest(field(row, "plan_sha256")?)? == parsed.identity.runtime_plan_sha256
                    && field(row, "runtime_gate")? == field(&parsed.plan, "runtime_gate")?
                    && field(row, "observer_sha256")? == field(&parsed.plan, "observer_sha256")?,
                CaptureError::Provenance,
            )?;
            continue;
        }
        require(begun, CaptureError::EventOrder)?;
        if event == "capture_end" {
            require(
                current.is_none()
                    && pending.is_none()
                    && pending_marker.is_none()
                    && active.is_none()
                    && streams.iter().all(|s| s.finished)
                    && !flag(row, "observer_failed")?
                    && final_gate,
                CaptureError::Incomplete,
            )?;
            ended = true;
            continue;
        }
        if event == "saved_output_binding" {
            require(
                pending_marker.as_ref().is_some_and(|m| m.words[1] == 13),
                CaptureError::Stream,
            )?;
            require(
                current.is_none() && pending.is_none(),
                CaptureError::EventOrder,
            )?;
            let occurrence = s(row, "occurrence_id")?.to_owned();
            require(
                !bound_outputs.contains_key(&occurrence),
                CaptureError::Provenance,
            )?;
            bound_outputs.insert(occurrence, (row.clone(), row_index));
            continue;
        }
        let cp = if event == "read_initialized" {
            require(pending.is_none(), CaptureError::EventOrder)?;
            None
        } else {
            let cp = pending.take().ok_or(CaptureError::Frame)?;
            require(
                cp.name == event
                    && cp.frame == s(row, "frame_id")?
                    && cp.thread == *field(row, "thread")?,
                CaptureError::Frame,
            )?;
            Some(cp)
        };
        if event == "read_entry" || event == "write_entry" {
            require(current.is_none(), CaptureError::EventOrder)?;
            let cp = cp.as_ref().ok_or(CaptureError::Frame)?;
            let binding = field(row, "binding")?.clone();
            for key in ["occurrence_id", "lifecycle_id", "owner_lifetime_id"] {
                let identity = s(&binding, key)?;
                require(
                    !identity.is_empty() && identity.len() <= limits.max_identity_bytes,
                    CaptureError::Provenance,
                )?;
            }
            let stream = streams
                .get(active.ok_or(CaptureError::Stream)?)
                .ok_or(CaptureError::Stream)?;
            let role = if event == "read_entry" {
                WireRole::ReadInputCore
            } else {
                WireRole::SavedOutputCore
            };
            require(
                stream.kind == role
                    && n(&binding, "stream_occurrence")? == stream.occurrence
                    && n(&binding, "stream_pointer")? == stream.pointer
                    && cp.register("rdi")? == stream.pointer,
                CaptureError::Stream,
            )?;
            input_identities(&parsed, &binding)?;
            require(
                n(row, "military_pointer")? == cp.register("rsi")?
                    && n(row, "stream_pointer")? == stream.pointer,
                CaptureError::Owner,
            )?;
            let progress = if role == WireRole::ReadInputCore {
                Progress::Read(Box::new(ReadProgress {
                    prior_owner_pointer: n(row, "prior_owner_pointer")?,
                    prior: vectors(field(row, "prior_vectors")?, limits)?,
                    initialize_prior: None,
                    initialized: None,
                    ready: None,
                    returned: None,
                    personality: None,
                    leader: None,
                    query: None,
                    definitions: None,
                    count: None,
                    initialize_frame: None,
                    count_frame: None,
                    phase: 0,
                    owner_installation: false,
                }))
            } else {
                let entry = state(field(row, "snapshot")?, limits)?;
                require(
                    entry.military_pointer == cp.register("rsi")?
                        && entry.vectors == vectors(field(row, "prior_vectors")?, limits)?,
                    CaptureError::Owner,
                )?;
                Progress::Write(Box::new(WriteProgress {
                    entry,
                    guard: None,
                    returned: None,
                    definitions: None,
                    arrays: 0,
                }))
            };
            current = Some(Record {
                frame: NativeFrame::entry(cp)?,
                military: cp.register("rsi")?,
                stream: stream.pointer,
                stream_occurrence: stream.occurrence,
                start: usize::try_from(n(&binding, "core_start")?)
                    .map_err(|_| CaptureError::ResourceLimit)?,
                end: None,
                binding,
                progress,
                events: vec![event_witness(row, row_index, Some(cp))?],
            });
        } else {
            process_record(
                current.as_mut().ok_or(CaptureError::EventOrder)?,
                event,
                row,
                cp.as_ref(),
                bias.ok_or(CaptureError::Frame)?,
                limits,
            )?;
            current
                .as_mut()
                .ok_or(CaptureError::EventOrder)?
                .events
                .push(event_witness(row, row_index, cp.as_ref())?);
        }
    }
    require(
        begun && ended && current.is_none() && pending.is_none() && !records.is_empty(),
        CaptureError::Incomplete,
    )?;
    let mut snapshots = Vec::new();
    for mut record in records {
        let stream = streams
            .iter()
            .find(|s| s.occurrence == record.stream_occurrence)
            .ok_or(CaptureError::Stream)?;
        let (core, core_hash, output_id) = match &record.progress {
            Progress::Read(_) => {
                require(
                    stream.size as usize == parsed.input_core.len(),
                    CaptureError::ArtifactIdentity,
                )?;
                (parsed.input_core, parsed.identity.input_core_sha256, None)
            }
            Progress::Write(_) => {
                let output = parsed
                    .outputs
                    .iter()
                    .find(|(name, _, _)| *name == stream.path)
                    .ok_or(CaptureError::ArtifactIdentity)?;
                let (binding, binding_row) = bound_outputs
                    .remove(s(&record.binding, "occurrence_id")?)
                    .ok_or(CaptureError::Incomplete)?;
                let mut witness = event_witness(&binding, binding_row, None)?;
                witness.frame_id = record.frame.id.clone();
                record.events.push(witness);
                require(
                    s(&binding, "output_id")? == stream.path
                        && digest(field(&binding, "output_core_sha256")?)? == output.2
                        && digest(field(&binding, "saved_output_sha256")?)? == output.2
                        && stream.size as usize == output.1.len(),
                    CaptureError::ArtifactIdentity,
                )?;
                let range = array(field(&binding, "military_core_range")?)?;
                require(
                    range.len() == 2
                        && number(&range[0])? as usize == record.start
                        && Some(number(&range[1])? as usize) == record.end,
                    CaptureError::Stream,
                )?;
                (output.1, output.2, Some(stream.path.as_bytes().to_vec()))
            }
        };
        let end = record.end.ok_or(CaptureError::Incomplete)?;
        let raw = core.get(record.start..end).ok_or(CaptureError::Stream)?;
        let (present, owner) = match &record.progress {
            Progress::Read(r) => {
                let ready = r.ready.as_ref().ok_or(CaptureError::Incomplete)?;
                (
                    ready.vectors.allowed.size_u32 != 0,
                    ready.player.player_id_i32,
                )
            }
            Progress::Write(w) => {
                let guard = w.guard.as_ref().ok_or(CaptureError::Incomplete)?;
                (
                    guard.vectors.allowed.size_u32 != 0,
                    guard.player.player_id_i32,
                )
            }
        };
        let military = player_ai_military::Military::decode_in_context(
            raw,
            crate::unit::Limits {
                max_bytes: raw.len(),
                max_items: limits.max_events,
            },
            player_ai_military::Context {
                operation_type_arrays_present: present,
            },
        )
        .map_err(|_| CaptureError::MilitaryWire)?;
        require(military.player_id == owner, CaptureError::Owner)?;
        let (states, defs) = match record.progress {
            Progress::Read(r) => {
                validate_initialization(&r)?;
                validate_read_vectors(&military, &r, r.definitions.as_ref())?;
                (
                    RoleStates::Read {
                        prior_owner_pointer: r.prior_owner_pointer,
                        prior: Box::new(r.prior),
                        initialize_prior: Box::new(
                            r.initialize_prior.ok_or(CaptureError::Incomplete)?,
                        ),
                        initialized: Box::new(r.initialized.ok_or(CaptureError::Incomplete)?),
                        read_initialized: Box::new(r.ready.ok_or(CaptureError::Incomplete)?),
                        read_return: Box::new(r.returned.ok_or(CaptureError::Incomplete)?),
                        has_ai_personality: r.personality.ok_or(CaptureError::Incomplete)?,
                        leader: r.leader,
                        query: r.query,
                    },
                    r.definitions,
                )
            }
            Progress::Write(w) => {
                let guard = w.guard.ok_or(CaptureError::Incomplete)?;
                if present {
                    military_arrays(
                        &military,
                        &guard,
                        w.definitions.as_ref().ok_or(CaptureError::Definitions)?,
                    )?;
                }
                (
                    RoleStates::Write {
                        entry: Box::new(w.entry),
                        guard: Box::new(guard),
                        returned: Box::new(w.returned.ok_or(CaptureError::Incomplete)?),
                    },
                    w.definitions,
                )
            }
        };
        let inputs = input_identities(&parsed, &record.binding)?;
        #[allow(unused_mut)]
        let mut snapshot = RoleSnapshot {
            capture: Arc::clone(&parsed.identity),
            binding: WireBinding {
                role: stream.kind,
                core_sha256: core_hash,
                occurrence_id: stream.occurrence.to_string().into_bytes(),
                output_id,
                inputs,
            },
            military_occurrence_id: s(&record.binding, "occurrence_id")?.to_owned(),
            thread_id: {
                let thread = array(&record.frame.thread)?;
                [
                    number(&thread[0])?,
                    number(&thread[1])?,
                    number(&thread[2])?,
                ]
            },
            frame_id: record.frame.id,
            events: record.events,
            lifecycle_id: s(&record.binding, "lifecycle_id")?.to_owned(),
            owner_lifetime_id: s(&record.binding, "owner_lifetime_id")?.to_owned(),
            stream_occurrence: stream.occurrence,
            stream_pointer: stream.pointer,
            core_range: [record.start, end],
            states,
            active_definitions: defs,
        };
        snapshots
            .try_reserve(1)
            .map_err(|_| CaptureError::ResourceLimit)?;
        #[cfg(test)]
        if origin == Origin::SourceOnly {
            snapshot.binding.role = match snapshot.binding.role {
                WireRole::ReadInputCore => WireRole::SourceOnlyReadCase,
                WireRole::SavedOutputCore => WireRole::SourceOnlyWriteCase,
                role => role,
            };
        }
        snapshots.push(Arc::new(InitializationSnapshot::from_role(
            snapshot, origin,
        )));
    }
    require(bound_outputs.is_empty(), CaptureError::Provenance)?;
    Ok(AdmittedCapture {
        identity: parsed.identity,
        snapshots,
    })
}

pub struct CapturePlayers<'a> {
    capture: &'a AdmittedCapture,
    binding: &'a WireBinding,
}
impl AdmittedCapture {
    /// Select one actual stream role/occurrence. Per-Military occurrence/frame
    /// metadata remains independent in every snapshot.
    pub fn for_binding<'a>(&'a self, binding: &'a WireBinding) -> CapturePlayers<'a> {
        CapturePlayers {
            capture: self,
            binding,
        }
    }
}
impl crate::core_player::PlayerInitializations for CapturePlayers<'_> {
    fn for_player(&self, _identity: PlayerIdentity) -> Option<Arc<InitializationSnapshot>> {
        None
    }
    fn for_player_at(
        &self,
        identity: PlayerIdentity,
        cursor: usize,
    ) -> Option<Arc<InitializationSnapshot>> {
        let mut found = self.capture.snapshots.iter().filter(|snapshot| {
            snapshot.captured_role().is_some_and(|r| {
                r.binding == *self.binding
                    && r.player_id() == identity.record_owner
                    && r.core_range[0] == cursor
            })
        });
        let first = found.next()?;
        if found.next().is_some() {
            None
        } else {
            Some(Arc::clone(first))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn hex(d: Digest) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }
    fn raw_vector(pointer: u64, values: &[i32]) -> Value {
        json!({"data_pointer":pointer,"capacity_u32":values.len(),"size_u32":values.len(),"padding_u32":u32::MAX,"tail_padding_u32":0x12345678,"values_i32":values})
    }
    fn raw_vectors(a: &[i32], p: &[i32], c: &[i32]) -> Value {
        json!({"allowed":raw_vector(0x3000,a),"per_war":raw_vector(0x3100,p),"current":raw_vector(0x3200,c)})
    }
    fn snapshot(vectors: Value) -> Value {
        json!({"military_pointer":0x1000,"player_pointer":0x2000,"player_id_i32":13,"was_ever_alive":true,"handicap_i32":-1,"vectors":vectors})
    }
    fn defs() -> Value {
        json!({"collection_pointer":0x7000,"begin_pointer":0x7100,"end_pointer":0x7110,"capacity_pointer":0x7110,"ordered":[{"pointer":0x7200,"database_row_i32":99,"collection_index_u32":1,"operation_type_hash_u32":17,"default_value_i32":42},{"pointer":0x7300,"database_row_i32":-8,"collection_index_u32":0,"operation_type_hash_u32":31,"default_value_i32":43}]})
    }
    fn military(write: bool) -> player_ai_military::Military {
        use player_ai_military::{HashValue, OperationTypes};
        let pairs = |a, b| {
            vec![
                HashValue { hash: 17, value: a },
                HashValue { hash: 31, value: b },
            ]
        };
        player_ai_military::Military {
            player_id: 13,
            operation_types: Some(OperationTypes {
                allowed: if write {
                    pairs(42, 43)
                } else {
                    pairs(-111, 999)
                },
                per_war: if write { pairs(8, 9) } else { pairs(-888, 777) },
                current: if write { pairs(-7, 99) } else { pairs(99, -7) },
            }),
            ..player_ai_military::Military::default()
        }
    }
    struct SourceLog {
        rows: Vec<Value>,
        sequence: u64,
    }
    impl SourceLog {
        fn plain(&mut self, event: &str, mut fields: Value) {
            fields["event"] = json!(event);
            self.rows.push(fields);
        }
        fn observation(&mut self, event: &str, mut fields: Value) {
            self.sequence += 1;
            fields["schema"] = json!("c6-linux-military-observation-v1");
            fields["sequence"] = json!(self.sequence);
            fields["thread"] = json!([1, 2, 3]);
            self.plain(event, fields);
        }
        fn cp(
            &mut self,
            event: &str,
            frame: &str,
            rsp: u64,
            ret: Option<u64>,
            mut registers: Value,
        ) {
            registers["pc"] =
                json!(site(event).map_or_else(|| ret.unwrap(), |rva| 0x10000000 + rva));
            registers["rsp"] = json!(rsp);
            self.plain("checkpoint",json!({"checkpoint":event,"frame_id":frame,"thread":[1,2,3],"actual_entry_return_pc":ret,"registers":registers}));
        }
        fn marker(&mut self, event: u64, occurrence: u64, path: &str, size: usize) {
            let offset = if event == 1 || event == 11 { 0 } else { size };
            self.marker_checkpoint(event, occurrence, size, offset);
            self.marker_row(event, occurrence, path, size, offset);
        }
        fn marker_checkpoint(&mut self, event: u64, occurrence: u64, size: usize, offset: usize) {
            let words = [
                1,
                event,
                occurrence,
                0x9000,
                0xa000,
                0xb000,
                size as u64,
                0xc100,
                0,
                offset as u64,
            ];
            let raw: String = words
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .map(|b| format!("{b:02x}"))
                .collect();
            self.plain("marker_checkpoint",json!({"descriptor_raw80":raw,"registers":{"pc":0xc000,"rdi":0xd000},"thread":[1,2,3]}));
        }
        fn marker_row(
            &mut self,
            event: u64,
            occurrence: u64,
            path: &str,
            size: usize,
            offset: usize,
        ) {
            if event == 4 || event == 15 {
                self.plain("stream_destroyed",json!({"marker":{"schema":1,"event":event,"occurrence":occurrence,"context":0x9000,"stream":0xa000,"buffer":0xb000,"path":0xc100,"size":size,"result":0,"offset":offset}}));
            } else {
                self.plain("stream_marker",json!({"schema":1,"marker_event":event,"occurrence":occurrence,"context":0x9000,"stream":0xa000,"buffer":0xb000,"path":path,"size":size,"offset":offset,"result":0}));
            }
        }
    }
    fn source_parsed<'a>(input: &'a [u8], output: &'a [u8]) -> ParsedArtifacts<'a> {
        let cfg = json!({"original_filesystem_profile":{"full_content_input_gate_sha256":hex([3;32])},"saved_configuration_sha256":hex([4;32]),"dlc_descriptors":[]});
        let configuration_sha256 = canonical_hash(&cfg).unwrap();
        let gate = json!({"source_files":{}});
        let plan = json!({"runtime_gate":gate,"observer_sha256":hex([5;32])});
        let identity = Arc::new(CaptureIdentity {
            raw_observation_sha256: [6; 32],
            runtime_plan_sha256: canonical_hash(&plan).unwrap(),
            runtime_plan_artifact_sha256: [11; 32],
            static_plan_artifact_sha256: [12; 32],
            exit_artifact_sha256: [13; 32],
            prepared_report_sha256: [7; 32],
            configuration_sha256,
            input_save_sha256: [8; 32],
            input_core_sha256: hash(input),
            live_database_snapshot_unavailable: true,
        });
        let mods = json!({"full_content_input_gate_sha256":hex([3;32]),"saved_configuration_sha256":hex([4;32])});
        let binding = |kind, occurrence, stream_occurrence, output_id| json!({"kind":kind,"occurrence_id":occurrence,"stream_occurrence":stream_occurrence,"lifecycle_id":"source-context","owner_lifetime_id":format!("source-{kind}-scope"),"owner_lifetime_kind":"selected_original_Read_or_Write_frame_only","stream_pointer":0xa000,"configuration_sha256":hex(configuration_sha256),"runtime_profile_sha256":hex(configuration_sha256),"runner_source_sha256":hex(canonical_hash(&json!({})).unwrap()),"rules_inputs_sha256":hex([3;32]),"rules_database_sha256":null,"live_database_snapshot_unavailable":true,"dlc_sha256":hex(canonical_hash(&json!([])).unwrap()),"mods_sha256":hex(canonical_hash(&mods).unwrap()),"input_save_sha256":hex([8;32]),"input_core_sha256":hex(hash(input)),"output_id":output_id,"core_start":0});
        let read = binding("read", "source-call-read", 9, None::<&str>);
        let write = binding("write", "source-call-write", 27, Some("source-output"));
        let prior = raw_vectors(&[-9], &[11, 12, 13], &[-8]);
        let initialized = raw_vectors(&[42, 43], &[8, 9], &[0, 0]);
        let returned = raw_vectors(&[42, 43], &[8, 9], &[-7, 99]);
        let query = json!({"handler_pointer":0x5000,"query_pointer":0x6000,"default_value_i32":42,"civilization_i32":4,"handicap_i32":-1,"captured_definition_pointer":0x7000,"count_callback_pointer":0x7400,"count_callback_vtable_pointer":0x7500});
        let observations = json!({"has_ai_personality":true,"leader_i32":12,"leader_query":query,"active_definitions":defs()});
        let mut log = SourceLog {
            rows: vec![],
            sequence: 0,
        };
        log.observation("observer_begin",json!({"gamecore_sha256":hex(CORE_IDENTITY),"host_sha256":hex(HOST_IDENTITY),"plan_sha256":hex(identity.runtime_plan_sha256),"runtime_gate":gate,"observer_sha256":hex([5;32])}));
        log.plain("installed",json!({"profile_sha256":hex(configuration_sha256),"hardware_target_lanes":3,"provenance_lanes":1,"software_game_breakpoints":false,"marker_symbol_address":0xc000,"marker_stop_pc":0xc000}));
        log.marker(1, 9, "source-input", input.len());
        log.cp(
            "read_entry",
            "root-read",
            0x8000,
            Some(0x8888),
            json!({"rdi":0xa000,"rsi":0x1000}),
        );
        log.observation("read_entry",json!({"binding":read,"frame_id":"root-read","military_pointer":0x1000,"stream_pointer":0xa000,"prior_owner_pointer":0,"snapshot":null,"prior_vectors":prior}));
        log.cp(
            "initialize_entry",
            "init",
            0x7000,
            Some(0x10000000 + 0x2145669),
            json!({"rdi":0x1000,"rsi":0x2000}),
        );
        log.plain("owner_installation_witness",json!({"read_frame":"root-read","initialize_frame":"init","military_pointer":0x1000,"actual_Player_argument":0x2000,"stored_Player_pointer":0x2000}));
        log.observation(
            "initialize_entry",
            json!({"binding":read,"frame_id":"init","snapshot":snapshot(prior)}),
        );
        log.cp("personality_return", "init", 0x6800, None, json!({"rax":1}));
        log.observation(
            "personality_return",
            json!({"binding":read,"frame_id":"init","observations":{"has_ai_personality":true}}),
        );
        log.cp("leader_return", "init", 0x6800, None, json!({"rax":12}));
        log.observation(
            "leader_return",
            json!({"binding":read,"frame_id":"init","observations":{"leader_i32":12}}),
        );
        log.cp(
            "query_entry",
            "init",
            0x6800,
            None,
            json!({"rdi":0x5000,"rsi":0x1000+0x158,"rcx":4,"r8":u32::MAX}),
        );
        log.observation(
            "query_entry",
            json!({"binding":read,"frame_id":"init","observations":{"leader_query":query}}),
        );
        log.cp(
            "count_entry",
            "count",
            0x6000,
            Some(0x10000000 + 0x203ac8b),
            json!({"rdi":0x7000,"rbx":0x5000,"rbp":0x1000+0x158}),
        );
        log.observation(
            "count_entry",
            json!({"binding":read,"frame_id":"count","definitions":defs()}),
        );
        log.cp(
            "count_return",
            "count",
            0x6008,
            Some(0x10000000 + 0x203ac8b),
            json!({"rax":2}),
        );
        log.observation(
            "count_return",
            json!({"frame_id":"count","count_u32":2,"definitions":defs()}),
        );
        log.cp(
            "initialize_return",
            "init",
            0x7008,
            Some(0x10000000 + 0x2145669),
            json!({}),
        );
        log.observation("initialize_return",json!({"binding":read,"frame_id":"init","snapshot":snapshot(initialized.clone()),"normal_native_return":true,"observations":observations}));
        log.observation(
            "read_initialized",
            json!({"binding":read,"frame_id":"root-read","snapshot":snapshot(initialized)}),
        );
        log.cp("read_return", "root-read", 0x8008, Some(0x8888), json!({}));
        log.observation("read_return",json!({"binding":read,"frame_id":"root-read","snapshot":snapshot(returned.clone()),"normal_native_return":true}));
        log.plain("original_stream_range",json!({"binding":read,"frame_id":"root-read","kind":"read","core_range":[0,input.len()]}));
        for event in [2, 3, 4] {
            log.marker(event, 9, "source-input", input.len());
        }
        log.marker(11, 27, "source-output", output.len() * 2);
        log.cp(
            "write_entry",
            "root-write",
            0x8000,
            Some(0x9999),
            json!({"rdi":0xa000,"rsi":0x1000}),
        );
        log.observation("write_entry",json!({"binding":write,"frame_id":"root-write","military_pointer":0x1000,"stream_pointer":0xa000,"prior_vectors":returned,"snapshot":snapshot(returned.clone())}));
        log.cp(
            "write_guard",
            "root-write",
            0x7800,
            None,
            json!({"rax":2,"rbx":0x1000,"r14":0xa000}),
        );
        log.observation("write_guard",json!({"binding":write,"frame_id":"root-write","snapshot":snapshot(returned.clone()),"operation_type_arrays_present":true}));
        for (i, name) in ["allowed", "per_war", "current"].into_iter().enumerate() {
            log.cp(
                "hash_array_entry",
                "root-write",
                0x7800,
                None,
                json!({"rdi":0xa000,"rsi":0x3000+i as u64*0x100,"rdx":0x7000}),
            );
            log.observation("hash_array_entry",json!({"binding":write,"frame_id":"root-write","name":name,"vector":returned[name],"definitions":defs()}));
        }
        log.cp(
            "write_return",
            "root-write",
            0x8008,
            Some(0x9999),
            json!({}),
        );
        log.observation("write_return",json!({"binding":write,"frame_id":"root-write","snapshot":snapshot(returned),"normal_native_return":true}));
        log.plain("original_stream_range",json!({"binding":write,"frame_id":"root-write","kind":"write","core_range":[0,output.len()]}));
        log.marker(12, 27, "source-output", output.len());
        // Actual source emits binding rows during marker13 handling, before its
        // trailing human-readable stream_marker row.
        log.marker_checkpoint(13, 27, output.len(), output.len());
        log.observation("saved_output_binding",json!({"occurrence_id":"source-call-write","output_id":"source-output","saved_output_sha256":hex(hash(output)),"output_core_sha256":hex(hash(output)),"military_core_range":[0,output.len()]}));
        log.marker_row(13, 27, "source-output", output.len(), output.len());
        for event in [14, 15] {
            log.marker(event, 27, "source-output", output.len());
        }
        log.plain(
            "final_live_original_method_fingerprints_equal",
            json!({"method_count":34}),
        );
        log.observation(
            "capture_end",
            json!({"observer_failed":false,"capture_complete":false}),
        );
        ParsedArtifacts {
            identity,
            plan,
            report: json!({"configuration":cfg.clone()}),
            rows: log.rows,
            input_core: input,
            inputs: expected_inputs(&cfg, configuration_sha256).unwrap(),
            runner_source_sha256: canonical_hash(&json!({})).unwrap(),
            outputs: vec![("source-output", output, hash(output))],
        }
    }
    #[test]
    fn raw_source_roles_frames_vectors_and_saved_bytes_are_distinct() {
        let input = military(false)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let output = military(true)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let parsed = source_parsed(&input, &output);
        let admitted =
            admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly).unwrap();
        assert!(!admitted.native_acceptance());
        assert_eq!(admitted.snapshots().len(), 2);
        let read = admitted.snapshots()[0].captured_role().unwrap();
        let write = admitted.snapshots()[1].captured_role().unwrap();
        assert_eq!(read.binding.role, WireRole::SourceOnlyReadCase);
        assert_eq!(write.binding.role, WireRole::SourceOnlyWriteCase);
        assert_ne!(read.binding.core_sha256, write.binding.core_sha256);
        assert_ne!(read.owner_lifetime_id, write.owner_lifetime_id);
        assert_eq!(read.stream_occurrence, 9);
        assert_eq!(read.binding.occurrence_id, b"9");
        assert_eq!(read.military_occurrence_id, "source-call-read");
        let player = PlayerIdentity {
            manager_index: 29,
            requested_owner: 13,
            record_owner: 13,
        };
        let provider = admitted.for_binding(&read.binding);
        use crate::core_player::PlayerInitializations;
        assert!(provider.for_player(player).is_none());
        assert!(provider.for_player_at(player, 0).is_some());
        assert!(provider.for_player_at(player, 1).is_none());
        let mut native = read.binding.clone();
        native.role = WireRole::ReadInputCore;
        assert_eq!(
            admitted.snapshots()[0].context_for(player, &native),
            Err(super::super::Error::SourceOnly)
        );
    }
    #[test]
    fn raw_source_admission_refuses_semantic_and_provenance_corruption() {
        let input = military(false)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let output = military(true)
            .encode(crate::unit::Limits::default())
            .unwrap();
        for (event, field, value) in [
            ("count_return", "count_u32", json!(3)),
            (
                "owner_installation_witness",
                "stored_Player_pointer",
                json!(0x2001),
            ),
            ("write_guard", "operation_type_arrays_present", json!(false)),
            (
                "saved_output_binding",
                "output_core_sha256",
                json!(hex([1; 32])),
            ),
            ("initialize_return", "normal_native_return", json!(false)),
            ("capture_end", "observer_failed", json!(true)),
        ] {
            let mut parsed = source_parsed(&input, &output);
            let row = parsed
                .rows
                .iter_mut()
                .find(|r| r["event"] == event)
                .unwrap();
            row[field] = value;
            assert!(
                admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly)
                    .is_err(),
                "{event}/{field}"
            );
        }
        let mut parsed = source_parsed(&input, &output);
        parsed
            .rows
            .iter_mut()
            .find(|r| r["event"] == "read_return")
            .unwrap()["snapshot"]["vectors"]["allowed"]["values_i32"][0] = json!(-111);
        assert!(matches!(
            admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly),
            Err(CaptureError::Vector)
        ));
        let mut parsed = source_parsed(&input, &output);
        parsed
            .rows
            .retain(|r| r["event"] != "owner_installation_witness");
        assert!(
            admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly).is_err()
        );
    }
    #[test]
    fn report_canonicalization_and_preparse_budgets_are_bounded() {
        assert_eq!(
            canonical(&json!({"z":"😀é","a":[1e-5,1e16]})).unwrap(),
            br#"{"a":[1e-05,1e+16],"z":"\ud83d\ude00\u00e9"}"#
        );
        assert!(matches!(json(b"[[]]", 3), Err(CaptureError::ResourceLimit)));
        let raw = raw_vector(0, &[1]);
        assert!(matches!(
            vector(&raw, CaptureLimits::default()),
            Err(CaptureError::Vector)
        ));
        let mut raw = defs();
        raw["capacity_pointer"] = json!(0x7109);
        assert!(matches!(
            definitions(&raw, CaptureLimits::default()),
            Err(CaptureError::Definitions)
        ));
    }
    #[test]
    fn source_raw_artifact_ingestion_hashes_preimages_and_never_admits_target_stubs() {
        let input = military(false)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let output = military(true)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let mut parsed = source_parsed(&input, &output);
        let source = b"explicitly synthetic observer source";
        let static_plan = json!({"schema":"c6-linux-military-observer-plan-v1","gamecore_sha256":hex(CORE_IDENTITY),"host_sha256":hex(HOST_IDENTITY),"observer_sha256":hex(hash(source))});
        let static_bytes = serde_json::to_vec(&static_plan).unwrap();
        let cfg = &mut parsed.report["configuration"];
        cfg["lower_core_military_initialization_observer"] = json!({"mode":"ordinary_load_save_passive","static_plan":static_plan,"static_plan_sha256":hex(hash(&static_bytes)),"input_save_artifact":{"sha256":hex(hash(b"source-save"))}});
        cfg["lower_core_stream"] = json!({"sha256":hex(hash(&input))});
        let config_hash = canonical_hash(cfg).unwrap();
        parsed.report["configuration_sha256"] = json!(hex(config_hash));
        let report_bytes = serde_json::to_vec(&parsed.report).unwrap();
        let sources = json!({"/source/military_initialization_observer.py":hex(hash(source))});
        let gate = json!({"source_files":sources,"configuration_sha256":hex(config_hash),"report_sha256":hex(hash(&report_bytes))});
        let mut plan = static_plan.clone();
        plan["runtime_gate"] = gate.clone();
        let plan_bytes = serde_json::to_vec(&plan).unwrap();
        for row in &mut parsed.rows {
            if row["event"] == "installed" {
                row["profile_sha256"] = json!(hex(config_hash));
            }
            if row["event"] == "observer_begin" {
                row["runtime_gate"] = gate.clone();
                row["plan_sha256"] = json!(hex(canonical_hash(&plan).unwrap()));
                row["observer_sha256"] = json!(hex(hash(source)));
            }
            if let Some(binding) = row.get_mut("binding") {
                binding["configuration_sha256"] = json!(hex(config_hash));
                binding["runtime_profile_sha256"] = json!(hex(config_hash));
                binding["runner_source_sha256"] = json!(hex(canonical_hash(&sources).unwrap()));
                binding["input_save_sha256"] = json!(hex(hash(b"source-save")));
            }
        }
        let mut raw = Vec::new();
        for row in parsed.rows {
            raw.extend(serde_json::to_vec(&row).unwrap());
            raw.push(b'\n');
        }
        let outputs = [NamedArtifact {
            name: "source-output",
            bytes: &output,
        }];
        let source_files = [NamedArtifact {
            name: "/source/military_initialization_observer.py",
            bytes: source,
        }];
        let artifacts = CaptureArtifacts {
            observations_jsonl: &raw,
            runtime_plan_json: &plan_bytes,
            static_plan_json: &static_bytes,
            prepared_report_json: &report_bytes,
            exit_json: br#"{"observer_failed":false}"#,
            target_module: b"source-only module stub",
            target_host: b"source-only host stub",
            input_save: b"source-save",
            input_core: &input,
            outputs: &outputs,
            source_files: &source_files,
        };
        // The public production route always verifies the real pinned targets.
        assert!(matches!(
            admit_capture(artifacts, CaptureLimits::default()),
            Err(CaptureError::TargetIdentity)
        ));
        // Only this private test route skips unavailable original image bytes;
        // its resulting snapshots remain SourceOnly and refuse native roles.
        let checked = parse_artifacts_with_images(
            artifacts,
            CaptureLimits::default(),
            TargetImages::SourceOnly,
        )
        .unwrap();
        let admitted =
            admit_parsed_with_origin(checked, CaptureLimits::default(), Origin::SourceOnly)
                .unwrap();
        assert_eq!(admitted.identity.raw_observation_sha256, hash(&raw));
        assert!(admitted
            .snapshots()
            .iter()
            .all(|s| s.origin() == Origin::SourceOnly));
        let mut changed = report_bytes.clone();
        changed.push(b' ');
        let bad = CaptureArtifacts {
            prepared_report_json: &changed,
            ..artifacts
        };
        assert!(matches!(
            parse_artifacts_with_images(bad, CaptureLimits::default(), TargetImages::SourceOnly),
            Err(CaptureError::ArtifactIdentity)
        ));
        let bad_sources = [NamedArtifact {
            name: "/source/military_initialization_observer.py",
            bytes: b"changed",
        }];
        let bad = CaptureArtifacts {
            source_files: &bad_sources,
            ..artifacts
        };
        assert!(matches!(
            parse_artifacts_with_images(bad, CaptureLimits::default(), TargetImages::SourceOnly),
            Err(CaptureError::ArtifactIdentity)
        ));
        let short = CaptureLimits {
            max_log_bytes: raw.len() - 1,
            ..CaptureLimits::default()
        };
        assert!(matches!(
            parse_artifacts_with_images(artifacts, short, TargetImages::SourceOnly),
            Err(CaptureError::ResourceLimit)
        ));
    }
    #[test]
    fn all_source_initializer_branches_preserve_their_distinct_vector_rules() {
        let limits = CaptureLimits::default();
        let prior = state(&snapshot(raw_vectors(&[5], &[6, 7, 8], &[9, 10])), limits).unwrap();
        let base = |initialized: CapturedState, personality, leader, query, count| ReadProgress {
            prior_owner_pointer: 0,
            prior: prior.vectors.clone(),
            initialize_prior: Some(prior.clone()),
            initialized: Some(initialized.clone()),
            ready: Some(initialized),
            returned: None,
            personality: Some(personality),
            leader,
            query,
            definitions: None,
            count,
            initialize_frame: None,
            count_frame: None,
            phase: 8,
            owner_installation: true,
        };
        let mut unchanged = prior.clone();
        unchanged.player.was_ever_alive_raw = 0;
        let mut false_case = base(unchanged.clone(), false, None, None, None);
        false_case.initialize_prior = Some(unchanged.clone());
        assert!(validate_initialization(&false_case).is_ok());
        false_case
            .ready
            .as_mut()
            .unwrap()
            .vectors
            .per_war
            .values_i32[0] += 1;
        assert_eq!(
            validate_initialization(&false_case),
            Err(CaptureError::Initialization)
        );
        let unknown = state(&snapshot(raw_vectors(&[0, 0], &[0, 0], &[0, 0])), limits).unwrap();
        assert!(validate_initialization(&base(unknown, true, Some(-1), None, Some(2))).is_ok());
        let query = QueryObservation {
            handler_pointer: 1,
            query_pointer: 0,
            default_value_i32: 42,
            civilization_i32: 9,
            handicap_i32: -1,
            captured_definition_pointer: None,
        };
        let null = state(
            &snapshot(raw_vectors(&[5], &[-9, -8, -7, -6], &[0])),
            limits,
        )
        .unwrap();
        assert!(
            validate_initialization(&base(null, true, Some(12), Some(query.clone()), None)).is_ok()
        );
        let mut nonnull = query;
        nonnull.query_pointer = 2;
        nonnull.captured_definition_pointer = Some(3);
        let zero = state(&snapshot(raw_vectors(&[], &[99, 98, 97], &[])), limits).unwrap();
        assert!(
            validate_initialization(&base(zero, true, Some(12), Some(nonnull), Some(0))).is_ok()
        );
    }
    #[test]
    fn source_raw_frame_hierarchy_and_every_incomplete_event_prefix_refuse() {
        let input = military(false)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let output = military(true)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let size = source_parsed(&input, &output).rows.len();
        for end in 0..size {
            let mut parsed = source_parsed(&input, &output);
            parsed.rows.truncate(end);
            assert!(
                admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly)
                    .is_err(),
                "prefix {end}"
            );
        }
        let mut parsed = source_parsed(&input, &output);
        parsed
            .rows
            .iter_mut()
            .find(|r| r["event"] == "checkpoint" && r["checkpoint"] == "count_return")
            .unwrap()["registers"]["rsp"] = json!(0x6009);
        assert!(matches!(
            admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly),
            Err(CaptureError::Frame)
        ));
        let mut parsed = source_parsed(&input, &output);
        parsed
            .rows
            .iter_mut()
            .find(|r| r["event"] == "read_entry")
            .unwrap()["binding"]["stream_occurrence"] = json!(27);
        assert!(matches!(
            admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly),
            Err(CaptureError::Stream)
        ));
    }
    #[test]
    fn final_fingerprint_is_terminal_and_read_ready_precedes_return_checkpoint() {
        let input = military(false)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let output = military(true)
            .encode(crate::unit::Limits::default())
            .unwrap();
        for position in [0, 1, 2] {
            let mut parsed = source_parsed(&input, &output);
            let index = parsed
                .rows
                .iter()
                .position(|r| r["event"] == "final_live_original_method_fingerprints_equal")
                .unwrap();
            let gate = parsed.rows.remove(index);
            let index = match position {
                0 => 0,
                1 => {
                    parsed
                        .rows
                        .iter()
                        .position(|r| r["event"] == "observer_begin")
                        .unwrap()
                        + 1
                }
                _ => {
                    parsed
                        .rows
                        .iter()
                        .position(|r| r["event"] == "stream_marker" && r["marker_event"] == 11)
                        .unwrap()
                        - 1
                }
            };
            parsed.rows.insert(index, gate);
            assert!(
                admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly)
                    .is_err(),
                "early gate {position}"
            );
        }
        for event in [
            "checkpoint",
            "stream_marker",
            "final_live_original_method_fingerprints_equal",
            "late_diagnostic",
        ] {
            let mut parsed = source_parsed(&input, &output);
            let index = parsed
                .rows
                .iter()
                .position(|r| r["event"] == "capture_end")
                .unwrap();
            parsed
                .rows
                .insert(index, json!({"event":event,"method_count":34}));
            assert!(
                matches!(
                    admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly),
                    Err(CaptureError::EventOrder)
                ),
                "aftergate {event}"
            );
        }
        let mut parsed = source_parsed(&input, &output);
        let index = parsed
            .rows
            .iter()
            .position(|r| r["event"] == "checkpoint" && r["checkpoint"] == "read_return")
            .unwrap();
        let cp = parsed.rows.remove(index);
        let ready = parsed
            .rows
            .iter()
            .position(|r| r["event"] == "read_initialized")
            .unwrap();
        parsed.rows.insert(ready, cp);
        assert!(matches!(
            admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly),
            Err(CaptureError::EventOrder)
        ));
    }
    #[test]
    fn hostile_raw_pointers_and_frame_span_ends_error_without_unwind() {
        let input = military(false)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let output = military(true)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let mut parsed = source_parsed(&input, &output);
        let hostile = u64::MAX - 7;
        for row in &mut parsed.rows {
            match row["event"].as_str().unwrap() {
                "read_entry" => row["military_pointer"] = json!(hostile),
                "owner_installation_witness" => row["military_pointer"] = json!(hostile),
                "checkpoint" if row["checkpoint"] == "read_entry" => {
                    row["registers"]["rsi"] = json!(hostile)
                }
                "checkpoint" if row["checkpoint"] == "initialize_entry" => {
                    row["registers"]["rdi"] = json!(hostile)
                }
                _ => {}
            }
            if row.get("snapshot").is_some_and(|s| !s.is_null()) {
                row["snapshot"]["military_pointer"] = json!(hostile);
            }
        }
        let outcome = std::panic::catch_unwind(|| {
            admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly)
        });
        assert!(matches!(outcome, Ok(Err(CaptureError::Owner))));
        let cp = Checkpoint {
            row_index: 0,
            name: "return".into(),
            registers: json!({"rsp":0,"pc":9}),
            frame: "frame".into(),
            thread: json!([1, 2, 3]),
            return_pc: None,
        };
        let frame = NativeFrame {
            id: "frame".into(),
            thread: json!([1, 2, 3]),
            entry_rsp: u64::MAX - 3,
            return_pc: 9,
        };
        assert!(matches!(
            std::panic::catch_unwind(|| frame.returned(&cp)),
            Ok(Err(CaptureError::Frame))
        ));
        assert_eq!(target_address(u64::MAX, 1), Err(CaptureError::Frame));
        let raw = raw_vector(u64::MAX - 3, &[1, 2]);
        assert!(matches!(
            std::panic::catch_unwind(|| vector(&raw, CaptureLimits::default())),
            Ok(Err(CaptureError::Vector))
        ));
    }
    #[test]
    fn allocated_stream_span_and_available_helper_metadata_refuse_contradictions() {
        let input = military(false)
            .encode(crate::unit::Limits::default())
            .unwrap();
        let output = military(true)
            .encode(crate::unit::Limits::default())
            .unwrap();
        // The raw descriptor and readable row agree on a hostile buffer; the
        // allocated-span check must still refuse without overflow/unwind.
        let mut streams = Vec::new();
        let mut active = None;
        let row = json!({"schema":1,"marker_event":1,"occurrence":1,"context":1,"stream":2,"buffer":u64::MAX-3,"size":8,"offset":0,"path":"source","result":0});
        let descriptor = MarkerDescriptor {
            words: [1, 1, 1, 1, 2, u64::MAX - 3, 8, 3, 0, 0],
        };
        assert!(matches!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| stream_marker(
                &row,
                &mut streams,
                &mut active,
                &descriptor
            ))),
            Ok(Err(CaptureError::Stream))
        ));
        for helper in ["query_entry", "hash_array_entry"] {
            for field in ["return", "stack"] {
                let mut parsed = source_parsed(&input, &output);
                let row = parsed
                    .rows
                    .iter_mut()
                    .find(|r| r["event"] == "checkpoint" && r["checkpoint"] == helper)
                    .unwrap();
                if field == "return" {
                    row["actual_entry_return_pc"] = json!(1);
                } else {
                    row["registers"]["rsp"] = json!(0x9000);
                }
                assert!(
                    matches!(
                        admit_parsed_with_origin(
                            parsed,
                            CaptureLimits::default(),
                            Origin::SourceOnly
                        ),
                        Err(CaptureError::Frame)
                    ),
                    "{helper}/{field}"
                );
            }
        }
        // Optional future metadata is checked if supplied; absence remains
        // explicit under the frozen producer's source-checked return guards.
        let mut parsed = source_parsed(&input, &output);
        let mut index = 0;
        for row in &mut parsed.rows {
            if row["event"] == "checkpoint" {
                if row["checkpoint"] == "query_entry" {
                    row["actual_entry_return_pc"] = json!(0x10000000 + 0x2145c9b);
                }
                if row["checkpoint"] == "hash_array_entry" {
                    row["actual_entry_return_pc"] =
                        json!(0x10000000 + [0x2145526u64, 0x2145548, 0x214556a][index]);
                    index += 1;
                }
            }
        }
        assert!(
            admit_parsed_with_origin(parsed, CaptureLimits::default(), Origin::SourceOnly).is_ok()
        );
    }
}
