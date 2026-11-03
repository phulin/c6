#!/usr/bin/env python3
"""Assemble a raw static plan from an actual CLI built report and extraction.

Does not prepare/launch a native profile. Actual normal CLI prepare and independent
plan review remain external; all copied identities are checked before output.
"""
import argparse
import hashlib
import json
from pathlib import Path
import tempfile

from full_manager_observer_runner import ROOT,canonical,compiled_contracts,validate_plan,runtime_module_contracts
from save_lifecycle_audit import GAME_SHA,HOST_SHA


def require(test,reason):
    if not test:raise ValueError(reason)


def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def launch_host(configuration,explicit=None,expected_sha256=HOST_SHA):
    filesystem=configuration.get('original_filesystem_profile',{})
    configured=filesystem.get('launch_host')
    require(isinstance(configured,str) and Path(configured).is_absolute(),
            'actual configured absolute launch_host is required; asset-copy inference forbidden')
    host=Path(configured).resolve()
    if explicit is not None:
        require(Path(explicit).resolve()==host,'explicit launch host differs from configured launch path')
    require(digest(host)==expected_sha256,'actual configured launch host bytes differ')
    copied=filesystem.get('validated_launch',{}).get('copied_host',{})
    require(copied.get('sha256')==expected_sha256,'configured launch fixture host identity differs')
    return str(host)


