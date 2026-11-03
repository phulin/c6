#!/usr/bin/env python3
"""Gate exact original Linux cache dispatch and genuine network send boundaries."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA


EXPECTED_PLT = [(0x3b21cc0,
 '_ZN8GameCore8Messages7Manager16BroadcastMessageERNS0_11BaseMessageE'),(0x3bc1560,
  '_ZN8GameCore9Diplomacy7Manager16ProcessStatementEPNS0_9Statement8InstanceE'),
 (0x3b97fa8,
  '_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDEii'),
 (62034136, '_ZN8GameCore4Game9Processor6UpdateEv'),
 (62104680, '_ZN8GameCore2AI8GlobalAi9ProcessAiEv'),
 (62146424, '_ZN8GameCore8Messages7Manager14ReceiveMessageEiRN13Serialization13MemoryIStreamE'),
 (62367792, '_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDE'),
 (62425216, '_ZN8GameCore8Messages7Manager12QueueMessageEiRN13Serialization13MemoryIStreamE'),
 (62501096,
  '_ZN8GameCore2AI10Diplomatic13ProcessActionERKNS_9Diplomacy6Action8InstanceENS_11PlayerTypesE'),
 (62567136, '_ZN8GameCore2AI7Control7Manager6DoTurnEv'),
 (62583176,
  '_ZN8GameCore2AI10Diplomatic11ProcessDealENS_9Diplomacy4Deal14ProposalActionERKNS3_8InstanceEb'),
 (62829120, '_ZN8GameCore4Game8Instance6UpdateEv'),
 (62905200, '_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDEi')]

EXPECTED_PLT += [(0x3b4eb38, '_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS0_5ValueES5_'),
                 (0x3b9bbd0, '_ZN8GameCore9Reporting5Event4SendENS1_4TypeE'),
                 (0x3b69c98, '_ZN8GameCore9Reporting5Event4SendENS1_4TypeEii')]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=Path(__file__).resolve().parents[3]/'c6-assets-linux')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    game=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    host=Elf(args.assets/'Civ6',HOST_SHA)
    calls=[(game,0x291199d,0x1f3ccf0,'PendingStatement_drain_enters_actual_ProcessStatement_guard'),
           (host,0x31d1526,0x31d1550,'ControllerInitialize_subscribes_original_messages'),
           (host,0x31d14ef,0x31dd2a0,'ControllerInitialize_initializes_real_player_controllers'),
           (host,0x31d1428,0x31cd1e0,'ControllerUninitialize_releases_actual_app_cache'),
           (host,0x31d1430,0x31d16c0,'ControllerUninitialize_releases_its_context_binding'),
           (host,0x31d1438,0x31d1760,'ControllerUninitialize_unsubscribes_messages'),
           (game,0x2c7cd06,0x1f749c0,'GetCachePlayer_gets_original_valid_cache_player'),
           (game,0x2c7cd26,0x1e26df0,'GetCachePlayer_constructs_actual_cache_facade'),
           (game,0x28061de,0x1ee11f0,'RequestOperation_requires_cache_active'),
           (host,0x33af804,0x33d4150,'NetGameCoreMessage_requires_original_GameCoreActive'),
           (host,0x33d4151,0x31d4cf0,'GameCoreActive_gets_actual_controller'),
           (host,0x32b4d0d,0x33af670,'EngineSend_constructs_original_NetGameCoreMessage'),
           (host,0x32b4d1d,0x33bdbd0,'EngineSend_broadcasts_via_real_NetworkManager')]
    for elf,address,target,name in calls:elf.call(address,target)
    checks=[(host,0x31d4bd7,'e94485ffff','ControllerInitializeCache_delegates_to_AppCacheInitialize'),
            (host,0x31cd1b7,'be01000000ff5038','AppCacheInitialize_calls_GameInitializeCache_true'),
            (game,0x2c7c870,'c3','ContextDestroy_preserves_framework_owner'),
            (host,0x31d5090,'488b86500400004889074889f8c3','ControllerGetContext_native_hidden_pointer_return'),
            (host,0x31d1720,'8a470bc3','ControllerIsInitialized_native_field'),
            (game,0x2c8c611,'488b7f10e9d64b25ff','CachePlayerFacade_active_queries_original_cache'),
            (host,0x33bdbdc,'488b80c0000000','NetworkBroadcast_original_session_slot_c0')]
    for elf,address,expected,name in checks:
        value=bytes.fromhex(expected)
        if elf.read(address,len(value))!=value:raise ValueError('original contract mismatch: '+name)
    methods=[(0x2c7cd00,0x3b,'GetCachePlayer'),(0x2c8c5c0,0x1a,'Player_GetID'),(0x2c8c600,0x1a,'Player_IsTurnActive'),(0x2c8c700,0x1a,'Player_IsHuman')]
    relocations={}
    for line in subprocess.check_output(['readelf','-rW',str(args.assets/'libGameCore_XP2.so')],text=True).splitlines():
        fields=line.split()
        if len(fields)>4 and ('JUMP_SLOT' in line or 'GLOB_DAT' in line):relocations[int(fields[0],16)]=fields[4]
    for offset,symbol in EXPECTED_PLT:
        if relocations.get(offset)!=symbol:raise ValueError('original PLT relocation mismatch: '+symbol)
    if host.read(0x31dd2a0,7)!=bytes.fromhex('8937c6470400c3'):
        raise ValueError('original player controller Initialize is no longer bookkeeping-only')
    result=dict(schema='c6-linux-action-execution-abi-v1',host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
                checks_passed=True,player_controller_initialize='two native bookkeeping stores and return; no calls',
                guard_plt_relocations=[dict(offset=hex(o),symbol=n)for o,n in EXPECTED_PLT],
                guard_scope='Selected known policy/update PLT entries plus actual UnitOperation Execute vtable; unexpected entries terminate the process, never return a fabricated policy result.',
                direct_calls=[dict(call=hex(a),target=hex(t),contract=n)for _,a,t,n in calls],
                methods=[dict(address=hex(a),size=z,contract=n,sha256=hashlib.sha256(game.read(a,z)).hexdigest())for a,z,n in methods],
                instruction_contracts=[n for _,_,_,n in checks],native_execution=False,action_executed=False,
                scope='Actual cached/live player precondition observer only; network bytes are delivery leads, not executed acceptance.')
    from action_event_audit import audit, PROFILE
    result['reporting_observer_profile']=PROFILE
    result['event_observer_contract']=audit(args.assets)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,native_execution=False)))


if __name__=='__main__':main()
