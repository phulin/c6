//! Actual field-only Linux handler bindings plus separately source-derived
//! registration/queue initialization. No reverse-map or predicate return is
//! fabricated: ObservedRegistry always receives an empty hash-answer slice.
use super::resources::Bound;
use crate::unit_operation_manager::OperationDefinitions;
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    messages::{HandlerId, OperationHash},
    unit::{OperationNode, SKIP_TURN_HASH},
    unit_operation_interruption::{
        self as interruption, InterruptionTarget, ObservationIdentity, ObservedHandler,
        ObservedRegistry,
    },
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};
const GAME: &str = "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa";
const HOST: &str = "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4";
const PROFILE: &str = "321c39581fd268b9e75d2efaa2a672e8662a3468573bc1cad8bfbec8e15e7045";
const FACTS: &str = "77c31dd6dda9736588ee71c06637eb92ec002b0f7acacd780b30739b9ebef942";
const RUNTIME: &str = "ce40c14a3eb5882ef04d2524e2231d33b77737a1fc8c542a8ede1cf921184553";
const SNAPSHOT: &str = "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884";
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Identity(&'static str),
    Shape(&'static str),
    Registry(interruption::Error),
    Resources(super::resources::Error),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Queue {
        owner: i32,
        id: u32,
        field: &'static str,
    },
    UnregisteredHash(u32),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "observed handler adapter: {self:?}")
    }
}
impl std::error::Error for Error {}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn parse(bytes: &[u8]) -> Result<Value, Error> {
    serde_json::from_slice(bytes).map_err(Error::Json)
}
fn read(path: &Path, expected: &str, maximum: usize) -> Result<Vec<u8>, Error> {
    let file = std::fs::File::open(path).map_err(Error::Io)?;
    if file.metadata().map_err(Error::Io)?.len() > maximum as u64 {
        return Err(Error::Shape("artifact resource bound"));
    }
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(Error::Io)?;
    if bytes.len() > maximum || hash(&bytes) != expected {
        return Err(Error::Identity("closed artifact hash/size"));
    }
    Ok(bytes)
}
fn u(v: &Value, k: &'static str) -> Result<u64, Error> {
    v[k].as_u64().ok_or(Error::Shape(k))
}
fn word(v: &Value, k: &'static str) -> Result<u32, Error> {
    u(v, k)?.try_into().map_err(|_| Error::Shape(k))
}
fn int(v: &Value, k: &'static str) -> Result<i32, Error> {
    v[k].as_i64()
        .and_then(|x| i32::try_from(x).ok())
        .ok_or(Error::Shape(k))
}
fn bytes(v: &Value, k: &'static str) -> Result<Vec<u8>, Error> {
    let s = v[k].as_str().ok_or(Error::Shape(k))?;
    if !s.is_ascii() || s.len() % 2 != 0 {
        return Err(Error::Shape(k));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| Error::Shape(k)))
        .collect()
}
fn le32(b: &[u8], i: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        b.get(i..i + 4)
            .ok_or(Error::Shape("raw u32 extent"))?
            .try_into()
            .unwrap(),
    ))
}
fn le64(b: &[u8], i: usize) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(
        b.get(i..i + 8)
            .ok_or(Error::Shape("raw u64 extent"))?
            .try_into()
            .unwrap(),
    ))
}
fn one<'a>(rows: &'a [Value], kind: &'static str) -> Result<&'a Value, Error> {
    let mut found = rows.iter().filter(|r| r["kind"] == kind);
    let v = found.next().ok_or(Error::Shape(kind))?;
    if found.next().is_some() {
        return Err(Error::Shape(kind));
    }
    Ok(v)
}

