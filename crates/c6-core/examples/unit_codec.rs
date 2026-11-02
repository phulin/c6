//! Read-only current Formation record roundtrips; never executes original code.
use c6_core::unit::{Error, Formation, Limits, OperationQueue};
use std::{hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: unit_codec FORMATION_DIRECTORY ITERATIONS QUEUE_DIRECTORY".into());
    }
    let iterations: usize = args[2].parse()?;
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    let mut paths: Vec<_> = std::fs::read_dir(&args[1])?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    paths.sort();
    if paths.is_empty() {
        return Err("no attributed records".into());
    }
    let mut records = Vec::new();
    let mut total_bytes = 0;
    let mut nodes = 0;
    let mut unit_ids = 0;
    let mut nonempty_paths = 0;
    let mut all_lengths = Vec::new();
    for path in paths {
        let bytes = std::fs::read(path)?;
        let formation = Formation::decode(&bytes, Limits::default())?;
        assert_eq!(formation.encode(Limits::default())?, bytes);
        // Independently predict record extent from typed fields. Native exact
        // owner boundaries supply the input; the decoder must consume it all.
        let expected = 52 + 20 * formation.last_path.len() + 4 * formation.unit_ids.len();
        assert_eq!(expected, bytes.len());
        all_lengths.push(expected);
        total_bytes += bytes.len();
        nodes += formation.last_path.len();
        unit_ids += formation.unit_ids.len();
        nonempty_paths += usize::from(!formation.last_path.is_empty());
        records.push((bytes, formation));
    }
    let mut queue_roundtrips = 0;
    let mut queue_rejections = 0;
    for entry in std::fs::read_dir(&args[3])? {
        let bytes = std::fs::read(entry?.path())?;
        match OperationQueue::decode(&bytes, Limits::default()) {
            Ok(queue) => {
                assert_eq!(queue.encode(Limits::default())?, bytes);
                queue_roundtrips += 1;
            }
            Err(Error::UnsupportedHandler(_)) => queue_rejections += 1,
            Err(error) => return Err(error.into()),
        }
    }
    let start = Instant::now();
    for _ in 0..iterations {
        for (bytes, _) in &records {
            black_box(Formation::decode(black_box(bytes), Limits::default())?);
        }
    }
    let decode_ns = start.elapsed().as_nanos() / (iterations * records.len()) as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        for (_, state) in &records {
            black_box(state.encode(Limits::default())?);
        }
    }
    let encode_ns = start.elapsed().as_nanos() / (iterations * records.len()) as u128;
    println!("{{\"native_queue_roundtrips\":{},\"native_queue_unsupported_handlers\":{},\"records\":{},\"bytes\":{},\"path_nodes\":{},\"unit_ids\":{},\"nonempty_paths\":{},\"record_lengths\":{:?},\"iterations\":{},\"decode_ns_per_record\":{},\"encode_ns_per_record\":{}}}", queue_roundtrips, queue_rejections, records.len(), total_bytes, nodes, unit_ids, nonempty_paths, all_lengths, iterations, decode_ns, encode_ns);
    Ok(())
}
