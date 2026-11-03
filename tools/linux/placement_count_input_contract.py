#!/usr/bin/env python3
"""Pinned Linux placement candidate topology/input contract, no native execution."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf,GAME_SHA
from unit_placement_audit import METHODS as PLACEMENT
ROOT=Path(__file__).resolve().parents[2]
METHODS=[
 ('Formation.Layer',0x293daf0,161,'652273c96a8a65d0f2d72d0f614e81e423f23e03acda8ae53d79f8d17ebd2a6f'),
 ('Map.AdjacentCoord',0x1fe2450,16,'d49aaa6a8e22ce6c82d1f979740d9ee339fe37d6bceaa260971abb2e1a9a1ebe'),
 ('Map.AdjacentXY',0x1fe6bb0,117,'20d1b6cd6f0a199ffc43585e40fed52480a603425383291e73ba00e68f8f8e6e'),
 ('Hex.PlotXToX',0x1fe6c30,27,'ef7fa80e635393c23fb28022e3b5776e0726ac8ba056e1e34756e729abd8a692'),
 ('Hex.XToPlotX',0x1fe6c70,27,'ecd5d792efcce060e5d9a9c8cf81309197f8be32e680337c624bde85ae851aba'),
 ('Hex.DirectionX',0x1fe6c50,14,'6970d362e7b883fd0bc6155fe3e8327590c9cd99d9e1870580cd8d72e61b0986'),
 ('Hex.DirectionY',0x1fe6c60,14,'da454a67c04c94bd3a82f8a320cd534a2f8e8ac60ead40e66a3025f6ea10298e'),
 ('Map.GetPlotCoord',0x2a48ba0,169,'44dca219db554b1884a343cdbdfe4e5f1ef3ca7695c848cb1e0fe72b933ad4ec'),
 ('Map.GetPlotXY',0x2a48a30,168,'55e2269824af4db4e69fd3c1525d033284cbe8890485439af00e1b97db203839'),
 ('Map.CoordRange',0x243bb00,28,'dc7e6d453df9764c167593b8c1b68adccf891a0f82332a4c04809768fd3b3136'),
]
def sha(data):return hashlib.sha256(data).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--module',type=Path,default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
 p.add_argument('--output',type=Path,required=True)
 a=p.parse_args();elf=Elf(a.module,GAME_SHA)
 for name,address,size,digest in PLACEMENT+METHODS:
  if sha(elf.read(address,size))!=digest:raise ValueError('method mismatch: '+name)
 if elf.relocated_pointer(0x3afb150+16+0x68)!=0x2a48a30:raise ValueError('actual XP2 Map GetPlotXY binding')
 if struct.unpack('<6i',elf.read(0x3c00480,24))!=(0,1,1,0,-1,-1):raise ValueError('native X directions')
 if struct.unpack('<6i',elf.read(0x3c004a0,24))!=(1,0,-1,-1,0,1):raise ValueError('native Y directions')
 paths=['crates/c6-core/src/world.rs','crates/c6-core/src/unit_placement.rs','crates/c6-differential/src/bin/c6-linux-placement-count-candidate.rs','tools/linux/placement_count_candidate.py','tools/linux/placement_count_input_contract.py','docs/linux-placement-count-input.schema.json']
 result=dict(schema='c6-linux-placement-count-input-contract-v1',gamecore_sha256=GAME_SHA,static_checks_passed=True,
  native_execution=False,native_count_comparison=False,native_handler_comparison=False,
  existing_methods_checked=len(PLACEMENT),additional_methods=[dict(name=n,address=hex(v),size=z,sha256=h) for n,v,z,h in METHODS],
  sources={p:sha((ROOT/p).read_bytes()) for p in paths},
  original_direction_x=[0,1,1,0,-1,-1],original_direction_y=[1,0,-1,-1,0,1],
  formation_layer='valid location and nonempty members: first raw UnitID at Formation.location; no Find or death test',
  lookup_state='complete logical slots, explicit absent allocator metadata; actual mapping and captured null slots remain distinct',
  expected_answers_accepted_as_inputs=False,complete_state_actions_turns=False,historical_raw_save_load_difference_bytes=89198)
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps(dict(static_checks_passed=True,existing_methods=len(PLACEMENT),additional_methods=len(METHODS))))
if __name__=='__main__':main()
