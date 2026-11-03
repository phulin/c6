#!/usr/bin/env python3
"""Pin native War/Alliance collection builders and distinct enum application."""
import argparse
import json
from pathlib import Path
from definition_ids_audit import ROOT, TARGET, HOST
from load_archive_postlude_audit import methods
import unit_initialization_definitions_audit as builders

METHODS=dict(war_validate_hash=0x2d514d0,alliance_hash_to_index=0x2bf6fc0,
    war_find=0x295cfb0,alliance_find=0x206f1a0,
    war_hash=0x2319660,alliance_hash=0x2082e90,alliance_index=0x208ed30)

def audit(assets,private):
    native,asm,_=methods(assets/'libGameCore_XP2.so',TARGET,METHODS,private)
    checks=dict(war_returns_type_hash='War::GetTypeHash()' in asm['war_validate_hash'],
        alliance_returns_collection_index='Alliance::GetCollectionIndex()' in asm['alliance_hash_to_index'],
        war_direct_index_first='jbe ' in asm['war_find'] and 'shl    $0x3,%rbx' in asm['war_find'],
        alliance_direct_index_first='jbe ' in asm['alliance_find'] and 'shl    $0x3,%rbx' in asm['alliance_find'],
        missing_war_minus_one='mov    $0xffffffff,%eax' in asm['war_validate_hash'],
        missing_alliance_minus_one='mov    $0xffffffff,%eax' in asm['alliance_hash_to_index'])
    if not all(checks.values()):raise ValueError(checks)
    builders.COLLECTIONS={
        'Alliance':('Alliances','AllianceType',0x21e8f90,0x224ad00,1),
        'War':('Wars','WarType',0x221f7c0,0x22609c0,3)}
    definitions=builders.audit(assets,private/'builders')
    return dict(schema='c6-linux-game-relations-definition-adapter-v1',gamecore_sha256=TARGET,
        host_sha256=HOST,checks=checks,methods=native,native_builders=definitions,
        raw_re='private linux-game-relations-definitions',
        war_semantics='Find ordinal first or unsigned hash; return found definition type hash; miss UINT_MAX',
        alliance_semantics='Find ordinal first or unsigned hash; return found definition collection index; miss -1',
        ordering='signed rowid SQL order -> size-before-append index; duplicate hash last insertion wins',
        actual_native_alliance_runtime_comparison=False,original_execution=False,
        complete_diplomacy_or_state_action_turn_parity=False)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-output',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();a.private_output.mkdir(parents=True,exist_ok=True)
    result=audit(a.assets,a.private_output)
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(result,sort_keys=True,indent=2)+'\n')

if __name__=='__main__':main()
