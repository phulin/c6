#!/usr/bin/env python3
"""Gate exact original Linux cache dispatch and genuine network send boundaries."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=Path(__file__).resolve().parents[3]/'c6-assets-linux')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    game=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    host=Elf(args.assets/'Civ6',HOST_SHA)
    calls=[(game,0x2c7cd06,0x1f749c0,'GetCachePlayer_gets_original_valid_cache_player'),
           (game,0x2c7cd26,0x1e26df0,'GetCachePlayer_constructs_actual_cache_facade'),
           (game,0x28061de,0x1ee11f0,'RequestOperation_requires_cache_active'),
           (host,0x33af804,0x33d4150,'NetGameCoreMessage_requires_original_GameCoreActive'),
           (host,0x33d4151,0x31d4cf0,'GameCoreActive_gets_actual_controller'),
           (host,0x32b4d0d,0x33af670,'EngineSend_constructs_original_NetGameCoreMessage'),
           (host,0x32b4d1d,0x33bdbd0,'EngineSend_broadcasts_via_real_NetworkManager')]
    for elf,address,target,name in calls:elf.call(address,target)
    checks=[(host,0x31d5090,'488b86500400004889074889f8c3','ControllerGetContext_native_hidden_pointer_return'),
            (host,0x31d1720,'8a470bc3','ControllerIsInitialized_native_field'),
            (game,0x2c8c611,'488b7f10e9d64b25ff','CachePlayerFacade_active_queries_original_cache'),
            (host,0x33bdbdc,'488b80c0000000','NetworkBroadcast_original_session_slot_c0')]
    for elf,address,expected,name in checks:
        value=bytes.fromhex(expected)
        if elf.read(address,len(value))!=value:raise ValueError('original contract mismatch: '+name)
    methods=[(0x2c7cd00,0x3b,'GetCachePlayer'),(0x2c8c5c0,0x1a,'Player_GetID'),(0x2c8c600,0x1a,'Player_IsTurnActive'),(0x2c8c700,0x1a,'Player_IsHuman')]
    result=dict(schema='c6-linux-action-execution-abi-v1',host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
                checks_passed=True,direct_calls=[dict(call=hex(a),target=hex(t),contract=n)for _,a,t,n in calls],
                methods=[dict(address=hex(a),size=z,contract=n,sha256=hashlib.sha256(game.read(a,z)).hexdigest())for a,z,n in methods],
                instruction_contracts=[n for _,_,_,n in checks],native_execution=False,action_executed=False,
                scope='Actual cached/live player precondition observer only; network bytes are delivery leads, not executed acceptance.')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,native_execution=False)))


if __name__=='__main__':main()
