#!/usr/bin/env python3
"""Identity-gated isolated original Linux mod-framework boundary; no standalone context."""
import argparse
import hashlib
import tempfile
import re
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--source-view',type=Path)
    parser.add_argument('--launch-host',type=Path)
    parser.add_argument('--saved-configuration',type=Path)
    parser.add_argument('--configure',action='store_true')
    parser.add_argument('--content-paths',action='store_true')
    parser.add_argument('--art-manager',action='store_true')
    parser.add_argument('--update-constructor-callback',action='store_true')
    parser.add_argument('--ui-constructor',action='store_true')
    parser.add_argument('--rules-snapshot-dir',type=Path)
    parser.add_argument('--network',action='store_true')
    parser.add_argument('--attach-saved-session',action='store_true')
    parser.add_argument('--game-load-stream',type=Path)
    parser.add_argument('--game-load-profile',choices=('benchmark','captured-initial'),default='benchmark')
    parser.add_argument('--game-load-observe',action='store_true')
    parser.add_argument('--action-observe',action='store_true')
    parser.add_argument('--action-execution-observe',action='store_true')
    parser.add_argument('--action-lifecycle-observe',action='store_true')
    action_modes=parser.add_mutually_exclusive_group()
    action_modes.add_argument('--action-skip-turn-control',action='store_true')
    action_modes.add_argument('--action-skip-turn',action='store_true')
    action_modes.add_argument('--action-skip-turn-registered-control',action='store_true')
    action_modes.add_argument('--action-skip-turn-registered',action='store_true')
    action_modes.add_argument('--core-component-control',action='store_true')
    action_modes.add_argument('--core-component',action='store_true')
    for name in ('before','after','repeat','command'):
        parser.add_argument('--core-component-'+name,type=Path)
    parser.add_argument('--action-snapshot-before',type=Path)
    parser.add_argument('--action-snapshot-after',type=Path)
    parser.add_argument('--action-snapshot-repeat',type=Path)
    parser.add_argument('--action-trace-directory',type=Path)
    parser.add_argument('--definition-ids',action='store_true')
    parser.add_argument('--unit-definition-ids',action='store_true')
    parser.add_argument('--operation-registry',action='store_true')
    parser.add_argument('--combat-kernel-input',type=Path)
    parser.add_argument('--combat-kernel-output',type=Path)
    parser.add_argument('--save-normalization-trace',type=Path)
    parser.add_argument('--request-load-save',type=Path)
    parser.add_argument('--game-save-output',type=Path)
    parser.add_argument('--game-save-repeat-output',type=Path)
    parser.add_argument('--timeout',type=int,default=30)
    parser.add_argument('--localization',action='store_true')
    parser.add_argument('--cleanup',action='store_true')
    parser.add_argument('--client-only',action='store_true')
    parser.add_argument('--client-constructor',action='store_true')
    parser.add_argument('--expected-configuration')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();assets=args.assets.resolve();view=args.source_view.resolve() if args.source_view else None
    if args.update_constructor_callback and not all((args.saved_configuration,args.configure,args.localization,args.content_paths,args.art_manager)):
        parser.error('--update-constructor-callback requires saved configuration, configure, localization, content paths and art manager')
    if args.ui_constructor and not args.localization:parser.error('--ui-constructor requires original localization')
    if args.attach_saved_session and not all((args.network,args.saved_configuration,args.cleanup)):
        parser.error('--attach-saved-session requires network, saved configuration and cleanup')
    if not 1<=args.timeout<=600:parser.error('--timeout must be between 1 and 600 seconds')
    game_input=args.game_load_stream.resolve() if args.game_load_stream else None
    action_mode='execute' if args.action_skip_turn or args.action_skip_turn_registered else 'control' if args.action_skip_turn_control or args.action_skip_turn_registered_control else None
    registered_action=args.action_skip_turn_registered or args.action_skip_turn_registered_control
    core_mode='execute' if args.core_component else 'control' if args.core_component_control else None
    core_paths={}
    if core_mode:
        if not game_input or any((args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.combat_kernel_input,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('Core component requires a separate fixture without host action or other diagnostic modes')
        for label in ('before','after','repeat','command'):
            path=getattr(args,'core_component_'+label)
            if path is None:parser.error('Core component requires all four fresh artifact paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('Core component artifacts must be new private files in existing directories')
            core_paths[label]=path
        if len(set(core_paths.values()))!=4:parser.error('Core component artifact paths must be distinct')
    elif any(getattr(args,'core_component_'+label) for label in ('before','after','repeat','command')):
        parser.error('Core component artifact paths require an explicit component mode')
    combat_input=args.combat_kernel_input.resolve() if args.combat_kernel_input else None
    combat_output=args.combat_kernel_output.resolve() if args.combat_kernel_output else None
    if bool(combat_input)!=bool(combat_output):parser.error('combat kernel requires both input and output paths')
    if combat_input:
        if not game_input or any((action_mode,core_mode,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('combat kernel requires a separate loaded Core fixture without other action/registry/serialization observers')
        if not combat_input.is_file():parser.error('combat input file is missing')
        if combat_output.exists() or not combat_output.parent.is_dir() or not combat_output.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):
            parser.error('combat output must be a new private c6-ghidra file in an existing directory')
    action_snapshots={}
    request_input=args.request_load_save.resolve() if args.request_load_save else None
    if request_input:
        if not args.cleanup or any((args.saved_configuration,args.configure,args.update_constructor_callback,args.network,args.attach_saved_session,game_input,args.rules_snapshot_dir)):
            parser.error('--request-load-save is a request-only cleanup fixture; do not combine restoration, activation, network, Core load or snapshots')
        if (not request_input.is_file() or request_input.stat().st_size!=7348717 or
            digest(request_input)!='a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8'):
            raise SystemExit('complete original save request identity mismatch')
    if args.game_load_observe and not game_input:parser.error('--game-load-observe requires --game-load-stream')
    if args.action_observe and not game_input:parser.error('--action-observe requires --game-load-stream')
    if sum((args.definition_ids,args.unit_definition_ids,args.operation_registry))>1:
        parser.error('registry observers require separate native fixture profiles')
    if (args.definition_ids or args.unit_definition_ids or args.operation_registry) and (not game_input or action_mode or core_mode or args.action_observe or args.action_execution_observe or args.action_lifecycle_observe or args.save_normalization_trace):
        parser.error('definition registry observers require a separate Core load fixture without actor/lifecycle/serialization instrumentation')
    if args.action_execution_observe and not args.action_observe:parser.error('--action-execution-observe requires --action-observe and its Core load fixture')
    action_trace=args.action_trace_directory.resolve() if args.action_trace_directory else None
    if action_mode:
        if not args.action_observe or args.action_execution_observe or args.action_lifecycle_observe or args.save_normalization_trace:
            parser.error('Skip Turn control/execute require a separate actor/Core fixture with the action trace')
        for label,path in (('before',args.action_snapshot_before),('after',args.action_snapshot_after),('repeat',args.action_snapshot_repeat)):
            if not path:parser.error('Skip Turn modes require all three action snapshot paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):
                parser.error('action snapshots must be new private c6-ghidra files in existing directories')
            action_snapshots[label]=path
        if len(set(action_snapshots.values()))!=3:parser.error('action snapshot paths must be distinct')
    elif args.action_snapshot_before or args.action_snapshot_after or args.action_snapshot_repeat:
        parser.error('action snapshots require an explicit Skip Turn control/execute mode')
    if args.action_lifecycle_observe or action_mode or core_mode:
        if (not args.action_observe and not core_mode) or args.action_execution_observe or not action_trace:
            parser.error('--action-lifecycle-observe requires actor observation and its own trace directory, separately from the phase-one execution observer')
        if not action_trace.is_dir() or any(action_trace.iterdir()) or not action_trace.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):
            parser.error('action trace directory must be a new empty existing private c6-ghidra directory')
    elif action_trace:parser.error('--action-trace-directory requires --action-lifecycle-observe')
    normalization_trace=args.save_normalization_trace.resolve() if args.save_normalization_trace else None
    if normalization_trace and (not game_input or normalization_trace.exists() or not normalization_trace.parent.is_dir() or
                                not normalization_trace.is_relative_to((ROOT.parent/'c6-ghidra').resolve())):
        parser.error('--save-normalization-trace requires Core load and a new file in an existing private c6-ghidra directory')
    game_outputs={}
    captured_gate=None
    captured_gate_path=None
    expected_game_size=76556938
    expected_game_sha='a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa'
    if args.game_load_profile=='captured-initial':
        if not game_input:parser.error('--game-load-profile requires --game-load-stream')
        captured_gate_path=ROOT/'tools/linux/game_load_captured_gate.json'
        captured_gate=json.loads(captured_gate_path.read_text())
        if captured_gate['schema']!='c6-linux-game-load-captured-input-v1' or captured_gate['profile']!='captured-initial':
            raise SystemExit('captured Core stream gate schema mismatch')
        origin=(ROOT/captured_gate['origin_report']).resolve()
        if not origin.is_relative_to(ROOT) or digest(origin)!=captured_gate['origin_report_sha256']:
            raise SystemExit('captured Core stream origin report mismatch')
        origin_report=json.loads(origin.read_text())
        origin_configuration=hashlib.sha256(json.dumps(origin_report['configuration'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
        if (origin_report['returncode']!=0 or not origin_report['framework_cleanup_proven'] or
            origin_report['configuration_sha256']!=origin_configuration or
            origin_configuration!=captured_gate['origin_configuration_sha256'] or
            any((save['bytes'],save['sha256'])!=(captured_gate['bytes'],captured_gate['sha256']) for save in origin_report['game_saves'].values()) or
            len(origin_report['game_saves'])!=2):
            raise SystemExit('captured Core stream origin proof mismatch')
        expected_game_size=captured_gate['bytes'];expected_game_sha=captured_gate['sha256']
    if game_input:
        if not all((args.attach_saved_session,args.update_constructor_callback,args.ui_constructor,args.game_save_output,args.game_save_repeat_output)):
            parser.error('--game-load-stream requires attached session, native update, UI constructor and both save output paths')
        if (not game_input.is_file() or game_input.stat().st_size!=expected_game_size or digest(game_input)!=expected_game_sha):
            raise SystemExit('lower Core stream identity mismatch')
        for label,path in (('save',args.game_save_output),('repeat',args.game_save_repeat_output)):
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):
                parser.error('game save outputs must be new files in an existing private c6-ghidra directory')
            game_outputs[label]=path
        if game_outputs['save']==game_outputs['repeat']:parser.error('game save outputs must be distinct')
        if set(game_outputs.values())&set(action_snapshots.values()):parser.error('action snapshots and outer game Saves must be distinct')
        if combat_output in game_outputs.values():parser.error('combat output and game Saves must be distinct')
        if set(core_paths.values())&set(game_outputs.values()):parser.error('Core component and outer Save paths must be distinct')
    elif args.game_save_output or args.game_save_repeat_output:
        parser.error('game save output paths require --game-load-stream')
    snapshot=args.rules_snapshot_dir.resolve() if args.rules_snapshot_dir else None
    if snapshot and (not args.update_constructor_callback or not snapshot.is_dir() or any(snapshot.iterdir()) or
                     not snapshot.is_relative_to((ROOT.parent/'c6-ghidra').resolve())):
        parser.error('--rules-snapshot-dir requires update callback and an existing empty private c6-ghidra directory')
    if not view and not args.launch_host:parser.error('--source-view or --launch-host required')
    gate=json.loads((ROOT/'tools/linux/runtime-gate.json').read_text())
    gate.update({'libGameCore_Base.so':'8ded6da8035e5e3d6940b5d1dde9d653b6f61947647704523dbc278bcc45bef6',
                 'libGameCore_XP1.so':'431f6e33f0fb5ac081016d14868cff60b3c3c2417acee82e75e23f24c466c80b'})
    for name,expected in gate.items():
        if digest(assets/name)!=expected:raise SystemExit(f'asset identity mismatch: {name}')
    state_audit=None
    state_audit_paths=[]
    if args.game_load_observe:
        from game_load_state_audit import audit
        state_audit=audit(assets/'libGameCore_XP2.so',ROOT.parent/'c6-ghidra/evidence/linux-action-boundary/abi.dwarf')
        state_audit_paths=[ROOT/'tools/linux/game_load_state_audit.py',ROOT/'tools/linux/inspect_elf.py',ROOT/'evidence/linux/game-load-state-facade-contracts.json']
        if state_audit!=json.loads(state_audit_paths[-1].read_text()):raise SystemExit('loaded-state facade contracts differ')
    request_audit=None
    request_audit_paths=[]
    if request_input:
        request_audit_paths=[ROOT/'tools/linux/save_lifecycle_request_audit.py',ROOT/'tools/linux/save_lifecycle_audit.py',ROOT/'tools/linux/inspect_elf.py',ROOT/'evidence/linux/save-lifecycle-request-contracts.json']
        with tempfile.TemporaryDirectory(prefix='c6-request-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(request_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            request_audit=json.loads(result_path.read_text())
        if request_audit!=json.loads(request_audit_paths[-1].read_text()):raise SystemExit('original request contracts differ')
    action_audit=None
    action_audit_paths=[]
    if args.action_observe or core_mode:
        action_audit_paths=[ROOT/'tools/linux/action_observer_audit.py',ROOT/'tools/linux/save_lifecycle_audit.py',ROOT/'tools/linux/inspect_elf.py',ROOT/'evidence/linux/action-observer-abi.json']
        with tempfile.TemporaryDirectory(prefix='c6-action-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(action_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            action_audit=json.loads(result_path.read_text())
        if action_audit!=json.loads(action_audit_paths[-1].read_text()):raise SystemExit('original action observer contracts differ')
    normalization_audit=None
    normalization_audit_paths=[]
    if normalization_trace:
        from save_normalization_audit import audit
        normalization_audit=audit(assets/'libGameCore_XP2.so',ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/serialization-write-functions.dwarf.txt')
        normalization_audit_paths=[ROOT/'tools/linux/save_normalization_audit.py',ROOT/'tools/linux/inspect_elf.py',ROOT/'tools/linux/save_normalization_owners.inc',ROOT/'evidence/linux/save-normalization-trace-contracts.json']
        if normalization_audit!=json.loads(normalization_audit_paths[-1].read_text()):raise SystemExit('original serialization trace contracts differ')
    execution_audit=None
    execution_audit_paths=[]
    if args.action_execution_observe or args.action_lifecycle_observe or action_mode or core_mode:
        execution_audit_paths=[ROOT/'tools/linux/action_execution_audit.py',ROOT/'tools/linux/save_lifecycle_audit.py',ROOT/'tools/linux/inspect_elf.py',ROOT/'evidence/linux/action-execution-abi.json']
        with tempfile.TemporaryDirectory(prefix='c6-execution-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(execution_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            execution_audit=json.loads(result_path.read_text())
        if execution_audit!=json.loads(execution_audit_paths[-1].read_text()):raise SystemExit('original cache dispatch contracts differ')
    dispatch_audits={}
    dispatch_audit_paths=[]
    if action_mode or args.action_lifecycle_observe or core_mode:
        for stem in ('action_dispatch','action_state','action_delivery','action_registration'):
            tool=ROOT/f'tools/linux/{stem}_audit.py'
            canonical=ROOT/f'evidence/linux/{stem.replace("_","-")}-abi.json'
            with tempfile.TemporaryDirectory(prefix='c6-dispatch-audit-') as temporary:
                result_path=Path(temporary)/'contracts.json'
                subprocess.run([sys.executable,str(tool),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
                result=json.loads(result_path.read_text())
            if result!=json.loads(canonical.read_text()):raise SystemExit('original dispatch/state contracts differ')
            dispatch_audits[stem]=result;dispatch_audit_paths.extend((tool,canonical))
    definition_audit=None
    definition_audit_paths=[]
    if args.definition_ids:
        definition_audit_paths=[ROOT/'tools/linux/definition_ids_audit.py',ROOT/'evidence/linux/definition-ids-static.json']
        with tempfile.TemporaryDirectory(prefix='c6-definition-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(definition_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            definition_audit=json.loads(result_path.read_text())
        if definition_audit!=json.loads(definition_audit_paths[-1].read_text()):raise SystemExit('original definition registry contracts differ')
    unit_definition_audit=None
    unit_definition_audit_paths=[]
    if args.unit_definition_ids:
        unit_definition_audit_paths=[ROOT/'tools/linux/unit_definition_ids_audit.py',ROOT/'evidence/linux/unit-definition-ids-static.json']
        with tempfile.TemporaryDirectory(prefix='c6-unit-definition-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(unit_definition_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            unit_definition_audit=json.loads(result_path.read_text())
        if unit_definition_audit!=json.loads(unit_definition_audit_paths[-1].read_text()):raise SystemExit('original Unit definition registry contracts differ')
    operation_registry_audit=None
    operation_registry_audit_paths=[]
    if args.operation_registry:
        from operation_registry_audit import contract
        operation_registry_audit_paths=[ROOT/'tools/linux/operation_registry_audit.py',ROOT/'evidence/linux/operation-registry-abi.json']
        operation_registry_audit=contract(assets)[0]
        if operation_registry_audit!=json.loads(operation_registry_audit_paths[-1].read_text()):raise SystemExit('original operation registry contracts differ')
    combat_audit_paths=[]
    combat_profile=None
    if combat_input:
        from combat_kernel_audit import audit
        combat_audit_paths=[ROOT/'tools/linux/combat_kernel_audit.py',ROOT/'evidence/linux/combat-kernel-contracts.json',ROOT/'evidence/linux/combat-kernel-input.json']
        manifest=json.loads(combat_audit_paths[-1].read_text())
        if manifest['schema']!='c6-linux-xp2-average-damage-input-v1' or manifest['use_average'] is not True or digest(combat_input)!=manifest['input_sha256'] or combat_input.stat().st_size!=manifest['input_bytes']:
            raise SystemExit('combat input manifest identity mismatch')
        for name in ('native/linux/combat_kernel.c','native/linux/combat_kernel.h','tools/linux/combat_kernel_audit.py'):
            if digest(ROOT/name)!=manifest['source_sha256'][name]:raise SystemExit('combat native source identity mismatch')
        with tempfile.TemporaryDirectory(prefix='combat-audit-',dir=ROOT.parent/'c6-ghidra/evidence') as temporary:
            contracts=audit(assets/'libGameCore_XP2.so',Path(temporary))
        if contracts!=json.loads(combat_audit_paths[-2].read_text()):raise SystemExit('combat original contracts differ')
        combat_profile={'input_sha256':manifest['input_sha256'],'input_bytes':manifest['input_bytes'],'case_count':manifest['case_count'],'use_average':True,'contract':contracts}
    core_contract=None
    core_audit_paths=[]
    if core_mode:
        core_audit_paths=[ROOT/'tools/linux/core_component_audit.py',ROOT/'evidence/linux/core-component-static.json']
        with tempfile.TemporaryDirectory(prefix='core-component-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(core_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            core_contract=json.loads(result_path.read_text())
        if core_contract!=json.loads(core_audit_paths[-1].read_text()):raise SystemExit('original Core component contracts differ')
    content=json.loads((ROOT/'tools/linux/runtime-content-gate.json').read_text())['files']
    if view:
        profile=json.loads((view/'source-view.json').read_text())
        mapping=[{'source':name,'staged':str(Path(*('Schema' if part=='schema' else part for part in Path(name).parts))),
                  'sha256':expected} for name,expected in sorted(content.items())]
        if profile['mapping']!=mapping:raise SystemExit('source-view mapping mismatch')
        for row in mapping:
            source=assets/row['source'];staged=view/row['staged']
            if digest(source)!=row['sha256'] or not staged.is_symlink() or staged.resolve()!=source.resolve():
                raise SystemExit('source view identity mismatch')
    host=assets/'Civ6'
    filesystem_sources=[]
    filesystem_identity=None
    if args.launch_host:
        host=args.launch_host.absolute()
        if host.is_symlink() or host.resolve().is_relative_to(assets) or digest(host)!=gate['Civ6']:
            raise SystemExit('copied launch host identity mismatch')
        from rules_launch_fixture import validate
        launch_profile=validate(assets,host.parent)
        metadata=json.loads((ROOT/'tools/linux/filesystem_gate.json').read_text())
        full_inputs=json.loads((ROOT/'evidence/linux/rules-content-inputs.json').read_text())
        for profile_inputs in (metadata['files'],full_inputs['files']):
            for name,expected in profile_inputs.items():
                if digest(assets/name)!=expected:raise SystemExit('native content input mismatch')
        filesystem_sources=[ROOT/'native/linux/filesystem.c']
        filesystem_identity={'launch_host':str(host),'launch_fixture_sha256':digest(host.parent/'launch-fixture.json'),'validated_launch':launch_profile,
                             'metadata_gate_sha256':digest(ROOT/'tools/linux/filesystem_gate.json'),
                             'full_content_input_gate_sha256':digest(ROOT/'evidence/linux/rules-content-inputs.json')}
    sources=[ROOT/'native/linux/mod_activation.c',ROOT/'native/linux/mod_activation.h',
             ROOT/'native/linux/host_services.c',ROOT/'native/linux/host_services.h',
             ROOT/'native/linux/temp_heap.c',ROOT/'native/linux/temp_heap.h']
    snapshot_sources=[ROOT/'native/linux/rules_snapshot.c'] if snapshot else []
    network_sources=[ROOT/'native/linux/network.c'] if args.network else []
    lifecycle_sources=[ROOT/'native/linux/save_lifecycle.c'] if args.attach_saved_session else []
    game_sources=[ROOT/'native/linux/game_load.c',ROOT/'native/linux/context_stream.c'] if game_input else []
    request_sources=[ROOT/'native/linux/request_load.c'] if request_input else []
    action_sources=[ROOT/'native/linux/action_observer.c'] if args.action_observe or core_mode else []
    normalization_sources=[ROOT/'tools/linux/save_normalization_trace.c'] if normalization_trace else []
    execution_sources=[ROOT/'native/linux/action_execution.c'] if args.action_execution_observe or args.action_lifecycle_observe or action_mode or core_mode else []
    lifecycle_observer_sources=[ROOT/'native/linux/action_lifecycle.c',ROOT/'tools/linux/action_execution_trace.c'] if args.action_lifecycle_observe or action_mode or core_mode else []
    dispatch_sources=[ROOT/'native/linux/action_dispatch.c',ROOT/'native/linux/action_state.c',ROOT/'native/linux/action_registration.c'] if action_mode or args.action_lifecycle_observe or core_mode else []
    definition_sources=[ROOT/'native/linux/definition_ids.c'] if args.definition_ids else []
    unit_definition_sources=[ROOT/'native/linux/unit_definition_ids.c'] if args.unit_definition_ids else []
    operation_registry_sources=[ROOT/'native/linux/operation_registry.c'] if args.operation_registry else []
    combat_sources=[ROOT/'native/linux/combat_kernel.c'] if combat_input else []
    core_sources=[ROOT/'native/linux/core_component.c'] if core_mode else []
    configuration_sources=[]
    if args.saved_configuration:
        if digest(args.saved_configuration)!='a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8':raise SystemExit('saved configuration identity mismatch')
        configuration_sources=[ROOT/'native/linux/saved_configuration.c']
    localization_sources=[]
    localization_profile=None
    localization_environment={}
    if args.localization:
        from localization_gate import prepare
        localization_profile,localization_environment=prepare(assets,ROOT/'target/linux-mod-localization')
        localization_sources=[ROOT/'native/linux/localization.c']
    # Full descriptor inventory gates original discovery inputs; data actions are
    # not yet invoked by this init/configure-only probe.
    descriptors={str(p.relative_to(assets)):digest(p) for p in sorted((assets/'DLC').rglob('*.modinfo'))}
    work=ROOT/'target/linux/mod-activation';work.mkdir(parents=True,exist_ok=True)
    lock=(work/'probe.lock').open('w');fcntl.flock(lock,fcntl.LOCK_EX)
    userdata=Path(tempfile.mkdtemp(prefix='userdata-',dir=work))
    preload=work/'mod_activation_probe.so'
    identity={'schema':'c6-linux-mod-activation-v1','assets':gate,'base_content_gate_sha256':digest(ROOT/'tools/linux/runtime-content-gate.json'),
              'update_callback_profile':'native LoadGameViewState constructor only; requestNone and constructor-default censorship=false; no saved Request/Begin, not saved-game parity' if args.update_constructor_callback else None,
              'ui_constructor_only':args.ui_constructor,
              'native_rules_snapshot_requested':bool(snapshot),
              'native_network_requested':args.network,'native_saved_session_control_requested':args.attach_saved_session,
              'lower_core_stream':{'sha256':digest(game_input),'bytes':game_input.stat().st_size} if game_input else None,
              'lower_core_stream_profile':args.game_load_profile if game_input else None,
              'captured_core_stream_gate':captured_gate,
              'lower_core_state_observer':state_audit,
              'lower_core_actor_legality_observer':action_audit,
              'save_normalization_trace':normalization_audit,
              'lower_core_cache_dispatch_observer':execution_audit,
              'lower_core_controller_cache_lifecycle_observer':args.action_lifecycle_observe,
              'bounded_action_mode':action_mode,'pre_controller_registration':registered_action,'bounded_action_audits':dispatch_audits,
              'core_component_mode':core_mode,'core_component_contract':core_contract,
              'lower_core_definition_registry_observer':definition_audit,
              'lower_core_unit_definition_registry_observer':unit_definition_audit,
              'lower_core_operation_registry_observer':operation_registry_audit,
              'lower_core_average_damage_kernel':combat_profile,
              'full_save_request':{'sha256':digest(request_input),'bytes':request_input.stat().st_size,'profile':'supplied-stream device4; both real game-state getters NULL; original deferred request only'} if request_input else None,
              'full_save_request_audit':request_audit,
              'timeout_seconds':args.timeout,
              'source_view_manifest_sha256':digest(view/'source-view.json') if view else None,'dlc_descriptors':descriptors,
              'sources':{str(p.relative_to(ROOT)):digest(p) for p in sources},'configure_requested':args.configure,'content_paths_requested':args.content_paths,'art_manager_requested':args.art_manager,'localization_profile':localization_profile,'cleanup_requested':args.cleanup,'client_constructor_requested':args.client_constructor,'client_only':args.client_only,
              'source_view':str(view) if view else None,'userdata_policy':'new empty private task directory','userdata_initially_empty':True,'cwd':str(work),
              'runner_sha256':digest(Path(__file__)),'original_filesystem_profile':filesystem_identity,
              'saved_configuration_sha256':digest(args.saved_configuration) if args.saved_configuration else None}
    if configuration_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*configuration_sources,ROOT/'native/linux/saved_configuration.h']})
    if snapshot_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*snapshot_sources,ROOT/'native/linux/rules_snapshot.h']})
    if network_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*network_sources,ROOT/'native/linux/network.h']})
    if lifecycle_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*lifecycle_sources,ROOT/'native/linux/save_lifecycle.h']})
    if game_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*game_sources,ROOT/'native/linux/game_load.h',ROOT/'native/linux/context_stream.h']})
    if request_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*request_sources,ROOT/'native/linux/request_load.h']})
    if action_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*action_sources,ROOT/'native/linux/action_observer.h']})
    for path in normalization_sources:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    if execution_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*execution_sources,ROOT/'native/linux/action_execution.h']})
    if lifecycle_observer_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*lifecycle_observer_sources,ROOT/'native/linux/action_lifecycle.h']})
    if definition_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*definition_sources,ROOT/'native/linux/definition_ids.h']})
    if unit_definition_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*unit_definition_sources,ROOT/'native/linux/unit_definition_ids.h']})
    if operation_registry_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*operation_registry_sources,ROOT/'native/linux/operation_registry.h']})
    if combat_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*combat_sources,ROOT/'native/linux/combat_kernel.h']})
    if core_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*core_sources,ROOT/'native/linux/core_component.h']})
    if dispatch_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*dispatch_sources,ROOT/'native/linux/action_dispatch.h',ROOT/'native/linux/action_state.h',ROOT/'native/linux/action_registration.h']})
    if captured_gate_path:identity['sources'][str(captured_gate_path.relative_to(ROOT))]=digest(captured_gate_path)
    for path in state_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in request_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in action_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in normalization_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in execution_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in [*definition_audit_paths,*unit_definition_audit_paths,*operation_registry_audit_paths,*combat_audit_paths,*core_audit_paths]:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in dispatch_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    if localization_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*localization_sources,ROOT/'native/linux/localization.h',ROOT/'tools/linux/localization_gate.py']})
    if filesystem_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*filesystem_sources,ROOT/'native/linux/filesystem.h']})
    subprocess.run(['cc','-shared','-fPIC','-Wall','-Wextra','-Werror','-DC6_MOD_ACTIVATION_PROBE_MAIN',
                    str(sources[0]),str(sources[2]),str(sources[4]),*map(str,[*filesystem_sources,*configuration_sources,*localization_sources,*snapshot_sources,*network_sources,*lifecycle_sources,*game_sources,*request_sources,*action_sources,*normalization_sources,*execution_sources,*lifecycle_observer_sources,*definition_sources,*unit_definition_sources,*operation_registry_sources,*combat_sources,*core_sources,*dispatch_sources]),'-ldl','-o',str(preload)],check=True)
    identity['preload_sha256']=digest(preload)
    archived_preload=None
    if core_mode:
        archived_preload=core_paths['command'].parent/'core-component-preload.so'
        with archived_preload.open('xb') as stream:stream.write(preload.read_bytes())
    dependency_env=dict(os.environ,LD_LIBRARY_PATH=f'{ROOT}/target/linux/sysroot/usr/lib/x86_64-linux-gnu:{assets}')
    listing=subprocess.check_output(['ldd',str(host)],env=dependency_env,text=True)
    if 'not found' in listing:raise SystemExit('native runtime dependency missing')
    identity['runtime_dependencies']={}
    for line in listing.splitlines():
        match=re.search(r'(?:=>\s+)?(/\S+)\s+\(',line)
        if match:
            path=Path(match[1]).resolve();identity['runtime_dependencies'][path.name]={'path':str(path),'sha256':digest(path)}
    if any(digest(ROOT/p)!=expected for p,expected in identity['sources'].items()):raise SystemExit('source changed during compile')
    env={k:v for k,v in os.environ.items() if not k.startswith(('LD_','C6_'))}
    env.update({'LD_PRELOAD':str(preload),'LD_LIBRARY_PATH':f'{ROOT}/target/linux/sysroot/usr/lib/x86_64-linux-gnu:{assets}',
                'C6_HOST_ASSETS':str(assets),'C6_RULES_SOURCE_ROOT':str(view) if view else str(host.parent/'steamassets'),'C6_HOST_USERDATA':str(userdata),'C6_HOST_LUA':'1'})
    if args.launch_host:env['C6_MOD_FILESYSTEM']='1'
    if args.saved_configuration:env['C6_SAVED_CONFIGURATION']=str(args.saved_configuration.resolve())
    if args.cleanup:env['C6_MOD_CLEANUP']='1'
    if args.localization:
        env.update(localization_environment);env['C6_LOCALIZATION']='1'
    if args.configure:env['C6_MOD_CONFIGURE']='1'
    if args.content_paths:env['C6_MOD_CONTENT_PATHS']='1'
    if args.art_manager:env['C6_MOD_ART_MANAGER']='1'
    if args.update_constructor_callback:env['C6_MOD_UPDATE_CONSTRUCTOR_CALLBACK']='1'
    if args.ui_constructor:env['C6_MOD_UI_CONSTRUCTOR']='1'
    if snapshot:env['C6_RULES_SNAPSHOT_DIR']=str(snapshot)
    if args.network:env['C6_HOST_NETWORK']='1'
    if args.attach_saved_session:env['C6_MOD_ATTACH_SAVED_SESSION']='1'
    if request_input:env['C6_REQUEST_LOAD_SAVE']=str(request_input)
    if normalization_trace:env['C6_SAVE_NORMALIZATION_TRACE']=str(normalization_trace)
    if game_input:
        env['C6_GAME_LOAD_STREAM']=str(game_input)
        env['C6_GAME_SAVE_OUTPUT']=str(game_outputs['save'])
        env['C6_GAME_SAVE_REPEAT_OUTPUT']=str(game_outputs['repeat'])
        if captured_gate:env['C6_GAME_LOAD_EXPECTED_SIZE']=str(expected_game_size)
        if args.game_load_observe:env['C6_GAME_LOAD_OBSERVE']='1'
        if args.action_observe:env['C6_ACTION_OBSERVE']='1'
        if args.definition_ids:env['C6_DEFINITION_IDS']='1'
        if args.unit_definition_ids:env['C6_UNIT_DEFINITION_IDS']='1'
        if args.operation_registry:env['C6_OPERATION_REGISTRY']='1'
        if combat_input:env.update(C6_COMBAT_KERNEL_INPUT=str(combat_input),C6_COMBAT_KERNEL_OUTPUT=str(combat_output))
        if args.action_execution_observe:env['C6_ACTION_EXECUTION']='observe_cache'
        if args.action_lifecycle_observe:
            env['C6_ACTION_LIFECYCLE']='observe_cache_initialization'
            env['C6_ACTION_TRACE_DIRECTORY']=str(action_trace)
        if action_mode:
            env['C6_ACTION_SKIP_TURN']=action_mode
            if registered_action:env['C6_ACTION_REGISTER_PRECONTROLLER']='1'
            env['C6_ACTION_TRACE_DIRECTORY']=str(action_trace)
            for label,path in action_snapshots.items():env[f'C6_ACTION_SNAPSHOT_{label.upper()}']=str(path)
    if core_mode:
        env['C6_CORE_COMPONENT_MODE']=core_mode
        env['C6_ACTION_TRACE_DIRECTORY']=str(action_trace)
        for label,path in core_paths.items():env['C6_CORE_COMPONENT_'+('COMMAND' if label=='command' else 'SNAPSHOT_'+label.upper())]=str(path)
    if args.client_only:env['C6_MOD_CLIENT_ONLY']='1'
    if args.client_constructor:env['C6_MOD_CLIENT_CONSTRUCTOR']='1'
    configuration_identity=hashlib.sha256(json.dumps(identity,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    if args.expected_configuration and args.expected_configuration!=configuration_identity:
        raise SystemExit('configuration identity mismatch; native execution refused')
    try:
        run=subprocess.run([str(host)],cwd=work,env=env,text=True,capture_output=True,timeout=args.timeout)
        code,out,err=run.returncode,run.stdout,run.stderr
    except subprocess.TimeoutExpired as exc:
        decode=lambda x:x.decode(errors='replace') if isinstance(x,bytes) else x or ''
        code,out,err=124,decode(exc.stdout),decode(exc.stderr)
    report={'configuration':identity,'configuration_sha256':configuration_identity,
            'execution_paths':{'userdata':str(userdata),'native_rules_snapshot_directory':str(snapshot) if snapshot else None,
                               'lower_core_stream':str(game_input) if game_input else None,'game_outputs':{k:str(v) for k,v in game_outputs.items()},
                               'full_save_request':str(request_input) if request_input else None},
            'returncode':code,'stdout':out,'stderr':err,'framework_boundary_returned':code==0 and 'boundary_return result 0' in err,
            'framework_cleanup_proven':code==0 and 'cleanup_return 0' in err,'client_cleanup_proven':code==0 and 'client_shutdown_only_return unpublished 1' in err,'xp2_activation_proven':False,'initialized_game':False,'gameplay_parity':False,
            'limits':'Original host content-transition diagnostic only; selected module path and actual active-vtable module are distinct observations. Constructor-state callback is not saved Request/Begin semantics. No initialized-game or gameplay parity proof. Teardown is measured only when requested and its marker is present.'}
    report['native_activation_selection']={}
    report['native_backup_events']=[]
    for line in err.splitlines():
        if line.startswith('C6_LINUX_MOD selection '):
            selection=json.loads(line.removeprefix('C6_LINUX_MOD selection '))
            report['native_activation_selection'].setdefault(selection['phase'],{})[selection['list']]=selection
        if line.startswith('C6_LINUX_RULES '):
            report['native_backup_events'].append(json.loads(line.removeprefix('C6_LINUX_RULES ')))
    report['rules_snapshots']={}
    report['game_saves']={}
    report['action_saves']={}
    if action_snapshots:
        report['execution_paths']['action_snapshots']={k:str(v) for k,v in action_snapshots.items()}
        for label,path in action_snapshots.items():
            if path.is_file():report['action_saves'][label]={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
    report['action_trace_files']={}
    if action_trace:
        report['execution_paths']['action_trace_directory']=str(action_trace)
        for path in sorted(action_trace.iterdir()):
            if path.is_file():report['action_trace_files'][path.name]={'sha256':digest(path),'bytes':path.stat().st_size}
    report['core_component_saves']={}
    report['core_component_command']=None
    if core_mode:
        report['execution_paths']['core_component']={k:str(v) for k,v in core_paths.items()}
        report['execution_paths']['preload']=str(archived_preload)
        for label,path in core_paths.items():
            if path.is_file():
                artifact={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
                if label=='command':report['core_component_command']=artifact
                else:report['core_component_saves'][label]=artifact
    report['combat_kernel_output']=None
    if combat_input:
        report['execution_paths'].update(combat_kernel_input=str(combat_input),combat_kernel_output=str(combat_output))
        if combat_output.is_file():report['combat_kernel_output']={'filename':combat_output.name,'sha256':digest(combat_output),'bytes':combat_output.stat().st_size}
    report['save_normalization_trace']=None
    if normalization_trace:
        report['execution_paths']['save_normalization_trace']=str(normalization_trace)
        if normalization_trace.is_file():report['save_normalization_trace']={'filename':normalization_trace.name,'sha256':digest(normalization_trace),'bytes':normalization_trace.stat().st_size}
    for label,path in game_outputs.items():
        if path.is_file():report['game_saves'][label]={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
    if snapshot:
        for label in ('configuration','gameplay'):
            path=snapshot/f'{label}.sqlite'
            if path.is_file():
                with path.open('rb') as stream:header=stream.read(16).hex()
                report['rules_snapshots'][label]={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size,
                    'header_hex':header,'source':'original App-owned connection getter; native Database::Connection::BackupTo'}
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    tail=[line for line in err.splitlines() if not line.startswith('C6_LINUX_MOD selection ')][-12:]
    print(json.dumps({'output':str(args.output),'returncode':code,'tail':tail},indent=2))

if __name__=='__main__':main()
