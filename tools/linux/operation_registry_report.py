#!/usr/bin/env python3
"""Validate private native active-operation registry and all attributed Unit queues."""
import argparse
from collections import Counter
import gzip
import json
from pathlib import Path
import struct
from operation_registry_audit import ROOT, contract, sha
from unit_codec_check import SAVES, TRACES


def require(value, message):
    if not value:
        raise ValueError(message)


def queue_corpus(private_root, entries, candidate):
    by_hash={entry['hash']:entry for entry in entries if entry['handler_present']}
    # Exact current wire shapes proved in unit-wire-contracts-operations.json.
    sizes={0x8374d954:64,0x580f2f68:52,0xe547bc17:40,0xd3a6f7e8:40,
           0x223eea54:40,0xc31647e8:40,0xed72b77d:40,0x09c1ecc1:40}
    output=[]
    manifest=json.loads((private_root/'evidence/linux-game-load/canonical/manifest.json').read_text())
    require(manifest['module_sha256']=='a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa','canonical module mismatch')
    for save_hash,(trace_name,trace_hash) in zip(SAVES,TRACES):
        with gzip.open(private_root/'evidence/linux-game-load/canonical'/f'{save_hash}.bin.gz','rb') as f:
            data=f.read(80657463)
        require(len(data)==80657462 and sha(data)==save_hash,'canonical Save identity mismatch')
        with gzip.open(private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name,'rb') as f:
            trace=f.read()
        require(sha(trace)==trace_hash,'serializer trace identity mismatch')
        rows=[json.loads(line) for line in trace.splitlines() if b'"owner":"GameCore::Unit::Operations::OperationQueue"' in line]
        rows=sorted((r for r in rows if r['save']==1),key=lambda r:r['start'])
        require(len(rows)==1262,'native Unit queue count mismatch')
        counts=Counter()
        last_end=0
        for row in rows:
            start,end=row['start'],row['end']
            require(last_end<=start<end<=len(data),'invalid queue range')
            last_end=end
            raw=data[start:end]
            version,count=struct.unpack_from('<iI',raw)
            require(version==1 and count in (0,1),'unsupported corpus queue shape')
            if not count:
                require(len(raw)==8,'empty queue framing mismatch')
                counts['empty']+=1
                continue
            h=struct.unpack_from('<I',raw,8)[0]
            require(h in sizes and len(raw)==sizes[h],'unsupported attributed handler shape')
            require(struct.unpack_from('<i',raw,12)[0]==2,'native node version mismatch')
            require(h in by_hash,'native wire hash has no active handler')
            e=by_hash[h]
            require(e['resolved_id']==e['id']==e['handler_id'],'wire hash resolution differs')
            counts[f'{h:08x}']+=1
        # Unit version-28 leading named fields: id at +4, owner at +40.
        # Boundaries are original trace ownership, never searched byte patterns.
        unit_rows=[json.loads(line) for line in trace.splitlines() if b'"owner":"GameCore::Unit::Instance"' in line]
        unit_rows=[r for r in unit_rows if r['save']==1]
        selected=candidate['selected']
        matches=[r for r in unit_rows if struct.unpack_from('<i',data,r['start'])[0]==28
                 and struct.unpack_from('<I',data,r['start']+4)[0]==selected['id']
                 and struct.unpack_from('<i',data,r['start']+40)[0]==selected['owner']]
        require(len(matches)==1,'candidate Unit identity missing or ambiguous')
        queues=[r for r in rows if r['parent']==matches[0]['call']]
        require(len(queues)==1,'candidate original queue ownership ambiguous')
        raw=data[queues[0]['start']:queues[0]['end']]
        before=candidate['live_observations'][0]
        require(before['stage']=='before_a' and before['queue_size']==1 and len(before['queue'])==1,'unsupported existing candidate snapshot')
        node=before['queue'][0]
        h=struct.unpack_from('<I',raw,8)[0]
        require(by_hash[h]['id']==node['operation'],'candidate numeric ID/wire hash mismatch')
        for key,offset in [('directive',16),('flags',20),('push_turn',24),('pre_activity',28),('pre_automation',32)]:
            require(node[key]==struct.unpack_from('<I',raw,offset)[0],'candidate queue field differs: '+key)
        output.append(dict(save_sha256=save_hash,trace_sha256=trace_hash,units=1262,
                           candidate_snapshot_matches=1,
                           nonempty_queues=sum(v for k,v in counts.items() if k!='empty'),
                           queue_counts=dict(sorted(counts.items()))))
    return output


