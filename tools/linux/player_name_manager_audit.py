#!/usr/bin/env python3
"""Hash-gated static contracts and offline original NameManager wire validation. No original execution."""
import argparse
import gzip
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from core_stream_index import sha
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from player_districts_audit import FIXTURES
ROOT = Path(__file__).resolve().parents[2]
METHODS = [(45436592, 104, 'Player.NameManager writer', '761e296e96b83ef4842abd3f039c2c19e87e13274eafcd855a143dda0d445c34'), (45436704, 285, 'Player.NameManager reader', '8f4af8af607a82e78fe248d858e1b352e0cbac6133c323a1aabe2420a329dfc4'), (45436992, 171, 'PadNameUsageCount', '691e5d2b0ac7d81f98bff97670101a5feb78d22be40ca74cf5fd04b71dda6c20'), (47515936, 74, 'WriteArray(short)', 'fce7582bd65f8473492f5e2a2490fd1fc3ef63749b835ffad4034b6ac75dfa25'), (47515264, 211, 'ReadArray(short)', 'a8def904159d3bf90136d12887f3c1bdc2eaced48c4c2988a900d947b954ef02'), (45411840, 1679, 'Player writer', 'cbeacc8a90c5fa8853e90d6971174683990e8bbee2c91c00a9be1bb2023d8593')]
HARNESS = r'''
pub use c6_core::unit;
#[path="MODULE"] pub mod player_name_manager;
use player_name_manager::NameManager;
fn main() {
 let args:Vec<_>=std::env::args().collect();
 let input=std::fs::read(&args[1]).unwrap();
 let limits=unit::Limits{max_bytes:input.len(),max_items:1_000_000};
 let (value,n,items)=NameManager::decode_prefix_with_items(&input,limits).unwrap();
 assert_eq!(n,input.len());
 let (out,write_items)=value.encode_with_items(unit::Limits{max_bytes:n,max_items:items}).unwrap();
 assert_eq!(items,write_items);
 assert_eq!(NameManager::decode(&input,limits).unwrap(),value);
 std::fs::write(&args[2],out).unwrap();
 println!("{} {}",n,items);
}
'''

