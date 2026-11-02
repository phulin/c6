#!/usr/bin/env python3
"""Independent private-native rules parity and rejection review; no gameplay claim."""
import argparse
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/assets'))
from rules_snapshot import fingerprint  # noqa: E402

HARNESS = r'''
use c6_data::{Rules, SourceIdentity, xp2_source_identity};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let identity: SourceIdentity = if args.len() > 2 {
        serde_json::from_str(&std::fs::read_to_string(&args[2])?)?
    } else { xp2_source_identity() };
    let rules = Rules::load(&args[1], &identity)?;
    println!("{}", serde_json::json!({"source":rules.source(), "databases":[
        rules.configuration().fingerprint(), rules.gameplay().fingerprint()]}));
    Ok(())
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exports', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/rules')
    args = parser.parse_args()
    manifest = json.loads((ROOT / 'evidence/assets/xp2-rules-snapshot.json').read_text())
    with tempfile.TemporaryDirectory(prefix='c6-rules-review-') as temporary:
        directory = Path(temporary)
        (directory / 'src').mkdir()
        (directory / 'src/main.rs').write_text(HARNESS)
        (directory / 'Cargo.toml').write_text(
            '[package]\nname="c6-rules-review"\nversion="0.0.0"\nedition="2021"\n'
            '[dependencies]\nc6-data={path=' + json.dumps(str(ROOT / 'crates/c6-data')) + '}\nserde_json="1"\n')
        target = ROOT / 'target/rules-review'
        subprocess.run(['cargo', 'build', '--offline', '--manifest-path', str(directory / 'Cargo.toml'),
                        '--target-dir', str(target)], check=True, capture_output=True, text=True, timeout=180)
        binary = target / 'debug/c6-rules-review'

        def run(path, source=None, expected=None):
            command = [str(binary), str(path)]
            if source is not None:
                source_file = directory / 'source.json'
                source_file.write_text(json.dumps(source))
                command.append(str(source_file))
            result = subprocess.run(command, capture_output=True, text=True, timeout=60)
            if expected is not None:
                assert result.returncode != 0 and expected in result.stderr, result.stderr
                return None
            assert result.returncode == 0, result.stderr
            return json.loads(result.stdout)

        baseline = None
        for name in ('run-1', 'run-2'):
            path = args.exports / name
            report = run(path)
            for actual, pinned in zip(report['databases'], manifest['databases'], strict=True):
                independent = fingerprint(path / pinned['filename'])
                expected = {key: value for key, value in pinned.items() if key != 'database'}
                assert actual == independent == expected, f'{name} {pinned["filename"]}: Rust/Python/native manifest mismatch'
            if baseline is None:
                baseline = report
            else:
                assert report == baseline, 'independent native exports differ'
        print('native run-1/run-2: Rust, live Python and pinned manifest match all 506 tables / 43,570 rows')
        source = baseline['source']
        for key in source:
            changed = dict(source)
            changed[key] = 'wrong' if isinstance(source[key], str) else {'incorrect': True}
            run(directory / 'absent', changed, 'source/profile differs')
        print(f'source gate: all {len(source)} declared identity fields reject before file I/O')
        copy = directory / 'modified'
        copy.mkdir()

        def reset():
            for name in ('configuration.sqlite', 'gameplay.sqlite'):
                shutil.copyfile(args.exports / 'run-1' / name, copy / name)

        reset()
        with (copy / 'gameplay.sqlite').open('ab') as output:
            output.write(b'file-identity-only-tamper')
        assert fingerprint(copy / 'gameplay.sqlite')['semantic_sha256'] == manifest['databases'][1]['semantic_sha256']
        run(copy, expected='complete schema/file/semantic identity mismatch')
        reset()
        path = copy / 'gameplay.sqlite'
        with path.open('r+b') as output:
            output.truncate(path.stat().st_size // 2)
        run(copy, expected='database disk image is malformed')
        reset()
        with sqlite3.connect(copy / 'gameplay.sqlite') as connection:
            connection.execute("UPDATE Units SET Cost=X'3130' WHERE UnitType='UNIT_WARRIOR'")
        run(copy, expected='table Units: schema/row identity or typed rows mismatch')
        reset()
        with sqlite3.connect(copy / 'gameplay.sqlite') as connection:
            connection.execute("UPDATE Units SET rowid=rowid+100000 WHERE UnitType='UNIT_WARRIOR'")
        run(copy, expected='table Units: schema/row identity or typed rows mismatch')
        reset()
        with sqlite3.connect(copy / 'gameplay.sqlite') as connection:
            connection.execute('ALTER TABLE Units ADD COLUMN ReviewExtra BLOB')
        run(copy, expected='table Units: column metadata digest mismatch')
        reset()
        with sqlite3.connect(copy / 'gameplay.sqlite') as connection:
            connection.execute('DROP TABLE Units')
        run(copy, expected='table Units: missing required table')
        print('actual native-copy rejection: physical-only tamper, truncation, SQL storage type, intrinsic rowid, column schema and missing table')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
