#!/usr/bin/env python3
"""Fingerprint Unit registry ABI and audit private native observation fail closed."""
import argparse
import json
from pathlib import Path
import subprocess
import zlib
from definition_ids_audit import elf, sha, TARGET, HOST

ROOT = Path(__file__).resolve().parents[2]
METHODS = dict(unit_get=0x22147d0, unit_count=0x1fe82a0,
               unit_at=0x1fcd120, unit_hash=0x1fe5a70,
               unit_type_hash=0x1fe52c0, unit_index=0x1fcccb0,
               unit_rowid=0x2114a70, unit_name=0x1fe4b00,
               unit_find_index=0x290b650, unit_find=0x1ffc150,
               unit_find_lower=0x1ffc1d0, unit_find_compare=0x1ffc250,
               unit_find_upper=0x1ffc270, unit_find_upper_compare=0x1ffc300,
               unit_push=0x230f6f0, unit_push_lower=0x230fb50,
               unit_push_compare=0x230fbd0, unit_set_index=0x230f6e0,
               unit_set_type=0x221b740, unit_build_row=0x221b780,
               unit_build=0x225eec0, unit_get_sql=0x221c190,
               make_hash=0x2d577c0)


def static_contract(assets):
    module = assets / 'libGameCore_XP2.so'
    read = elf(module, TARGET)
    elf(assets / 'Civ6', HOST)
    symbols = {}
    for line in subprocess.check_output(['nm', '-D', '-S', str(module)], text=True).splitlines():
        f = line.split()
        if len(f) == 4:
            symbols.setdefault(int(f[0], 16), []).append((f[3], int(f[1], 16)))
    methods = {}
    for name, addr in METHODS.items():
        entries = symbols[addr]
        sizes = {n for _, n in entries}
        if len(sizes) != 1:
            raise ValueError('ambiguous target function extent')
        n = sizes.pop()
        methods[name] = dict(address=hex(addr), bytes=n, sha256=sha(read(addr, n)),
                             symbols=sorted(s for s, _ in entries))
    sql = read(0x2e427f8, 4096).split(b'\0')[0]
    if not sql.startswith(b'SELECT rowid, ') or not sql.endswith(b' FROM Units ORDER BY rowid'):
        raise ValueError('Unit builder ordering contract changed')
    return dict(schema='c6-linux-unit-definition-ids-v1', gamecore_sha256=TARGET,
                host_sha256=HOST, methods=methods,
                unit_builder_sql=dict(sha256=sha(sql), bytes=len(sql), order_by='rowid'),
                full_state_action_turn_parity=False, native_query=None)


