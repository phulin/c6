#!/usr/bin/env python3
"""Validate direct Rust raw-Core prefix parsing against original saved bytes; no original execution."""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from core_stream_index import Reader, index_core, read_stream, sha, METHODS
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
ROOT=Path(__file__).resolve().parents[2]

def normal(value):
    if isinstance(value,bytes):return value.hex()
    if isinstance(value,(tuple,list)):return [normal(x) for x in value]
    if isinstance(value,dict):return {k:normal(v) for k,v in value.items()}
    return value

def projection(data):
    r=Reader(data);r.version(5);metadata=r.variant_map();r.version(1);r.version(22);r.rules()
    processor=r.processor();scalars=r.static_array('i',3);properties=r.untyped_map()
    return normal(dict(metadata=metadata,processor=processor,game_scalars=scalars,game_properties=properties))

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    modules=dict(gamecore=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA),host=Elf(args.assets/'Civ6',HOST_SHA))
    for module,a,n,name,digest in METHODS:
        if sha(modules[module].read(a,n))!=digest:raise ValueError('original prefix method differs: '+name)
    symbols=subprocess.check_output(['nm','-C',str(args.assets/'libGameCore_XP2.so')],text=True)
    globals_=[(0x3c05cb8,'GameCore::Game::ms_bTurnComplete'),(0x3c05cb9,'GameCore::Game::ms_bAdvanceTurn'),(0x3c05cba,'GameCore::Game::ms_bNewGame')]
    for address,name in globals_:
        if not any(row.split()[:1]==[format(address,'016x')] and row.endswith(' '+name)for row in symbols.splitlines()):raise ValueError('Game bool global identity differs')
    paths=[ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'crates/c6-core/Cargo.toml',ROOT/'crates/c6-data/Cargo.toml',
        ROOT/'crates/c6-differential/Cargo.toml',ROOT/'crates/c6-differential/src/bin/c6-core-prefix-candidate.rs',
        *sorted((ROOT/'crates/c6-core/src').rglob('*.rs')),*sorted((ROOT/'crates/c6-data/src').rglob('*.rs'))]
    sources={str(x.relative_to(ROOT)):sha(x.read_bytes())for x in paths}
    subprocess.run(['cargo','build','--offline','-p','c6-differential','--bin','c6-core-prefix-candidate'],cwd=ROOT,check=True)
    if sources!={str(x.relative_to(ROOT)):sha(x.read_bytes())for x in paths}:raise ValueError('source changed during build')
    candidate=ROOT/'target/debug/c6-core-prefix-candidate'
    fixture_ids=['ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884',
                 '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8']
    cases=[];inputs=[]
    with tempfile.TemporaryDirectory(prefix='c6-core-prefix-')as directory:
        directory=Path(directory)
        def check(data,label):
            source=directory/(label+'.bin');output=directory/(label+'.map');source.write_bytes(data)
            actual=json.loads(subprocess.check_output([str(candidate),str(source),str(output)],text=True))
            indexed=index_core(data);expected=next(x for x in indexed['sections']if x['name']=='map')
            if (actual['map_start'],actual['map_end'],actual['consumed'])!=(expected['start'],expected['end'],expected['end']):raise ValueError('direct Rust structural cursor differs')
            if output.read_bytes()!=data[expected['start']:expected['end']] or actual['map_sha256']!=expected['sha256']:raise ValueError('direct Rust typed Map roundtrip differs')
            if not actual['tail_borrowed'] or actual['tail_bytes']!=len(data)-expected['end'] or actual['tail_sha256']!=sha(data[expected['end']:]):raise ValueError('unparsed tail was not preserved')
            expected_values=projection(data)
            if {k:actual[k]for k in expected_values}!=expected_values:raise ValueError('typed prefix projection differs')
            if actual['metadata_compatibility_checked'] or actual['complete_core_import'] or actual['gameplay_parity'] or actual['original_game_execution']:raise ValueError('partial scope overclaimed')
            return actual,indexed
        for identity in fixture_ids:
            data=read_stream(args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz'))
            if sha(data)!=identity:raise ValueError('original raw Core identity differs')
            actual,indexed=check(data,str(len(cases)))
            cases.append(dict(stream_sha256=identity,stream_bytes=len(data),map_start=actual['map_start'],map_end=actual['map_end'],
                consumed=actual['consumed'],map_bytes=actual['map_bytes'],map_sha256=actual['map_sha256'],
                width=actual['map_width'],height=actual['map_height'],plots=actual['plots'],
                tail_bytes=actual['tail_bytes'],tail_sha256=actual['tail_sha256'],tail_borrowed=True,
                typed_prefix_sha256=sha(json.dumps(projection(data),sort_keys=True,separators=(',',':')).encode())))
            inputs.append(data)
        data=inputs[0];indexed=index_core(data);spans={x['name']:x for x in indexed['sections']}
        metadata=spans['context_metadata'];prefix=bytearray(data[:metadata['end']])
        count=struct.unpack_from('<I',prefix,metadata['start'])[0]
        if count&0x80000000:raise ValueError('unexpected synthetic carrier metadata header')
        struct.pack_into('<I',prefix,metadata['start'],count+1)
        prefix.extend(struct.pack('<IHHIII',0xfffffffe,3,0,0,0,123))
        shifted,_=check(bytes(prefix)+data[metadata['end']:],'metadata-shift')
        if shifted['map_start']!=cases[0]['map_start']+20 or shifted['map_sha256']!=cases[0]['map_sha256']:raise ValueError('metadata length relocation differs')
        props=spans['game_properties']
        replacement=struct.pack('<III',1,0x12345678,5)+struct.pack('<IIIII',1,0x87654321,0,0,999)
        shifted,_=check(data[:props['start']]+replacement+data[props['end']:],'property-shift')
        delta=len(replacement)-props['bytes']
        if shifted['map_start']!=cases[0]['map_start']+delta or shifted['map_sha256']!=cases[0]['map_sha256']:raise ValueError('property length relocation differs')
    differences=sum(a!=b for a,b in zip(*inputs))+abs(len(inputs[0])-len(inputs[1]))
    report=dict(schema='c6-linux-rust-core-prefix-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
        methods=[dict(module=m,address=hex(a),size=n,name=name,sha256=digest)for m,a,n,name,digest in METHODS],
        game_bool_globals=[dict(address=hex(a),name=n)for a,n in globals_],candidate_sha256=sha(candidate.read_bytes()),candidate_sources=sources,
        audit_source_sha256=sha(Path(__file__).read_bytes()),independent_indexer_sha256=sha((ROOT/'tools/linux/core_stream_index.py').read_bytes()),
        fixtures=cases,source_only_relocation_cases=dict(metadata_extra_entry_bytes=20,properties_delta_bytes=delta,passed=True),
        raw_stream_byte_differences=differences,unknown_bytes_masked=0,
        direct_rust_byte0_parser=True,python_offsets_required_by_library=False,native_traces_required_by_library=False,
        next_unparsed_section='Game.Religion',metadata_compatibility_checked=False,rules_profile='Placement v1 only; unknown unframed providers refused',
        complete_core_import=False,complete_player_import=False,rng_reconstructed=False,gameplay_parity=False,
        original_game_execution=False,offline_rust_candidate_execution=True)
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,original_raw_streams=len(cases),consumed=cases[0]['consumed'],
        raw_stream_byte_differences=differences,source_only_relocations=2,original_game_execution=False)))
if __name__=='__main__':main()
