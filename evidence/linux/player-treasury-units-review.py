#!/usr/bin/env python3
"""One-archive Treasury/Units/Unit-prefix independent offline review; no game execution."""
import argparse
import gzip
import hashlib
import importlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'a4fb0ab8d9431f3e2d03f02f0b62f17df844535b'

def sha(data):
    return hashlib.sha256(data).hexdigest()

def run(args, **kwargs):
    return subprocess.check_output(args, **kwargs)

EXTRA_UNITS = r'''
 // Independent algebra: each dynamic element, live slot, free word and Unit
 // child's aggregate charge belongs to this envelope's exact item total.
 let mut algebra=v.records.entries.len()+v.records.free_links.len();
 algebra += v.current_unit_cost.len()+v.unit_production_multiplier_q8.len()
   +v.extra_unit_copy.len()+v.enable_wall_attack_whole_game_promotion_class.len()
   +v.unit_build_disabled.len()+v.add_on_district_founding.len()
   +v.add_on_district_founding.iter().map(|x|x.values.len()).sum::<usize>()
   +v.unit_free_pass_hashes.len()+v.unit_embarkation_pass_hashes.len()
   +v.extra_build_charges_by_unit.len()+v.promotion_selections.len()
   +v.proposed_promotions.len()+v.base_promotions.len()
   +v.unlimited_promotion_choice_unit_hashes.len()+v.current_unit_resource_cost.len()
   +v.unit_fuel_demands.len();
 let mut slot_offsets=Vec::new();let mut cursor=20usize;
 for entry in &v.records.entries {
  slot_offsets.push(cursor);cursor+=4;
  let (bytes,child_items)=entry.value.encode_with_items(limits).unwrap();
  let (decoded,consumed,charged)=unit::Unit::decode_prefix_with_items(&input[cursor..],unit::Limits{max_bytes:bytes.len(),max_items:child_items}).unwrap();
  assert_eq!((decoded,consumed,charged),(entry.value.clone(),bytes.len(),child_items));
  algebra+=child_items;cursor+=bytes.len();
 }
 assert_eq!(items,algebra);
 // Framing/slot decoder refusals, rather than encoder-only invalid values.
 for (offset,value) in [(0,21u32),(4,65536),(8,15),(16,u32::MAX)] {
  let mut bad=input.clone();bad[offset..offset+4].copy_from_slice(&value.to_le_bytes());
  assert!(Units::decode(&bad,limits).is_err());
 }
 if !slot_offsets.is_empty() {
  for index in [u32::MAX,v.records.capacity] {
   let mut bad=input.clone();let p=slot_offsets[0];bad[p..p+4].copy_from_slice(&index.to_le_bytes());
   assert_eq!(Units::decode(&bad,limits),Err(Error::InvalidContainer));
  }
 }
 if slot_offsets.len()>1 {
  let mut bad=input.clone();let from=slot_offsets[0];let to=slot_offsets[1];
  bad.copy_within(from..from+4,to);
  assert_eq!(Units::decode(&bad,limits),Err(Error::InvalidContainer));
 }
 // Keep exact allocator metadata and free-entry order; there is no extra count.
 let mut expected=Vec::new();
 for word in [v.records.current_id_bits,v.records.free_links.len() as u32,v.records.free_head,v.records.free_tail] {expected.extend(word.to_le_bytes());}
 for word in &v.records.free_links {expected.extend(word.to_le_bytes());}
 assert_eq!(&input[cursor..cursor+expected.len()],expected);
'''

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    assets=ROOT.parent/'c6-assets-linux'; private=ROOT.parent/'c6-ghidra'
    packed=run(['git','archive',SOURCE],cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-treasury-units-review-') as temp:
        temp=Path(temp); source=temp/'source';source.mkdir()
        subprocess.run(['tar','-x','-C',str(source)],input=packed,check=True)
        sys.path.insert(0,str(source/'tools/linux'))
        audits={n:importlib.import_module(n+'_audit') for n in ['player_treasury','player_units','unit_prefix']}
        from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
        from player_districts_audit import FIXTURES
        from player_religion_audit import relocation_symbols
        game=Elf(assets/'libGameCore_XP2.so',GAME_SHA);Elf(assets/'Civ6',HOST_SHA)
        relocations=relocation_symbols(game)
        symbols={}
        for row in run(['nm','-S','-C',str(assets/'libGameCore_XP2.so')],text=True).splitlines():
            words=row.split(' ',3)
            if len(words)==4 and words[2] in ('T','W'):symbols[words[3]]=(int(words[0],16),int(words[1],16))
        static={}
        for name in ['player_treasury','player_units']:
            mod=audits[name]
            for address,size,symbol,digest in mod.METHODS:
                assert symbols[symbol]==(address,size)
                assert sha(game.read(address,size))==digest
            for site,plt,body,kind in mod.BINDINGS:
                code=game.read(site,5)
                assert code[0]==(0xe8 if kind=='call' else 0xe9)
                assert site+5+struct.unpack_from('<i',code,1)[0]==plt
                stub=game.read(plt,6);assert stub[:2]==b'\xff\x25'
                assert relocations[plt+6+struct.unpack_from('<i',stub,2)[0]]==body
            names=['Treasury'] if name=='player_treasury' else ['Units','RandomPromotionSelection','ProposedPromotion','BasePromotion']
            cmd=['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so')]
            for n in names:cmd+=['-ex',"ptype /o 'GameCore::Player::"+n+"'"]
            dwarf=run(cmd,text=True);assert sha(dwarf.encode())==mod.DWARF_SHA
            static[name]={'methods':len(mod.METHODS),'bindings':len(mod.BINDINGS),'method_manifest_sha256':sha(json.dumps(mod.METHODS,separators=(',',':')).encode()),'dwarf_sha256':sha(dwarf.encode())}
        for address,version in [(0x2f6937c,10),(0x2f69c10,22),(0x2f69c14,65537)]:assert struct.unpack('<I',game.read(address,4))[0]==version
        for address,target in [(0x2b4f0e6,0x1f66d50),(0x2b4f0f9,0x1f35f50)]:
            game.call(address,target)
        tests={}
        for prefix in ['player_treasury::tests','player_units::tests','unit::']:
            result=run(['cargo','test','--offline','-p','c6-core','--lib',prefix],cwd=source,text=True)
            tests[prefix]=next(x for x in result.splitlines() if x.startswith('test result:'))
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        harnesses={
            'treasury':audits['player_treasury'].HARNESS.replace('pub use c6_core::unit;\n#[path="MODULE"] pub mod player_treasury;','pub use c6_core::{unit,player_treasury};'),
            'units':audits['player_units'].HARNESS.replace('pub use c6_core::unit;\n#[path="MODULE"] pub mod player_units;','pub use c6_core::{unit,player_units};').replace(' std::fs::write(&args[2],out).unwrap();',EXTRA_UNITS+'\n std::fs::write(&args[2],out).unwrap();'),
            'prefix':audits['unit_prefix'].ADAPTER,
        }
        candidates={}
        for name,text in harnesses.items():
            assert 'MODULE' not in text
            (temp/(name+'.rs')).write_text(text)
            binary=temp/name
            subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(temp/(name+'.rs')),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(binary)],check=True)
            candidates[name]={'harness_sha256':sha(text.encode()),'candidate_sha256':sha(binary.read_bytes())}
        fixtures=[]
        for identity,trace_name,trace_sha in FIXTURES:
            data=gzip.decompress((private/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());assert sha(data)==identity
            trace=gzip.decompress((private/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(trace)==trace_sha
            rows=[json.loads(x) for x in trace.splitlines()]
            report={'stream_sha256':identity,'trace_sha256':trace_sha,'components':{}}
            for mode,owner in [('treasury','Treasury'),('units','Units')]:
                cuts=sorted([x for x in rows if x.get('event')=='owner' and x.get('save')==1 and x.get('owner')=='GameCore::Player::'+owner],key=lambda x:x['start'])
                assert len(cuts)==43
                observations=[];digests=[];metadata=[];nested=0
                for cut in cuts:
                    payload=data[cut['start']:cut['end']];(temp/'in').write_bytes(payload)
                    result=run([str(temp/mode),str(temp/'in'),str(temp/'out')],text=True).splitlines()
                    assert (temp/'out').read_bytes()==payload
                    words=[int(x) for x in result[0].split()];assert words[0]==len(payload)
                    digests.append(sha(payload));observations.append(words)
                    if mode=='units':
                        children=sorted([x for x in rows if x.get('event')=='owner' and x.get('save')==1 and x.get('parent')==cut['call'] and x.get('owner')=='GameCore::Unit::Instance'],key=lambda x:x['start'])
                        emitted=[[int(x) for x in line.split()[1:]] for line in result[1:]]
                        assert [x[:2] for x in emitted]==[[x['start']-cut['start'],x['end']-cut['start']] for x in children]
                        assert len(emitted)==words[3]
                        nested+=len(emitted)
                        cursor=20
                        slots=[]
                        for child in emitted:
                            assert child[0]==cursor+4
                            assert struct.unpack_from('<i',payload,cursor)[0]==child[2]
                            slots.append(child[2]);cursor=child[1]
                        assert len(slots)==len(set(slots)) and all(0<=s<words[2] for s in slots)
                        current,count,head,tail=struct.unpack_from('<IIII',payload,cursor)
                        assert [current,head,tail,count]==words[4:8]
                        metadata.append(sha(payload[cursor:cursor+16+4*count]))
                if mode=='units':assert nested==1262
                report['components'][mode]={'records':43,'bytes':sum(x[0] for x in observations),'aggregate_items':sum(x[1] for x in observations),'first_record_observation':observations[0],'nested_units':nested,'ordered_record_digests_sha256':sha(json.dumps(digests,separators=(',',':')).encode()),'typed_observations_sha256':sha(json.dumps(observations,separators=(',',':')).encode())}
                if metadata:report['components'][mode]['ordered_allocator_metadata_digests_sha256']=sha(json.dumps(metadata,separators=(',',':')).encode())
            (temp/'core').write_bytes(data)
            children=sorted([x for x in rows if x.get('event')=='owner' and x.get('save')==1 and x.get('owner')=='GameCore::Unit::Instance'],key=lambda x:x['start'])
            assert len(children)==1262
            (temp/'bounds').write_text(''.join(f"{x['start']} {x['end']}\n" for x in children))
            report['unit_prefix']=json.loads(run([str(temp/'prefix'),str(temp/'core'),str(temp/'bounds')],text=True))
            fixtures.append(report)
        subprocess.run(['python3',str(source/'tools/linux/unit_codec_check.py'),'--private-root',str(private),'--iterations','1','--output',str(temp/'strict.json')],cwd=source,check=True,stdout=subprocess.DEVNULL)
        strict=json.loads((temp/'strict.json').read_text())
        files=['Cargo.toml','Cargo.lock','crates/c6-core/Cargo.toml','crates/c6-core/src/lib.rs','crates/c6-core/src/player_treasury.rs','crates/c6-core/src/player_units.rs','crates/c6-core/src/unit.rs','crates/c6-core/src/unit/state.rs','crates/c6-core/src/unit/variants.rs']
        sources={path:sha((source/path).read_bytes()) for path in files}
        result={'schema':'c6-linux-treasury-units-independent-review-v1','source_commit':SOURCE,'archive_sha256':sha(packed),'review_script_sha256':sha(Path(__file__).read_bytes()),'source_sha256':sources,'candidates':candidates,'gamecore_sha256':GAME_SHA,'host_sha256':HOST_SHA,'static_contracts':static,'tests':tests,'fixtures':fixtures,'strict_unit_codec_check':strict,'checks_passed':True,'resolved_findings':{'signed_free_pass_count':'a4fb0ab adds explicit signed i32 decode/encode count gates before allocation; huge-Limits negative regression passes','allocation_claim':'1184f1c qualifies envelope-owned fallible allocation versus inherited nested Unit allocation policy'},'new_independent_checks':['envelope item algebra includes nested charges','encoded duplicate/negative/out-of-capacity slot decoder rejection','table version/index-bit/hostile-live-count decoder rejection','typed allocator metadata equals exact uncounted free-word wire sequence','independently traced nested Unit boundaries and slot positions'],'allocation_caveat':'Envelope-owned vectors preflight items, minimum bytes, available input and addressable layouts and reserve fallibly. Nested Unit generic vectors retain their prior Vec::with_capacity policy after item and byte-minimum checks; no new fallible allocation or total heap-byte promise.','original_execution':False,'private_writes':False,'scope':'bounded Treasury v10, Player.Units v22 recycled envelope and Unit v28 prefix wrappers; expanded Core prefix separately reviewed','full_43_players':False,'full_gameplay_state':False,'all_gameplay_rng':False,'actions':False,'turns':False,'raw_save_load_difference_bytes':89198,'unknown_bytes_masked':False}
        args.output.write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps({'checks_passed':True,'source':SOURCE,'treasury_records':86,'units_envelopes':86,'unit_prefixes':2524}))

if __name__=='__main__':main()