def jump_slot(elf, address):
    start=struct.unpack_from('<Q',elf.raw,40)[0]
    width,count=struct.unpack_from('<HH',elf.raw,58)
    sections=[struct.unpack_from('<IIQQQQIIQQ',elf.raw,start+i*width)for i in range(count)]
    for section in sections:
        if section[1]!=4:continue
        symbols=sections[section[6]]
        for pos in range(section[4],section[4]+section[5],section[9]):
            offset,info,addend=struct.unpack_from('<QQq',elf.raw,pos)
            if offset!=address:continue
            if info & 0xffffffff != 7:raise ValueError('expected JUMP_SLOT')
            return struct.unpack_from('<Q',elf.raw,symbols[4]+(info>>32)*symbols[9]+8)[0]+addend
    raise ValueError('missing JUMP_SLOT')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    game=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    Elf(args.assets/'Civ6',HOST_SHA)
    for address,size,name,digest in METHODS:
        if sha(game.read(address,size))!=digest:raise ValueError('method differs: '+name)
    if struct.unpack('<I',game.read(0x2f65e6c,4))[0]!=2:raise ValueError('current version differs')
    # Resolve each PLT jump through the exact ELF relocation; names alone are insufficient.
    bindings=[(0x2b4f061,0x1f0d360,0x2b54eb0),(0x2b54f06,0x1f06500,0x2d50920),
              (0x2b5500a,0x1e93ef0,0x2d50680),(0x2b55012,0x1e93f10,0x2b55040)]
    for call,plt,target in bindings:
        game.call(call,plt)
        raw=game.read(plt,6)
        if raw[:2]!=b'\xff\x25':raise ValueError('PLT encoding differs')
        got=plt+6+struct.unpack('<i',raw[2:])[0]
        if jump_slot(game,got)!=target:raise ValueError('concrete provider differs')
    dwarf=subprocess.check_output(['gdb','-nx','-batch',str(args.assets/'libGameCore_XP2.so'),
        '-ex',"ptype /o 'GameCore::Player::NameManager'"],text=True)
    for fragment in ['m_aiCitizenNameUsageCount','Vector<short,','/*    176','m_pPlayer']:
        if fragment not in dwarf:raise ValueError('matching DWARF differs')
    subprocess.run(['cargo','build','--offline','-p','c6-core'],cwd=ROOT,check=True)
    source_paths=[ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'crates/c6-core/Cargo.toml',*sorted((ROOT/'crates/c6-core/src').rglob('*.rs'))]
    sources={str(path.relative_to(ROOT)):sha(path.read_bytes())for path in source_paths}
    reports=[]
    with tempfile.TemporaryDirectory(prefix='c6-name-manager-')as temporary:
        tmp=Path(temporary)
        harness=HARNESS.replace('MODULE',str(ROOT/'crates/c6-core/src/player_name_manager.rs'))
        (tmp/'candidate.rs').write_text(harness)
        command=['rustc','--edition=2021',str(tmp/'candidate.rs'),'--extern','c6_core='+str(ROOT/'target/debug/libc6_core.rlib'),'-L','dependency='+str(ROOT/'target/debug/deps')]
        subprocess.run(command+['-o',str(tmp/'candidate')],check=True)
        subprocess.run(command+['--test','-o',str(tmp/'tests')],check=True)
        subprocess.run([str(tmp/'tests')],check=True)
        candidate_sha=sha((tmp/'candidate').read_bytes())
        for identity,trace_name,trace_sha in FIXTURES:
            data=gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
            if sha(data)!=identity:raise ValueError('original Core differs')
            trace=gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
            if sha(trace)!=trace_sha:raise ValueError('owner trace differs')
            rows=[json.loads(line)for line in trace.splitlines()]
            rows=[row for row in rows if row.get('event')=='owner'and row.get('save')==1]
            districts=sorted([row for row in rows if row['owner']=='GameCore::Player::Districts'],key=lambda row:row['start'])
            cultures=sorted([row for row in rows if row['owner']=='GameCore::Player::Culture'],key=lambda row:row['start'])
            if len(districts)!=len(cultures):raise ValueError('roster cut count differs')
            observations=[]
            for district,culture in zip(districts,cultures):
                start,end=district['end'],culture['start']
                payload=data[start:end]  # Validation-only adjacent native owners, never production offsets.
                (tmp/'input').write_bytes(payload)
                result=subprocess.check_output([str(tmp/'candidate'),str(tmp/'input'),str(tmp/'output')],text=True)
                used,items=map(int,result.split())
                if (tmp/'output').read_bytes()!=payload:raise ValueError('record roundtrip differs')
                version,count,array_count=struct.unpack_from('<III',payload)
                if (version,count,array_count)!=(2,items,items)or used!=12+items*2:raise ValueError('typed counts differ')
                observations.append(dict(start=start,end=end,bytes=used,items=items,sha256=sha(payload),
                    nonzero_counts=sum(value!=0 for value in struct.unpack_from('<'+'h'*items,payload,12))))
            reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(observations),
                record_bytes=sum(row['bytes']for row in observations),observations=observations))
    if sources!={str(path.relative_to(ROOT)):sha(path.read_bytes())for path in source_paths}:raise ValueError('candidate sources changed during audit')
    report=dict(schema='c6-linux-player-name-manager-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
        methods=[dict(address=hex(a),size=n,name=name,sha256=digest)for a,n,name,digest in METHODS],
        version=dict(address='0x2f65e6c',value=2),concrete_bindings=[dict(call=hex(a),plt=hex(p),target=hex(t))for a,p,t in bindings],
        dwarf_sha256=sha(dwarf.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),candidate_sources=sources,
        candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),fixtures=reports,
        complete_current_player_name_manager_wire=True,historical_v1=False,native_read_padding=False,
        original_execution=False,private_writes=False,full_player=False,world_import=False,gameplay_state=False,
        actions=False,turns=False,unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(output=str(args.output),records=[row['records']for row in reports])))
if __name__=='__main__':main()
