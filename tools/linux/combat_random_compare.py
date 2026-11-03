#!/usr/bin/env python3
"""Prepare representative random-damage cases or compare a real sync-stream run."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

from combat_kernel_audit import ROOT,digest

SOURCES=['native/linux/combat_random.c','native/linux/combat_random.h',
         'native/linux/combat_kernel.c','native/linux/combat_kernel.h',
         'tools/linux/combat_random_audit.py','tools/linux/combat_random_compare.py',
         'crates/c6-core/src/combat_random.rs','crates/c6-core/examples/combat_random.rs',
         'crates/c6-core/src/combat.rs','crates/c6-core/src/rng.rs',
         'tools/linux/combat_kernel_audit.py','tools/linux/rng_inventory_audit.py','tools/linux/inspect_elf.py']


def prepare(args):
    base=[5120,5120,100,0,100,0xffffffff,10,1,24,0,0,0]
    cases=[]
    for changes in [{},{0:0},{1:0},{0:0,1:0},{2:0},{7:101},{5:0x5e97d629},
                    {5:0x5e97d629,3:99},{5:0x5e97d629,3:100},{5:0x5e97d629,11:1},
                    {5:0x5e97d629,4:0},{5:0x5e97d629,4:-1},
                    {8:2**31-1},{8:-2**31},{0:-2**31,1:2**31-1},{0:-1,1:1}]:
        p=base.copy()
        for key,value in changes.items():p[key]=value
        cases.append(p)
    # Equal strengths expose the actual sample in the result without clamps.
    # A continuous real stream is used, never reset or populated synthetically.
    cases.extend(base.copy() for _ in range(64))
    corpus=''.join(' '.join(map(str,p))+'\n' for p in cases)
    args.input.parent.mkdir(parents=True,exist_ok=True)
    if args.input.exists() and args.input.read_text()!=corpus:raise ValueError('refusing to overwrite different frozen corpus')
    args.input.write_text(corpus)
    manifest=dict(schema='c6-linux-random-damage-input-v1',input_sha256=digest(args.input),
        input_bytes=args.input.stat().st_size,case_count=len(cases),use_average=False,
        source_sha256={path:digest(ROOT/path) for path in SOURCES},
        scope='Representative parameter branches plus continuous genuine Game sync draws; initial state comes only from actual loaded native observation')
    args.output.write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(manifest,indent=2))


def raw_difference(before,after,path):
    a,b=before.read_bytes(),after.read_bytes();positions=[]
    if a!=b:
        for i,(x,y) in enumerate(zip(a,b)):
            if x!=y:positions.append(dict(offset=i,before=x,after=y))
        for i in range(min(len(a),len(b)),max(len(a),len(b))):
            positions.append(dict(offset=i,before=a[i] if i<len(a) else None,after=b[i] if i<len(b) else None))
    path.write_text(json.dumps(dict(before_sha256=digest(before),after_sha256=digest(after),
        before_bytes=len(a),after_bytes=len(b),differences=positions),separators=(',',':'))+'\n')
    return dict(before_bytes=len(a),after_bytes=len(b),different_bytes=len(positions),
        private_difference_sha256=digest(path),unknown_bytes_masked=False)


def compare(args):
    manifest=json.loads(args.manifest.read_text())
    if digest(args.input)!=manifest['input_sha256']:raise ValueError('native input digest mismatch')
    for path,sha in manifest['source_sha256'].items():
        if digest(ROOT/path)!=sha:raise ValueError('frozen source differs: '+path)
    runtime=json.loads(args.native_profile.read_text())
    if runtime['returncode']!=0 or not runtime['framework_cleanup_proven']:raise ValueError('native fixture unavailable')
    config=runtime['configuration']
    if hashlib.sha256(json.dumps(config,sort_keys=True,separators=(',',':')).encode()).hexdigest()!=runtime['configuration_sha256']:
        raise ValueError('native configuration identity differs')
    kernel=config['lower_core_random_damage_kernel']
    if (kernel['input_sha256']!=manifest['input_sha256'] or kernel['input_bytes']!=manifest['input_bytes']
            or kernel['case_count']!=manifest['case_count'] or kernel['use_average'] is not False):
        raise ValueError('native random-damage input/profile differs')
    for path in SOURCES[:6]+SOURCES[-3:]:
        if path=='tools/linux/combat_random_compare.py':continue
        if config['sources'][path]!=manifest['source_sha256'][path]:raise ValueError('native code/source mismatch: '+path)
    if runtime['combat_random_output']['sha256']!=digest(args.native_output):raise ValueError('native output identity differs')
    expected_files=[(args.before_save,runtime['game_saves']['save']),
                    (args.repeat_save,runtime['game_saves']['repeat']),
                    (args.control_save,runtime['combat_random_saves']['control']),
                    (args.post_save,runtime['combat_random_saves']['post'])]
    for path,identity in expected_files:
        if path.stat().st_size!=identity['bytes'] or digest(path)!=identity['sha256']:
            raise ValueError('original Save identity differs: '+str(path))
    rows=[];fp={}
    for line in args.native_output.read_text().splitlines():
        if line.startswith('floating_environment_'):
            key,rounding,mxcsr,x87=line.split();fp[key]=dict(fe_round=int(rounding),mxcsr=int(mxcsr),x87_control=int(x87))
        else:rows.append(json.loads(line))
    profile,initial,*cases=rows
    if profile['kind']!='profile' or initial['kind']!='initial' or len(cases)!=manifest['case_count']:
        raise ValueError('native observation incomplete')
    marker=f"C6_LINUX_COMBAT_RANDOM calculations_returned {len(cases)} requested {len(cases)} result 0 complete_rng_inventory 0 full_combat_parity 0"
    if marker not in runtime['stderr'].splitlines():raise ValueError('original calculation completion marker absent')
    mapping=profile['pointer_mapping'];pointers=[int(row['pointer'],16) for row in mapping]
    ranks=[row['rank'] for row in mapping]
    ordered=sorted(pointers)
    if len(set(pointers))!=len(mapping) or sorted(ranks)!=list(range(len(mapping))) or any(
            ordered[row['rank']]!=int(row['pointer'],16) for row in mapping):raise ValueError('native pointer-to-rank mapping invalid')
    state=initial['state'];header=[state[k] for k in ['seed','calls','resets','locks','synchronous_byte']]
    if profile['registered_count']!=len(mapping) or mapping[profile['sync_id']]['rank']!=profile['sync_rank']:
        raise ValueError('actual named sync wrapper mapping differs')
    header += [profile['sync_rank'],len(state['dirty_ranks']),*state['dirty_ranks']]
    requests=' '.join(map(str,header))+'\n'+args.input.read_text()
    variants=[];mismatches=[];native_state=state;insertions=0;samples=set()
    for i,row in enumerate(cases):
        if row['index']!=i or row['kind']!='case' or row['before']!=native_state:raise ValueError('native state continuity differs')
        a,b=row['before'],row['after'];native_state=b
        seed=(a['seed']*0x41c64e6d+0x3039)&0xffffffff
        if b['seed']!=seed or b['calls']!=((a['calls']+1)&0xffffffff):raise ValueError('native did not consume exactly one stream draw')
        if any(a[k]!=b[k] for k in ['resets','locks','synchronous_byte','wrapper16']):raise ValueError('native non-draw live fields changed')
        if b['dirty_ranks']!=sorted(set(a['dirty_ranks'])|{profile['sync_rank']}):raise ValueError('native dirty membership/order differs')
        insertions+=profile['sync_rank'] not in a['dirty_ranks']
        samples.add(((seed>>16)*(profile['global_extra_damage']&65535))>>16)
        raw_a,raw_b=bytes.fromhex(a['random32']),bytes.fromhex(b['random32'])
        if raw_a[:8]!=raw_b[:8] or raw_a[16:]!=raw_b[16:]:raise ValueError('native opaque/nonselected Random bytes changed')
        archive_a,archive_b=bytes.fromhex(a['archive152']),bytes.fromhex(b['archive152'])
        if archive_a[:56]!=archive_b[:56] or archive_a[80:]!=archive_b[80:]:raise ValueError('native non-tree archive bytes changed')
    for release in [False,True]:
        build=['cargo','build','-p','c6-core','--example','combat_random']+(['--release'] if release else [])
        subprocess.run(build,cwd=ROOT,check=True)
        executable=ROOT/'target'/('release' if release else 'debug')/'examples/combat_random'
        answer=subprocess.run([str(executable),str(profile['global_extra_damage'])],input=requests,text=True,capture_output=True,check=True)
        match=re.search(r'^C6_COMBAT_RANDOM_RUNTIME (\d+) (\d+) (\d+)$',answer.stderr,re.M)
        if not match:raise ValueError('candidate math/FP observation absent')
        expf,mxcsr,x87=map(int,match.groups());before=fp['floating_environment_before'];after=fp['floating_environment_after']
        if expf!=profile['expf_offset'] or (mxcsr&~63)!=(before['mxcsr']&~63) or x87!=before['x87_control']:
            raise ValueError('candidate actual math/FP controls differ')
        if (after['mxcsr']&~63)!=(before['mxcsr']&~63) or after['x87_control']!=before['x87_control'] or after['fe_round']!=before['fe_round']:
            raise ValueError('native changed FP control environment')
        native_libm=Path(profile['libm_path']);candidate_libm=None
        for line in subprocess.check_output(['ldd',str(executable)],text=True).splitlines():
            found=re.search(r'libm\.so\.\d+ => (/\S+)',line)
            if found:candidate_libm=Path(found[1])
        if candidate_libm is None or digest(candidate_libm)!=digest(native_libm):raise ValueError('math module identity differs')
        answers=answer.stdout.splitlines()
        if len(answers)!=len(cases):raise ValueError('candidate refused supported native cases')
        for i,(line,row) in enumerate(zip(answers,cases)):
            words=line.split();actual=list(map(int,words[2:]));b=row['after']
            sample=((b['seed']>>16)*(profile['global_extra_damage']&65535))>>16
            expected=[row['damage'],sample,*[b[k] for k in ['seed','calls','resets','locks','synchronous_byte']],len(b['dirty_ranks']),*b['dirty_ranks']]
            if int(words[0])!=i or words[1]!='damage' or actual!=expected:mismatches.append(dict(release=release,index=i,candidate=line))
        variants.append(dict(release=release,executable_sha256=digest(executable),expf_offset=expf,mxcsr=mxcsr,x87_control=x87,
            libm_sha256=digest(native_libm),stdout_sha256=hashlib.sha256(answer.stdout.encode()).hexdigest()))
    # Paths are explicit private artifacts; no save differences are normalized.
    differences={name:raw_difference(a,b,args.private_differences/f'{name}.json') for name,a,b in [
        ('baseline_control',args.before_save,args.control_save),('random_draws',args.before_save,args.post_save),
        ('postdraw_control',args.post_save,args.repeat_save)]}
    if differences['baseline_control']['different_bytes'] or differences['postdraw_control']['different_bytes']:
        raise ValueError('original Save control mismatch')
    report=dict(schema='c6-linux-game-sync-random-damage-comparison-v1',native_profile_sha256=digest(args.native_profile),
        configuration_sha256=runtime['configuration_sha256'],input_manifest=manifest,unique_native_cases=len(cases),
        comparisons=len(cases)*len(variants),mismatches=mismatches,variants=variants,native_fp=fp,
        native_expf_offset=profile['expf_offset'],native_floorf_offset=profile['floorf_offset'],
        global_max_extra_damage=profile['global_extra_damage'],observed_samples=sorted(samples),
        dirty_first_insertions=insertions,dirty_existing_edits=len(cases)-insertions,save_differences=differences,
        complete_rng_inventory=False,full_live_layout_equivalence=False,full_combat_parity=False,
        limits=['Named conditional Game sync transition; logical ranks are validated pointer-order mapping, not native addresses or archive IDs',
                'Original tree allocation/shape and logging/service state are not Rust live-layout equivalence',
                'Trap-after-draw prefix is explicit in Rust; no native trapping input is a passing numeric case',
                'Raw Save byte changes remain reported; no unknown bytes masked, no full-state import/restore parity'])
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:report[k] for k in ['unique_native_cases','comparisons','dirty_first_insertions','dirty_existing_edits']}))
    if mismatches:raise SystemExit(1)


def main():
    ap=argparse.ArgumentParser(description=__doc__);subs=ap.add_subparsers(dest='command',required=True)
    prep=subs.add_parser('prepare');prep.add_argument('--input',type=Path,required=True);prep.add_argument('--output',type=Path,required=True)
    comp=subs.add_parser('compare')
    for flag in ['input','manifest','native-output','native-profile','before-save','control-save','post-save','repeat-save','private-differences','output']:
        comp.add_argument('--'+flag,type=Path,required=True)
    args=ap.parse_args()
    if args.command=='compare':args.private_differences.mkdir(parents=True,exist_ok=True)
    prepare(args) if args.command=='prepare' else compare(args)


if __name__=='__main__':main()
