#!/usr/bin/env python3
"""Source-close Linux Unit MaxMoves and HasMoved without engine execution."""
import argparse
import json
from pathlib import Path
import struct
from definition_ids_audit import ROOT, TARGET, HOST, elf, sha
from load_archive_postlude_audit import methods
import unit_initialization_definitions_audit as builders

METHODS = dict(unit_max_moves=0x2c27180,unit_base_moves=0x2c27990,
    unit_has_moved=0x2c27b50,unit_remaining=0x213cc90,unit_plot=0x2c1c820,
    unit_extra_moves=0x2c2c390,fight_while_embarked=0x28e0500,
    diplomatic_movement=0x2aaf540,diplomacy_writer=0x2a9c520,
    territory=0x2bed4f0,has_allied=0x2aa78a0,is_allied=0x2983170,
    symmetric_alliance=0x2988430,plot_cost=0x2bc9510,plot_city=0x1fe25b0,
    city_attributes_is_city=0x2bc8170,city_attributes_district_type=0x2bc81b0,
    plot_owner=0x1fe23d0,map_plot=0x2a48ba0,unit_base_getter=0x20df1d0,
    terrain_cost_getter=0x2bcad60,feature_cost_getter=0x2bcad70,
    improvement_cost_getter=0x2bcad80,embarked_base_getter=0x2c2c370,
    fixedpoint_integer=0x1fcdc10)


def audit(assets, private):
    module=assets/'libGameCore_XP2.so'
    elf(assets/'Civ6',HOST)
    native,asm,read=methods(module,TARGET,METHODS,private)
    base=0x2f59940
    targets=[base+x for x in struct.unpack('<8i',read(base,32))]
    expected=[0x2aaf5a1,0x2aaf5ae,0x2aaf5c8,0x2aaf5d5,0x2aaf5e2,0x2aaf5bb,0x2aaf60b,0x2aaf5ef]
    if targets!=expected:raise ValueError('diplomatic source jump table differs')
    import subprocess
    reloc={}
    for line in subprocess.check_output(['readelf','-rW',str(module)],text=True).splitlines():
        f=line.split()
        if len(f)>=5 and f[2]=='R_X86_64_64':reloc[int(f[0],16)]=int(f[3],16)
    if reloc.get(0x3ae23c0)!=0x2bc9510:raise ValueError('Plot slot20 is not actual GetMovementCost')
    checks=dict(
        has_moved_uses_unit_remaining='GameCore::Unit::Instance::GetMovesRemaining' in asm['unit_has_moved'],
        has_moved_uses_unit_max='GameCore::Unit::Instance::GetMaxMoves' in asm['unit_has_moved'],
        max_base_no_domain='mov    $0xffffffff,%esi' in asm['unit_max_moves'],
        max_queries_plot_cost_slot20='call   *0x20(%rax)' in asm['unit_max_moves'],
        q8_shift8=read(0x1fcdc10,3)==bytes.fromhex('c1e608'),
        diplomatic_positive_timestamp='test   %ecx,%ecx' in asm['diplomatic_movement'],
        diplomatic_expiry_inclusive='2aaf605:' in asm['diplomatic_movement'] and 'jl ' in asm['diplomatic_movement'],
        plot_cost_no_pillage_branch='IsPillaged' not in asm['plot_cost'],
        no_formation_call_in_unit_max='Formation::' not in asm['unit_max_moves'])
    if not all(checks.values()):raise ValueError(checks)
    builders.COLLECTIONS={
        'Unit':('Units','UnitType',0x221c190,0x225eec0,64),
        'Terrain':('Terrains','TerrainType',0x2219e80,0x225e380,13),
        'Feature':('Features','FeatureType',0x21f9150,0x22517e0,12),
        'Improvement':('Improvements','ImprovementType',0x21ffb60,0x2254cc0,24),
        'District':('Districts','DistrictType',0x21f51e0,0x224fce0,19)}
    definitions=builders.audit(assets,private/'definition-builders')
    return dict(schema='c6-linux-unit-movement-state-source-v1',gamecore_sha256=TARGET,
        host_sha256=HOST,original_execution=False,methods=native,checks=checks,
        plot_virtual_slot=dict(vtable='0x3ae2390',address_point_offset='0x20',relocation='0x3ae23c0',target='0x2bc9510',meaning='GetMovementCost, not PlotType'),
        diplomatic_source_to_timing_index={'1':2,'2':0,'3':1,'4':3,'5':4,'6':6,'8':5},
        ignored_sources='7 and all values outside1..8',
        timing_predicate='last>0 && wrapping_i32(last+turns_active)>=current_turn; ordered wrapping sum',
        formulas=dict(base_no_domain='definition.BaseMoves unless domain!=1 && embarked && !fight_while_embarked; then global MOVEMENT_WHILE_EMBARKED_BASE + owner Units embarked modifier',
            max_moves='base + extra + current diplomatic modifier + conditional plot-cost-one clear bonus + diplomatic territory bonus; wrapping_i32 throughout',
            has_moved='Unit.remaining_raw_q8 < wrapping_i32(max_moves << 8)',
            formation_interaction=False),
        native_definition_builders=definitions,
        missing_inputs=['actual mutable Game.DiplomaticRelations alliance vector and player_count only when unequal territorial bonuses require it',
            'actual manager validity if city/district lookup requires it'],
        direct_native_unit_query_comparison=False,complete_state_action_turn_parity=False)


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
