#!/usr/bin/env python3
"""Inventory current Linux content inputs; never infer native activation/order."""
import argparse
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'assets'))
from inventory import digest, identity

ROOT=Path(__file__).resolve().parents[2]
HOST='106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
GAME='a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'


def inventory(assets):
    assets=assets.resolve(strict=True)
    modules={name:digest(assets/name) for name in ['Civ6','libGameCore_Base.so','libGameCore_XP1.so','libGameCore_XP2.so']}
    if modules['Civ6']!=HOST or modules['libGameCore_XP2.so']!=GAME:
        raise ValueError('current Linux target module identity mismatch')
    files={}
    for name in ('Base/assets','DLC'):
        for source in sorted((assets/name).rglob('*')):
            if source.is_file():
                if not source.resolve().is_relative_to(assets):raise ValueError('source escapes supplied Linux tree')
                files[source.relative_to(assets).as_posix()]=digest(source)
    mods=[]
    for name in files:
        if not name.endswith('.modinfo'):continue
        mod=ET.parse(assets/name).getroot()
        actions=[]
        for phase in ('FrontEndActions','InGameActions'):
            for index,action in enumerate(mod.findall(phase+'/*')):
                actions.append({'phase':phase,'declaration_index':index,'kind':action.tag,
                                'id':action.get('id'),'criteria':action.get('criteria'),
                                'load_order':action.findtext('Properties/LoadOrder'),
                                'file_count':len(action.findall('File'))})
        mods.append({'path':name,'id':mod.get('id'),'version':mod.get('version'),'actions':actions})
    return {'format':'c6.linux-content-input-inventory.v1','modules':modules,'files':files,
            'source_inventory_sha256':identity(files),'mods':mods,
            'active_mods_proven':False,'native_component_order_proven':False,
            'limits':['Installed Linux files are inputs, not an enabled-mod list or native load recipe.',
                      'Declaration metadata is evidence for native resolution; no Windows order or eligibility is reused.']}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();result=inventory(a.assets)
    a.output.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    print(json.dumps({'files':len(result['files']),'mods':len(result['mods']),
                      'source_inventory_sha256':result['source_inventory_sha256']}))


if __name__=='__main__':main()
