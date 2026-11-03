#!/usr/bin/env python3
"""Gate native source identities and compare fresh Trade Read order, no native execution."""
import argparse
import gzip
import hashlib
import json
import re
from pathlib import Path
import struct
import subprocess
import tempfile

from save_normalization_trade import INITIAL, RELOADED, START, END, SIZE, Reader
from trade_serialization_audit import TARGET

ROOT = Path(__file__).resolve().parents[2]
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
# Complete symbol extents, never guessed instruction windows.
MODULE_METHODS = {
    'graph_read': 0x2bf8210, 'graph_write': 0x2bf7fe0,
    'graph_construct': 0x2bf9740, 'graph_clear': 0x2bf99c0,
    'vertex_hash': 0x2bf8540, 'bitwise_hash': 0x2bfcd30, 'fnv_bytes': 0x20ea2f0,
    'outer_hash': 0x2bfe2e0, 'outer_insert': 0x2bfe010,
    'outer_rehash': 0x2bfe210, 'outer_construct': 0x2c038a0,
    'outer_clear': 0x2bfdae0, 'outer_begin': 0x2bfca40,
    'outer_advance': 0x2bfdef0, 'outer_next_bucket': 0x2bfdeb0,
    'outer_find_node': 0x2bfe170, 'outer_find': 0x2bfd540,
    'outer_allocate_node': 0x2bfe1c0, 'outer_pair_construct': 0x2bfe360,
    'inner_construct_default': 0x2bfe380, 'inner_construct': 0x2bfe390,
    'inner_set': 0x2bfcc90, 'inner_can_set': 0x2bfe9a0,
    'inner_resize': 0x2bfe9c0, 'inner_set_base': 0x2bfeb20,
    'inner_copy': 0x2bfed50, 'inner_copy_insert': 0x2bfee00,
    'inner_clear': 0x2bfedd0, 'inner_advance': 0x2bfdfd0,
    'inner_hash': 0x2bfed20, 'inner_uint64_fold': 0x2bfefb0,
    'inner_range': 0x28e91c0, 'inner_probe_length': 0x28e91f0,
    'inner_key_equal': 0x2bfefa0, 'prime_policy_construct': 0x1fced20,
    'iterate_routes': 0x2bfa860, 'iterate_outgoing_routes': 0x2bfa910,
}
HOST_METHODS = {'prime_next_count': 0x3974b36, 'prime_rehash_required': 0x3974c2a}
SOURCES = ['crates/c6-core/src/trade_read_order.rs', 'crates/c6-core/src/trade.rs',
           'crates/c6-core/examples/trade_read_order.rs', 'tools/linux/trade_read_order_audit.py']


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Elf:
    def __init__(self, path, expected):
        self.path = path
        self.blob = path.read_bytes()
        if sha(self.blob) != expected:
            raise ValueError(f'unsupported target identity: {path}')
        if self.blob[:6] != b'\x7fELF\x02\x01':
            raise ValueError('expected little-endian ELF64')
        at = struct.unpack_from('<Q', self.blob, 32)[0]
        width, count = struct.unpack_from('<HH', self.blob, 54)
        self.segments = [struct.unpack_from('<IIQQQQQQ', self.blob, at + i * width)
                         for i in range(count)]
        self.symbols = {}
        for line in subprocess.check_output(['nm', '-D', '-S', str(path)], text=True).splitlines():
            fields = line.split()
            if len(fields) == 4:
                self.symbols.setdefault(int(fields[0], 16), []).append((fields[3], int(fields[1], 16)))

    def read(self, va, size):
        for typ, _flags, offset, start, _pa, file_size, _mem, _align in self.segments:
            if typ == 1 and start <= va and va + size <= start + file_size:
                at = offset + va - start
                return self.blob[at:at + size]
        raise ValueError(f'unbacked ELF VA {va:x}')

    def methods(self, addresses):
        result = {}
        for label, address in addresses.items():
            symbols = self.symbols[address]
            sizes = {size for _, size in symbols}
            if len(sizes) != 1:
                raise ValueError('ambiguous symbol extent')
            size = sizes.pop()
            result[label] = dict(va=hex(address), bytes=size,
                                 sha256=sha(self.read(address, size)),
                                 symbols=sorted(name for name, _ in symbols))
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--private-root', type=Path, default=ROOT.parent / 'c6-ghidra')
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    module = Elf(args.assets / 'libGameCore_XP2.so', TARGET)
    host = Elf(args.assets / 'Civ6', HOST)
    primes = struct.unpack('<256I', host.read(0x46433c0, 1024))
    if module.read(0x2e038d8, 4) != struct.pack('<f', 0.7):
        raise ValueError('inner default load mismatch')
    if module.read(0x2dfac6c, 4) != struct.pack('<f', 1.0):
        raise ValueError('outer default load mismatch')
    buckets = [53]
    while buckets[-1] < 1_000_000:
        buckets.append(next(p for p in primes if p >= buckets[-1] * 2))
    rust_source = (ROOT / SOURCES[0]).read_text()
    encoded_buckets = re.search(r'const BUCKETS: &\[usize\] = &\[(.*?)\];', rust_source, re.S)
    if not encoded_buckets or [int(x) for x in re.findall(r'\d+', encoded_buckets[1])] != buckets:
        raise ValueError('Rust growth sequence differs from pinned host prime policy')
    typed_report = json.loads((ROOT / 'evidence/linux/trade-rust-codec-1.json').read_text())
    graphs, complete = [], []
    for index, identity in enumerate([INITIAL, RELOADED]):
        data = gzip.decompress((args.private_root / 'evidence/linux-game-load/canonical' /
                                f'{identity}.bin.gz').read_bytes())
        if len(data) != SIZE or sha(data) != identity:
            raise ValueError('canonical full stream mismatch')
        graph = data[START:END]
        if sha(graph) != typed_report['graph_sha256'][index]:
            raise ValueError('typed complete Graph owner mismatch')
        decoded = Reader(data).graph()
        graphs.append((graph, decoded))
        complete.append(data)
    comparisons = []
    with tempfile.TemporaryDirectory(prefix='c6-trade-order-') as temporary:
        directory = Path(temporary)
        for index, (graph, _decoded) in enumerate(graphs):
            (directory / f'{index}.bin').write_bytes(graph)
        subprocess.run(['cargo', 'build', '-q', '-p', 'c6-core', '--example', 'trade_read_order'],
                       cwd=ROOT, check=True)
        adapter = ROOT / 'target/debug/examples/trade_read_order'
        for index in range(2):
            output = directory / f'reconstructed-{index}.bin'
            observed = json.loads(subprocess.check_output([
                str(adapter), str(directory / f'{index}.bin'), str(directory / '1.bin'),
                str(output)], text=True))
            generated = output.read_bytes()
            observed.update(input_sha256=sha(graphs[index][0]),
                            reconstructed_sha256=sha(generated),
                            compared_to_first_owner_differing_bytes=sum(a != b for a, b in zip(generated, graphs[0][0])),
                            compared_to_reloaded_owner_differing_bytes=sum(a != b for a, b in zip(generated, graphs[1][0])))
            if not observed['logical_equal_input'] or (index == 0 and not observed['encoded_equal_expected']):
                raise ValueError('source reconstruction differs from complete native reloaded owner')
            comparisons.append(observed)
        adapter_sha = sha(adapter.read_bytes())
    result = dict(schema='c6-linux-trade-read-order-v1', module_sha256=TARGET,
                  host_sha256=HOST, host_va_convention='ET_EXEC absolute virtual addresses',
                  methods=dict(module=module.methods(MODULE_METHODS), host=host.methods(HOST_METHODS)),
                  constants=dict(outer_initial_request=51, outer_buckets=buckets,
                                 outer_load_factor=1.0, outer_growth_factor=2.0,
                                 inner_initial_slots=4, inner_load_factor_f32=0.7,
                                 host_prime_table_va='0x46433c0', host_prime_table_sha256=sha(host.read(0x46433c0, 1024))),
                  graph_range=[START, END], stream_sha256=[INITIAL, RELOADED],
                  typed_owner_report='evidence/linux/trade-rust-codec-1.json',
                  comparisons=comparisons, adapter_sha256=adapter_sha,
                  raw_full_stream_differing_bytes=sum(a != b for a, b in zip(*complete)),
                  raw_trade_owner_differing_bytes=sum(a != b for a, b in zip(graphs[0][0], graphs[1][0])),
                  source_sha256={path: sha((ROOT / path).read_bytes()) for path in SOURCES},
                  scope='Source-derived fresh default-container serialized order; no native execution or live gameplay acceptance',
                  limits=['Existing-container Graph Read requires pre-read container state; this API requires fresh defaults.',
                          'ID/reverse map, allocator/archive lifecycle, Route finalizing and gameplay remain separate.',
                          'Canonical outputs were native observed lower diagnostic Core loads; full saved host lifecycle remains incomplete.',
                          'All raw 89198 changed bytes remain unmasked.'])
    if result['raw_full_stream_differing_bytes'] != 89198 or result['raw_trade_owner_differing_bytes'] != 89156:
        raise ValueError('canonical raw discrepancy changed')
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(comparisons=comparisons, raw_full_stream_differing_bytes=89198)))


if __name__ == '__main__':
    main()
