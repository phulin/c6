#!/usr/bin/env python3
"""Read-only byte contracts for mixed Linux turn activation and technology policy.

This audit executes no native game code and establishes no resumable boundary.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf, GAME_SHA

CALLS = [
    (0x2ba1077, 0x1e41590, 'player_activation_enters_AI_control_activation'),
    (0x201e941, 0x1f3c7f0, 'activation_dispatches_offturn_signal'),
    (0x201e962, 0x1f4b450, 'activation_waits_offturn_jobs'),
    (0x201eacc, 0x1fa1e50, 'activation_calls_priority_subscribers'),
    (0x201e6b7, 0x1e66210, 'begin_turn_dispatches_priority_signal'),
    (0x1fcf8f3, 0x1e1e000, 'city_manager_subscribes_offturn'),
    (0x2120fb5, 0x1e1e000, 'grand_strategy_subscribes_offturn'),
    (0x2122fd0, 0x1f5f3f0, 'grand_strategy_submits_job'),
    (0x2123019, 0x1f5f3f0, 'grand_strategy_submits_region_job'),
    (0x212302d, 0x1f564d0, 'grand_strategy_runs_jobs'),
    (0x1fd8dc4, 0x1f5f3f0, 'city_build_submits_job'),
    (0x1fd8dcc, 0x1f564d0, 'city_build_runs_jobs'),
    (0x20fb05f, 0x1ec0f30, 'global_AI_queues_original_job_with_jobset'),
    (0x20faa3f, 0x1e69ea0, 'global_AI_waits_original_jobset'),
    (0x2185e9f, 0x1e87800, 'research_activation_checks_human'),
    (0x2185eb0, 0x1e6cad0, 'research_activation_enters_technology_policy'),
    (0x218612e, 0x1e87800, 'technology_policy_independently_checks_human'),
    (0x21861e5, 0x1fb8ef0, 'analysis_choice_checks_original_CanResearch'),
    (0x2186224, 0x1fb7080, 'analysis_choice_commits_research_queue'),
    (0x2186277, 0x1f03f80, 'fallback_constructs_original_game_RNG_delegate'),
    (0x2186307, 0x1fb8900, 'fallback_excludes_owned_technologies'),
    (0x2186319, 0x1fb8ef0, 'fallback_checks_original_CanResearch'),
    (0x218632d, 0x1e0f800, 'fallback_collects_native_technology_ids'),
    (0x2189f20, 0x1e6d460, 'random_choice_invokes_original_delegate'),
    (0x1fe524f, 0x1f22910, 'game_RNG_edits_original_Game_Random'),
    (0x2186360, 0x1ecd240, 'fallback_makes_random_policy_choice'),
    (0x218638d, 0x1fb7080, 'fallback_commits_research_queue'),
    (0x2913590, 0x1f04c20, 'pending_diplomacy_processes_AI_statement'),
    (0x291197c, 0x1f48180, 'pending_diplomacy_removes_next_statement'),
    (0x291199d, 0x1f3ccf0, 'pending_diplomacy_processes_removed_statement'),
    (0x2917af5, 0x1e9a1c0, 'session_queue_pops_before_processing'),
    (0x2917b00, 0x1f8af60, 'session_queue_processes_owned_statement'),
]
BYTES = [
    (0x2186133, '84c00f85a2020000', 'QueueNextTech_human_returns_before_selection'),
    (0x2185ea8, 'beffffffff', 'activation_completedTech_sentinel_minus_one'),
    (0x2186218, '31d2', 'analysis_insert_mode_zero'),
    (0x1fe5262, 'e98950e9ff', 'game_RNG_tailcalls_original_Random_Get'),
    (0x2185ea4, '84c0750d', 'human_skips_activation_QueueNextTech_call'),
    (0x2186322, 'ba01000000', 'fallback_candidate_weight_one'),
    (0x2186388, '31d2', 'fallback_insert_mode_zero'),
    (0x201eaf5, '4183be100100000079a1', 'activation_continues_priority_loop'),
    (0x2913580, '84c074dc', 'pending_diplomacy_skips_non_AI_or_finished'),
]
POINTERS = [
    (0x3b1de78, 0x1fcfc60, 'city_manager_offturn_subscription'),
    (0x3b20990, 0x2122f70, 'grand_strategy_offturn_subscription'),
    (0x3b1f608, 0x2185a70, 'research_activation_callback'),
    (0x3b1c648, 0x1fe5240, 'fallback_uses_Game_GetRandNum'),
]


def symbol_pointer(game, address):
    """Resolve GLOB_DAT through this ELF's own defined dynamic symbol."""
    start = struct.unpack_from('<Q', game.raw, 40)[0]
    width, count = struct.unpack_from('<HH', game.raw, 58)
    sections = [struct.unpack_from('<IIQQQQIIQQ', game.raw, start+i*width) for i in range(count)]
    for section in sections:
        if section[1] != 4:
            continue
        symbols = sections[section[6]]
        for pos in range(section[4], section[4]+section[5], section[9]):
            offset, info, addend = struct.unpack_from('<QQq', game.raw, pos)
            if offset != address:
                continue
            if info & 0xffffffff != 6:
                raise ValueError('expected GLOB_DAT')
            symbol = struct.unpack_from('<IBBHQQ', game.raw, symbols[4]+(info >> 32)*symbols[9])
            if symbol[3] == 0:
                raise ValueError('undefined callback symbol')
            return symbol[4]+addend
    raise ValueError('missing callback relocation')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for address, target, _ in CALLS:
        game.call(address, target)
    for address, expected, name in BYTES:
        if game.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError(f'byte contract mismatch: {name}')
    for address, target, name in POINTERS:
        if symbol_pointer(game, address) != target:
            raise ValueError(f'relocation mismatch: {name}')
    report = dict(schema=1, method='SHA-gated static Linux ELF calls, branches and relocated pointers; no native execution',
                  gamecore_sha256=GAME_SHA, checks_passed=True,
                  direct_calls=[dict(address=hex(a), target=hex(t), contract=n) for a,t,n in CALLS],
                  byte_contracts=[dict(address=hex(a), contract=n) for a,_,n in BYTES],
                  pointer_contracts=[dict(address=hex(a), target=hex(t), contract=n) for a,t,n in POINTERS],
                  mixed_policy_body=dict(address='0x2186110', size=0x2fc,
                      sha256=hashlib.sha256(game.read(0x2186110, 0x2fc)).hexdigest()),
                  lowest_proven_choice='AI::Research::QueueNextTech: analyzed technology or native RNG fallback, then original SetResearchingTech',
                  offturn_callback_classification='unknown: proven subscriptions, state writes and job scheduling do not establish planning-only status',
                  runtime_executed=False, suspend_resume_proven=False, complete_decision_coverage=False,
                  gameplay_rng_exclusion_proven=False, action_or_turn_parity=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, calls=len(CALLS), branches=len(BYTES), pointers=len(POINTERS))))


if __name__ == '__main__':
    main()
