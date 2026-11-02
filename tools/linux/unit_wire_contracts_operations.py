#!/usr/bin/env python3
"""Static Linux Unit operation wire audit; never runs original game code."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from unit_archive_audit import trace

PREFIX = 'GameCore::Unit::Operations::'
OBSERVED = {
    'XP2::MakeTradeRoute': 0x8374d954, 'MoveTo': 0x580f2f68,
    'Alert': 0xe547bc17, 'Fortify': 0xd3a6f7e8,
    'XP1::SpySiphonFunds': 0x223eea54, 'XP1::SpyDisruptRocketry': 0xc31647e8,
    'XP1::SpyGainSources': 0xed72b77d, 'XP1::SpySabotageProduction': 0x09c1ecc1,
}


def symbols(path):
    result = {}
    for line in subprocess.check_output(['nm', '-S', '-C', str(path)], text=True).splitlines():
        parts = line.split(maxsplit=3)
        if len(parts) != 4:
            continue
        try:
            address, size = int(parts[0], 16), int(parts[1], 16)
        except ValueError:
            continue
        result[parts[3]] = (address, size)
    return result


def relocations(elf):
    raw = elf.raw
    start = struct.unpack_from('<Q', raw, 40)[0]
    width, count = struct.unpack_from('<HH', raw, 58)
    sections = [struct.unpack_from('<IIQQQQIIQQ', raw, start + i * width) for i in range(count)]
    result = {}
    for section in sections:
        if section[1] != 4:
            continue
        for pos in range(section[4], section[4] + section[5], section[9]):
            target, info, addend = struct.unpack_from('<QQq', raw, pos)
            kind = info & 0xffffffff
            if kind == 8:
                result[target] = addend
            elif kind in (1, 6):
                symbol = sections[section[6]][4] + (info >> 32) * sections[section[6]][9]
                result[target] = struct.unpack_from('<Q', raw, symbol + 8)[0] + addend
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--private-dir', type=Path)
    args = parser.parse_args()
    path = args.assets / 'libGameCore_XP2.so'
    game = Elf(path, GAME_SHA)
    Elf(args.assets / 'Civ6', HOST_SHA)
    sy = symbols(path)
    inverse = {address: name for name, (address, _) in sy.items()}
    rel = relocations(game)
    # One bounded disassembly covers every Unit operation plus the shared parameter
    # and Resource enum helpers, preserving raw copyrighted evidence privately.
    raw = subprocess.check_output(['objdump', '-d', '-C', '--start-address=0x2d4ebe0',
                                   '--stop-address=0x2dcc000', str(path)], text=True)
    raw += subprocess.check_output(['objdump', '-d', '-C', '--start-address=0x2c9adc0',
                                    '--stop-address=0x2c9ae6c', str(path)], text=True)
    rows = []
    for line in raw.splitlines():
        match = re.match(r'\s*([0-9a-f]+):\s+(?:[0-9a-f]{2} )+\s*(.*)', line)
        if match:
            rows.append((int(match[1], 16), match[2]))
    def body(address, size):
        return [(a, ins) for a, ins in rows if address <= a < address + size]
    def method(name):
        address, size = sy[name]
        return dict(address=hex(address), size=size, name=name,
                    sha256=hashlib.sha256(game.read(address, size)).hexdigest())
    def pointer(table, slot):
        address = rel[table + 16 + slot]
        return dict(address=hex(address), name=inverse.get(address))
    def constant(address, expected):
        value = struct.unpack('<i', game.read(address, 4))[0]
        if value != expected:
            raise ValueError(f'version mismatch at {address:#x}')
        return dict(address=hex(address), value=value)
    versions = dict(node=constant(0x2f94cc0, 2), empty=constant(0x2f94cc4, 0),
                    single_location=constant(0x2f88ea0, 1), trade=constant(0x2f96490, 1))
    if game.read(0x2d96820, 10) != bytes.fromhex('0f57c00f11074889f8c3'):
        raise ValueError('null parameter factory mismatch')
    handlers, factories, parameter_types = [], {}, set()
    for name, (table, size) in sorted(sy.items()):
        if not name.startswith('vtable for ' + PREFIX) or size < 0xd8:
            continue
        cls = name.removeprefix('vtable for ' + PREFIX)
        if cls == 'Manager':
            continue
        write, read = pointer(table, 0xb0), pointer(table, 0xb8)
        if (write['address'], read['address']) != ('0x2d965a0', '0x2d96630'):
            raise ValueError(f'unclassified node serialization: {cls}')
        factory = pointer(table, 0x80)
        factory_name = factory['name']
        if factory_name not in factories:
            factory_address, factory_size = sy[factory_name]
            constructors = []
            for _, ins in body(factory_address, factory_size):
                m = re.search(r'<(GameCore::.*)::([^:<>]+)\(\)@plt>', ins)
                if m and m[1].rsplit('::', 1)[-1] == m[2]:
                    constructors.append(m[1])
            if factory_address == 0x2d96820:
                parameter_type = None
            elif len(set(constructors)) == 1:
                parameter_type = constructors[0]
                parameter_types.add(parameter_type)
            else:
                raise ValueError(f'parameter constructor unresolved: {factory_name}')
            factories[factory_name] = dict(**method(factory_name), parameter_type=parameter_type)
        item = dict(class_name=cls, vtable=hex(table), write=write, read=read,
                    make_parameters=factory, parameter_type=factories[factory_name]['parameter_type'])
        hash_name = PREFIX + cls + '::GetOperationHash()'
        if hash_name in sy:
            address, size = sy[hash_name]
            code = game.read(address, size)
            if size == 6 and code[0] == 0xb8 and code[-1] == 0xc3:
                item['operation_hash'] = f'0x{struct.unpack("<I", code[1:5])[0]:08x}'
                item['hash_method'] = method(hash_name)
            elif code == bytes.fromhex('31c0c3'):
                item['operation_hash'] = '0x00000000'
                item['hash_method'] = method(hash_name)
            else:
                raise ValueError(f'nonconstant hash: {cls}')
        register = PREFIX + cls + '::Register()'
        if register in sy:
            item['register_method'] = method(register)
        constructor = PREFIX + cls + '::' + cls.rsplit('::', 1)[-1] + '()'
        if constructor in sy:
            item['constructor_method'] = method(constructor)
        if cls in OBSERVED:
            ca, cz = sy[constructor]
            instructions = body(ca, cz)
            if not any(cls + '::GetOperationHash()@plt>' in ins for _, ins in instructions):
                raise ValueError(f'constructor hash provenance missing: {cls}')
            if not any(ins == 'mov    %eax,0xc(%rbx)' for _, ins in instructions):
                raise ValueError(f'constructor hash destination missing: {cls}')
            got = [int(m[1], 16) for _, ins in instructions
                   if (m := re.search(r'mov\s+.*%rip\),%rax\s+# ([0-9a-f]+)', ins))]
            if not any(rel.get(address) == table for address in got):
                raise ValueError(f'constructor vtable provenance missing: {cls}')
        handlers.append(item)
    for cls, expected in OBSERVED.items():
        item = next(x for x in handlers if x['class_name'] == cls)
        if item.get('operation_hash') != f'0x{expected:08x}':
            raise ValueError(f'observed operation hash mismatch: {cls}')
    parameters = []
    for cls in sorted(parameter_types):
        table, _ = sy['vtable for ' + cls]
        writer = pointer(table, 0x18)
        reader = pointer(table, 0x10)
        name = writer['name']
        address, size = sy[name]
        fields = []
        for call, ins, regs, version_address in trace(body(address, size), 'rdi'):
            if 'Serialization::' not in ins or not ins.startswith('call'):
                continue
            field = dict(ordinal=len(fields), call=hex(call), serializer=ins.split('<', 1)[1][:-1].removesuffix('@plt'))
            offset = regs.get('rsi')
            if offset is not None:
                field['object_offset'] = hex(offset)
            elif version_address is not None and 'operator<< <int>' in ins:
                field['constant_address'] = hex(version_address)
                field['constant_i32'] = struct.unpack('<i', game.read(version_address, 4))[0]
            elif regs.get('rdi') is not None:
                field['object_offset'] = hex(regs['rdi'])
                field['member_source_receiver'] = True
            else:
                field['unresolved_provenance'] = True
            fields.append(field)
        if any(f.get('unresolved_provenance') for f in fields):
            raise ValueError(f'unresolved parameter field provenance: {cls}')
        calls = [ins for _, ins in body(address, size) if ins.startswith('call')]
        if len(calls) != len(fields):
            raise ValueError(f'unclassified parameter writer call: {cls}')
        parameters.append(dict(class_name=cls, vtable=hex(table), writer=method(name),
                               reader=method(reader['name']), write_order=fields))
    shared = [PREFIX + 'BaseHandler::Write(Serialization::OStream&, GameCore::Unit::Operations::Node const&) const',
              PREFIX + 'BaseHandler::Read(Serialization::IStream&, GameCore::Unit::Operations::Node&)',
              'GameCore::SharedParameters::SingleLocation::Write(Serialization::OStream&) const',
              'GameCore::SharedParameters::SingleLocation::Read(Serialization::IStream&)',
              'Serialization::operator<<(Serialization::OStream&, GameCore::ResourceTypes const&)',
              'Serialization::operator>>(Serialization::IStream&, GameCore::ResourceTypes&)']
    enums = []
    for enum_type in ['DistrictTypes', 'ImprovementTypes', 'RouteTypes', 'WMDTypes', 'ResourceTypes']:
        writer = 'Serialization::operator<<(Serialization::OStream&, GameCore::' + enum_type + ' const&)'
        reader = 'Serialization::operator>>(Serialization::IStream&, GameCore::' + enum_type + '&)'
        a, z = sy[writer]
        calls = [ins.split('<', 1)[1][:-1].removesuffix('@plt')
                 for _, ins in body(a, z) if ins.startswith('call')]
        if len(calls) != 2 or 'Definitions::Get' not in calls[0] or 'WriteIndexToHash<' not in calls[1]:
            raise ValueError(f'unclassified enum helper: {enum_type}')
        a, z = sy[reader]
        rcalls = [ins.split('<', 1)[1][:-1].removesuffix('@plt')
                  for _, ins in body(a, z) if ins.startswith('call')]
        if len(rcalls) != 3 or 'operator>><unsigned int>' not in rcalls[0] or 'HashToIndex<' not in rcalls[2]:
            raise ValueError(f'unclassified enum reader: {enum_type}')
        enums.append(dict(enum_type=enum_type, wire_type='definition_hash:u32',
                          writer=method(writer), reader=method(reader),
                          writer_calls=calls, reader_calls=rcalls))
    unit_id_writer = 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::UnitID const&)'
    unit_id_reader = 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::UnitID&)'
    names = {'DistrictTypes': 'eDistrictType', 'ImprovementTypes': 'eImprovementType',
             'RouteTypes': 'eRouteType', 'WMDTypes': 'eWMDType', 'ResourceTypes': 'eResource'}
    for parameter in parameters:
        wire = []
        for field in parameter['write_order']:
            serializer = field['serializer']
            if 'constant_i32' in field:
                if field['constant_i32'] != 1:
                    raise ValueError('unexpected parameter version')
                wire.append(dict(name='version', wire_type='i32', value=1))
            elif 'operator<< <unsigned int>' in serializer:
                wire.append(dict(name='uiModifiers', wire_type='u32'))
            elif 'GameCore::PlotCoord const&' in serializer:
                name = 'kLocation1' if field['object_offset'] == '0x14' else 'kLocation0'
                wire.extend(dict(name=name + '.' + xy, wire_type='i32') for xy in ['x', 'y'])
            elif 'GameCore::Player::UnitID const&' in serializer:
                wire.extend(dict(name='kUnit0.' + name, wire_type=kind)
                            for name, kind in [('owner', 'i16'), ('type', 'i16'), ('id', 'u32')])
            elif 'TypedVariantMap::Serialize' in serializer:
                wire.append(dict(name='kParameters', wire_type='TypedVariantMap', options=1,
                                 contract='evidence/linux/unit-wire-contracts.json'))
            else:
                found = [enum_type for enum_type in names if 'GameCore::' + enum_type + ' const&' in serializer]
                if len(found) != 1:
                    raise ValueError(f'unclassified parameter scalar: {serializer}')
                wire.append(dict(name=names[found[0]], wire_type='definition_hash:u32', definition_enum=found[0]))
        parameter['wire_fields'] = wire
    parameter_map = {p['class_name']: p['wire_fields'] for p in parameters}
    groups = {}
    for handler in handlers:
        if 'operation_hash' in handler:
            groups.setdefault(handler['operation_hash'], []).append(handler)
    duplicate_hashes = []
    for operation_hash, entries in sorted(groups.items()):
        if len(entries) < 2:
            continue
        shapes = [parameter_map.get(entry['parameter_type'], [{'name': 'empty_version', 'wire_type': 'i32', 'value': 0}])
                  for entry in entries]
        duplicate_hashes.append(dict(operation_hash=operation_hash,
            classes=[entry['class_name'] for entry in entries],
            parameter_types=[entry['parameter_type'] for entry in entries],
            current_parameter_wire_shapes_equal=all(shape == shapes[0] for shape in shapes)))
        if not all(shape == shapes[0] for shape in shapes):
            raise ValueError(f'ambiguous current wire shape for hash {operation_hash}')
    report = dict(schema='c6-linux-unit-wire-contracts-operations-v1', gamecore_sha256=GAME_SHA,
                  host_sha256=HOST_SHA, static_checks_passed=True, native_execution=False,
                  reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175',
                  reference_attribution='cru121 Civ6-GameCore-reference; CC BY 4.0; docs/reference.md',
                  versions=versions, methods=[method(name) for name in shared],
                  base_node_write_order=['version:i32=2', 'directive:i32', 'flags:i32', 'push_turn:i32',
                                         'saved_activity:i32', 'saved_automation:i32', 'handler_parameters'],
                  observed_hashes={f'0x{v:08x}': k for k, v in OBSERVED.items()},
                  observed_parameter_contracts={
                      'none': ['empty_version:i32=0'],
                      'GameCore::SharedParameters::SingleLocation':
                          ['version:i32=1', 'location.x:i32', 'location.y:i32', 'modifiers:u32'],
                      PREFIX + 'MakeTradeRoute::ClassParameters':
                          ['version:i32=1', 'modifiers:u32', 'location0.x:i32', 'location0.y:i32',
                           'location1.x:i32', 'location1.y:i32', 'resource_definition_hash:u32']},
                  duplicate_hashes=duplicate_hashes, enum_helpers=enums, player_unit_id=dict(writer=method(unit_id_writer), reader=method(unit_id_reader),
                      wire_fields=['owner:i16', 'type:i16', 'id:u32']),
                  handlers=handlers, parameter_factories=list(factories.values()), parameter_types=parameters,
                  limitations=['Static class inventory does not establish runtime enabled registration/order.',
                               'TypedVariantMap nested contract is coordinated in evidence/linux/unit-wire-contracts.json.',
                               'Exact definitions/configuration, other versions and native boundary fixtures remain required.',
                               'No complete Unit load, gameplay operation or turn parity is claimed.'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    if args.private_dir:
        args.private_dir.mkdir(parents=True, exist_ok=True)
        (args.private_dir / 'operation-methods.asm').write_text(raw)
        commands = ['gdb', '-q', '-nx', '-batch', str(path)]
        for cls in sorted(parameter_types | {'GameCore::SharedParameters::Instance',
                                             'GameCore::SharedParameters::Location0',
                                             'GameCore::SharedParameters::Location1', 'GameCore::SharedParameters::Unit0'}):
            commands += ['-ex', f"ptype /o '{cls}'"]
        dwarf = subprocess.check_output(commands, text=True, stderr=subprocess.STDOUT)
        (args.private_dir / 'parameter-types.dwarf.txt').write_text(dwarf)
    print(json.dumps(dict(handlers=len(handlers), parameter_factories=len(factories),
                          parameter_types=len(parameters), observed_hashes=len(OBSERVED),
                          output=str(args.output))))


if __name__ == '__main__':
    main()