def audit_runtime(runtime, contract, before_path, after_path):
    config = runtime['configuration']
    canonical = json.dumps(config, sort_keys=True, separators=(',', ':')).encode()
    if sha(canonical) != runtime['configuration_sha256']:
        raise ValueError('configuration digest mismatch')
    if config.get('schema') != 'c6-linux-mod-activation-v1':
        raise ValueError('unsupported runtime schema')
    assets = config.get('assets', {})
    if assets.get('libGameCore_XP2.so') != TARGET or assets.get('Civ6') != HOST:
        raise ValueError('named module/host identity missing')
    if config.get('lower_core_unit_definition_registry_observer') != contract:
        raise ValueError('Unit registry ABI gate missing')
    for path in ['native/linux/unit_definition_ids.c', 'native/linux/unit_definition_ids.h']:
        if config.get('sources', {}).get(path) != sha((ROOT / path).read_bytes()):
            raise ValueError('Unit observer source identity differs')
    if config.get('cleanup_requested') is not True:
        raise ValueError('cleanup not configured')
    if (type(runtime.get('returncode')) is not int or runtime['returncode'] != 0
            or runtime.get('framework_cleanup_proven') is not True
            or runtime.get('client_cleanup_proven') is not True):
        raise ValueError('native execution/cleanup failed')
    if not any(line.startswith('C6_LINUX_GAME_LOAD module ')
               and '/libGameCore_XP2.so context_initialized 1 ' in line
               for line in runtime['stderr'].splitlines()):
        raise ValueError('initialized XP2 context missing')
    records = [json.loads(line.removeprefix('C6_LINUX_UNIT_DEFINITION_IDS '))
               for line in runtime['stderr'].splitlines()
               if line.startswith('C6_LINUX_UNIT_DEFINITION_IDS ')]
    if len(records) != 1:
        raise ValueError('expected exactly one complete Unit query')
    query = records[0]
    entries = query['entries']
    if query['registry'] != 'Units' or type(query['count']) is not int or not 0 < query['count'] <= 1000000 or query['count'] != len(entries):
        raise ValueError('invalid Unit query dimensions')
    last_row = None
    for i, entry in enumerate(entries):
        name = bytes.fromhex(entry['type_hex'])
        if not name or b'\0' in name:
            raise ValueError('invalid native type bytes')
        if any(type(entry[k]) is not int for k in ['index','hash','native_string_hash','find_index']):
            raise ValueError('invalid native integer field')
        h = (~zlib.crc32(name)) & 0xffffffff
        if (entry['index'], entry['hash'], entry['native_string_hash'], entry['find_index']) != (i, h, h, i):
            raise ValueError('native index/hash/lookup disagrees')
        row = entry['database_rowid']
        if type(row) is not int or not -(2**31) <= row < 2**31:
            raise ValueError('invalid native signed rowid')
        # No rowid=index+1 assumption: zero and gaps are valid. This checked
        # profile has no narrowing wrap; unsupported wrap requires own fixture.
        if last_row is not None and row <= last_row:
            raise ValueError('profile rowids not strictly increasing')
        last_row = row
    if any(type(query[k]) is not int for k in ['index_zero','missing_hash','missing_result','empty_string_hash','lowercase_warrior_hash']):
        raise ValueError('invalid native boundary integer')
    if (query['index_zero'] != 0 or query['at_count_is_null'] is not True
            or not query['count'] <= query['missing_hash'] <= 0xffffffff
            or query['missing_result'] != -1 or query['empty_string_hash'] != 0xffffffff
            or query['lowercase_warrior_hash'] != (~zlib.crc32(b'unit_warrior')) & 0xffffffff):
        raise ValueError('native boundary check failed')
    captures = []
    for label, path in [('save', before_path), ('repeat', after_path)]:
        data = path.read_bytes()
        identity = dict(sha256=sha(data), bytes=len(data))
        if any(runtime['game_saves'][label][k] != v for k, v in identity.items()):
            raise ValueError('canonical capture identity mismatch')
        captures.append((data, identity))
    if captures[0][0] != captures[1][0]:
        raise ValueError('canonical Save changed around query')
    return dict(mapping=query, source_identity=config,
                audit_source_sha256=sha(Path(__file__).read_bytes()),
                configuration_sha256=runtime['configuration_sha256'],
                canonical_save=captures[0][1], canonical_core_unchanged=True,
                full_live_state_unchanged_proven=False)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--runtime-report', type=Path)
    p.add_argument('--before-save', type=Path)
    p.add_argument('--after-save', type=Path)
    p.add_argument('--private-output', type=Path)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    contract = static_contract(args.assets)
    result = dict(contract, native_query=None)
    if args.runtime_report:
        if not all([args.before_save, args.after_save, args.private_output]):
            p.error('native audit requires both canonical captures and --private-output')
        private = audit_runtime(json.loads(args.runtime_report.read_text()), contract,
                                args.before_save, args.after_save)
        private['runtime_report_sha256'] = sha(args.runtime_report.read_bytes())
        args.private_output.parent.mkdir(parents=True, exist_ok=True)
        args.private_output.write_text(json.dumps(private, sort_keys=True, indent=2) + '\n')
        result['native_query'] = dict(count=private['mapping']['count'],
            mapping_sha256=sha(json.dumps(private['mapping'],sort_keys=True,separators=(',', ':')).encode()),
            private_report_sha256=sha(args.private_output.read_bytes()),
            audit_source_sha256=private['audit_source_sha256'],
            runtime_report_sha256=private['runtime_report_sha256'],
            configuration_sha256=private['configuration_sha256'],
            canonical_save=private['canonical_save'], canonical_core_unchanged=True,
            full_live_state_unchanged_proven=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, sort_keys=True, indent=2) + '\n')
    print(json.dumps(dict(methods=len(contract['methods']), native_query=result['native_query'] is not None)))


if __name__ == '__main__':
    main()
