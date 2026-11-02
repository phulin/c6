#!/usr/bin/env python3
"""Validate one actual original Linux read-only actor/legality observation report."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import GAME_SHA, HOST_SHA


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-report', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    runtime = json.loads(args.runtime_report.read_text())
    configuration = runtime['configuration']
    canonical = json.dumps(configuration, sort_keys=True, separators=(',', ':')).encode()
    if hashlib.sha256(canonical).hexdigest() != runtime['configuration_sha256']:
        raise ValueError('runtime configuration identity differs')
    identities = json.dumps(configuration['assets'])
    if GAME_SHA not in identities or HOST_SHA not in identities:
        raise ValueError('missing active original module SHA gates')
    if runtime['returncode'] != 0 or not runtime['framework_cleanup_proven'] or not runtime['client_cleanup_proven']:
        raise ValueError('native run or genuine cleanup failed')
    observations = []
    for line in runtime['stderr'].splitlines():
        if line.startswith('C6_LINUX_ACTION_OBSERVER '):
            observations.append(json.loads(line.split(' ', 1)[1]))
    before = [x for x in observations if x['stage'] == 'before']
    player = [x for x in observations if x['stage'] == 'player']
    after = [x for x in observations if x['stage'] == 'after']
    actors = [x for x in observations if x['stage'] == 'unit']
    if len(before) != 1 or len(player) != 1 or len(after) != 1:
        raise ValueError('requires exactly one complete observer pass')
    before, player, after = before[0], player[0], after[0]
    if after['result'] != 0 or after['action_executed'] or after['full_state_equivalence']:
        raise ValueError('observer failure or unsupported claims')
    for key in ('pending_messages', 'needs_update', 'needs_turn_advance', 'ai_processing', 'turn_processing'):
        if before[key] != 0 or after[key] != 0:
            raise ValueError('unsettled native observer boundary: '+key)
    for key in ('current_turn', 'current_phase', 'local_player'):
        if before[key] != after[key]:
            raise ValueError('read-only observer boundary changed: '+key)
    if player['unit_count'] != len(actors) or after['units_observed'] != len(actors):
        raise ValueError('native unit iteration incomplete')
    if [x['index'] for x in actors] != list(range(len(actors))):
        raise ValueError('native iteration ordering differs')
    if any(x['owner'] != player['local_player'] or x['hash'] != '2c681908' for x in actors):
        raise ValueError('observed actor owner or pinned operation differs')
    if sum(x['can_start'] for x in actors) != after['legal_skip_turn']:
        raise ValueError('legality result count differs')
    saves = {}
    for label in ('save', 'repeat'):
        path = Path(runtime['execution_paths']['game_outputs'][label])
        actual = dict(sha256=digest(path), bytes=path.stat().st_size)
        expected = runtime['game_saves'][label]
        if any(actual[k] != expected[k] for k in actual):
            raise ValueError('captured original Save identity differs: '+label)
        saves[label] = actual
    if saves['save'] != saves['repeat']:
        raise ValueError('native serialized core state changed during queries')
    candidates = [dict(owner=x['owner'],id=x['id'],component_bits=x['component_bits']) for x in actors
                  if x['can_start'] and not x['dead'] and not x['delayed_death']]
    result = dict(schema='c6-linux-action-observer-native-v1',
                  runtime_report_sha256=digest(args.runtime_report),
                  configuration_sha256=runtime['configuration_sha256'],
                  host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
                  before=before,player=player,after=after,
                  actor_count=len(actors),legal_skip_turn_candidates=candidates,
                  candidates_are_human_active=bool(player['human'] and player['turn_active']),
                  original_core_saves=saves,serialized_core_unchanged=True,
                  wrapper_and_factory_cleanup=True,native_action_executed=False,
                  full_host_finalization=False,full_state_equivalence=False,gameplay_parity=False,
                  limits='Actual lower-Core-loaded fixture read-only unit iteration and SKIP_TURN predicate only. Save equality covers serialized Core state, not all caches, jobs, queues, reporting subscribers, host state or excluded planning data. No action, message drain, Update or decision-policy execution performed by this observer.')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(actor_count=len(actors),legal_candidates=len(candidates),serialized_core_unchanged=True)))


if __name__ == '__main__':
    main()
