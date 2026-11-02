#!/usr/bin/env python3
"""Manifest private static policy evidence without publishing proprietary dumps."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    root = Path(__file__).resolve().parents[3]
    parser.add_argument('--assets', type=Path, default=root / 'c6-assets-linux')
    parser.add_argument('--private-evidence', type=Path,
                        default=root / 'c6-ghidra/evidence/linux-policy-boundary')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    required = ['QueueNextTech.dwarf', 'SetPaused.dwarf', 'IsTurnProcessing.dwarf',
                'offturn.dwarf.json', 'AI_Research_QueueNextTech.asm',
                'AI_Control_Manager_DoTurnActivate.asm', 'AI_Control_Manager_InitializePostLoad.asm']
    if not all((args.private_evidence / name).is_file() for name in required):
        raise ValueError('required private source missing')
    rows = []
    for path in sorted(args.private_evidence.iterdir()):
        if not path.is_file() or path.suffix not in ('.asm', '.dwarf', '.json', '.txt'):
            continue
        raw = path.read_bytes()
        rows.append(dict(file=path.name, bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest()))
    report = dict(schema=1, gamecore_sha256=GAME_SHA,
                  method='Hashes of private nm/objdump/llvm-dwarfdump output; no game execution',
                  private_directory='../c6-ghidra/evidence/linux-policy-boundary',
                  files=rows, limits='Source hashes pin observations; they do not assert runtime safety or complete decision coverage.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(files=len(rows))))


if __name__ == '__main__':
    main()
