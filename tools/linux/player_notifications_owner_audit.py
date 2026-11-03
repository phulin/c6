#!/usr/bin/env python3
"""Pin notification owner/index contracts and existing ff31 source framing.

No native execution, general Core indexing, new codec, save writer or live
notification defaults. Proprietary method bytes remain private.
"""
import argparse
import gzip
import json
from pathlib import Path
import struct
from definition_ids_audit import ROOT,TARGET,HOST,elf,sha
from load_archive_postlude_audit import methods

CORE='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE='528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
METHODS=dict(manager_read=0x2d2f820,manager_initialize=0x2d2ee00,
    manager_find=0x2d2f3d0,manager_get_slot=0x2d2f350,manager_edit_slot=0x2d2fa90,
    player_initialize=0x2d30840,player_find_id=0x2d30940)


def audit(assets,private):
    elf(assets/'Civ6',HOST)
    code,assembly,_=methods(assets/'libGameCore_XP2.so',TARGET,METHODS,private)
    core=gzip.decompress((ROOT.parent/f'c6-ghidra/evidence/linux-game-load/canonical/{CORE}.bin.gz').read_bytes())
    trace=gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz').read_bytes())
    if sha(core)!=CORE or sha(trace)!=TRACE:raise ValueError('attributed source identity differs')
    rows=[json.loads(line) for line in trace.splitlines()]
    frames=[r for r in rows if r.get('event')=='owner' and r.get('save')==1 and
        r.get('owner')=='GameCore::Reporting::Notification::PlayerNotifications']
    if len(frames)!=64:raise ValueError('incomplete actual slot inventory')
    previous=frames[0]['start'];headers=[]
    for slot,frame in enumerate(frames):
        if frame['depth']!=1 or frame['parent']!=1 or frame['call']!=201876+slot or frame['start']!=previous or frame['end']<=previous:
            raise ValueError('actual ordered owner framing differs')
        version,owner,next_id,count=struct.unpack_from('<iiII',core,frame['start'])
        if version!=1:raise ValueError('unsupported actual owner version')
        headers.append(dict(slot=slot,stored_owner=owner,next_id=next_id,records=count,
            bytes=frame['end']-frame['start']))
        previous=frame['end']
    start=frames[0]['start']-8;end=frames[-1]['end']
    if struct.unpack_from('<II',core,start)!=(1,64):raise ValueError('actual Manager header differs')
    checks=dict(player_find_compares_numeric_id='ComponentID::GetID' in assembly['player_find_id'],
        player_find_stops_first_match='je     2d309dc' in assembly['player_find_id'],
        manager_find_requires_notification_type='cmp    $0x26,%eax' in assembly['manager_find'],
        player_initialize_sets_owner='mov    %esi,(%rdi)' in assembly['player_initialize'])
    if not all(checks.values()):raise ValueError(checks)
    return dict(schema='c6-linux-player-notifications-owner-source-v1',gamecore_sha256=TARGET,
        host_sha256=HOST,original_execution=False,methods=code,checks=checks,
        core_sha256=CORE,core_bytes=len(core),owner_trace_decompressed_sha256=TRACE,
        manager_range=[start,end],manager_bytes=end-start,manager_sha256=sha(core[start:end]),
        owner_slots=headers,logical_ids='owner-relative fixed slot/index occurrence, distinct from ComponentID; no structural mutation implemented',
        saved_state_only=True,omitted_live_fields=['locks','pool/lifetime ownership','default blocking','subtype/vtable bindings'],
        native_loading_or_query_comparison=False,complete_state_actions_turns_rng=False)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-output',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();result=audit(a.assets,a.private_output)
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')


if __name__=='__main__':main()
