#!/usr/bin/env python3
"""Reproduce localization source/configuration refusals before original execution."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from localization_gate import ROOT, digest, prepare


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assets = args.assets.resolve()
    gate = json.loads((ROOT / 'tools/linux/localization-sources.json').read_text())
    checks = {}
    with tempfile.TemporaryDirectory(prefix='c6-localization-boundaries-') as directory:
        root = Path(directory)
        fixture = root / 'assets'
        fixture.mkdir()
        (fixture / 'Civ6').symlink_to(assets / 'Civ6')
        for name in gate['files']:
            target = fixture / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.symlink_to(assets / name)
        prepare(fixture, root / 'valid')
        checks['unchanged_sources_accepted'] = True

        def rejects(label, target, inventory=False):
            if not inventory:
                target.unlink()
            target.write_bytes(b'c6 deliberate invalid source fixture\n')
            try:
                prepare(fixture, root / label)
            except ValueError as error:
                checks[label] = str(error)
            else:
                raise RuntimeError(f'gate unexpectedly accepted {label}')
            target.unlink()
            if not inventory:
                target.symlink_to(assets / target.relative_to(fixture))

        rejects('icu_sha256_mismatch_rejected', fixture / 'Base/assets/text/icudt56l.dat')
        english = next(name for name in gate['files'] if '/en_us/' in name)
        rejects('english_sha256_mismatch_rejected', fixture / english)
        rejects('english_extra_source_rejected', fixture / 'Base/assets/text/en_us/c6-extra.xml', True)
        for label, relative in [('optional_ui_text_rejected', 'Base/Assets/UI/Text'),
                                ('existing_cache_rejected', 'launch/a/b/c6-no-localization-cache.sqlite')]:
            target = root / 'valid' / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(b'c6 invalid optional input\n')
            try:
                prepare(fixture, root / 'valid')
            except ValueError as error:
                checks[label] = str(error)
            else:
                raise RuntimeError(f'gate unexpectedly accepted {label}')
            target.unlink()
    # Runtime checks configuration identity after compilation but before original
    # executable execution. The invalid identity must refuse the child entirely.
    run = subprocess.run([sys.executable, str(ROOT / 'tools/linux/runtime_probe.py'),
                          '--assets', str(assets), '--localization',
                          '--expected-configuration', '0' * 64,
                          '--output', str(ROOT / 'target/linux-localization/refused.json')],
                         capture_output=True, text=True)
    checks['configuration_mismatch_refused_before_execution'] = (
        run.returncode != 0 and
        'configuration identity mismatch; original execution refused' in run.stderr)
    if not checks['configuration_mismatch_refused_before_execution']:
        raise RuntimeError('runtime configuration identity refusal failed')
    result = {'schema': 'c6-linux-localization-boundaries-v1', 'checks': checks,
              'all_passed': True, 'original_code_executed': False,
              'gate_sha256': digest(ROOT / 'tools/linux/localization-sources.json'),
              'gate_adapter_sha256': digest(ROOT / 'tools/linux/localization_gate.py'),
              'checker_sha256': digest(Path(__file__).resolve()),
              'limits': 'These are pre-execution refusals, not localization behavior parity.'}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'all_passed': True, 'output': str(args.output)}))


if __name__ == '__main__':
    main()
