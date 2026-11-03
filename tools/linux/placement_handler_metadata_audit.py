#!/usr/bin/env python3
"""Pin handler producer source, unchanged count code and original ABI packets."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from placement_metadata_audit import audit as count_audit

ROOT=Path(__file__).resolve().parents[2]
BASELINE='9ae7f7e'


def section(source,start,end):
    return source[source.index(start):source.index(end,source.index(start))]


def source_contract():
    old=subprocess.check_output(['git','show',BASELINE+':native/linux/placement_metadata.c'],cwd=ROOT,text=True)
    new=(ROOT/'native/linux/placement_handler_metadata.c').read_text()
    normalized=new.replace('c6_linux_placement_handler_metadata_','c6_linux_placement_metadata_')
    assert section(old,'typedef const void *(*P0)', '} Api;')==section(new,'typedef const void *(*P0)', '} Api;'), 'count ABI declarations changed'
    assert section(old,'static uintptr_t pointer_at','static int members(')==section(normalized,'static uintptr_t pointer_at','static int members('), 'count bindings/registry/relations/query bodies changed'
    members=section(new,'static int members(','static int players(')
    members=members.replace('unsigned kind,HandlerUnits *handler_units)', 'unsigned kind)',1)
    inventory='''            if(handler_units->count>=MAX_HANDLER_UNITS)return 251;
            handler_units->entries[handler_units->count++]=(HandlerUnit){member,container,owner,n,id};
'''
    assert members.count(inventory)==1
    members=members.replace(inventory,'',1)
    assert members==section(old,'static int members(','static int players('), 'actual-member traversal/count queries changed'
    players=section(new,'static int players(','static int handler_queries(')
    players=players.replace('const void *map_units,HandlerUnits *handler_units)', 'const void *map_units)',1).replace('player,i,kind,handler_units);','player,i,kind);',1)
    assert players==section(old,'static int players(','int c6_linux_placement_metadata_probe('), 'player traversal changed'
    assert 'for(unsigned repeat=0;repeat<2;repeat++)' in new
    assert 'unsigned variants=current?2:1;' in new
    assert 'if(a->find_unit(entry->container,entry->id)!=entry->unit||' in new
    assert new.index('rc=handler_queries(')<new.index('for(unsigned i=2;')
    assert 'delayed_death' not in new and 'a->alive(entry' not in new, 'handler inventory must not filter stored Units'
    assert 'original_leaf_called\\\":false' in new and 'original_leaf_called\\\":true' in new
    names=['native/linux/placement_handler_metadata.c','native/linux/placement_handler_metadata.h','tools/linux/placement_handler_metadata_audit.py']
    return {'baseline_commit':subprocess.check_output(['git','rev-parse',BASELINE],cwd=ROOT,text=True).strip(),'count_bodies_and_complete_membership_preserved':True,'unit_inventory_unconditional':True,'repeat_passes':2,'plot_variants':'actual current Plot and NULL; NULL only when current Plot absent','visible_variants':[False,True],'no_call_controls_separate':True,'sources':{n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in names}}


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux');parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    counts=count_audit(args.assets/'libGameCore_XP2.so')
    assert counts==json.loads((ROOT/'evidence/linux/placement-metadata-preparation.json').read_text()), 'count native packet changed'
    with tempfile.TemporaryDirectory(prefix='handler-query-audit-') as temporary:
        path=Path(temporary)/'query.json'
        subprocess.run([sys.executable,str(ROOT/'tools/linux/skip_turn_handler_query_audit.py'),'--assets',str(args.assets),'--output',str(path)],check=True,capture_output=True,text=True)
        handler=json.loads(path.read_text())
    assert handler==json.loads((ROOT/'evidence/linux/skip-turn-handler-query-abi.json').read_text()), 'handler native packet changed'
    result={'schema':'c6-linux-placement-handler-producer-v1','native_execution_performed':False,'source_contract':source_contract(),'count_contract':counts,'handler_contract':handler,'full_manager_legality':False,'complete_state_actions_turns':False}
    args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'source_checks_passed':True,'native_execution_performed':False}))


if __name__=='__main__':main()
