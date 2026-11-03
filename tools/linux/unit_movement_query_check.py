#!/usr/bin/env python3
"""Freeze an answer-independent candidate, or compare a separately run original query.

This tool never executes original game code. Native acquisition/guard admission is
reviewed by the sole runner; numerical agreement alone cannot establish it.
"""
import argparse
import copy
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
TARGET = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'

def sha(data):
    return hashlib.sha256(data).hexdigest()

def rows(path):
    if path.stat().st_size > 32 * 1024 * 1024:
        raise ValueError('query artifact exceeds bound')
    return [json.loads(line) for line in path.read_text().splitlines() if line]

def keyed(records, kinds):
    result = {}
    for r in records:
        kind = r['kind']
        if kind not in kinds:
            raise ValueError('unexpected row kind')
        key = (kind,) if kind == 'global' else ((kind, r['query']) if kind == 'player' else (kind, r['owner'], r['id']))
        if key in result:
            raise ValueError('duplicate logical record')
        result[key] = r
    return result

def compare_values(candidate, inputs, answers, profile, core):
    """Strict input agreement precedes any desired-answer lookup/comparison."""
    header, summary = inputs[0], inputs[-1]
    if header != dict(kind='header', schema='c6-linux-unit-movement-inputs-v1',
                      profile_sha256=profile, candidate_core_sha256=core,
                      expected_units=1262, desired_answers_in_inputs=False):
        raise ValueError('input header/admission mismatch')
    if summary.get('kind') != 'summary' or summary.get('status') != 0 or summary.get('complete') is not True or summary.get('units') != 1262 or summary.get('known_rng_only') is not True:
        raise ValueError('incomplete original input capture')
    passes = []
    for phase in range(3):
        records = []
        for row in inputs[1:-1]:
            if row.get('pass') == phase:
                row = dict(row)
                del row['pass']
                records.append(row)
        passes.append(keyed(records, {'global'} if phase == 1 else {'global', 'player', 'unit'}))
    if sum(len(p) for p in passes) != len(inputs)-2:
        raise ValueError('unknown pass')
    if passes[0] != passes[2]:
        raise ValueError('query input fields changed across original calls')
    if set(passes[1]) != {('global',)} or passes[1][('global',)] != passes[0].get(('global',)):
        raise ValueError('global/RNG fields changed across original calls')
    rng = passes[0][('global',)].pop('known_rng', None)
    if not isinstance(rng, str) or not re.fullmatch('[0-9a-f]{128}', rng):
        raise ValueError('missing known RNG witnesses')
    expected = keyed([r for r in candidate if r['kind'] != 'answer'], {'global', 'player', 'unit'})
    if len([k for k in expected if k[0] == 'player']) != 65 or len([k for k in expected if k[0] == 'unit']) != 1262:
        raise ValueError('candidate coverage')
    if passes[0] != expected:
        mismatches = [str(k) for k in sorted(set(expected) | set(passes[0])) if expected.get(k) != passes[0].get(k)]
        raise ValueError('native inputs differ from owned candidate: '+','.join(mismatches[:16]))
    # Answers are deliberately not inspected until every relevant input agrees.
    if answers[0] != dict(kind='header', schema='c6-linux-unit-movement-answers-v1', profile_sha256=profile):
        raise ValueError('answer header')
    if answers[-1] != dict(kind='summary', status=0, answers=1262, complete=True):
        raise ValueError('incomplete answers')
    actual = keyed(answers[1:-1], {'answer'})
    desired = keyed([r for r in candidate if r['kind'] == 'answer'], {'answer'})
    if len(desired) != 1262 or actual != desired:
        mismatches = [str(k) for k in sorted(set(desired) | set(actual)) if desired.get(k) != actual.get(k)]
        raise ValueError('direct native query mismatch: '+','.join(mismatches[:16]))
    return dict(units=1262, direct_get_max_moves=1262, direct_has_moved=1262,
                query_inputs_match=True, native_query_values_match=True,
                known_rng_bytes_unchanged=64, full_rng_state_claimed=False,
                unplaced_queried=summary['unplaced_queried'], delayed_death_queried=summary['delayed_death_queried'],
                full_state_action_turn_parity=False, execution_admission_external_review_required=True)

