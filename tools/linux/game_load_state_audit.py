#!/usr/bin/env python3
"""Audit exact XP2 read-only state facade entries; no gameplay code is executed."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from inspect_elf import sections

ROOT = Path(__file__).resolve().parents[2]
XP2 = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
DWARF = '55b7c3d6e9fb5eb0729430d5290c44fe427a7a763927e9798e5e94afd0974fbd'
METHODS = [
    ('context_get_game', '_ZN8GameCore7Exports7Context7GetGameEv', 0x2c7cae0, 0x5c, 'IGame1*(Context*)'),
    ('release_wrapper', '_ZN8GameCore10Interfaces10ICvUnknowndlEPv', 0x290bba0, 0x16, 'void(void*)'),
    ('context_local_player', '_ZNK8GameCore7Exports7Context14GetLocalPlayerEv', 0x2c7ca80, 0x2e, 'PlayerTypes(Context const*)'),
    ('pending_messages', '_ZN8GameCore7Exports7Context18HasPendingMessagesEv', 0x2c7d340, 5, 'bool(Context*)'),
    ('current_turn', '_ZNK8GameCore7Exports4Game14GetCurrentTurnEv', 0x2c868b0, 0x1a, 'int(Game const*)'),
    ('current_phase', '_ZNK8GameCore7Exports4Game15GetCurrentPhaseEv', 0x2c86850, 0x11, 'int(Game const*)'),
    ('game_local_player', '_ZNK8GameCore7Exports4Game14GetLocalPlayerEv', 0x2c86380, 0x1a, 'PlayerTypes(Game const*)'),
    ('needs_update', '_ZNK8GameCore7Exports4Game11NeedsUpdateEv', 0x2c86a60, 5, 'bool(Game const*)'),
    ('needs_turn_advance', '_ZNK8GameCore7Exports4Game16NeedsTurnAdvanceEv', 0x2c869f0, 5, 'bool(Game const*)'),
    ('ai_processing', '_ZNK8GameCore7Exports4Game14IsAiProcessingEv', 0x2c867e0, 0x11, 'bool(Game const*)'),
    ('turn_processing', '_ZNK8GameCore7Exports4Game16IsTurnProcessingEv', 0x2c86800, 0x11, 'bool(Game const*)'),
]


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def audit(module, dwarf):
    if digest(module) != XP2:
        raise ValueError('unsupported original XP2 module SHA-256')
    if digest(dwarf) != DWARF:
        raise ValueError('matching native facade DWARF excerpt SHA-256 differs')
    symbols = {}
    for line in subprocess.check_output(['nm', '-D', '-S', str(module)], text=True).splitlines():
        columns = line.split()
        if len(columns) == 4:
            symbols[columns[3]] = (int(columns[0], 16), int(columns[1], 16))
    text = sections(module)['.text']
    results = []
    with module.open('rb') as source:
        for label, name, address, size, abi in METHODS:
            if symbols.get(name) != (address, size):
                raise ValueError('original facade method address/size differs: ' + label)
            source.seek(address - int(text['address'], 16) + text['offset'])
            code = source.read(size)
            if len(code) != size:
                raise ValueError('incomplete original method: ' + label)
            results.append(dict(name=label, symbol=name, address=hex(address), bytes=size,
                                sha256=hashlib.sha256(code).hexdigest(), abi=abi))
    return dict(schema='c6-linux-loaded-state-facade-audit-v1', module_sha256=XP2,
                matching_dwarf_excerpt_sha256=DWARF, methods=results,
                ownership='Context GetGame returns native IGame1 wrapper after original game-initialized check; release through original ICvUnknown::operator delete dispatches native Destroy. Never delete framework Context.',
                native_execution=False, gameplay_parity=False,
                limits='Static facade ABI audit only. Observe actual loaded state and compare Save before/after separately; these fields do not prove full-state completeness or action readiness.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--dwarf', type=Path, default=ROOT.parent/'c6-ghidra/evidence/linux-action-boundary/abi.dwarf')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.module, args.dwarf)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'output': str(args.output), 'methods_checked': len(result['methods']), 'native_execution': False}))


if __name__ == '__main__':
    main()
