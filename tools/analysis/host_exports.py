#!/usr/bin/env python3
"""Read-only PE host-export inventory; string occurrences are leads, not calls."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct

EXE_SHA256 = 'e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63'
XP2_SHA256 = '324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c'


class PE:
    def __init__(self, data):
        self.data = data
        pe = self.u32(0x3c)
        if data[:2] != b'MZ' or data[pe:pe + 4] != b'PE\0\0':
            raise ValueError('not PE')
        optional = pe + 24
        if self.u16(optional) != 0x20b:
            raise ValueError('not PE32+')
        self.image_base = struct.unpack_from('<Q', data, optional + 24)[0]
        self.export_rva = self.u32(optional + 112)
        self.sections = []
        start = optional + self.u16(pe + 20)
        for index in range(self.u16(pe + 6)):
            at = start + index * 40
            virtual_size, rva, raw_size, raw = struct.unpack_from('<IIII', data, at + 8)
            self.sections.append((data[at:at + 8].rstrip(b'\0').decode(),
                                  rva, virtual_size, raw, raw_size))

    def u16(self, at):
        return struct.unpack_from('<H', self.data, at)[0]

    def u32(self, at):
        return struct.unpack_from('<I', self.data, at)[0]

    def offset(self, rva):
        for _, base, _, raw, size in self.sections:
            if base <= rva < base + size:
                return raw + rva - base
        raise ValueError('RVA has no file-backed bytes: ' + hex(rva))

    def rva(self, offset):
        for _, base, _, raw, size in self.sections:
            if raw <= offset < raw + size:
                return base + offset - raw
        return None

    def exports(self):
        result = {}
        if not self.export_rva:
            return result
        at = self.offset(self.export_rva)
        count = self.u32(at + 24)
        functions, names, ordinals = [self.offset(self.u32(at + n)) for n in (28, 32, 36)]
        for index in range(count):
            name_at = self.offset(self.u32(names + index * 4))
            end = self.data.index(0, name_at)
            name = self.data[name_at:end].decode('ascii')
            result[name] = self.u32(functions + self.u16(ordinals + index * 2) * 4)
        return result


def inventory(root):
    executable = root / 'Base/Binaries/Win64Steam/CivilizationVI.exe'
    data = executable.read_bytes()
    if hashlib.sha256(data).hexdigest() != EXE_SHA256:
        raise ValueError('original executable SHA-256 gate failed')
    xp2 = root / 'DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll'
    if hashlib.sha256(xp2.read_bytes()).hexdigest() != XP2_SHA256:
        raise ValueError('audited XP2 DLL SHA-256 gate failed')
    pe = PE(data)
    getter = pe.offset(0x9aee10)
    if (data[getter:getter + 3] != bytes.fromhex('488d05') or
            data[getter + 7] != 0xc3 or
            0x9aee17 + struct.unpack_from('<i', data, getter + 3)[0] != 0x2d227d8):
        raise ValueError('audited empty-string getter shape changed')
    if not any(base + size <= 0x2d227d0 and 0x2d227dc <= base + virtual_size
               for _, base, virtual_size, _, size in pe.sections):
        raise ValueError('audited empty-string sentinel is not loader-zero-filled')
    exports = {name: hex(rva) for name, rva in pe.exports().items() if name.startswith('EXP_')}
    modules = []
    for path in sorted(root.rglob('*.dll')):
        if '/Binaries/' not in path.as_posix():
            continue
        data = path.read_bytes()
        module = PE(data)
        found = {}
        for match in re.finditer(rb'(?<![A-Za-z0-9_])EXP_[A-Za-z0-9_]+\0', data):
            name = match.group()[:-1].decode()
            found.setdefault(name, []).append(hex(module.rva(match.start()) or 0))
        if found:
            modules.append({'path': path.relative_to(root).as_posix(),
                            'sha256': hashlib.sha256(data).hexdigest(),
                            'exp_string_rvas': found})
    return {'format': 'c6.host-exports.v1', 'executable_sha256': EXE_SHA256,
            'audited_xp2_sha256': XP2_SHA256,
            'executable_exports': exports, 'modules': modules,
            'empty_string': {'getter_rva': '0x9aee10', 'sentinel_rva': '0x2d227d8',
                             'capacity_header_rva': '0x2d227d0',
                             'length_header_rva': '0x2d227d4',
                             'storage': 'PE loader zero-fill; no CRT initialization needed',
                             'abi': 'Windows x64; no arguments; pointer result in RAX',
                             'getter_dependencies': []},
            'xp2_verified_dynamic_request_wrappers': {
                'EXP_GetLoggingOptions': '0x991880',
                'EXP_GetLogEventCallback': '0x9918e0',
                'EXP_JobManager_CreateJobList': '0x992160',
                'EXP_JobManager_DestroyJobSet': '0x992190',
                'EXP_JobManager_FreeJobList': '0x9921d0',
                'EXP_JobManager_PushJobOntoList': '0x992210',
                'EXP_JobManager_SpawnList': '0x992280',
                'EXP_JobManager_Wait': '0x9922c0',
                'EXP_GetTempHeap': '0x9934b0',
                'EXP_GetEmptyBasicString': '0x993b30',
                'EXP_GetTelemetrySessionHash': '0x9947a0',
                'EXP_GetTelemetryEventSystem': '0x9947e0'},
            'limits': ['EXP string occurrences are potential dynamic requests, not proven call sites.',
                       'Only the empty-string getter is audited for uninitialized-image delegation.',
                       'No game initialization, service equivalence or gameplay parity is established.']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('asset_root', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    result = inventory(args.asset_root.resolve())
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'executable_exports': len(result['executable_exports']),
                      'modules_with_exp_strings': len(result['modules'])}))


if __name__ == '__main__':
    main()
