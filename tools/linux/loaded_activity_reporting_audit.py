#!/usr/bin/env python3
"""Check exact Linux activity reporting source contracts; never run original code."""
import argparse
import hashlib
import json
from pathlib import Path

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
PARENTS = {
    'unit-event-effects.json': 'a4d2bf10ab89ec32ec74b8242e9c32565f5185e6cb4325ffbe13e558b51ffa03',
    'unit-event-consumers.json': 'f2f41595a031845ac42bac956e229ec5a8fd3c90537a330dea9a0681ee67429c',
    'reporting-gameplay-scope.json': '31d08f421df213b8c5299925fe98f9dca71fe337b1c85232dc2991a6bf8fde26',
    'reporting-provider-static.json': 'd43f80990999745fb06de53b9225120d0375f86a50bd9f701bb778d89da20405',
    'skip-turn-reporting-closure.json': 'f2e5d83cdb4294dc8096e7b6a96aa4870ae2d5bbd65dd54876b33439a1dc2772',
}
ADDITIONAL = [
    ('Plot.IsWater', 0x2bc97d0, 80, 'c935f8d2ca7eddae13a2d74b29312b175126f21390531088f67149d29098383b'),
    ('Plot.GetOwner', 0x1fe23d0, 41, 'd40d1bd008ab26ed6eac1cc16bfd19ecf84925b3ce6bf6ef79d1d97e1520ce05'),
    ('Unit.GetMovesRemaining', 0x213cc90, 21, 'cacb37411e9aab85d6dd05c1d07fb2f2a36acd2877aeee83ac8bd09f08e92c13'),
    ('Map.GetAdjacentPlotLocation', 0x2426a30, 193, 'd010168af2f392f555b334004652a99f16bcdde8611afe908b2e30d007c8536d'),
]
ADDITIONAL_HOST = [
    ('EventData1.Constructor', 0x32b54f0, 132, 'd867586f001aeb91f98643bceb8446c309cc41d3a465ab01d850587caf153614'),
    ('Activity.Constructor', 0x321e990, 29, '676762dbd4199aec595ab39b09e25d919d5e8e7fb5dc44219b28638ff0877749'),
    ('Operation.Constructor', 0x321eb30, 5, '461e657c282d0d803ff2eddff0062baeece27ed4a0bb6e19f42e468a987111e8'),
    ('Publish35', 0x3217350, 80, '94114fc5515f4f73ed389bdeb7c2fc0b39c199c0b9c9629992b78da2a6c9e742'),
    ('Publish67', 0x3217770, 80, '47fd4cc9931bb1f447eeb562a8af7d734da658849130455d1a7b0504fe38051a'),
    ('Publish68', 0x32177d0, 80, 'fae2df0561528438bd9bb826ee74c0aca49bca0410144fd26b9fa12be9690541'),
    ('Publish70', 0x3217650, 80, '3fc3ea42f2feff68f741f579750376794d6319fa81b205913cc21eb66d67af61'),
]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    images = {'game': Elf(a.assets / 'libGameCore_XP2.so', GAME_SHA),
              'host': Elf(a.assets / 'Civ6', HOST_SHA)}
    methods = {}
    for name, expected in PARENTS.items():
        raw = (ROOT / 'evidence/linux' / name).read_bytes()
        if digest(raw) != expected:
            raise ValueError('closed parent changed: ' + name)
        report = json.loads(raw)
        for method in report.get('methods', []):
            if 'image' not in method:
                # These two parent inventories explicitly contain host bodies.
                if name not in ('reporting-gameplay-scope.json', 'skip-turn-reporting-closure.json'):
                    raise ValueError('missing original image: ' + name)
                method = dict(method, image='host', rva=method['address'])
            key = (method['image'], method['rva'], method['bytes'])
            if digest(images[key[0]].read(int(key[1], 16), key[2])) != method['sha256']:
                raise ValueError('original method changed: ' + method['name'])
            methods[key] = method
    for name, address, size, expected in ADDITIONAL:
        if digest(images['game'].read(address, size)) != expected:
            raise ValueError('original leaf changed: ' + name)
        methods[('game', hex(address), size)] = dict(
            name=name, image='game', rva=hex(address), bytes=size, sha256=expected)
    for name, address, size, expected in ADDITIONAL_HOST:
        if digest(images['host'].read(address, size)) != expected:
            raise ValueError('original host body changed: ' + name)
        methods[('host', hex(address), size)] = dict(
            name=name, image='host', rva=hex(address), bytes=size, sha256=expected)
    source = ROOT / 'crates/c6-data/src/loaded_actions/reporting.rs'
    result = dict(
        schema='c6-linux-loaded-ordered-reporting-source-v2', checks_passed=True,
        original_execution=False, gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        parents=PARENTS, methods=list(methods.values()),
        source_sha256=digest(source.read_bytes()), audit_sha256=digest(Path(__file__).read_bytes()),
        first_event='e23b112d', synchronous_receiver='27ff430 -> 27ff860 -> 27ff2d0 -> 2805100',
        original_query_order='snapshot readiness before host event35 translation; six native-order neighbors; placement before HasMoved before conditional Plot movement cost',
        excluded_output='concrete reflected Unit snapshot stream and stock UnitFlag dirty vectors only',
        actual_null_prepublish='CacheUpdated275 seven; activity35 three; other actual captured callbacks remain as parent evidence',
        next_effectful_readiness_branch='HasAdjacentMove=false -> command HoldCycling -> if false operation HoldCycling',
        captured_adjacent_result_used_as_computation=False,
        unit_translators=[35, 67, 68, 70, 71],
        scalar_field_projection={'67/68': ['subtype', 'first'], '70/71': ['first', 'second']},
        absent_EventData1_scalar_default=0, activity_default_operation=0,
        activity_default_parameters=[-1, -1, -1, -1],
        original_unit_scalar_rows=9, full_entrypoint_comparison='executed prefix only',
        host_publication_fields_captured_by_original_scalar_rows=False,
        queued_event35_consumers_available=False, full_action_acceptance=False,
        complete_state_actions_turns=False, complete_rng=False,
        historical_raw_save_load_failure_bytes=89198,
    )
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, method_bodies=len(methods), original_execution=False)))


if __name__ == '__main__':
    main()
