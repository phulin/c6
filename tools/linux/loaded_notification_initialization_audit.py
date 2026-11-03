#!/usr/bin/env python3
"""Pin notification initialization, pre-Add duplicate and actual event ordering."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA
ROOT=Path(__file__).resolve().parents[2]
METHODS=[('Instance.Initialize',0x2d2d7e0,0x520),('Instance.Create',0x2d2e5b0,0x7a),
         ('Instance.InitializeText',0x2d2dd10,0x70),('Instance.IsDuplicate',0x2d2e2d0,0xdc),
         ('PlayerNotifications.IsRedundant',0x2d30ad0,0xb2),('PlayerNotifications.Add',0x2d30b90,0xaf),
         ('Player.IsTurnComplete',0x2b5bb60,0x19),('Player.PreviouslyActiveThisTurn',0x2b5bbc0,0x2d),
         ('Player.SetEndTurnBlockingState',0x2b5be10,0x52),('Player.Processor.Read',0x2b5a620,0xc4),('Player.Processor.Write',0x2b5a6f0,0xb6),
         ('EndTurnBlockingTypes.Write',0x2c9af80,0x21),('EndTurnBlockingTypes.Read',0x2c9afb0,0x26)]

def sha(value):return hashlib.sha256(value).hexdigest()

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    m=Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA)
    tables=json.loads((ROOT/'evidence/linux/notification-lifecycle.json').read_bytes())['virtual_bindings']
    names=['ChooseTech','ChooseCivic','CommandUnits','FillCivicSlot','CityRangeAttack','ConsiderGovernmentChange']
    bindings=[]
    for name in names:
        row=next(r for r in tables if r['symbol'].endswith(name+'E'))
        address=int(row['vtable_rva'],16)+16
        target=m.relocated_pointer(address+0x80)
        if target!=0x2d2dd10:raise ValueError('auto InitializeText override: '+name)
        bindings.append(dict(subtype=name,addresspoint=hex(address),slot='0x80',target=hex(target)))
    checks=[(0x2d2daf0,'750c','auto=true skips InitializeText during base initialization'),
            (0x2d2f563,'ff9180000000','InitializeText after redundancy before Add'),
            (0x2d30bc5,'8d4201894304','wrapping nextID increment before SetID'),
            (0x2d30be6,'ff9090000000','post-append virtual0x90'),
            (0x2d30c03,'becf3a6ff2','actual added report hashf26f3acf')]
    for address,expected,label in checks:
        if m.read(address,len(bytes.fromhex(expected))).hex()!=expected:raise ValueError(label)
    if m.read(0x2b4ef15,7)!=bytes.fromhex('498db6a8070000') or m.read(0x2b4f860,7)!=bytes.fromhex('498db6a8070000'):
        raise ValueError('saved blocking scalar/cache field differs')
    m.call(0x2b4ef1f,0x1e4cec0);m.call(0x2b4f86a,0x1f2dc00)
    m.call(0x2c9af93,0x1f5fbd0);m.call(0x2c9afbf,0x1e757d0)
    if m.read(0x2c9afc8,3)!=bytes.fromhex("418906"):raise ValueError("raw enum read assignment differs")
    import struct
    for address,target in [(0x2b5a632,0x3c06110),(0x2b5a700,0x3c06110),(0x2b5bbcc,0x3c06110),
                           (0x2b5a66e,0x3c06210),(0x2b5a73c,0x3c06210),(0x2b5bb69,0x3c06210)]:
        if m.read(address,3) not in (bytes.fromhex('488d35'),bytes.fromhex('488d3d')) or address+7+struct.unpack('<i',m.read(address+3,4))[0]!=target:
            raise ValueError('processor array source mapping differs')
    m.call(0x2d30bcb,0x1f2ec80);m.call(0x2d30bd9,0x1e7d230);m.call(0x2d30c15,0x1eaf290)
    report=dict(schema='c6-linux-loaded-notification-initialization-source-v2',checks_passed=True,
                gamecore_sha256=GAME_SHA,original_execution=False,
                source_sha256=sha((ROOT/'crates/c6-data/src/loaded_actions/notifications.rs').read_bytes()),
                shared_setter_source_sha256=sha((ROOT/'crates/c6-core/src/unit_operation_aftermath.rs').read_bytes()),
                checker_sha256=sha(Path(__file__).read_bytes()),
                methods=[dict(name=n,rva=hex(v),size=z,sha256=sha(m.read(v,z))) for n,v,z in METHODS],
                stock_auto_initialize_text_bindings=bindings,
                checked_sites=[dict(rva=hex(v),bytes=b,meaning=s) for v,b,s in checks],
                owner_queries=dict(blocking='PlayerHeader.ordered_scalars[14] == native Player+7a8; actual Read/Write field, not guessed zero',
                    turn_complete='fifth processor bool array ms_abIsTurnComplete at3c06210',
                    previously_active='!first processor activation_event array && !actual current IsTurnActive; short-circuit preserved'),
                compatibility_method='aftermath.Notifications.send means virtual InitializeText, not reporting Send',
                supported_empty_candidate_duplicate=dict(order=['type','location','target ComponentID','existing discriminator key55c7a3f8 absent'],own_notification_id_read=False,
                    requires='actual inherited IsDuplicate slot; prior IsNeeded dispatch; auto=true/null candidate parameters'),
                logical_allocation_scope='native pool addresses/capacity are not semantic gates; initialized fields and all pre-Add reads must match',
                post_add_order=['set added turn if -1','increment nextID and SetID','append','virtual0x90','reportf26f3acf','ExpireAllSupersededBy'],
                actual_pool_or_creator_capture=False,native_creation_or_update_comparison=False,full_action_state_rng_parity=False)
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,auto_initialize_text_bindings=6,original_execution=False)))
if __name__=='__main__':main()
