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
         ('PlayerNotifications.IsRedundant',0x2d30ad0,0xb2),('PlayerNotifications.Add',0x2d30b90,0xaf)]

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
    m.call(0x2d30bcb,0x1f2ec80);m.call(0x2d30bd9,0x1e7d230);m.call(0x2d30c15,0x1eaf290)
    report=dict(schema='c6-linux-loaded-notification-initialization-source-v1',checks_passed=True,
                gamecore_sha256=GAME_SHA,original_execution=False,
                source_sha256=sha((ROOT/'crates/c6-data/src/loaded_actions/notifications.rs').read_bytes()),
                checker_sha256=sha(Path(__file__).read_bytes()),
                methods=[dict(name=n,rva=hex(v),size=z,sha256=sha(m.read(v,z))) for n,v,z in METHODS],
                stock_auto_initialize_text_bindings=bindings,
                checked_sites=[dict(rva=hex(v),bytes=b,meaning=s) for v,b,s in checks],
                compatibility_method='aftermath.Notifications.send means virtual InitializeText, not reporting Send',
                supported_empty_candidate_duplicate=dict(order=['type','location','target ComponentID','existing discriminator key55c7a3f8 absent'],own_notification_id_read=False,
                    requires='actual inherited IsDuplicate slot; prior IsNeeded dispatch; auto=true/null candidate parameters'),
                logical_allocation_scope='native pool addresses/capacity are not semantic gates; initialized fields and all pre-Add reads must match',
                post_add_order=['set added turn if -1','increment nextID and SetID','append','virtual0x90','reportf26f3acf','ExpireAllSupersededBy'],
                actual_pool_or_creator_capture=False,native_creation_or_update_comparison=False,full_action_state_rng_parity=False)
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,auto_initialize_text_bindings=6,original_execution=False)))
if __name__=='__main__':main()
