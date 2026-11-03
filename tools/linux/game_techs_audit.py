#!/usr/bin/env python3
"""Static pinned Linux Game.Techs owner audit; never execute original code."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import shutil
import sys
import struct
import subprocess
import tarfile
from save_lifecycle_audit import Elf, GAME_SHA
ROOT = Path(__file__).resolve().parents[2]
CORE = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE = '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
METHODS = [('Techs.Write', '0x2a2c1b0', 66, '38891bb4fd33d37b12897b2bd9546ab3559e6d75764a90f1e30b0400e3deaeb9'), ('Techs.Read', '0x2a2c200', 93, 'e7d98e81ccf16a8ad9f610fcfcf45a2caae4998ff2b048e7112f257b0ee860aa'), ('TechNode.Write', '0x2a2c0c0', 83, 'fe624df634f96761b9eaed6c42b925250ee9737a2973b1f4b459582ae848e784'), ('TechNode.Read', '0x2a2c120', 141, '183c90b1835862e01b7c2b78a260381b4072db9b349cbd0f4d9d117dd57fd8b6'), ('Techs.Clear', '0x2a2c260', 48, '5db6dbf3090f30e70c25a2a3e3c978b550e4ab5a834747d288aea3161ee38bbf'), ('Techs.Validate', '0x2a2c290', 350, '691e8f60d013663f6a99c4e96885c9d59116c20dbbeb994e8830b396f68b4126'), ('Techs.Initialize', '0x2a2c8b0', 9, '89ba4a942059f1ab0c99733e322c6579bc9f4cb26dcbf3f8863604df28f91d51'), ('Techs.InitializeNewGame', '0x2a2c8d0', 5, '2a4f8b744a58031dd6b2f3062f09553a948c3a272790e192626d362879ec5efd'), ('Techs.Build', '0x2a2c8e0', 2084, '82fb0a3ec1d8b84c36b1a512d827ee0898c8af2423a75c1b3909fd7be51cb7d9'), ('Techs.Reset', '0x2a2d110', 5, '8b48c50c19c94cfc2c73869d40ef39ef261dbaecd4b94edcb75e7969552806cf'), ('Techs.PostLoad', '0x2a2d120', 47, '3884e2f4180bd0196c0b7cd3369b38ecf69fcc40b646756bddd4fe52de715571'), ('TechNodeSequence.Read', '0x2a2e980', 125, '33691a8e6ec50ed5945cbf955d4fa46bd30f656ebb254e3af180f08df5e6556c'), ('TechTypesSequence.Read', '0x282a4b0', 125, '7a3af50f3afa339a57fc66debc2577ba1b143721966a8d3d73d7fe2f8c1966da'), ('TechTypes.Read', '0x2d4ef20', 58, '047050d264acc31d6a4477873412f54329d63a12b971850fa8ca7602c1cfa31c'), ('TechTypes.Write', '0x2d4eef0', 36, 'e8b21f56f96030386a4c9aac4582d7fce63120854b0a642cf15392ac5210cdad'), ('HashToIndex', '0x2016480', 34, 'bbba3422fa0bd6194a1c30c88ee7b8e77f8f7f0733c020e55d5c967202a1dd21'), ('Validate.node', '0x2a2e040', 30, 'fb203fbdcc5fd48a43d793549445e8f568ca801f7f54eb63cd84b32ac26b4c7e'), ('Validate.iteration', '0x2a2e100', 17, '4f439f0638e1989c0cc31e497cba16d58734204bfdb09108f338d2441f95abf8'), ('Validate.prereq', '0x2a2e1c0', 17, '0626a4b2e67d798e616f9ec0a9c793ce50cbba95de8c6a8fd27b64e3246d4e29'), ('Fixed.Read', '0x2c9b010', 43, '5d47fcdfb3868524b3d8da8a2fab95de165678efa05f23c615963da12ddf010d'), ('ReadVersioned.bool', '0x1fe5e80', 28, '57e8e97c7be2741a1dc218f74049b9cf7a99191cffd41785ce1aa9dd4c6518e5'), ('ReadVersioned.Fixed', '0x285ed40', 36, '3523338d40d65af7b912247d0f6c3c40fa502332c8ed14e65366fe93765463ff'), ('GetTechNode', '0x2a2d180', 132, 'a7ba4456eb131bf96c471355b9e7266eeca1fb924e828b1dfa32670b15614095'), ('HasTech', '0x2a2d160', 24, '15b73ac27200be281d4dc5657f341244ff1f86e80e95a10d7d3c0dfeda4125a9'), ('GetPrerequisites', '0x2a2d3e0', 39, '3bdf1c56be93ca9db7c0ccfb7e02e3c00fe8460c96604c44c6f24e7be24ac5ed')]
METHODS += [('Techs.Constructor', '0x2a2c6c0', 381, 'd98ec6ed85d64e04b7d81e7855cf6b8b4cada7248acb6c481e27dc9300ac1271'), ('TechTypesVector.Constructor', '0x21e39d0', 5, 'e9718a86f403f7659a94009643824fd99bd5b4a040559a22c664e3b9093282d3'), ('TechTypesVectorBase.Constructor', '0x21e4110', 30, 'febf3205371d569515494ec5516df379288755e663651a06739bbb8f6aee5cde')]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def audit(args):
    private = args.private_output
    private.mkdir(parents=True, exist_ok=True)
    elf = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for name, address, size, expected in METHODS:
        if sha(elf.read(int(address, 16), size)) != expected:
            raise ValueError('method mismatch: ' + name)
    if elf.read(0x2f549d0, 4) != struct.pack('<I', 3):
        raise ValueError('current wire version differs')
    calls = [(0x2a2c789,0x1f2dfa0), (0x2a2c217,0x1f19c00), (0x2a2c24d,0x1e93430),
             (0x2a2d12b,0x1ea1750), (0x2a2d13b,0x1e0d770),
             (0x2016488,0x1f4e920), (0x2a2e051,0x1f4e920),
             (0x2a2e104,0x1f4e920), (0x2a2e1c4,0x1f4e920)]
    for address,target in calls:
        elf.call(address,target)
    # Keep proprietary disassembly private, not in the public evidence report.
    with (private/'owner-source.disasm').open('w') as f:
        for _,address,size,_ in METHODS:
            subprocess.run(['objdump','-d','-C','--start-address='+address,
                '--stop-address='+hex(int(address,16)+size), str(args.assets/'libGameCore_XP2.so')],
                stdout=f,check=True)
    core = gzip.decompress((ROOT.parent/f'c6-ghidra/evidence/linux-game-load/canonical/{CORE}.bin.gz').read_bytes())
    trace = gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz').read_bytes())
    if sha(core)!=CORE or sha(trace)!=TRACE:
        raise ValueError('original diagnostic provenance differs')
    frames = [json.loads(row) for row in trace.splitlines()]
    frames = [r for r in frames if r.get('event')=='owner' and r.get('save')==1 and r.get('owner')=='GameCore::Game::Techs']
    if len(frames)!=1 or frames[0]['depth']!=1 or frames[0]['parent']!=1:
        raise ValueError('owner source framing differs')
    frame=frames[0]; wire=core[frame['start']:frame['end']]
    owner_path=private/'original-owner.bin';owner_path.write_bytes(wire)
    # Retain every source preimage used by this std-only build and audit. This
    # archive is the provenance, not a hash-only claim about concurrent WIP.
    paths=['crates/c6-core/src/game_techs.rs','tools/linux/game_techs_check.rs',
           'tools/linux/game_techs_audit.py','tools/linux/save_lifecycle_audit.py']
    archive=io.BytesIO()
    with tarfile.open(fileobj=archive,mode='w') as tar:
        for relative in paths:
            data=(ROOT/relative).read_bytes(); info=tarfile.TarInfo(relative)
            info.size=len(data);info.mode=0o644;info.mtime=0
            tar.addfile(info,io.BytesIO(data))
    archive_bytes=archive.getvalue();archive_sha=sha(archive_bytes)
    source_path=private/f'source-preimages-{archive_sha}.tar'
    if source_path.exists() and source_path.read_bytes()!=archive_bytes:
        raise ValueError('immutable archive differs')
    source_path.write_bytes(archive_bytes)
    compiler=Path(shutil.which('rustc')).resolve()
    binary=private/'game-techs-check'
    subprocess.run(['rustc','--edition=2024','-D','warnings',
        str(ROOT/'tools/linux/game_techs_check.rs'),'-o',str(binary)],check=True)
    checked=subprocess.run([str(binary),str(owner_path)],check=True,capture_output=True,text=True)
    decoded=json.loads(checked.stdout)
    sysroot=Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())
    dependencies={str(p.relative_to(sysroot)):sha(p.read_bytes()) for p in sorted(sysroot.glob('lib/rustlib/*/lib/*.rlib'))}
    return dict(schema='c6-linux-game-techs-source-v1',checks_passed=True,
        gamecore_sha256=GAME_SHA,methods=[dict(name=n,address=a,bytes=s,sha256=h) for n,a,s,h in METHODS],
        calls=[dict(address=hex(a),target=hex(t)) for a,t in calls],wire_version=3,
        owner_layout='u32 version, u32 node count, nodes, u32 iteration count, TechTypes hashes',
        node_layout='u32 version, u32 type hash, byte active bool, i32 raw FixedPoint8 cost, u32 prereq count, TechTypes hashes',
        read='ClearTechs; read both vectors; ValidateTechs stably removes definition-Find-null nodes, iteration types and retained-node prerequisites',
        remapping='HashToIndex uses actual Find(hash), returns GetCollectionIndex or -1; Validate uses Find(native index as unsigned), without a separate sentinel test',
        fallback='owner constructor calls empty TechTypes vector constructor at +0x110; Clear/Read/Validate/no-rebuild path does not modify fallback',
        post_load='BuildTechs iff nodes or iteration is empty; otherwise no graph mutation',
        original_diagnostic=dict(core_sha256=CORE,core_bytes=len(core),owner_trace_decompressed_sha256=TRACE,
            owner_range=[frame['start'],frame['end']],owner_sha256=sha(wire),decoded=decoded,
            native_registry_projection=False,meaning='saved owner grammar only; no live initialized graph comparison'),
        candidate=dict(audit_python_version=sys.version,audit_python_sha256=sha(Path(sys.executable).resolve().read_bytes()),executable_sha256=sha(binary.read_bytes()),compiler_sha256=sha(compiler.read_bytes()),
            compiler_version=subprocess.check_output(['rustc','-Vv'],text=True),dependencies=dependencies,
            source_preimages_archive=str(source_path.relative_to(ROOT.parent)),source_preimages_sha256=archive_sha,
            source_preimages=paths,test_dependencies='std-only API-shaped Limits/DefinitionLookup shims retained in checker; not full Core'),
        tests='synthetic duplicate/positional lookup, stable unknown filtering, prereq/order preservation, cost bits, all truncations, exact/one-short byte/items, version/bool/trailing refusal, empty-after-filter rebuild gap; original owner decode',
        private_evidence=str(private.relative_to(ROOT.parent)),original_execution=False,native_differential_acceptance=False,
        complete_core_import=False,gameplay_parity=False,action_parity=False,turn_parity=False,
        raw_save_load_difference_bytes=89198,
        remaining=['native-registry-bound projection of actual retained owner','witness-bound Game.Techs export from retained tail',
                   'BuildTechs fallback for empty vectors','passive initialized graph capture and affected-state native comparison'])


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-output',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-game-techs')
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args();result=audit(args)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    print(json.dumps(dict(checks_passed=True,original_owner=result['original_diagnostic']['decoded'],original_execution=False)))


if __name__=='__main__':
    main()
