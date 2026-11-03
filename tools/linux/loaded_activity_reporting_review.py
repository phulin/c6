#!/usr/bin/env python3
"""Independently review the frozen first activity report; no original execution.

A supplied replay is output from the retained, hash-verified Rust checker. This
never labels that replay as a rebuild from the deleted first source snapshot.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
COMMIT = '94910318109c9ba7b759bde8cd69c67a4807d7b2'
CANDIDATE = '2667fe4b21b2ac68dc22e097c1cdb36bd9c30606571cb1c3c5a1b5580740496e'
CHECK_SHA = 'a584655cc71ec2dafac9b5c88fe4189534317bacf53287eb1890f97418979658'
SOURCE_SHA = '8a6086acca7aae66b029e2bc2959799e96e9a9a8374b4a396f8337c76788117e'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def frozen(path):
    return subprocess.check_output(['git', 'show', COMMIT+':'+path], cwd=ROOT)


def require(value, message):
    if not value:
        raise ValueError(message)


def review(assets, replay_path, executable):
    check_bytes = (ROOT/'evidence/linux/loaded-activity-reporting-check.json').read_bytes()
    source_bytes = (ROOT/'evidence/linux/loaded-activity-reporting-source.json').read_bytes()
    require(sha(check_bytes) == CHECK_SHA, 'first checkpoint changed')
    require(sha(source_bytes) == SOURCE_SHA, 'first static source report changed')
    check, source = json.loads(check_bytes), json.loads(source_bytes)
    images = {'game': Elf(assets/'libGameCore_XP2.so', GAME_SHA),
              'host': Elf(assets/'Civ6', HOST_SHA)}
    for method in source['methods']:
        require(sha(images[method['image']].read(int(method['rva'],16), method['bytes'])) == method['sha256'],
                'pinned original method differs: '+method['name'])
    provider = frozen('crates/c6-data/src/loaded_actions/reporting.rs')
    checker = frozen('tools/linux/loaded_activity_reporting_check.rs')
    for name, data in [('crates/c6-data/src/loaded_actions/reporting.rs', provider),
                       ('tools/linux/loaded_activity_reporting_check.rs', checker),
                       ('crates/c6-data/src/unit_movement_state.rs', frozen('crates/c6-data/src/unit_movement_state.rs'))]:
        require(sha(data) == check['source_sha256'][name], 'frozen owned source differs: '+name)
    require(sha(provider) == source['source_sha256'], 'static audit provider source differs')
    # These encoded branches precede every formation/CanCyclePast continuation.
    game = images['game']
    require(game.read(0x2c281ab, 2) == bytes.fromhex('7528'), 'pending operation false branch differs')
    require(game.read(0x2c28038, 6) == bytes.fromhex('0f8496000000'), 'ReadyMove short circuit differs')
    require(game.read(0x2c28139, 2) == bytes.fromhex('7518'), 'adjacent true bypass differs')
    require(check['actual_snapshot_path'] == dict(activity=0x6b25012e, path='MemberNotReady', pending_nodes=1, remaining_q8=256),
            'actual first snapshot path differs')
    require(check['source_actor'] == dict(owner=0,id=15794187), 'actual actor differs')
    require(check['synchronous_activity_publications'] == [dict(activity=0x6b25012e,id=15794187,owner=0)],
            'first publication differs')
    reports = [r for r in check['partial_actual_entrypoint_trace'] if r['kind']=='report']
    require([r['type_bits'] for r in reports] == ['00000000e23b112d','00000000a60328c5'],
            'partial report ordering differs')
    require(check['negative_shape_identity_playback_checks'] and check['immutable_source_snapshot']
            and check['root_changed_after_snapshot'], 'first scope/provenance flags differ')
    require(not check['original_execution'] and not check['native_predicate_or_action_comparison']
            and not check['full_live_affected_state_rng_parity'], 'first comparison scope differs')
    require(check['candidate_sha256'] == CANDIDATE, 'first Rust executable identity differs')
    # Compare the metadata's frozen preimage identities with what its named
    # commit actually retains. Do not read changing workspace files as preimages.
    paths = set(subprocess.check_output(['git','ls-tree','-r','--name-only',COMMIT],cwd=ROOT,text=True).splitlines())
    matches, absent, differs = 0, [], []
    for name, expected in check['source_sha256'].items():
        if name not in paths:
            absent.append(name)
        elif sha(frozen(name)) != expected:
            differs.append(name)
        else:
            matches += 1
    replay = None
    if replay_path:
        require(executable is not None and sha(executable.read_bytes()) == CANDIDATE,
                'retained Rust executable differs')
        replay_bytes = replay_path.read_bytes()
        actual = json.loads(replay_bytes)
        require(all(check.get(key) == value for key,value in actual.items()),
                'retained binary replay differs from first checkpoint')
        replay = dict(executable_sha256=CANDIDATE, output_sha256=sha(replay_bytes),
                      fields_equal=len(actual), source_rebuild=False, original_execution=False)
    return dict(schema='c6-linux-loaded-activity-reporting-independent-review-v1',
        reviewed_commit=COMMIT, api_commit='974430329059ba993da2eeca0bb1aa000adfbde8',
        checkpoint_sha256=CHECK_SHA, static_report_sha256=SOURCE_SHA,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, original_method_bodies_checked=len(source['methods']),
        accepted_boundary='first UnitActivityChanged report: actual old MoveTo pending, MemberNotReady, then event35 publication',
        native_pending_branch='0x2c281ab ->0x2c281d5 false before formation or CanCyclePast',
        exercised_adjacency=False, exercised_hold_cycling=False, exercised_plot_movement_cost=False,
        authoritative_state_observed_at_report=dict(activity=0x6b25012e, pending_nodes=1, remaining_q8=256),
        partial_failure_retains_action_prefix=True, unsupported_next_report='a60328c5',
        publication_is_copied_identity_and_int=True, playback_consumers_available=False,
        scope_admission='StockHeadlessNoAddedConsumers is a caller condition; supplied Lua alone cannot prove it',
        omitted_outputs=['proven reflected Unit output stream', 'stock visual UnitFlag dirty history'],
        unexercised_source_review='no concrete adjacency success defect found; general placement, callback fallbacks and native parity not accepted',
        historical_snapshot=dict(immutable_source_snapshot=True, root_changed_after_snapshot=True,
            frozen_source_files=len(check['source_sha256']), matching_commit_preimages=matches,
            commit_preimages_differ=sorted(differs), commit_preimages_absent=sorted(absent),
            complete_source_reconstruction_available=False,
            limitation='first temporary source directory was deleted; metadata hashes and retained binary do not recover missing source preimages'),
        retained_binary_replay=replay, original_execution=False, native_action_comparison=False,
        complete_state_actions_turns_rng=False)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--replay-json',type=Path)
    p.add_argument('--retained-executable',type=Path)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    result=review(a.assets,a.replay_json,a.retained_executable)
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(result,sort_keys=True,indent=2)+'\n')
    print(json.dumps(dict(reviewed_commit=COMMIT,method_bodies=result['original_method_bodies_checked'],
        replayed=result['retained_binary_replay'] is not None,original_execution=False)))


if __name__ == '__main__':
    main()