def prepare(a):
    revision = subprocess.check_output(['git','rev-parse',a.source_commit+'^{commit}'],cwd=ROOT,text=True).strip()
    a.directory=a.directory.resolve()
    a.directory.mkdir(parents=True, exist_ok=False)
    archive = subprocess.check_output(['git','archive','--format=tar',revision],cwd=ROOT)
    (a.directory/'source.tar').write_bytes(archive)
    snapshot = a.directory/'source'
    snapshot.mkdir()
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            if member.name.startswith('/') or '..' in Path(member.name).parts or member.issym() or member.islnk():
                raise ValueError('unsafe source archive member')
        tar.extractall(snapshot)
    for name in ['unit_movement_query_compare.rs','unit_movement_query_check.py']:
        if not (snapshot/'tools/linux'/name).is_file():
            raise ValueError('source commit does not contain query tools')
    env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target'))
    build=subprocess.run(['cargo','build','--locked','--offline','-p','c6-data','--message-format=json'],cwd=snapshot,env=env,capture_output=True,text=True)
    (a.directory/'build.stderr').write_text(build.stderr)
    if build.returncode: raise RuntimeError(build.stderr)
    libs={}
    for line in build.stdout.splitlines():
        row=json.loads(line)
        if row.get('reason')=='compiler-artifact':
            for file in row.get('filenames',[]):
                if file.endswith('.rlib'): libs[row['target']['name']]=Path(file)
    binary=a.directory/'candidate'
    command=['rustc','--edition=2021','-Dwarnings',str(snapshot/'tools/linux/unit_movement_query_compare.rs'),'-L','dependency='+str(libs['c6_core'].parent),'-o',str(binary)]
    for name in ['c6_core','c6_data','serde_json','sha2']: command+=['--extern',name+'='+str(libs[name])]
    subprocess.run(command,check=True,cwd=snapshot)
    trace=a.directory/'owner-trace.jsonl'
    trace.write_bytes(gzip.decompress(a.trace.read_bytes()))
    args=[a.manifest,a.target_runtime,a.rules,a.rules/'report.json',a.placement,a.manager,trace]
    result=subprocess.run([str(binary),*map(str,args)],capture_output=True,text=True)
    (a.directory/'candidate.stderr').write_text(result.stderr)
    if result.returncode: raise RuntimeError(result.stderr)
    output=a.directory/'candidate.jsonl'
    candidate=[json.loads(line) for line in result.stdout.splitlines()]
    output.write_text(''.join(json.dumps(r,sort_keys=True)+'\n' for r in candidate if r['kind']!='answer'))
    candidate_answers=a.directory/'candidate-answers.jsonl'
    candidate_answers.write_text(''.join(json.dumps(r,sort_keys=True)+'\n' for r in candidate if r['kind']=='answer'))
    if len([r for r in candidate if r['kind']=='answer'])!=1262: raise ValueError('candidate coverage')
    csource=snapshot/'native/linux/unit_movement_query_fixture.c'
    obj=a.directory/'unit_movement_query_fixture.o'
    subprocess.run(['gcc','-std=gnu11','-Wall','-Wextra','-Werror','-fanalyzer','-fPIC','-c',str(csource),'-o',str(obj)],check=True)
    subprocess.run(['python3',str(snapshot/'tools/linux/unit_movement_query_audit.py'),
        '--assets',str(a.assets),'--private',str(a.directory/'native-source'),'--output',str(a.directory/'native-abi.json')],check=True,cwd=snapshot)
    # Preserve all argument identities and the full manifest-resolved artifacts.
    artifacts={}
    def retain_identity(path):
        path=path.resolve()
        if path.is_dir():
            for child in sorted(path.rglob('*')):
                if child.is_file(): retain_identity(child)
        else: artifacts[str(path)]=dict(bytes=path.stat().st_size,sha256=sha(path.read_bytes()))
    for path in args: retain_identity(path)
    manifest=json.loads(a.manifest.read_text())
    for value in manifest.values():
        paths=value.values() if isinstance(value,dict) else [value]
        for path in paths:
            if isinstance(path,str) and (a.manifest.parent/path).is_file(): retain_identity(a.manifest.parent/path)
    plan=dict(schema='c6-linux-unit-movement-query-candidate-v1',source_commit=revision,
        source_archive_sha256=sha(archive),candidate_sha256=sha(binary.read_bytes()),
        candidate_projection_sha256=sha(output.read_bytes()),candidate_answers_sha256=sha(candidate_answers.read_bytes()),fixture_object_sha256=sha(obj.read_bytes()),
        cargo_lock_sha256=sha((snapshot/'Cargo.lock').read_bytes()),artifacts=artifacts,
        native_abi_sha256=sha((a.directory/'native-abi.json').read_bytes()),
        toolchain={tool:subprocess.check_output(command,text=True) for tool,command in
            [('rustc',['rustc','-Vv']),('cargo',['cargo','--version']),('gcc',['gcc','--version'])]},
        dependency_rlibs={name:sha(path.read_bytes()) for name,path in libs.items()},
        expected_units=1262,expected_target_calls=2524,original_execution=False,
        candidate_core_sha256='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884',
        target_module_sha256=TARGET,target_host_sha256=HOST,
        execution_pending=['sole-runner independent retained-owner guard/acquisition review',
            'complete new native profile identity and frozen caller/guard/helper sources',
            'run original query after primary Manager profile closes','compare separately captured inputs and answers'],
        native_query_comparison=False,full_state_action_turn_parity=False)
    (a.directory/'plan.json').write_text(json.dumps(plan,sort_keys=True,indent=2)+'\n')
    print(json.dumps({k:v for k,v in plan.items() if k!='artifacts'},sort_keys=True))

