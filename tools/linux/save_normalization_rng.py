#!/usr/bin/env python3
"""Compare the complete observed original Game synchronous RNG serialized record."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from inspect_elf import sections
from save_normalization_trade import INITIAL, RELOADED, INITIAL_TRACE, RELOADED_TRACE, SIZE

ROOT = Path(__file__).resolve().parents[2]
XP2 = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
DWARF = 'b12b87198e53aed3a44191deaa6a1c93a80efc2c1d81997fd90d6b6455da15d4'
START, END = 71027778, 71027796
METHODS = [('random_write', 0x2d48a20, 0x69), ('random_read', 0x2d48ab0, 0xa4),
           ('game_const_get_sync_random', 0x2d7b4d0, 8), ('game_write_rng_call_site', 0x2d7a89e, 0x13)]


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def observe(path, expected, trace, trace_expected):
    if digest(path) != expected or path.stat().st_size != SIZE or digest(trace) != trace_expected:
        raise ValueError('unsupported canonical native output or exact owner trace')
    occurrences = {1: [], 2: []}
    with trace.open() as source:
        for line in source:
            if '"owner":"GameCore::Random"' in line:
                row = json.loads(line)
                occurrences[row['save']].append(row)
    if any(len(rows) != 1 or (rows[0]['start'], rows[0]['end'], rows[0]['depth'], rows[0]['parent']) !=
           (START, END, 1, 1) for rows in occurrences.values()):
        raise ValueError('native Game synchronous Random serialization occurrence differs')
    with path.open('rb') as source:
        source.seek(START)
        data = source.read(END-START)
    version, seed, calls, resets, synchronous, legacy = struct.unpack('<IIIIBB', data)
    if version != 1 or synchronous not in (0, 1) or legacy != 0:
        raise ValueError('unsupported native Random serialization shape')
    return dict(owner='GameCore::Game::Instance::m_kSyncRandom',
                serialized_instances_observed=1, serialization_occurrences_per_save=1,
                stream_start=START, stream_end=END, complete_record_bytes=len(data),
                complete_record_sha256=hashlib.sha256(data).hexdigest(),
                values=dict(version=version, random_seed=seed, call_count=calls,
                            reset_count=resets, synchronous=bool(synchronous), legacy_history_present=False))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--initial', type=Path, required=True)
    parser.add_argument('--reloaded', type=Path, required=True)
    parser.add_argument('--initial-trace', type=Path, required=True)
    parser.add_argument('--reloaded-trace', type=Path, required=True)
    parser.add_argument('--module', type=Path, default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--dwarf', type=Path, default=ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/random-types.dwarf.txt')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if digest(args.module) != XP2 or digest(args.dwarf) != DWARF:
        raise ValueError('unsupported native module or matching Random ownership/layout DWARF')
    native_methods = []
    text = sections(args.module)['.text']
    with args.module.open('rb') as source:
        for name, address, size in METHODS:
            source.seek(address-int(text['address'], 16)+text['offset'])
            native_methods.append(dict(name=name, address=hex(address), bytes=size,
                                       sha256=hashlib.sha256(source.read(size)).hexdigest()))
    a = observe(args.initial, INITIAL, args.initial_trace, INITIAL_TRACE)
    b = observe(args.reloaded, RELOADED, args.reloaded_trace, RELOADED_TRACE)
    result = dict(schema='c6-linux-save-normalization-serialized-sync-rng-v1', module_sha256=XP2,
                  matching_dwarf_sha256=DWARF, native_methods=native_methods,
                  wire_schema=['u32 version=1', 'u32 m_ulRandomSeed', 'u32 m_ulCallCount', 'u32 m_ulResetCount',
                               'u8 m_bSynchronous', 'u8 legacy_history_present=false'],
                  initial=a, reloaded=b, complete_observed_serialized_rng_equal=a == b,
                  limits='Only the single observed Game-owned synchronous RNG serialized record is compared. Native Write omits live m_uiLockCount and always writes false for legacy history. This does not establish every live RNG object, lock state, asynchronous stream, full game state or future action/turn parity.')
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(output=str(args.output), record=a, serialized_equal=a == b), indent=2))


if __name__ == '__main__':
    main()
