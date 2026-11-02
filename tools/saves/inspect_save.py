#!/usr/bin/env python3
"""Bounded Civ6 benchmark container metadata inspection; never loads gameplay state.

Read contracts recovered from fingerprinted XP2 DLL: map 0x9a9a40,
variant 0x9a0e50, payload 0x9a44f0, list 0x9a8280. Unknown flags fail closed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import zlib

SAVE_HASHES = {
    'XP2Benchmark.Civ6Save': 'a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8',
    'AIBenchmark.Civ6Save': '097ac9c8719f0e62ee67b1cfb6596cc27c7f07a26308b71396c9eda89876c61c',
    'GraphicsBenchmark.Civ6Save': 'f9cc2c82af76d37aabd8ff50db8ffd3408713258d1874c5b66169fc7a685e700',
}
DLL_HASH = '324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c'

class Reader:
    def __init__(self, data):
        self.data, self.pos, self.nodes = data, 0, 0

    def take(self, size):
        if size < 0 or self.pos + size > len(self.data):
            raise ValueError(f'truncated read at {self.pos:#x}, size {size}')
        result = self.data[self.pos:self.pos + size]
        self.pos += size
        return result

    def u32(self):
        return struct.unpack('<I', self.take(4))[0]

    def count(self):
        count = self.u32()
        if count > 100000:
            raise ValueError(f'unsupported count at {self.pos - 4:#x}: {count:#x}')
        return count

    def map(self, depth=0):
        rows = []
        for _ in range(self.count()):
            offset = self.pos
            key = self.u32()
            row = self.variant(depth + 1)
            row.update(key=f'{key:08x}', offset=offset)
            rows.append(row)
        return rows

    def variant(self, depth):
        self.nodes += 1
        if depth > 64 or self.nodes > 200000:
            raise ValueError('metadata resource limit')
        kind, auxiliary = struct.unpack('<HH', self.take(4))
        flags, width = self.u32(), self.u32()
        row = dict(type=kind, auxiliary=auxiliary, flags=f'{flags:08x}', width=width)
        if flags & 0x7e000000 == 0x10000000:
            row['items'] = [self.variant(depth + 1) for _ in range(self.count())]
        elif flags & 0x7e000000 == 0x04000000:
            row['entries'] = self.map(depth + 1)
        elif flags & 0x01000000 and flags & 0x5e000000 == 0:
            size = (flags & 0xffffff) * width
            raw = self.take(size)
            if kind in (5, 6):
                encoding = 'utf-8' if kind == 5 else 'utf-16-le'
                if width != (1 if kind == 5 else 2):
                    raise ValueError('unexpected text element width')
                row['text'] = raw.decode(encoding).rstrip('\0')
            else:
                row['bytes'] = size
                row['sha256'] = hashlib.sha256(raw).hexdigest()
        elif flags & 0x5f000000 == 0:
            raw = self.take(8 if flags & 0x80000000 else 4)
            row['unsigned'] = int.from_bytes(raw, 'little')
        else:
            raise ValueError(f'unsupported payload flags {flags:#x} at {self.pos:#x}')
        return row


def inspect(path):
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if SAVE_HASHES.get(path.name) != digest:
        raise ValueError('unsupported benchmark save identity')
    r = Reader(data)
    if r.take(4) != b'CIV6' or r.u32() != 1:
        raise ValueError('unsupported container magic/version')
    metadata = r.map()
    return dict(save=path.name, sha256=digest, size=len(data), container_version=1,
                metadata_end=r.pos, metadata=metadata,
                gamecore_stream_offset=None, initialized_game=False)


def extract_xp2(path, destination):
    """Extract a fingerprinted observed stream boundary, not a general save loader.

    The final chunk uses Z_SYNC_FLUSH. It has no Z_STREAM_END/Adler trailer;
    section termination requires the original consumer's requested reads.
    Therefore this probe pins the observed 111-chunk fixture boundary explicitly.
    """
    result = inspect(path)
    if path.name != 'XP2Benchmark.Civ6Save':
        raise ValueError('stream extraction only established for XP2Benchmark')
    data = path.read_bytes()
    r = Reader(data)
    r.pos = result['metadata_end']
    config_start = r.pos
    if (r.u32(), r.u32()) != (3, 1):
        raise ValueError('unexpected configuration versions')
    config_metadata = r.map()
    blocks = []
    for expected in (67, 0, 0):
        start = r.pos
        if r.u32() != 1:
            raise ValueError('unexpected configuration block version')
        entries = r.map()
        if len(entries) != expected:
            raise ValueError('unexpected configuration block count')
        blocks.append(dict(start=start, end=r.pos, entries=entries))
    for expected in (15, 67):
        start = r.pos
        entries = r.map()
        if len(entries) != expected:
            raise ValueError('unexpected configuration map count')
        blocks.append(dict(start=start, end=r.pos, entries=entries))
    if r.pos != 103901:
        raise ValueError('unexpected compressed section start')
    inflater = zlib.decompressobj()
    payload = bytearray()
    chunks = []
    for index in range(111):
        start = r.pos
        size = r.u32()
        if size != (65536 if index < 110 else 6366):
            raise ValueError('unexpected pinned compressed chunk size')
        compressed = r.take(size)
        payload.extend(inflater.decompress(compressed, 128 * 1024 * 1024 - len(payload)))
        if inflater.unconsumed_tail or inflater.unused_data or inflater.eof:
            raise ValueError('unexpected inflate boundary')
        chunks.append(dict(offset=start, size=size))
    if r.pos != 7319671 or not compressed.endswith(bytes.fromhex('0000ffff')):
        raise ValueError('unexpected sync-flush boundary')
    if struct.unpack_from('<I', payload)[0] != 4:
        raise ValueError('unexpected context serialization version')
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(payload)
    result.update(configuration_start=config_start, configuration_metadata=config_metadata,
                  configuration_blocks=blocks, gamecore_stream_offset=103901,
                  gamecore_compressed_end=r.pos, gamecore_chunk_count=len(chunks),
                  gamecore_payload_bytes=len(payload),
                  gamecore_payload_sha256=hashlib.sha256(payload).hexdigest(),
                  gamecore_serialization_version=4, zlib_eof=inflater.eof,
                  boundary_basis='pinned fixture / final short sync-flush chunk; original consumer not run')
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('save', type=Path)
    p.add_argument('--extract-xp2-stream', type=Path, help='Private output for pinned decompressed payload')
    p.add_argument('--output', type=Path, help='Private output: contains decoded source metadata')
    args = p.parse_args()
    result = (extract_xp2(args.save, args.extract_xp2_stream)
              if args.extract_xp2_stream else inspect(args.save))
    text = json.dumps(result, indent=2, ensure_ascii=False) + '\n'
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text)
        print(json.dumps({k: v for k, v in result.items() if k not in ('metadata', 'configuration_metadata', 'configuration_blocks')}))
    else:
        print(text, end='')

if __name__ == '__main__':
    main()
