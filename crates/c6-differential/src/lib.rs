//! Fail-closed, engine-independent differential protocol. JSON snapshots describe
//! observed state; this crate does not infer or implement game mechanics.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineIdentity {
    pub kind: String,
    pub binary_sha256: String,
    pub configuration_sha256: String,
}
impl EngineIdentity {
    fn validate(&self) -> Result<(), String> {
        if self.kind.is_empty() {
            return Err("engine kind is empty".into());
        }
        for (name, value) in [
            ("binary_sha256", &self.binary_sha256),
            ("configuration_sha256", &self.configuration_sha256),
        ] {
            if value.len() != 64
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(format!(
                    "{name} must be 64 lowercase hexadecimal characters"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// e.g. native_observation, disassembly, or hypothesis. Never silently promoted.
    pub level: String,
    pub references: Vec<String>,
    pub coverage: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub initial_state: Value,
    /// Explicit operation objects, defined by each engine's declared capabilities.
    pub steps: Vec<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema_version: u32,
    pub oracle: EngineIdentity,
    pub candidate: EngineIdentity,
    pub evidence: Evidence,
    pub cases: Vec<Case>,
}
impl Corpus {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("unsupported corpus schema version".into());
        }
        self.oracle.validate()?;
        self.candidate.validate()?;
        if self.oracle.kind != "original_dll" {
            return Err("oracle must identify as original_dll".into());
        }
        if self.candidate.kind != "rust" {
            return Err("candidate must identify as rust".into());
        }
        if self.evidence.level.is_empty()
            || self.evidence.references.is_empty()
            || self.evidence.coverage.is_empty()
        {
            return Err("evidence level, references, and coverage are required".into());
        }
        if self.cases.is_empty() {
            return Err("corpus has no cases".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for case in &self.cases {
            if case.id.is_empty() || !ids.insert(&case.id) {
                return Err("case IDs must be nonempty and unique".into());
            }
            if case.steps.is_empty() {
                return Err(format!("case {} has no steps", case.id));
            }
            if case.steps.iter().any(|s| !s.is_object()) {
                return Err("steps must be operation objects".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol_version: u32,
    pub expected_engine: EngineIdentity,
    pub case: Case,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RngEvent {
    pub stream: String,
    pub ordinal: u64,
    pub operation: String,
    pub arguments: Value,
    pub result: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepObservation {
    pub step_index: usize,
    pub snapshot: Value,
    pub rng_trace: Vec<RngEvent>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Unsupported,
    Unavailable,
    Error,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol_version: u32,
    pub status: Status,
    pub engine: EngineIdentity,
    pub case_id: String,
    pub steps: Vec<StepObservation>,
    pub coverage: Vec<String>,
    #[serde(default)]
    pub message: Option<String>,
}

pub trait Runner {
    fn run(&mut self, request: &Request) -> Result<Response, String>;
}

/// Raw protocol evidence, including rejected responses, for reproducible audits.
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionRecord {
    pub request: Request,
    pub response: Option<Response>,
    pub error: Option<String>,
}
pub struct RecordingRunner<R> {
    pub inner: R,
    pub records: Vec<ExecutionRecord>,
}
impl<R> RecordingRunner<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            records: vec![],
        }
    }
}
impl<R: Runner> Runner for RecordingRunner<R> {
    fn run(&mut self, request: &Request) -> Result<Response, String> {
        let result = self.inner.run(request);
        self.records.push(ExecutionRecord {
            request: request.clone(),
            response: result.as_ref().ok().cloned(),
            error: result.as_ref().err().cloned(),
        });
        result
    }
}

/// One isolated child per case. Bounded stdout/stderr and a deadline ensure a
/// broken oracle cannot hang a corpus run or exhaust host memory.
pub struct ProcessRunner {
    pub program: String,
    pub args: Vec<String>,
    pub timeout: Duration,
    pub output_limit: usize,
}
impl Runner for ProcessRunner {
    fn run(&mut self, request: &Request) -> Result<Response, String> {
        let mut child = Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("oracle unavailable ({}): {e}", self.program))?;
        let mut input = serde_json::to_vec(request).map_err(|e| e.to_string())?;
        input.push(b'\n');
        let mut stdin = child.stdin.take().ok_or("child stdin missing")?;
        let writer = thread::spawn(move || stdin.write_all(&input));
        let stdout = child.stdout.take().ok_or("child stdout missing")?;
        let stderr = child.stderr.take().ok_or("child stderr missing")?;
        let limit = self.output_limit;
        let out = thread::spawn(move || read_bounded(stdout, limit));
        let err = thread::spawn(move || read_bounded(stderr, limit));
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break Ok(s),
                Ok(None) if start.elapsed() < self.timeout => {
                    thread::sleep(Duration::from_millis(5))
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err("oracle deadline exceeded".to_string());
                }
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(format!("oracle wait failed: {e}"));
                }
            }
        };
        // A descendant could retain pipe handles. Do not join readers on failure.
        let status = status?;
        if !out.is_finished() || !err.is_finished() || !writer.is_finished() {
            let grace = Instant::now();
            while (!out.is_finished() || !err.is_finished() || !writer.is_finished())
                && grace.elapsed() < Duration::from_millis(100)
            {
                thread::sleep(Duration::from_millis(1));
            }
        }
        if !out.is_finished() || !err.is_finished() || !writer.is_finished() {
            return Err("oracle left inherited pipes open".into());
        }
        let stdout = out.join().map_err(|_| "stdout reader panicked")??;
        let stderr = err.join().map_err(|_| "stderr reader panicked")??;
        writer
            .join()
            .map_err(|_| "stdin writer panicked")?
            .map_err(|e| format!("oracle input failed: {e}"))?;
        if !status.success() {
            return Err(format!(
                "oracle exited {status}: {}",
                String::from_utf8_lossy(&stderr)
            ));
        }
        serde_json::from_slice(&stdout).map_err(|e| format!("invalid oracle response: {e}"))
    }
}
fn read_bounded(mut r: impl Read, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    let mut overflow = false;
    loop {
        let n = r.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if bytes.len().saturating_add(n) > limit {
            overflow = true;
        } else if !overflow {
            bytes.extend_from_slice(&chunk[..n]);
        }
    }
    if overflow {
        Err("oracle output limit exceeded".into())
    } else {
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Divergence {
    pub case_id: String,
    pub step_index: usize,
    pub path: String,
    pub oracle: Value,
    pub candidate: Value,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub cases: usize,
    pub steps: usize,
    pub evidence_level: String,
    pub coverage: Vec<String>,
}

fn validate_response(
    request: &Request,
    response: &Response,
    coverage: &[String],
) -> Result<(), String> {
    if response.protocol_version != PROTOCOL_VERSION {
        return Err("unsupported response protocol".into());
    }
    if response.engine != request.expected_engine {
        return Err("engine identity mismatch".into());
    }
    if response.case_id != request.case.id {
        return Err("response case ID mismatch".into());
    }
    if !matches!(response.status, Status::Ok) {
        return Err(format!(
            "engine did not execute case: {:?}: {}",
            response.status,
            response.message.as_deref().unwrap_or("no reason supplied")
        ));
    }
    if response.steps.len() != request.case.steps.len() {
        return Err("response step count mismatch".into());
    }
    for (index, step) in response.steps.iter().enumerate() {
        if step.step_index != index {
            return Err("response step indices must be contiguous and zero based".into());
        }
    }
    if coverage.iter().any(|c| !response.coverage.contains(c)) {
        return Err("response is missing required coverage".into());
    }
    Ok(())
}

#[derive(Debug)]
pub enum RunError {
    Invalid(String),
    Engine {
        case_id: String,
        side: &'static str,
        message: String,
    },
    Divergence(Divergence),
}
impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RunError {}

pub fn compare(
    corpus: &Corpus,
    oracle: &mut impl Runner,
    candidate: &mut impl Runner,
) -> Result<Report, RunError> {
    corpus.validate().map_err(RunError::Invalid)?;
    let mut count = 0;
    for case in &corpus.cases {
        let mut observations = Vec::new();
        for (side, identity, runner) in [
            ("oracle", &corpus.oracle, oracle as &mut dyn Runner),
            ("candidate", &corpus.candidate, candidate as &mut dyn Runner),
        ] {
            let request = Request {
                protocol_version: PROTOCOL_VERSION,
                expected_engine: identity.clone(),
                case: case.clone(),
            };
            let response = runner
                .run(&request)
                .and_then(|r| {
                    validate_response(&request, &r, &corpus.evidence.coverage)?;
                    Ok(r)
                })
                .map_err(|message| RunError::Engine {
                    case_id: case.id.clone(),
                    side,
                    message,
                })?;
            observations.push(response);
        }
        for (index, (a, b)) in observations[0]
            .steps
            .iter()
            .zip(&observations[1].steps)
            .enumerate()
        {
            let a = serde_json::to_value(a).unwrap();
            let b = serde_json::to_value(b).unwrap();
            if let Some((path, oracle, candidate)) = first_difference(&a, &b, "") {
                return Err(RunError::Divergence(Divergence {
                    case_id: case.id.clone(),
                    step_index: index,
                    path,
                    oracle,
                    candidate,
                }));
            }
            count += 1;
        }
    }
    Ok(Report {
        cases: corpus.cases.len(),
        steps: count,
        evidence_level: corpus.evidence.level.clone(),
        coverage: corpus.evidence.coverage.clone(),
    })
}
fn first_difference(a: &Value, b: &Value, path: &str) -> Option<(String, Value, Value)> {
    if a == b {
        return None;
    }
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
            for key in keys {
                let p = format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"));
                match (a.get(key), b.get(key)) {
                    (Some(a), Some(b)) => {
                        if let Some(d) = first_difference(a, b, &p) {
                            return Some(d);
                        }
                    }
                    (a, b) => {
                        return Some((
                            p,
                            a.cloned().unwrap_or(Value::Null),
                            b.cloned().unwrap_or(Value::Null),
                        ))
                    }
                }
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                if let Some(d) = first_difference(a, b, &format!("{path}/{i}")) {
                    return Some(d);
                }
            }
        }
        _ => {}
    }
    Some((path.into(), a.clone(), b.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn identity(kind: &str) -> EngineIdentity {
        EngineIdentity {
            kind: kind.into(),
            binary_sha256: "a".repeat(64),
            configuration_sha256: "b".repeat(64),
        }
    }
    fn corpus() -> Corpus {
        Corpus {
            schema_version: 1,
            oracle: identity("original_dll"),
            candidate: identity("rust"),
            evidence: Evidence {
                level: "harness_test_only".into(),
                references: vec!["unit test".into()],
                coverage: vec!["test".into()],
            },
            cases: vec![Case {
                id: "test".into(),
                initial_state: json!({}),
                steps: vec![json!({"op":"test"}), json!({"op":"test"})],
            }],
        }
    }
    struct Fake {
        mismatch: Option<&'static str>,
    }
    impl Runner for Fake {
        fn run(&mut self, r: &Request) -> Result<Response, String> {
            let mut response = Response {
                protocol_version: 1,
                status: Status::Ok,
                engine: r.expected_engine.clone(),
                case_id: r.case.id.clone(),
                steps: (0..r.case.steps.len())
                    .map(|step_index| StepObservation {
                        step_index,
                        snapshot: json!({"turn":step_index,"nested":{"value":7}}),
                        rng_trace: vec![RngEvent {
                            stream: "test".into(),
                            ordinal: step_index as u64,
                            operation: "get".into(),
                            arguments: json!({"bound":10}),
                            result: json!(7),
                        }],
                    })
                    .collect(),
                coverage: vec!["test".into()],
                message: None,
            };
            match self.mismatch {
                Some("state") => response.steps[1].snapshot["nested"]["value"] = json!(8),
                Some("rng") => response.steps[0].rng_trace[0].result = json!(8),
                Some("identity") => response.engine.binary_sha256 = "c".repeat(64),
                Some("unsupported") => response.status = Status::Unsupported,
                Some("missing") => {
                    response.steps.pop();
                }
                Some("order") => response.steps[0].step_index = 1,
                Some("coverage") => response.coverage.clear(),
                Some("unavailable") => return Err("unavailable".into()),
                _ => {}
            }
            Ok(response)
        }
    }
    #[test]
    fn harness_matching_transport_is_not_game_parity() {
        let r = compare(
            &corpus(),
            &mut Fake { mismatch: None },
            &mut Fake { mismatch: None },
        )
        .unwrap();
        assert_eq!(r.steps, 2);
        assert_eq!(r.evidence_level, "harness_test_only");
    }
    #[test]
    fn first_state_divergence_has_path_and_step() {
        let e = compare(
            &corpus(),
            &mut Fake { mismatch: None },
            &mut Fake {
                mismatch: Some("state"),
            },
        )
        .unwrap_err();
        match e {
            RunError::Divergence(d) => {
                assert_eq!(d.step_index, 1);
                assert_eq!(d.path, "/snapshot/nested/value");
                assert_eq!(d.oracle, json!(7));
                assert_eq!(d.candidate, json!(8));
            }
            _ => panic!("wrong error"),
        }
    }
    #[test]
    fn rng_draw_order_and_output_are_compared() {
        let e = compare(
            &corpus(),
            &mut Fake { mismatch: None },
            &mut Fake {
                mismatch: Some("rng"),
            },
        )
        .unwrap_err();
        assert!(
            matches!(e,RunError::Divergence(Divergence {step_index:0,path,..}) if path == "/rng_trace/0/result")
        );
    }
    #[test]
    fn fail_closed_on_bad_or_missing_evidence() {
        for fault in [
            "identity",
            "unsupported",
            "missing",
            "order",
            "coverage",
            "unavailable",
        ] {
            assert!(
                matches!(
                    compare(
                        &corpus(),
                        &mut Fake {
                            mismatch: Some(fault)
                        },
                        &mut Fake { mismatch: None }
                    ),
                    Err(RunError::Engine { side: "oracle", .. })
                ),
                "{fault}"
            );
        }
        let mut c = corpus();
        c.oracle.kind = "rust".into();
        assert!(matches!(
            compare(
                &c,
                &mut Fake { mismatch: None },
                &mut Fake { mismatch: None }
            ),
            Err(RunError::Invalid(_))
        ));
        c = corpus();
        c.evidence.references.clear();
        assert!(c.validate().is_err());
    }
    #[test]
    fn missing_process_is_unavailable() {
        let mut p = ProcessRunner {
            program: "/definitely/missing/c6-oracle".into(),
            args: vec![],
            timeout: Duration::from_secs(1),
            output_limit: 1024,
        };
        assert!(p
            .run(&Request {
                protocol_version: 1,
                expected_engine: identity("original_dll"),
                case: corpus().cases.remove(0)
            })
            .unwrap_err()
            .contains("unavailable"));
    }
    #[cfg(unix)]
    #[test]
    fn process_deadline_and_output_limit_and_invalid_json() {
        for (script, expected, timeout, limit) in [
            ("sleep 2", "deadline", 20, 1024),
            ("printf invalid", "invalid oracle response", 1000, 1024),
            ("printf 123456789", "output limit", 1000, 4),
        ] {
            let mut p = ProcessRunner {
                program: "/bin/sh".into(),
                args: vec!["-c".into(), script.into()],
                timeout: Duration::from_millis(timeout),
                output_limit: limit,
            };
            // Shell may close stdin before writer runs; keep it consumed for determinism.
            p.args[1] = format!("cat >/dev/null; {script}");
            assert!(p
                .run(&Request {
                    protocol_version: 1,
                    expected_engine: identity("original_dll"),
                    case: corpus().cases.remove(0)
                })
                .unwrap_err()
                .contains(expected));
        }
    }
}
