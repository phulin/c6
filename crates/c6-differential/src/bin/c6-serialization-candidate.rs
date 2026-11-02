//! Native Random wire-record codec adapter, separate from simulator snapshots.
use c6_core::rng::{
    native_serialization::{self, SerializedRandomRecord},
    RandomState,
};
use c6_differential::EngineIdentity;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, BufRead},
};
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn identity() -> EngineIdentity {
    EngineIdentity {
        kind: "rust_serialization".into(),
        binary_sha256: hash(&fs::read(std::env::current_exe().unwrap()).unwrap()),
        configuration_sha256: hash(include_bytes!("../../serialization-candidate-config.json")),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    #[serde(default)]
    seed: u32,
    #[serde(default)]
    calls: u32,
    #[serde(default = "one")]
    resets: u32,
    #[serde(default)]
    lock_count: u32,
    #[serde(default)]
    synchronous_byte: u8,
}
fn one() -> u32 {
    1
}
impl Default for State {
    fn default() -> Self {
        Self {
            seed: 0,
            calls: 0,
            resets: 1,
            lock_count: 0,
            synchronous_byte: 0,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: String,
    expected_engine: EngineIdentity,
    op: String,
    #[serde(default)]
    state: State,
    bytes_hex: Option<String>,
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid hexadecimal bytes".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
fn execute(r: &Request) -> Result<Value, String> {
    let s = &r.state;
    let record = SerializedRandomRecord {
        seed: s.seed,
        calls: s.calls,
        resets: s.resets,
        synchronous_byte: s.synchronous_byte,
    };
    match r.op.as_str() {
        "write" => Ok(json!({"bytes_hex":hex(&record.encode())})),
        "read" => {
            let bytes = unhex(r.bytes_hex.as_deref().ok_or("missing bytes_hex")?)?;
            let (decoded, consumed) =
                SerializedRandomRecord::decode_prefix(&bytes).map_err(|e| format!("{e:?}"))?;
            // Canonical bytes additionally exercise the actual logical-state API.
            // Noncanonical bytes are raw wire records; never coerce them to bool.
            if decoded.synchronous_byte <= 1 {
                let mut state = RandomState {
                    seed: s.seed,
                    calls: s.calls,
                    resets: s.resets,
                    lock_count: s.lock_count,
                    synchronous: s.synchronous_byte != 0,
                };
                let n = native_serialization::read_into(&mut state, &bytes)
                    .map_err(|e| format!("{e:?}"))?;
                if n != consumed
                    || SerializedRandomRecord::from(state) != decoded
                    || state.lock_count != s.lock_count
                {
                    return Err("logical application differs from decoded record".into());
                }
            }
            Ok(
                json!({"state":{"seed":decoded.seed,"calls":decoded.calls,"resets":decoded.resets,"lock_count":s.lock_count,"synchronous_byte":decoded.synchronous_byte},"consumed":consumed,"trailing_hex":hex(&bytes[consumed..])}),
            )
        }
        _ => Err("unsupported operation".into()),
    }
}
fn main() {
    let engine = identity();
    if std::env::args().nth(1).as_deref() == Some("--identity") {
        println!("{}", serde_json::to_string(&engine).unwrap());
        return;
    }
    for line in io::stdin().lock().lines() {
        let line = line.expect("stdin read");
        let r: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("invalid request: {e}");
                std::process::exit(1)
            }
        };
        let mut response = json!({"id":r.id,"engine":engine});
        if r.expected_engine != engine {
            response["status"] = json!("error");
            response["message"] = json!("expected_engine identity mismatch");
        } else {
            match execute(&r) {
                Ok(fields) => {
                    response["status"] = json!("ok");
                    response
                        .as_object_mut()
                        .unwrap()
                        .extend(fields.as_object().unwrap().clone());
                }
                Err(e) => {
                    response["status"] = json!("unsupported");
                    response["message"] = json!(e);
                }
            }
        }
        println!("{response}");
    }
}
