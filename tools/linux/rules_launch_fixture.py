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
MODULE_ALIASES={name:name for name in ('libGameCore_Base.so','libGameCore_XP1.so','libGameCore_XP2.so')}
CWD_ALIASES={'steamassets/base/assets':'Base/assets','steamassets/dlc':'DLC','aspyrassets':'aspyrassets'}
CWD_DIRECTORIES=('steamassets','steamassets/base','steamassets/base/binaries','steamassets/base/binaries/linux')


def stage(assets,destination,gamecore_siblings=False,preserve_cwd=False):
    assets=assets.resolve(strict=True);destination=destination.resolve()
    if destination.exists() or destination.is_relative_to(ROOT) or destination.is_relative_to(assets):
        raise ValueError('launch fixture must be a new private directory outside source and original assets')
    if digest(assets/'Civ6')!=HOST:raise ValueError('Linux host SHA-256 gate failed')
    aliases={**(CWD_ALIASES if preserve_cwd else ALIASES),**(MODULE_ALIASES if gamecore_siblings or preserve_cwd else {})}
    for source in aliases.values():
        if not (assets/source).resolve(strict=True).is_relative_to(assets):raise ValueError('alias input escapes Linux assets')
    destination.mkdir(parents=True)
    shutil.copyfile(assets/'Civ6',destination/'Civ6')
    (destination/'Civ6').chmod(0o555)
    if digest(destination/'Civ6')!=HOST:raise ValueError('copied native host identity mismatch')
    for relative in (CWD_DIRECTORIES if preserve_cwd else ('steamassets',)):
        (destination/relative).mkdir()
    for relative,source in aliases.items():(destination/relative).symlink_to(assets/source,target_is_directory=(assets/source).is_dir())
    mapping=[{'fixture_path':path,'original_path':source} for path,source in sorted(aliases.items())]
    result={'format':('c6.linux-original-asl-launch-fixture.v3' if preserve_cwd else 'c6.linux-original-asl-launch-fixture.v2' if gamecore_siblings else 'c6.linux-original-asl-launch-fixture.v1'),'copied_host':{'filename':'Civ6','sha256':HOST},
            'mapping':mapping,'mapping_sha256':identity(mapping),
            'native_authority':{'assets_path_method':'ASL::ASL_GetAppAssetsPath','address':'0x2cae9c0',
                                'rule':'dirname(/proc/self/exe) + /steamassets',
                                'filesystem_initialize':'ASL::ASL_FileInit','initializer_address':'0x2cc7d98'},
            'limits':['Source aliases are an input contract; original content is not modified or copied.',
                      'Native ASL initialization and mod activation must be executed and validated separately.',
                      'Exact copied host bytes are unchanged; no binary relocation or opcode patching.']}
    if preserve_cwd:
        result['directory_skeleton']=list(CWD_DIRECTORIES)
    (destination/'launch-fixture.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    return result


def validate(assets,fixture):
    """Validate exact copied host, complete alias layout and current Linux inputs."""
    assets=assets.resolve(strict=True);fixture=fixture.resolve(strict=True)
    if fixture.is_relative_to(ROOT) or fixture.is_relative_to(assets):
        raise ValueError('launch fixture must remain outside public/original asset trees')
    manifest_path=fixture/'launch-fixture.json'
    manifest=json.loads(manifest_path.read_text())
    version=manifest.get('format')
    if version not in ('c6.linux-original-asl-launch-fixture.v1','c6.linux-original-asl-launch-fixture.v2','c6.linux-original-asl-launch-fixture.v3'):
        raise ValueError('unsupported native ASL launch mapping version')
    preserve_cwd=version.endswith('.v3')
    aliases={**(CWD_ALIASES if preserve_cwd else ALIASES),**(MODULE_ALIASES if not version.endswith('.v1') else {})}
    mapping=[{'fixture_path':path,'original_path':source} for path,source in sorted(aliases.items())]
    if (
            manifest.get('mapping')!=mapping or manifest.get('mapping_sha256')!=identity(mapping) or
            manifest.get('copied_host')!={'filename':'Civ6','sha256':HOST}):
        raise ValueError('unsupported native ASL launch mapping profile')
    host=fixture/'Civ6'
    if host.is_symlink() or not host.is_file() or digest(host)!=HOST:
        raise ValueError('launch requires an exact regular-file host copy')
    if (fixture/'steamassets').is_symlink() or not (fixture/'steamassets').is_dir():
        raise ValueError('steamassets must remain a real fixture directory')
    if preserve_cwd:
        if manifest.get('directory_skeleton')!=list(CWD_DIRECTORIES):
            raise ValueError('native cwd directory skeleton mismatch')
        for relative in CWD_DIRECTORIES:
            path=fixture/relative
            if path.is_symlink() or not path.is_dir():raise ValueError('native cwd must stay in real fixture directories')
    for path,source in aliases.items():
        link=fixture/path
        if not link.is_symlink() or link.resolve(strict=True)!=(assets/source).resolve(strict=True):
            raise ValueError('native ASL source alias mismatch: '+path)
    expected={'Civ6','steamassets','launch-fixture.json',*aliases.keys(),*(CWD_DIRECTORIES if preserve_cwd else ())}
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
    p.add_argument('--gamecore-siblings',action='store_true',help='Original native module lookup requires all three GameCore SO siblings')
    p.add_argument('--preserve-native-cwd',action='store_true',help='Keep ASL reverse-mapped cwd beneath a real base/binaries/linux directory skeleton')
    a=p.parse_args();result=stage(a.assets,a.destination,a.gamecore_siblings,a.preserve_native_cwd)
    print(json.dumps({'host_sha256':result['copied_host']['sha256'],'mapping_sha256':result['mapping_sha256']}))


if __name__=='__main__':main()
