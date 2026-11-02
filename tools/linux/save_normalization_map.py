#!/usr/bin/env python3
"""Map exact captured-roundtrip differences to observed original serialization owners."""
import argparse
from collections import defaultdict
import hashlib
import heapq
import json
from pathlib import Path

INITIAL = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
RELOADED = '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8'
SIZE = 80657462


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def trace_owners(path):
    events = [json.loads(line) for line in path.read_text().splitlines()]
    owners = []
    for save in (1, 2):
        selected = [x for x in events if x['save'] == save]
        if selected[0] != dict(event='save_begin', save=save, offset=0):
            raise ValueError('native trace has no matching initial offset')
        if selected[-1] != dict(event='save_end', save=save, offset=SIZE, depth=0):
            raise ValueError('native trace did not complete original Save')
        rows = [x for x in selected if x['event'] == 'owner']
        if not rows:
            raise ValueError('no original serialization owner boundaries observed')
        ids = {x['call'] for x in rows}
        if len(ids) != len(rows) or any(x['end'] < x['start'] or x['end'] > SIZE or
                                      (x['parent'] and x['parent'] not in ids) for x in rows):
            raise ValueError('invalid native owner boundaries')
        owners.append(sorted([{k: v for k, v in x.items() if k not in ('event', 'save')} for x in rows], key=lambda x: x['call']))
    if owners[0] != owners[1]:
        raise ValueError('consecutive native Save owner boundaries differ')
    return owners[0]


def map_differences(owners, positions):
    starts = sorted(enumerate(owners), key=lambda pair: pair[1]['start'])
    active, next_start, grouped = [], 0, defaultdict(list)
    for position in positions:
        while next_start < len(starts) and starts[next_start][1]['start'] <= position:
            index, row = starts[next_start]
            heapq.heappush(active, (-row['depth'], row['end']-row['start'], index, row['end']))
            next_start += 1
        while active and active[0][3] <= position:
            heapq.heappop(active)
        grouped[active[0][2] if active else None].append(position)
    by_id = {x['call']: x for x in owners}
    results = []
    for index, offsets in grouped.items():
        if index is None:
            results.append(dict(owner=None, different_bytes=len(offsets), first=offsets[0], last_exclusive=offsets[-1]+1))
            continue
        row = owners[index]
        path, cursor = [], row
        while cursor:
            path.append(dict(call=cursor['call'], owner=cursor['owner']))
            cursor = by_id.get(cursor['parent'])
        results.append(dict(owner=row['owner'], call=row['call'], start=row['start'], end=row['end'],
                            different_bytes=len(offsets), first=offsets[0], last_exclusive=offsets[-1]+1,
                            owner_path=list(reversed(path))))
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--initial', type=Path, required=True)
    parser.add_argument('--reloaded', type=Path, required=True)
    parser.add_argument('--trace', type=Path, required=True)
    parser.add_argument('--native-report', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--private-details', type=Path, required=True)
    args = parser.parse_args()
    native = json.loads(args.native_report.read_text())
    configuration = hashlib.sha256(json.dumps(native['configuration'], sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    if native['configuration_sha256'] != configuration or native['returncode'] != 0 or not native['framework_cleanup_proven']:
        raise ValueError('native trace fixture configuration/return/cleanup did not pass')
    metadata = native['save_normalization_trace']
    if Path(native['execution_paths']['save_normalization_trace']).resolve() != args.trace.resolve() or metadata['sha256'] != digest(args.trace) or metadata['bytes'] != args.trace.stat().st_size:
        raise ValueError('private trace does not match exact native report artifact')
    saves = native['game_saves']
    expected = saves['save']['sha256']
    if expected not in (INITIAL, RELOADED) or saves['repeat']['sha256'] != expected:
        raise ValueError('instrumented Save outputs differ from known native baseline')
    for label in ('save', 'repeat'):
        artifact = Path(native['execution_paths']['game_outputs'][label])
        if saves[label]['bytes'] != SIZE or artifact.stat().st_size != SIZE or digest(artifact) != expected:
            raise ValueError('private native Save output identity differs')
    if digest(args.initial) != INITIAL or digest(args.reloaded) != RELOADED:
        raise ValueError('unsupported canonical original stream identities')
    a, b = args.initial.read_bytes(), args.reloaded.read_bytes()
    if len(a) != SIZE or len(b) != SIZE:
        raise ValueError('canonical native stream lengths differ')
    positions = [i for i, (x, y) in enumerate(zip(a, b)) if x != y]
    owners = trace_owners(args.trace)
    mapped = map_differences(owners, positions)
    result = dict(schema='c6-linux-save-normalization-owner-map-v1', initial_sha256=INITIAL,
                  reloaded_sha256=RELOADED, bytes=SIZE, different_bytes=len(positions),
                  trace_sha256=digest(args.trace), native_report_sha256=digest(args.native_report),
                  configuration_sha256=configuration, traced_output_sha256=expected, observed_owner_calls=len(owners), mapped_differences=mapped,
                  limits='Owner ranges alone do not prove field semantics, native Load normalization or scope exclusion. AI namespaces are not blanket exclusions; gameplay state and RNG remain required.')
    with args.private_details.open('x') as details:
        details.write(json.dumps(result, indent=2)+'\n')
    aggregate = {}
    for row in mapped:
        owner = row['owner']
        summary = aggregate.setdefault(owner, dict(owner=owner, different_bytes=0, calls=0, first=row['first'], last_exclusive=row['last_exclusive']))
        summary['different_bytes'] += row['different_bytes']
        summary['calls'] += 1
        summary['first'] = min(summary['first'], row['first'])
        summary['last_exclusive'] = max(summary['last_exclusive'], row['last_exclusive'])
    result['mapped_differences'] = list(aggregate.values())
    result['private_details'] = dict(filename=args.private_details.name, sha256=digest(args.private_details), bytes=args.private_details.stat().st_size)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(output=str(args.output), different_bytes=len(positions), mapped_owner_calls=len(mapped),
                          deepest_owners=sorted({x['owner'] for x in mapped if x['owner']}))))


if __name__ == '__main__':
    main()
