#!/usr/bin/env python3
"""Decode pinned native Unit slices and compare Unit hashes to a private registry.

Only e_type_index belongs to Units. Other Unit-state definition hashes require
other original registries and are deliberately not resolved against Units.
"""
import argparse
import gzip
import json
from pathlib import Path
import subprocess
import tempfile
from unit_codec_check import MODULE, SAVES, TRACES, sha

ROOT = Path(__file__).resolve().parents[2]
ADAPTER = r'''
use c6_core::unit::{Limits,Unit};
use std::collections::BTreeMap;
fn main()->Result<(),Box<dyn std::error::Error>> {
 let mut counts=BTreeMap::<u32,usize>::new();
 for p in std::fs::read_dir(std::env::args().nth(1).ok_or("missing path")?)? {
  let bytes=std::fs::read(p?.path())?;
  let unit=Unit::decode(&bytes,Limits::default())?;
  if unit.encode(Limits::default())?!=bytes {return Err("roundtrip mismatch".into())}
  *counts.entry(unit.e_type_index).or_default()+=1;
 }
 println!("{{{}}}",counts.iter().map(|(k,v)|format!("\"{}\":{}",k,v)).collect::<Vec<_>>().join(","));
 Ok(())
}
'''


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--private-root', type=Path, default=ROOT.parent/'c6-ghidra')
    p.add_argument('--registry-report', type=Path)
    p.add_argument('--registry-runtime-report', type=Path)
    p.add_argument('--registry-before-save', type=Path)
    p.add_argument('--registry-after-save', type=Path)
    p.add_argument('--private-output', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    manifest = json.loads((args.private_root/'evidence/linux-game-load/canonical/manifest.json').read_text())
    if manifest['module_sha256'] != MODULE:
        raise ValueError('native module identity mismatch')
    private = dict(schema='c6-linux-unit-definition-hash-inventory-v1', gamecore_sha256=MODULE,
                   field='Unit.e_type_index', registry='Units', fixtures=[])
    with tempfile.TemporaryDirectory(prefix='c6-unit-id-inventory-') as temp:
        temp = Path(temp)
        (temp/'src').mkdir()
        (temp/'src/main.rs').write_text(ADAPTER)
        (temp/'Cargo.toml').write_text('[package]\nname="c6-unit-id-inventory"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nc6-core={path='+json.dumps(str(ROOT/'crates/c6-core'))+'}\n')
        subprocess.run(['cargo','build','--release','--offline','--manifest-path',str(temp/'Cargo.toml')],check=True)
        for save_sha, (trace_name, trace_sha) in zip(SAVES,TRACES):
            with gzip.open(args.private_root/'evidence/linux-game-load/canonical'/f'{save_sha}.bin.gz','rb') as f:
                data = f.read(80657463)
            if len(data)!=80657462 or sha(data)!=save_sha:
                raise ValueError('native canonical Save identity mismatch')
            with gzip.open(args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name,'rb') as f:
                trace=f.read()
            if sha(trace)!=trace_sha:
                raise ValueError('native serializer ownership trace identity mismatch')
            rows = [json.loads(line) for line in trace.splitlines() if b'"owner":"GameCore::Unit::Instance"' in line]
            rows = sorted((r for r in rows if r['save']==1),key=lambda r:r['start'])
            if len(rows)!=1262:
                raise ValueError('native Unit count mismatch')
            directory=temp/save_sha
            directory.mkdir()
            last=0
            for i,row in enumerate(rows):
                start,end=row['start'],row['end']
                if not last<=start<end<=len(data):
                    raise ValueError('invalid/overlapping original Unit boundary')
                last=end
                (directory/f'{i:06}.bin').write_bytes(data[start:end])
            counts=json.loads(subprocess.check_output([str(temp/'target/release/c6-unit-id-inventory'),str(directory)],text=True))
            if sum(counts.values())!=1262:
                raise ValueError('decoded Unit count mismatch')
            private['fixtures'].append(dict(save_sha256=save_sha, trace_sha256=trace_sha,
                records=1262, typehash_counts=counts,
                record_bytes_sha256=sha(b''.join(data[r['start']:r['end']] for r in rows))))
    if args.registry_report:
        registry=json.loads(args.registry_report.read_text())
        if not all([args.registry_runtime_report,args.registry_before_save,args.registry_after_save]):
            p.error('registry comparison requires raw runtime report and both canonical controls')
        from unit_definition_ids_audit import audit_runtime, static_contract
        validated=audit_runtime(json.loads(args.registry_runtime_report.read_text()),
            static_contract(ROOT.parent/'c6-assets-linux'),
            args.registry_before_save,args.registry_after_save)
        validated['runtime_report_sha256']=sha(args.registry_runtime_report.read_bytes())
        if registry!=validated:
            raise ValueError('private registry audit differs from original evidence')
        lookup={str(e['hash']):e['index'] for e in registry['mapping']['entries']}
        for fixture in private['fixtures']:
            unresolved=set(fixture['typehash_counts'])-lookup.keys()
            if unresolved:
                raise ValueError(f'unresolved Unit definition hashes: {sorted(unresolved)}')
            fixture['native_indices']={h:lookup[h] for h in fixture['typehash_counts']}
        private['registry_report_sha256']=sha(args.registry_report.read_bytes())
        private['registry_configuration_sha256']=registry['configuration_sha256']
    private['sources']={name:sha((ROOT/name).read_bytes()) for name in ['crates/c6-core/src/unit.rs','crates/c6-core/src/unit/state.rs','crates/c6-core/src/unit/variants.rs','tools/linux/unit_definition_ids_inventory.py']}
    args.private_output.parent.mkdir(parents=True,exist_ok=True)
    args.private_output.write_text(json.dumps(private,sort_keys=True,indent=2)+'\n')
    public=dict(schema=private['schema'], gamecore_sha256=MODULE,
                field=private['field'],registry=private['registry'],
                private_report_sha256=sha(args.private_output.read_bytes()),
                sources=private['sources'], adapter_source_sha256=sha(ADAPTER.encode()),
                fixtures=[{k:v for k,v in f.items() if k not in ['typehash_counts','native_indices']} | dict(distinct_hashes=len(f['typehash_counts']),all_hashes_resolved='native_indices' in f) for f in private['fixtures']],
                full_state_action_turn_parity=False,
                scope='Current typed wire Unit.e_type_index only. Other definition kinds, live remapping and gameplay are unsupported. Raw 89198-byte Save/Load difference remains unmasked.')
    if args.registry_report:
        public['registry_report_sha256']=private['registry_report_sha256']
        public['registry_configuration_sha256']=private['registry_configuration_sha256']
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(public,sort_keys=True,indent=2)+'\n')
    print(json.dumps(public))


if __name__=='__main__':
    main()
