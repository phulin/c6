#!/usr/bin/env python3
"""Reproduce bounded original LocalizationDLL English dictionary initialization."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def win(path):
    return 'Z:' + str(path.resolve()).replace('/', '\\')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets')
    parser.add_argument('--output', type=Path, default=ROOT / 'evidence/bootstrap/localization.json')
    parser.add_argument('--expected-configuration', help='refuse execution if complete probe configuration differs')
    args = parser.parse_args()
    text = args.assets / 'Base/Assets/Text'
    fixture = json.loads((ROOT / 'tools/analysis/localization_sources.json').read_text())
    sources = [args.assets / item['path'] for item in fixture['sources']]
    for item, source in zip(fixture['sources'], sources):
        if sha(source) != item['sha256']:
            raise SystemExit('localization source SHA256 mismatch: ' + item['path'])
    manifest = ROOT / 'target/bootstrap/localization-paths.tsv'
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(''.join(f'{p.suffix[1:]}\t{sha(p)}\t{win(p)}\n' for p in sources))
    subprocess.run(['make', '-C', str(ROOT / 'native/bootstrap/localization')], check=True)
    binary = ROOT / 'target/bootstrap/localization_probe.exe'
    env = dict(os.environ, WINEPREFIX=str(ROOT / 'target/bootstrap/wine-prefix'), WINEARCH='win64', WINEDEBUG='-all', WINEDLLOVERRIDES='msvcp140,vcruntime140,ucrtbase=n')
    deps = args.assets / 'Base/Binaries/Win64Steam'
    command = ['/usr/lib/wine/wine64', str(binary), win(deps / 'CivilizationVI.exe'), win(deps), win(manifest), win(text / 'icudt56l.dat')]
    configuration = {'language': 'en_US', 'selection': 'explicit bounded Base English source fixture; not DLC/mod resolver',
                     'sources_manifest_sha256': sha(ROOT / 'tools/analysis/localization_sources.json'),
                     'source_count': len(sources),
                     'native_sources': {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / 'native/bootstrap/localization').glob('*'))},
                     'xml_import_sources': {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / 'native/bootstrap/xml-import').glob('*.[ch]')) if p.stem in ('xml', 'imports')},
                     'driver_sha256': sha(Path(__file__)),
                     'original_executable_sha256': sha(deps / 'CivilizationVI.exe'),
                     'database_dll_sha256': sha(deps / 'DatabaseDLL_FinalRelease.dll'),
                     'wine_version': subprocess.check_output(['/usr/lib/wine/wine64', '--version'], text=True).strip(),
                     'wine_dll_overrides': env['WINEDLLOVERRIDES'],
                     'icu_sha256': sha(text / 'icudt56l.dat'), 'host_sha256': sha(binary),
                     'dll_sha256': sha(deps / 'Localization_FinalRelease.dll')}
    identity = hashlib.sha256(json.dumps(configuration, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    if args.expected_configuration and args.expected_configuration != identity:
        raise SystemExit('localization configuration SHA256 mismatch: refusing execution')
    result = subprocess.run(command, capture_output=True, text=True, env=env, timeout=120)
    output = {'format': 'c6.bootstrap-localization.v1', 'configuration': configuration,
              'configuration_sha256': identity,
              'returncode': result.returncode, 'observations': [json.loads(l) for l in result.stdout.splitlines() if l.startswith('{')],
              'stderr': result.stderr, 'initialized_game': False}
    args.output.write_text(json.dumps(output, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'returncode': result.returncode, 'observations': output['observations'], 'stderr': result.stderr}))

if __name__ == '__main__':
    main()
