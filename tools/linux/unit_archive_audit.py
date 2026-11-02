#!/usr/bin/env python3
"""Recover bounded Unit/Formation archive schema from exact original Linux ELF.

Static only. Does not decode an enclosing Core Save or execute gameplay.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

CONSTRUCTORS = [(0x2C1C180, 0x409, 'Unit::IInstance'),
                (0x2C128B0, 0x21D, 'Unit::CInstance'),
                (0x2C24F10, 0x14AE, 'Unit::Instance'),
                (0x293B6F0, 0x6F6, 'Formation::Instance')]
METHODS = [(0x2C22D00, 0x4E6, 'Unit.write'), (0x2C231F0, 0xBC3, 'Unit.read'),
           (0x293B440, 0x114, 'Formation.write'),
           (0x293B560, 0x181, 'Formation.read'),
           (0x293DC90, 0x14, 'Formation.domain.write'),
           (0x2C9ABA0, 0x21, 'PlayerTypes.write'),
           (0x2C9AF20, 0x29, 'PlotCoord.write'),
           (0x2C9AF50, 0x29, 'PlotCoord.read'),
           (0x2C9AFE0, 0x2A, 'FixedPoint8.write'),
           (0x2C9B110, 3, 'FixedPoint8.GetRaw'),
           (0x2A53B90, 0x5F, 'PathNode.write'),
           (0x2A53BF0, 0x5D, 'PathNode.read'),
           (0x2A53D10, 0x2E, 'PathArray.write_count'),
           (0x2A53D40, 0x7D, 'PathArray.read_sequence'),
           (0x2A53DC0, 8, 'PathArray.write_element'),
           (0x2944720, 0x5B, 'Formation.UnitIDs.write'),
           (0x29447D0, 0x2E, 'Formation.UnitIDs.write_count'),
           (0x2944800, 8, 'Formation.UnitIDs.write_element'),
           (0x2944450, 0x7D, 'Formation.UnitIDs.read_sequence'),
           (0x293D8E0, 0x30, 'Formation.ClearPathCache'),
           (0x2DA66B0, 0xC4, 'OperationQueue.write'), (0x2DA6780, 0xF2, 'OperationQueue.read'),
           (0x2D965A0, 0x82, 'BaseHandler.write'), (0x2D96630, 0xEF, 'BaseHandler.read'),
           (0x2DA4010, 0x3C, 'Activity.CopyAssignmentDo'),
           (0x1FE7180, 0x3C, 'Timer.CopyAssignmentDo'),
           (0x1FE5510, 0xE, 'PlotCoord.SetInvalid'),
           (0x2A53C50, 0x5B, 'PathArray.write'),
           (0x2DA2A20, 0x18, 'Manager.SetTimer'),
           (0x2DA3100, 0x34, 'OperationQueue.edit'),
           (0x2D4FDB0, 0x21, 'Activity.write'), (0x2D4FE10, 0x21, 'Automation.write'),
           (0x2D4FED0, 0x21, 'Directive.write'),
           (0x1FCC130, 0xE, 'OStream.int32'), (0x1FCC150, 0xE, 'OStream.uint32'),
           (0x2C2B510, 0x46, 'Hero.default_wrapper_constructor')]
HOST_METHODS = [(0x3C459E8, 0x27, 'Archive.add'), (0x3C45A10, 0x53, 'Archive.remove'),
                (0x3C45A64, 0x36, 'Archive.save'), (0x3C45A9A, 0x77, 'Archive.load'),
                (0x3C49C28, 0x3E, 'VariableBase.constructor'),
                (0x3C49C66, 0x3E, 'VariableBase.constructor_bool'),
                (0x3C45F34, 0xD9, 'Archive.getDeltaIndexes'),
                (0x3C46B38, 0x19B, 'Archive.getVariable'),
                (0x3C46CD4, 0x2F6, 'Archive.getAutoVariableId'),
                (0x2FDD7C0, 0xE, 'OStream.WriteObject.int32'),
                (0x2F66F20, 0xE, 'OStream.WriteObject.uint32'),
                (0x2FE34F0, 0xE, 'OStream.WriteObject.bool'),
                (0x3962650, 0xE, 'OStream.WriteObject.int16')]


def disassemble(path, address, size):
    raw = subprocess.check_output(['objdump', '-d', '-C', f'--start-address={address}',
                                   f'--stop-address={address + size}', str(path)], text=True)
    rows = []
    for line in raw.splitlines():
        match = re.match(r'\s*([0-9a-f]+):\s+(?:[0-9a-f]{2} )+\s*(.*)', line)
        if match:
            rows.append((int(match[1], 16), match[2]))
    return rows


def trace(rows, argument):
    """Small fail-closed offset tracer for these straight-line pinned functions.

    None means unclassified; never infer an object offset from an unknown value.
    Model front() only as an identity for attribution of the later indexed-array
    writer, not as a general C++ container layout or safe dereference operation.
    """
    registers = {argument: 0}
    string_address = None
    for address, instruction in rows:
        if instruction == 'ret':
            return
        match = re.match(r'mov\s+%(r\w+),%(r\w+)$', instruction)
        if match:
            registers[match[2]] = registers.get(match[1])
        match = re.match(r'lea\s+(0x[0-9a-f]+)?\(%(r\w+)\),%(r\w+)$', instruction)
        if match:
            base = registers.get(match[2])
            registers[match[3]] = None if base is None else base + int(match[1] or '0', 16)
        match = re.match(r'add\s+\$(0x[0-9a-f]+),%(r\w+)$', instruction)
        if match:
            base = registers.get(match[2])
            registers[match[2]] = None if base is None else base + int(match[1], 16)
        match = re.search(r'lea\s+[^#]*%rip\),%(r\w+)\s+# ([0-9a-f]+)', instruction)
        if match:
            registers[match[1]] = None
            if match[1] == 'rsi':
                string_address = int(match[2], 16)
        if instruction.startswith('call'):
            yield address, instruction, dict(registers), string_address
            result = registers.get('rdi') if '::front() const@plt' in instruction else None
            for register in ['rax', 'rdi', 'rsi', 'rdx', 'rcx', 'r8', 'r9', 'r10', 'r11']:
                registers[register] = None
            registers['rax'] = result


def fingerprint(elf, methods):
    return [dict(address=hex(a), size=z, name=n,
                 sha256=hashlib.sha256(elf.read(a, z)).hexdigest()) for a, z, n in methods]


def registration_schema(path, elf):
    unit, formation, unregistered = [], [], []
    for address, size, owner in CONSTRUCTORS:
        for call, instruction, registers, name_address in trace(disassemble(path, address, size), 'rdi'):
            if '>::FAutoVariable(' not in instruction:
                continue
            target = instruction.split('<', 1)[1][:-1].removesuffix('@plt')
            cpp_type = target.split('FAutoVariable<', 1)[1].rsplit(', GameCore::', 1)[0]
            offset = registers.get('rdi')
            if offset is None:
                raise ValueError(f'unclassified wrapper receiver at {call:#x}')
            item = dict(owner=owner, wrapper_offset=hex(offset), cpp_value_type=cpp_type,
                        constructor_call=hex(call), constructor=target)
            if target.endswith('::FAutoVariable()'):
                item.update(name='m_kHero', archive_index=None)
                unregistered.append(item)
                continue
            expected_archive = 0 if owner.startswith('Formation') else 8
            if registers.get('rdx') != expected_archive or name_address is None:
                raise ValueError(f'archive/name provenance unresolved at {call:#x}')
            item['name'] = elf.read(name_address, 256).split(b'\0')[0].decode('ascii')
            item['archive_offset'] = hex(expected_archive)
            destination = formation if owner.startswith('Formation') else unit
            item['archive_index'] = len(destination)
            destination.append(item)
    if len(unit) != 73 or len(formation) != 17 or len(unregistered) != 1:
        raise ValueError('unexpected original registration counts')
    return unit, formation, unregistered


def dwarf_fields(path):
    commands = ['gdb', '-q', '-nx', '-batch', str(path)]
    for owner in ['Unit::IInstance', 'Unit::CInstance', 'Unit::Instance', 'Formation::Instance']:
        commands += ['-ex', f"ptype /o 'GameCore::{owner}'"]
    raw = subprocess.check_output(commands, text=True, stderr=subprocess.STDOUT)
    fields, owner, pending = {}, None, None
    for line in raw.splitlines():
        match = re.search(r'type = class GameCore::(Unit::(?:IInstance|CInstance|Instance)|Formation::Instance)', line)
        if match:
            owner = match[1]
        match = re.match(r'/\*\s+(\d+)\s+\|\s+(\d+) \*/\s+class FAutoVariable<', line)
        if match:
            pending = dict(wrapper_offset=hex(int(match[1])), wrapper_bytes=int(match[2]))
        match = re.match(r'/\*\s+(\d+)\s+\|\s+(\d+) \*/\s+ObjectType m_value;', line)
        if match and pending:
            pending.update(value_offset=hex(int(match[1])), value_bytes=int(match[2]))
        match = re.search(r'} (m_\w+);', line)
        if match and pending:
            fields[(owner, match[1])] = pending
            pending = None
    return fields


def write_schema(path, address, size, fields):
    by_offset = {int(row['wrapper_offset'], 16): row for row in fields}
    result = []
    for call, instruction, registers, constant in trace(disassemble(path, address, size), 'rsi'):
        virtual_properties = address == 0x2C22D00 and call == 0x2C231D7
        if 'Serialization::' not in instruction and not virtual_properties:
            continue
        if not result:
            if constant is None:
                raise ValueError('missing record version provenance')
            result.append(dict(ordinal=0, kind='version', address=hex(constant), call=hex(call)))
            continue
        offset = registers.get('rdi' if virtual_properties else 'rsi')
        if offset not in by_offset and offset is not None and offset - 16 in by_offset:
            offset -= 16
        if offset not in by_offset:
            raise ValueError(f'unclassified serializer input at {call:#x}')
        row = by_offset[offset]
        result.append(dict(ordinal=len(result), kind='field', name=row['name'],
                           archive_index=row['archive_index'], wrapper_offset=hex(offset),
                           call=hex(call), serializer=instruction.split('<', 1)[1][:-1]
                           if '<' in instruction else 'TypedVariantMap virtual +0x1f8, flag=1'))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    path = args.assets / 'libGameCore_XP2.so'
    game, host = Elf(path, GAME_SHA), Elf(args.assets / 'Civ6', HOST_SHA)
    unit, formation, unregistered = registration_schema(path, game)
    dwarf = dwarf_fields(path)
    for row in unit + formation + unregistered:
        match = dwarf.get((row['owner'], row['name']))
        if not match or match['wrapper_offset'] != row['wrapper_offset']:
            raise ValueError('constructor/DWARF disagreement: ' + row['name'])
        row.update(match)
    symbols = subprocess.check_output(['nm', '-D', '-S', '-C', str(path)], text=True)
    symbol_rows = {}
    for line in symbols.splitlines():
        parts = line.split(maxsplit=3)
        if len(parts) == 4 and parts[2] in {'T', 'W'}:
            symbol_rows[parts[3]] = (int(parts[0], 16), int(parts[1], 16))
    constructors = []
    for name in sorted({row['constructor'] for row in unit + formation}):
        address, size = symbol_rows[name]
        calls = [ins for _, ins in disassemble(path, address, size)
                 if ins.startswith('call') and 'FAutoVariableBase::FAutoVariableBase(' in ins]
        if len(calls) != 1:
            raise ValueError('unexpected typed wrapper registration: ' + name)
        constructors.append((address, size, name))
    unit_order = write_schema(path, 0x2C22D00, 0x4E6, unit + unregistered)
    formation_order = write_schema(path, 0x293B440, 0x114, formation)
    if len(unit_order) != 74 or len(formation_order) != 17:
        raise ValueError('incomplete write record')
    for order in [unit_order, formation_order]:
        order[0]['value'] = struct.unpack('<i', game.read(int(order[0]['address'], 16), 4))[0]
    expected = {10: ('m_eAutomation', '0x268'), 11: ('m_eActivity', '0x280'),
                32: ('m_iOperationTimer', '0x518'), 70: ('m_formationID', '0xba8'),
                71: ('m_kOperationsDisabled', '0xbd0'), 72: ('m_kOperations', '0xc00')}
    for index, (name, offset) in expected.items():
        if (unit[index]['name'], unit[index]['wrapper_offset']) != (name, offset):
            raise ValueError('affected Unit field identity mismatch')
    for address, target in [(0x2A53D33, 0x1E20270), (0x2A53D66, 0x1E57840),
                            (0x29447F3, 0x1E20270), (0x2944476, 0x1E57840),
                            (0x2A53BA1, 0x1F5FBD0), (0x2A53BD0, 0x1F040F0),
                            (0x2A53BDF, 0x1F040F0), (0x293B682, 0x1E1D6B0),
                            (0x293B6CF, 0x1ECE0B0)]:
        game.call(address, target)
    host.call(0x3C49C4A, 0x3C459E8)
    host.call(0x3C49C88, 0x3C459E8)
    game.call(0x293D8EB, 0x1E1FBE0)
    game.call(0x293D902, 0x1EA2B90)
    for slot, target in [(0xB0, 0x2D965A0), (0xB8, 0x2D96630)]:
        if game.relocated_pointer(0x3B00DA0 + 16 + slot) != target:
            raise ValueError('SkipTurn base serializer binding mismatch')
    if game.read(0x2C231D7, 6) != bytes.fromhex('ff90f8010000'):
        raise ValueError('properties serializer virtual mismatch')
    report = dict(schema='c6-linux-unit-archive-v1', gamecore_sha256=GAME_SHA,
                  host_sha256=HOST_SHA, static_checks_passed=True, native_execution=False,
                  native_index_mapping_validated=False, full_unit_codec_implemented=False,
                  full_affected_state_contract_complete=False,
                  unit_registered_variables=unit, unit_unregistered_wrappers=unregistered,
                  formation_registered_variables=formation, unit_write_order=unit_order,
                  formation_write_order=formation_order,
                  methods=fingerprint(game, CONSTRUCTORS + METHODS),
                  host_methods=fingerprint(host, HOST_METHODS),
                  typed_wrapper_constructors=fingerprint(game, constructors),
                  formation_wire_contract=dict(version=1, endian='little', scalar_prefix_bytes=36,
                      size_formula='52 + 20 * path_node_count + 4 * unit_id_count',
                      plot_coord=['int32:x', 'int32:y'], fixed_point='int32:raw',
                      player_and_domain='int32:enum_bits', booleans='8 one-byte bool fields in write order',
                      path_array_count='uint32', path_node_version=1,
                      path_node_fields=['int32:version', 'int32:data1_raw', 'int32:data2',
                                        'int32:flags', 'int16:x', 'int16:y'],
                      unit_ids_count='uint32', unit_ids_element='uint32',
                      reader_side_effects=['evaluatingOperation=false', 'resolve actual owner player',
                                           'set archive ID from actual component identity'],
                      native_wire_comparison=False),
                  skip_turn_serializer_slots={'write': '0x2d965a0', 'read': '0x2d96630'},
                  operation_versions={name: struct.unpack('<i', game.read(a, 4))[0] for name, a in
                                      [('queue', 0x2F96BCC), ('node', 0x2F94CC0),
                                       ('empty_parameters', 0x2F94CC4), ('path_node', 0x2F56760)]},
                  limits=['Static constructor indices require intact original archive vector; native getter cross-check pending.',
                          'Field order is not a recursive codec or Core Save byte offset map.',
                          'Definition-indexed and dynamic fields require exact Linux rules and nested serializers.',
                          'Unknown handlers/parameters are unsupported; opaque retention does not permit advancing them.',
                          'No action execution, complete-state parity, or policy-boundary acceptance.'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(f'checked 73 Unit registrations, 17 Formation registrations, {len(unit_order)} Unit write entries')


if __name__ == '__main__':
    main()