def add_script_snapshot(fields,host,sources):
    """Add only the explicitly selected collector and its host layout gates."""
    from script_state_snapshot import SCHEMA,PHASES,ARGUMENT_BODIES,validate_spec
    source=str(ROOT/'tools/linux/script_state_snapshot.py');identity=digest(source)
    require(sources.get(source)==identity,'opt-in collector absent/changed in actual built source inventory')
    spec=dict(schema=SCHEMA,phases=list(PHASES),byte_budget=16*1024*1024,
              max_items=65536,source_sha256=identity)
    validate_spec(spec)
    bodies=list(fields['live_bodies'])
    for name,address,size,sha in ARGUMENT_BODIES:
        row=dict(name=name,module=host,relative_address=address,size=size,sha256=sha)
        matches=[old for old in bodies if old['module']==host and old['relative_address']==address]
        require(not matches or all(old['size']==size and old['sha256']==sha for old in matches),
                'conflicting argument layout body in compiled fragment')
        if not matches:bodies.append(row)
    return dict(fields,live_bodies=bodies,script_snapshot=spec)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--built-report',type=Path,required=True)
    parser.add_argument('--fragment',type=Path,required=True)
    parser.add_argument('--output-directory',type=Path,required=True)
    parser.add_argument('--launch-host',type=Path,help='Optional explicit check against configured actual launch path')
    parser.add_argument('--script-snapshot',action='store_true',
                        help='Opt in to reviewed bounded natural script-state images; GC stays unresolved')
    args=parser.parse_args()
    built_raw=args.built_report.read_bytes();built=json.loads(built_raw)
    require(built['schema']=='c6-linux-full-manager-preload-built-v1' and
            built['native_execution_performed'] is False,'expected unexecuted actual CLI built report')
    fragment_raw=args.fragment.read_bytes();fragment=json.loads(fragment_raw)
    require(fragment['schema']=='c6-linux-full-manager-compiled-plan-fragment-v1' and
            fragment['preload_sha256']==built['preload_sha256'] and
            fragment['preload']==str(Path(built['preload']).resolve()),'fragment/actual built preload identity differs')
    require(digest(built['preload'])==built['preload_sha256'],'actual built preload changed')
    sources={str((ROOT/name).resolve()):sha for name,sha in built['sources'].items()}
    sources[str(ROOT/'tools/linux/mod_activation_probe.py')]=built['runner_sha256']
    for path,sha in sources.items():require(digest(path)==sha,'actual built source changed: '+path)
    cfg=built['configuration_without_final_plan']
    require(cfg['sources']==built['sources'] and cfg['runner_sha256']==built['runner_sha256'],
            'built configuration/source map differs')
    host=launch_host(cfg,args.launch_host)
    runtime_paths,modules=runtime_module_contracts(cfg,built['preload'])
    require(host==runtime_paths['host'],'actual launch host role contract differs')
    require(fragment['preload']==runtime_paths['preload'],'actual preload role contract differs')
    core=runtime_paths['core']
    out=args.output_directory.resolve();out.mkdir(parents=True,exist_ok=False)
    fields=('caller_suffix_frames','marker_parent_frames','window_parent_prefixes','live_bodies',
            'compiled_symbols','main_entry_witness')
    values={key:fragment[key] for key in fields}
    if args.script_snapshot:values=add_script_snapshot(values,host,sources)
    packet=dict(schema='c6-linux-full-manager-stack-contract-v1',observation_mode='raw_unresolved',
        reviewed_sources=sources,no_VM_or_GC_parent_frames_source_closed=False,
        **values,original_execution=False,native_profile_admitted=False,
        construction='Actual immutable compiled owned-ELF facts; assembled for independent final plan review.',
        additional_startup_HKS_GC_frames='unresolved; no zero-parent or registry/effects inference',
        built_report_sha256=hashlib.sha256(built_raw).hexdigest(),
        extraction_fragment_sha256=hashlib.sha256(fragment_raw).hexdigest(),
        assembler_sha256=digest(__file__))
    packet_path=out/'stack-contract.json';packet_path.write_text(json.dumps(packet,indent=2,sort_keys=True)+'\n')
    plan=dict(schema='c6-linux-full-manager-observer-plan-v1',profile_kind='genuine_lower_core_full_Manager',
        observation_mode='raw_unresolved',host=host,core=core,preload=fragment['preload'],libc=runtime_paths['libc'],modules=modules,
        marker_relative_address=fragment['marker_relative_address'],
        **values,
        stack_contract_review=dict(path=str(packet_path),sha256=digest(packet_path)))
    compiled_contracts(plan,Path(plan['preload']).read_bytes())
    plan_path=out/'observer-plan.json';plan_path.write_text(json.dumps(plan,indent=2,sort_keys=True)+'\n')
    # Exercise the real adapter validator with an explicitly synthetic temporary
    # envelope. This is not the normal CLI-prepared profile and cannot become
    # a retained native runtime gate by accident.
    with tempfile.TemporaryDirectory(prefix='c6-static-plan-validation-') as temporary:
        temporary=Path(temporary)
        synthetic_cfg=json.loads(json.dumps(cfg))
        synthetic_cfg['lower_core_full_manager_observer']['static_plan']=plan
        synthetic_cfg['lower_core_full_manager_observer']['static_plan_sha256']=digest(plan_path)
        synthetic_cfg['preload_sha256']=built['preload_sha256']
        profile=hashlib.sha256(canonical(synthetic_cfg)).hexdigest()
        prepared=temporary/'SYNTHETIC-NOT-CLI-PREPARED.json'
        prepared.write_text(json.dumps(dict(schema='c6-linux-native-profile-prepared-v1',
            configuration=synthetic_cfg,configuration_sha256=profile,native_execution_performed=False,
            returncode=None,execution_paths=dict(preload=plan['preload']),synthetic_validation_only=True)))
        runtime=dict(plan,runtime_gate=dict(report_path=str(prepared),report_sha256=digest(prepared),
            configuration_sha256=profile,source_files=sources,static_plan_path=str(plan_path)))
        validate_plan(runtime)
    summary=dict(schema='c6-linux-full-manager-final-static-plan-v1',original_execution=False,
        actual_CLI_built_preload_sha256=built['preload_sha256'],source_files=len(sources),
        actual_configured_launch_host=host,launch_host_sha256=HOST_SHA,
        static_plan_path=str(plan_path),static_plan_sha256=digest(plan_path),
        stack_contract_path=str(packet_path),stack_contract_sha256=digest(packet_path),
        actual_compiled_symbols=len(plan['compiled_symbols']),own_caller_frames=len(plan['caller_suffix_frames']),
        marker_phases=len(plan['marker_parent_frames']),module_files=len(modules),
        real_adapter_offline_schema_symbol_source_validation=True,
        actual_CLI_prepare_performed=False,normal_prepared_profile_and_independent_final_review_pending=True,
        callback_registry_decoded=False,full_manager_parity=False,complete_state_actions_rng_turns=False)
    (out/'built-report.json').write_bytes(built_raw)
    (out/'extractor-fragment.json').write_bytes(fragment_raw)
    (out/'assembler.py').write_bytes(Path(__file__).read_bytes())
    (out/'summary.json').write_text(json.dumps(summary,indent=2,sort_keys=True)+'\n')
    print(json.dumps(summary))


if __name__=='__main__':main()
