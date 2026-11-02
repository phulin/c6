#!/usr/bin/env python3
"""Check Rust Formation wire codec against exact native serializer-owned slices.

Read-only original Save/trace reuse. No engine execution or boundary inference
from byte patterns. Private payloads are kept in temporary files.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
MODULE = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
SAVES = [
    'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884',
    '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8',
]
TRACES = [
    ('trace-benchmark-1.jsonl.gz', '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
    ('trace-captured-1.jsonl.gz', '027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369'),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--private-root', type=Path, default=ROOT.parent / 'c6-ghidra')
    parser.add_argument('--iterations', type=int, default=1000)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.iterations <= 0:
        parser.error('--iterations must be positive')
    manifest = json.loads((args.private_root / 'evidence/linux-game-load/canonical/manifest.json').read_text())
    if manifest['module_sha256'] != MODULE:
        raise ValueError('unsupported original module identity')
    subprocess.run(['cargo', 'build', '--release', '-p', 'c6-core', '--example', 'unit_codec'], cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1', '--no-deps'], cwd=ROOT))
    binary = Path(metadata['target_directory']) / 'release/examples/unit_codec'
    results = []
    for identity, (trace_name, trace_sha) in zip(SAVES, TRACES):
        with gzip.open(args.private_root / 'evidence/linux-game-load/canonical' / f'{identity}.bin.gz', 'rb') as source:
            data = source.read(80657463)
        if len(data) != 80657462 or sha(data) != identity:
            raise ValueError('unsupported original stream identity')
        with gzip.open(args.private_root / 'evidence/linux-save-normalization/checkpoint-203-owners' / trace_name, 'rb') as source:
            trace = source.read()
        if sha(trace) != trace_sha:
            raise ValueError('unsupported original trace identity')
        rows = [json.loads(line) for line in trace.splitlines() if b'"owner":"GameCore::Formation::Instance"' in line]
        rows = sorted((r for r in rows if r['save'] == 1), key=lambda r: r['start'])
        if len(rows) < 2:
            raise ValueError('insufficient attributed real formations')
        with tempfile.TemporaryDirectory(prefix='c6-formation-codec-') as temp:
            formation_dir, queue_dir = Path(temp) / 'formations', Path(temp) / 'queues'
            formation_dir.mkdir()
            queue_dir.mkdir()
            queue_rows = [json.loads(line) for line in trace.splitlines() if b'\"owner\":\"GameCore::Unit::Operations::OperationQueue\"' in line]
            queue_rows = [row for row in queue_rows if row['save'] == 1]
            if len(queue_rows) != 1262:
                raise ValueError('unexpected native queue record count')
            for i, row in enumerate(queue_rows):
                start, end = row['start'], row['end']
                if not 0 <= start < end <= len(data):
                    raise ValueError('invalid native queue bounds')
                (queue_dir / f'{i:06}.bin').write_bytes(data[start:end])
            ranges = []
            for i, row in enumerate(rows):
                start, end = row['start'], row['end']
                if not 0 <= start < end <= len(data) or (ranges and start < ranges[-1][1]):
                    raise ValueError('invalid/overlapping native owner range')
                ranges.append([start, end])
                (formation_dir / f'{i:06}.bin').write_bytes(data[start:end])
            result = json.loads(subprocess.check_output([str(binary), str(formation_dir), str(args.iterations), str(queue_dir)], cwd=ROOT))
        if result.pop('record_lengths') != [end-start for start, end in ranges]:
            raise ValueError('Rust typed record extents differ from original boundaries')
        if (result['native_queue_roundtrips'], result['native_queue_unsupported_handlers']) != (1262, 0):
            raise ValueError('unexpected native queue acceptance/rejection profile')
        if (result['records'], result['bytes'], result['path_nodes'], result['unit_ids'], result['nonempty_paths']) != (1250, 71168, 56, 1262, 8):
            raise ValueError('unexpected pinned native Formation fixture dimensions')
        result.update(save_sha256=identity, native_trace_sha256=trace_sha,
                      all_native_boundaries_matched=len(rows),
                      attributed_ranges_sha256=sha(json.dumps(ranges, separators=(',', ':')).encode()),
                      records_sha256=sha(b''.join(data[start:end] for start, end in ranges)))
        results.append(result)
    sources = ['crates/c6-core/src/unit.rs', 'crates/c6-core/examples/unit_codec.rs', 'tools/linux/unit_codec_check.py']
    report = dict(schema='c6-linux-formation-rust-codec-v1', module_sha256=MODULE,
                  fixtures=results, adapter_sha256=sha(binary.read_bytes()),
                  source_sha256={name: sha((ROOT / name).read_bytes()) for name in sources},
                  rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                  timing_scope='Release mean per Formation record, decode/encode including allocation/drop. No game, rules, action, turn, RNG or original-code performance measurement.',
                  limits='Current Formation v1 serialized state only. Live evaluating-operation, definition remapping, tracked archive dirtiness, complete Unit state and actions unsupported. Queue codec supports nine statically verified handlers and roundtrips all native fixtures across eight observed hashes, including typed MoveTo/trade-route parameters; no attributed nonempty SkipTurn fixture. Other handlers remain unsupported. Raw Save/reload/Save failure remains 89198 unmasked bytes.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
