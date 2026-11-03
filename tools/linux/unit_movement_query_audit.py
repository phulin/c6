#!/usr/bin/env python3
"""Pin the standalone movement query ABI; never load/execute original code."""
import argparse
import json
import re
import subprocess
from pathlib import Path
from definition_ids_audit import ROOT, TARGET, HOST, elf, sha
from load_archive_postlude_audit import methods

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    source=ROOT/'native/linux/unit_movement_query_fixture.c'
    text=source.read_text()
    addresses={name:int(address,16) for name,address in re.findall(r'BIND\((\w+),(0x[0-9a-f]+)\)',text)}
    addresses.update(unit_base_moves=0x2c27990,unit_remaining=0x213cc90,
        unit_location=0x2004620,unit_activity=0x265b300,unit_type=0x1fe4ef0,unit_delayed_death=0x2004610,unit_combat_modifiers=0x213cea0,
        unit_modifiers_extra=0x2c2c390,unit_modifiers_clear=0x2c2c3a0,
        unit_modifiers_friendly=0x2c2c3b0,unit_modifiers_enemy=0x2c2c3c0,
        combat_fight_embarked=0x28e0500,diplomatic_movement=0x2aaf540)
    # Existing source audit independently pins semantics, jump table and Plot
    # virtual relocation. This pins every new fixture call site to this ELF.
    report,asm,read=methods(a.assets/'libGameCore_XP2.so',TARGET,addresses,a.private)
    elf(a.assets/'Civ6',HOST)
    fields = {0x265b300:'8b8790020000c3',0x2004610:'8a87f0020000c3',
        0x1fe4ef0:'8b87d0000000c3',0x213cea0:'488d87a8070000c3',
        0x28e0500:'8a8782000000c3'}
    for address, expected in fields.items():
        value=bytes.fromhex(expected)
        if read(address,len(value))!=value: raise ValueError('field layout drift')
    if '0x5ff1bbdf' not in asm['is_embarked'] or '0x2c0' not in asm['is_embarked']:
        raise ValueError('effective embarkation activity predicate')
    if 'GetPlot() const'  not in asm['plot'] or 'GetMaxMoves() const' not in asm['max_moves'] or 'HasMoved() const' not in asm['has_moved']:
        raise ValueError('target symbol signatures')
    if 'IsDead' in text or 'IsBusy' in text or 'fixture_run' in text:
        raise ValueError('unexpected gameplay filtering/invocation')
    checks=dict(all_stored_units=True,owner_full_id_find_checked=True,
        no_delayed_death_filter=True,no_unplaced_filter=True,
        independent_three_argument_guard_required=True,
        guard_implementation_and_acquisition_not_supplied=True,
        complete_inputs_before_after=True,answers_in_separate_file=True,
        no_save_equality_or_idle_queue_gate=True,
        no_original_execution=True,native_query_comparison=False)
    result=dict(schema='c6-linux-unit-movement-query-abi-v1',gamecore_sha256=TARGET,host_sha256=HOST,
        sources={str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in [source,source.with_suffix('.h'),Path(__file__)]},
        methods=report,checks=checks,expected_units=1262,expected_direct_target_calls=2524,
        preconditions='genuine stored Unit with owner resolved through actual initialized Player manager, full ID Find identity, actual initialized definitions/global owners; no placement/death predicate in target getters',
        coverage_pending_original_execution=True,full_state_action_turn_parity=False)
    a.output.write_text(json.dumps(result,sort_keys=True,indent=2)+'\n')
    print(json.dumps(dict(methods=len(report),checks=checks),sort_keys=True))

if __name__=='__main__':main()
