#!/usr/bin/env python3
"""Audit pinned Linux trade wire identities; never load or mutate simulator state."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]
TARGET = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
ENTRIES = {
    'graph_write': 0x2bf7fe0, 'graph_read': 0x2bf8210,
    'route_write': 0x2c113f0, 'route_read': 0x2c114b0,
    'manager_write': 0x2c050b0, 'manager_read': 0x2c05100,
    'manager_initialize': 0x2c05590,
    'edge_write': 0x2bf8150, 'edge_read': 0x2bf83c0,
    'vertex_write': 0x2bf7f10, 'vertex_read': 0x2bf7f90,
    'yield_hash_write': 0x1ff74c0, 'yield_hash_read': 0x1ff7160,
    'get_edge_by_vertices': 0x2bfc240, 'iterate_routes': 0x2bfa860,
    'iterate_outgoing_routes': 0x2bfa910, 'validate_graph': 0x2bfc690,
    'city_id_write': 0x2c9ad10, 'city_id_read': 0x2c9ad70,
    'unit_id_write': 0x2c9adc0, 'unit_id_read': 0x2c9ae20,
    'fixed8_write': 0x2c9afe0, 'fixed8_read': 0x2c9b010,
    'path_vector_write': 0x20455c0, 'path_vector_read': 0x20452e0,
    'path_count_write': 0x2045690,
}
EDGE_FIELDS = [
    ('m_iID', 152, 'i32'), ('m_kMostRecentRoute', 176, 'Route'),
    ('m_bHasActiveRoute', 304, 'u8'), ('m_iRoutesStarted', 328, 'i32'),
    ('m_iRoutesCompleted', 352, 'i32'), ('m_vMostRecentPath', 376, 'u32 count; i32[count]'),
    ('m_iPathTraversalIndex', 424, 'i32'), ('m_bPathTraversalGoingForward', 448, 'u8'),
    ('m_kFromVertex', 472, 'Vertex'), ('m_kToVertex', 504, 'Vertex'),
]
ROUTE_FIELDS = [
    ('m_iID', 0, 'i32'), ('m_iStartTurn', 4, 'i32'), ('m_kTraderUnit', 8, 'ComponentID'),
    ('m_eResourceType', 32, 'u32 type hash'), ('m_iResourceQuantity', 36, 'i32'),
    ('m_Origin', 16, 'ComponentID'), ('m_Destination', 24, 'ComponentID'),
    ('m_axOriginYields', 40, 'YieldHashArray'), ('m_axDestinationYields', 72, 'YieldHashArray'),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    blob = args.module.read_bytes()
    if hashlib.sha256(blob).hexdigest() != TARGET:
        raise SystemExit('Pinned Linux GameCore SHA-256 mismatch')
    if blob[:6] != b'\x7fELF\x02\x01':
        raise ValueError('Expected little-endian ELF64')
    phoff = struct.unpack_from('<Q', blob, 32)[0]
    phsize, phnum = struct.unpack_from('<HH', blob, 54)
    segments = [struct.unpack_from('<IIQQQQQQ', blob, phoff + i * phsize) for i in range(phnum)]

    def read(va, size):
        for typ, _flags, offset, start, _pa, file_size, _mem, _align in segments:
            if typ == 1 and start <= va and va + size <= start + file_size:
                at = offset + va - start
                return blob[at:at + size]
        raise ValueError(f'Unbacked ELF virtual address {va:x}')

    symbols = {}
    for line in subprocess.check_output(['nm', '-D', '-S', str(args.module)], text=True).splitlines():
        fields = line.split()
        if len(fields) == 4:
            symbols.setdefault(int(fields[0], 16), []).append((fields[3], int(fields[1], 16)))
    methods = {}
    for label, address in ENTRIES.items():
        names = symbols[address]
        sizes = {size for _name, size in names}
        if len(sizes) != 1:
            raise ValueError(f'Ambiguous dynamic function extent: {label}')
        size = sizes.pop()
        methods[label] = dict(address=hex(address), bytes=size, symbols=sorted(name for name, _ in names),
                              sha256=hashlib.sha256(read(address, size)).hexdigest())
    for address, expected in [(0x2f6ee80, 1), (0x2f7072c, 4)]:
        if struct.unpack('<i', read(address, 4))[0] != expected:
            raise ValueError('Original serializer version constant mismatch')
    for address, encoded in [(0x2bf815a, bytes.fromhex('c744240403000000')),
                             (0x2bf7f1a, bytes.fromhex('c744240401000000'))]:
        if read(address, len(encoded)) != encoded:
            raise ValueError('Original stack version assignment mismatch')
    result = dict(
        schema='c6-linux-trade-serialization-contracts-v1', module_sha256=TARGET,
        scope='Exact-target static wire/lookup contracts; native typed fixture comparison is separate',
        methods=methods, versions=dict(graph=1, route=4, edge=3, vertex=1),
        layouts=dict(Graph=352, Edge=536, Route=112, Vertex=12, Manager=640, ComponentID=8),
        edge_fields=[dict(name=name, wrapper_offset=offset, wire=wire) for name, offset, wire in EDGE_FIELDS],
        route_fields=[dict(name=name, offset=offset, wire=wire) for name, offset, wire in ROUTE_FIELDS],
        encodings=dict(ComponentID=['i16 owner', 'i16 type', 'u32 id'],
                       Vertex=['i32 version=1', 'i32 type (sign-extended from native i16)', 'i32 data1', 'i32 data2'],
                       YieldHashArray=['u32 definition count', 'for each current definition: u32 type hash, i32 fixed8 raw'],
                       Graph=['i32 version=1', 'i32 totalEdgesCreated', 'i32 originCount',
                              'per origin in native hash iteration: Vertex, i32 destinationCount',
                              'per destination in native hash iteration: Vertex, Edge'],
                       Manager=['i32 version', 'Graph', 'i32 totalRoutesCreated']),
        route_omitted_fields=['m_bFinalizing at native offset104'],
        read_contract=dict(graph='Allocate Edge; Read; AttachAutoArchive; rebuild EdgeID, origin and destination indices without renumbering ID',
                           yield_hash_mapping='Resolve serialized hash to current definition index; consume unknown hash payload into temporary FixedPoint',
                           legacy='Edge version2 discards extra vector after path; Route<3 consumes obsolete arrays; strict current-version contract does not decode these branches'),
        ordering=dict(write='Native hash-container iteration, not sorted ID',
                      get_edge='Keyed origin lookup followed by destination lookup',
                      iterate_routes='Walk IDMap and invoke callback for active routes; rebuilt order can be observable',
                      limit='Keyed wire equality does not prove future callback/tie-break or turn equivalence'),
        limits=['Layouts checked against supplied Linux DWARF, not historical Windows evidence',
                'No opaque-byte normalization or gameplay state exclusion',
                'No complete save loader, initialized simulator, action or turn equivalence claim'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(methods=len(methods), current_versions=result['versions'])))


if __name__ == '__main__':
    main()
