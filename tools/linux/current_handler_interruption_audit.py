#!/usr/bin/env python3
"""Audit current-handler interruption against pinned Linux bytes and closed captures.

Captured identities plus original-image dispatch predictions only; runtime method
slots are absent. No original execution, predicate comparison or mutation.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import unittest
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
FACTS_SHA = '3fd5b6e64b5f81e06fc06187b9dae53691ddb536a6fc0b03f6f728d759dea585'
RUNTIME_SHA = '5b68cfff7a3c88033694df0ef5f882a43dc780700e5ca4b194e312ca115b3740'
REGISTRY_SHA = 'd8a43667523e9385bfa9ac35594027609b26d7755551c0bab46d226917899901'
METHODS = [
    ('Manager.CanCurrentOperationBeInterrupted', 0x2da2a60, 116, '6b2b1cbb8b18460cf4fb4f4f7a8000cd6f42ae2108cab549666eb1a387772ebe'),
    ('Manager.GetHandler.const', 0x2da1650, 82, 'a6c3ddb3a4c632585f054c52a76e6a0377372006cfa26f686dda45efe145ad13'),
    ('Manager.GetHandlerFromHash.const', 0x2da16f0, 40, '68cbcbc6d76334d0b491a466d0e0738d8b271329af36a2e42d89db3fb0e0da64'),
    ('Manager.GetHandlerIDFromHash', 0x2da1720, 86, 'e67620512f7440c1ca5486dc10569f19d23e2e7bc7ff46a97d7174146b9a7a86'),
    ('HashID.ctor.int', 0x27807f0, 3, 'e57f6082d61fbfc2f1375ebe719921bae81e81ff55cb3522867874eaaf77ff16'),
    ('HashID.IsIndex', 0x270bbb0, 10, 'ce1dd75acdae031898267e299d8f0aaac9c1abef458d6f23dd9f4476624ce95d'),
    ('BaseHandler.CanBeInterrupted', 0x2d95660, 3, 'e18aacce29affcdddaa5f641ac1ec12552e0fe9d066e6408941b1792818b8619'),
    ('MakeTradeRoute.CanBeInterrupted', 0x2da0920, 3, '251447ee91a9067dcd6ab96703133f617565974cd6c4819021760c4688c91abf'),
]
BINDINGS = [(0x88, 0x2da1650), (0x98, 0x2da16f0),
            (0xa0, 0x2da1720), (0xd8, 0x2da2a60)]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def checked_rows(raw, repeat):
    require(sha(raw) == FACTS_SHA, 'closed metadata identity mismatch')
    require(raw == repeat, 'complete repeated metadata mismatch')
    rows = [json.loads(line) for line in raw.splitlines()]
    profiles = [r for r in rows if r['kind'] == 'profile']
    definitions = [r for r in rows if r['kind'] == 'operation_definitions']
    require(len(profiles) == len(definitions) == 1, 'ambiguous metadata owners')
    definitions = definitions[0]
    require(definitions['count'] == definitions['handler_slots'] == len(definitions['entries']) == 63,
            'incomplete actual registry')
    return profiles[0], definitions['entries']


def checked_entry(row, original, module_base, target):
    i = row['ordinal']
    require(row['collection_index'] == row['runtime_id'] == row['handler_id'] == i,
            'runtime/index distinction unresolved')
    require(row['handler_pointer'] != 0 and row['hash'] == row['handler_hash'],
            'handler identity unresolved')
    require(original['id'] == original['resolved_id'] == original['handler_id'] == i
            and original['handler_present'] and original['lookup_matches']
            and original['hash'] == original['handler_hash'] == row['hash'],
            'independent registry identity mismatch')
    vptr = row['handler_vptr'] - module_base
    require(vptr == original['vtable_rva'], 'independent concrete vptr mismatch')
    require(target in (0x2d95660, 0x2da0920), 'unsupported interruption virtual')
    require((target == 0x2da0920) == (i == 18 and row['hash'] == 0x8374d954
                                    and vptr == 0x3b03960),
            'trade-route family identity mismatch')
    return dict(runtime_id=i, operation_hash=f'{row["hash"]:08x}',
                observed_vptr_rva=hex(vptr), predicted_slot_offset='0x20', predicted_target_rva=hex(target),
                runtime_slot_observed=False,
                target_provenance='original-image ELF relocation; runtime interposition not excluded',
                predicted_leaf_result=target == 0x2d95660,
                predicted_leaf_effects='constant leaf; no object reads, memory writes, calls, allocation or RNG')


class RejectionTests(unittest.TestCase):
    def test_wrong_capture_identity(self):
        with self.assertRaisesRegex(ValueError, 'identity'):
            checked_rows(b'{}\n', b'{}\n')

    def test_unsupported_virtual(self):
        row = dict(ordinal=0, collection_index=0, runtime_id=0, handler_id=0,
                   handler_pointer=1, hash=55, handler_hash=55, handler_vptr=100)
        original = dict(id=0, resolved_id=0, handler_id=0, handler_present=True,
                        lookup_matches=True, hash=55, handler_hash=55, vtable_rva=100)
        with self.assertRaisesRegex(ValueError, 'unsupported'):
            checked_entry(row, original, 0, 123)
        row['runtime_id'] = 55
        with self.assertRaisesRegex(ValueError, 'distinction'):
            checked_entry(row, original, 0, 0x2d95660)
        row['runtime_id'] = 0
        original['vtable_rva'] = 101
        with self.assertRaisesRegex(ValueError, 'vptr'):
            checked_entry(row, original, 0, 0x2d95660)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--private-root', type=Path, default=ROOT.parent / 'c6-ghidra')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    elf = Elf(a.assets / 'libGameCore_XP2.so', GAME_SHA)
    Elf(a.assets / 'Civ6', HOST_SHA)
    methods = []
    for name, address, size, expected in METHODS:
        require(sha(elf.read(address, size)) == expected, 'method fingerprint mismatch: ' + name)
        methods.append(dict(name=name, rva=hex(address), size=size, sha256=expected))
    for offset, target in BINDINGS:
        require(elf.relocated_pointer(0x3aff880 + offset) == target, 'Manager virtual mismatch')
    require(elf.read(0x2d95660, 3).hex() == 'b001c3', 'constant true leaf mismatch')
    require(elf.read(0x2da0920, 3).hex() == '31c0c3', 'constant false leaf mismatch')
    require(elf.read(0x270bbb0, 10).hex() == '813f000800000f92c0c3', 'unsigned index tag mismatch')
    capture = a.private_root / 'evidence/linux-unit-operation-manager-metadata/native-1'
    profile, entries = checked_rows((capture / 'facts.jsonl').read_bytes(),
                                    (capture / 'facts-repeat.jsonl').read_bytes())
    require(sha((capture / 'runtime.json').read_bytes()) == RUNTIME_SHA, 'closed runtime mismatch')
    registry_path = a.private_root / 'evidence/linux-operation-registry/native-1.json'
    raw_registry = registry_path.read_bytes()
    require(sha(raw_registry) == REGISTRY_SHA, 'independent registry report mismatch')
    registry = json.loads(raw_registry)
    original = registry['mapping']['entries']
    require(len(original) == len(entries), 'independent registry dimensions mismatch')
    inventory = []
    for row, old in zip(entries, original):
        target = elf.relocated_pointer(row['handler_vptr'] - profile['module_base'] + 0x20)
        inventory.append(checked_entry(row, old, profile['module_base'], target))
    corpus = []
    by_hash = {r['operation_hash']: r for r in inventory}
    for sample in registry['corpus']:
        counts = Counter()
        for key, count in sample['queue_counts'].items():
            if key == 'empty':
                counts['empty'] += count
            else:
                require(key in by_hash, 'unresolved attributed queue hash')
                family = 'predicted_constant_true' if by_hash[key]['predicted_leaf_result'] else 'predicted_constant_false'
                counts[family] += count
        require(sum(counts.values()) == 1262, 'incomplete attributed queue census')
        corpus.append(dict(save_sha256=sample['save_sha256'], counts=dict(counts),
                           source='saved queues and observed vptrs joined to image-predicted targets across different capture contexts',
                           runtime_dispatch_admitted=False, native_predicate_results_observed=False))
    report = dict(schema='c6-linux-current-handler-interruption-v2', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, original_execution_performed=False,
        native_predicate_comparison_performed=False, runtime_handler_slots_observed=False,
        runtime_dispatch_admitted=False,
        supersedes=dict(schema='c6-linux-current-handler-interruption-v1', commit='3bd25f6',
            report_sha256='d9b7ff5a2c20a92397f473e0b91f80dcd4073996f77d206f8ac9848346c8a367',
            correction='prior target/result fields were source predictions, not observed runtime dispatch'),
        methods=methods,
        manager_static_vptr_rva='0x3aff880', manager_static_bindings=[
            dict(slot_offset=hex(o), target_rva=hex(t)) for o, t in BINDINGS],
        capture_facts_sha256=FACTS_SHA, capture_runtime_sha256=RUNTIME_SHA,
        independent_registry_sha256=REGISTRY_SHA,
        observed_registry_with_source_predictions=inventory,
        predicted_family_counts=dict(Counter(r['predicted_target_rva'] for r in inventory)),
        attributed_queue_corpus=corpus,
        identity_semantics=dict(native_front='first native node u32 bits, not saved operation hash',
            same_operation='compare raw current node bits with captured requested bits before handler lookup',
            get_handler='unsigned bits below 0x800: unchecked vector index; otherwise hash lookup',
            hash_miss='GetHandlerIDFromHash returns -1; GetHandlerFromHash returns null',
            no_general_null='out-of-range numeric access is not a defined null case',
            missing_current_handler='only an actually null GetHandler result permits interruption'),
        source_closure='two image-predicted virtual families have closed leaf implementations; no actual runtime dispatch slot captured',
        native_coverage_gaps=['actual runtime handler vptr+0x20 method pointers and ownership/module validation; dynamic interposition not excluded',
            'live Manager vptr and delegated slots 0x88/0x98/0xa0/0xd8',
            'same-context per-Unit native queue front bits/handler identity and replay comparison',
            'native predicate boundary comparisons; empty/same/different/null/invalid distinctions'],
        protocol='docs/linux-current-handler-interruption.md',
        full_manager_supported=False, complete_state_action_rng_turns_supported=False,
        historical_raw_save_load_failure_bytes=89198,
        source_sha256={
            'tools/linux/current_handler_interruption_audit.py': sha(Path(__file__).read_bytes()),
            'crates/c6-core/src/unit_operation_interruption.rs': sha(
                (ROOT / 'crates/c6-core/src/unit_operation_interruption.rs').read_bytes())})
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, handlers=len(inventory),
                         source_predicted_families=report['predicted_family_counts'], original_execution=False)))


if __name__ == '__main__':
    main()
