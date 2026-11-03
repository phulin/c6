#!/usr/bin/env python3
"""Focused Trade v35 immutable-source review; static ELF reads and offline payloads only."""
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

ROOT=Path(__file__).resolve().parents[2]
SOURCE='e7f2aab03db7473a4e9c5b5d29b9ab7c42678114'
PREFIX='player outgoing_route_capacity notified_outgoing_route_capacity extra_route_range improved_route_level immediate_trading_post_count gain_tiles_en_route_radius anyone_plunder_favor anyone_trade_to_favor extra_land_range_refuel extra_water_range_refuel free_tech_boost_first_trading_post_each_civ free_civic_boost_first_trading_post_each_civ trade_route_capacity_increase_on_meeting forbid_land_route_domestic forbid_land_route_international'.split()
FIRST='flat_yield_bonus flat_yield_bonus_for_domestic flat_yield_bonus_for_city_state flat_yield_bonus_for_international flat_yield_bonus_for_intercontinental_domestic flat_yield_bonus_for_intercontinental_international flat_yield_bonus_for_cultural_dominance yield_bonus_for_districts_city_state origin_yield_for_ally_route destination_yield_for_ally_route origin_yield_for_suzerain_route destination_yield_for_suzerain_route flat_yield_bonus_to_others'.split()
SECOND='per_specialty_district_yield_bonus_for_domestic yield_bonus_per_trading_post_in_foreign_city yield_bonus_per_trading_post_in_own_city yield_bonus_per_path_tile yield_bonus_per_follower international_yield_modifier origin_yield_modifier destination_yield_modifier'.split()
IMPROVEMENT='flat_origin_yield_per_improvement_at_destination flat_destination_yield_per_improvement_at_destination flat_international_yield_per_improvement_at_origin'.split()
TERRAIN='flat_origin_yield_per_terrain_for_domestic flat_destination_yield_per_terrain_for_domestic flat_origin_yield_per_terrain_for_international flat_destination_yield_per_terrain_for_international'.split()
SCHEMA=[(x,'i32')for x in PREFIX]+[(x,'bools')for x in ['civ_received_civic_boost_first_trading_post','civ_received_tech_boost_first_trading_post']]+[(x,'table')for x in FIRST]+[('techs_civics_per_yield','i32')]+[(x,'table')for x in SECOND]+[('flat_yield_bonus_per_player','players')]+[(x,'nested')for x in IMPROVEMENT]+[('per_specialty_district_yield_bonus_for_international','table')]+[(x,'nested')for x in TERRAIN]+[(x,'i32')for x in ['international_majors_trade_disabled_count','international_minors_trade_disabled_count','domestic_trade_disabled_count']]

def sha(data):return hashlib.sha256(data).hexdigest()
def run(cmd,**kw):return subprocess.check_output(cmd,**kw)

def parse(data):
    pos=0;items=0;tokens=[];counts=[];bools=[];shapes=[]
    def word(signed=False):
        nonlocal pos
        value=struct.unpack_from('<i'if signed else '<I',data,pos)[0];pos+=4;tokens.append(value);return value
    def table():
        nonlocal items
        counts.append(pos);n=word();items+=n
        for _ in range(n):word();word(True)
        return n
    assert word()==35
    for name,kind in SCHEMA:
        if kind=='i32':word(True)
        elif kind=='bools':
            counts.append(pos);n=word();items+=n;shapes.append([name,n])
            for _ in range(n):
                bools.append(pos);v=data[pos];pos+=1;assert v in [0,1];tokens.append(v)
        elif kind=='table':shapes.append([name,table()])
        elif kind=='players':
            items+=64;shapes.append([name,[table()for _ in range(64)]])
        elif kind=='nested':
            counts.append(pos);n=word();items+=n;inner=[]
            for _ in range(n):word();inner.append(table())
            shapes.append([name,inner])
    assert pos==len(data)
    return tokens,items,counts,bools,shapes

def synthetic():
    out=bytearray(struct.pack('<I',35));serial=0
    def word(v):out.extend(struct.pack('<I',v&0xffffffff))
    def table(n=3):
        word(n)
        for i in range(n):word([0xffffffff,0x80000001,0x80000001][i]);word([-2147483648,2147483647,-257][i])
    for _,kind in SCHEMA:
        serial+=1
        if kind=='i32':word(-257*serial)
        elif kind=='bools':word(3);out.extend([1,0,1])
        elif kind=='table':table()
        elif kind=='players':
            for i in range(64):table(i%3)
        elif kind=='nested':
            word(2)
            for i in range(2):word(0x87654321);table(2 if i==0 else 0)
    return bytes(out)

