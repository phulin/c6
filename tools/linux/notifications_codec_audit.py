#!/usr/bin/env python3
"""Read-only exact Linux notification wire audit; executes no original code."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from skip_turn_state_success import original_contract
from save_lifecycle_audit import GAME_SHA
ROOT = Path(__file__).resolve().parents[2]

def digest(b):
    return hashlib.sha256(b).hexdigest()

def audit(module, before, after, iterations):
    contract = original_contract(module)
    sections = [('before', before, 76400879, 76404572,
                 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'),
                ('after', after, 76400735, 76403364,
                 'be08f44a4b83bd7eaa8d15a24443731764705c32961c570697bc3bb1736097bc')]
    subprocess.run(['cargo','build','--release','-p','c6-core','--example','notifications_codec'],cwd=ROOT,check=True)
    results={}
    with tempfile.TemporaryDirectory(prefix='c6-notifications-') as tmp:
        d=Path(tmp)
        for label,path,start,end,expected in sections:
            data=path.read_bytes()
            if digest(data)!=expected: raise ValueError('wrong complete native snapshot identity: '+label)
            owner=data[start:end]; src=d/(label+'.bin'); src.write_bytes(owner)
            out=d/(label+'.out'); bounds=d/(label+'.csv')
            result=subprocess.check_output([str(ROOT/'target/release/examples/notifications_codec'),str(src),str(out),str(bounds),str(iterations)],text=True)
            if out.read_bytes()!=owner: raise ValueError('roundtrip mismatch')
            rows=[]
            for line in bounds.read_text().splitlines():
                a,b,id_,type_,dismissed,activated,ms,me,ss,se,ps,pe=line.split(',')
                a,b=int(a),int(b)
                rows.append(dict(start=a,end=b,id=int(id_),type_hash=int(type_),dismissed=dismissed=='true',activated=activated=='true',sha256=digest(owner[a:b]),summary_range=[int(ss),int(se)],message_range=[int(ms),int(me)],parameters_range=[int(ps),int(pe)]))
            reference=json.loads((ROOT/'evidence/linux/skip-turn-state-execute-3.json').read_text())['notification_container'][label]['records']
            if len(reference)!=len(rows): raise ValueError('independent record count differs')
            for row, ref in zip(rows,reference):
                for key in ['start','end','id','type_hash','summary_range','message_range','parameters_range']:
                    if row[key]!=ref[key]: raise ValueError('independent typed boundary differs: '+key)
                if row['sha256']!=ref['record_sha256']: raise ValueError('independent record hash differs')
            results[label]=dict(independent_typed_boundaries_checked=True,core_sha256=expected,owner_range=[start,end],owner_sha256=digest(owner),records=rows,checks=json.loads(result),byte_exact=True)
    return dict(schema='c6-linux-notifications-codec-v1',gamecore_sha256=GAME_SHA,original_contract=contract,independent_state_report_sha256=digest((ROOT/'evidence/linux/skip-turn-state-execute-3.json').read_bytes()),fixtures=results,
                text_field_labels=dict(first='message',second='summary',correction='Target DWARF and compiled getters corroborate +20=message, +a0=summary; historical raw bytes unchanged'),
                acceptance=dict(component_wire_only=True,original_code_executed=False,gameplay_parity=False,full_action_acceptance=0),
                limits='Only PlayerNotifications v1 and Instance v5; locale v1 zero arguments and supported typed variant branches. No factory creation, native Load normalization, selection, cleanup/destruction, activation event subscribers, complete gameplay state/action/turns. Performance measures codec only. Unknown save bytes and the separate 89,198-byte Save→Load failure remain unclassified/unmasked.')

def main():
    p=argparse.ArgumentParser(); p.add_argument('--module',type=Path,required=True); p.add_argument('--before',type=Path,required=True); p.add_argument('--after',type=Path,required=True); p.add_argument('--output',type=Path,required=True); p.add_argument('--iterations',type=int,default=1000)
    a=p.parse_args()
    if a.iterations<=0: p.error('iterations must be positive')
    result=audit(a.module,a.before,a.after,a.iterations)
    result['sources']={name:digest((ROOT/name).read_bytes()) for name in ['crates/c6-core/src/notifications.rs','crates/c6-core/examples/notifications_codec.rs','tools/linux/notifications_codec_audit.py']}
    a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_text(json.dumps(result,indent=2)+'\n')
if __name__=='__main__': main()
