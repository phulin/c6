#!/usr/bin/env python3
"""Read-only pinned Linux Operation saved-wire source contract; no game execution."""
import argparse
import json
from pathlib import Path
import re
import struct
import subprocess

from player_wmds_audit import Elf, GAME_SHA, ROOT, relocation_symbols, sha

SCHEMAS = {
    'TeamRequirement': (2, [('unit_tag_hash', 'i'), ('unit_requirement_hash', 'i'),
        ('min_units', 'i'), ('max_units', 'i'), ('min_percentage_bits', 'u'),
        ('max_percentage_bits', 'u'), ('reconsider_until_ready', 'b')]),
    'TeamDefinition': (2, [('min_units', 'i'), ('max_units', 'i'),
        ('strength_advantage_bits', 'u'), ('ongoing_strength_advantage_bits', 'u'),
        ('safe_rally_point', 'b'), ('condition_hash', 'u'),
        ('requirements', '[TeamRequirement]')]),
    'OperationDef': (7, [('op_name', 'String'), ('name_hash', 'i'),
        ('target_type', 'i'), ('target_parameter', 'i'), ('update_team_points', 'b'),
        ('op_type', 'i'), ('behavior_tree_name', 'String'), ('priority', 'i'),
        ('teams', '[TeamDefinition]'), ('max_dist_in_region', 'i'),
        ('max_dist_in_area', 'i'), ('max_dist_in_world', 'i'),
        ('max_target_strength', 'i'), ('max_target_defense', 'i'),
        ('min_odds_of_success_bits', 'u'), ('self_start', 'b'),
        ('must_be_at_war', 'b'), ('must_have_nukes', 'b'), ('must_have_min_units', 'i'),
        ('enemy_type', 'String'), ('target_lua_script', 'String'),
        ('active_emergency', 'String')]),
    'OperationTarget': (1, [('enemy', 'i'), ('target_type', 'i'),
        ('location', 'Coord'), ('rally', 'Coord'), ('target_id', 'ComponentId')]),
    'OperationTeam': (3, [('is_at_minimums', 'b'), ('is_ready', 'b'),
        ('is_full_strength', 'b'), ('needed_unit_hash', 'u'),
        ('team_members', '[u]'), ('new_team_members', '[u]'),
        ('member_storage', '[[u]]'), ('member_storage_location', '[Coord]'),
        ('endangered_members', '[u]'), ('has_enemies_near', 'b'),
        ('op_positions', 'Fixed6'), ('local_enemy_strength', 'i'),
        ('local_team_strength', 'i')]),
    'Operation': (6, [('player_id', 'i'), ('op_definition', 'OperationDef'),
        ('teams', '[OperationTeam]'), ('target', 'OperationTarget'),
        ('rally_point', 'Coord'), ('blocked_turns', 'i'), ('tree_control_id', 'u'),
        ('tree_manager_id', 'u'), ('backup_target', 'OperationTarget'),
        ('force_target_switch', 'b'), ('op_strength', 'i'), ('target_strength', 'i'),
        ('current_combat_strength', 'i'), ('force_ready', 'b'), ('war_ready', 'b'),
        ('is_emergency', 'b'), ('control_id', 'u')]),
}
DWARF_TYPES = ['GameCore::AI::Operation::' + name for name in SCHEMAS]


def contract(assets):
    path = assets / 'libGameCore_XP2.so'
    elf = Elf(path, GAME_SHA)
    # All six record pairs, narrow string pair, array pair, enum wrappers,
    # vectors instantiated for these records, and float scalar wrappers.
    listing = subprocess.check_output(['nm', '-S', '-C', str(path)], text=True)
    selected = []
    for line in listing.splitlines():
        match = re.fullmatch(r'([0-9a-f]+) ([0-9a-f]+) \w (.*)', line)
        if not match:
            continue
        address, size, name = match.groups()
        record = ('Serialization::operator' in name and
                  any('GameCore::AI::Operation::' + n in name for n in SCHEMAS))
        helper = ('Serialization::' in name and any(s in name for s in (
            'WriteArray<GameCore::PlotCoord>', 'ReadArray<GameCore::PlotCoord>',
            'EnumToStream<GameCore::AI::AiTargetType>',
            'EnumFromStream<GameCore::AI::AiTargetType>',
            'EnumToStream<GameCore::AI::Control::ControlPriority>',
            'EnumFromStream<GameCore::AI::Control::ControlPriority>')))
        string = name.startswith(('Serialization::OStream& Serialization::operator<< <Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>',
                                  'Serialization::IStream& Serialization::operator>><Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>'))
        float_scalar = name in ('Serialization::OStream::WriteObject(float const&)',
                                'Serialization::IStream::ReadObject(float&)',
                                'Serialization::OStream& Serialization::OStream::operator<< <float>(float const&)',
                                'Serialization::IStream& Serialization::IStream::operator>><float>(float&)')
        if record or helper or string or float_scalar:
            address, size = int(address, 16), int(size, 16)
            selected.append([address, size, name, sha(elf.read(address, size))])
    selected.sort()
    # Resolve direct PLT calls/jumps using their actual target relocation.
    symbols = relocation_symbols(elf)
    bindings = []
    for address, size, _, _ in selected:
        asm = subprocess.check_output(['objdump', '-d', '--start-address='+hex(address),
                                      '--stop-address='+hex(address+size), str(path)], text=True)
        for line in asm.splitlines():
            match = re.search(r'^\s*([0-9a-f]+):\s+(?:[0-9a-f]{2} )+\s*(call|jmp)\s+([0-9a-f]+)', line)
            if not match:
                continue
            site, kind, destination = match.groups()
            site, destination = int(site, 16), int(destination, 16)
            stub = elf.read(destination, 6)
            resolved = destination
            if stub[:2] == b'\xff\x25':
                got = destination + 6 + struct.unpack_from('<i', stub, 2)[0]
                resolved = symbols.get(got)
                if resolved is None:
                    raise ValueError('unresolved PLT binding')
            bindings.append([site, destination, resolved, kind])
    command = ['gdb', '-nx', '-batch', str(path)]
    for name in DWARF_TYPES:
        command += ['-ex', "ptype /o '"+name+"'"]
    dwarf = subprocess.check_output(command, text=True)
    return dict(module_sha256=GAME_SHA, methods=selected, bindings=bindings,
                dwarf_types=DWARF_TYPES, dwarf_sha256=sha(dwarf.encode()),
                schemas=SCHEMAS, custom_grammar={
                    'String': 'u32 byte count; exactly count raw bytes; no terminator',
                    'Fixed6': 'u32 count exactly 6; six Coord pairs of i32',
                    'Coord': 'i32 x; i32 y',
                    'ComponentId': 'i16 owner; i16 component_type; u32 id',
                    '[T]': 'u32 element count; count T values'},
                limits='Shared byte/item budgets; strings charge bytes as items; Fixed6 charges six items',
                native_execution=False,
                scope='Current-version bounded saved wire, not gameplay, AI policy, loading or turns')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    result = json.dumps(contract(args.assets), indent=2) + '\n'
    if args.output:
        args.output.write_text(result)
    else:
        expected = ROOT/'evidence/linux/military-operations-contract.json'
        if expected.read_text() != result:
            raise ValueError('source contract differs from checked-in evidence')
        print('Pinned Linux Operation source contract matches')


if __name__ == '__main__':
    main()
