#!/usr/bin/env python3
"""Offline current-handler integration source gates; never launches original code."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
ROOT=Path(__file__).resolve().parents[2]
SOURCES=['native/linux/current_handler_interruption.c','native/linux/current_handler_interruption.h',
'native/linux/current_handler_metadata_fixture.c','native/linux/current_handler_metadata_fixture.h',
'native/linux/current_handler_metadata_probe.c','native/linux/action_lifecycle.c','native/linux/action_lifecycle.h',
'native/linux/game_load.c','native/linux/mod_activation.c','native/linux/context_stream.c','native/linux/context_stream.h',
'native/linux/stream_observation.h','tools/linux/mod_activation_probe.py','tools/linux/current_handler_fixture_audit.py']
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
 with tempfile.TemporaryDirectory(prefix='c6-current-handler-integration-') as temp:
  work=Path(temp)
  packets=[]
  for stem,canonical in [('current_handler_interruption_metadata','current-handler-interruption-metadata-preparation'),('current_handler_field_guard','current-handler-field-guard-source'),('current_handler_caller','current-handler-caller-source')]:
   dest=work/(stem+'.json');subprocess.run([sys.executable,str(ROOT/'tools/linux'/f'{stem}_audit.py'),'--output',str(dest)],check=True,capture_output=True)
   packet=json.loads(dest.read_text());assert packet==json.loads((ROOT/'evidence/linux'/f'{canonical}.json').read_text());packets.append(digest(dest))
  subprocess.run(['cc','-std=gnu11','-Wall','-Wextra','-Werror','-fanalyzer','-fsyntax-only',*[str(ROOT/n) for n in SOURCES if n.endswith('.c')]],check=True,capture_output=True)
  runner=ROOT/'tools/linux/mod_activation_probe.py'
  for extra,reason in [(['--prepare-only','--current-handler-metadata'],'separate main-thread Core cleanup fixture'),(['--prepare-only','--current-handler-metadata-output','/tmp/not-created'],'paths require separate'),(['--prepare-only','--current-handler-metadata','--unit-operation-manager-metadata'],'not allowed with argument'),(['--current-handler-metadata'],'requires exact reviewed configuration')]:
   result=subprocess.run([sys.executable,str(runner),'--output',str(work/'never-created.json'),*extra],capture_output=True,text=True)
   assert result.returncode==2 and reason in result.stderr,(extra,result.returncode,result.stderr)
 runner_source=runner.read_text()
 for dependency in ['tools/linux/rules_launch_fixture.py','tools/assets/inventory.py']:
  assert "ROOT/'"+dependency+"'" in runner_source
 source=(ROOT/'native/linux/action_lifecycle.c').read_text()
 assert 'c6_linux_current_handler_metadata_caller_idle' in source
 assert '"c6_linux_current_handler_metadata_quiescent"' not in source
 probe=(ROOT/'native/linux/current_handler_metadata_probe.c').read_text()
 assert probe.index('main_tid()!=syscall(SYS_gettid)')<probe.index('void *facade=acquire(context)')
 assert probe.index('fields_end(module,context')<probe.index('release(facade)')
 report=dict(schema='c6-linux-current-handler-fixture-source-v1',native_execution=False,integrated_preparation_admission=False,
  source_sha256={n:digest(ROOT/n) for n in SOURCES},canonical_packet_sha256=packets,
  checks=dict(canonical_packets_equal=True,combined_strict_c_analyzer=True,parser_refusals=4,separate_locked_and_unlocked_callbacks=True),
  scope=dict(raw_passes=2,internal_observational_saves=4,outer_observational_saves=2,
   original_calls_in_helper_or_guard=False,Save_lock_continuity_claimed=False,full_manager_called=False,script_callbacks_called=False),
  pending=['actual frozen preload/config/source/archive and fresh paths','independent exact preparation admission','original facts/affected-state/RNG/terminal controls'])
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
if __name__=='__main__':main()
