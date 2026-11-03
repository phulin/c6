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

def lifecycle_report(code, stderr):
    """Measured marker stages stay distinct from the overall fixture result."""
    boundary=re.findall(r'^C6_LINUX_MOD boundary_return result (-?\d+)(?: |$)',stderr,re.M)
    cleanup=re.findall(r'^C6_LINUX_MOD cleanup_return (-?\d+)(?: |$)',stderr,re.M)
    contexts=re.findall(r'^C6_LINUX_GAME_LOAD module .+ context_initialized ([01]) host_finalization [01](?: |$)',stderr,re.M)
    loads=re.findall(r'^C6_LINUX_STREAM load_return result (-?\d+) offset (\d+) size (\d+)(?: |$)',stderr,re.M)
    controller=re.findall(r'^C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized ([01]) context_bound ([01]) executing ([01]) cleanup ([01])(?: |$)',stderr,re.M)
    result=int(boundary[-1]) if boundary else None
    cleanup_result=int(cleanup[-1]) if cleanup else None
    load=tuple(map(int,loads[-1])) if loads else None
    loaded=bool(load and load[0]==0 and load[1]==load[2] and load[2]>0)
    initialized=bool(contexts and contexts[-1]=='1')
    client=bool(re.search(r'^C6_LINUX_MOD client_shutdown_only_return unpublished 1(?: |$)',stderr,re.M))
    stages={'native_context_initialized_observed':initialized,'native_core_load_completed':loaded,
            'native_core_load_result':load[0] if load else None,
            'game_startup_returned':bool(re.search(r'^C6_LINUX_GAME_LOAD startup_return(?: |$)',stderr,re.M)),
            'game_shutdown_returned':bool(re.search(r'^C6_LINUX_GAME_LOAD shutdown_return(?: |$)',stderr,re.M)),
            'controller_cache_initialized':bool(re.search(r'^C6_LINUX_ACTION_LIFECYCLE initialize_cache_return(?: |$)',stderr,re.M)),
            'controller_uninitialized_cleanly':bool(controller and controller[-1]==('0','0','0','1')),
            'framework_cleanup_result':cleanup_result,'client_unpublished_observed':client}
    return {'fixture_success':code==0 and result==0,'native_boundary_result':result,
            'framework_boundary_returned':result is not None,
            'framework_cleanup_proven':cleanup_result==0,'client_cleanup_proven':client,
            'initialized_game':initialized and loaded,
            'initialized_game_scope':'observed native Core context and complete stream Load only; not complete live-state coverage or gameplay parity',
            'lifecycle_stages':stages}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--source-view',type=Path)
    parser.add_argument('--launch-host',type=Path)
    parser.add_argument('--prepare-main-thread',action='store_true')
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
    parser.add_argument('--game-load-profile',choices=('benchmark','captured-initial','captured-second'),default='benchmark')
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
    action_modes.add_argument('--movement-cost-control',action='store_true')
    action_modes.add_argument('--movement-cost',action='store_true')
    action_modes.add_argument('--domain-metadata',action='store_true')
    action_modes.add_argument('--placement-metadata',action='store_true')
    action_modes.add_argument('--military-initialization-observer',action='store_true')
    parser.add_argument('--military-observer-plan',type=Path)
    parser.add_argument('--military-observer-directory',type=Path)
    action_modes.add_argument('--current-handler-metadata',action='store_true')
    for label in ('output','repeat-output','control-output','before','control-save','after','repeat'):
        parser.add_argument('--current-handler-metadata-'+label,type=Path)
    action_modes.add_argument('--unit-operation-manager-metadata',action='store_true')
    parser.add_argument('--unit-operation-manager-candidate-manifest',type=Path)
    for label in ('output','repeat-output','control-output','before','control-save','after','repeat'):
        parser.add_argument('--unit-operation-manager-metadata-'+label,type=Path)
    action_modes.add_argument('--placement-handler-metadata',action='store_true')
    parser.add_argument('--placement-handler-candidate-manifest',type=Path)
    for label in ('output','expected-output','handler-expected-output','before','control-save','after','repeat'):
        parser.add_argument('--placement-handler-metadata-'+label,type=Path)
    parser.add_argument('--placement-metadata-candidate-manifest',type=Path)
    for label in ('output','expected-output','before','control-save','after','repeat'):
        parser.add_argument('--placement-metadata-'+label,type=Path)
    parser.add_argument('--domain-metadata-reconstruction-manifest',type=Path)
    for label in ('output','before','control-save','after','repeat'):
        parser.add_argument('--domain-metadata-'+label,type=Path)
    for label in ('output','before','control-save','after','repeat'):
        parser.add_argument('--movement-cost-'+label,type=Path)
    parser.add_argument('--movement-cost-max-formations',type=int,default=256)
    parser.add_argument('--movement-cost-diagnostic',action='store_true')
    parser.add_argument('--movement-cost-xp2-map',action='store_true')
    parser.add_argument('--prepare-only',action='store_true')
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
    for label in ('input','output','control-save','post-save'):
        parser.add_argument('--combat-random-'+label,type=Path)
    parser.add_argument('--save-normalization-trace',type=Path)
    parser.add_argument('--request-load-save',type=Path)
    parser.add_argument('--game-save-output',type=Path)
    parser.add_argument('--game-save-repeat-output',type=Path)
    parser.add_argument('--timeout',type=int,default=30)
    parser.add_argument('--movement-cost-timing-baseline',type=Path)
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
    if not 1<=args.timeout<=900:parser.error('--timeout must be between 1 and 900 seconds')
    game_input=args.game_load_stream.resolve() if args.game_load_stream else None
    action_mode='execute' if args.action_skip_turn or args.action_skip_turn_registered else 'control' if args.action_skip_turn_control or args.action_skip_turn_registered_control else None
    registered_action=args.action_skip_turn_registered or args.action_skip_turn_registered_control
    core_mode='execute' if args.core_component else 'control' if args.core_component_control else None
    movement_mode='execute' if args.movement_cost else 'control' if args.movement_cost_control else None
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
    random_input=args.combat_random_input.resolve() if args.combat_random_input else None
    random_paths={}
    if random_input:
        if not game_input or any((action_mode,core_mode,combat_input,args.game_load_observe,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('random damage requires a separate loaded Core fixture without other diagnostic modes')
        if not random_input.is_file():parser.error('random damage input is missing')
        for label in ('output','control_save','post_save'):
            path=getattr(args,'combat_random_'+label)
            if path is None:parser.error('random damage requires output, control-save and post-save paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('random damage artifacts must be new private files')
            random_paths[label]=path
        if len(set(random_paths.values()))!=3:parser.error('random damage artifact paths must be distinct')
    elif any((args.combat_random_output,args.combat_random_control_save,args.combat_random_post_save)):
        parser.error('random damage output paths require its input')
    if args.movement_cost_diagnostic and args.movement_cost_xp2_map:parser.error('movement probe revisions must be separate')
    if args.movement_cost_xp2_map and not movement_mode:parser.error('XP2 movement probe revision requires a movement mode')
    if args.movement_cost_diagnostic and movement_mode!='control':parser.error('movement admission diagnostic requires fact-only control mode')
    if args.movement_cost_timing_baseline and (movement_mode!='execute' or args.timeout!=900):parser.error('movement timing baseline requires cost mode and900s supervisor')
    movement_paths={}
    if movement_mode:
        if not game_input or not args.prepare_main_thread or any((combat_input,random_input,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('movement cost requires a separate loaded Core fixture with original main-thread preparation')
        if not 1<=args.movement_cost_max_formations<=4096:parser.error('movement formation limit must be1..4096')
        args.game_load_observe=True
        for label in ('output','before','control_save','after','repeat'):
            path=getattr(args,'movement_cost_'+label)
            if path is None:parser.error('movement cost requires output and all four snapshot paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('movement artifacts must be fresh private files in existing directories')
            movement_paths[label]=path
        if len(set(movement_paths.values()))!=5:parser.error('movement artifact paths must be distinct')
    elif any(getattr(args,'movement_cost_'+label) for label in ('output','before','control_save','after','repeat')):
        parser.error('movement artifact paths require a movement mode')
    domain_paths={}
    if args.domain_metadata:
        if not game_input or not args.prepare_main_thread or not args.cleanup or not args.domain_metadata_reconstruction_manifest or any((combat_input,random_input,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('domain metadata requires a separate prepared-main-thread loaded Core cleanup fixture and frozen reconstruction manifest')
        args.game_load_observe=True
        for label in ('output','before','control_save','after','repeat'):
            path=getattr(args,'domain_metadata_'+label)
            if path is None:parser.error('domain metadata requires output and all four snapshot paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('domain metadata artifacts must be fresh private files')
            domain_paths[label]=path
        if len(set(domain_paths.values()))!=5:parser.error('domain metadata artifact paths must be distinct')
    elif args.domain_metadata_reconstruction_manifest or any(getattr(args,'domain_metadata_'+label) for label in ('output','before','control_save','after','repeat')):
        parser.error('domain metadata paths require the separate domain mode')
    placement_paths={}
    if args.placement_metadata:
        if not game_input or not args.prepare_main_thread or not args.cleanup or not args.placement_metadata_candidate_manifest or any((combat_input,random_input,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('placement metadata requires a separate prepared-main-thread loaded Core cleanup fixture and frozen reconstruction manifest')
        args.game_load_observe=True
        for label in ('output','expected_output','before','control_save','after','repeat'):
            path=getattr(args,'placement_metadata_'+label)
            if path is None:parser.error('placement metadata requires output and all four snapshot paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('placement metadata artifacts must be fresh private files')
            placement_paths[label]=path
        if len(set(placement_paths.values()))!=6:parser.error('placement metadata artifact paths must be distinct')
    elif args.placement_metadata_candidate_manifest or any(getattr(args,'placement_metadata_'+label) for label in ('output','expected_output','before','control_save','after','repeat')):
        parser.error('placement metadata paths require the separate domain mode')
    placement_handler_paths={}
    if args.placement_handler_metadata:
        if not game_input or not args.prepare_main_thread or not args.cleanup or not args.placement_handler_candidate_manifest or any((combat_input,random_input,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('placement handler metadata requires a separate prepared-main-thread loaded Core cleanup fixture and frozen reconstruction manifest')
        args.game_load_observe=True
        for label in ('output','expected_output','handler_expected_output','before','control_save','after','repeat'):
            path=getattr(args,'placement_handler_metadata_'+label)
            if path is None:parser.error('placement handler metadata requires output and all four snapshot paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('placement handler metadata artifacts must be fresh private files')
            placement_handler_paths[label]=path
        if len(set(placement_handler_paths.values()))!=7:parser.error('placement handler metadata artifact paths must be distinct')
    elif args.placement_handler_candidate_manifest or any(getattr(args,'placement_handler_metadata_'+label) for label in ('output','expected_output','handler_expected_output','before','control_save','after','repeat')):
        parser.error('placement handler metadata paths require the separate domain mode')
    manager_paths={}
    if args.unit_operation_manager_metadata:
        if not game_input or not args.prepare_main_thread or not args.cleanup or not args.unit_operation_manager_candidate_manifest or any((combat_input,random_input,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('Manager metadata requires separate main-thread Core cleanup fixture and frozen reconstruction manifest')
        args.game_load_observe=True
        for label in ('output','repeat_output','control_output','before','control_save','after','repeat'):
            path=getattr(args,'unit_operation_manager_metadata_'+label)
            if path is None:parser.error('Manager metadata requires all raw/control/snapshot paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('Manager metadata artifacts must be fresh private files')
            manager_paths[label]=path
        if len(set(manager_paths.values()))!=7:parser.error('Manager metadata artifact paths must be distinct')
    elif args.unit_operation_manager_candidate_manifest or any(getattr(args,'unit_operation_manager_metadata_'+label) for label in ('output','repeat_output','control_output','before','control_save','after','repeat')):
        parser.error('Manager metadata paths require separate Manager mode')
    current_handler_paths={}
    if args.current_handler_metadata:
        if not args.prepare_only and not args.expected_configuration:parser.error('current-handler execution requires exact reviewed configuration')
        if not game_input or not args.prepare_main_thread or not args.cleanup or any((combat_input,random_input,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir)):
            parser.error('Current-handler metadata requires separate main-thread Core cleanup fixture')
        args.game_load_observe=True
        for label in ('output','repeat_output','control_output','before','control_save','after','repeat'):
            path=getattr(args,'current_handler_metadata_'+label)
            if path is None:parser.error('Current-handler metadata requires all raw/control/snapshot paths')
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('Current-handler metadata artifacts must be fresh private files')
            current_handler_paths[label]=path
        if len(set(current_handler_paths.values()))!=7:parser.error('Current-handler metadata artifact paths must be distinct')
    elif any(getattr(args,'current_handler_metadata_'+label) for label in ('output','repeat_output','control_output','before','control_save','after','repeat')):
        parser.error('Current-handler metadata paths require separate current-handler mode')
    military_directory=None
    military_plan=None
    if args.military_initialization_observer:
        if not game_input or not args.prepare_main_thread or not args.cleanup or not args.military_observer_plan or not args.military_observer_directory or not args.saved_configuration or not args.configure or not args.game_save_output or not args.game_save_repeat_output or any((combat_input,random_input,args.action_observe,args.action_execution_observe,args.action_lifecycle_observe,args.definition_ids,args.unit_definition_ids,args.operation_registry,args.save_normalization_trace,args.rules_snapshot_dir,args.request_load_save,args.game_load_observe)):
            parser.error('Military observer requires separate ordinary main-thread Core Load/two Saves/cleanup fixture and frozen observer plan')
        if not args.prepare_only and not args.expected_configuration:parser.error('Military original execution requires exact admitted expected configuration')
        military_directory=args.military_observer_directory.resolve()
        if not military_directory.is_dir() or not military_directory.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):parser.error('Military observer directory must be private')
        for name in ('observations.jsonl','transcript.txt','exit.json','capture.gdb','runtime-plan.json','runtime-prepared.json','supervisor.json','inferior.json','disposal.json'):
            if (military_directory/name).exists():parser.error('Military observer runtime artifacts must be fresh')
        military_plan=args.military_observer_plan.resolve()
    elif args.military_observer_plan or args.military_observer_directory:
        parser.error('Military observer paths require separate observer mode')
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
    if args.action_lifecycle_observe or action_mode or core_mode or movement_mode or args.domain_metadata or args.placement_metadata or args.placement_handler_metadata or args.unit_operation_manager_metadata or args.current_handler_metadata:
        if (not args.action_observe and not core_mode and not movement_mode and not args.domain_metadata and not args.placement_metadata and not args.placement_handler_metadata and not args.unit_operation_manager_metadata and not args.current_handler_metadata) or args.action_execution_observe or not action_trace:
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
    captured_extra_sources=[]
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
    if args.game_load_profile=='captured-second':
        if not game_input:parser.error('--game-load-profile requires --game-load-stream')
        from game_load_second_capture_gate import load_gate as second_gate
        captured_gate_path=ROOT/'tools/linux/game_load_second_capture_gate.json'
        captured_gate=second_gate(ROOT)
        captured_extra_sources=[ROOT/'tools/linux/game_load_second_capture_gate.py',ROOT/captured_gate['origin_summary']]
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
        if set(random_paths.values())&set(game_outputs.values()):parser.error('random damage and outer Save paths must be distinct')
        if set(core_paths.values())&set(game_outputs.values()):parser.error('Core component and outer Save paths must be distinct')
        if set(movement_paths.values())&set(game_outputs.values()):parser.error('movement and outer Save paths must be distinct')
        if set(domain_paths.values())&set(game_outputs.values()):parser.error('domain metadata and outer Save paths must be distinct')
        if set(placement_paths.values())&set(game_outputs.values()):parser.error('placement metadata and outer Save paths must be distinct')
        if set(manager_paths.values())&set(game_outputs.values()):parser.error('Manager metadata and outer Save paths must be distinct')
        if set(current_handler_paths.values())&set(game_outputs.values()):parser.error('current-handler metadata and outer Save paths must be distinct')
        if set(placement_handler_paths.values())&set(game_outputs.values()):parser.error('handler metadata and outer Save paths must be distinct')
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
    if args.action_execution_observe or args.action_lifecycle_observe or action_mode or core_mode or movement_mode or args.domain_metadata or args.placement_metadata or args.placement_handler_metadata or args.unit_operation_manager_metadata or args.current_handler_metadata:
        execution_audit_paths=[ROOT/'tools/linux/action_execution_audit.py',ROOT/'tools/linux/action_event_audit.py',ROOT/'evidence/linux/action-event-abi.json',ROOT/'tools/linux/save_lifecycle_audit.py',ROOT/'tools/linux/inspect_elf.py',ROOT/'evidence/linux/action-execution-abi.json']
        with tempfile.TemporaryDirectory(prefix='c6-execution-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(execution_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            execution_audit=json.loads(result_path.read_text())
        if execution_audit!=json.loads(execution_audit_paths[-1].read_text()):raise SystemExit('original cache dispatch contracts differ')
    dispatch_audits={}
    dispatch_audit_paths=[]
    if action_mode or args.action_lifecycle_observe or core_mode or movement_mode or args.domain_metadata or args.placement_metadata or args.placement_handler_metadata or args.unit_operation_manager_metadata or args.current_handler_metadata:
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
    random_profile=None
    random_audit_paths=[]
    if random_input:
        from combat_random_audit import audit as random_audit
        random_audit_paths=[ROOT/'tools/linux/combat_random_audit.py',ROOT/'evidence/linux/combat-random-contracts.json',ROOT/'evidence/linux/combat-random-input.json']
        random_manifest=json.loads(random_audit_paths[-1].read_text())
        if random_manifest['schema']!='c6-linux-random-damage-input-v1' or random_manifest['use_average'] is not False or digest(random_input)!=random_manifest['input_sha256'] or random_input.stat().st_size!=random_manifest['input_bytes']:
            raise SystemExit('random damage input manifest mismatch')
        for name,sha in random_manifest['source_sha256'].items():
            if digest(ROOT/name)!=sha:raise SystemExit('random damage source identity mismatch: '+name)
        with tempfile.TemporaryDirectory(prefix='combat-random-audit-',dir=ROOT.parent/'c6-ghidra/evidence') as temporary:
            random_contract=random_audit(assets/'libGameCore_XP2.so',assets/'Civ6',Path(temporary))
        if random_contract!=json.loads(random_audit_paths[-2].read_text()):raise SystemExit('random damage native contracts differ')
        random_profile={'input_sha256':random_manifest['input_sha256'],'input_bytes':random_manifest['input_bytes'],'case_count':random_manifest['case_count'],'use_average':False,'contract':random_contract}
    movement_profile=None
    movement_audit_paths=[]
    if movement_mode:
        movement_audit_paths=[ROOT/'tools/linux/movement_probe_audit.py',ROOT/('evidence/linux/movement-cost-probe-xp2-contract.json' if args.movement_cost_xp2_map else 'evidence/linux/movement-cost-probe-diagnostic-contract.json' if args.movement_cost_diagnostic else 'evidence/linux/movement-cost-probe-contract.json'),ROOT/'tools/linux/game_load_state_audit.py',ROOT/'evidence/linux/game-load-state-facade-contracts.json']
        admission_audit=ROOT/'tools/linux/oracle_admission_cache_audit.py'
        admission_contract=json.loads(subprocess.check_output([sys.executable,str(admission_audit)],cwd=ROOT,text=True))
        movement_audit_paths.extend([admission_audit,ROOT/'tools/linux/domain_metadata_fixture_audit.py',ROOT/'tools/linux/placement_metadata_fixture_audit.py',ROOT/'tools/linux/placement_handler_metadata_fixture_audit.py'])
        movement_contract=json.loads(movement_audit_paths[1].read_text())
        for name,sha in movement_contract['sources'].items():
            if digest(ROOT/name)!=sha:raise SystemExit('movement source identity mismatch: '+name)
        frozen={}
        for kind,reference in movement_contract['frozen_builds'].items():
            path=ROOT/'target/frozen/movement-cost-probe'/reference['sha256']/kind
            if not path.exists():
                source=ROOT/reference['path']
                if not source.is_file() or digest(source)!=reference['sha256'] or source.stat().st_size!=reference['bytes']:raise SystemExit('movement reference artifact mismatch: '+kind)
                path.parent.mkdir(parents=True,exist_ok=True)
                with path.open('xb') as stream:stream.write(source.read_bytes())
                path.chmod(0o555)
            if digest(path)!=reference['sha256'] or path.stat().st_size!=reference['bytes']:raise SystemExit('frozen movement artifact mismatch: '+kind)
            frozen[kind]={'path':str(path.relative_to(ROOT)),'sha256':digest(path),'bytes':path.stat().st_size}
        with tempfile.TemporaryDirectory(prefix='movement-probe-audit-') as temporary:
            result=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(movement_audit_paths[0]),'--assets',str(assets),'--native-helper',frozen['native_helper']['path'],'--candidate',frozen['candidate']['path'],'--output',str(result)],check=True,capture_output=True,text=True)
            actual=json.loads(result.read_text())
        # Only relocate immutable artifact paths; their complete bytes/hash are
        # checked above and remain in the actual execution configuration.
        for kind in frozen:actual['frozen_builds'][kind]['path']=movement_contract['frozen_builds'][kind]['path']
        if actual!=movement_contract:raise SystemExit('movement original/source/build contracts differ')
        candidate_source_commit=subprocess.check_output(['git','rev-parse','cb6113b^{commit}'],cwd=ROOT,text=True).strip()
        candidate_source_files={}
        for name in ('crates/c6-core/src/movement.rs','crates/c6-differential/src/bin/c6-movement-cost-candidate.rs','crates/c6-core/src/lib.rs','crates/c6-core/Cargo.toml','crates/c6-differential/Cargo.toml','Cargo.lock'):
            content=subprocess.check_output(['git','show',candidate_source_commit+':'+name],cwd=ROOT)
            source_sha=hashlib.sha256(content).hexdigest()
            if name in movement_contract['sources'] and source_sha!=movement_contract['sources'][name]:raise SystemExit('frozen movement candidate source reference differs')
            candidate_source_files[name]=source_sha
        candidate_manifest={'schema':'c6-frozen-movement-candidate-source-v1','candidate_sha256':frozen['candidate']['sha256'],'source_reference_commit':candidate_source_commit,'sources_at_reference_commit':candidate_source_files,'mutable_release_binary_not_used':True}
        candidate_manifest_path=ROOT/frozen['candidate']['path'];candidate_manifest_path=candidate_manifest_path.with_name('source-manifest.json')
        candidate_manifest_bytes=(json.dumps(candidate_manifest,sort_keys=True,indent=2)+'\n').encode()
        if candidate_manifest_path.exists():
            if candidate_manifest_path.read_bytes()!=candidate_manifest_bytes:raise SystemExit('immutable movement source manifest differs')
        else:
            with candidate_manifest_path.open('xb') as stream:stream.write(candidate_manifest_bytes)
            candidate_manifest_path.chmod(0o444)
        bootstrap_commit='50a91ae839ab8c1ff5b5e893be24d56084e66772'
        bootstrap_source='native/linux/localization.c'
        bootstrap_sha='b898657a0e88837be77ecdfa89516494ce3e325ddb43bcc6bb1bca4e9e02d89d'
        committed_bootstrap=subprocess.check_output(['git','show',bootstrap_commit+':'+bootstrap_source],cwd=ROOT)
        if hashlib.sha256(committed_bootstrap).hexdigest()!=bootstrap_sha or digest(ROOT/bootstrap_source)!=bootstrap_sha:raise SystemExit('movement committed localization bridge differs from accepted source')
        movement_profile={'admission_binding_cache':{'revision':'movement-scope-immutable-bindings-v1','audit':admission_contract,'native_acceptance_pending':True},'mode':movement_mode,'admission_diagnostic':args.movement_cost_diagnostic,'xp2_map_revision':args.movement_cost_xp2_map,'max_formations':args.movement_cost_max_formations,'query_options':0,'actual_move_options_available':False,'contract':movement_contract,'frozen_artifacts':frozen,'candidate_frozen_before_original_execution':True,'candidate_source_manifest':{'path':str(candidate_manifest_path.relative_to(ROOT)),'sha256':digest(candidate_manifest_path),'source_reference_commit':candidate_source_commit},'bootstrap_source':{'path':bootstrap_source,'sha256':bootstrap_sha,'commit':bootstrap_commit,'accepted_native_configuration':'b6ca58d53073abc2afa9176a4ceba350fafe3f91ba12dbf84fd08e25a3be9a5b'},'internal_core_saves':4,'known_rng_objects':['Game sync Random32','Map generator Random32'],'quiescence':'original controller/context idle plus acquired/released native Game facade lock barrier; unknown/work fatal before poststate/teardown'}
    domain_profile=None
    domain_audit_paths=[]
    if args.domain_metadata:
        domain_audit_paths=[ROOT/'tools/linux/domain_metadata_audit.py',ROOT/'evidence/linux/domain-metadata-query-contracts.json',ROOT/'tools/linux/domain_metadata_fixture_audit.py',ROOT/'evidence/linux/domain-metadata-fixture-contract.json']
        with tempfile.TemporaryDirectory(prefix='domain-metadata-audit-') as temporary:
            target=Path(temporary)/'contract.json'
            subprocess.run([sys.executable,str(domain_audit_paths[0]),'--module',str(assets/'libGameCore_XP2.so'),'--output',str(target)],check=True,capture_output=True,text=True)
            domain_contract=json.loads(target.read_text())
        domain_audit_paths.extend([ROOT/'tools/linux/placement_metadata_fixture_audit.py',ROOT/'tools/linux/placement_handler_metadata_fixture_audit.py'])
        if domain_contract!=json.loads(domain_audit_paths[1].read_text()):raise SystemExit('domain native contracts differ')
        fixture_contract=json.loads(subprocess.check_output([sys.executable,str(domain_audit_paths[2])],cwd=ROOT,text=True))
        if fixture_contract!=json.loads(domain_audit_paths[3].read_text()):raise SystemExit('domain fixture source contract differs')
        manifest_path=args.domain_metadata_reconstruction_manifest.resolve()
        manifest=json.loads(manifest_path.read_text())
        if manifest['schema']!='c6-linux-domain-reconstruction-candidate-v1' or manifest['expected_outputs_used_as_inputs'] is not False or manifest['native_execution'] is not False or manifest['native_comparison'] is not False:raise SystemExit('domain reconstruction candidate boundary differs')
        candidate=ROOT/manifest['candidate_path'];domain_source_snapshot=ROOT/manifest['source_snapshot_path']
        if digest(candidate)!=manifest['candidate_sha256'] or candidate.stat().st_size!=manifest['candidate_bytes']:raise SystemExit('domain reconstruction executable differs')
        if digest(domain_source_snapshot)!=manifest['source_snapshot_sha256'] or domain_source_snapshot.stat().st_size!=manifest['source_snapshot_bytes']:raise SystemExit('domain reconstruction source archive differs')
        for name,sha in manifest['sources'].items():
            committed=subprocess.check_output(['git','show',manifest['source_commit']+':'+name],cwd=ROOT)
            if hashlib.sha256(committed).hexdigest()!=sha:raise SystemExit('domain reconstruction committed source differs: '+name)
        domain_profile={'mode':'read_only_domain_metadata','contract':domain_contract,'fixture_contract':fixture_contract,'reconstruction_manifest':manifest,'reconstruction_manifest_sha256':digest(manifest_path),'expected_outputs_used_as_inputs':False,'internal_core_saves':4,'known_rng_objects':['Game sync Random32','Map generator Random32'],'complete_rng_inventory':False,'native_acceptance_pending':True}
    placement_profile=None
    placement_audit_paths=[]
    if args.placement_metadata:
        placement_audit_paths=[ROOT/'tools/linux/placement_metadata_audit.py',ROOT/'evidence/linux/placement-metadata-preparation.json',ROOT/'tools/linux/placement_metadata_fixture_audit.py',ROOT/'evidence/linux/placement-metadata-fixture-contract.json']
        with tempfile.TemporaryDirectory(prefix='placement-metadata-audit-') as temporary:
            target=Path(temporary)/'contract.json'
            subprocess.run([sys.executable,str(placement_audit_paths[0]),'--module',str(assets/'libGameCore_XP2.so'),'--output',str(target)],check=True,capture_output=True,text=True)
            placement_contract=json.loads(target.read_text())
        placement_audit_paths.extend([ROOT/'tools/linux/domain_metadata_fixture_audit.py',ROOT/'tools/linux/unit_placement_audit.py',ROOT/'tools/linux/save_lifecycle_audit.py',ROOT/'tools/linux/placement_handler_metadata_fixture_audit.py'])
        if placement_contract!=json.loads(placement_audit_paths[1].read_text()):raise SystemExit('placement native contracts differ')
        for name,sha in placement_contract['source_sha256'].items():
            if digest(ROOT/name)!=sha:raise SystemExit('placement native source differs: '+name)
        fixture_contract=json.loads(subprocess.check_output([sys.executable,str(placement_audit_paths[2])],cwd=ROOT,text=True))
        if fixture_contract!=json.loads(placement_audit_paths[3].read_text()):raise SystemExit('placement fixture source contract differs')
        manifest_path=args.placement_metadata_candidate_manifest.resolve()
        manifest=json.loads(manifest_path.read_text())
        if manifest['schema']!='c6-linux-placement-count-candidate-v1' or manifest['expected_outputs_used_as_inputs'] is not False or manifest['native_execution'] is not False or manifest['native_comparison'] is not False:raise SystemExit('placement count candidate boundary differs')
        candidate=ROOT/manifest['candidate_path'];placement_source_snapshot=ROOT/manifest['source_snapshot_path']
        if digest(candidate)!=manifest['candidate_sha256'] or candidate.stat().st_size!=manifest['candidate_bytes']:raise SystemExit('placement count executable differs')
        if digest(placement_source_snapshot)!=manifest['source_snapshot_sha256'] or placement_source_snapshot.stat().st_size!=manifest['source_snapshot_bytes']:raise SystemExit('placement count source archive differs')
        for name,sha in manifest['sources'].items():
            committed=subprocess.check_output(['git','show',manifest['source_commit']+':'+name],cwd=ROOT)
            if hashlib.sha256(committed).hexdigest()!=sha:raise SystemExit('placement count committed source differs: '+name)
        placement_profile={'mode':'read_only_friendly_formation_counts','native_bool_argument':False,'native_handler_query_available':False,'full_membership_required':True,'sampling_allowed':False,'contract':placement_contract,'fixture_contract':fixture_contract,'reconstruction_manifest':manifest,'reconstruction_manifest_sha256':digest(manifest_path),'expected_outputs_used_as_inputs':False,'internal_core_saves':4,'known_rng_objects':['Game sync Random32','Map generator Random32'],'complete_rng_inventory':False,'native_acceptance_pending':True}
    placement_handler_profile=None
    placement_handler_audit_paths=[]
    if args.placement_handler_metadata:
        placement_handler_audit_paths=[ROOT/'tools/linux/placement_handler_metadata_audit.py',ROOT/'evidence/linux/placement-handler-metadata-preparation.json',ROOT/'tools/linux/placement_handler_metadata_fixture_audit.py',ROOT/'evidence/linux/placement-handler-metadata-fixture-contract.json']
        with tempfile.TemporaryDirectory(prefix='placement-handler-metadata-audit-') as temporary:
            target=Path(temporary)/'contract.json'
            subprocess.run([sys.executable,str(placement_handler_audit_paths[0]),'--assets',str(assets),'--output',str(target)],check=True,capture_output=True,text=True)
            placement_handler_contract=json.loads(target.read_text())
        placement_handler_audit_paths.extend([ROOT/'tools/linux/domain_metadata_fixture_audit.py',ROOT/'tools/linux/unit_placement_audit.py',ROOT/'tools/linux/save_lifecycle_audit.py',ROOT/'tools/linux/placement_metadata_fixture_audit.py',ROOT/'tools/linux/placement_metadata_audit.py',ROOT/'tools/linux/skip_turn_handler_query_audit.py',ROOT/'evidence/linux/skip-turn-handler-query-abi.json'])
        if placement_handler_contract!=json.loads(placement_handler_audit_paths[1].read_text()):raise SystemExit('placement native contracts differ')
        handler_sources={**placement_handler_contract['source_contract']['sources'],**placement_handler_contract['count_contract']['source_sha256'],**placement_handler_contract['handler_contract']['source_sha256']}
        for name,sha in handler_sources.items():
            if digest(ROOT/name)!=sha:raise SystemExit('placement native source differs: '+name)
        fixture_contract=json.loads(subprocess.check_output([sys.executable,str(placement_handler_audit_paths[2])],cwd=ROOT,text=True))
        if fixture_contract!=json.loads(placement_handler_audit_paths[3].read_text()):raise SystemExit('placement fixture source contract differs')
        manifest_path=args.placement_handler_candidate_manifest.resolve()
        manifest=json.loads(manifest_path.read_text())
        if manifest['schema']!='c6-linux-placement-handler-candidate-v1' or manifest['expected_outputs_used_as_inputs'] is not False or manifest['native_execution'] is not False or manifest['native_comparison'] is not False:raise SystemExit('placement count candidate boundary differs')
        reference=manifest['reference_manifest'];reference_path=ROOT/reference['path']
        if digest(reference_path)!=reference['sha256']:raise SystemExit('historical count candidate manifest differs')
        historical=json.loads(reference_path.read_text())
        if reference['schema']!=historical['schema'] or historical['schema']!='c6-linux-placement-count-candidate-v1':raise SystemExit('historical candidate schema differs')
        for key,value in historical.items():
            if key!='schema' and manifest.get(key)!=value:raise SystemExit('handler reference changed historical candidate field: '+key)
        if manifest['candidate_protocol_unchanged'] is not True or manifest['candidate_internal_native_compared_value'] is not False:raise SystemExit('handler reference relabels historical candidate protocol')
        preflight=manifest['handler_preflight']
        if digest(ROOT/preflight['path'])!=preflight['sha256']:raise SystemExit('handler typed preflight differs')
        if manifest['outer_attestation']['full_manager_legality'] is not False or manifest['outer_attestation']['native_action_execution'] is not False:raise SystemExit('handler reference exceeds predicate boundary')
        candidate=ROOT/manifest['candidate_path'];placement_handler_source_snapshot=ROOT/manifest['source_snapshot_path']
        if digest(candidate)!=manifest['candidate_sha256'] or candidate.stat().st_size!=manifest['candidate_bytes']:raise SystemExit('placement count executable differs')
        if digest(placement_handler_source_snapshot)!=manifest['source_snapshot_sha256'] or placement_handler_source_snapshot.stat().st_size!=manifest['source_snapshot_bytes']:raise SystemExit('placement count source archive differs')
        for name,sha in manifest['sources'].items():
            committed=subprocess.check_output(['git','show',manifest['source_commit']+':'+name],cwd=ROOT)
            if hashlib.sha256(committed).hexdigest()!=sha:raise SystemExit('placement count committed source differs: '+name)
        placement_handler_profile={'mode':'read_only_counts_and_skipturn_handler','native_bool_argument':False,'full_manager_legality':False,'native_action_execution':False,'handler_matrix':{'plot':'current_and_NULL_or_NULL_if_unplaced','visible_only':[False,True],'repeats':2,'NULL_unit_is_no_leaf_control':True,'delayed_death_filter':False},'native_handler_query_available':True,'full_membership_required':True,'sampling_allowed':False,'contract':placement_handler_contract,'fixture_contract':fixture_contract,'reconstruction_manifest':manifest,'reconstruction_manifest_sha256':digest(manifest_path),'expected_outputs_used_as_inputs':False,'internal_core_saves':4,'known_rng_objects':['Game sync Random32','Map generator Random32'],'complete_rng_inventory':False,'native_acceptance_pending':True}
    manager_profile=None
    manager_audit_paths=[]
    if args.unit_operation_manager_metadata:
        manager_audit_paths=[ROOT/'tools/linux/unit_operation_manager_metadata_audit.py',ROOT/'evidence/linux/unit-operation-manager-metadata-preparation.json',ROOT/'tools/linux/unit_operation_manager_metadata_fixture_audit.py',ROOT/'evidence/linux/unit-operation-manager-metadata-fixture-contract.json']
        with tempfile.TemporaryDirectory(prefix='manager-metadata-audit-') as temporary:
            target=Path(temporary)/'contract.json'
            subprocess.run([sys.executable,str(manager_audit_paths[0]),'--assets',str(assets),'--output',str(target)],check=True,capture_output=True,text=True)
            manager_contract=json.loads(target.read_text())
        if manager_contract!=json.loads(manager_audit_paths[1].read_text()):raise SystemExit('Manager native contracts differ')
        fixture_contract=json.loads(subprocess.check_output([sys.executable,str(manager_audit_paths[2])],cwd=ROOT,text=True))
        if fixture_contract!=json.loads(manager_audit_paths[3].read_text()):raise SystemExit('Manager fixture source contract differs')
        manifest_path=args.unit_operation_manager_candidate_manifest.resolve()
        manifest=json.loads(manifest_path.read_text())
        if manifest['schema']!='c6-linux-unit-operation-manager-candidate-v1' or manifest['expected_outputs_used_as_inputs'] is not False or manifest['native_execution'] is not False or manifest['native_comparison'] is not False:raise SystemExit('Manager candidate boundary differs')
        candidate=ROOT/manifest['candidate_path'];manager_source_snapshot=ROOT/manifest['source_snapshot_path']
        if digest(candidate)!=manifest['candidate_sha256'] or candidate.stat().st_size!=manifest['candidate_bytes']:raise SystemExit('Manager candidate executable differs')
        if digest(manager_source_snapshot)!=manifest['source_snapshot_sha256'] or manager_source_snapshot.stat().st_size!=manifest['source_snapshot_bytes']:raise SystemExit('Manager source archive differs')
        for name,sha in manifest['sources'].items():
            committed=subprocess.check_output(['git','show',manifest['source_commit']+':'+name],cwd=ROOT)
            if hashlib.sha256(committed).hexdigest()!=sha:raise SystemExit('Manager candidate committed source differs: '+name)
        manager_profile={'mode':'read_only_manager_raw_metadata','contract':manager_contract,'fixture_contract':fixture_contract,'reconstruction_manifest':manifest,'reconstruction_manifest_sha256':digest(manifest_path),'raw_passes':2,'internal_core_saves':4,'expected_outputs_used_as_inputs':False,'disabled_vectors_are_reconstruction_targets':True,'phase_and_controls_are_raw_inputs':True,'full_manager_called':False,'script_callbacks_called':False,'complete_rng_inventory':False,'native_acceptance_pending':True}
    current_handler_profile=None
    current_handler_audit_paths=[]
    if args.current_handler_metadata:
        current_handler_audit_paths=[ROOT/'tools/linux/current_handler_interruption_metadata_audit.py',ROOT/'evidence/linux/current-handler-interruption-metadata-preparation.json',ROOT/'tools/linux/current_handler_field_guard_audit.py',ROOT/'evidence/linux/current-handler-field-guard-source.json',ROOT/'tools/linux/current_handler_caller_audit.py',ROOT/'evidence/linux/current-handler-caller-source.json',ROOT/'tools/linux/current_handler_fixture_audit.py',ROOT/'evidence/linux/current-handler-fixture-contract.json']
        packets=[]
        with tempfile.TemporaryDirectory(prefix='current-handler-audit-') as temporary:
            for tool,canonical in zip(current_handler_audit_paths[::2],current_handler_audit_paths[1::2]):
                target=Path(temporary)/(tool.stem+'.json')
                command=[sys.executable,str(tool),'--output',str(target)]
                if tool==current_handler_audit_paths[0]:command+=['--assets',str(assets)]
                subprocess.run(command,check=True,capture_output=True,text=True)
                packet=json.loads(target.read_text())
                if packet!=json.loads(canonical.read_text()):raise SystemExit('current-handler source packet differs: '+str(tool))
                packets.append(packet)
        current_handler_profile={'mode':'passive_current_handler_fields','packets':packets,'raw_passes':2,'internal_core_saves':4,'original_calls_in_helper_or_guard':False,'facade_acquisition_is_caller_scope':True,'Save_lock_continuity_claimed':False,'full_manager_called':False,'script_callbacks_called':False,'complete_rng_inventory':False,'native_acceptance_pending':True}
    military_profile=None
    military_audit_paths=[]
    if args.military_initialization_observer:
        military_audit_paths=[ROOT/'tools/linux/military_initialization_observer.py',ROOT/'tools/linux/military_initialization_observer_audit.py',ROOT/'evidence/linux/military-initialization-observer-preparation.json',ROOT/'tools/linux/military_observer_runner.py',ROOT/'tools/linux/military_stream_fixture_audit.py',ROOT/'evidence/linux/military-stream-fixture-contract.json']
        plan=json.loads(military_plan.read_text())
        if plan['schema']!='c6-linux-military-observer-plan-v1' or plan['observer_sha256']!=digest(military_audit_paths[0]) or plan.get('runtime_gate') is not None:raise SystemExit('Military static observer plan differs')
        with tempfile.TemporaryDirectory(prefix='military-observer-audit-') as temporary:
            generated=Path(temporary)/'plan.json';packet=Path(temporary)/'packet.json'
            subprocess.run([sys.executable,str(military_audit_paths[0]),'--module',str(assets/'libGameCore_XP2.so'),'--host',str(args.launch_host.absolute() if args.launch_host else assets/'Civ6'),'--output',str(generated)],check=True,capture_output=True,text=True)
            if json.loads(generated.read_text())!=plan:raise SystemExit('Military static plan differs from pinned target/host bytes')
            subprocess.run([sys.executable,str(military_audit_paths[1]),'--assets',str(assets),'--output',str(packet)],check=True,capture_output=True,text=True)
            if json.loads(packet.read_text())!=json.loads(military_audit_paths[2].read_text()):raise SystemExit('Military observer source/protocol packet differs')
        from military_stream_fixture_audit import audit as military_fixture_audit
        fixture=military_fixture_audit(assets,args.launch_host.absolute() if args.launch_host else assets/'Civ6')
        if fixture!=json.loads(military_audit_paths[-1].read_text()):raise SystemExit('Military stream fixture differs')
        military_audit_paths.extend([ROOT/'tools/linux/ai_military_initialization_audit.py',ROOT/'tools/linux/player_wmds_audit.py',ROOT/'tools/linux/inspect_elf.py'])
        military_profile={'mode':'ordinary_load_save_passive','input_save_artifact':{'path':str(args.saved_configuration.resolve()),'bytes':args.saved_configuration.stat().st_size,'sha256':digest(args.saved_configuration),'kind':'whole_file_supplied_to_saved_configuration_loader'},'input_core_artifact':{'path':str(game_input),'bytes':game_input.stat().st_size,'sha256':digest(game_input),'kind':'actual_MemoryIStream_payload'},'save_to_core_extraction_binding_proven':False,'runtime_policy_counters_available':False,'policy_scope':'source-only ordinary Load/Save invocation; no claims of runtime policy-call absence','static_plan':plan,'static_plan_sha256':digest(military_plan),'fixture_contract':fixture,'hardware_breakpoints':4,'software_game_breakpoints':False,'gdb_internal_loader_rendezvous':True,'inferior_calls':False,'game_field_writes':False,'extra_original_queries':False,'dirty_getter_called':False,'sql_added':False,'source_profile_review_required':True,'nested_Read_initialization_only':True,'rules_database_snapshot_unavailable':True,'complete_state_action_rng_turns_supported':False}
    core_contract=None
    core_audit_paths=[]
    if core_mode:
        core_audit_paths=[ROOT/'tools/linux/core_component_audit.py',ROOT/'evidence/linux/core-component-static.json']
        with tempfile.TemporaryDirectory(prefix='core-component-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(core_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            core_contract=json.loads(result_path.read_text())
        if core_contract!=json.loads(core_audit_paths[-1].read_text()):raise SystemExit('original Core component contracts differ')
    main_thread_contract=None
    main_thread_audit_paths=[]
    if args.prepare_main_thread:
        main_thread_audit_paths=[ROOT/'tools/linux/action_worker_audit.py',ROOT/'evidence/linux/action-worker-abi.json']
        with tempfile.TemporaryDirectory(prefix='main-thread-audit-') as temporary:
            result_path=Path(temporary)/'contracts.json'
            subprocess.run([sys.executable,str(main_thread_audit_paths[0]),'--assets',str(assets),'--output',str(result_path)],check=True,capture_output=True,text=True)
            main_thread_contract=json.loads(result_path.read_text())
        if main_thread_contract!=json.loads(main_thread_audit_paths[-1].read_text()):raise SystemExit('original early main-thread contracts differ')
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
    action_sources=[ROOT/'native/linux/action_observer.c'] if args.action_observe or core_mode or movement_mode or args.domain_metadata or args.placement_metadata or args.placement_handler_metadata or args.unit_operation_manager_metadata or args.current_handler_metadata else []
    normalization_sources=[ROOT/'tools/linux/save_normalization_trace.c'] if normalization_trace else []
    execution_sources=[ROOT/'native/linux/action_execution.c'] if args.action_execution_observe or args.action_lifecycle_observe or action_mode or core_mode or movement_mode or args.domain_metadata or args.placement_metadata or args.placement_handler_metadata or args.unit_operation_manager_metadata or args.current_handler_metadata else []
    lifecycle_observer_sources=[ROOT/'native/linux/action_lifecycle.c',ROOT/'tools/linux/action_execution_trace.c'] if args.action_lifecycle_observe or action_mode or core_mode or movement_mode or args.domain_metadata or args.placement_metadata or args.placement_handler_metadata or args.unit_operation_manager_metadata or args.current_handler_metadata else []
    dispatch_sources=[ROOT/'native/linux/action_dispatch.c',ROOT/'native/linux/action_state.c',ROOT/'native/linux/action_registration.c'] if action_mode or args.action_lifecycle_observe or core_mode or movement_mode or args.domain_metadata or args.placement_metadata or args.placement_handler_metadata or args.unit_operation_manager_metadata or args.current_handler_metadata else []
    definition_sources=[ROOT/'native/linux/definition_ids.c'] if args.definition_ids else []
    unit_definition_sources=[ROOT/'native/linux/unit_definition_ids.c'] if args.unit_definition_ids else []
    operation_registry_sources=[ROOT/'native/linux/operation_registry.c'] if args.operation_registry else []
    combat_sources=[ROOT/'native/linux/combat_kernel.c'] if combat_input else []
    random_sources=[ROOT/'native/linux/combat_random.c'] if random_input else []
    core_sources=[ROOT/'native/linux/core_component.c'] if core_mode else []
    movement_sources=[ROOT/'native/linux/movement_cost.c'] if movement_mode else []
    domain_sources=[ROOT/'native/linux/domain_metadata.c',ROOT/'native/linux/domain_metadata_fixture.c'] if args.domain_metadata else []
    placement_sources=[ROOT/'native/linux/placement_metadata.c',ROOT/'native/linux/placement_metadata_fixture.c'] if args.placement_metadata else []
    manager_sources=[ROOT/'native/linux/unit_operation_manager_metadata.c',ROOT/'native/linux/unit_operation_manager_metadata_fixture.c'] if args.unit_operation_manager_metadata else []
    current_handler_sources=[ROOT/'native/linux/current_handler_interruption.c',ROOT/'native/linux/current_handler_metadata_fixture.c',ROOT/'native/linux/current_handler_metadata_probe.c'] if args.current_handler_metadata else []
    placement_handler_sources=[ROOT/'native/linux/placement_handler_metadata.c',ROOT/'native/linux/placement_handler_metadata_fixture.c',ROOT/'native/linux/skip_turn_handler_query.c'] if args.placement_handler_metadata else []
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
              'prepare_main_thread':args.prepare_main_thread,'main_thread_preparation_contract':main_thread_contract,
              'lower_core_definition_registry_observer':definition_audit,
              'lower_core_unit_definition_registry_observer':unit_definition_audit,
              'lower_core_operation_registry_observer':operation_registry_audit,
              'lower_core_average_damage_kernel':combat_profile,
              'lower_core_random_damage_kernel':random_profile,
              'lower_core_movement_cost':movement_profile,
              'lower_core_domain_metadata':domain_profile,
              'lower_core_placement_metadata':placement_profile,
              'lower_core_placement_handler_metadata':placement_handler_profile,
              'lower_core_unit_operation_manager_metadata':manager_profile,
              'lower_core_current_handler_metadata':current_handler_profile,
              'lower_core_military_initialization_observer':military_profile,
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
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*game_sources,ROOT/'native/linux/game_load.h',ROOT/'native/linux/context_stream.h',ROOT/'native/linux/stream_observation.h']})
    if args.military_initialization_observer:identity['sources']['native/linux/stream_observation.h']=digest(ROOT/'native/linux/stream_observation.h')
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
    if random_sources:
        identity['sources'].update({name:digest(ROOT/name) for name in random_manifest['source_sha256']})
    if movement_sources:
        identity['sources'].update(movement_contract['sources'])
    if domain_sources:
        identity['sources'].update({str(q.relative_to(ROOT)):digest(q) for q in [*domain_sources,ROOT/'native/linux/domain_metadata.h',ROOT/'native/linux/domain_metadata_fixture.h']})
    if placement_sources:
        identity['sources'].update({str(q.relative_to(ROOT)):digest(q) for q in [*placement_sources,ROOT/'native/linux/placement_metadata.h',ROOT/'native/linux/placement_metadata_fixture.h']})
    if placement_handler_sources:
        identity['sources'].update(handler_sources)
        identity['sources'].update({str(q.relative_to(ROOT)):digest(q) for q in [*placement_handler_sources,ROOT/'native/linux/placement_handler_metadata.h',ROOT/'native/linux/placement_handler_metadata_fixture.h',ROOT/'native/linux/skip_turn_handler_query.h']})
    if manager_sources:
        identity['sources'].update({str(q.relative_to(ROOT)):digest(q) for q in [*manager_sources,ROOT/'native/linux/unit_operation_manager_metadata.h',ROOT/'native/linux/unit_operation_manager_metadata_fixture.h']})
    if current_handler_sources:
        identity['sources'].update({str(q.relative_to(ROOT)):digest(q) for q in [*current_handler_sources,ROOT/'native/linux/current_handler_interruption.h',ROOT/'native/linux/current_handler_metadata_fixture.h']})
    if core_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*core_sources,ROOT/'native/linux/core_component.h']})
    if dispatch_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*dispatch_sources,ROOT/'native/linux/action_dispatch.h',ROOT/'native/linux/action_state.h',ROOT/'native/linux/action_registration.h']})
    if captured_gate_path:identity['sources'][str(captured_gate_path.relative_to(ROOT))]=digest(captured_gate_path)
    for path in captured_extra_sources:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in [*current_handler_audit_paths,ROOT/'tools/linux/save_lifecycle_audit.py',ROOT/'tools/linux/script_can_start_boundary_audit.py',ROOT/'tools/linux/unit_operation_manager_metadata_audit.py',ROOT/'tools/linux/current_handler_interruption_audit.py',ROOT/'tools/linux/inspect_elf.py'] if args.current_handler_metadata else []:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in state_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in request_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in action_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in normalization_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in execution_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in [*definition_audit_paths,*unit_definition_audit_paths,*operation_registry_audit_paths,*combat_audit_paths,*random_audit_paths,*movement_audit_paths,*domain_audit_paths,*placement_audit_paths,*placement_handler_audit_paths,*manager_audit_paths,*military_audit_paths,*core_audit_paths,*main_thread_audit_paths]:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    for path in dispatch_audit_paths:identity['sources'][str(path.relative_to(ROOT))]=digest(path)
    if localization_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*localization_sources,ROOT/'native/linux/localization.h',ROOT/'tools/linux/localization_gate.py']})
    if filesystem_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*filesystem_sources,ROOT/'native/linux/filesystem.h']})
    subprocess.run(['cc','-shared','-fPIC','-Wall','-Wextra','-Werror','-DC6_MOD_ACTIVATION_PROBE_MAIN',
                    str(sources[0]),str(sources[2]),str(sources[4]),*map(str,[*filesystem_sources,*configuration_sources,*localization_sources,*snapshot_sources,*network_sources,*lifecycle_sources,*game_sources,*request_sources,*action_sources,*normalization_sources,*execution_sources,*lifecycle_observer_sources,*definition_sources,*unit_definition_sources,*operation_registry_sources,*combat_sources,*random_sources,*core_sources,*movement_sources,*domain_sources,*placement_sources,*placement_handler_sources,*manager_sources,*current_handler_sources,*dispatch_sources]),'-ldl','-o',str(preload)],check=True)
    identity['preload_sha256']=digest(preload)
    if movement_profile:movement_profile['integrated_preload_sha256']=identity['preload_sha256']
    archived_preload=None
    if core_mode:
        archived_preload=core_paths['command'].parent/'core-component-preload.so'
        with archived_preload.open('xb') as stream:stream.write(preload.read_bytes())
    if movement_mode:
        archived_preload=movement_paths['output'].parent/'movement-cost-preload.so'
        if archived_preload.exists():
            if digest(archived_preload)!=digest(preload):raise SystemExit('movement immutable preload differs')
        else:
            with archived_preload.open('xb') as stream:stream.write(preload.read_bytes())
    if args.domain_metadata:
        archived_preload=domain_paths['output'].parent/'domain-metadata-preload.so'
        if archived_preload.exists():
            if digest(archived_preload)!=digest(preload):raise SystemExit('domain immutable preload differs')
        else:
            with archived_preload.open('xb') as stream:stream.write(preload.read_bytes())
    if args.placement_metadata:
        archived_preload=placement_paths['output'].parent/'placement-metadata-preload.so'
        if archived_preload.exists():
            if digest(archived_preload)!=digest(preload):raise SystemExit('placement immutable preload differs')
        else:
            with archived_preload.open('xb') as stream:stream.write(preload.read_bytes())
    if args.placement_handler_metadata:
        archived_preload=placement_handler_paths['output'].parent/'placement-handler-metadata-preload.so'
        if archived_preload.exists():
            if digest(archived_preload)!=digest(preload):raise SystemExit('handler immutable preload differs')
        else:
            with archived_preload.open('xb') as stream:stream.write(preload.read_bytes())
    if args.unit_operation_manager_metadata:
        archived_preload=manager_paths['output'].parent/'unit-operation-manager-metadata-preload.so'
        if archived_preload.exists():
            if digest(archived_preload)!=digest(preload):raise SystemExit('Manager immutable preload differs')
        else:
            with archived_preload.open('xb') as stream:stream.write(preload.read_bytes())
    if args.current_handler_metadata:
        archived_preload=current_handler_paths['output'].parent/'current-handler-metadata-preload.so'
        if archived_preload.exists():
            if digest(archived_preload)!=digest(preload):raise SystemExit('current-handler immutable preload differs')
        else:
            with archived_preload.open('xb') as stream:stream.write(preload.read_bytes())
    if args.military_initialization_observer:
        archived_preload=military_directory/'military-stream-preload.so'
        if archived_preload.exists():
            if digest(archived_preload)!=digest(preload):raise SystemExit('Military immutable preload differs')
        else:
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
    if args.prepare_main_thread:env['C6_MOD_PREPARE_MAIN_THREAD']='1'
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
        if random_input:env.update(C6_COMBAT_RANDOM_INPUT=str(random_input),C6_COMBAT_RANDOM_OUTPUT=str(random_paths['output']),C6_COMBAT_RANDOM_CONTROL_SAVE=str(random_paths['control_save']),C6_COMBAT_RANDOM_POST_SAVE=str(random_paths['post_save']))
        if movement_mode:
            env.update(C6_MOVEMENT_COST_MODE=movement_mode,C6_MOVEMENT_COST_EXECUTE='1' if movement_mode=='execute' else '0',C6_MOVEMENT_COST_MAX_FORMATIONS=str(args.movement_cost_max_formations),C6_MOVEMENT_COST_OUTPUT=str(movement_paths['output']),C6_MOVEMENT_COST_SNAPSHOT_BEFORE=str(movement_paths['before']),C6_MOVEMENT_COST_SNAPSHOT_CONTROL=str(movement_paths['control_save']),C6_MOVEMENT_COST_SNAPSHOT_AFTER=str(movement_paths['after']),C6_MOVEMENT_COST_SNAPSHOT_REPEAT=str(movement_paths['repeat']),C6_ACTION_TRACE_DIRECTORY=str(action_trace))
        if args.domain_metadata:
            env.update(C6_DOMAIN_METADATA_OUTPUT=str(domain_paths['output']),C6_DOMAIN_METADATA_SNAPSHOT_BEFORE=str(domain_paths['before']),C6_DOMAIN_METADATA_SNAPSHOT_CONTROL=str(domain_paths['control_save']),C6_DOMAIN_METADATA_SNAPSHOT_AFTER=str(domain_paths['after']),C6_DOMAIN_METADATA_SNAPSHOT_REPEAT=str(domain_paths['repeat']),C6_ACTION_TRACE_DIRECTORY=str(action_trace))
        if args.placement_metadata:
            env.update(C6_PLACEMENT_METADATA_OUTPUT=str(placement_paths['output']),C6_PLACEMENT_METADATA_EXPECTED_OUTPUT=str(placement_paths['expected_output']),C6_PLACEMENT_METADATA_SNAPSHOT_BEFORE=str(placement_paths['before']),C6_PLACEMENT_METADATA_SNAPSHOT_CONTROL=str(placement_paths['control_save']),C6_PLACEMENT_METADATA_SNAPSHOT_AFTER=str(placement_paths['after']),C6_PLACEMENT_METADATA_SNAPSHOT_REPEAT=str(placement_paths['repeat']),C6_ACTION_TRACE_DIRECTORY=str(action_trace))
        if args.unit_operation_manager_metadata:
            for label,suffix in {'output':'OUTPUT','repeat_output':'REPEAT_OUTPUT','control_output':'CONTROL_OUTPUT','before':'SNAPSHOT_BEFORE','control_save':'SNAPSHOT_CONTROL','after':'SNAPSHOT_AFTER','repeat':'SNAPSHOT_REPEAT'}.items():
                env['C6_UNIT_OPERATION_MANAGER_METADATA_'+suffix]=str(manager_paths[label])
            env['C6_ACTION_TRACE_DIRECTORY']=str(action_trace)
        if args.current_handler_metadata:
            for label,suffix in {'output':'OUTPUT','repeat_output':'REPEAT_OUTPUT','control_output':'CONTROL_OUTPUT','before':'SNAPSHOT_BEFORE','control_save':'SNAPSHOT_CONTROL','after':'SNAPSHOT_AFTER','repeat':'SNAPSHOT_REPEAT'}.items():
                env['C6_CURRENT_HANDLER_METADATA_'+suffix]=str(current_handler_paths[label])
            env['C6_CURRENT_HANDLER_METADATA']='1'
            env['C6_ACTION_TRACE_DIRECTORY']=str(action_trace)
        if args.placement_handler_metadata:
            env.update(C6_PLACEMENT_HANDLER_METADATA_OUTPUT=str(placement_handler_paths['output']),C6_PLACEMENT_HANDLER_METADATA_EXPECTED_OUTPUT=str(placement_handler_paths['expected_output']),C6_PLACEMENT_HANDLER_METADATA_HANDLER_EXPECTED_OUTPUT=str(placement_handler_paths['handler_expected_output']),C6_PLACEMENT_HANDLER_METADATA_SNAPSHOT_BEFORE=str(placement_handler_paths['before']),C6_PLACEMENT_HANDLER_METADATA_SNAPSHOT_CONTROL=str(placement_handler_paths['control_save']),C6_PLACEMENT_HANDLER_METADATA_SNAPSHOT_AFTER=str(placement_handler_paths['after']),C6_PLACEMENT_HANDLER_METADATA_SNAPSHOT_REPEAT=str(placement_handler_paths['repeat']),C6_ACTION_TRACE_DIRECTORY=str(action_trace))
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
    timing_change=None
    if args.movement_cost_timing_baseline:
        baseline_path=args.movement_cost_timing_baseline.resolve()
        baseline=json.loads(baseline_path.read_text())
        if baseline['returncode']!=124 or baseline['configuration']['lower_core_movement_cost']['mode']!='execute':raise SystemExit('timing baseline must be the preserved cost timeout')
        prior_hash=hashlib.sha256(json.dumps(baseline['configuration'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
        if prior_hash!=baseline['configuration_sha256']:raise SystemExit('timing baseline configuration hash mismatch')
        prior=dict(baseline['configuration']);current=dict(identity)
        changed={key for key in set(prior)|set(current) if prior.get(key)!=current.get(key)}
        if changed!={'timeout_seconds','runner_sha256'} or prior['timeout_seconds']!=180 or current['timeout_seconds']!=900:
            raise SystemExit('movement timing retry changes more than supervisor budget/runner: '+str(sorted(changed)))
        timing_change={'schema':'c6-movement-cost-supervisor-only-change-v1','baseline_runtime_sha256':digest(baseline_path),'baseline_configuration_sha256':prior_hash,'configuration_sha256':configuration_identity,'allowed_changed_keys':sorted(changed),'native_source_helper_preload_candidate_inputs_identical':True,'previous_timeout_seconds':180,'timeout_seconds':900,'full_case_count_from_control':1536,'max_formations':args.movement_cost_max_formations,'quiescence_checks_weakened':False,'partial_run_accepted':False}
        (movement_paths['output'].parent/'supervisor-timing-change.json').write_text(json.dumps(timing_change,indent=2)+'\n')
    if movement_mode:env['C6_MOVEMENT_COST_PROFILE_SHA256']=configuration_identity
    if args.domain_metadata:env['C6_DOMAIN_METADATA_PROFILE_SHA256']=configuration_identity
    if args.placement_metadata:env['C6_PLACEMENT_METADATA_PROFILE_SHA256']=configuration_identity
    if args.placement_handler_metadata:env['C6_PLACEMENT_HANDLER_METADATA_PROFILE_SHA256']=configuration_identity
    if args.unit_operation_manager_metadata:env['C6_UNIT_OPERATION_MANAGER_METADATA_PROFILE_SHA256']=configuration_identity
    if args.current_handler_metadata:env['C6_CURRENT_HANDLER_METADATA_PROFILE_SHA256']=configuration_identity
    if args.military_initialization_observer:env['C6_MILITARY_STREAM_OBSERVER']='1'
    if args.prepare_only:
        args.output.write_text(json.dumps({'schema':'c6-linux-native-profile-prepared-v1','configuration':identity,'configuration_sha256':configuration_identity,'native_execution_performed':False,'returncode':None,'execution_paths':{'preload':str(archived_preload or preload),'userdata':str(userdata),'movement':{k:str(v) for k,v in movement_paths.items()},'domain_metadata':{k:str(v) for k,v in domain_paths.items()},'placement_metadata':{k:str(v) for k,v in placement_paths.items()},'placement_handler_metadata':{k:str(v) for k,v in placement_handler_paths.items()},'unit_operation_manager_metadata':{k:str(v) for k,v in manager_paths.items()},'current_handler_metadata':{k:str(v) for k,v in current_handler_paths.items()},'military_observer':{'directory':str(military_directory),'game_outputs':{k:str(v) for k,v in game_outputs.items()}} if military_directory else None},'limits':'offline build and identity preparation only; no original-code comparison or cleanup proof'},indent=2)+'\n')
        print(json.dumps({'output':str(args.output),'prepared_only':True,'native_execution_performed':False,'configuration_sha256':configuration_identity}))
        return
    try:
        if args.military_initialization_observer:
            runtime_prepared=military_directory/'runtime-prepared.json'
            admitted_prepared=military_directory/'prepared.json'
            admitted=json.loads(admitted_prepared.read_text())
            if admitted['configuration']!=identity or admitted['configuration_sha256']!=configuration_identity or admitted.get('native_execution_performed') is not False:raise SystemExit('Military admitted preparation differs')
            with runtime_prepared.open('xb') as stream:stream.write(admitted_prepared.read_bytes())
            from military_observer_runner import run as military_run
            code,out,err=military_run(host,env,work,args.timeout,military_directory,runtime_prepared,military_plan)
        else:
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
    if args.military_initialization_observer:
        report['execution_paths']['military_observer']={'directory':str(military_directory),'game_outputs':{k:str(v) for k,v in game_outputs.items()}}
        report['execution_paths']['preload']=str(archived_preload)
        report['military_observer_artifacts']={q.name:{'sha256':digest(q),'bytes':q.stat().st_size} for q in military_directory.iterdir() if q.is_file() and q.name in ('observations.jsonl','transcript.txt','exit.json','capture.gdb','runtime-plan.json','runtime-prepared.json','supervisor.json','inferior.json','disposal.json')}
    report['supervisor_timing_change']=timing_change
    report.update(lifecycle_report(code,err))
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
    report['movement_cost_output']=None
    report['movement_cost_saves']={}
    if movement_mode:
        report['execution_paths']['movement']={k:str(v) for k,v in movement_paths.items()}
        report['execution_paths']['preload']=str(archived_preload)
        for label,path in movement_paths.items():
            if path.is_file():
                artifact={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
                if label=='output':report['movement_cost_output']=artifact
                else:report['movement_cost_saves']['control' if label=='control_save' else label]=artifact
    report['domain_metadata_output']=None
    report['domain_metadata_saves']={}
    if args.domain_metadata:
        report['execution_paths']['domain_metadata']={k:str(v) for k,v in domain_paths.items()}
        report['execution_paths']['preload']=str(archived_preload)
        for label,path in domain_paths.items():
            if path.is_file():
                artifact={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
                if label=='output':report['domain_metadata_output']=artifact
                else:report['domain_metadata_saves']['control' if label=='control_save' else label]=artifact
    report['placement_metadata_output']=None
    report['placement_metadata_expected_output']=None
    report['placement_metadata_saves']={}
    if args.placement_metadata:
        report['execution_paths']['placement_metadata']={k:str(v) for k,v in placement_paths.items()}
        report['execution_paths']['preload']=str(archived_preload)
        for label,path in placement_paths.items():
            if path.is_file():
                artifact={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
                if label=='output':report['placement_metadata_output']=artifact
                elif label=='expected_output':report['placement_metadata_expected_output']=artifact
                else:report['placement_metadata_saves']['control' if label=='control_save' else label]=artifact
    report['placement_handler_metadata_output']=None
    report['placement_handler_count_expected_output']=None
    report['placement_handler_expected_output']=None
    report['placement_handler_metadata_saves']={}
    if args.placement_handler_metadata:
        report['execution_paths']['placement_handler_metadata']={k:str(v) for k,v in placement_handler_paths.items()}
        report['execution_paths']['preload']=str(archived_preload)
        for label,path in placement_handler_paths.items():
            if path.is_file():
                artifact={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
                if label=='output':report['placement_handler_metadata_output']=artifact
                elif label=='expected_output':report['placement_handler_count_expected_output']=artifact
                elif label=='handler_expected_output':report['placement_handler_expected_output']=artifact
                else:report['placement_handler_metadata_saves']['control' if label=='control_save' else label]=artifact
    report['unit_operation_manager_metadata_artifacts']={}
    if args.unit_operation_manager_metadata:
        report['execution_paths']['unit_operation_manager_metadata']={k:str(v) for k,v in manager_paths.items()}
        report['execution_paths']['preload']=str(archived_preload)
        for label,path in manager_paths.items():
            if path.is_file():report['unit_operation_manager_metadata_artifacts'][label]={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
    report['current_handler_metadata_artifacts']={}
    if args.current_handler_metadata:
        report['execution_paths']['current_handler_metadata']={k:str(v) for k,v in current_handler_paths.items()}
        report['execution_paths']['preload']=str(archived_preload)
        for label,path in current_handler_paths.items():
            if path.is_file():report['current_handler_metadata_artifacts'][label]={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
    report['combat_random_output']=None
    report['combat_random_saves']={}
    if random_input:
        report['execution_paths']['combat_random_input']=str(random_input)
        report['execution_paths']['combat_random']={k:str(v) for k,v in random_paths.items()}
        for label,path in random_paths.items():
            if path.is_file():
                artifact={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
                if label=='output':report['combat_random_output']=artifact
                else:report['combat_random_saves'][label.removesuffix('_save')]=artifact
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
