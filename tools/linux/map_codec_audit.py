#!/usr/bin/env python3
"""Read-only Rust Map4/Plot6 full-value comparison against original Linux data.

All owner spans come from original traces; inner structure is independently
parsed from the exact original schema. No native execution/private upload.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

from map_state_audit import Reader, XP2, INITIAL, RELOADED
from unit_codec_check import TRACES
ROOT=Path(__file__).resolve().parents[2]
OWNER_NAMES={'GameCore::Map::Player::Visibility::Manager':'visibility','GameCore::Map::Player::Visibility::Instance':'visibility_instance','GameCore::Map::Area::Instance':'area','GameCore::Map::Landmass::Instance':'landmass','GameCore::Map::Region::Instance':'region','GameCore::Map::Continent::Instance':'continent','GameCore::Map::UnitContainer':'units'}
SECTIONS={'header','visibility','areas','landmasses','regions','continents','features','plots','units','components'}

def sha(b):return hashlib.sha256(b).hexdigest()
def require(v,msg):
    if not v:raise ValueError(msg)
def leaves(v):
    if isinstance(v,dict):return sum(leaves(x) for x in v.values())
    if isinstance(v,list):return sum(leaves(x) for x in v)
    return 1

def freeze(binary,target):
    data=binary.read_bytes();identity=sha(data);directory=target/'linux/map-codec-candidates'/identity;directory.mkdir(parents=True,exist_ok=True);output=directory/'candidate'
    with tempfile.NamedTemporaryFile(dir=directory,delete=False) as stream:stream.write(data);temporary=Path(stream.name)
    try:
        temporary.chmod(0o500)
        try:os.link(temporary,output)
        except FileExistsError:pass
    finally:temporary.unlink()
    require(sha(output.read_bytes())==identity,'frozen candidate changed');return output,identity

def run(candidate,section,directory,iterations):
    source,output,values,ranges=[directory/name for name in ('input','output','values.json','ranges.json')]
    source.write_bytes(section)
    process=subprocess.run([str(candidate),str(source),str(output),str(values),str(ranges),str(iterations)],check=True,capture_output=True,text=True)
    require(output.read_bytes()==section,'complete Map roundtrip mismatch')
    return json.loads(values.read_text()),json.loads(ranges.read_text()),json.loads(process.stdout)

def reject_cases(candidate,section,reference,ranges,directory):
    plots=[r for r in ranges if r['name']=='plot']
    sections={r['name']:r for r in ranges if r['name'] in SECTIONS}
    cases={name+' truncated':section[:row['end']-1] for name,row in sections.items()}
    def changed(offset,value):
        data=bytearray(section);data[offset:offset+len(value)]=value;return bytes(data)
    cases['future Map version']=changed(0,(5).to_bytes(4,'little'))
    cases['old Plot version']=changed(sections['plots']['start'],(5).to_bytes(4,'little'))
    cases['noncanonical wrap bool']=changed(20,b'\x02')
    cases['unknown Plot flag']=changed(plots[0]['start']+50,bytes([section[plots[0]['start']+50]|2]))
    first=reference['components'][0]
    name_start=first['start']-reference['_outer_start']-4-len(first['name'])
    cases['unknown component name']=changed(name_start,b'X')
    cases['unit dimensions inconsistent']=changed(sections['units']['start']+4,(reference['header']['width']+1).to_bytes(4,'little'))
    cases['trailing data']=section+b'\x00'
    rejected=[]
    for label,data in cases.items():
        source=directory/'bad';source.write_bytes(data)
        process=subprocess.run([str(candidate),str(source),str(directory/'bad-output'),str(directory/'bad-values'),str(directory/'bad-ranges'),'1'],capture_output=True,text=True)
        require(process.returncode!=0,'unsupported/truncated input accepted: '+label);rejected.append(label)
    return rejected

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    parser.add_argument('--module',type=Path,default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--iterations',type=int,default=50)
    parser.add_argument('--source-only-fixture',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();require(0<args.iterations<=10000,'iteration resource bound');require(sha(args.module.read_bytes())==XP2,'wrong original Linux target')
    canonical=args.private_root/'evidence/linux-game-load/canonical';manifest=json.loads((canonical/'manifest.json').read_text());require(manifest['module_sha256']==XP2,'canonical module mismatch')
    source_paths=sorted(['crates/c6-core/src/map.rs','crates/c6-core/src/lib.rs','crates/c6-core/src/unit/variants.rs','crates/c6-differential/src/bin/c6-map-codec-candidate.rs','tools/linux/map_codec_audit.py','tools/linux/map_state_audit.py']+[str(p.relative_to(ROOT)) for p in (ROOT/'crates/c6-core/src/map').glob('*.rs')])
    sources={p:sha((ROOT/p).read_bytes()) for p in source_paths}
    subprocess.run(['cargo','test','-p','c6-core','map::tests'],cwd=ROOT,check=True,stdout=subprocess.PIPE)
    subprocess.run(['cargo','build','--release','-p','c6-differential','--bin','c6-map-codec-candidate'],cwd=ROOT,check=True)
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--format-version=1','--no-deps'],cwd=ROOT));target=Path(metadata['target_directory']);candidate,candidate_sha=freeze(target/'release/c6-map-codec-candidate',target)
    require(sources=={p:sha((ROOT/p).read_bytes()) for p in source_paths},'source changed during build')
    fixtures=[];negative=None
    with tempfile.TemporaryDirectory(prefix='c6-map-codec-') as tmp:
        directory=Path(tmp)
        for index,identity in enumerate((INITIAL,RELOADED)):
            with gzip.open(canonical/(identity+'.bin.gz'),'rb') as stream:data=stream.read(80657463)
            require(len(data)==80657462 and sha(data)==identity,'original stream identity mismatch')
            trace_name,trace_sha=TRACES[index]
            with gzip.open(args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name,'rb') as stream:trace=stream.read(64*1024*1024+1)
            require(sha(trace)==trace_sha,'original owner trace mismatch')
            rows=[json.loads(line) for line in trace.splitlines()];rows=[r for r in rows if r.get('event')=='owner' and r.get('save')==1]
            outer=[r for r in rows if r['owner']=='GameCore::Map::Instance'];require(len(outer)==1,'missing unique original Map owner');outer=outer[0]
            lo,hi=outer['start'],outer['end'];section=data[lo:hi]
            reader=Reader(data,lo,hi);reference=reader.map();require(reader.pos==hi,'independent Map framing did not exhaust owner')
            # Strip only envelope diagnostics; every decoded typed field remains.
            expected=dict(reference);expected['components']=[{'name':v['name'],'value':v['value']} for v in reference['components']]
            expected=json.loads(json.dumps(expected))
            actual,ranges,benchmark=run(candidate,section,directory,args.iterations)
            require(actual==expected,'independent complete typed Map values differ')
            expected_owners=sorted((OWNER_NAMES[r['owner']],r['start']-lo,r['end']-lo) for r in rows if r['owner'] in OWNER_NAMES and lo<=r['start']<=r['end']<=hi)
            actual_owners=sorted((r['name'],r['start'],r['end']) for r in ranges if r['name'] in set(OWNER_NAMES.values()))
            require(actual_owners==expected_owners,'Rust nested owner boundaries differ from original trace')
            expected_sections=sorted((r['name'],r['start']-lo,r['end']-lo) for r in reader.ranges if r['name'] in SECTIONS)
            actual_sections=sorted((r['name'],r['start'],r['end']) for r in ranges if r['name'] in SECTIONS)
            require(actual_sections==expected_sections,'Rust section framing differs from independent native schema')
            if index==0:
                reference['_outer_start']=lo;negative=reject_cases(candidate,section,reference,ranges,directory)
            value_hash=sha(json.dumps(actual,sort_keys=True,separators=(',',':')).encode())
            fixtures.append(dict(stream_sha256=identity,trace_sha256=trace_sha,map_range=[lo,hi],map_sha256=sha(section),bytes=len(section),complete_byte_roundtrip=True,all_typed_values_equal=True,typed_leaf_values_compared=leaves(actual),typed_values_sha256=value_hash,original_nested_owner_boundaries_matched=len(expected_owners),independent_sections_matched=len(expected_sections),plot_records_compared=len(actual['plots']),plots_with_properties=sum(p['properties']is not None for p in actual['plots']),plots_with_city_attributes=sum(p['city']is not None for p in actual['plots']),counts=dict(visibility=len(actual['visibility']),areas=len(actual['areas']['entries']),landmasses=len(actual['landmasses']['entries']),regions=len(actual['regions']['regions']['entries']),chokepoints=len(actual['regions']['chokepoints']['entries']),continents=len(actual['continents']),feature_layouts=len(actual['features']['entries']),unit_layers=len(actual['units']['layers']),components=len(actual['components'])),benchmark=benchmark))
        source_only=None
        if args.source_only_fixture:
            synthetic=args.source_only_fixture.read_bytes();actual,_,_=run(candidate,synthetic,directory,1);independent=Reader(synthetic);expected=independent.map();require(independent.pos==len(synthetic),'source-only carrier trailing data');expected['components']=[{'name':v['name'],'value':v['value']} for v in expected['components']];require(actual==json.loads(json.dumps(expected)),'independent source-only typed values differ');source_only=dict(sha256=sha(synthetic),bytes=len(synthetic),exact_roundtrip=True,all_typed_values_equal=True,original_native_fixture=False,gameplay_valid_world=False,description='Independent hand-authored component/Plot wire carrier; no original-code parity.')
    require(sha(candidate.read_bytes())==candidate_sha,'frozen candidate changed during comparison');require(sources=={p:sha((ROOT/p).read_bytes()) for p in source_paths},'source changed during comparison')
    result=dict(schema='c6-linux-map-rust-codec-v1',checks_passed=True,module_sha256=XP2,canonical_manifest_sha256=sha((canonical/'manifest.json').read_bytes()),native_structural_contract_sha256=sha((ROOT/'evidence/linux/map-state.json').read_bytes()),native_structural_parser_sha256=sources['tools/linux/map_state_audit.py'],candidate_sha256=candidate_sha,frozen_candidate_path=str(candidate),candidate_frozen_before_comparison=True,source_sha256=sources,fixtures=fixtures,both_streams_same_map_bytes=fixtures[0]['map_sha256']==fixtures[1]['map_sha256'],negative_cases_rejected=negative,source_only_general_schema_test_groups=7,optional_independent_source_only_fixture=source_only,original_code_executed=False,private_upload_performed=False,whole_world_clone_per_action=False,performance_scope='Release decode includes model allocations/validation/drop with range collection disabled; encode includes allocations/temporary component buffers/drop; lookup uses precomputed canonical coordinates and plot indices. JSON diagnostics/range collection/setup/native code/I/O/game initialization/live reconstruction/definition resolution/movement/actions/turns/RNG are excluded.',allocation_policy='max_allocation charges logical model-owned vectors, boxes and conservative typed-variant storage; it is not a process heap ceiling. Encoded output/temporary component buffers are separately bounded by max_bytes. Range mode charges its output records and ordinary decode does not collect ranges.',native_fixture_coverage_gaps=['Nonempty Route portals','Nonempty six Terrain lists','Nonempty Territory records and recursive localized names','Nonempty Ice','Wrapped maps and native movement'],transport_profile_limits=['Current versions only','Known component names only','Supported typed-variant branches','Canonical plot spare bits/property counts/bools','Map plot count and UnitContainer dimensions consistent','Area player arrays width64','Current localized prefix/format and one name per named argument'],live_world_import_implemented=False,movement_implemented=False,full_action_acceptance=0,gameplay_parity=False,raw_save_load_different_bytes_retained=89198,unknown_bytes_masked=0,limits='Complete supported current Map/Plot wire state and typed named components, not native Initialize/Read rebuilds, live pointers/archive dirtiness, index/definition remapping, movement, actions or turns. Both native streams contain the same Map; source-only generalized branch fixtures are not native differential coverage.')
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'checks_passed':True,'candidate_sha256':candidate_sha,'fixtures':[{k:r[k] for k in ('bytes','plot_records_compared','typed_leaf_values_compared','original_nested_owner_boundaries_matched')} for r in fixtures],'gameplay_parity':False}))

if __name__=='__main__':main()
