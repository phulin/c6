#!/usr/bin/env python3
"""Fingerprint Linux relation Read/query code; inspect an attributed saved owner.

No original execution. Owner trace offsets are independent component evidence,
not a general Core index or native live-state import.
"""
import argparse
import collections
import gzip
import json
from pathlib import Path
import struct
from definition_ids_audit import ROOT, TARGET, HOST, sha, elf
from load_archive_postlude_audit import methods

CORE = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE = '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
METHODS = dict(read=0x29808e0, is_allied=0x2983170,
    alliance_type=0x2983b80, is_at_war=0x2983ba0, war_type=0x2980ea0,
    symmetric_alliance=0x2988430, symmetric_war=0x29885d0,
    alliance_record_read=0x29905e0, bool_record_read=0x2988c70,
    alliance_sequence_read=0x2990500, alliance_enum_read=0x2bf6c00,
    alliance_hash_to_index=0x2bf6fc0, war_enum_read=0x2d503a0,
    war_validate_hash=0x2d514d0)


def audit(assets, private, core_path, trace_path):
    elf(assets/'Civ6', HOST)
    code, assembly, _ = methods(assets/'libGameCore_XP2.so', TARGET, METHODS, private)
    core = gzip.decompress(core_path.read_bytes())
    trace = gzip.decompress(trace_path.read_bytes())
    if sha(core) != CORE or sha(trace) != TRACE:
        raise ValueError('original Core/owner attribution identity differs')
    frames = [json.loads(line) for line in trace.splitlines()]
    frames = [f for f in frames if f.get('event') == 'owner' and f.get('save') == 1
              and f.get('owner') == 'GameCore::Game::DiplomaticRelations']
    if len(frames) != 1 or frames[0].get('depth') != 1 or frames[0].get('parent') != 1:
        raise ValueError('ambiguous original serializer owner')
    frame = frames[0]
    owner = core[frame['start']:frame['end']]
    pos = 0
    def integer():
        nonlocal pos
        value = struct.unpack_from('<i', owner, pos)[0]
        pos += 4
        return value
    version, player_count = integer(), integer()
    if version != 16 or player_count != 64:
        raise ValueError('unsupported current fixture')
    rows = {}
    for name, width in [('at_c8',5), ('at_f0',5), ('at_118',5), ('at_140',5),
                        ('at_168',8), ('saved_war',8), ('at_1b8',8), ('at_1e0',8)]:
        count = integer()
        start = pos
        if count < 0 or count > 100000:
            raise ValueError('invalid count')
        pos += count * width
        rows[name] = dict(count=count, data=owner[start:pos], width=width)
    at_320 = owner[pos]
    pos += 1
    count = integer()
    rows['saved_alliances'] = dict(count=count, data=owner[pos:pos+count*8], width=8)
    pos += count*8
    for row in rows.values():
        if len(row['data']) != row['count']*row['width']:
            raise ValueError('truncated relationship vector')
    if at_320 not in (0,1):
        raise ValueError('noncanonical bool')
    def summary(row):
        values = [struct.unpack_from('<ii', row['data'], i*8) for i in range(row['count'])]
        return dict(count=row['count'], wire_sha256=sha(row['data']),
            saved_first_words=dict(sorted(collections.Counter(str(x[0]) for x in values).items())),
            values_sha256=sha(b''.join(struct.pack('<i',x[0]) for x in values)),
            turns_sha256=sha(b''.join(struct.pack('<i',x[1]) for x in values)))
    checks = dict(alliance_enum_uses_definitions='GetAllianceDefinitions' in assembly['alliance_enum_read'],
        alliance_enum_maps_hash_to_index='HashToIndex' in assembly['alliance_enum_read'],
        alliance_missing_definition_returns_minus_one='mov    $0xffffffff,%eax' in assembly['alliance_hash_to_index'],
        war_enum_validates_hash='ValidateHash' in assembly['war_enum_read'],
        same_player_allied_false='xor    %eax,%eax' in assembly['is_allied'],
        symmetric_signed_wrapping_index='sar    $1,%esi' in assembly['symmetric_alliance'])
    if not all(checks.values()):
        raise ValueError(checks)
    private.mkdir(parents=True,exist_ok=True)
    (private/'owner.bin').write_bytes(owner)
    (private/'expected-records.bin').write_bytes(b''.join(row['data'] for row in rows.values()))
    return dict(schema='c6-linux-game-diplomatic-relations-source-v1', gamecore_sha256=TARGET,
        host_sha256=HOST, methods=code, checks=checks, original_execution=False,
        core_sha256=CORE, core_bytes=len(core), owner_trace_decompressed_sha256=TRACE,
        owner_frame=frame, owner_sha256=sha(owner), version=version, player_count=player_count,
        prefix_bytes=pos, prefix_sha256=sha(owner[:pos]), unclassified_tail_bytes=len(owner)-pos,
        unclassified_tail_sha256=sha(owner[pos:]), saved_war=summary(rows['saved_war']),
        saved_alliances=summary(rows['saved_alliances']),
        alliance_runtime_values_compared=False, live_action_parity=False,
        complete_state_action_turn_parity=False)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--core',type=Path,default=ROOT.parent/f'c6-ghidra/evidence/linux-game-load/canonical/{CORE}.bin.gz')
    p.add_argument('--trace',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz')
    p.add_argument('--private-output',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    result=audit(a.assets,a.private_output,a.core,a.trace)
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(result,sort_keys=True,indent=2)+'\n')


if __name__ == '__main__':
    main()
