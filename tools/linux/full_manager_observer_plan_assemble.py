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

from full_manager_observer_runner import ROOT,canonical,compiled_contracts,validate_plan
from save_lifecycle_audit import GAME_SHA,HOST_SHA


def require(test,reason):
    if not test:raise ValueError(reason)


def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--built-report',type=Path,required=True)
    parser.add_argument('--fragment',type=Path,required=True)
    parser.add_argument('--output-directory',type=Path,required=True)
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
    host=str((ROOT.parent/'c6-assets-linux/Civ6').resolve())
    core=str((ROOT.parent/'c6-assets-linux/libGameCore_XP2.so').resolve())
    modules={host:dict(sha256=HOST_SHA),core:dict(sha256=GAME_SHA),
             fragment['preload']:dict(sha256=fragment['preload_sha256'])}
    # libc is definitely a host/preload dependency; other optional startup
    # frames can remain raw/unclassified rather than becoming invented roots.
    libc=cfg['runtime_dependencies'].get('libc.so.6')
    if libc:modules[str(Path(libc['path']).resolve())]=dict(sha256=libc['sha256'])
    for path,row in modules.items():require(digest(path)==row['sha256'],'static module file identity differs: '+path)
    out=args.output_directory.resolve();out.mkdir(parents=True,exist_ok=False)
    fields=('caller_suffix_frames','marker_parent_frames','window_parent_prefixes','live_bodies',
            'compiled_symbols','main_entry_witness')
    packet=dict(schema='c6-linux-full-manager-stack-contract-v1',observation_mode='raw_unresolved',
        reviewed_sources=sources,no_VM_or_GC_parent_frames_source_closed=False,
        **{key:fragment[key] for key in fields},original_execution=False,native_profile_admitted=False,
        construction='Actual immutable compiled owned-ELF facts; assembled for independent final plan review.',
        additional_startup_HKS_GC_frames='unresolved; no zero-parent or registry/effects inference',
        built_report_sha256=hashlib.sha256(built_raw).hexdigest(),
        extraction_fragment_sha256=hashlib.sha256(fragment_raw).hexdigest(),
        assembler_sha256=digest(__file__))
    packet_path=out/'stack-contract.json';packet_path.write_text(json.dumps(packet,indent=2,sort_keys=True)+'\n')
    plan=dict(schema='c6-linux-full-manager-observer-plan-v1',profile_kind='genuine_lower_core_full_Manager',
        observation_mode='raw_unresolved',host=host,core=core,preload=fragment['preload'],modules=modules,
        marker_relative_address=fragment['marker_relative_address'],
        **{key:fragment[key] for key in fields},
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
