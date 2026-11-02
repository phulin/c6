use c6_differential::{compare, Corpus, ProcessRunner, RecordingRunner};
use std::{env, fs, process::ExitCode, time::Duration};
fn main() -> ExitCode {
    let args: Vec<_> = env::args().collect();
    if args.len() != 4 && args.len() != 5 {
        eprintln!("usage: c6-differential CORPUS.json ORACLE_WRAPPER CANDIDATE_WRAPPER [TRACE_DIR]\nWrappers accept one protocol-v1 JSON request on stdin and emit one JSON response.");
        return ExitCode::from(2);
    }
    let run = || -> Result<(), String> {
        let corpus: Corpus =
            serde_json::from_slice(&fs::read(&args[1]).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let make = |program: String| {
            RecordingRunner::new(ProcessRunner {
                program,
                args: vec![],
                timeout: Duration::from_secs(30),
                output_limit: 16 * 1024 * 1024,
            })
        };
        let mut oracle = make(args[2].clone());
        let mut candidate = make(args[3].clone());
        let result = compare(&corpus, &mut oracle, &mut candidate);
        if let Some(directory) = args.get(4) {
            fs::create_dir_all(directory).map_err(|e| e.to_string())?;
            for (side, records) in [("oracle", oracle.records), ("candidate", candidate.records)] {
                for (index, record) in records.iter().enumerate() {
                    let path =
                        std::path::Path::new(directory).join(format!("{side}-{index:06}.json"));
                    fs::write(
                        path,
                        serde_json::to_vec_pretty(record).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        let report = result.map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
        Ok(())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
