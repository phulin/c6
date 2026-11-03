#!/usr/bin/env python3
"""Opt-in passive Linux script-state image at genuine selected CCall windows.
No inferior calls, locks, Lua APIs, GC operations or callback execution. Decoder
results describe copied bytes only; GC/semantic admission remains unresolved.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from script_hks_registry import Image,SnapshotError,Table,inspect_roots,inspect_named_event,table_provenance,load_image
from script_manager_observer import CaptureImage,inspect_resolved_function,SITES

ROOT=Path(__file__).resolve().parents[2]
SCHEMA='c6-linux-script-natural-snapshot-v1'
PHASES=('acquired','resolved','before_release')
# Whole pinned host-body identities for the late inline-vector/variant layout.
ARGUMENT_BODIES=[('ArgsCount', 52714304, 9, '1a2ecf211c66613088260199646da57d0fb77aa50284ca0b5121a8be1e9407a1'), ('ArgsGetInt', 52714768, 81, '75b6c47a4e21230256e108a5d55043c22a529a3ae6ddec07c9396f0b79f04b1c'), ('ArgsVectorCtor', 52717168, 66, '09b8d493cbbc78da74645c0fee3366a0bb9b5e5456500d299cd8a1ae3aff4655'), ('ArgsVectorSize', 52717392, 7, '1c6dc39d1c7e9b73d24abadbd210d86aee248e730d3a47fd9c16d4f10af461f2'), ('ArgsVectorAt', 52717584, 26, 'bf4e8119e5fb04ef13273210fc059e1b76ba56ba812b8f82140781a6441057bc'), ('VariantSetInt', 52255248, 15, '19acf61dde8576acabde9f7814b02ebd3c6dd30045a5f94457a1faf0c59427f2'), ('ArrayVariantSetInt', 52255264, 11, '88d9556e1b0444dd7ac7132cb1f9ce5a80f7eafc6bdf5c2a32d8f718c74bd6c9'), ('TypedVariantGetValue', 52047120, 5, '337222f0640328de908dacc52ced92ff844fe6d4f480a84a6e4aa98bb839dc80')]


def require(value,reason):
    if not value:raise SnapshotError(reason)


def validate_spec(spec):
    require(spec.get('schema')==SCHEMA,'wrong natural script snapshot schema')
    require(spec.get('phases') and set(spec['phases'])<=set(PHASES),'unknown/empty natural snapshot phases')
    require(type(spec.get('byte_budget')) is int and 1024<=spec['byte_budget']<=32*1024*1024,'snapshot byte budget')
    require(type(spec.get('max_items')) is int and 1<=spec['max_items']<=65536,'snapshot item budget')
    require(spec.get('source_sha256')==hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'snapshot collector source identity')


def arguments(image,pointer):
    raw=image.read(pointer,0x1b8)
    vptr=struct.unpack_from('<Q',raw)[0]
    result=dict(pointer=hex(pointer),raw_object=raw.hex(),vptr=hex(vptr),
                concrete_binding=vptr==0x53a6300,unsupported_variant_cells=[])
    require(vptr==0x53a6300,'actual ScriptSystemArgs override is not decoded')
    data=struct.unpack_from('<Q',raw,0x1a0)[0]
    capacity,size=struct.unpack_from('<I',raw,0x1a8)[0],struct.unpack_from('<I',raw,0x1b0)[0]
    require(size<=capacity<=image.limit,'actual argument vector extent unsupported')
    require(size==0 or data>0,'actual argument data null')
    cells=image.read(data,size*24) if size else b''
    result.update(parent_system=hex(struct.unpack_from('<Q',raw,8)[0]),data=hex(data),capacity=capacity,size=size,cells=[])
    for index in range(size):
        cell=cells[index*24:(index+1)*24];kind=struct.unpack_from('<H',cell)[0]
        item=dict(index=index,raw=cell.hex(),type_low16=kind)
        if kind==2:item['signed_i32']=struct.unpack_from('<i',cell,8)[0]
        else:result['unsupported_variant_cells'].append(index)
        result['cells'].append(item)
    return result


def weak_mode(image,table):
    record=table_provenance(image,table)
    meta=struct.unpack_from('<Q',table.header,0x10)[0]
    if meta:
        require(not meta&1,'struct metatable weak mode remains unsupported')
        mode=Table(image,meta).lookup_string(b'__mode')
        record['raw_weak_mode_status']=mode['status']
        if mode['value'] is not None:
            record['raw_weak_mode_value']=mode['value'].record()
            if mode['value'].tag==4:
                record['raw_weak_mode_bytes_hex']=image.string(mode['value'].bits).hex()
    return record


def decode(image,state,args,base,resolved=False):
    # Independent sections preserve useful facts when a different graph is
    # unsupported, unreadable or exceeds the shared bound. No absent defaults.
    result=dict(GC_classification='unresolved',pthread_return_status_observed=False,
        initialized_VM_proven=False,ASL_owner_zero_missing_lock_inferred=False,
        semantic_snapshot_admitted=False,callback_effects_supported=False,sections={},errors={})
    def section(name,fn):
        try:result['sections'][name]=fn()
        except Exception as error:result['errors'][name]=str(error)
    def state_bytes():
        header=image.read(state,0x80);glob=struct.unpack_from('<Q',header,0x10)[0]
        return dict(actual_CCall_state=hex(state),raw_header=header.hex(),
            global_state_pointer=hex(glob),collector_prefix_raw=image.read(glob+0x58,0x128).hex())
    def lock_bytes():
        lock=image.read(0x6ec9c70,64);mutex=struct.unpack_from('<Q',lock,0x18)[0]
        return dict(raw64=lock.hex(),pointed_pthread_mutex=hex(mutex),
            pointed_pthread_mutex_raw=image.read(mutex,40).hex())
    def actual_function():
        actual=inspect_resolved_function(image,state);closure=actual['closure']
        if closure['kind']=='cclosure' and closure['function']=='0x36b3540':
            actual['groups']=weak_mode(image,Table(image,int(closure['upvalues'][0]['payload_hex'],0)))
            for group in actual['callback_arrays']:
                group['table']=weak_mode(image,Table(image,int(group['table']['pointer'],0)))
        return actual
    section('state',state_bytes)
    section('Lua_lock',lock_bytes)
    section('arguments',lambda:arguments(image,args))
    if resolved:section('actual_resolved_function',actual_function)
    section('current_singleton_roots',lambda:inspect_roots(image,base))
    section('raw_named_event',lambda:inspect_named_event(image,state))
    return result


class NaturalSnapshot:
    def __init__(self,read,emit,base,profile,directory,spec):
        validate_spec(spec)
        self.read,self.emit,self.base,self.profile=read,emit,base,profile
        self.directory=Path(directory).resolve();self.spec=spec;self.ordinal=0
        require(not self.directory.is_relative_to(ROOT) and not self.directory.exists(),'fresh private script snapshot directory required')
        self.directory.mkdir(parents=True,exist_ok=False)

    def selected(self,phase,r,frames,call):
        if phase not in PHASES or r.get('pc')!=SITES[phase]:return False
        if not call or 'entry_rsp' not in call:return False
        if r['r14']!=call['context']:return False
        if phase in ('acquired','before_release'):
            return r['rsp']==call['entry_rsp']-24 and r['r15']==0x36b3bc0 and (phase!='acquired' or r['rbx']==call['state'])
        if r['rbx']!=call['state']:return False
        # Actual original ancestor register/RSP facts, not a prediction of the
        # intermediate HKS/GC stack or a null-ASL-owner rejection.
        parents=[row for row in frames if row.get('pc')==0x3bf2308]
        return any(row.get('native_registers',{}).get('rsp')==call['entry_rsp']-24 and
                   row['native_registers'].get('r14')==call['context'] and
                   row['native_registers'].get('r15')==0x36b3bc0 and
                   row['native_registers'].get('rbx')==call['state'] for row in parents)

    def capture(self,phase,r,frames,call):
        if phase not in self.spec['phases']:return
        if not self.selected(phase,r,frames,call):
            self.emit(dict(event='script_snapshot_unattributed',phase=phase,
                reason='selected actual CCall context/ancestor register facts unavailable',
                GC_classification='unresolved',callback_absence_proven=False))
            return
        self.ordinal+=1
        image=CaptureImage(self.read,budget=self.spec['byte_budget'],limit=self.spec['max_items'])
        identity=dict(configuration_sha256=self.profile,gamecore_base=hex(self.base),phase=phase,
            occurrence=self.ordinal,source_schema=SCHEMA,GC_classification='unresolved',
            selected_CCall_context=hex(call['context']),actual_CCall_state=hex(call['state']),
            actual_CCall_arguments=hex(call['args']),collector_source_sha256=self.spec['source_sha256'],
            observation_boundary_accepted=False)
        result=None;failure=None
        try:
            result=decode(image,call['state'],call['args'],self.base,phase=='resolved')
            replay=decode(Image(image.merged(),image.limit),call['state'],call['args'],self.base,phase=='resolved')
            require(all(replay['sections'].get(key)==value for key,value in result['sections'].items()),
                    'immutable image offline decoder differs')
            if result['errors']:failure='one or more bounded snapshot sections unresolved'
        except Exception as error:
            # Inspection/layout/budget failure is a retained diagnostic, not an
            # absent registry or a reason to execute a substitute native call.
            failure=str(error)
        manifest=image.write_private(self.directory/('snapshot-%04d-%s'%(self.ordinal,phase)),
            dict(identity,partial=failure is not None,failure=failure))
        self.emit(dict(event='script_natural_snapshot',**identity,image_manifest=manifest,
            decoder_result=result,decoder_failure=failure,partial=failure is not None,
            registry_absence_inferred=False,callback_effects_accepted=False,
            selected_ancestor_facts=frames,collector_stopped_or_pinned_by_observer=False))


def main():
    parser=argparse.ArgumentParser(description='Decode a retained passive natural-window image; no process access.')
    parser.add_argument('--image',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();manifest,image=load_image(args.image)
    require(manifest.get('source_schema')==SCHEMA,'not a natural-window image')
    result=decode(image,int(manifest['actual_CCall_state'],0),int(manifest['actual_CCall_arguments'],0),
        int(manifest['gamecore_base'],0),manifest['phase']=='resolved')
    args.output.write_text(json.dumps(dict(schema=SCHEMA,offline_decode=True,
        image_manifest_sha256=hashlib.sha256(args.image.read_bytes()).hexdigest(),
        collector_source_sha256=manifest['collector_source_sha256'],result=result),indent=2,sort_keys=True)+'\n')


if __name__=='__main__':main()
