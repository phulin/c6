#!/usr/bin/env python3
"""Exact original Linux placement chain and immutable serialized occupancy replay.

No original code executes. Synthetic count cases have no native boundary fixtures.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA
from map_state_audit import Reader, INITIAL
ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    ('CUnit.PostLoad',0x2c12d00,19,'5a9347cbc48de5fb33deba3be4527993a6cf6a7692be18090a23adfb5f9e44c7'),
    ('Definition.DomainHash',0x1fe5870,4,'77205cdc9a7bd9bf1f7e6ff1fdda57017736a90fa0063ff3b0d23ffeea318cab'),
    ('Definition.FormationClassHash',0x2429d80,4,'8e1eb1e8ee76f9d9fd9091b8e14772902a0c006899df4ef599bf389b61b64d35'),
    ('Definition.TradeRoute',0x200ddf0,12,'bed3d2de9ff4f59141f8dc17112d86315d74d824c8c36c128d8b5a508da6705b'),
    ('Definition.Spy',0x20b4490,11,'6228fa6dc481a7a976ff0613ee4528990a6e90b314edf685c0d6ba4ecd7c9d79'),
    ('Definition.Combat',0x1fe4c20,4,'464ad404f57fdf46d34aca82663f5924e733344590a2d76cd2ff81e199b23f83'),
    ('Definition.Range',0x21c1000,4,'3d9fac76e2d74cce46d6bf0966941e3fca0c64f4eefd03aebbd8b827766428c1'),
    ('Religion.Strength',0x213ce90,4,'4628fb538250bfc88b327da2acefeb54924dfb050ac98dcea105347fb07d6367'),
    ('Religion.Type',0x213ce80,4,'123844b3d139f0b75833926319edcaa7dacf7529e5ca1b5d6afa59806bc75758'),
    ('Player.Alive',0x2003cb0,7,'3002fe1bd5eaf1a0e1fa05cede54870c8999f7e31479aef1a8dfdcc00460032d'),
    ('Player.Team',0x2b31af0,15,'91b3ec5a8477dc9a77ac40ae060e9bd8eef5f4fc57051363a323183f8eea15f9'),

    ('Formation.FriendlyCount',0x2c433f0,184,'4906e83b115c6514f33383cb3c172897b42ca41380facd5fd9f11b46adad1048'),
    ('Unit.FriendlyCount',0x2c432a0,322,'c78099a9a470bf120e00faa147c00660d78abdb0603819d5020fde18450899e4'),
    ('Placement.SameType',0x2c43240,70,'4da58f4f1b79d3a455a366ce8a7a6c493e4b7cfe21e4e36ce0219a8fa3728d78'),
    ('Manager.SameType',0x26af8c0,299,'425865181a79f2cf7559cfd57a38376127df98e8eb55ad093b37a7098f656361'),
    ('Manager.SameFormationClass',0x26afa40,48,'2bdadce876eabd69b4665685b1ed241944cfa1132bb73f2644df68e37ba7da4c'),
    ('Unit.IsCargo',0x28e04f0,3,'251447ee91a9067dcd6ab96703133f617565974cd6c4819021760c4688c91abf'),
    ('Combat.Rivals',0x2be1250,189,'47381b5ff133ac7f2ac8e8590fb0ad8af2bbf05f27f11973a5c006a7fb2ae239'),
    ('Rules.SameTeam',0x26ae6c0,117,'5555adc86b843917bbc26f353eeb9c5aec4cedac99549ded0848dc14feb12ee5'),
    ('Diplomacy.AtWar',0x2aa4bb0,91,'95598b7b9fe28eedb4e31712dd005eb94d2f1eb389c1db8d5f8d210b30196122'),
    ('Relations.AtWar',0x2983ba0,31,'7feb0befd035aa90f70df5bdfcc528ef2cf2ea7ae223e19d1e1a93f5fcfb5661'),
    ('Relations.WarLookup',0x29885d0,52,'4bd31b9475fc8fc65094489a8177b41a9f320506e9f8e1e302934d106e44d2e3'),
    ('Unit.LocationLayer',0x2c1c790,136,'e45e60d49000512d47997c7d10c6eaaa573ad63882c6d3955f98b94c5c0e9efe'),
    ('Map.UnitLayer',0x2a4e990,160,'0c6978a4a1b77b224a8d03d274849472ae7129181c30dfbfd74325d375b20e21'),
    ('Grid.Contains',0x2a4e170,43,'e15baf01568fcf6d53511fc1d01e9c1ca52e273524ec165ec9d94867264aabfc'),
    ('Vector.Find',0x2a4ede0,77,'7c24aa478239b5cc0dd1cfc85c19584d754c3e16c6857a442e8744aa2977e94f'),
    ('ComponentID.Equal',0x2006720,34,'565a5536704f1c2eda96270c96177cd8dea251e96a1c60e185c70c2ab5c49f80'),
    ('ComponentID.Write',0x2c9ac60,95,'bd29bea4faf93d9428367ceceb01e636f498a5a209f6d011e3a69664a921fdff'),
    ('ComponentID.Read',0x2c9acc0,76,'7021044d9575aaff00e08945164f5c91ba17cd8de1eedafd8c6781e3c69060be'),
    ('UnitID.Ctor',0x20bcfc0,10,'d5e35bf64814cd22ffa58215a152329f4f4647676e2d5f856506b87e5dee4af5'),
    ('Units.GetUnit',0x2c15810,93,'39ba0dbaa5365853cdddb366d64b750bb340c938b431d9c10fef5c5a57e29db9'),
    ('ComponentID.Valid',0x208e9e0,21,'6b7ab1333e17d6c73ef1445fc18db84c1ff4ecc644a31115c46534e071f1d771'),
    ('Unit.DefinitionInit',0x2c1c6e0,87,'d3e05bf90be350eb8f482e319ca09f5b3b1f3cc9ef950a7182844c422c512714'),
    ('Unit.PostLoad',0x2c1c850,11,'5498b517d76fe29a543e1d4169543884d999e1ebf85494e7c05463b47a2d28d8'),
    ('UnitTypes.Read',0x2d4ec80,58,'d2f6a44260938ca04b9c2055201efa60f4b0ad7cd539089934501f6a40e139be'),
    ('UnitTypes.HashToIndex',0x1ffc120,34,'26ec484a59dc05ceb242dc6d0e5d36fef9e8cfc39abd042b918b16ea8db67122'),
    ('Unit.Read',0x2c231f0,3011,'e39f2c7b9a358527bbf3ceed7ff41d9c5f5dccfcf6cb3f5802bafbc0d0f2d202'),
    ('UnitType.Get',0x1fe4ef0,7,'110c1e76bddb14dc8632d99b080ee2b717407fc8733d1e38f768c4b8c6d66a73'),
    ('Plot.Valid',0x1fe2340,22,'8aa1013da5016cf18bf96ce8be0b0c3b75c0e03c3681e9ea9349e97db360631f'),
    ('Grid.GetXY',0x2a4e030,10,'090091f5ddac7731eb9caa0e9085d7ca6803d830485fdb29067eeea22366ed40'),
    ('Grid.Get',0x2a4e010,24,'fc2a2c196cfb04ed6165cab3f5aefff32ed0224cdee525fea1ff6154ad47f47e'),
    ('Map.GetUnits',0x2a4e6c0,118,'032dbeb462279ba6d24b0cf9eab171f396017aa6a403814b73cdaf74ec5e9011'),
    ('Config.IsHuman',0x28ec230,14,'a98088b62436c799ea309acd50c5ee744fd7ad1846e1be89d12429e6ffb9015a'),
    ('Parameters.PlotUnitLimit',0x2807b10,7,'573d7da7612dc26e9b2df470fd3b62bc359f9c02e5fe296620a41ea8b25d72f4'),
]

def sha(data):
    return hashlib.sha256(data).hexdigest()
def require(value, reason):
    if not value:
        raise ValueError(reason)
def audit(a):
    elf = Elf(a.module, GAME_SHA)
    fixture_path = ROOT/'evidence/linux/game-load-native-attempt-1.json'
    fixture_raw = fixture_path.read_bytes()
    require(sha(fixture_raw) == '8c03bcdad2e44e9e0f9c7d5b08d011d0265b42a93582b5eaff2c3759d1380bd5', 'original fixture report identity')
    fixture = json.loads(fixture_raw)
    configuration = fixture['configuration']
    config_sha = sha(json.dumps(configuration,sort_keys=True,separators=(',',':')).encode())
    require(config_sha == fixture['configuration_sha256'] == '446b6e6afc2542d244f4335ef513392e3e75f4fafefdb0d0e724afcaac8f202c', 'original configuration identity')
    require(configuration['assets']['libGameCore_XP2.so'] == GAME_SHA, 'configuration module')
    require(fixture['game_saves']['save']['sha256'] == INITIAL, 'original Save identity')
    for name, address, size, digest in METHODS:
        require(sha(elf.read(address, size)) == digest, 'method mismatch: '+name)
    require(elf.relocated_pointer(0x3ae5f70+16+0x28) == 0x2c12d00, 'actual Unit post-load binding')
    require(elf.relocated_pointer(0x3ab3130+16+0xc0) == 0x26af8c0, 'same-type manager binding')
    require(elf.relocated_pointer(0x3ab3130+16+0xd0) == 0x26afa40, 'formation-class manager binding')
    for address, target in [(0x2c9ac7f,0x1f040f0),(0x2c9ac96,0x1f040f0),
                            (0x2c15835,0x1ed3bb0),(0x2a4ee17,0x1f814a0),
                            (0x2c2322f,0x1f2a8a0),(0x2c23242,0x1f905f0),
                            (0x2d4eca6,0x1e27fd0),(0x2c1c6f2,0x1eb18e0),
                            (0x2c43321,0x1fb8b98),(0x2c43343,0x1eb91d0),
                            (0x2c43399,0x1e723a0),(0x2c43481,0x1e35a90)]:
        elf.call(address,target)
    require(elf.read(0x28e04f0,3).hex()=='31c0c3','literal cargo body')
    trace_path=a.private_root/'evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz'
    trace=gzip.decompress(trace_path.read_bytes())
    require(sha(trace)=='528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae','trace identity')
    data=gzip.decompress((a.private_root/'evidence/linux-game-load/canonical'/f'{INITIAL}.bin.gz').read_bytes())
    require(sha(data)==INITIAL,'Core stream identity')
    rows=[json.loads(x) for x in trace.splitlines()]
    rows=[r for r in rows if r.get('event')=='owner' and r.get('save')==1]
    map_row=next(r for r in rows if r['owner']=='GameCore::Map::Instance')
    r=Reader(data,map_row['start'],map_row['end']); decoded=r.map()
    require(r.pos==r.end,'Map decode boundary')
    units=[r for r in rows if r['owner']=='GameCore::Unit::Instance']
    formations=[r for r in rows if r['owner']=='GameCore::Formation::Instance']
    selected=None
    for row in units:
        record=data[row['start']:row['end']]
        require(struct.unpack_from('<I',record)[0]==28,'Unit version')
        uid=struct.unpack_from('<I',record,4)[0]
        owner=struct.unpack_from('<i',record,40)[0]
        fid=struct.unpack_from('<I',record,49)[0]
        x,y=struct.unpack_from('<ii',record,28)
        for frow in formations:
            frecord=data[frow['start']:frow['end']]
            if struct.unpack_from('<I',frecord,4)[0]==fid and struct.unpack_from('<i',frecord,16)[0]==owner:
                index=(y*decoded['units']['width']+x)&0xffffffff
                packed=((owner&65535)|(1<<16))
                candidates=[]
                for layer in decoded['units']['layers']:
                    if index>=layer['maximum']: continue
                    cell=next((u for plot,u in layer['cells'] if plot==index),[])
                    if (packed,uid) in cell:
                        candidates=cell;break
                if candidates and (selected is None or len(candidates)>len(selected[2])):
                    selected=(row,frow,candidates,owner,uid)
                break
    require(selected is not None,'no genuine unit/formation/occupancy match')
    row,frow,expected,owner,uid=selected
    subprocess.run(['cargo','build','--release','-p','c6-core','--example','unit_placement'],cwd=ROOT,check=True)
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version=1'],cwd=ROOT))
    binary=Path(metadata['target_directory'])/'release/examples/unit_placement'
    with tempfile.TemporaryDirectory(prefix='c6-placement-') as temporary:
        d=Path(temporary)
        for name,record in [('map',map_row),('unit',row),('formation',frow)]:
            (d/name).write_bytes(data[record['start']:record['end']])
        output=subprocess.check_output([str(binary),str(d/'map'),str(d/'unit'),str(d/'formation')],text=True)
    results=[json.loads(x) for x in output.splitlines()]
    actual=next(x for x in results if x['kind']=='native_occupancy')
    require(actual['references']==[list(x) for x in expected],'native-order occupancy projection mismatch')
    require(actual['owner']==owner and actual['unit']==uid,'selected identity mismatch')
    paths=['crates/c6-core/src/unit_placement.rs','crates/c6-core/src/world.rs','crates/c6-core/src/map/containers.rs',
           'crates/c6-core/src/skip_turn.rs','crates/c6-core/examples/unit_placement.rs','tools/linux/unit_placement_audit.py']
    return dict(schema='c6-linux-unit-placement-v1',checks_passed=True,gamecore_sha256=GAME_SHA,
        original_execution_performed=False,methods=[dict(name=n,address=hex(v),size=s,sha256=h) for n,v,s,h in METHODS],
        source_sha256={p:sha((ROOT/p).read_bytes()) for p in paths},candidate_sha256=sha(binary.read_bytes()),
        original_stream_sha256=INITIAL,original_trace_sha256=sha(trace),
        configuration_sha256=config_sha,configuration_report_sha256=sha(fixture_raw),
        original_rules_snapshot_requested=configuration['native_rules_snapshot_requested'],
        original_record_sha256={n:sha(data[x['start']:x['end']]) for n,x in [('map',map_row),('unit',row),('formation',frow)]},
        native_serialized_occupancy_references_compared=len(expected),packed_component_type_preserved=True,
        native_count_result_observed=False,synthetic_checks=results[-1],
        complete_native_player_control_and_placement_definitions_available=False,
        aggregate_count_oracle_input=False,war_type_rivalry_cargo_oracle_inputs=False,
        native_definition_pointer_reconstructed='Unit hash-to-index then original InitializePostLoad/InitializeDefinition Find/CollectionIndex; null on missing definition',
        full_manager_activation_predicate_closed=False,engine_gameplay_apply_supported=False,
        full_state_action_turns_supported=False,historical_raw_save_load_failure_bytes=89198,
        gaps=['Actual configuration/player scalars, native ordered definition properties and complete relation values need importer/export provenance',
              'No native count boundary fixtures; no complete action legality/callback/aftermath/RNG parity'])
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--module',type=Path,default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();report=audit(a)
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods=len(METHODS),references_compared=report['native_serialized_occupancy_references_compared'],synthetic_checks=report['synthetic_checks'])))
if __name__=='__main__':main()
