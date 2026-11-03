#!/usr/bin/env python3
"""Origin-only gate for a separate fresh Load of the closed 85d Core output.

Reads existing artifacts only. Does not predict/produce the next native output,
execute original code, or change the older benchmark/ff31 selectors.
"""
import hashlib
import json
from pathlib import Path

PROFILE='captured-second'
CORE='85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8'
BYTES=80657462
GAME='a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST='106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'

def require(test,reason):
    if not test:raise ValueError(reason)

def digest(data):return hashlib.sha256(data).hexdigest()
def read_hashed(path,expected):
    data=path.read_bytes();require(digest(data)==expected,'captured-second origin artifact hash differs');return json.loads(data)

def validate(gate,runtime,summary):
    require(gate['schema']=='c6-linux-game-load-closed-output-v1' and gate['profile']==PROFILE,'captured-second gate schema/profile')
    require(gate['sha256']==CORE and gate['bytes']==BYTES,'captured-second input identity')
    cfg=runtime['configuration'];cfg_sha=digest(json.dumps(cfg,sort_keys=True,separators=(',',':')).encode())
    require(cfg_sha==runtime['configuration_sha256']==gate['origin_configuration_sha256'],'closed origin configuration identity')
    require(runtime['returncode']==0 and runtime['framework_cleanup_proven'] is True and runtime['client_cleanup_proven'] is True,'closed origin failed or cleanup unproven')
    stages=runtime['lifecycle_stages']
    require(all(stages.get(k) is True for k in ('native_context_initialized_observed','native_core_load_completed','game_startup_returned','game_shutdown_returned')),'closed original Load/Save lifetime incomplete')
    require(cfg['core_component_mode'] is None and cfg['bounded_action_mode'] is None and cfg['full_save_request'] is None and cfg['lower_core_military_initialization_observer']['mode']=='ordinary_load_save_passive','closed origin mode differs')
    require(cfg['lower_core_stream']['sha256']=='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884' and cfg['lower_core_stream_profile']=='captured-initial','closed origin incoming stream differs')
    saves=runtime['game_saves'];require(set(saves)=={'save','repeat'} and all((r['bytes'],r['sha256'])==(BYTES,CORE) for r in saves.values()),'closed output pair differs')
    require(summary['checks_passed'] is True and summary['original_capture_closed_exit_zero'] is True and summary['runtime_profile_sha256']==cfg_sha,'closed classification identity differs')
    require(summary['artifacts']['runtime.json']['sha256']==gate['origin_runtime_sha256'] and summary['gamecore_sha256']==GAME and summary['host_sha256']==HOST,'closed target/runtime identity differs')
    writes=[r for r in summary['streams'] if r['role']=='write']
    require(len(writes)==2 and all(r['core_sha256']==CORE and r['core_bytes']==BYTES and r['physical_lifetime_destroyed'] is True for r in writes),'closed writer/lifetime evidence differs')
    # Deliberately do not convert the observer's pending-attribution footer into
    # a success flag or require controller Uninitialize in this ordinary mode.
    return gate

def load_gate(root):
    root=Path(root);path=root/'tools/linux/game_load_second_capture_gate.json';gate=json.loads(path.read_text())
    runtime_path=(root/gate['origin_runtime']).resolve();summary_path=(root/gate['origin_summary']).resolve()
    require(runtime_path.is_relative_to((root.parent/'c6-ghidra').resolve()) and summary_path.is_relative_to(root.resolve()),'origin path outside declared repositories')
    runtime=read_hashed(runtime_path,gate['origin_runtime_sha256']);summary=read_hashed(summary_path,gate['origin_summary_sha256'])
    return validate(gate,runtime,summary)

def self_test(root):
    import copy
    import subprocess
    gate=load_gate(root);runtime=json.loads((root/gate['origin_runtime']).read_text());summary=json.loads((root/gate['origin_summary']).read_text())
    mutations=[lambda g,r,s:g.update(profile='captured-initial'),lambda g,r,s:g.update(sha256='0'*64),lambda g,r,s:g.update(bytes=1),lambda g,r,s:g.update(origin_configuration_sha256='0'*64),lambda g,r,s:r.update(returncode=1),lambda g,r,s:r.update(framework_cleanup_proven=False),lambda g,r,s:r.update(client_cleanup_proven=False),lambda g,r,s:r['lifecycle_stages'].update(game_shutdown_returned=False),lambda g,r,s:r['game_saves']['repeat'].update(sha256='0'*64),lambda g,r,s:s.update(gamecore_sha256='0'*64),lambda g,r,s:s.update(checks_passed=False),lambda g,r,s:s['streams'][1].update(physical_lifetime_destroyed=False)]
    for change in mutations:
        g,r,s=copy.deepcopy((gate,runtime,summary));change(g,r,s)
        try:validate(g,r,s)
        except ValueError:pass
        else:raise ValueError('corrupt origin accepted')
    source=(root/'tools/linux/mod_activation_probe.py').read_text().replace("choices=('benchmark','captured-initial','captured-second')","choices=('benchmark','captured-initial')",1).replace('    captured_extra_sources=[]\n','',1)
    begin=source.index("    if args.game_load_profile=='captured-second':");end=source.index('    if game_input:',begin);source=source[:begin]+source[end:]
    source=source.replace("    for path in captured_extra_sources:identity['sources'][str(path.relative_to(ROOT))]=digest(path)\n",'',1)
    old=subprocess.check_output(['git','show','394ddb0:tools/linux/mod_activation_probe.py'],cwd=root,text=True)
    require(source==old,'older runner branches changed')
    return dict(checks_passed=True,origin_corruption_refusals=len(mutations),full_older_runner_projection_equal=True,original_execution=False,next_output_generated=False)

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--self-test',action='store_true');args=p.parse_args();root=Path(__file__).resolve().parents[2]
    print(json.dumps(self_test(root) if args.self_test else load_gate(root),indent=2))
