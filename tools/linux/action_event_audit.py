#!/usr/bin/env python3
"""Exact six reporting overloads and bounded historical return-use audit; no native execution."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

PROFILE = 'proven_skipturn_callers_v2'
METHODS = [('gamecore', 47491488, 239, 'Send.ValueValue', '6c10753489e9561b949dc7d044fcfc4341265597c58f39d7944149a5afc67259'), ('gamecore', 47486000, 149, 'Send.Type', '261df2cf341d0de0f60e7fd1055f2f5b1d9d706e7be567497673c0d083adf22f'), ('gamecore', 47486160, 165, 'Send.Identity', 'c4442f27d4aef52737d8aa4f462c3a99d2575d484f09ee088673f20c020fe54c'), ('gamecore', 47486336, 196, 'Send.IdentityInt', 'f7c2eed021070f37bf9da17af534c4f6c69702aa4588b02afe7693dbd777976a'), ('gamecore', 47486544, 209, 'Send.IdentityIntInt', 'e66ac378f4f4ad435dd54f9b721091e94d079f86486f5619a67afa2bdb4a2e4c'), ('gamecore', 47500656, 94, 'Value.ComponentID.C1', '8b23d01e532390d0575dcccae77ddd0c4f17be3a3cdfc296cae1af44df9911e4'), ('gamecore', 47499632, 58, 'Value.Bool.C1', 'c4b2a608a1daacdfb7d6b463f4e316d5e6fe489dcc595760998d4a64615ae987'), ('gamecore', 47493792, 3, 'Value.GetKey', 'cadd8f26469e7198cb603ea5ba1b165c25958dd0f2df803f0ade24798f5915ac'), ('gamecore', 47493808, 5, 'Value.GetValue', '337222f0640328de908dacc52ced92ff844fe6d4f480a84a6e4aa98bb839dc80'), ('gamecore', 47373696, 170, 'Notification.Activate', '9192f22f1e862e7d178aa4b1f2d2df25ce9db724048778cea71dd90545a8d74c'), ('host', 52517728, 23, 'TypedVariant.GetAsComponentID', '757433ab2ffc48146a8d077c5d3a5f3303dde025beafcdd92ac19df962119708'), ('host', 52255216, 15, 'TypedVariant.SetBool', 'b11e40fb29b084f9e430c29dfc1b4a2103c8dfbdd03e881e35f36131cf3e0e51'), ('host', 52255232, 12, 'ArrayVariant.SetBool', 'd8185b6ff5f06b83f5084fa54d0d6335c8e08db51e550b68d23d180a47bc235f'), ('host', 49689760, 14, 'TypedVariant.IsArray', 'f34dcf459773e7f857e1ad157d46af70e1597597c627d8f3d2ff62b456679a4a'), ('host', 49691712, 9, 'ArrayVariant.ElementCount', 'e2eede0eb4f9d6b2d384bf3b6cea3ad9006081f4fd8943a34e0687f444bd8ce8'), ('gamecore', 47847488, 99, 'OldCall.SetActivity', '3a27b2b9a2b33e4a5e2a2752db9898c104287e8fbdd97757974fd045f7dac837'), ('gamecore', 47801536, 92, 'OldCall.Deactivate', '5565a0000d074995967258e4eacce940f68123b2fbc9c3659cbaf368606d424b'), ('gamecore', 47848640, 176, 'OldCall.PopFront', 'f26e2c34d6283beb3a7947ffc17e211be2bf1c61289134a503a7042a47eec341'), ('gamecore', 47850976, 148, 'OldCall.InsertAtEnd', '994b4ed82208aae3f9b924044510ad88ed620ce9b9b1e4e41d84e37dc3432305'), ('gamecore', 47892704, 105, 'OldCall.SkipStart', 'ff9d84e4a36edb8c79756b6301aeda9826cc7f86dbfaa73af8d91f160c050e1a')]


METHODS += [('gamecore', 47488880, 203, 'Send.TypeIntInt', 'aea0072a2473016e436e12da9e82c6be1bb2457339daa4e906fd1cb1e1da39a2'), ('gamecore', 45465104, 82, 'Player.SetEndTurnBlocking', 'e20044c8a217e29d4f7fc297bffbfe836f656c7b99b7ed7db376bef55f18d521')]

def audit(assets):
    elves = {'gamecore': Elf(assets/'libGameCore_XP2.so', GAME_SHA), 'host': Elf(assets/'Civ6', HOST_SHA)}
    for module,address,size,name,expected in METHODS:
        if hashlib.sha256(elves[module].read(address,size)).hexdigest()!=expected:
            raise ValueError('original reporting method differs: '+name)
    core=elves['gamecore']
    calls=[(0x2d2dde4,0x1e578a0),(0x2d24935,0x1ef19d0),(0x2b5be54,0x1e8db60),(0x2da1897,0x1fb5910),
           (0x2d9650b,0x1eea180),(0x2da1d5d,0x1eaf290),(0x2da263f,0x1fb5910),(0x2dac91d,0x1fb5910)]
    for address,target in calls:core.call(address,target)
    for address,expected in [(0x2da189c,'4883c4185b5dc3'),(0x2da1d62,'4883c4105b415c415d415e415fc3'),(0x2d96510,'31c0')]:
        if core.read(address,len(bytes.fromhex(expected)))!=bytes.fromhex(expected):raise ValueError('old event return-use epilogue differs')
    source=Path(__file__).resolve().parents[2]/'tools/linux/action_execution_trace.c'
    content=source.read_text()
    for name in ['trace_event_id','trace_event_id_int','trace_event_id_two_int','trace_event_type','trace_event_values','trace_event_two_int']:
        if 'int '+name+'(' not in content:raise ValueError('int reporting forwarding wrapper missing')
    return dict(schema='c6-linux-action-event-abi-v1',checks_passed=True,profile=PROFILE,
        host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
        methods=[dict(module=m,address=hex(a),size=n,name=s,sha256=h)for m,a,n,s,h in METHODS],
        calls=[dict(address=hex(a),target=hex(t))for a,t in calls],
        source_sha256={'tools/linux/action_execution_trace.c':hashlib.sha256(source.read_bytes()).hexdigest()},
        event_type_size=8,value_size=32,variant_size=24,
        abi='int Send(uint64 Type RDI, const Value* RSI, const Value* RDX); int EAX return; Type-only/identity overloads also int',
        value_layout={'key':0,'variant':8,'variant_type_u16':8,'modifiers_u16':10,'storage':16,'element_size_u32':24,'count_flags_u32':28},
        notification_event=dict(hash='025f6cfd',first_key='eab69e5a',first_type='native ComponentID userdata1024, element8/count1',second_key='78a2f946',second_type='native booltype1 scalar'),
        queue_event=dict(hash='9ea4c222',overload='Type-only',ordering='after original Receive/Execute and queue pop'),
        conditional_blocking_event=dict(hash='55f15a7b',overload='Type,int,int',ordering='only changed local player blocking; old/new integers'),
        old_return_sites={'SetActivity':'void enclosing method epilogue','Deactivate':'immediate xorEAX0','PopFront':'void enclosing method epilogue','InsertAtEnd':'GetSize overwrites EAX before compare','SkipStart':'Manager.Edit overwrites RAX before use'},
        old_trace_caller_pc_recorded=False,old_wrapper_declared_return='void; archived stack-canary epilogue clobbers RAX',
        selected_overloads=['identity','identity_int','identity_two_int','type_only','value_value','type_two_int'],
        coverage='Known original SkipTurn replacement/activation/queue-drain callers only; transitive subscribers/other overloads not a complete engine event inventory',
        return_forwarding='save actual original int, log after-call observation, return saved int',
        native_execution=False,complete_event_inventory=False,gameplay_parity=False)


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--assets',type=Path,default=Path(__file__).resolve().parents[3]/'c6-assets-linux');p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    result=audit(a.assets);a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(dict(checks_passed=True,profile=PROFILE,native_execution=False)))


if __name__=='__main__':main()
