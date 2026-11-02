#!/usr/bin/env python3
"""Stage unchanged Linux base rule sources for native Bootstrap's Schema spelling.

Only the directory spelling evidenced in the Linux executable is adapted.
No SQL/XML content or original installed asset is changed. The explicit private
mapping is fixture input, not a claim that application filesystem setup ran.
"""
import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'assets'))
from inventory import digest, identity

ROOT=Path(__file__).resolve().parents[2]


def stage(assets, destination, gate_path):
    assets=assets.resolve(strict=True);destination=destination.resolve()
    if destination.exists() or destination.is_relative_to(ROOT) or destination.is_relative_to(assets):
        raise ValueError('choose a new private destination outside public source/original assets')
    gate=json.loads(gate_path.read_text())
    sources=gate['files']
    mapping=[]
    for name, expected in sorted(sources.items()):
        source=(assets/name).resolve(strict=True)
        if not source.is_relative_to(assets) or digest(source)!=expected:
            raise ValueError('Linux original source identity mismatch: '+name)
        components=Path(name).parts
        staged=Path(*('Schema' if component=='schema' else component for component in components))
        mapping.append({'source':name,'staged':staged.as_posix(),'sha256':expected})
    destination.mkdir(parents=True)
    for item in mapping:
        target=destination/item['staged'];target.parent.mkdir(parents=True,exist_ok=True)
        target.symlink_to(assets/item['source'])
    result={'format':'c6.linux-native-base-source-view.v1','original_source_gate_sha256':digest(gate_path),
            'source_view_sha256':identity(mapping),'mapping':mapping,
            'native_authority':{'gameplay_bootstrap_rva':'0x3041ab0','schema_literal_reference_rva':'0x3041b33',
                                'schema_literal':'Schema','directory_enumeration_rva':'0x3c41283'},
            'limits':['Directory case adaptation only; original application filesystem initialization remains unexecuted.',
                      'All linked SQL/XML bytes are unchanged pinned Linux sources; no Windows source/profile imported.']}
    (destination/'source-view.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--destination',type=Path,required=True)
    parser.add_argument('--gate',type=Path,default=ROOT/'tools/linux/runtime-content-gate.json')
    args=parser.parse_args()
    result=stage(args.assets,args.destination,args.gate)
    print(json.dumps({'source_view_sha256':result['source_view_sha256'],'sources':len(result['mapping'])}))


if __name__=='__main__':main()
