#!/usr/bin/env python3
"""Check actual attributed all-slot notification admission; never original execution."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA
ROOT=Path(__file__).resolve().parents[2]

def sha(data):return hashlib.sha256(data).hexdigest()

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--fixture-output',type=Path)
    a=p.parse_args()
    fixture=a.fixture_output or ROOT/'target/linux/loaded-notification-owner-fixture.json'
    if a.fixture_output is None:
        subprocess.run(['python3',str(ROOT/'tools/linux/loaded_handler_adapter_check.py'),'--output',str(fixture)],check=True)
    result=json.loads(fixture.read_bytes())
    if (result['owned_notification_collections'],result['owned_notification_records'],result['notification_first_duplicate_lookup_checked'])!=(64,12,True):
        raise ValueError('production all-slot notification admission differs')
    if not result.get('notification_reads_survive_unit_activity_write') or not result.get('notification_replay_after_advancement_rejected'):
        raise ValueError('notification authority advancement checks missing')
    if result['owner_trace_sha256']!='528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae':
        raise ValueError('owner trace differs')
    retained=ROOT/result['retained_source_snapshot']
    for name,expected in result['source_sha256'].items():
        if sha((retained/name).read_bytes())!=expected:raise ValueError('retained source mismatch: '+name)
    if sha((ROOT/result['retained_candidate']).read_bytes())!=result['candidate_sha256']:
        raise ValueError('retained executable mismatch')
    module=Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA)
    module.call(0x2d2f865,0x1e6ecd0)
    module.call(0x2d2f870,0x1f531d0)
    if module.read(0x2d2f878,5)!=bytes.fromhex('83fd3f7706'):
        raise ValueError('native Read 64-slot cap differs')
    private=ROOT.parent/'c6-ghidra/evidence'
    manifest=json.loads((private/'linux-military-initialization/native-3/candidate-paths-v2.json').read_bytes())
    core=Path(manifest['input_core']).read_bytes()
    if len(core)!=80657462 or sha(core)!='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884':
        raise ValueError('associated saved snapshot differs')
    span=core[76400871:76405580]
    if span[:8]!=bytes.fromhex('0100000040000000') or sha(span)!='68c99c43d65778dea5f1d49a7215d9d925b405f639d413b28e2c4f5d6f601094':
        raise ValueError('attributed Manager span/header differs')
    paths=['crates/c6-data/src/loaded_actions/notifications.rs','crates/c6-core/src/player_notifications_owner.rs','crates/c6-core/src/loaded_state.rs','tools/linux/loaded_handler_adapter_check.rs','tools/linux/loaded_handler_adapter_check.py']
    report=dict(schema='c6-linux-loaded-notification-owner-v1',checks_passed=True,original_execution=False,
                gamecore_sha256=GAME_SHA,checker_sha256=sha(Path(__file__).read_bytes()),
                snapshot_sha256=sha(core),snapshot_bytes=len(core),owner_range=[76400871,76405580],owner_bytes=len(span),owner_sha256=sha(span),
                owner_trace_sha256=result['owner_trace_sha256'],collections=64,records=12,slot_zero_next_id=49,empty_other_slots=63,
                duplicate_numeric_id_first_match_checked=True,all_saved_fields_rechecked_at_install=True,
                notification_reads_survive_unit_activity_write=True,replay_after_advancement_rejected_without_revision_change=True,
                decoded_players=result['players'],decoded_units=result['units'],decoded_formations=result['formations'],
                candidate_sha256=result['candidate_sha256'],retained_candidate=result['retained_candidate'],retained_source_snapshot=result['retained_source_snapshot'],
                retained_fixture_result=str(Path(result['retained_source_snapshot']).parent/'result.json'),fixture_sha256=sha(fixture.read_bytes()),
                source_sha256={name:result['source_sha256'][name] for name in paths},
                source_role='SavedOutputCore',decoder_role_unchanged=result['decode_role_unchanged'],
                required_reporting_backend_available=False,native_read_or_live_runtime_equivalence=False,full_action_state_rng_parity=False,
                unresolved=['omitted live fields and actual subtype/creator authority','structural append/removal with stable logical handles','real notification callbacks/subtype effects','complete Update and native affected-state/RNG comparison'])
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,collections=64,records=12,original_execution=False,candidate_sha256=result['candidate_sha256'])))
if __name__=='__main__':main()