def compare(a):
    plan=json.loads((a.directory/'plan.json').read_text())
    candidate_path=a.directory/'candidate.jsonl'
    if sha(candidate_path.read_bytes())!=plan['candidate_projection_sha256']:
        raise ValueError('frozen candidate changed')
    candidate_answers=a.directory/'candidate-answers.jsonl'
    if sha(candidate_answers.read_bytes())!=plan['candidate_answers_sha256']:
        raise ValueError('frozen candidate answers changed')
    report=compare_values(rows(candidate_path)+rows(candidate_answers),rows(a.inputs),rows(a.answers),a.profile_sha256,plan['candidate_core_sha256'])
    report.update(schema='c6-linux-unit-movement-query-value-comparison-v1',plan_sha256=sha((a.directory/'plan.json').read_bytes()),
        inputs_sha256=sha(a.inputs.read_bytes()),answers_sha256=sha(a.answers.read_bytes()),profile_sha256=a.profile_sha256)
    a.output.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
    print(json.dumps(report,sort_keys=True))

class ContractTests(unittest.TestCase):
    @staticmethod
    def example():
        # Artificial parser fixture only: never emitted as native observations.
        candidate=[dict(kind='global',turn=4)]+[dict(kind='player',query=i) for i in range(65)]
        candidate += [dict(kind='unit',owner=0,id=i,ordinal=i) for i in range(1262)]
        candidate += [dict(kind='answer',owner=0,id=i,ordinal=i,max_moves=3,has_moved=False) for i in range(1262)]
        inputs=[dict(kind='header',schema='c6-linux-unit-movement-inputs-v1',profile_sha256='p',candidate_core_sha256='c',expected_units=1262,desired_answers_in_inputs=False)]
        for phase in range(3):
            for row in candidate:
                if row['kind']=='answer' or (phase==1 and row['kind']!='global'):continue
                item=dict(row,**{'pass':phase})
                if item['kind']=='global':item['known_rng']='00'*64
                inputs.append(item)
        inputs.append(dict(kind='summary',status=0,complete=True,units=1262,known_rng_only=True,unplaced_queried=0,delayed_death_queried=0))
        answers=[dict(kind='header',schema='c6-linux-unit-movement-answers-v1',profile_sha256='p')]
        answers += [copy.deepcopy(r) for r in candidate if r['kind']=='answer']
        answers.append(dict(kind='summary',status=0,answers=1262,complete=True))
        return candidate,inputs,answers
    def test_complete_input_agreement_precedes_answer_comparison(self):
        candidate,inputs,answers=self.example()
        self.assertEqual(compare_values(candidate,inputs,answers,'p','c')['direct_has_moved'],1262)
        # A changed input must reject even if every desired answer still matches.
        inputs[67]['id']=9999
        with self.assertRaisesRegex(ValueError,'changed across'):
            compare_values(candidate,inputs,answers,'p','c')
    def test_native_answer_mismatch_rejected(self):
        candidate,inputs,answers=self.example()
        answers[19]['has_moved']=True
        with self.assertRaisesRegex(ValueError,'direct native query mismatch'):
            compare_values(candidate,inputs,answers,'p','c')
    def test_duplicate_identity_rejected(self):
        with self.assertRaisesRegex(ValueError,'duplicate'):
            keyed([dict(kind='unit',owner=0,id=2)]*2,{'unit'})
    def test_answers_cannot_enter_inputs(self):
        with self.assertRaisesRegex(ValueError,'unexpected'):
            keyed([dict(kind='answer',owner=0,id=2)],{'unit'})
    def test_partial_capture_rejected_before_answers(self):
        h=dict(kind='header',schema='c6-linux-unit-movement-inputs-v1',profile_sha256='p',candidate_core_sha256='c',expected_units=1262,desired_answers_in_inputs=False)
        with self.assertRaisesRegex(ValueError,'incomplete'):
            compare_values([], [h,dict(kind='summary',status=243)], [], 'p','c')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='mode',required=True)
    p=sub.add_parser('prepare')
    p.add_argument('--source-commit',required=True)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--directory',type=Path,required=True)
    private=ROOT.parent/'c6-ghidra/evidence'
    p.add_argument('--manifest',type=Path,default=private/'linux-military-initialization/native-3/candidate-paths-v2.json')
    p.add_argument('--target-runtime',type=Path,default=private/'linux-military-initialization/native-3/runtime.json')
    p.add_argument('--rules',type=Path,default=private/'linux-rules/mod-activation-xp2-constructor-final-1')
    p.add_argument('--placement',type=Path,default=private/'linux-placement-metadata/native-1')
    p.add_argument('--manager',type=Path,default=private/'linux-unit-operation-manager-metadata/native-1')
    p.add_argument('--trace',type=Path,default=private/'linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz')
    p=sub.add_parser('compare')
    for arg in ['directory','inputs','answers','output']: p.add_argument('--'+arg,type=Path,required=True)
    p.add_argument('--profile-sha256',required=True)
    sub.add_parser('test')
    a=parser.parse_args()
    if a.mode=='test': unittest.main(argv=[__file__],exit=True)
    elif a.mode=='prepare': prepare(a)
    else: compare(a)

if __name__=='__main__': main()