/// Reads code only from an immutable SHA-gated original ELF, never executes it.
struct Image<'a> {
    bytes: &'a [u8],
}
impl<'a> Image<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, Error> {
        if hash(bytes) != GAME || bytes.get(..6) != Some(b"\x7fELF\x02\x01") {
            return Err(Error::Identity("original Linux module"));
        }
        Ok(Self { bytes })
    }
    fn code(&self, rva: u64, count: usize) -> Result<&'a [u8], Error> {
        let b = self.bytes;
        let ph = usize::try_from(le64(b, 32)?).map_err(|_| Error::Shape("ELF program headers"))?;
        let size = u16::from_le_bytes(
            b.get(54..56)
                .ok_or(Error::Shape("ELF header"))?
                .try_into()
                .unwrap(),
        ) as usize;
        let n = u16::from_le_bytes(
            b.get(56..58)
                .ok_or(Error::Shape("ELF header"))?
                .try_into()
                .unwrap(),
        ) as usize;
        if size != 56 || n > 512 {
            return Err(Error::Shape("ELF program headers"));
        }
        for i in 0..n {
            let p = ph
                .checked_add(i.checked_mul(size).ok_or(Error::Shape("ELF extent"))?)
                .ok_or(Error::Shape("ELF extent"))?;
            if le32(b, p)? != 1 || le32(b, p + 4)? & 5 != 5 {
                continue;
            }
            let start = le64(b, p + 16)?;
            let len = le64(b, p + 32)?;
            if rva < start
                || rva
                    .checked_add(count as u64)
                    .is_none_or(|end| end > start.saturating_add(len))
            {
                continue;
            }
            let offset = le64(b, p + 8)?
                .checked_add(rva - start)
                .and_then(|x| usize::try_from(x).ok())
                .ok_or(Error::Shape("ELF file extent"))?;
            return b
                .get(
                    offset
                        ..offset
                            .checked_add(count)
                            .ok_or(Error::Shape("ELF extent"))?,
                )
                .ok_or(Error::Shape("ELF code extent"));
        }
        Err(Error::Shape("unmapped executable original method"))
    }
}
fn method(
    row: &Value,
    base: u64,
    image: &Image<'_>,
    slot: u64,
    rva: u64,
    size: usize,
) -> Result<(), Error> {
    if u(row, "slot_offset")? != slot
        || u(row, "owner_base")? != base
        || u(row, "rva")? != rva
        || u(row, "actual_pointer")?
            != base
                .checked_add(rva)
                .ok_or(Error::Shape("actual method pointer"))?
        || [
            "vtable_slot_mapping_matches_hashed_file",
            "runtime_mapping_readable_executable",
            "runtime_mapping_matches_hashed_file",
        ]
        .iter()
        .any(|k| row[*k] != true)
    {
        return Err(Error::Identity("actual mapped slot/method"));
    }
    let live = bytes(row, "live_code_raw")?;
    if live.len() != size || live != image.code(rva, size)? {
        return Err(Error::Identity("complete live/original code"));
    }
    Ok(())
}
#[derive(Debug)]
struct NativeNode {
    id: HandlerId,
    directive: i32,
    flags: i32,
    push_turn: i32,
    activity: i32,
    automation: i32,
}
#[derive(Debug)]
struct NativeQueue {
    owner: i32,
    id: u32,
    nodes: Vec<NativeNode>,
}
/// Full original raw packet is retained, including unknown unused queue bytes.
/// Validating only known node fields does not classify or mask the rest.
pub struct Capture {
    raw: Vec<u8>,
    runtime: Vec<u8>,
    slots: Vec<Option<ObservedHandler>>,
    queues: Vec<NativeQueue>,
}
impl Capture {
    pub fn from_closed_capture(directory: &Path, module: &Path) -> Result<Self, Error> {
        let review_bytes =
            include_bytes!("../../../../evidence/linux/current-handler-closed-review.json");
        let review = parse(review_bytes)?;
        if hash(review_bytes) != "e0243a249b2971d5a726dfa5f64702a9fd6614eb4527038fd0300367f20d1b15"
            || review["review_passed"] != true
            || review["profile_sha256"] != PROFILE
            || review["facts_sha256"] != FACTS
            || review["runtime_sha256"] != RUNTIME
        {
            return Err(Error::Identity("independent closed capture admission"));
        }
        let runtime = read(&directory.join("runtime.json"), RUNTIME, 32 << 20)?;
        let prepared = parse(&read(
            &directory.join("prepared.json"),
            "6dbdc72ad84737fe99071d600f7a6397a8ae78e075c6001048b6760bcb70185b",
            32 << 20,
        )?)?;
        let configuration = parse(&read(
            &directory.join("configuration.json"),
            "fe6b65208f7b963d14211489f7c5c1f7311e77ca27766d3a3f82e7c1151c2f03",
            32 << 20,
        )?)?;
        let report = parse(&runtime)?;
        if report["configuration"] != configuration
            || prepared["configuration"] != configuration
            || report["configuration_sha256"] != PROFILE
            || prepared["configuration_sha256"] != PROFILE
            || hash(&serde_json::to_vec(&configuration).map_err(Error::Json)?) != PROFILE
            || report["returncode"] != 0
            || report["fixture_success"] != true
            || report["framework_cleanup_proven"] != true
            || report["client_cleanup_proven"] != true
        {
            return Err(Error::Identity("closed capture/configuration"));
        }
        read(
            &directory.join("controls.jsonl"),
            "79705e486e53b367e5f53be0f8cb2776550a67e0b08d5816ca60b0e74d4959fb",
            1 << 20,
        )?;
        for name in [
            "before.bin",
            "control.bin",
            "after.bin",
            "after-repeat.bin",
            "save.bin",
            "repeat.bin",
        ] {
            let data = read(&directory.join(name), SNAPSHOT, 80_657_462)?;
            if data.len() != 80_657_462 {
                return Err(Error::Identity("associated saved snapshot length"));
            }
        }
        let raw = read(&directory.join("facts.jsonl"), FACTS, 1_197_257)?;
        let repeat = read(&directory.join("facts-repeat.jsonl"), FACTS, 1_197_257)?;
        if raw.len() != 1_197_257 || raw != repeat {
            return Err(Error::Identity("complete identical raw passes"));
        }
        let module = read(module, GAME, 1 << 30)?;
        let image = Image::new(&module)?;
        Self::parse_raw(raw, runtime, &image)
    }
    fn parse_raw(raw: Vec<u8>, runtime: Vec<u8>, image: &Image<'_>) -> Result<Self, Error> {
        let rows = raw
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .map(parse)
            .collect::<Result<Vec<_>, _>>()?;
        if rows.len() != 2700 {
            return Err(Error::Shape("complete ordered packet rows"));
        }
        let profile = one(&rows, "profile")?;
        if rows.first() != Some(profile)
            || profile["schema"] != "c6-linux-current-handler-interruption-metadata-v1"
            || profile["profile_sha256"] != PROFILE
            || profile["gamecore_sha256"] != GAME
            || profile["host_sha256"] != HOST
            || profile["helper_direct_original_calls"] != false
            || profile["predicate_results"] != false
            || profile["caller_quiescence_callback_invoked"] != true
        {
            return Err(Error::Identity("field-only actual capture profile"));
        }
        let base = u(profile, "module_base")?;
        let manager = one(&rows, "manager")?;
        let manager_raw = bytes(manager, "manager_raw")?;
        if manager_raw.len() != 104 || le64(&manager_raw, 0)? != base + 0x3aff880 {
            return Err(Error::Shape("actual Manager object"));
        }
        let count = le64(&manager_raw, 32)?
            .checked_sub(le64(&manager_raw, 24)?)
            .filter(|n| n % 8 == 0)
            .map(|n| n / 8)
            .ok_or(Error::Shape("actual handler vector extent"))? as usize;
        let methods = manager["live_methods"]
            .as_array()
            .ok_or(Error::Shape("actual Manager methods"))?;
        if methods.len() != 4 {
            return Err(Error::Shape("all Manager methods"));
        }
        for (row, (slot, rva, size)) in methods.iter().zip([
            (0x88, 0x2da1650, 82),
            (0x98, 0x2da16f0, 40),
            (0xa0, 0x2da1720, 86),
            (0xd8, 0x2da2a60, 116),
        ]) {
            method(row, base, image, slot, rva, size)?;
        }
        let handlers = rows
            .iter()
            .filter(|r| r["kind"] == "handler")
            .collect::<Vec<_>>();
        if count != 63 || handlers.len() != count {
            return Err(Error::Shape("all63actual handler slots"));
        }
        let mut slots = Vec::with_capacity(count);
        for (index, row) in handlers.into_iter().enumerate() {
            if word(row, "slot")? as usize != index {
                return Err(Error::Shape("actual handler slot order"));
            }
            if u(row, "pointer")? == 0 {
                slots.push(None);
                continue;
            }
            let object = bytes(row, "handler_raw")?;
            if object.len() != 16
                || le64(&object, 0)? != u(row, "vptr")?
                || le32(&object, 8)? as i32 != int(row, "runtime_id")?
                || int(row, "runtime_id")? != index as i32
                || le32(&object, 12)? != word(row, "operation_hash")?
            {
                return Err(Error::Shape("actual handler ID/hash/vptr fields"));
            }
            let rva = u(&row["actual_method"], "rva")?;
            method(&row["actual_method"], base, image, 0x20, rva, 3)?;
            slots.push(Some(ObservedHandler {
                runtime_id: HandlerId(index as i32),
                operation_hash: OperationHash(word(row, "operation_hash")?),
                interruption_target: InterruptionTarget::LiveSlotObservation(rva),
            }));
        }
        let registry =
            ObservedRegistry::new(Self::identity(), &slots, &[]).map_err(Error::Registry)?;
        let memberships = rows
            .iter()
            .filter(|r| r["kind"] == "unit_membership")
            .collect::<Vec<_>>();
        let queue_rows = rows
            .iter()
            .filter(|r| r["kind"] == "unit_queue")
            .collect::<Vec<_>>();
        if memberships.len() != 1262 || queue_rows.len() != memberships.len() {
            return Err(Error::Shape("all1262Units/memberships/queues"));
        }
        let mut queues = Vec::with_capacity(queue_rows.len());
        let mut previous: Option<(i32, u32)> = None;
        for (membership, row) in memberships.into_iter().zip(queue_rows) {
            let owner = int(row, "owner")?;
            let id = word(row, "id")?;
            if int(membership, "owner")? != owner
                || word(membership, "id")? != id
                || u(membership, "unit")? != u(row, "unit")?
            {
                return Err(Error::Shape("matching actual Unit membership"));
            }
            let ordinal = word(membership, "ordinal")?;
            if ordinal
                != previous
                    .filter(|(p, _)| *p == owner)
                    .map_or(0, |(_, o)| o + 1)
            {
                return Err(Error::Shape("Unit native member order"));
            }
            previous = Some((owner, ordinal));
            let q = bytes(row, "queue_raw")?;
            let block = bytes(row, "block_allocation_raw")?;
            let n = word(row, "count")? as usize;
            let capacity = word(row, "capacity")? as usize;
            let address = u(row, "queue")?;
            let allocation = u(row, "block")?;
            if q.len() != 200
                || address
                    != u(row, "unit")?
                        .checked_add(0xc10)
                        .ok_or(Error::Shape("queue address"))?
                || le32(&q, 192)? as usize != n
                || le32(&q, 184)? as usize != capacity
                || le64(&q, 176)? != allocation
                || n > capacity
                || capacity.checked_mul(40) != Some(block.len())
            {
                return Err(Error::Shape("complete queue/header/block bytes"));
            }
            let inline = allocation >= address && allocation < address + 176;
            if row["inline"] != inline
                || (capacity == 0 && allocation != 0)
                || (capacity != 0 && allocation % 16 != 0)
                || (inline
                    && (capacity != 4
                        || allocation != (address + 15) & !15
                        || q.get(
                            (allocation - address) as usize
                                ..(allocation - address) as usize + block.len(),
                        ) != Some(block.as_slice())))
                || (!inline && capacity != 0 && capacity <= 4)
            {
                return Err(Error::Shape("actual queue allocation shape"));
            }
            let mut nodes = Vec::with_capacity(n);
            for node in block.as_chunks::<40>().0.iter().take(n) {
                let numeric = le32(node, 0)?;
                if numeric >= 0x800 || numeric as usize >= slots.len() {
                    return Err(Error::Shape("actual numeric queue operation"));
                }
                nodes.push(NativeNode {
                    id: HandlerId(numeric as i32),
                    directive: le32(node, 4)? as i32,
                    flags: le32(node, 24)? as i32,
                    push_turn: le32(node, 28)? as i32,
                    activity: le32(node, 32)? as i32,
                    automation: le32(node, 36)? as i32,
                });
            }
            if let Some(front) = nodes.first() {
                if row["native_gethandler_return_observed"] != false
                    || row["numeric_slot_selection_source_derived"] != true
                    || word(row, "native_front_bits")? != front.id.0 as u32
                {
                    return Err(Error::Shape(
                        "numeric front/source selection classification",
                    ));
                }
                let selected = registry.handler(front.id).map_err(Error::Registry)?;
                match selected {
                    Some(handler) => {
                        let slot = rows
                            .iter()
                            .find(|r| r["kind"] == "handler" && r["slot"] == front.id.0)
                            .ok_or(Error::Shape("actual selected slot"))?;
                        if u(row, "observed_numeric_slot_pointer")? != u(slot, "pointer")?
                            || word(row, "numeric_slot_handler_hash")?
                                != handler.identity().operation_hash.0
                            || u(row, "numeric_slot_handler_vptr")? != u(slot, "vptr")?
                            || u(row, "numeric_slot_actual_target")?
                                != base
                                    + match handler.identity().interruption_target {
                                        InterruptionTarget::LiveSlotObservation(rva) => rva,
                                        _ => return Err(Error::Shape("predicted live handler")),
                                    }
                        {
                            return Err(Error::Shape("actual selected slot fields"));
                        }
                    }
                    None => {
                        if u(row, "observed_numeric_slot_pointer")? != 0 {
                            return Err(Error::Shape("actual null selected slot"));
                        }
                    }
                }
            }
            queues.push(NativeQueue { owner, id, nodes });
        }
        let complete = one(&rows, "complete")?;
        if rows.last() != Some(complete)
            || word(complete, "handler_slots")? != 63
            || word(complete, "player_slots")? != 65
        {
            return Err(Error::Shape("complete field packet"));
        }
        Ok(Self {
            raw,
            runtime,
            slots,
            queues,
        })
    }
    fn identity() -> ObservationIdentity<'static> {
        ObservationIdentity {
            gamecore_sha256: GAME,
            configuration_sha256: PROFILE,
            capture_sha256: FACTS,
        }
    }
    pub fn registry(&self) -> Result<ObservedRegistry<'_>, Error> {
        ObservedRegistry::new(Self::identity(), &self.slots, &[]).map_err(Error::Registry)
    }
    pub fn raw_facts(&self) -> &[u8] {
        &self.raw
    }
    /// Source Register assigns definition collection index to the actual handler
    /// ID and its reverse-map entry. This is a reconstruction, never an observed
    /// GetHandlerIDFromHash answer. All initial queue IDs are independently
    /// compared with the actual numeric raw nodes before admission.
    pub fn admit<'a>(
        &'a self,
        state: &LoadedState,
        bound: &Bound<'_>,
    ) -> Result<BoundHandlers<'a>, Error> {
        if !state.is_pristine() {
            return Err(Error::Identity(
                "handler snapshot admission requires pristine state",
            ));
        }
        if bound.destination_binding() != state.binding() {
            return Err(Error::Identity("handler snapshot destination binding"));
        }
        let witness = state
            .observed_snapshot()
            .ok_or(Error::Identity("observed saved-snapshot controls required"))?;
        if witness.snapshot_bytes != 80_657_462
            || hex_digest(&witness.snapshot_sha256) != SNAPSHOT
            || hash(state.source_image()) != SNAPSHOT
        {
            return Err(Error::Identity("actual associated complete snapshot"));
        }
        bound
            .validate_static_runtime(&self.runtime)
            .map_err(Error::Resources)?;
        let mut remap = Vec::new();
        for handler in self.slots.iter().flatten() {
            let id = bound
                .operations()
                .find_index(handler.operation_hash.0)
                .ok_or(Error::UnregisteredHash(handler.operation_hash.0))?;
            if id.0 != handler.runtime_id.0
                || remap
                    .iter()
                    .any(|(hash, _)| *hash == handler.operation_hash)
            {
                return Err(Error::Identity(
                    "source Register definition/actual handler correspondence",
                ));
            }
            remap.push((handler.operation_hash, handler.runtime_id));
        }
        let mut initialized = Vec::with_capacity(self.queues.len());
        let mut index = 0usize;
        for player in state.world().players() {
            for entry in player.unit_records().map_err(Error::World)? {
                let observed = self
                    .queues
                    .get(index)
                    .ok_or(Error::Shape("complete Unit queue comparison"))?;
                index += 1;
                let u = &entry.value;
                if (player.owner(), u.id) != (observed.owner, observed.id)
                    || u.k_operations.nodes.len() != observed.nodes.len()
                {
                    return Err(Error::Queue {
                        owner: player.owner(),
                        id: u.id,
                        field: "ordered Unit/count",
                    });
                }
                let mut nodes = Vec::with_capacity(observed.nodes.len());
                for (wire, native) in u.k_operations.nodes.iter().zip(&observed.nodes) {
                    let id = remap
                        .iter()
                        .find(|(hash, _)| hash.0 == wire.operation_hash)
                        .map(|(_, id)| *id)
                        .ok_or(Error::UnregisteredHash(wire.operation_hash))?;
                    if (
                        id,
                        wire.directive,
                        wire.flags,
                        wire.push_turn,
                        wire.previous_activity,
                        wire.previous_automation,
                    ) != (
                        native.id,
                        native.directive,
                        native.flags,
                        native.push_turn,
                        native.activity,
                        native.automation,
                    ) {
                        return Err(Error::Queue {
                            owner: player.owner(),
                            id: u.id,
                            field: "remapped ID/known Node fields",
                        });
                    }
                    nodes.push(RuntimeNode {
                        runtime_id: id,
                        wire: wire.clone(),
                    });
                }
                initialized.push(RuntimeQueue {
                    owner: player.owner(),
                    id: u.id,
                    nodes,
                });
            }
        }
        if index != self.queues.len() {
            return Err(Error::Shape("unfiltered complete owned roster"));
        }
        let requested = remap
            .iter()
            .find(|(hash, _)| *hash == OperationHash(SKIP_TURN_HASH))
            .map(|(_, id)| *id)
            .ok_or(Error::UnregisteredHash(SKIP_TURN_HASH))?;
        Ok(BoundHandlers {
            capture: self,
            destination: state.binding().clone(),
            remap,
            requested,
            initialized,
        })
    }
}
fn hex_digest(value: &[u8; 32]) -> String {
    value.iter().map(|b| format!("{b:02x}")).collect()
}
#[derive(Debug)]
pub struct RuntimeNode {
    pub runtime_id: HandlerId,
    pub wire: OperationNode,
}
#[derive(Debug)]
pub struct RuntimeQueue {
    pub owner: i32,
    pub id: u32,
    pub nodes: Vec<RuntimeNode>,
}
/// Typed evidence authority; initialization is admitted only at the associated
/// untouched snapshot. Initial queue diagnostics are distinct from later state.
pub struct BoundHandlers<'a> {
    capture: &'a Capture,
    destination: WireBinding,
    remap: Vec<(OperationHash, HandlerId)>,
    requested: HandlerId,
    initialized: Vec<RuntimeQueue>,
}
impl BoundHandlers<'_> {
    pub fn registry(&self) -> Result<ObservedRegistry<'_>, Error> {
        self.capture.registry()
    }
    pub fn initial_runtime_queues(&self) -> &[RuntimeQueue] {
        &self.initialized
    }
    pub fn requested_skip_turn(&self) -> HandlerId {
        self.requested
    }
    pub(crate) fn runtime_id(&self, hash: u32) -> Result<HandlerId, Error> {
        self.remap
            .iter()
            .find(|(h, _)| h.0 == hash)
            .map(|(_, id)| *id)
            .ok_or(Error::UnregisteredHash(hash))
    }
    pub(crate) fn validate_execution(
        &self,
        state: &LoadedState,
        registry: &ObservedRegistry<'_>,
        definitions: &OperationDefinitions,
    ) -> Result<(), Error> {
        if state.binding() != &self.destination
            || state
                .observed_snapshot()
                .is_none_or(|w| hex_digest(&w.snapshot_sha256) != SNAPSHOT)
            || registry.identity() != Capture::identity()
            || definitions.count() != self.capture.slots.len()
        {
            return Err(Error::Identity("handler execution authority"));
        }
        for (index, expected) in self.capture.slots.iter().enumerate() {
            let actual = registry
                .handler(HandlerId(index as i32))
                .map_err(Error::Registry)?
                .map(|h| h.identity());
            if actual != *expected
                || expected.is_some_and(|h| {
                    definitions
                        .find_index(h.operation_hash.0)
                        .is_none_or(|d| d.0 != h.runtime_id.0)
                })
            {
                return Err(Error::Identity("handler/definition authority changed"));
            }
        }
        Ok(())
    }
    pub fn provenance(&self) -> Value {
        json!({"schema":"c6-linux-live-handler-binding-source-v1","observed_profile_sha256":PROFILE,"observed_raw_facts_sha256":FACTS,"observed_runtime_sha256":RUNTIME,"associated_saved_snapshot_sha256":SNAPSHOT,"observation_identity_unchanged":true,"live_slot_code_observation":true,"closed_review_sha256":"e0243a249b2971d5a726dfa5f64702a9fd6614eb4527038fd0300367f20d1b15","hash_lookup_answers_supplied":0,"native_gethandler_return_observed":false,"native_caninterrupt_return_observed":false,"runtime_queue_ids":"source_register_remap_compared_with_actual_numeric_nodes","parameters_pointees_observed":false,"native3_read_live_equivalence":false,"full_action_acceptance":false})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires independently closed private handler capture and original ELF; no original execution"]
    fn actual_live_slots_have_no_reverse_lookup_answers() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let capture = Capture::from_closed_capture(
            &root.join("c6-ghidra/evidence/linux-current-handler-metadata/native-3"),
            &root.join("c6-assets-linux/libGameCore_XP2.so"),
        )
        .unwrap();
        assert_eq!(capture.raw_facts().len(), 1_197_257);
        assert_eq!(capture.queues.len(), 1262);
        assert_eq!(
            capture
                .queues
                .iter()
                .filter(|q| !q.nodes.is_empty())
                .count(),
            308
        );
        let registry = capture.registry().unwrap();
        assert_eq!(registry.identity().configuration_sha256, PROFILE);
        assert_eq!(registry.identity().capture_sha256, FACTS);
        assert!(matches!(
            registry.saved_hash_runtime_id(OperationHash(SKIP_TURN_HASH)),
            Err(interruption::Error::UnobservedHash(OperationHash(
                SKIP_TURN_HASH
            )))
        ));
        let mut true_leaves = 0;
        let mut false_leaves = 0;
        for id in 0..63 {
            let handler = registry.handler(HandlerId(id)).unwrap().unwrap();
            if handler.can_be_interrupted() {
                true_leaves += 1
            } else {
                false_leaves += 1
            }
        }
        assert_eq!((true_leaves, false_leaves), (62, 1));
        let skip = registry.handler(HandlerId(36)).unwrap().unwrap();
        assert_eq!(
            skip.identity().operation_hash,
            OperationHash(SKIP_TURN_HASH)
        );
        assert_eq!(
            capture
                .queues
                .iter()
                .filter(|q| q.nodes.first().is_some_and(|n| n.id == HandlerId(18)))
                .count(),
            185
        );
    }
}
