#!/usr/bin/env python3
"""Read-only SHA-gated Linux Military initialization dependency audit.

No game execution, save framing inference, native fixture creation or private writes.
"""
import argparse
import json
import struct
import subprocess
from pathlib import Path
from player_wmds_audit import Elf, GAME_SHA, ROOT, relocation_symbols, sha

METHODS = [[34892032,
  3133,
  'GameCore::AI::Military::Military()',
  'b2c3cb501163f81e4ccb78f86cd7d85208d575a3c39b48292825fbc5f568541d'],
 [34922944,
  69,
  'FAutoVariable<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::Military>::FAutoVariable(AutoVariableString const&, FAutoArchive&)',
  '45a62456fdba540af58f66fcf0be263d62cf5dda44a43cbda95f2eabd31cc19a'],
 [33436016,
  44,
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::Vector()',
  '6552304b4ed006c5c6ae768660b666eb4ef890d0270920d5a7745e8d29a04397'],
 [34895616,
  377,
  'GameCore::AI::Military::Initialize(GameCore::Player::Instance*)',
  'ae064d33ad7964b7b8e5f3d7a3383de5c0beeda6a0b3bd8c466a932a8e3444ff'],
 [34896000,
  460,
  'GameCore::AI::Military::Uninitialize()',
  '8d41933a9145d7214b754dd6b64c63efc3bb2e229a4ba4bad57f96513004a323'],
 [34888672,
  1919,
  'GameCore::AI::Military::InitializeLists(GameCore::Player::Instance const*)',
  'f253867d3401330c91e2cd70cc95edce23fa76eb46f7df21d7c254e4f2133740'],
 [34582736,
  26,
  'GameCore::AI::GlobalAi::HasAiPersonality(GameCore::Player::Instance const*)',
  'f553a85a417ec3db5834de8d0ddcc9479d84f0c225a3cd7134c770eae71f7ac3'],
 [34582768,
  159,
  'GameCore::AI::GlobalAi::GetLeaderType(GameCore::Player::Instance const*)',
  'a66882df0f246ef6f86bfac51b013be0d19596a02007f56814d66b63d4d9bd41'],
 [33795120,
  678,
  'GameCore::AI::Lists::Handler<int>::ExecuteLeaderQuery(Types::Vector<int, '
  'Types::LinearAlloc::Default<1u, 0u> >&, char const*, GameCore::CivilizationTypes, '
  'GameCore::HandicapTypes)',
  'e70a7b53e346ff2f63b63645895109a1a8823c3333cfec7c5320dc20b0f7117d'],
 [34617344,
  417,
  'void '
  'GameCore::AI::Lists::Handler<int>::Initialize<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  '>(char const*, char const*, int const&&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  'const&, std::__1::function<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >& '
  '(GameCore::Player::Instance&)>&&)',
  '23df4f193224ac36cf15b25f23f6909549fbd801dca6c81afc9d50f0e4075d5a'],
 [34685456,
  8,
  'GameCore::AI::Lists::Handler<int>::Initialize<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  '>(char const*, char const*, int const&&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  'const&, std::__1::function<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >& '
  '(GameCore::Player::Instance&)>&&)::{lambda()#1}::operator()() const',
  '6f7a64b49a5f8a5069e3798ebf6c03d2b8b90bd81c6299c9a375a868a8b6c57d'],
 [34684720,
  1,
  'GameCore::AI::Lists::Handler<int>::Initialize<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  '>(char const*, char const*, int const&&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  'const&, std::__1::function<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >& '
  '(GameCore::Player::Instance&)>&&)::{lambda(Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> '
  '>&)#1}::operator()(Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >&) const',
  'ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e'],
 [34685472,
  5,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType>::GetCount() '
  'const',
  '1aa54d5b608a877f05488ef65cd55d8965b7ddbbeaf91d8d3d3dc4419ed181f2'],
 [34685488,
  12,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType>::size() '
  'const',
  '8375e78d364001210bb02186b4a5388e8e09554d7905f614c75f29f2692f7228'],
 [34606400,
  71,
  'GameCore::AI::Lists::Handler<int>::Handler()',
  'cea551a7eb3552a140a557480d1cba6d30196b78572711a4af158c1195f46e3c'],
 [34564880,
  4962,
  'GameCore::AI::GlobalAi::Initialize()',
  '0a39c4daecc1737c028dc8a437b82ef16987d45d35930d6e68e9b6d4d3717204'],
 [35400576,
  164,
  'GameCore::AI::Lists::Factory::MakeLeaderQuery(char const*, char const*, char const*)',
  '89b73bfa2ec4e0fe3f71a4fb7eb34666fb7825660bad3f093750f199417281d0'],
 [34928160,
  8,
  'GameCore::AI::GlobalAi::GetOperationsHandler()',
  '165ad9b3b0d37f7e9684265ec4d8fecef7f1b7e723848b3cf09e86823fc025ca'],
 [34928176,
  8,
  'GameCore::AI::GlobalAi::GetPerWarOperationsHandler()',
  '81d47638cb2583f2ccbcd5b8f2310b2cd79bd38626193d08fb90bd3de623ccaf'],
 [34685888,
  12,
  'GameCore::AI::Military::GetAllowedOperationTypes()',
  'b7f404a52d838d3926d32e39f458e6e5b4384f68126c8fd5b8616068a77da0a2'],
 [33446608,
  4,
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::size() const',
  '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'],
 [34619376,
  7,
  'GameCore::Player::Instance::WasEverAlive() const',
  '2985741e4b49f0a7c866f3f07b21af359bda5e5061a503294f85e3244bcfaa79'],
 [33342976,
  7,
  'GameCore::Player::Instance::GetHandicap() const',
  'b112eda9f6609f4b2dc3be0eb54d0f72a54b267d42af0473d4272bf53259eab6'],
 [45286592,
  32,
  'GameCore::Player::Instance::GetCivilizationType() const',
  'af5c4ff6e0297c5f1f9cecfd7f0dc12392462e947e0b3a79c638205176582e71'],
 [45290896,
  16,
  'GameCore::Player::Instance::IsBarbarian() const',
  'aa0d06d424eb98919ddc1c35d7c8b15e72694839dccf353c10482f2148f5255e'],
 [45290912,
  15,
  'GameCore::Player::Instance::GetLevel() const',
  '871aaec65291fcbe8e249418909eb5f3b31b4a529069d22a0a4436e216558971'],
 [34921456,
  149,
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  'const&)',
  '0e56fed5a4cdac565065c061f0feab380854c1411305c9c4fd46b39f674968f9'],
 [34920368,
  141,
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  'const&)',
  '77ed7b582cd32ad928a00d514d90cd959cc94e3e96775f5183cd85b3f0f87376'],
 [34921616,
  120,
  'bool Serialization::ReadIndexHashArrayVersioned<int, int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType> '
  'const&, int, int&&, int&&)',
  'd495d5728083c2f7f7f45810ab79b1a87e3f3df923beb6069f9340ebb7d24191']]
DWARF_TYPES = ['GameCore::AI::Military', 'GameCore::AI::Lists::Handler<int>', 'Types::Vector<int, Types::LinearAlloc::Default<1, 0> >']
DWARF_SHA = 'a896974fac24c52d5134f5f55b1121479527d7911c7923e3e7b655c1888044c3'

# Direct calls/tails through the pinned ELF's PLT: site, PLT, actual target body.
CALLS = [
    (0x2145c02, 0x1f7cfc0, 0x20fb0d0),
    (0x2145c12, 0x1f82820, 0x20fb0f0),
    (0x2145c96, 0x1eb4050, 0x203ac30),
    (0x2145dc5, 0x1ed3bd0, 0x22135e0),
    (0x2145dcd, 0x1f5a900, 0x2114220),
    (0x2145664, 0x1f714b0, 0x2145be0),
    (0x214771d, 0x1f714b0, 0x2145be0),
    (0x2114213, 0x1f5a900, 0x2114220),
    (0x2114220, 0x1fb5660, 0x2114230),
    (0x20f7a3c, 0x1ef4000, 0x2103800),
    (0x214d7c9, 0x1f5a900, 0x2114220),
    (0x2145521, 0x1f3b990, 0x214d7b0),
    (0x2145820, 0x1e5faf0, 0x214dbf0),
]
# Exact branch/assignment sites supplement complete-method fingerprints.
SITES = [
    (0x2145c09, '0f84bc030000', 'no-personality skips list changes'),
    (0x203ac4b, '4885ff0f8473020000', 'null leader query returns before clear/resize'),
    (0x2113f30, 'c3', 'standard list initialization callback does not change size'),
    (0x2145504, '85c07462', 'writer tests allowed-vector size'),
    (0x21457cd, '85c00f84c2000000', 'reader tests allowed-vector size'),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    module = args.assets / 'libGameCore_XP2.so'
    elf = Elf(module, GAME_SHA)
    for address, size, name, digest in METHODS:
        if sha(elf.read(address, size)) != digest:
            raise ValueError('method mismatch: ' + name)
    relocations = relocation_symbols(elf)
    bindings = []
    for site, plt, body in CALLS:
        raw = elf.read(site, 5)
        if raw[0] not in (0xe8, 0xe9):
            raise ValueError(f'not direct call/tail: {site:#x}')
        if site + 5 + struct.unpack_from('<i', raw, 1)[0] != plt:
            raise ValueError(f'call target changed: {site:#x}')
        raw = elf.read(plt, 6)
        if raw[:2] != b'\xff\x25':
            raise ValueError('not expected PLT shape')
        slot = plt + 6 + struct.unpack_from('<i', raw, 2)[0]
        if relocations[slot] != body:
            raise ValueError(f'PLT binding changed: {site:#x}')
        bindings.append(dict(site=hex(site), plt=hex(plt), got=hex(slot), body=hex(body)))
    for site, raw_hex, description in SITES:
        if elf.read(site, len(bytes.fromhex(raw_hex))).hex() != raw_hex:
            raise ValueError('site mismatch: ' + description)
    command = ['gdb', '-nx', '-batch', str(module)]
    for name in DWARF_TYPES:
        command += ['-ex', "ptype /o '" + name + "'"]
    if sha(subprocess.check_output(command)) != DWARF_SHA:
        raise ValueError('matching target DWARF mismatch')
    report = dict(
        schema='c6-linux-ai-military-initialization-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA,
        audit_source_sha256=sha(Path(__file__).read_bytes()),
        methods=[dict(address=hex(a), size=n, name=name, sha256=d)
                 for a, n, name, d in METHODS],
        direct_bindings=bindings,
        sites=[dict(address=hex(a), bytes=h, role=r) for a, h, r in SITES],
        dwarf_types=DWARF_TYPES, dwarf_sha256=DWARF_SHA,
        size_contract=dict(
            personality='IsBarbarian(player) || WasEverAlive(player)',
            initialized_allowed_count=(
                'personality && (resolved_leader_index == -1 || '
                'standard_handler.leader_query != NULL) ? active_definition_count '
                ': prior_allowed_count'),
            presence='initialized_allowed_count != 0',
            definition_count='active AiOperationType collection pointer span / 8',
            no_personality='preserves all three prior vectors',
            current_count='personality ? initialized_allowed_count : prior_current_count',
            successful_query='clear; resize(active_definition_count, default=0); modify rows in place',
            row_filtering_changes_size=False,
            generic_count_equal_to_active_definitions=False,
            normal_completion_required=True,
            standard_handler_initialization_required=True),
        native_snapshot_abi=dict(
            military_player_pointer='Military + 0x98',
            allowed_value_vector='Military + 0x158',
            per_war_value_vector='Military + 0x180',
            current_value_vector='Military + 0x1a8',
            vector_data_pointer_offset='0x0', vector_count_offset='0x10',
            vector_count_type='u32', vector_element_type='i32',
            operations_handler='GlobalAi + 0x25d0',
            per_war_operations_handler='GlobalAi + 0x2720',
            handler_leader_query_pointer_offset='0x0',
            handler_civilization_query_pointer_offset='0x8',
            player_was_ever_alive_byte='Player + 0x278',
            player_handicap_i32='Player + 0x2d8',
            read_after_initialize_lists_return='0x2145669',
            unsafe_getter='0x21143c0 GetAllowedOperationTypes tails edit; do not call for readonly capture'),
        minimum_genuine_context=[
            'module/host and fixture/configuration/rules/DLC/mod identities',
            'same Player ID and native object lifetime as this Military record',
            'actual player civilization-level/WasEverAlive and configuration leader/civilization IDs, actual handicap',
            'ordered active AiOperationType definitions and hashes/indices',
            'proven constructor or Uninitialize-cleared prior vectors, or attributed prior vectors',
            'standard GlobalAi.Initialize completion and actual query construction outcome',
            'normal Military.InitializeLists completion under the same immutable inputs',
            'or attributed live post-initialization complete three-vector contents with the above identities and lifecycle boundary'],
        finite_remaining_dependency=(
            'No genuine completed initialization snapshot was captured here. '
            'Acquire the named same-lifetime native inputs plus query construction '
            'outcome and complete three vectors; do not invent preparation or infer it from saved bytes.'),
        original_execution=False, native_snapshot_captured=False,
        private_writes=False, future_capture_profile_changed=False,
        initialization_runtime_implemented=False, full_player=False, world_import=False,
        actions=False, turns=False, unknown_save_bytes_masked=False,
        recorded_raw_save_load_difference_bytes=89198,
        attribution='Independent analysis of pinned supplied Linux ELF bytes, symbols and matching DWARF; no external code reproduced')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS),
                          direct_bindings=len(bindings),
                          original_execution=False, native_snapshot_captured=False)))


if __name__ == '__main__':
    main()
