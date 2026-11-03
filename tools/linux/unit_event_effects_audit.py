#!/usr/bin/env python3
"""Read-only exact Linux Unit event snapshot/effect contract; no engine run."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent / 'c6-ghidra/evidence/linux-rules/asl-launch-xp2-2/libGameCore_XP2.so'
HOST = ROOT.parent / 'c6-assets-linux/Civ6'

METHODS = [
    ('Unit.OnUpdate',0x27ff430,9), ('Unit.PublishUpdate',0x27ff860,114), ('Unit.Add',0x27ff2d0,222),
    ('Unit.FillVariants',0x2805100,1068), ('Unit.KeyedAlloc',0x27ffb20,74), ('CacheEventStreamID.Ctor',0x2707010,6),
    ('Upgrade.UnitType',0x2c48bd0,107), ('Upgrade.GoldCost',0x2c48c40,148),
    ('Upgrade.ResourceCost',0x2c49160,470), ('Upgrade.ResourceType',0x2c49340,62),
    ('Rules.AvailableUpgrade',0x2bf4080,120), ('Rules.NextUpgrade',0x2bf4100,141),
    ('Rules.UpgradePrerequisites',0x2bf4190,111), ('Rules.UpgradeCycle',0x2bf4200,89),
    ('Rules.UpgradeGoldCost',0x2bf4260,761), ('Player.Units.GetCost',0x2ba5960,20),
    ('Player.Units.ResourceCost',0x2ba5b10,75), ('Player.Units.BuildMapping',0x1fccc80,31),
    ('Player.HasTech',0x2b8ac60,79), ('Player.HasCivic',0x2a73e70,86),
    ('Unit.ReadyAutomate',0x2c28320,29), ('Unit.ReadySelect',0x2c27ff0,41),
    ('Unit.ReadyMove',0x2c28020,201), ('Unit.MemberReadyMove',0x2c28180,93),
    ('Unit.CanCyclePast',0x2c280f0,138), ('Unit.HasAdjacentMove',0x2c28200,278),
    ('Unit.NeedsAttention',0x2b5cb10,7), ('Unit.CanMove',0x2c27930,60),
    ('Unit.PlacementValid',0x2c424c0,1509), ('Combat.BestDefender',0x28c9e70,684), ('Combat.BetterDefender',0x2bdfdb0,61),
    ('Commands.HoldCycling',0x2d8bb00,296), ('Operations.HoldCycling',0x2da2b90,321),
    ('Unit.MaxDamage',0x2c27600,48),
]
HOST_METHODS = [('TypedMap.Ctor',0x395f070,124), ('TypedMap.ConstFind',0x395f3b0,90),
                ('TypedMap.ConstTreeFind',0x3962060,51), ('TypedMap.Copy',0x39612d0,1012)]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def audit(module, host_path, private_root):
    game, host = Elf(module,GAME_SHA), Elf(host_path,HOST_SHA)
    provider_path = ROOT / 'evidence/linux/reporting-provider-focused-2.json'
    provider_static_path = ROOT / 'evidence/linux/reporting-provider-focused-static.json'
    provider = json.loads(provider_path.read_text())
    require(provider['checks_passed'] and provider['gamecore_sha256']==GAME_SHA and provider['host_sha256']==HOST_SHA,
            'focused provider result identity differs')
    require(provider['focused_replay']['native_exit']==provider['focused_replay']['gdb_exit']==0,
            'focused native replay failed')
    observations_path = private_root / 'evidence/linux-reporting-provider/native-passive-1/observations.jsonl'
    observations_bytes = observations_path.read_bytes()
    rows = [json.loads(line) for line in observations_bytes.splitlines()]
    selected = [r for r in rows if r.get('stage')=='cache_delegate_tail' and r.get('trace_armed')
                and r.get('trace_phase')==5 and r['resolved_callback']['module_offset']=='0x2708590']
    require(len(selected)==7 and all(int(r['adjusted_receiver'],16)-int(r['game_base'],16)==0x27ff430 for r in selected),
            'actual seven Unit callbacks or static target provenance differs')
    expected_events = [0xe23b112d,0x383669f1,0x15c35863,0xe23b112d,0x3d4006f2,0xe23b112d,0x383669f1]
    require([r['event_hash'] for r in selected]==expected_events,'actual Unit callback event order differs')
    for call,target in [(0x27ff88d,0x1eed900),(0x27ff30b,0x1f25530),(0x27ff327,0x1e73580),
                        (0x27ff356,0x1e2afc0),(0x2805330,0x1ea9580),(0x2805353,0x1f48ae0),
                        (0x280536b,0x1f32b50),(0x28053d1,0x1f98dd0),(0x28053f5,0x1f6aeb0),
                        (0x2805419,0x1e356f0),(0x2c28132,0x1eaa170),
                        (0x2bf4093,0x2bf4100),(0x2bf40cc,0x2bf4200),(0x28ca077,0x1e932d0)]:
        game.call(call,target)
    require(game.read(0x27ff306,2).hex()=='31d2','actual Fill bool is not false')
    require(game.read(0x27ff881,9).hex()=='bf0200000031d231c9','actual update command/layer/flag differ')
    require(game.read(0x2c28137,4).hex()=='84c07518','HasAdjacentMove result checkpoint differs')
    require(game.read(0x2c28289,13).hex()=='ba0100000031c941b8ffffff7f','actual adjacent placement arguments differ')
    require(game.read(0x2c42767,3).hex()=='f6c130','placement alternate-location gate differs')
    require(game.read(0x27ffb4c,3).hex()=='894348','cache next-key state store differs')
    require(game.relocated_pointer(0x3afcd50+16+0xa8)==0x2d8bb00,'command HoldCycling binding differs')
    require(game.relocated_pointer(0x3aff870+16+0x110)==0x2da2b90,'operation HoldCycling binding differs')
    require(struct.unpack('<Q',host.read(0x53b3680+16+0x60,8))[0]==0x395f3b0,
            'max damage concrete const property lookup differs')
    host.call(0x395f3c8,0x3962060)
    # Reuse original before-owned field ranges, not Unit-ID pattern searches.
    initial = json.loads((ROOT/'evidence/linux/skip-turn-state-execute-failure-1.json').read_text())['typed_initial_state']
    before_path = private_root / 'evidence/linux-core-component/native-execute-3/before.bin'
    before = before_path.read_bytes()
    require(sha(before)==initial['original_core_sha256'],'original initial Unit stream identity differs')
    actual_fields = {}
    for field in initial['unit_fields']:
        if field['name'] in ('m_kProperties','m_bNeedsAttention','m_eTypeIndex','m_xMovesRemaining','m_bIsLevied'):
            lo,hi=field['native_range'];raw=before[lo:hi]
            actual_fields[field['name']]=dict(range=[lo,hi],raw_hex=raw.hex(),sha256=sha(raw))
    require(actual_fields['m_kProperties']['raw_hex']=='00000000','initial properties not empty')
    require(actual_fields['m_bNeedsAttention']['raw_hex']=='00','initial attention branch differs')
    fingerprints = [dict(name=n,image='game',rva=hex(a),bytes=z,sha256=sha(game.read(a,z))) for n,a,z in METHODS]
    fingerprints += [dict(name=n,image='host',rva=hex(a),bytes=z,sha256=sha(host.read(a,z))) for n,a,z in HOST_METHODS]
    # Record exact first-level dependencies as an inventory, not a purity proof.
    raw = subprocess.check_output(['objdump','-d','-C','--start-address=0x2805100','--stop-address=0x280552c',str(module)],text=True)
    calls = [dict(call_rva='0x'+a,plt_rva='0x'+b,symbol=n) for a,b,n in
             re.findall(r'^\s*([0-9a-f]+):.*?\bcall\s+([0-9a-f]+)\s+<(.+)@plt>',raw,re.M)]
    return dict(schema='c6-linux-unit-event-effects-v1',checks_passed=True,static_analysis=True,native_execution=False,
                gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,source_audit_sha256=sha(Path(__file__).read_bytes()),
                provider_static_sha256=sha(provider_static_path.read_bytes()),provider_focused_sha256=sha(provider_path.read_bytes()),
                observations_sha256=sha(observations_bytes),native_observation_reused=True,
                actual_unit_callbacks=[{k:r[k] for k in ('sequence','event_hash','adjusted_receiver','event_data','typed_map','thread','trace_phase')} for r in selected],
                methods=fingerprints,fill_direct_calls=calls,
                direct_store_scope='Fill destination snapshot and stack only; imported string assignment allocates/copies destination name',
                fill_bool_argument=False,fill_bool_controls_dependencies=False,
                cache_entry=dict(command=2,map_layer=0,native_entry_bytes=296,constants_offset=4,constants_bytes=12,
                                 variants_offset=16,variants_bytes=184,properties_offset=200,map_layer_offset=288,
                                 global_container_rva='0x3c03858',node_map_offset=32,next_key_offset=72,
                                 successful_allocation='Store node at old u32 key, advance next key wrapping32, update peak allocation metric',
                                 allocation_failure='UINT_MAX; PublishUpdate does not send a cache-update event',
                                 stream_type_hash='15d11e18',cache_id_layout='u32 stream type at0, u32 returned key at4',
                                 observed_successful_host_cache_updates=7,actual_cache_keys_captured=False,
                                 complete_cache_state_captured=False),
                snapshot_bits={str(i):name for i,name in enumerate(['CannotAttack','Embarked','DelayedDeath','Immobile',
                               'ReadyToAutomate','ReadyToSelect','ReadyToMove','HasPendingOperations','MovedIntoZOC','HiddenVisibility','IgnoresZOC'])},
                snapshot_flags_offset=180,snapshot_upper_bits_and_padding_classified=False,
                snapshot_layout=dict(location=[0,8],facing=8,owner=12,name=16,name_native_size_unproven=True,
                    scalar_i32_fields={str(24+4*i):n for i,n in enumerate(['Automation','Activity','MilitaryFormation','Domain','FormationClass',
                    'Damage','MaxDamage','Combat','RangedCombat','BombardCombat','AntiAirCombat','Range','FortifyTurns','MaxMoves',
                    'MovesRemainingRaw','MovementMovesRemainingRaw','FormationMovesRemainingRaw','FormationMaxMoves','AttacksRemaining',
                    'BuildCharges','DisasterCharges','ParkCharges','AirSlotModifier','SpreadCharges','ReligiousHealCharges','ActionCharges',
                    'ReligiousStrength','ReligionType','UnitsInFormation','UpgradeCost','UpgradeResourceCost','UpgradeResourceType',
                    'GreatWorkIndex','ArchaeologyHomeCity','SpyOperation','SpyOperationEndTurn','AirSlots','BarbarianTargetPlayer','HeroClass'])}),
                upgrade_query_contract=dict(selection='Ordered first valid definition upgrade, player build-unit mapping, tech/civic prerequisites, cycle-checked recursive available upgrade',
                                            gold_cost='Existing Unit cost vectors, game-speed/base/net-production parameters, formation multiplier, Unit and Treasury discounts; stack-only fixedpoint temporaries',
                                            resource_cost='Existing resource references/cost vector, formation and levied discounts, minimum and modifier; no resource deduction',
                                            direct_authoritative_store=False,direct_rng_or_reporting=False,
                                            actual_upgrade_results_captured=False),
                max_damage_property_lookup=dict(key='7d3fc711',host_vtable='0x53b3680',slot='0x60',target='0x395f3b0',
                                                effect='Const tree find; found variant int conversion or global fallback, no lookup insertion'),
                defender_comparator=dict(rva='0x2bdfdb0',call_rva='0x28ca077',
                    contract='Null current best accepts candidate; otherwise null candidate rejects; strict signed candidate.GetCombat(false)>best.GetCombat(false); ties preserve earlier candidate',
                    attacker_argument_ignored=True,direct_rng_reporting_or_source_store=False),
                actual_initial_fields=actual_fields,
                readiness=dict(automate='CanMove && !HasPendingOperations',
                               select='IsReadyToMove && !IsAutomated && !CanCyclePast',
                               move='Self member readiness; when formation count>=2 check each existing other member in native order',
                               member='Null member returns true; otherwise !delayedDeath && CanMove && !pending && activityAWAKE && automate=-1 && !busy',
                               cycle='Invalid location/delayedDeath/noFormation returns true; NeedsAttention or HasAdjacentMove returns false; otherwise command then operation HoldCycling',
                               adjacent_arguments=dict(flags=1,output_plot=None,max_distance=2147483647,callback_empty=True,defender_flags=264),
                               adjacency_alternate_search_reachable=False,
                               hold_cycling='Registered definition IsHoldCycling filter, then full manager CanStart with testVisible=false and output map',
                               actual_adjacent_result_captured=False,actual_hold_cycling_reachability_proven=False),
                focused_readiness_checkpoint=dict(rva='0x2c28137',bytes='84c07518',result='AL',unit='RBX',
                                                 fields=dict(id='u32[RBX+b8]',owner='i32[RBX+130]',activity='u32[RBX+290]',needs_attention='u8[RBX+738]'),
                                                 require='Armed phase5; FillVariants -> IsReadyToSelect -> CanCyclePast stack; genuine source Unit identity',
                                                 true_branch='Skip both HoldCycling fallbacks; CanCyclePast=false'),
                unresolved=['Actual HasAdjacentMove result and fallback HoldCycling CanStart/script/subtype effects',
                            'Placement visibility/diplomacy/container full transitive closure and actual outcomes; strength comparator itself closed',
                            'Per-callback property-map contents; before/after serialized properties are empty but intermediate maps were not captured',
                            'Snapshot computed values, output node keys, allocator/peak state and complete queued cache entries'],
                complete_callback_effects_proven=False,complete_rng_inventory=False,complete_live_state=False,
                full_host_admission=False,rust_live_action_admitted=False,gameplay_parity=False,
                historical_raw_save_load_failure_bytes=89198,
                limits='Exact target callback/cache output and query branches, not complete gameplay support. Static absence of direct stores/RNG in inspected bodies is not proof of complete transitive purity. Actual cache state and readiness branch still require evidence; no AI/planning-only exclusion, byte masking, native run or project mutation.')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module',type=Path,default=GAME)
    parser.add_argument('--host',type=Path,default=HOST)
    parser.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    result=audit(args.module,args.host,args.private_root)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,unit_callbacks=len(result['actual_unit_callbacks']),complete_callback_effects_proven=False)))


if __name__=='__main__':
    main()