def audit(run,abi,inventory,before_path,after_path,private_root):
    config=run['configuration']
    require(sha(json.dumps(config,sort_keys=True,separators=(',',':')).encode())==run['configuration_sha256'],'configuration digest mismatch')
    require(config.get('schema')=='c6-linux-mod-activation-v1','runtime schema mismatch')
    require(config['assets'].get('libGameCore_XP2.so')==abi['gamecore_sha256'] and config['assets'].get('Civ6')==abi['host_sha256'],'original module gates missing')
    require(config.get('lower_core_operation_registry_observer')==abi,'registry ABI gate mismatch')
    for path in ['native/linux/operation_registry.c','native/linux/operation_registry.h']:
        require(config.get('sources',{}).get(path)==sha((ROOT/path).read_bytes()),'observer source mismatch: '+path)
    require(config.get('cleanup_requested') is True,'cleanup not configured')
    require(type(run.get('returncode')) is int and run['returncode']==0 and run.get('framework_cleanup_proven') is True and run.get('client_cleanup_proven') is True,'native execution or cleanup failed')
    lines=run['stderr'].splitlines()
    require(any(line.startswith('C6_LINUX_GAME_LOAD module ') and '/libGameCore_XP2.so context_initialized 1 ' in line for line in lines),'loaded initialized XP2 context missing')
    rows=[json.loads(line.split(' ',1)[1]) for line in lines if line.startswith('C6_LINUX_OPERATION_REGISTRY ')]
    require(len(rows)==1,'expected exactly one complete native registry record')
    record=rows[0]
    entries=record['entries']
    n=record['count']
    require(type(n) is int and 0<n<=1000000 and record['handler_slots']==n==len(entries),'registry dimensions mismatch')
    selected={x['hash']:x for x in inventory['selected_by_hash']}
    seen=set()
    for i,e in enumerate(entries):
        require(type(e['id']) is int and e['id']==i,'native definition index differs')
        require(type(e['hash']) is int and 0<=e['hash']<=0xffffffff and e['hash'] not in seen,'duplicate or invalid definition hash')
        seen.add(e['hash'])
        require(type(e['handler_present']) is bool and e['lookup_matches'] is True,'native handler lookup differs')
        if not e['handler_present']:
            require(e['resolved_id']==-1,'null handler has active hash mapping')
            require(e['hash'] not in selected,'registered definition unexpectedly lacks handler')
            continue
        require(e['resolved_id']==e['handler_id']==i and e['handler_hash']==e['hash'],'native handler/index/hash mismatch')
        require(e['hash'] in selected,'unexpected active hash outside actual XP2 registrations')
        expected=selected[e['hash']]
        for key in ['vtable_rva','make_parameters_rva','write_rva','read_rva']:
            require(type(e[key]) is int and e[key]==expected[key],'active concrete handler mismatch: '+key)
    require(type(record['missing_hash']) is int and n<=record['missing_hash']<=0xffffffff and record['missing_id']==-1 and record['missing_handler_is_null'] is True,'absent-hash boundary mismatch')
    captures=[]
    for label,path in [('save',before_path),('repeat',after_path)]:
        data=path.read_bytes()
        identity=dict(sha256=sha(data),bytes=len(data))
        require(all(run['game_saves'][label][k]==v for k,v in identity.items()),'actual capture identity mismatch')
        captures.append((data,identity))
    require(captures[0][0]==captures[1][0],'full Core Save changed around registry query')
    candidate_path=ROOT/'evidence/linux/action-dispatch-control-6.json'
    candidate=json.loads(candidate_path.read_text())
    require(candidate['native_full_cleanup'] is True and candidate['mode']=='control','candidate control identity invalid')
    corpus=queue_corpus(private_root,entries,candidate)
    return dict(mapping=record,configuration=config,configuration_sha256=run['configuration_sha256'],
        canonical_save=captures[0][1],canonical_core_unchanged=True,full_live_state_unchanged_proven=False,
        registered_hashes_without_definition=len(set(selected)-seen),
        corpus=corpus,candidate_report_sha256=sha(candidate_path.read_bytes()),active_handlers=sum(e['handler_present'] for e in entries),null_handlers=sum(not e['handler_present'] for e in entries))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--runtime-report',type=Path,required=True)
    p.add_argument('--before-save',type=Path,required=True)
    p.add_argument('--after-save',type=Path,required=True)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--private-output',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    abi,inventory=contract(args.assets)
    result=audit(json.loads(args.runtime_report.read_text()),abi,inventory,args.before_save,args.after_save,args.private_root)
    result['runtime_report_sha256']=sha(args.runtime_report.read_bytes())
    result['audit_source_sha256']=sha(Path(__file__).read_bytes())
    args.private_output.parent.mkdir(parents=True,exist_ok=True)
    args.private_output.write_text(json.dumps(result,indent=2)+'\n')
    public=dict(schema='c6-linux-operation-registry-observation-v1',gamecore_sha256=abi['gamecore_sha256'],
        host_sha256=abi['host_sha256'],configuration_sha256=result['configuration_sha256'],
        runtime_report_sha256=result['runtime_report_sha256'],private_report_sha256=sha(args.private_output.read_bytes()),
        audit_source_sha256=result['audit_source_sha256'],registry_count=result['mapping']['count'],
        active_handlers=result['active_handlers'],null_handlers=result['null_handlers'],
        registered_hashes_without_definition=result['registered_hashes_without_definition'],
        mapping_sha256=sha(json.dumps(result['mapping'],sort_keys=True,separators=(',',':')).encode()),
        canonical_save=result['canonical_save'],canonical_core_unchanged=True,full_live_state_unchanged_proven=False,
        corpus=result['corpus'],candidate_report_sha256=result['candidate_report_sha256'],full_state_action_turn_parity=False)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(public,indent=2)+'\n')
    print(json.dumps(dict(active_handlers=public['active_handlers'],null_handlers=public['null_handlers'],native_units=sum(x['units'] for x in public['corpus']))))

if __name__=='__main__':
    main()
