#!/usr/bin/env python3
"""Read-only target/source gates for the bounded owned diplomacy pending drain."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    source_path = ROOT / 'evidence/linux/diplomacy-manager-source.json'
    source = json.loads(source_path.read_text())
    if source['gamecore_sha256'] != GAME_SHA:
        raise ValueError('source target differs')
    elf = Elf(args.module, GAME_SHA)
    for method in source['methods']:
        if sha(elf.read(int(method['rva'], 16), method['bytes'])) != method['sha256']:
            raise ValueError('complete target method differs: ' + method['name'])
    paths = ['crates/c6-core/src/diplomacy_manager.rs',
             'crates/c6-core/src/loaded_state.rs',
             'crates/c6-core/src/unit_operation_aftermath.rs',
             'docs/linux-diplomacy-manager-state.md',
             'tools/linux/diplomacy_manager_state_audit.py']
    report = dict(schema='c6-linux-diplomacy-manager-state-v1',
                  gamecore_sha256=GAME_SHA, source_contract_sha256=sha(source_path.read_bytes()),
                  target_methods_verified=len(source['methods']),
                  sources={path: sha((ROOT / path).read_bytes()) for path in paths},
                  native_execution=False, actual_pending_snapshot_admitted=False,
                  tests_scope='source-only state/admission/reentrant drain',
                  current_capture_requirement='same dynamic context, actual empty open-list witness, ordered ever-alive handles, complete pending actions',
                  unsupported=['complete Statement routing/history/subtypes and external decisions',
                               'closed session history and active session routing',
                               'native full action/Read/RNG equivalence'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'target_methods_verified': len(source['methods']), 'native_execution': False}))


if __name__ == '__main__':
    main()
