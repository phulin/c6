#!/usr/bin/env python3
"""Offline Player47 saved-alive cross-check against actual native primitive state.

No native code executes. Complete Player payload import is deliberately not claimed.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
from core_stream_index import Reader
from player_header_control_audit import METHODS
from save_lifecycle_audit import Elf,GAME_SHA
ROOT=Path(__file__).resolve().parents[2]
CORE='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE='528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
KNOWN_PAIRS={(CORE,TRACE),('85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8','027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')}
def sha(data):return hashlib.sha256(data).hexdigest()
def require(value,reason):
 if not value:raise ValueError(reason)
def source_gate(module):
 elf=Elf(module,GAME_SHA)
 for address,size,name,digest in METHODS:
  require(sha(elf.read(address,size))==digest,'Player header method differs: '+name)
 require(elf.read(0x2b4ef24,7).hex()=='498db650020000','first header bool wrapper differs')
 elf.call(0x2b4ef2e,0x1e45260)
def saved_player_alive(data,trace,core_sha256=CORE,trace_sha256=TRACE):
 """Exact attributed current headers; never infer owner/boolean from byte search."""
 require((core_sha256,trace_sha256) in KNOWN_PAIRS,'unverified Core/owner-trace pairing')
 require(sha(data)==core_sha256,'Core owner-cut identity differs')
 require(sha(trace)==trace_sha256,'Player owner-trace identity differs')
 rows=[json.loads(line) for line in trace.splitlines()]
 rows=[r for r in rows if r.get('event')=='owner' and r.get('save')==1 and r.get('owner')=='GameCore::Player::Instance']
 require(bool(rows),'no attributed Player writer cuts')
 result={}
 for row in rows:
  start,end=row['start'],row['end'];require(0<=start<end<=len(data) and start+98<=end,'Player writer cut outside stream')
  reader=Reader(data[start:start+98]);reader.version(47)
  owner=reader.take('i')
  for _ in range(2+15):reader.take('i')
  flags=[reader.boolean() for _ in range(6)]
  for _ in range(4):reader.u32()
  require(reader.pos==98,'Player47 header boundary differs')
  require(owner not in result,'duplicate saved Player identity')
  result[owner]=dict(owner=owner,saved_is_alive=flags[0],header_start=start,header_bytes=98,header_sha256=sha(data[start:start+98]))
 return result

def compare_actual_alive(saved,metadata_rows):
 """Return equality evidence; an uncaptured actual player never means dead."""
 actual={}
 for row in metadata_rows:
  if row.get('kind')!='player':continue
  require(type(row.get('resolved')) is bool,'nonboolean Player resolution state')
  if not row['resolved']:continue
  owner=row['actual_owner'];require(type(owner) is int and 0<=owner<65,'invalid actual Player identity')
  require(owner not in actual,'duplicate actual Player identity')
  require(type(row['alive']) is bool,'nonboolean actual Core alive state')
  actual[owner]=row['alive']
 require(set(actual)==set(saved),'saved/actual Player roster differs; no implicit missing-player exclusion')
 for owner,value in actual.items():
  require(value==saved[owner]['saved_is_alive'],'saved/actual m_bIsAlive differs for owner '+str(owner))
 return dict(actual_players_compared=len(actual),all_saved_alive_equal_actual_core_fields=True)

def read(path):
 data=path.read_bytes();return gzip.decompress(data) if path.suffix=='.gz' else data

def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--module',type=Path,default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
 p.add_argument('--core',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-game-load/canonical'/(CORE+'.bin.gz'))
 p.add_argument('--trace',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz')
 p.add_argument('--core-sha256',default=CORE);p.add_argument('--trace-sha256',default=TRACE)
 p.add_argument('--metadata',type=Path,help='fresh original placement NDJSON metadata; never expected count answers')
 p.add_argument('--output',type=Path,required=True)
 a=p.parse_args();source_gate(a.module);data=read(a.core);trace=read(a.trace)
 saved=saved_player_alive(data,trace,a.core_sha256,a.trace_sha256)
 actual=compare_actual_alive(saved,[json.loads(x) for x in a.metadata.read_text().splitlines()]) if a.metadata else None
 report=dict(schema='c6-linux-player-alive-saved-crosscheck-v1',gamecore_sha256=GAME_SHA,checks_passed=True,
  original_execution_performed=False,source_contract='a087351; Player47 first header bool is m_bIsAlive; actual getter reads +0x260',
  original_core_sha256=sha(data),original_trace_sha256=sha(trace),
  player_headers_parsed=len(saved),saved_alive_players=sum(v['saved_is_alive'] for v in saved.values()),
  header_wire_bytes_exhausted=True,header_bytes=98,semantic_fields_compared=['m_bIsAlive'],
  actual_core_alive_comparison=actual,actual_native_metadata_available=a.metadata is not None,
  metadata_sha256=sha(a.metadata.read_bytes()) if a.metadata else None,
  audit_source_sha256=sha(Path(__file__).read_bytes()),
  full_player_payload_import=False,player_mapping_from_saved_headers=False,configuration_team_or_slot_from_saved_headers=False,
  native_count_comparison=False,complete_state_actions_turns=False,historical_raw_save_load_difference_bytes=89198)
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,player_headers=len(saved),actual_core_alive_compared=actual is not None)))
if __name__=='__main__':main()