HARNESS=r'''use c6_core::{player_trade::{Trade,Error,YieldTable,NestedYieldTable},unit::Limits};
fn table(v:&YieldTable,t:&mut Vec<i64>){t.push(v.len() as i64);for(h,q)in v{t.push(*h as i64);t.push(*q as i64);}}
fn nested(v:&NestedYieldTable,t:&mut Vec<i64>){t.push(v.len() as i64);for(h,q)in v{t.push(*h as i64);table(q,t);}}
fn main(){
 let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();
 if a.len()>3 {assert!(Trade::decode(&b,Limits{max_bytes:usize::MAX,max_items:usize::MAX}).is_err());return;}
 let loose=Limits{max_bytes:64*1024*1024,max_items:4_000_000};
 let(v,n,items)=Trade::decode_prefix_with_items(&b,loose).unwrap();assert_eq!(n,b.len());
 let exact=Limits{max_bytes:n,max_items:items};
 assert_eq!(Trade::decode(&b,exact).unwrap(),v);
 let(out,charged)=v.encode_with_items(exact).unwrap();assert_eq!((&out,charged),(&b,items));
 for k in 0..n{assert!(Trade::decode(&b[..k],exact).is_err());}
 for low in [Limits{max_bytes:n-1,..exact},Limits{max_items:items-1,..exact}]{assert!(Trade::decode(&b,low).is_err());assert!(v.encode(low).is_err());}
 let mut tail=b.clone();tail.extend_from_slice(&vec![0xa5;1024*1024]);
 assert_eq!(Trade::decode_prefix_with_items(&tail,exact).unwrap(),(v.clone(),n,items));
 assert_eq!(Trade::decode_prefix(&tail,exact).unwrap(),(v.clone(),n));
 assert_eq!(Trade::decode(&tail,loose),Err(Error::Trailing));
 let mut t=vec![35i64];
 PROJECTION
 std::fs::write(&a[2],out).unwrap();println!("{} {}",n,items);println!("{:?}",t);
}
'''

def harness():
    lines=[]
    for name,kind in SCHEMA:
        if kind=='i32':lines.append('t.push(v.'+name+' as i64);')
        elif kind=='table':lines.append('table(&v.'+name+',&mut t);')
        elif kind=='nested':lines.append('nested(&v.'+name+',&mut t);')
        elif kind=='players':lines.append('for value in &v.'+name+'{table(value,&mut t);}')
        elif kind=='bools':lines.append('t.push(v.'+name+'.len() as i64);for value in &v.'+name+'{t.push(i64::from(*value));}')
    return HARNESS.replace('PROJECTION','\n'.join(lines))

