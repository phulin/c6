#!/usr/bin/env python3
"""SHA-gated static nested Unit variant wire contracts (never executes the game)."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

METHODS = {
    'TypedVariant.write': (0x395ef70, 0x58),
    'TypedVariant.read': (0x395eff0, 0x60),
    'TypedVariantMap.write': (0x3961e50, 0xc4),
    'TypedKeyed.read': (0x3971330, 0x1e0),
    'TypedKeyed.header': (0x3971510, 0x6e),
    'TypedKeyed.member': (0x3971580, 0xce),
    'TypedVariantVector.write': (0x3963330, 0x6a),
    'TypedVariantVector.read': (0x39633c0, 0x89),
    'TypedVariantVectorMap.write': (0x3966c30, 0x73),
    'ArrayVariant.write': (0x3967da0, 0x11b),
    'VariantMap.write': (0x3967ec0, 0x16c),
    'ArrayVariant.write_default_options': (0x3968040, 0x16),
    'ArrayVariant.read': (0x3968060, 0x40a),
    'VariantMap.read': (0x3968470, 0xdf),
    'VariantVector.read_stub': (0x3968550, 4),
    'ArrayVariant.HasFlag': (0x2f63c30, 7),
    'ArrayVariant.count': (0x2f63c40, 9),
    'ArrayVariant.size': (0x2f63c50, 4),
    'WriteObject.u32': (0x2f66f20, 0xe),
    'WriteObject.u64': (0x2f68b70, 0xe),
    'WriteObject.i16': (0x3962650, 0xe),
    'WriteObject.u16': (0x3971de0, 0xe),
    'TypedVariant.Set.bool': (0x31d59f0, 0xf),
    'ArrayVariant.Set.bool': (0x31d5a00, 0xc),
    'TypedVariant.Set.i32': (0x31d5a10, 0xf),
    'ArrayVariant.Set.i32': (0x31d5a20, 0xb),
    'TypedVariant.Set.u32': (0x31ed0a0, 0xf),
    'ArrayVariant.Set.u32': (0x31ed0b0, 0xb),
    'TypedVariant.Set.f32': (0x31f8980, 0xf),
    'ArrayVariant.Set.f32': (0x31f8990, 0xd),
    'TypedVariant.Set.i64': (0x3962240, 0xf),
    'TypedVariant.Set.u64': (0x3962280, 0xf),
    'ArrayVariant.Set.i64': (0x3962610, 0x12),
    'ArrayVariant.Set.u64': (0x3962630, 0x12),
    'TypedVariant.Set.char8': (0x396f8f0, 0x11),
    'ArrayVariant.Set.char8': (0x396fb90, 0x96),
    'char8.length': (0x396f6b0, 0xd),
    'VariantType.default': (0x2f63810, 7),
    'ArrayVariant.default': (0x2f63820, 7),
    'ArrayVariant.SetFlags': (0x3246a40, 0xa),
    'ArrayVariant.Set.raw': (0x3246a20, 0x20),
    'ArrayVariant.Set.VariantMap': (0x395ee70, 0x1e),
    'ArrayVariant.Set.VariantVector': (0x395ee90, 0x1e),
    'ArrayVariant.Set.TypedVariantMap': (0x395e1d0, 0x1e),
    'ArrayVariant.Set.TypedVariantVector': (0x395e1f0, 0x1e),
    'ArrayVariant.Set.TypedVariantVectorMap': (0x395e210, 0x1e),
    'TypedVariant.Set.VariantMap': (0x395ecb0, 0xf),
    'TypedVariant.Set.VariantVector': (0x395ecc0, 0xf),
    'TypedVariant.Set.TypedVariantMap': (0x395e190, 0xf),
    'TypedVariant.Set.TypedVariantVector': (0x395e1b0, 0xf),
    'TypedVariant.Set.TypedVariantVectorMap': (0x395e1c0, 0xf),
    'VariantMap.Set.char16': (0x3969300, 0x4b),
    'char16.length': (0x3969350, 0xd),
}

VALUE_NAMES = [
    'None', 'Bool', 'Int32', 'UInt32', 'Float', 'char8', 'char16', 'GUID',
    'VariantMap', 'VariantVector', 'TypedVariantMap', 'TypedVariantVector',
    'Coord2DInt16', 'Coord2DInt32', 'Coord2DFloat', 'Coord3DInt16',
    'Coord3DInt32', 'Coord3DFloat', 'PairInt16', 'PairInt32', 'Int64',
    'UInt64', 'TypedVariantVectorMap', 'DDSImage', 'ZLibDDSImage', 'MaxBaseTypes',
]
FLAGS = {
    'OwnsMemory': 0x01000000, 'IsChildVariantMap': 0x02000000,
    'IsChildTypedVariantMap': 0x04000000, 'IsChildVariantVector': 0x08000000,
    'IsChildTypedVariantVector': 0x10000000, 'ValueIsArray': 0x20000000,
    'IsChildTypedVariantVectorMap': 0x40000000, 'ValueIsLong': 0x80000000,
}


def contracts():
    return {
        'wire_endian': 'little',
        'TypedVariant': {
            'sequence': ['i16 type_bits', 'i16 modifier_bits', 'ArrayVariant value'],
            'type_and_modifier_bits_preserved': True,
            'type_ids': dict(list(enumerate(VALUE_NAMES)) + [(1024, 'UserType')]),
            'modifier_ids': {'0': 'NoModifiers', '1': 'Array'},
            'payload_dispatch': 'ArrayVariant countFlags only; no switch on type or modifiers in writer/reader',
            'options': 'passed unchanged to ArrayVariant',
        },
        'VariantMap': {
            'sequence': ['i32 scalar_count', 'repeat scalar_count: u32 key,u32 value_bits',
                         'i32 array_count', 'repeat array_count: u32 key,ArrayVariant'],
            'entry_order': 'ascending unsigned key independently in each tree',
            'scalar_value': 'untyped 32 raw bits; manager type/name dictionary is not serialized here',
            'array_child_options': 0,
            'reader_negative_counts': 'treated as zero iterations independently for each tree',
            'reader_key': 'read through i32 then passed as unsigned key; all 32 bits retained',
        },
        'TypedKeyed': {
            'applies_to': ['TypedVariantMap', 'TypedVariantVectorMap'],
            'header': {
                'normal': ['u32 count'],
                'named': ['u32 0x80000001', 'u32 count'],
                'named_condition': 'count != 0 and (options & 1) != 0 and GetManager()!=nullptr',
                'empty_header': 'u32 zero even with names enabled',
                'reader': 'first word with bit31 set is prefix; names=prefix&1; read next u32 count; otherwise first word is count',
            },
            'member': ['u32 key', 'if names: u16 name_len then name_len raw char8 bytes', 'TypedVariant'],
            'name_length': 'strlen(name) truncated to u16; write that many bytes; no terminator; missing FindKey name writes u16 zero',
            'name_reader': 'read length/raw bytes, append in-memory NUL, call manager RegisterKey(name); ignore returned key and retain numeric wire key',
            'manager_dictionary_serialized': False,
            'options': {'0': 'NoOptions', '1': 'SaveKeyNamesIndividually/SaveKeyNames', '2': 'SaveKeyNamesDictionary (not acted upon by these methods)'},
            'recursive_options': 'member passes original options through TypedVariant; actual nested manager presence independently controls names',
            'ordering': {'TypedVariantMap': 'ascending unsigned key', 'TypedVariantVectorMap': 'stored keyed vector order'},
        },
        'TypedVariantVector': {
            'sequence': ['u32 count', 'repeat count: TypedVariant'],
            'ordering': 'stored vector order', 'recursive_options': 'unchanged',
        },
        'ArrayVariant': {
            'sequence': ['u32 countFlags', 'u32 elementSize', 'first matching payload branch'],
            'count_mask': '0x00ffffff', 'flags': {k: hex(v) for k, v in FLAGS.items()},
            'branches_in_precedence_order': [
                {'flag': '0x02000000', 'payload': 'VariantMap', 'options': 0},
                {'flag': '0x08000000', 'payload': 'none', 'note': 'reader creates empty VariantVector; no elements serialized'},
                {'flag': '0x04000000', 'payload': 'TypedVariantMap', 'options': 'unchanged'},
                {'flag': '0x10000000', 'payload': 'TypedVariantVector', 'options': 'unchanged'},
                {'flag': '0x40000000', 'payload': 'TypedVariantVectorMap', 'options': 'unchanged'},
                {'flag': '0x01000000', 'payload': 'raw buffer', 'byte_length': '((countFlags & 0xffffff) * elementSize) & 0xffffffff'},
                {'flag': '0x80000000', 'payload': 'u64 inline_bits'},
                {'flag': None, 'payload': 'u32 inline_bits'},
            ],
            'ValueIsArray': 'does not select a writer payload branch',
            'reader_normalization': [
                'Child setters normalize child metadata and add OwnsMemory; original redundant flags need not survive load.',
                'Raw branch allocates/read byte length with wrapping u32 multiplication, then Set(size,count,buffer,OwnsMemory), which adds both ValueIsArray and OwnsMemory.',
                'Inline32 branch reads u32 then, if ValueIsArray, zeros 8-byte union and countFlags (elementSize survives).',
            ],
            'semantic_limits': 'Framing is independent of declared type. A wire parser can preserve classified scalar bits/raw buffer bytes; that does not establish gameplay semantics of each declared type or authorize state advancement.',
        },
        'primitive_setters': {
            'None_default': 'type0,modifiers0; ArrayVariant constructor zeros all16 bytes => countFlags0,elementSize0,inline32 zero',
            'Bool': 'type1,elementSize0,countFlags0; setter stores only low byte, writer emits all four union bytes; retain upper bytes',
            'Int32': 'type2,elementSize0,countFlags0,inline32 two-complement bits',
            'UInt32': 'type3,elementSize0,countFlags0,inline32 bits',
            'Float': 'type4,elementSize0,countFlags0,inline32 IEEE754 bits; preserve NaN bits',
            'Int64': 'type20,elementSize0,countFlags0x80000000,inline64 bits',
            'UInt64': 'type21,elementSize0,countFlags0x80000000,inline64 bits',
            'raw_generic': 'Set(size,count,pointer,flags) stores count verbatim without masking, then ORs ((flags | ValueIsArray) & 0xff000000). Thus out-of-range count bits can collide with flags; reject excessive counts safely.',
            'char8': 'type5,modifiers0; nonempty fresh string setter uses elementSize1,count=strlen+1,OwnsMemory|ValueIsArray; raw bytes include NUL. Null/empty setter does not populate fresh value; serialize existing/default ArrayVariant metadata, not invented string framing.',
            'char16_untyped_map': 'VariantMap.SetVariant(key,char16*) passes elementSize2,count=char16_strlen+1 for nonempty input; includes terminal16-bit zero; null/empty passes size0,count0,null buffer. This does not establish a typed char16 constructor.',
            'children': 'Typed child setters set declared type8/9/10/11/22 and modifiers0. Array child setters zero size/count, store pointer, and for nonnull add child_flag | (caller_flags & 0xff000000); null leaves size/countFlags0,union pointer zero. Native child readers pass caller_flags=OwnsMemory, yielding child_flag|0x01000000 with count0,size0.',
            'other_declared_types': 'DWARF enum identity established; framing above applies, canonical construction/semantic element-width constraints not established by this audit',
        },
        'fail_closed_codec_requirements': [
            'Reject unsupported declared type/modifier semantics explicitly; do not silently treat an arbitrary byte blob as gameplay state.',
            'Bound recursion, counts and total bytes; reject products exceeding u32 and resource limits before allocation. Native wrapping is evidence, not permission for unsafe allocation.',
            'Keep countFlags/elementSize/type/modifier/scalar bits distinct; preserve float bits and string bytes exactly.',
            'Do not sort TypedVariantVectorMap; reject ambiguous duplicate/noncanonical map input unless reader normalization is explicitly modeled.',
            'No native oracle fixtures were executed; static framing is not native differential parity or complete Unit support.',
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path('../c6-assets-linux'))
    parser.add_argument('--output', type=Path, default=Path('evidence/linux/unit-wire-contracts-variants.json'))
    parser.add_argument('--private-output', type=Path)
    args = parser.parse_args()
    host_path = args.assets / 'Civ6'
    host = Elf(host_path, HOST_SHA)
    Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    vtables = {}
    for name, address, writer in [('TypedVariantMap', 0x53b3680, 0x3961e50),
                                  ('TypedVariantVectorMap', 0x53b39e0, 0x3966c30)]:
        slots = {hex(slot): hex(struct.unpack('<Q', host.read(address + 16 + slot, 8))[0])
                 for slot in [0x1f0, 0x1f8, 0x200, 0x208]}
        assert slots == {'0x1f0': '0x3971330', '0x1f8': hex(writer),
                         '0x200': '0x3971510', '0x208': '0x3971580'}, slots
        vtables[name] = {'address': hex(address), 'address_point_addend': 16, 'slots': slots}
    # Read enum identities from Linux target DWARF. No inferior is started.
    types = ['Data::ArrayVariant', 'Data::TypedVariant', 'Data::VariantValueType',
             'Data::VariantModifierType', 'Data::VariantSerializationOptions', 'Data::ArrayVariant::ElementFlags']
    command = ['gdb', '-q', '-nx', '-batch', str(args.assets / 'libGameCore_XP2.so')]
    for name in types:
        command += ['-ex', f"ptype /o '{name}'"]
    dwarf = subprocess.check_output(command, stderr=subprocess.STDOUT).decode()
    def enum_values(name):
        match = re.search(r'type = enum class ' + re.escape(name) + r' \{([^}]+)\}', dwarf)
        assert match is not None, name
        values, value = {}, -1
        for member in match[1].split(', '):
            parts = member.split(' = ')
            value = int(parts[1], 0) if len(parts) == 2 else value + 1
            values[parts[0].rsplit('::', 1)[1]] = value
        return values
    assert enum_values('Data::VariantValueType') == dict(
        [(f'dvt_{name}', value) for value, name in enumerate(VALUE_NAMES)] + [('dvt_UserType', 1024)])
    assert enum_values('Data::VariantModifierType') == {'dvt_NoModifiers': 0, 'dvt_Array': 1}
    enum_flags = enum_values('Data::ArrayVariant::ElementFlags')
    for name, value in FLAGS.items():
        assert enum_flags[name] & 0xffffffff == value, name
    fingerprints = {name: {'address': hex(address), 'size_bound': size,
                           'sha256': hashlib.sha256(host.read(address, size)).hexdigest()}
                    for name, (address, size) in METHODS.items()}
    result = {'schema': 'c6-linux-unit-wire-variants-v1', 'gamecore_sha256': GAME_SHA,
              'host_sha256': HOST_SHA, 'static_only': True, 'native_execution': False,
              'native_parity_validated': False, 'complete_unit_codec': False,
              'reference': {'repository': 'https://github.com/cru121/civ6-gamecore-reference',
                            'local_path': 'evidence/reference/upstream',
                            'commit': '5af389445118bed2bdebf7a8a42b219c8fba5175',
                            'lead': 'findings/topics/game-properties-storage.md',
                            'use': 'Discovery/ownership only; contracts independently read from pinned Linux host bytes and GameCore DWARF.'},
              'methods': fingerprints, 'vtables': vtables, 'contracts': contracts()}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    if args.private_output:
        args.private_output.mkdir(parents=True, exist_ok=True)
        (args.private_output / 'types.dwarf').write_text(dwarf)
        for name, (address, size) in METHODS.items():
            raw = subprocess.check_output(['objdump', '-d', '-C', f'--start-address={address}',
                                           f'--stop-address={address + size}', str(host_path)])
            (args.private_output / f'{name}.asm').write_bytes(raw)
    print(json.dumps({'output': str(args.output), 'methods': len(METHODS), 'native_execution': False}))


if __name__ == '__main__':
    main()
