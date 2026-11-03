#!/usr/bin/env python3
"""Audit Unit prefix APIs from an explicit committed archive; never run the game.

Native traces supply independent expected boundaries only. Production parsing gets
an entire Core tail and a structural starting cursor, never a traced end offset.
"""
import argparse
import gzip
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[2]
ADAPTER = r'''
use c6_core::unit::{Error, Limits, Unit};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let core = std::fs::read(&args[1]).unwrap();
    let bounds = std::fs::read_to_string(&args[2]).unwrap();
    let mut total_items = 0usize;
    let mut records = Vec::new();
    for line in bounds.lines() {
        let nums: Vec<usize> = line.split_whitespace().map(|v| v.parse().unwrap()).collect();
        let (start,end) = (nums[0],nums[1]);
        let bytes = &core[start..end];
        let unit = Unit::decode(bytes, Limits::default()).unwrap();
        let (wire,items) = unit.encode_with_items(Limits::default()).unwrap();
        assert_eq!(wire, bytes);
        let exact = Limits { max_bytes: bytes.len(), max_items: items };
        // This production input includes the untouched remainder of Core.
        let (prefix,consumed,charged) = Unit::decode_prefix_with_items(&core[start..], exact).unwrap();
        assert_eq!((consumed,charged), (bytes.len(),items));
        assert_eq!(prefix, unit);
        assert_eq!(Unit::decode_prefix(&core[start..], exact).unwrap(), (unit.clone(),bytes.len()));
        assert_eq!(prefix.encode_with_items(exact).unwrap(), (wire,items));
        assert_eq!(Unit::decode_with_ranges(bytes,exact).unwrap().0, unit);
        assert_eq!(Unit::decode(&core[start..],exact), Err(Error::ResourceLimit));
        let mut trailing = bytes.to_vec(); trailing.push(255);
        let loose = Limits { max_bytes: trailing.len(), ..exact };
        assert_eq!(Unit::decode(&trailing,loose), Err(Error::TrailingBytes));
        assert_eq!(Unit::decode_with_ranges(&trailing,loose), Err(Error::TrailingBytes));
        assert_eq!(Unit::decode_prefix_with_items(&trailing,loose).unwrap(),(unit.clone(),bytes.len(),items));
        assert_eq!(Unit::decode_prefix(&core[start..], Limits { max_bytes: bytes.len()-1, ..exact }),Err(Error::ResourceLimit));
        assert_eq!(unit.encode_with_items(Limits { max_bytes: bytes.len()-1, ..exact }), Err(Error::ResourceLimit));
        if items > 0 {
            let short = Limits { max_items: items-1, ..exact };
            assert_eq!(Unit::decode_prefix(&core[start..],short),Err(Error::ResourceLimit));
            assert_eq!(unit.encode_with_items(short),Err(Error::ResourceLimit));
        }
        if records.is_empty() {
            for n in 0..bytes.len() {
                assert!(Unit::decode_prefix(&bytes[..n],Limits::default()).is_err(),"truncated {n}");
                assert_eq!(Unit::decode_prefix(&core[start..],Limits { max_bytes:n, ..exact }),Err(Error::ResourceLimit),"clipped {n}");
            }
        }
        total_items += items;
        records.push((bytes.to_vec(),items));
    }
    // Carry the exact shared charge across every record, independent of native
    // field ranges. The input contains multiple records and a final unused tail.
    let mut joined: Vec<u8> = records.iter().flat_map(|(b,_)|b.iter().copied()).collect();
    joined.extend_from_slice(&[255; 17]);
    let mut pos = 0usize;
    let mut remaining = total_items;
    for (wire,items) in &records {
        let (_,consumed,charged) = Unit::decode_prefix_with_items(&joined[pos..],Limits { max_bytes: joined.len()-pos,max_items:remaining }).unwrap();
        assert_eq!((consumed,charged),(wire.len(),*items));
        pos += consumed; remaining -= charged;
    }
    assert_eq!(remaining,0); assert_eq!(joined.len()-pos,17);
    let mut short_remaining = total_items-1;
    let mut pos = 0usize;
    let mut failed = false;
    for (wire,items) in &records {
        match Unit::decode_prefix_with_items(&joined[pos..],Limits { max_bytes:joined.len()-pos,max_items:short_remaining }) {
            Ok((_,n,charged)) => { assert_eq!((n,charged),(wire.len(),*items)); pos+=n; short_remaining-=charged; }
            Err(Error::ResourceLimit) => { failed=true; break; }
            other => panic!("unexpected cumulative result {other:?}"),
        }
    }
    assert!(failed);
    println!("{{\"records\":{},\"bytes\":{},\"items\":{},\"first_record_all_clipped_and_truncated_prefixes\":{},\"aggregate_exact_and_one_short\":true}}", records.len(),pos_for_bytes(&records),total_items,records[0].0.len());
}
fn pos_for_bytes(records:&[(Vec<u8>,usize)])->usize { records.iter().map(|(b,_)|b.len()).sum() }
'''


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--private-root', type=Path, default=ROOT.parent / 'c6-ghidra')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    commit = subprocess.check_output(['git', 'rev-parse', args.source_commit + '^{commit}'], cwd=ROOT, text=True).strip()
    with tempfile.TemporaryDirectory(prefix='c6-unit-prefix-') as tmp:
        temp = Path(tmp)
        archive = temp / 'source'
        archive.mkdir()
        packed = subprocess.check_output(['git', 'archive', commit], cwd=ROOT)
        with tarfile.open(fileobj=io.BytesIO(packed)) as tar:
            tar.extractall(archive, filter='data')
        own = 'tools/linux/unit_prefix_audit.py'
        if (archive / own).read_bytes() != Path(__file__).read_bytes():
            raise ValueError('audit script must match the explicit source commit')
        spec = importlib.util.spec_from_file_location('unit_check', archive / 'tools/linux/unit_codec_check.py')
        checker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(checker)
        manifest = json.loads((args.private_root / 'evidence/linux-game-load/canonical/manifest.json').read_text())
        if manifest['module_sha256'] != checker.MODULE:
            raise ValueError('unsupported original module identity')
        adapter = temp / 'adapter'
        (adapter / 'src').mkdir(parents=True)
        (adapter / 'src/main.rs').write_text(ADAPTER)
        (adapter / 'Cargo.toml').write_text('[package]\nname="unit-prefix-audit"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nc6-core={path=' + json.dumps(str(archive / 'crates/c6-core')) + '}\n')
        subprocess.run(['cargo', 'build', '--release', '--manifest-path', str(adapter / 'Cargo.toml')], check=True, cwd=archive)
        binary = adapter / 'target/release/unit-prefix-audit'
        fixtures = []
        for identity, (name, trace_sha) in zip(checker.SAVES, checker.TRACES):
            data = gzip.decompress((args.private_root / f'evidence/linux-game-load/canonical/{identity}.bin.gz').read_bytes())
            trace = gzip.decompress((args.private_root / 'evidence/linux-save-normalization/checkpoint-203-owners' / name).read_bytes())
            if len(data) != 80657462 or sha(data) != identity or sha(trace) != trace_sha:
                raise ValueError('unsupported immutable native fixture')
            rows = sorted((r for line in trace.splitlines() if (r := json.loads(line)).get('owner') == 'GameCore::Unit::Instance' and r['save'] == 1), key=lambda r: r['start'])
            if len(rows) != 1262 or any(not 0 <= r['start'] < r['end'] < len(data) for r in rows):
                raise ValueError('invalid expected native Unit boundaries')
            (temp / 'core.bin').write_bytes(data)
            (temp / 'bounds.txt').write_text(''.join(f"{r['start']} {r['end']}\n" for r in rows))
            result = json.loads(subprocess.check_output([str(binary), str(temp / 'core.bin'), str(temp / 'bounds.txt')], text=True))
            result.update(save_sha256=identity, native_trace_sha256=trace_sha)
            fixtures.append(result)
        source_paths = sorted([p for p in archive.rglob('*') if p.is_file() and (p.is_relative_to(archive / 'crates/c6-core') and p.suffix in ('.rs', '.toml') or p.name in ('Cargo.toml', 'Cargo.lock') or p.relative_to(archive).as_posix() in (own, 'tools/linux/unit_codec_check.py'))])
        sources = {p.relative_to(archive).as_posix(): sha(p.read_bytes()) for p in source_paths}
        subprocess.run(['cargo', 'test', '-p', 'c6-core', 'unit::'], cwd=archive, check=True)
        strict_output = temp / 'strict.json'
        subprocess.run(['python3', str(archive / 'tools/linux/unit_codec_check.py'), '--private-root', str(args.private_root.resolve()), '--iterations', '1', '--output', str(strict_output)], cwd=archive, check=True, stdout=subprocess.DEVNULL)
        strict = json.loads(strict_output.read_text())
        report = dict(schema='c6-linux-unit-prefix-v1', source_commit=commit, module_sha256=checker.MODULE, source_sha256=sources,
                      adapter_source_sha256=sha(ADAPTER.encode()), adapter_sha256=sha(binary.read_bytes()), fixtures=fixtures,
                      strict_codec_check= strict, scope='Current Unit v28 wire prefix and exact aggregate dynamic-item charges; traces are independent expected bounds only. No original game execution, live-object fixups, domain, dirty history, gameplay, actions or turns. Raw Save/reload/Save failure remains 89198 unmasked bytes.')
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(dict(source_commit=commit, fixtures=fixtures)))


if __name__ == '__main__':
    main()
