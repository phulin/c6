#!/usr/bin/env python3
"""Audit bounded static startup contracts against the pinned Linux host bytes."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]
HOST_SHA = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
# Exact entry names from this ELF's dynamic symbol table, not transferred RVAs.
FUNCTIONS = {
    'run_app': (0x42b99f0, '_ZN7AppHost6RunAppEiPPcPNS_11ApplicationE'),
    'setup_platform': (0x42b4ad0, '_ZN7AppHost9_INTERNAL16SetupFXSPlatformEPKNS_14AppEnvironmentEPNS_10AppOptionsE'),
    'teardown_platform': (0x42b5260, '_ZN7AppHost9_INTERNAL19TeardownFXSPlatformEv'),
    'initial_init': (0x2f8d120, '_ZN7Civ6App11InitialInitEv'),
    'override_options': (0x2f8c990, '_ZN7Civ6App18OverrideAppOptionsEv'),
    'setup_content': (0x2f92a20, '_ZN7Civ6App17SetupContentPathsEv'),
    'shutdown_content': (0x2f986d0, '_ZN7Civ6App20ShutdownContentPathsEv'),
    'local_startup': (0x2f4f920, '_ZN3App11ContentPath5Local7StartupERKN7AppHost14AppEnvironmentERNS2_10AppOptionsEPKc'),
    'command_line': (0x2f4f450, '_ZN3App11ContentPath5Local25SetupCommandLineOverridesERKN7AppHost14AppEnvironmentERNS2_10AppOptionsE'),
    'manager_set': (0x2f4e610, '_ZN3App11ContentPath7Manager3SetEPS1_'),
}
# Each edge is checked as an actual x86-64 E8 relative CALL instruction.
CALLS = [
    ('run_app.setup_platform', 0x42b9e44, 0x42b4ad0),
    ('platform.perf_events', 0x42b4d7a, 0x3c42e9c),
    ('platform.get_temp_heap', 0x42b4d86, 0x3c3ee68),
    ('platform.temp_heap_startup', 0x42b4d94, 0x3c3ee86),
    ('platform.job_manager', 0x42b4dcf, 0x3c3f474),
    ('initial.events', 0x2f8dff7, 0x2f91a10),
    ('initial.network', 0x2f8f288, 0x2f91d60),
    ('initial.database', 0x2f8f4a2, 0x2f91db0),
    ('initial.localization', 0x2f8f8a0, 0x2f92970),
    ('initial.content', 0x2f8fcf9, 0x2f92a20),
    ('initial.ownership', 0x2f8ff57, 0x2f92a70),
    ('initial.external_live_client', 0x2f9081d, 0x31a1650),
    ('local.manager_ctor', 0x2f4f963, 0x2f4df90),
    ('local.manager_set', 0x2f4f96b, 0x2f4e610),
    ('local.catalog_scan', 0x2f4f970, 0x2f4fa10),
    ('local.content_definitions', 0x2f4f978, 0x2f4fa90),
    ('local.command_line', 0x2f4f9ad, 0x2f4f450),
    ('local.base_package', 0x2f4f9c5, 0x2f4e090),
    ('local.apply_changes', 0x2f4f9cd, 0x2f4e5c0),
]
STRINGS = [
    (0x45e9cea, False, 'Language'), (0x462ce11, False, 'AudioLanguage'),
    (0x45e2130, False, '-PlatformContent'), (0x45e2141, False, '-EnableDevFeatures'),
    (0x4658b1e, False, 'Base'), (0x45e2244, True, '../../../Base'),
    (0x45e227c, True, '*/Scenarios/*'), (0x45e22b4, True, '../../../Base/Assets'),
    (0x45e2308, True, '../../../Base/ArtDefs'),
    (0x45e2360, True, '../../../Base/Platforms/Windows/ARX'),
]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--host', type=Path, default=ROOT.parent / 'c6-assets-linux/Civ6')
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    blob = args.host.read_bytes()
    if hashlib.sha256(blob).hexdigest() != HOST_SHA:
        raise SystemExit('Pinned Linux host SHA-256 mismatch')
    if blob[:6] != b'\x7fELF\x02\x01':
        raise SystemExit('Expected little-endian ELF64')
    phoff = struct.unpack_from('<Q', blob, 32)[0]
    phsize, phnum = struct.unpack_from('<HH', blob, 54)
    segments = [struct.unpack_from('<IIQQQQQQ', blob, phoff + i * phsize)
                for i in range(phnum)]

    def read(va, count):
        for typ, _flags, offset, start, _pa, size, _mem, _align in segments:
            if typ == 1 and start <= va and va + count <= start + size:
                at = offset + va - start
                return blob[at:at + count]
        raise ValueError(f'Unbacked ELF virtual address: {va:x}')

    symbols = {}
    for line in subprocess.check_output(['nm', '-D', '-S', str(args.host)], text=True).splitlines():
        fields = line.split()
        if len(fields) == 4:
            symbols[fields[3]] = (int(fields[0], 16), int(fields[1], 16))
    functions = {}
    for key, (entry, symbol) in FUNCTIONS.items():
        actual, size = symbols[symbol]
        if actual != entry:
            raise ValueError(f'Original symbol entry mismatch: {key}')
        functions[key] = dict(entry=hex(entry), symbol=symbol, bytes=size,
                              sha256=hashlib.sha256(read(entry, size)).hexdigest())
    calls = []
    for label, site, expected in CALLS:
        raw = read(site, 5)
        actual = site + 5 + struct.unpack_from('<i', raw, 1)[0]
        if raw[0] != 0xe8 or actual != expected:
            raise ValueError(f'Original direct CALL mismatch: {label}')
        calls.append(dict(relationship=label, site=hex(site), target=hex(actual)))
    strings = []
    for va, wide, expected in STRINGS:
        encoded = expected.encode('utf-32-le' if wide else 'utf-8') + (b'\0' * (4 if wide else 1))
        if read(va, len(encoded)) != encoded:
            raise ValueError(f'Original string mismatch at {va:x}')
        strings.append(dict(address=hex(va), encoding='utf-32-le' if wide else 'utf-8', value=expected))
    result = dict(schema='c6-linux-startup-static-contracts-v1', host_sha256=HOST_SHA,
                  source_kind='actual supplied Linux ELF dynamic symbols and bytes',
                  functions=functions, verified_direct_calls=calls, verified_literals=strings,
                  contracts=dict(app_environment_offset='0x8', app_options_offset='0x880',
                                 argc_app_offset='0x30', argv_app_offset='0x38',
                                 temp_heap_default_bytes=16777216, temp_heap_option_offset='0x10',
                                 perf_events='unconditional SetupFXSPlatform call',
                                 job_manager='options+4 != 0 and options+0xc != 0',
                                 content_manager_allocation_bytes=64,
                                 content_manager_shutdown='Manager::Set(NULL) destroys/deletes previous manager',
                                 startup_scope='bounded static original-code contracts; no provider substitution'),
                  limits=['Static ordering and byte identity only; conditional paths remain conditional',
                          'Does not execute startup, synthesize providers, or establish initialized gameplay',
                          'Literal Windows directory names occur in the Linux target itself',
                          'PerfEvents Initialize is unconditional in SetupFXSPlatform; global optionality unproven'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(functions=len(functions), calls=len(calls), literals=len(strings))))


if __name__ == '__main__':
    main()
