#!/usr/bin/env python3
"""Pinned read-only Linux InfluenceMap saved-wire audit; no original execution."""
import argparse, gzip, json, struct, subprocess, tempfile
from pathlib import Path
from player_wmds_audit import Elf, GAME_SHA, FIXTURES, relocation_symbols, sha
ROOT = Path(__file__).resolve().parents[2]
METHODS = [[34868736, 72, 'a0a16314da6af675604dcd79e64135e1b6924929848d3a91ed40a08418f252d3'], [34868816, 81, 'dd1bd2f89a8d60da3a5aa2a714d61346c2f9557ed7667d7ec3bc592dd66ccc89'], [34875584, 21, '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'], [34875616, 21, '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'], [34875648, 38, '9317f3b8d7cdaec6dd55cd5ca25099cea13776973b9521efc3d96b24a166d8a8'], [34881040, 15, 'cce1a98027284c3cd481129fb2193fb7dcb873689d228b9bce55ddab17187407'], [34881152, 15, '51028c9426b423994d554cbb2edfc2f28b62d83d5c3a5943ca507adfd9ee0243'], [34883760, 14, '94efe331d8d54051f25aee89e2de7d66c597054ddab67b8079442e15e4676b81'], [34883776, 125, 'adbd9cfe2061e51f00b5f2e7aa809f16559be398179045e787d026b619899b54'], [34883984, 14, 'fcb9926377812eb599dd6355ba5034296d824db7b711b95bf20e7d6805cf1b15'], [34884032, 75, '38d2bb574d57e50a7813133fc053c34e1eb48935a8feef09c582fbe2057ce199'], [34884160, 91, '05bd91a3da50b51c2da8cf726a8aa18252c8c30f974c22f838b58c3cfe5693e5'], [34884256, 71, '70e1ee76fc41d96600a9704802d1b95a28728d5f3b58c6932c30c1de2633617f'], [34884368, 46, '44a2b6d8fd018ebccf434a9691f9d4d11c5db17b78aaf801969ee4299cd29a62'], [34884416, 8, 'c602c74e302860c60e4a55195fa9fa4383eef425f84bf44d4b11f62f63c491db'], [34884432, 14, 'b3a4ba0fe53a97a6f965db0493686a4226c6c361fae059c0f8d8b9ad2ae39fc0'], [34884448, 14, 'f2d3e22bc439750e47398663378c04dc03e443a86397bff40334966cebdde5f3'], [34868912, 296, '6fcf89243ae3482eb7eef52d5a95298b38346b7c78d34904120aa70d01a038e5'], [34869440, 204, '91aa744c18b39e619574678dcc247bfadf4c8a91bc1246db9970b2f15481f89c'], [45412912, 61, '7f42f7822613464c9dec5e80b4a2473a36238d51a7bb6da68b912b0028b1ff55'], [45413543, 12, '5db1fa27fae787f751784f6e982007274be6eb6b51ac182a1b872f73416232cf'], [45415649, 65, '251936512500dccab6060237507a7cb54937f2d7ad6713af7f02aa62e0d71390']]
BINDINGS = [[34868759, 31588976, 33341776, 'call'], [34868774, 31707584, 34875584, 'call'], [34868792, 31707584, 34875584, 'call'], [34868831, 31815744, 33341888, 'call'], [34868846, 33013040, 34875616, 'call'], [34868881, 31667200, 34875648, 'call'], [34875666, 31624912, 34879584, 'call'], [34875676, 33013040, 34875616, 'call'], [34881050, 32780256, 34883760, 'jmp'], [34881162, 32826832, 34884160, 'jmp'], [34883764, 31668016, 34883776, 'call'], [34883793, 31923152, 34878592, 'call'], [34883814, 31815744, 33341888, 'call'], [34883831, 31777024, 34883904, 'call'], [34883859, 33243776, 34883936, 'call'], [34883867, 32830064, 34884000, 'call'], [34883878, 32697920, 34883984, 'call'], [34883988, 32064384, 34884032, 'call'], [34884063, 31471216, 33348384, 'call'], [34884180, 31834240, 34879872, 'call'], [34884191, 32286608, 34884336, 'call'], [34884210, 31698736, 34884368, 'call'], [34884231, 32826272, 34884256, 'call'], [34884294, 32469024, 34884416, 'call'], [34884386, 33216624, 34876064, 'call'], [34884403, 31588976, 33341776, 'call'], [34884419, 32522480, 34884432, 'jmp'], [34884436, 32458256, 34884448, 'call'], [34868938, 31932752, 33434432, 'call'], [34868943, 31484512, 42092400, 'call'], [34868962, 32672144, 34875696, 'call'], [34868972, 32588624, 33342032, 'call'], [34868998, 31966592, 47521872, 'call'], [34869015, 31932752, 33434432, 'call'], [34869038, 32043552, 34875728, 'call'], [34869048, 32588624, 33342032, 'call'], [34869065, 31932752, 33434432, 'call'], [34869085, 32043552, 34875728, 'call'], [34869095, 32588624, 33342032, 'call'], [34869120, 32588624, 33342032, 'call'], [34869133, 32964624, 34875808, 'call'], [34869148, 32588624, 33342032, 'call'], [34869174, 32157296, 0, 'call'], [34869182, 32992576, 0, 'call'], [34869195, 32588624, 33342032, 'call'], [34869203, 32992576, 0, 'call'], [34869455, 32969888, 34869344, 'call'], [34869463, 33228816, 33341760, 'call'], [34869483, 32311008, 33441552, 'call'], [34869491, 32454096, 33446960, 'call'], [34869502, 32301232, 0, 'call'], [34869514, 32998480, 33342240, 'call'], [34869522, 32017184, 33437952, 'call'], [34869559, 32124448, 33611088, 'call'], [34869577, 31921104, 47521984, 'call'], [34869595, 32693792, 33611152, 'call'], [34869631, 32693792, 33611152, 'call'], [34869639, 32992576, 0, 'call'], [45412922, 32186880, 33673056, 'call'], [45412937, 32807584, 34868736, 'call'], [45412952, 31703840, 35353952, 'call'], [45412967, 32994672, 33355024, 'call'], [45413550, 33098448, 34869440, 'call'], [45415659, 32319408, 33673376, 'call'], [45415684, 31905968, 34868816, 'call'], [45415694, 33211440, 34869808, 'call'], [45415709, 31674336, 35354032, 'call']]
HARNESS = r'''use c6_core::{unit::Limits,player_ai_influence_map::{InfluenceMap,Error}};
fn main() {
 let args:Vec<_>=std::env::args().collect();let all=std::fs::read(&args[1]).unwrap();let mut at=0;let mut records=0;
 while at<all.len() { let n=u32::from_le_bytes(all[at..at+4].try_into().unwrap()) as usize;at+=4;let raw=&all[at..at+n];at+=n;
 let (v,used,items)=InfluenceMap::decode_prefix_with_items(raw,Limits{max_bytes:n,max_items:n}).unwrap();assert_eq!(used,n);
 let exact=Limits{max_bytes:n,max_items:items};assert_eq!(InfluenceMap::decode(raw,exact).unwrap(),v);assert_eq!(v.encode_with_items(exact).unwrap(),(raw.to_vec(),items));
 for end in 0..n {assert!(InfluenceMap::decode(&raw[..end],exact).is_err());}
 let short=Limits{max_bytes:n-1,..exact};assert_eq!(InfluenceMap::decode(raw,short),Err(Error::ResourceLimit));assert_eq!(v.encode(short),Err(Error::ResourceLimit));
 if items>0 {let short=Limits{max_items:items-1,..exact};assert_eq!(InfluenceMap::decode(raw,short),Err(Error::ResourceLimit));assert_eq!(v.encode(short),Err(Error::ResourceLimit));}
 let mut tail=raw.to_vec();tail.extend_from_slice(&[91;128]);assert_eq!(InfluenceMap::decode(&tail,exact),Err(Error::Trailing));assert_eq!(InfluenceMap::decode_prefix_with_items(&tail,exact).unwrap(),(v,n,items));records+=1;
 } assert_eq!(records,86);println!("{records}");
}'''
def candidate(pin, records):
    pin=subprocess.check_output(['git','rev-parse',pin+'^{commit}'],cwd=ROOT,text=True).strip()
    archive=subprocess.check_output(['git','archive',pin],cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-influence-') as tmp:
        tmp=Path(tmp);source=tmp/'source';source.mkdir()
        subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        if (source/'tools/linux/player_ai_influence_map_audit.py').read_bytes()!=Path(__file__).read_bytes():raise ValueError('driver must be committed with candidate')
        subprocess.run(['cargo','test','--offline','-p','c6-core','player_ai_influence_map'],cwd=source,check=True)
        subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        (tmp/'main.rs').write_text(HARNESS)
        subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'main.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(tmp/'candidate')],check=True)
        (tmp/'records').write_bytes(records)
        count=int(subprocess.check_output([str(tmp/'candidate'),str(tmp/'records')],text=True))
        return dict(source_commit=pin,archive_sha256=sha(archive),module_sha256=sha((source/'crates/c6-core/src/player_ai_influence_map.rs').read_bytes()),harness_sha256=sha(HARNESS.encode()),candidate_sha256=sha((tmp/'candidate').read_bytes()),records=count,exported_archive_cargo_tests=True,exported_archive_clippy=True,every_truncation=True,exact_byte_item_limits=True,one_short_byte_item_limits=True,strict_trailing=True,shared_tail_prefix=True)
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--source-commit');p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux');p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra');a=p.parse_args()
    e=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA);rel=relocation_symbols(e)
    for start,n,digest in METHODS:
        if sha(e.read(start,n))!=digest:raise ValueError('method mismatch')
    for site,plt,body,kind in BINDINGS:
        code=e.read(site,5)
        if code[0] != (0xe8 if kind=='call' else 0xe9) or site+5+struct.unpack_from('<i',code,1)[0]!=plt:raise ValueError('edge mismatch')
        stub=e.read(plt,6)
        if stub[:2]!=b'\xff\x25' or rel[plt+6+struct.unpack_from('<i',stub,2)[0]]!=body:raise ValueError('binding mismatch')
    virtuals=[(0x3a1d8a8,0x2143e10),(0x3a1d8c0,0x2143e80)]
    for slot,body in virtuals:
        if e.relocated_pointer(slot)!=body:raise ValueError('virtual mismatch')
    dwarf=subprocess.check_output(['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so'),'-ex',"ptype /o 'GameCore::AI::InfluenceMap'"],text=True)
    for token in ['Types::Vector<short','m_influence','m_religiousInfluence','m_player','m_unitDiedHandle']:
        if token not in dwarf:raise ValueError('DWARF mismatch')
    reports=[];corpus=bytearray()
    for identity,name,trace_sha in FIXTURES:
        data=gzip.decompress((a.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());trace=gzip.decompress((a.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/name).read_bytes())
        if sha(data)!=identity or sha(trace)!=trace_sha:raise ValueError('fixture mismatch')
        owners={x['call']:x for x in map(json.loads,trace.splitlines()) if x.get('event')=='owner' and x.get('save')==1}
        if any('InfluenceMap' in x['owner'] for x in owners.values()):raise ValueError('hook inventory changed')
        records=[]
        for control in owners.values():
            if control['owner']!='GameCore::AI::Control::Manager':continue
            parent=owners[control['parent']]
            planning=[x for x in owners.values() if x['parent']==control['parent'] and x['owner']=='GameCore::AI::Planning::Manager']
            if parent['owner']!='GameCore::Player::Instance' or len(planning)!=1:raise ValueError('parent mismatch')
            start=control['end'];end=planning[0]['start'];raw=data[start:end]
            version,n=struct.unpack_from('<II',raw);pos=8+2*n;m=struct.unpack_from('<I',raw,pos)[0];pos+=4+2*m
            if version!=2 or pos!=len(raw):raise ValueError('extent mismatch')
            influence=struct.unpack_from('<'+str(n)+'h',raw,8);religious=struct.unpack_from('<'+str(m)+'h',raw,12+2*n)
            rebuilt=struct.pack('<II',2,n)+struct.pack('<'+str(n)+'h',*influence)+struct.pack('<I',m)+struct.pack('<'+str(m)+'h',*religious)
            if rebuilt!=raw:raise ValueError('wire mismatch')
            pv,player=struct.unpack_from('<Ii',data,parent['start'])
            if pv!=47 or struct.unpack_from('<Ii',data,planning[0]['start'])!=(1,player):raise ValueError('Player/Planning mismatch')
            corpus.extend(struct.pack('<I',len(raw))+raw)
            records.append(dict(player=player,start=start,end=end,influence_items=n,religious_items=m,sha256=sha(raw)))
        if len(records)!=43:raise ValueError('record count mismatch')
        reports.append(dict(core_sha256=identity,trace_sha256=trace_sha,records=records))
    report=dict(schema='c6-linux-player-ai-influence-map-v1',checks_passed=True,gamecore_sha256=GAME_SHA,audit_source_sha256=sha(Path(__file__).read_bytes()),methods=METHODS,bindings=BINDINGS,virtuals=virtuals,dwarf_sha256=sha(dwarf.encode()),wire=['u32 version=2','u32 influence_count','i16 influence[influence_count]','u32 religious_count','i16 religious[religious_count]'],fixtures=reports,actual_influence_owner_hook=False,validation_cuts='Control.end to Planning.start; same Player parent, production parser has no offsets',native_player_read_initializes_before_version=True,native_initialization_reproduced=False,native_read_fixups_reproduced=False,planning_only_exclusion_proven=False,gameplay_rng_in_scope=True,original_execution=False,private_writes=False,unknown_bytes_masked=0,recorded_raw_save_load_difference_bytes=89198,full_player=False,world_import=False,actions=False,turns=False)
    if a.source_commit:report['candidate']=candidate(a.source_commit,corpus)
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(checks_passed=True,records=86)))
if __name__=='__main__':main()
