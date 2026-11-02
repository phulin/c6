#!/usr/bin/env python3
"""Read-only Rust Trade codec fixture comparison; never executes GameCore.

Requires private original streams and serializer traces. Publishes identities,
counts and timing only; native graph values stay in temporary private files.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

MODULE = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
SAVES = [
    'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884',
    '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8',
]
TRACES = [
    ('trace-benchmark-1.jsonl.gz', '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
    ('trace-captured-1.jsonl.gz', '027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369'),
]
START, END, SIZE = 71027800, 71497438, 80657462
ROOT = Path(__file__).resolve().parents[2]


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
    subprocess.run(['cargo', 'build', '--release', '-p', 'c6-core', '--example', 'trade_codec'], cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1', '--no-deps'], cwd=ROOT))
    binary = Path(metadata['target_directory']) / 'release/examples/trade_codec'
    graph_hashes, native_ranges = [], []
    with tempfile.TemporaryDirectory(prefix='c6-trade-codec-') as temporary:
        temp = Path(temporary)
        for index, identity in enumerate(SAVES):
            path = args.private_root / 'evidence/linux-game-load/canonical' / f'{identity}.bin.gz'
            with gzip.open(path, 'rb') as source:
                data = source.read(SIZE + 1)
            if len(data) != SIZE or sha(data) != identity:
                raise ValueError('unsupported exact original stream identity')
            graph = data[START:END]
            graph_hashes.append(sha(graph))
            (temp / f'{index}.bin').write_bytes(graph)
            name, expected = TRACES[index]
            trace_path = args.private_root / 'evidence/linux-save-normalization/checkpoint-203-owners' / name
            with gzip.open(trace_path, 'rb') as source:
                trace = source.read()
            if sha(trace) != expected:
                raise ValueError('unsupported exact original serializer trace identity')
            ranges = []
            for line in trace.splitlines():
                if b'"owner":"GameCore::Trade::Route"' not in line:
                    continue
                row = json.loads(line)
                if row['save'] == 1 and START <= row['start'] and row['end'] <= END:
                    ranges.append([row['start'] - START, row['end'] - START])
            native_ranges.append(sorted(ranges))
        result = json.loads(subprocess.check_output([str(binary), str(temp / '0.bin'), str(temp / '1.bin'),
                                                    str(args.iterations), str(temp / 'ranges')], cwd=ROOT))
        for index in range(2):
            rust_ranges = json.loads((temp / f'ranges-{index}.json').read_text())
            if len(rust_ranges) != 1571 or sorted(rust_ranges) != native_ranges[index]:
                raise ValueError('Rust typed Route boundaries differ from pinned original serializer trace')
    if (result['origins'], result['edges'], result['total_edges_created'], result['graph_bytes']) != (179, 1571, 1571, END-START):
        raise ValueError('unexpected exact original graph dimensions')
    if result['iteration_order_equal'] or result['wire_state_equal']:
        raise ValueError('original pair must retain its distinct serialization order')
    result.update(schema='c6-linux-trade-rust-codec-v1', module_sha256=MODULE,
                  save_sha256=SAVES, graph_sha256=graph_hashes,
                  native_trace_sha256=[expected for _, expected in TRACES],
                  all_native_route_boundaries_matched=[1571,1571], graph_start=START, graph_end=END,
                  adapter_sha256=sha(binary.read_bytes()),
                  source_sha256={name: sha((ROOT / name).read_bytes()) for name in [
                      'crates/c6-core/src/trade.rs', 'crates/c6-core/examples/trade_codec.rs',
                      'tools/linux/trade_codec_check.py']},
                  rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                  timing_scope='Release wall-clock mean over actual first graph: decode includes allocations/validation/indexes/drop; encode includes allocation/drop; mixed successful endpoint and ID lookups. No original-code execution, game initialization, rules, actions, turns or RNG benchmark.',
                  limits='Extracted current-version Graph wire codec only; exact bytes retained, keyed payload comparison excludes graph iteration order. Native ID-map callback order and live Route finalizing are not serialized. Not full trade gameplay/live-state or whole-game equivalence. Raw Save/reload/Save still differs by 89198 bytes.')
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
