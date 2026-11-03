#!/usr/bin/env python3
"""SHA-gated read-only Linux AI BoostManager static wire contract audit.

Does not execute original code, save native artifacts, or claim initialized state parity.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA

METHODS = [
    (35176416, 15, 'FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::Research>::load(Serialization::IStream&)', 'c47831e8dcf960191917a312dce43da1a02cc7e8b2fe024ad8a14a1251d8e56e'),
    (33828832, 15, 'FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::CultureAi>::load(Serialization::IStream&)', 'b3719a6d30c2f9c6ffcd5315b032a5c8aec94a66d868b2ea540c2d9cbda90119'),
    (33830880, 77, 'FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::CultureAi>::reset()', '7910f8a510df15ab12dac6c65a3949bbc8168560f56f8742edda54b5fd708254'),
    (33348304, 75, 'Serialization::IStream::ReadObject(int&)', '09ddfd4d815a2204789075821ff06a3e8d881dc2bae4791ff3f746206deb3230'),
    (33348528, 75, 'Serialization::IStream::ReadObject(unsigned int&)', 'b5c97ed9421b7f58e0ea8a7e2989ec1c05dd9c16a79416af11d59bd45e57dc3e'),
    (33341856, 14, 'Serialization::IStream& Serialization::IStream::operator>><int>(int&)', 'c6cf9d3175d5134bd8d7b3d396184fd7b9abd3f7ca364cbdc38a1794ce742775'),
    (33341888, 14, 'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)', '5edd23b31cc38a2cc74b61c4b33889da2f35c69f9ce10ee24f2899eb7e91af01'),
    (33348288, 14, 'Serialization::OStream::WriteObject(int const&)', '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'),
    (33348512, 14, 'Serialization::OStream::WriteObject(unsigned int const&)', '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'),
    (33341744, 14, 'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)', '2a3fc110a86f71122b45f03f192ddbc4499241d298e49b843ea750bd49d5e1cf'),
    (33341776, 14, 'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)', '9731ca1d2959dc0f406a684267a29d425c9bf7ea2f127425123508fbc2ed2d3b'),
    (46771104, 33, 'Serialization::operator<<(Serialization::OStream&, GameCore::PlayerTypes const&)', 'e3aa2b85edbfcad11d2fe4b8d720bad7d662e88d8461f7994ba25444ae60c6d1'),
    (33330560, 173, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::BoostManager const&)', 'eef31e007cb7ce39ca344ffe3f306121410929be3026af00301b35bae29c714f'),
    (33721440, 444, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::CultureAi const&)', '45c6a6d52c4e9a042b4e0c188e61c95d4169fcad784c3a7cc1bf4244598da3b1'),
    (46771152, 38, 'Serialization::operator>>(Serialization::IStream&, GameCore::PlayerTypes&)', '2cc19dccb1ce5ba3727486d694c79a0b5c90c1d7658cf975ce07ade2572a6ee7'),
    (33330736, 197, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::BoostManager&)', '1126f3ef4c21220cdc8443db4ebb7f9fb9dcc1a760c594ace32a7091073146fd'),
    (33721888, 1227, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::CultureAi&)', '6b4f8de366f68cd51529bb1df13246923c96379bf3949f24a823a6136bf49264'),
    (33330944, 203, 'GameCore::AI::BoostManager::Initialize(GameCore::Player::Instance*, unsigned int)', '4ae7683cb9f2f81047459f809fabb0ddbe7c6a323042ab84f039bc399ff1e903'),
    (33331360, 62, 'GameCore::AI::BoostManager::BoostManager()', '0d9352eff089ad4085fe634a7d5dd8a3a6d1bcc1d8f3c8a12d6e2b14117fc9f4'),
    (35143232, 473, 'GameCore::AI::Research::Read(Serialization::IStream&)', '7920af77ab2594085727dce5587ae51dc393a93d9ab0b8e2921a89147a944cfb'),
    (35176528, 15, 'FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::Research>::save(Serialization::OStream&) const', '6262d46a1130a613bff9954667328a0289c51bd5bcb5c6f95ef0c87393fdbdd8'),
    (33828944, 15, 'FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::CultureAi>::save(Serialization::OStream&) const', '1661fe110811fda80cb9e5ee5de9f1af1ed322e0f7b2cd9dcfcc5b7882e4bde6'),
    (35142944, 241, 'GameCore::AI::Research::Write(Serialization::OStream&) const', 'ded8c844d3235828ac71e40b77e8cf6b086d069a9953a5e6031fa3a217fb38dd'),
    (35169680, 21, 'Serialization::OStream& operator<< <GameCore::AI::BoostManager, GameCore::AI::Research>(Serialization::OStream&, FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::Research> const&)', '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
    (33773536, 21, 'Serialization::OStream& operator<< <GameCore::AI::BoostManager, GameCore::AI::CultureAi>(Serialization::OStream&, FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::CultureAi> const&)', '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
    (35169376, 21, 'Serialization::IStream& operator>><GameCore::AI::BoostManager, GameCore::AI::Research>(Serialization::IStream&, FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::Research>&)', '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
    (33774576, 21, 'Serialization::IStream& operator>><GameCore::AI::BoostManager, GameCore::AI::CultureAi>(Serialization::IStream&, FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::CultureAi>&)', '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
]
DWARF_SHA = 'c0abc499a27bdfcdca02bcd44052bd57c02e45554a598e9a6fe7dca26599471e'

# Caller -> PLT -> exact body; symbols alone are not cross-owner proof.
BINDINGS = [
    (0x2028cbf, 0x1f9d220, 0x20357e0),
    (0x2028ea4, 0x1e24ee0, 0x2035bf0),
    (0x1fc959c, 0x1f5fbd0, 0x1fcc130),
    (0x1fc95b5, 0x1e8d360, 0x2c9aba0),
    (0x1fc95c1, 0x1e20270, 0x1fcc150),
    (0x1fc95de, 0x1e20270, 0x1fcc150),
    (0x1fc9606, 0x1e20270, 0x1fcc150),
    (0x1fc9612, 0x1f5fbd0, 0x1fcc130),
    (0x1fc9645, 0x1e757d0, 0x1fcc1a0),
    (0x1fc9652, 0x1faa940, 0x2c9abd0),
    (0x1fc9672, 0x1e57840, 0x1fcc1c0),
    (0x1fc967f, 0x1e57840, 0x1fcc1c0),
    (0x1fc96b6, 0x1e57840, 0x1fcc1c0),
    (0x1fc96ca, 0x1e757d0, 0x1fcc1a0),
    (0x1fc96e2, 0x1ede0a0, 0x1fc9700),
    (0x2183df4, 0x1f9ed30, 0x218a590),
    (0x2183fcc, 0x1ee6cf0, 0x218a460),
]
TAIL_BINDINGS = [
    (0x2042fea, 0x1f1bfa0, 0x1fc9630),
    (0x204305a, 0x1e1e4e0, 0x1fc9580),
    (0x218bfea, 0x1f1bfa0, 0x1fc9630),
    (0x218c05a, 0x1e1e4e0, 0x1fc9580),
]
VIRTUALS = [
    (0x3a081b8 + 16 + 0x10, 0x2042fe0),
    (0x3a081b8 + 16 + 0x28, 0x2043050),
    (0x3a201a8 + 16 + 0x10, 0x218bfe0),
    (0x3a201a8 + 16 + 0x28, 0x218c050),
]
FIELDS = [
    dict(name='version', wire='i32-le', current=1, reader='consumed without validation or branching'),
    dict(name='player', wire='PlayerTypes/i32-le', reader='resolve through Player::Manager::GetPlayer'),
    dict(name='m_allowedActiveBoosts', wire='u32-le', object_offset='0x38'),
    dict(name='managed_count', wire='u32-le', role='count, not maximum active boosts'),
    dict(name='m_managedBoosts', wire='managed_count ordered records', record=[
        dict(name='BehaviorTreeControlId', wire='u32-le', identity='GameCore::IDType', object_offset=0),
        dict(name='BoostUniquenessHash', wire='i32-le', identity='raw signed hash', object_offset=4)]),
]


def relocated_pointer(game, address):
    """Resolve ELF64 R_X86_64_64, RELATIVE and JUMP_SLOT without loading ELF."""
    raw = game.raw
    start = struct.unpack_from('<Q', raw, 40)[0]
    width, count = struct.unpack_from('<HH', raw, 58)
    sections = [struct.unpack_from('<IIQQQQIIQQ', raw, start + i * width) for i in range(count)]
    for section in sections:
        if section[1] != 4:
            continue
        symbols = sections[section[6]]
        for pos in range(section[4], section[4] + section[5], section[9]):
            target, info, addend = struct.unpack_from('<QQq', raw, pos)
            if target != address:
                continue
            kind = info & 0xffffffff
            if kind == 8:
                return addend
            if kind in (1, 7):
                symbol = symbols[4] + (info >> 32) * symbols[9]
                return struct.unpack_from('<Q', raw, symbol + 8)[0] + addend
            raise ValueError(f'unsupported relocation type: {kind}')
    raise ValueError(f'missing relocation at {address:#x}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    path = args.assets / 'libGameCore_XP2.so'
    game = Elf(path, GAME_SHA)
    for address, size, name, digest in METHODS:
        if hashlib.sha256(game.read(address, size)).hexdigest() != digest:
            raise ValueError(f'method hash mismatch: {name}')
    for site, plt, body in BINDINGS + TAIL_BINDINGS:
        raw = game.read(site, 5)
        opcode = 0xe9 if (site, plt, body) in TAIL_BINDINGS else 0xe8
        if raw[0] != opcode or site + 5 + struct.unpack_from('<i', raw, 1)[0] != plt:
            raise ValueError(f'edge mismatch at {site:#x}')
        stub = game.read(plt, 6)
        if stub[:2] != bytes.fromhex('ff25'):
            raise ValueError('unexpected PLT encoding')
        slot = plt + 6 + struct.unpack_from('<i', stub, 2)[0]
        if relocated_pointer(game, slot) != body:
            raise ValueError(f'PLT binding mismatch: {plt:#x}')
    for slot, body in VIRTUALS:
        if relocated_pointer(game, slot) != body:
            raise ValueError(f'virtual binding mismatch: {slot:#x}')
    if game.read(0x1fc958f, 8) != bytes.fromhex('c744240c01000000'):
        raise ValueError('BoostManager current signed version mismatch')
    if game.read(0x2028c6f, 8) != bytes.fromhex('c744240c0f000000'):
        raise ValueError('CultureAi current version mismatch')
    command = ['gdb', '-nx', '-batch', str(path)]
    for name in ['GameCore::AI::BoostManager', 'GameCore::AI::BoostManager::BoostManagement', 'GameCore::IDType']:
        command += ['-ex', "ptype /o '" + name + "'"]
    dwarf_sha = hashlib.sha256(subprocess.check_output(command)).hexdigest()
    if dwarf_sha != DWARF_SHA:
        raise ValueError('matching target DWARF layout mismatch')
    report = dict(
        schema='c6-linux-ai-boost-contract-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, method='read-only static ELF method/call/relocation/version/DWARF checks',
        methods=[dict(address=hex(a), size=n, name=name, sha256=h) for a,n,name,h in METHODS],
        bindings=[dict(site=hex(a), plt=hex(b), body=hex(c)) for a,b,c in BINDINGS],
        tail_bindings=[dict(site=hex(a), plt=hex(b), body=hex(c)) for a,b,c in TAIL_BINDINGS],
        virtual_bindings=[dict(slot=hex(a), body=hex(b)) for a,b in VIRTUALS], dwarf_sha256=dwarf_sha,
        culture_current_version=15, boost_current_version=1, wire_bytes='16 + 8 * managed_count', fields=FIELDS,
        no_q8_fields=True, no_civic_or_tech_indices=True, no_definition_hash_remapping=True,
        research_same_nested_binding_proven=True,
        native_read_appends_managed_records=True, native_read_resets_manager=False,
        reset_is_separate_method='0x20437e0',
        native_read_effects=['Resolve and overwrite player pointer',
            'Overwrite allowed-active count', 'Reserve and append managed records in source order',
            'Initialize(player, allowed): reserve managed capacity; when personality exists, query leader boost support and set up support functions'],
        not_serialized=['m_supportFunctions', 'native player pointer', 'vector allocation/capacity'],
        fixtures=[], fixture_gap='No BoostManager owner cuts or native fixture execution were used in this research-only audit',
        original_execution=False, native_read_effects_reproduced=False, private_writes=False,
        full_player=False, world_import=False, gameplay_state=False, actions=False, turns=False,
        planning_only_exclusion_proven=False, unknown_save_bytes_masked=False,
        recorded_raw_save_load_difference_bytes=89198,
        audit_source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        attribution='Independent analysis of pinned supplied Linux ELF symbols, bytes and matching DWARF; no external reference material reproduced')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS), bindings=len(BINDINGS)+len(TAIL_BINDINGS), dwarf_sha256=dwarf_sha)))


if __name__ == '__main__':
    main()