def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=Path,required=True);args=ap.parse_args()
    assets=ROOT.parent/'c6-assets-linux';private=ROOT.parent/'c6-ghidra'
    packed=run(['git','archive',SOURCE],cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-trade-review-')as temp:
        temp=Path(temp);source=temp/'source';source.mkdir();subprocess.run(['tar','-x','-C',str(source)],input=packed,check=True)
        sys.path.insert(0,str(source/'tools/linux'))
        audit=importlib.import_module('player_trade_audit')
        from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
        from player_districts_audit import FIXTURES
        from player_religion_audit import relocation_symbols
        game=Elf(assets/'libGameCore_XP2.so',GAME_SHA);Elf(assets/'Civ6',HOST_SHA);relocs=relocation_symbols(game)
        symbols={}
        for row in run(['nm','-S','-C',str(assets/'libGameCore_XP2.so')],text=True).splitlines():
            parts=row.split(' ',3)
            if len(parts)==4 and parts[2]in('T','W'):symbols[parts[3]]=(int(parts[0],16),int(parts[1],16))
        for address,size,name,digest in audit.METHODS:assert symbols[name]==(address,size)and sha(game.read(address,size))==digest
        for site,plt,body,kind in audit.BINDINGS:
            code=game.read(site,5);assert code[0]==(0xe8 if kind=='call'else 0xe9)and site+5+struct.unpack_from('<i',code,1)[0]==plt
            stub=game.read(plt,6);assert stub[:2]==b'\xff\x25'and relocs.get(plt+6+struct.unpack_from('<i',stub,2)[0])==body
        assert struct.unpack('<I',game.read(0x2f68bbc,4))[0]==35
        dwarf=run(['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so'),'-ex',"ptype /o 'GameCore::Player::Trade'"],text=True);assert sha(dwarf.encode())==audit.DWARF_SHA
        game.call(0x2b4f11f,0x1e397e0);game.call(0x2b4f132,0x1e524a0)
        tests=run(['cargo','test','--offline','-p','c6-core','--lib','player_trade::tests'],cwd=source,text=True)
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        text=harness();(temp/'candidate.rs').write_text(text);candidate=temp/'candidate'
        subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(temp/'candidate.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(candidate)],check=True)
        def compare(payload):
            tokens,items,counts,bools,shapes=parse(payload);(temp/'in').write_bytes(payload)
            lines=run([str(candidate),str(temp/'in'),str(temp/'out')],text=True).splitlines()
            assert [int(x)for x in lines[0].split()]==[len(payload),items]
            assert json.loads(lines[1])==tokens and (temp/'out').read_bytes()==payload
            return items,counts,bools,shapes,sha(lines[1].encode())
        specimen=synthetic();synthetic_items,counts,bools,_,_=compare(specimen)
        negatives=0
        for offset,value in [(0,34),(0,36),(0,0xffffffff)]+[(x,0xffffffff)for x in counts]:
            bad=bytearray(specimen);struct.pack_into('<I',bad,offset,value);(temp/'in').write_bytes(bad)
            subprocess.run([str(candidate),str(temp/'in'),str(temp/'out'),'reject'],check=True);negatives+=1
        for offset in bools:
            bad=bytearray(specimen);bad[offset]=2;(temp/'in').write_bytes(bad)
            subprocess.run([str(candidate),str(temp/'in'),str(temp/'out'),'reject'],check=True);negatives+=1
        fixtures=[]
        for identity,trace_name,trace_sha in FIXTURES:
            data=gzip.decompress((private/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());assert sha(data)==identity
            trace=gzip.decompress((private/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(trace)==trace_sha
            cuts=sorted([x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner'and x.get('save')==1 and x.get('owner')=='GameCore::Player::Trade'],key=lambda x:x['start']);assert len(cuts)==43
            digests=[];semantic=[];observations=[];first_shapes=None
            for cut in cuts:
                payload=data[cut['start']:cut['end']];items,_,_,shapes,typed_sha=compare(payload)
                if first_shapes is None:first_shapes=shapes
                digests.append(sha(payload));semantic.append(typed_sha);observations.append([len(payload),items])
            fixtures.append({'stream_sha256':identity,'trace_sha256':trace_sha,'records':len(cuts),'bytes':sum(x[0]for x in observations),'items':sum(x[1]for x in observations),'byte_item_shapes':sorted({tuple(x)for x in observations}),'field_shapes_first_record':first_shapes,'ordered_record_digests_sha256':sha(json.dumps(digests,separators=(',',':')).encode()),'ordered_semantic_tokens_digests_sha256':sha(json.dumps(semantic,separators=(',',':')).encode())})
        files=['Cargo.toml','Cargo.lock','crates/c6-core/Cargo.toml','crates/c6-core/src/lib.rs','crates/c6-core/src/player_trade.rs','tools/linux/player_trade_audit.py']
        result={'schema':'c6-linux-player-trade-independent-review-v1','checks_passed':True,'source_commit':SOURCE,'archive_sha256':sha(packed),'source_sha256':{p:sha((source/p).read_bytes())for p in files},'candidate_sha256':sha(candidate.read_bytes()),'harness_sha256':sha(text.encode()),'review_script_sha256':sha(Path(__file__).read_bytes()),'gamecore_sha256':GAME_SHA,'host_sha256':HOST_SHA,'static_contracts':{'methods':len(audit.METHODS),'bindings':len(audit.BINDINGS),'method_manifest_sha256':sha(json.dumps(audit.METHODS,separators=(',',':')).encode()),'bindings_manifest_sha256':sha(json.dumps(audit.BINDINGS,separators=(',',':')).encode()),'dwarf_sha256':audit.DWARF_SHA},'source_tests':next(x for x in tests.splitlines()if x.startswith('test result:')),'synthetic':{'bytes':len(specimen),'items':synthetic_items,'hostile_mutations_with_huge_limits':negatives,'description':'distinct signed scalars; true/false booleans; duplicate/high-bit hashes; Q8 extremes; empty/nonempty nested tables; distinct per-player table lengths'},'fixtures':fixtures,'independent_projection':'Python wire grammar and item algebra compared against every Rust typed scalar/hash/Q8/bool/count token in source-derived field order','each_record_checks':['exact decode/reencode bytes/items','every truncated prefix rejected','one-short byte/item caps rejected for decode and encode','one MiB unparsed tail accepted under exact prefix limits and rejected by strict decode'],'allocation_policy':'fallible typed reservations after count/item/wire-minimum preflight; encoder measures complete bytes/items before output reserve; no aggregate heap-byte cap','original_execution':False,'private_writes':False,'native_reader_calls_initialize_reset':True,'native_read_initialize_reset_reproduced':False,'definition_remapping':False,'callbacks':False,'runtime_lookup_authority':False,'strings_present':False,'packed_component_owner_in_wire':False,'native_initialize_constructs_archive_component_id':True,'native_archive_owner_reconstruction_reproduced':False,'full_43_players':False,'full_gameplay_state':False,'all_gameplay_rng':False,'actions':False,'turns':False,'raw_save_load_difference_bytes':89198,'unknown_bytes_masked':False,'next_component':'GreatPeoplePoints','whole_prefix_acceptance':'separate frozen producer/composition review'}
        args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'checks_passed':True,'source':SOURCE,'records':86,'synthetic_hostile_mutations':negatives}))
if __name__=='__main__':main()
