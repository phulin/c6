#!/usr/bin/env python3
"""Stage an exact Linux host with the layout required by genuine ASL_FileInit.

Civ6 must be copied: /proc/self/exe resolves symlinks to the original location.
Original content remains unchanged behind explicitly recorded input aliases.
"""
import argparse
import json
from pathlib import Path
import shutil
import sys

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'assets'))
from inventory import digest, identity

ROOT=Path(__file__).resolve().parents[2]
HOST='106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
ALIASES={'steamassets/base':'Base','steamassets/dlc':'DLC','aspyrassets':'aspyrassets'}


def stage(assets,destination):
    assets=assets.resolve(strict=True);destination=destination.resolve()
    if destination.exists() or destination.is_relative_to(ROOT) or destination.is_relative_to(assets):
        raise ValueError('launch fixture must be a new private directory outside source and original assets')
    if digest(assets/'Civ6')!=HOST:raise ValueError('Linux host SHA-256 gate failed')
    for source in ALIASES.values():
        if not (assets/source).resolve(strict=True).is_relative_to(assets):raise ValueError('alias input escapes Linux assets')
    destination.mkdir(parents=True)
    shutil.copyfile(assets/'Civ6',destination/'Civ6')
    (destination/'Civ6').chmod(0o555)
    if digest(destination/'Civ6')!=HOST:raise ValueError('copied native host identity mismatch')
    (destination/'steamassets').mkdir()
    for relative,source in ALIASES.items():(destination/relative).symlink_to(assets/source,target_is_directory=True)
    mapping=[{'fixture_path':path,'original_path':source} for path,source in sorted(ALIASES.items())]
    result={'format':'c6.linux-original-asl-launch-fixture.v1','copied_host':{'filename':'Civ6','sha256':HOST},
            'mapping':mapping,'mapping_sha256':identity(mapping),
            'native_authority':{'assets_path_method':'ASL::ASL_GetAppAssetsPath','address':'0x2cae9c0',
                                'rule':'dirname(/proc/self/exe) + /steamassets',
                                'filesystem_initialize':'ASL::ASL_FileInit','initializer_address':'0x2cc7d98'},
            'limits':['Source aliases are an input contract; original content is not modified or copied.',
                      'Native ASL initialization and mod activation must be executed and validated separately.',
                      'Exact copied host bytes are unchanged; no binary relocation or opcode patching.']}
    (destination/'launch-fixture.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    return result


def validate(assets,fixture):
    """Validate exact copied host, complete alias layout and current Linux inputs."""
    assets=assets.resolve(strict=True);fixture=fixture.resolve(strict=True)
    if fixture.is_relative_to(ROOT) or fixture.is_relative_to(assets):
        raise ValueError('launch fixture must remain outside public/original asset trees')
    manifest_path=fixture/'launch-fixture.json'
    manifest=json.loads(manifest_path.read_text())
    mapping=[{'fixture_path':path,'original_path':source} for path,source in sorted(ALIASES.items())]
    if (manifest.get('format')!='c6.linux-original-asl-launch-fixture.v1' or
            manifest.get('mapping')!=mapping or manifest.get('mapping_sha256')!=identity(mapping) or
            manifest.get('copied_host')!={'filename':'Civ6','sha256':HOST}):
        raise ValueError('unsupported native ASL launch mapping profile')
    host=fixture/'Civ6'
    if host.is_symlink() or not host.is_file() or digest(host)!=HOST:
        raise ValueError('launch requires an exact regular-file host copy')
    if (fixture/'steamassets').is_symlink() or not (fixture/'steamassets').is_dir():
        raise ValueError('steamassets must remain a real fixture directory')
    for path,source in ALIASES.items():
        link=fixture/path
        if not link.is_symlink() or link.resolve(strict=True)!=(assets/source).resolve(strict=True):
            raise ValueError('native ASL source alias mismatch: '+path)
    expected={'Civ6','steamassets','steamassets/base','steamassets/dlc','aspyrassets','launch-fixture.json'}
    observed={path.relative_to(fixture).as_posix() for path in fixture.rglob('*')}
    if observed!=expected:raise ValueError('undeclared launch fixture entry')
    gate_path=ROOT/'evidence/linux/rules-content-inputs.json'
    gate=json.loads(gate_path.read_text())
    for name,sha in {**gate['files'],**gate['modules']}.items():
        if digest(assets/name)!=sha:raise ValueError('native Linux content input mismatch: '+name)
    result=dict(manifest)
    result['validated_input_gate_sha256']=digest(gate_path)
    result['validated_source_inventory_sha256']=gate['source_inventory_sha256']
    result['validated_native_module_identities']=gate['modules']
    result['fixture_manifest_sha256']=digest(manifest_path)
    return result


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--destination',type=Path,required=True)
    a=p.parse_args();result=stage(a.assets,a.destination)
    print(json.dumps({'host_sha256':result['copied_host']['sha256'],'mapping_sha256':result['mapping_sha256']}))


if __name__=='__main__':main()
