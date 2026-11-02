#!/usr/bin/env python3
"""Audit bounded read-only action observations in the exact original Linux ELFs.

Does not run the fixture or establish complete state/action/turn parity.
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

GAME_METHODS = [
    (0x2660650, 8, 'Game.GetInstance'),
    (0x1FCC320, 11, 'Context.Globals.GetGame'),
    (0x2D7B4D0, 8, 'Game.GetSyncRandom.const'),
    (0x20E0710, 8, 'Unit.GetOperationQueue'),
    (0x2702F00, 5, 'Queue.GetSize'),
    (0x2702F10, 7, 'NodeVector.size'),
    (0x2DA1050, 0x26, 'Queue.GetAt'),
    (0x2DA11E0, 0x1A, 'NodeVector.const_index'),
    (0x265B300, 7, 'Unit.GetActivityType'),
    (0x2C2C4B0, 7, 'Unit.GetAutomateType'),
    (0x2C2C2C0, 7, 'Unit.GetOperationTimer'),
    (0x213CC90, 0x15, 'Unit.GetMovesRemaining.hidden_sret'),
    (0x1FCC8C0, 5, 'FixedPoint.copy_constructor'),
    (0x1FCDC00, 8, 'FixedPoint.copy_assignment'),
    (0x26B0D10, 5, 'Unit.GetAutoArchive'),
]
HOST_METHODS = [
    (0x3C46FCA, 9, 'Archive.hasDeltas'),
    (0x3C45F34, 0xD9, 'Archive.getDeltaIndexes.hidden_sret'),
    (0x327EE90, 4, 'IndexVector.size'),
    (0x327EEA0, 15, 'IndexVector.const_index'),
    (0x2FDD980, 4, 'IndexPointer.const_get'),
    (0x3283350, 0x2C, 'IndexVector.D2'),
    (0x371AA20, 0x46, 'IndexVector.push_back'),
    (0x371DFB0, 0xEC, 'IndexVector.RequestAndAppendLinearBlock'),
    (0x32835D0, 1, 'IndexVector.Destroy.noop'),
    (0x32835E0, 0x12, 'IndexVector.FreeLinearBlock'),
    (0x32836C0, 0x74, 'IndexVector.AllocLinearBlock'),
    (0x2F521B0, 5, 'IndexVector.StaticFree'),
    (0x2F52DE0, 0x1A, 'IndexVector.StaticAlloc'),
]


def disassembly(path, address, size):
    return subprocess.check_output(['objdump', '-d', '--no-show-raw-insn',
                                   f'--start-address={address}',
                                   f'--stop-address={address + size}', str(path)], text=True)


def memory_stores(text):
    result = []
    for line in text.splitlines():
        match = re.match(r'\s*([0-9a-f]+):\s+(\S+)\s+(.*)', line)
        if not match:
            continue
        address, op, operands = match.groups()
        # The audited routine's only nonstack writes use MOV/MOVUPS. Reject
        # other write-capable families instead of treating them as reads.
        if op.startswith(('stos', 'xchg', 'cmpxchg', 'inc', 'dec')):
            raise ValueError('unexpected write-capable instruction')
        if op.startswith(('mov', 'add', 'sub', 'and', 'or', 'xor')):
            destination = operands.split(',')[-1].strip()
            if '(' in destination:
                result.append((int(address, 16), op, destination))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    game_path, host_path = args.assets / 'libGameCore_XP2.so', args.assets / 'Civ6'
    game, host = Elf(game_path, GAME_SHA), Elf(host_path, HOST_SHA)
    symbols = {}
    for path in [game_path, host_path]:
        entries = {}
        for line in subprocess.check_output(['nm', '-DS', str(path)], text=True).splitlines():
            fields = line.split()
            if len(fields) == 4:
                entries[fields[3]] = (int(fields[0], 16), int(fields[1], 16))
        symbols[path.name] = entries
    expected_symbols = [
        ('libGameCore_XP2.so', '_ZN8GameCore4Game11GetInstanceEv', 0x2660650),
        ('libGameCore_XP2.so', '_ZNK8GameCore4Game8Instance13GetSyncRandomEv', 0x2D7B4D0),
        ('libGameCore_XP2.so', '_ZN8GameCore3Map9Generator10ms_kRandomE', 0x3C072F0),
        ('libGameCore_XP2.so', '_ZNK8GameCore4Unit8Instance17GetMovesRemainingEv', 0x213CC90),
        ('Civ6', '_ZNK12FAutoArchive15getDeltaIndexesEv', 0x3C45F34),
        ('Civ6', '_ZN5Types6VectorIjNS_11LinearAlloc7DefaultILj1ELj0EEEED2Ev', 0x3283350),
    ]
    for image, symbol, address in expected_symbols:
        if symbols[image].get(symbol, (None,))[0] != address:
            raise ValueError('symbol mismatch: ' + symbol)
    if symbols['libGameCore_XP2.so']['_ZN8GameCore3Map9Generator10ms_kRandomE'][1] != 32:
        raise ValueError('map RNG complete object size mismatch')
    contracts = [
        (game, 0x2D7B4D0, '488d87f8000000c3', 'const_sync_accessor_is_address_only'),
        (game, 0x265B300, '8b8790020000c3', 'activity_load_only'),
        (game, 0x2C2C4B0, '8b8778020000c3', 'automation_load_only'),
        (game, 0x2C2C2C0, '8b8728050000c3', 'timer_load_only'),
        (game, 0x26B0D10, '488d4708c3', 'archive_address_only'),
        (game, 0x1FCDC00, '8b0689074889f8c3', 'fixedpoint_copy_writes_output_only'),
        (host, 0x3C46FCA, '48837f48000f95c0c3', 'hasDeltas_reads_size_only'),
        (host, 0x3C45F41, '4989f54989fe', 'archive_RSI_output_RDI'),
        (host, 0x327EE90, '8b4710c3', 'index_vector_size_load_only'),
        (host, 0x2FDD980, '488b07c3', 'index_pointer_load_only'),
        (host, 0x32835D0, 'c3', 'unsigned_Destroy_noop'),
    ]
    for elf, address, expected, name in contracts:
        if elf.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('contract mismatch: ' + name)
    stores = memory_stores(disassembly(host_path, 0x3C45F34, 0xD9))
    expected_stores = [(0x3C45F4A, 'movups', '(%r14)'),
                       (0x3C45F4E, 'movq', '0x10(%r14)'),
                       (0x3C45F9F, 'mov', '0xc(%rsp)')]
    if stores != expected_stores:
        raise ValueError('getDeltaIndexes write destinations changed: ' + repr(stores))
    calls = [(host, 0x3C45FA9, 0x371AA20), (host, 0x3C46000, 0x3283350),
             (host, 0x3C46008, 0x2C62FE0), (host, 0x327EEA4, 0x2FDD980),
             (game, 0x213CC9B, 0x1E43290), (game, 0x2DA1058, 0x1E6ECF0),
             (game, 0x2DA1066, 0x1F535D0)]
    for elf, address, target in calls:
        elf.call(address, target)
    def methods(elf, entries):
        return [dict(address=hex(a), size=z, name=n,
                     sha256=hashlib.sha256(elf.read(a, z)).hexdigest()) for a, z, n in entries]
    result = dict(
        schema='c6-linux-action-state-abi-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        native_execution=False, action_executed=False, complete_affected_state=False,
        game_methods=methods(game, GAME_METHODS), host_methods=methods(host, HOST_METHODS),
        symbol_contracts=[dict(image=i, symbol=s, address=hex(a)) for i, s, a in expected_symbols],
        instruction_contracts=[name for _, _, _, name in contracts],
        delta_index_write_destinations=[dict(address=hex(a), instruction=o, destination=d) for a, o, d in stores],
        observer_abi=dict(moves=dict(output_bytes=4, hidden_output='RDI', receiver='RSI'),
                          delta_indexes=dict(output_bytes=24, alignment=8, hidden_output='RDI', receiver='RSI',
                                             destructor='Original nondeleting D2', order='Original dirty pointer tree order, no sorting')),
        purity=dict(
            archive='getDeltaIndexes reads the genuine archive and dirty tree; direct writes only initialize returned vector and local index; sole normal callee pushes into returned vector.',
            allocation='Original push/append paths write returned vector and its newly allocated index storage; original D2 frees that storage. Allocator bookkeeping and stderr I/O are observable host effects, not claimed bitwise process purity.',
            gameplay='No archive edit/reset, RNG draws, request, update, drain, policy, or per-variable virtual hash callback is called by observer.',
            runtime='Caller must hash/configuration gate and synchronize worker. Runtime before/after original Save, transient state and RNG checks remain required; static analysis does not replace them.'),
        random=dict(streams=['Game.GetInstance -> const Game.GetSyncRandom', 'Map.Generator.ms_kRandom'],
                    complete_object_bytes=32, offsets=dict(seed=8, calls=12, resets=16, lock=20, raw_sync=24),
                    preservation='Records all object bytes including vptr/padding; compares unchanged bytes across each capture; never normalizes pointers or RNG.',
                    coverage='Two identified streams only; complete initialized-game RNG ownership remains unresolved.'),
        queue=dict(node_bytes=40, order='Original GetAt indices',
                   pointers='raw40 retains all pointer bytes as opaque observations; parameter ownership and pointed-to content semantics remain unresolved.'),
        source_sha256={str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in
                       [Path('native/linux/action_state.c'), Path('native/linux/action_state.h')]},
        private_evidence='../c6-ghidra/evidence/linux-action-state/',
        scope='Narrow genuine-unit and two-stream observer. Isolated primitive support does not establish exact headless Civ VI state/action/turn parity.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, native_execution=False, action_executed=False)))


if __name__ == '__main__':
    main()
