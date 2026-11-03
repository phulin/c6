#!/usr/bin/env python3
"""Read-only target-byte/DWARF audit for bounded Context v4/v5 framing.

Never executes original code, scans for stream offsets, or changes private data.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
RAW_SHA = 'a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa'
DWARF_SHA = 'cc6452c25ae302d9632451e872673f949b49ebf6a2e03a874416f4061606cd12'
METHODS = [
    ('Civ6', 60172032, 32, 'typed rbtree reset', 'f7f099fe617f7bdfb4fac1b780b70ad5f21449e430253fc75c167ef2c18be17d'),
    ('Civ6', 60171952, 60, 'typed rbtree constructor', 'e26d3f673bd1588d15d427f0b45d8ecf959e47cd8841a1815641985275de84b4'),
    ('Civ6', 60170224, 5, 'typed map constructor', '3965f0de731d28ba8606926cbc545d7213d8f943b510c91bc0419b16e2904a7b'),
    ('libGameCore_XP2.so', 43039904, 310, 'Context.Read', '81dc994f9f6ad8bece5e2b421974aa6b3e78284d692e3c3323c417ad8ac7e371'),
    ('libGameCore_XP2.so', 43040512, 246, 'Context.ReadMetaData', '4362a58cea537d57fcd8815e6b1071c065278365709672169640b0a393ed9d29'),
    ('libGameCore_XP2.so', 43040768, 512, 'Context constructor', 'd390f3d34609b5a9f68a9bc782266098f5dff7498551d1556b236027970549fe'),
    ('libGameCore_XP2.so', 47399568, 12, 'IndexSerializer.LoadVersion', '814a4c9eb1631167536e0a585e613e070b286b3815881d2b9b01950a1ab2df61'),
    ('Civ6', 60158064, 124, 'TypedVariantMap constructor', '88ef146bf7cc119692ceb6363ed7eb31b05e1b25734903162c34b355bfca021e'),
    ('Civ6', 60159104, 95, 'GetVariant', 'b264f2c22424685f2c0d51945a672854768f0363768ac961e0bd0de247274118'),
    ('Civ6', 49689536, 125, 'unsigned conversion', '380b88868b20051709d622ff110351a07618ef077476ba5699935aa0b031581c'),
    ('Civ6', 49687984, 22, 'TypedVariant constructor', '657bc2880cacf477c0c3d599b06c494003f6a3cca16d5b26c8eb7aa26e7639d9'),
    ('Civ6', 49690640, 7, 'VariantType constructor', '1df8eed5de3158f416b76de6766b079a7067de033c465a3a0fb4c27772ff3be4'),
    ('Civ6', 49690656, 7, 'ArrayVariant constructor', '25c2c7617c598376bcc810f7665c963b57f80404e174ece55bbcd282c571b334'),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--raw-input', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/saves/xp2-gamecore-stream.bin')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    sources = {'libGameCore_XP2.so': Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA),
               'Civ6': Elf(args.assets / 'Civ6', HOST_SHA)}
    methods = []
    for image, address, size, name, digest in METHODS:
        if sha(sources[image].read(address, size)) != digest:
            raise ValueError('method mismatch: ' + name)
        methods.append(dict(image=image, address=hex(address), size=size, name=name, sha256=digest))
    game = sources['libGameCore_XP2.so']
    for address, target in [(0x290bcdd, 0x1f9be20), (0x290bcf1, 0x1eca1e0),
                            (0x290bd1b, 0x1eacef0), (0x290bd4f, 0x1f3dd30)]:
        game.call(address, target)
    # Whole-method fingerprints above gate these signed-version branches.
    dwarf = subprocess.check_output(['gdb', '-nx', '-batch', str(args.assets / 'libGameCore_XP2.so'),
                                     '-ex', "ptype /o 'GameCore::Context::Instance'"])
    if sha(dwarf) != DWARF_SHA:
        raise ValueError('Context DWARF mismatch')
    raw = args.raw_input.read_bytes()
    if len(raw) != 76_556_938 or sha(raw) != RAW_SHA:
        raise ValueError('requires immutable original a667 Read-input')
    tags = struct.unpack_from('<3i', raw)
    if tags != (4, 1, 19):
        raise ValueError('fixed byte-zero Context/Index/Game header mismatch')
    report = dict(
        schema='c6-linux-context-versions-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, methods=methods,
        dwarf_type='GameCore::Context::Instance', dwarf_sha256=DWARF_SHA,
        audit_source_sha256=sha(Path(__file__).read_bytes()),
        context_versions_supported=[4, 5],
        version_delta='v4 omits the TypedVariantMap; both read IndexSerializer before Game',
        metadata_v4='constructed empty map; no wire fields and no allocation/item charge',
        native_metadata_callback='ReadMetaData receives the constructed map in both versions',
        compatibility_validation_implemented=False,
        defaults_limit='Missing-key sentinel global initialization is not proven by this audit; no native compatibility result is fabricated',
        raw_input=dict(sha256=RAW_SHA, size=len(raw), context_version=tags[0],
                       index_version=tags[1], game_version=tags[2], offsets=[0, 4, 8]),
        context_only_expected_refusal=dict(section='Game', expected=22, actual=19, boundary='beforeMap'),
        original_execution=False, native_oracle_available=False,
        complete_raw_load=False, gameplay_state=False, actions=False, turns=False,
        unknown_save_bytes_masked=False, recorded_raw_save_load_difference_bytes=89198,
    )
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, raw_header=tags, complete_raw_load=False)))


if __name__ == '__main__':
    main()
