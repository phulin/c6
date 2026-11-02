//! Process adapter for verified primitive RNG only; never runs game turns.
use c6_core::{rng::RandomState, Engine, Outcome, RandomOperation, Snapshot};
use c6_differential::{EngineIdentity, Request, Response, RngEvent, Status, StepObservation};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, BufRead},
};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn identity() -> Result<EngineIdentity, String> {
    let binary =
        fs::read(std::env::current_exe().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(EngineIdentity {
        kind: "rust".into(),
        binary_sha256: digest(&binary),
        configuration_sha256: digest(include_bytes!("../../candidate-config.json")),
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Initial {
    rng: Option<State>,
    rng_streams: Option<BTreeMap<u32, State>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    seed: u32,
    calls: u32,
    resets: u32,
    lock_count: u32,
    synchronous: bool,
}
fn snapshot(r: &RandomState) -> Value {
    json!({"rng":{"seed":r.seed,"calls":r.calls,"resets":r.resets,"lock_count":r.lock_count,"synchronous":r.synchronous}})
}
#[derive(Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
enum Operation {
    #[serde(rename = "plot_valid")]
    PlotValid { x: i32, y: i32 },
    #[serde(rename = "plot_distance")]
    PlotDistance {
        width: i32,
        wrap: bool,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    },
    #[serde(rename = "rng_get")]
    Get {
        bound: u32,
        #[serde(default)]
        stream_id: u32,
        #[serde(default = "stream")]
        stream: String,
    },
    #[serde(rename = "rng_reset")]
    Reset {
        seed: u32,
        #[serde(default)]
        stream_id: u32,
    },
    #[serde(rename = "rng_initialize")]
    Initialize {
        seed: u32,
        #[serde(default)]
        stream_id: u32,
    },
    #[serde(rename = "rng_lock")]
    Lock {
        #[serde(default)]
        stream_id: u32,
    },
    #[serde(rename = "rng_unlock")]
    Unlock {
        #[serde(default)]
        stream_id: u32,
    },
}
fn stream() -> String {
    "sync".into()
}
fn execute(request: &Request) -> Result<Vec<StepObservation>, String> {
    let Initial {
        rng: state,
        rng_streams,
    } = serde_json::from_value(request.case.initial_state.clone()).map_err(|e| e.to_string())?;
    let has_default = state.is_some();
    let rng = state.map(|s| RandomState {
        seed: s.seed,
        calls: s.calls,
        resets: s.resets,
        lock_count: s.lock_count,
        synchronous: s.synchronous,
    });
    let streams = rng_streams.map(|states| {
        states
            .into_iter()
            .map(|(id, s)| {
                (
                    id,
                    RandomState {
                        seed: s.seed,
                        calls: s.calls,
                        resets: s.resets,
                        lock_count: s.lock_count,
                        synchronous: s.synchronous,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>()
    });
    if streams.as_ref().is_some_and(|s| s.contains_key(&0)) {
        return Err("stream ID zero belongs in initial_state.rng".into());
    }
    let include_streams = streams.is_some();
    let mut core_snapshot = Snapshot::new(rng.unwrap_or(RandomState {
        resets: 1,
        ..RandomState::default()
    }));
    core_snapshot.random_streams = streams.unwrap_or_default();
    let mut engine = Engine::new(core_snapshot).map_err(|e| e.to_string())?;
    let mut observations = Vec::new();
    for (step_index, step) in request.case.steps.iter().enumerate() {
        let op: Operation = serde_json::from_value(step.clone())
            .map_err(|e| format!("unsupported operation: {e}"))?;
        match op {
            Operation::PlotValid { x, y } => {
                observations.push(StepObservation {
                    step_index,
                    snapshot: json!({"result":c6_core::geometry::plot_valid(x,y)}),
                    rng_trace: vec![],
                });
                continue;
            }
            Operation::PlotDistance {
                width,
                wrap,
                x1,
                y1,
                x2,
                y2,
            } => {
                observations.push(StepObservation { step_index, snapshot: json!({"result":c6_core::geometry::plot_distance(width,wrap,x1,y1,x2,y2)}), rng_trace: vec![] });
                continue;
            }
            _ => {}
        }
        let stream_id = match &op {
            Operation::Get { stream_id, .. }
            | Operation::Reset { stream_id, .. }
            | Operation::Initialize { stream_id, .. }
            | Operation::Lock { stream_id }
            | Operation::Unlock { stream_id } => *stream_id,
            _ => unreachable!(),
        };
        if stream_id == 0 && !has_default {
            return Err("RNG default instance requires explicit initial state".into());
        }
        let mut rng_trace = vec![];
        match op {
            Operation::Get { bound, stream, .. } => {
                let Outcome::Random(value) = engine
                    .apply_random(
                        stream_id,
                        RandomOperation::Draw {
                            bound: bound as u16,
                        },
                    )
                    .map_err(|e| e.to_string())?
                else {
                    return Err("unexpected draw outcome".into());
                };
                rng_trace.push(RngEvent {
                    stream,
                    ordinal: u64::from(
                        engine
                            .random_state(stream_id)
                            .map_err(|e| e.to_string())?
                            .calls,
                    ),
                    operation: "get".into(),
                    arguments: json!({"bound":bound,"effective_bound":bound&65535,"stream_id":stream_id}),
                    result: json!(value),
                });
            }
            Operation::Reset { seed, .. } => {
                engine
                    .apply_random(stream_id, RandomOperation::Reset { seed })
                    .map_err(|e| e.to_string())?;
            }
            Operation::Initialize { seed, .. } => {
                engine
                    .apply_random(stream_id, RandomOperation::Initialize { seed })
                    .map_err(|e| e.to_string())?;
            }
            Operation::Lock { .. } => {
                engine
                    .apply_random(stream_id, RandomOperation::Lock)
                    .map_err(|e| e.to_string())?;
            }
            Operation::Unlock { .. } => {
                engine
                    .apply_random(stream_id, RandomOperation::Unlock)
                    .map_err(|e| e.to_string())?;
            }
            _ => unreachable!(),
        }
        let mut state = snapshot(&engine.snapshot().random);
        if include_streams {
            state["rng_streams"] = Value::Object(
                engine
                    .snapshot()
                    .random_streams
                    .iter()
                    .map(|(id, r)| (id.to_string(), snapshot(r)["rng"].clone()))
                    .collect(),
            );
        }
        observations.push(StepObservation {
            step_index,
            snapshot: state,
            rng_trace,
        });
    }
    Ok(observations)
}
fn main() {
    let engine = identity().expect("cannot hash candidate executable");
    if std::env::args().nth(1).as_deref() == Some("--identity") {
        println!("{}", serde_json::to_string(&engine).unwrap());
        return;
    }
    for line in io::stdin().lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1)
            }
        };
        let request: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("invalid request: {e}");
                std::process::exit(1)
            }
        };
        let mut response = Response {
            protocol_version: 1,
            status: Status::Error,
            engine: engine.clone(),
            case_id: request.case.id.clone(),
            steps: vec![],
            coverage: [
                "rng_get",
                "rng_reset",
                "rng_initialize",
                "rng_lock",
                "rng_unlock",
                "plot_valid",
                "plot_distance",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            message: None,
        };
        if request.protocol_version != 1 {
            response.message = Some("unsupported protocol version".into());
        } else if request.expected_engine != engine {
            response.message = Some("engine identity mismatch".into());
        } else {
            match execute(&request) {
                Ok(steps) => {
                    response.status = Status::Ok;
                    response.steps = steps;
                }
                Err(e) => {
                    response.status = Status::Unsupported;
                    response.message = Some(e);
                }
            }
        }
        println!("{}", serde_json::to_string(&response).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(steps: Vec<Value>) -> Request {
        Request {
            protocol_version: 1,
            expected_engine: EngineIdentity {
                kind: "rust".into(),
                binary_sha256: "a".repeat(64),
                configuration_sha256: "b".repeat(64),
            },
            case: c6_differential::Case {
                id: "atomic".into(),
                initial_state: json!({"rng":{"seed":1,"calls":0,"resets":0,"lock_count":0,"synchronous":false}}),
                steps,
            },
        }
    }
    #[test]
    fn unsupported_action_after_draw_returns_no_partial_observations() {
        let r = request(vec![
            json!({"op":"rng_get","bound":10}),
            json!({"op":"end_turn"}),
        ]);
        assert!(execute(&r).is_err());
        // Each request starts from its supplied state. A prior failed case has no effect.
        let clean = request(vec![json!({"op":"rng_get","bound":10})]);
        assert_eq!(
            execute(&clean).unwrap()[0].snapshot["rng"]["calls"],
            json!(1)
        );
    }
    #[test]
    fn reject_unknown_fields_and_missing_rng_state() {
        let r = request(vec![json!({"op":"rng_get","bound":10,"ignored":true})]);
        assert!(execute(&r).is_err());
        let mut r = request(vec![json!({"op":"rng_get","bound":10})]);
        r.case.initial_state = json!({});
        assert!(execute(&r).is_err());
        r.case.initial_state = json!({"game":{}});
        assert!(execute(&r).is_err());
    }
}
