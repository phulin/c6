#!/usr/bin/env python3
"""Replay original code, verify known edge cases and version gate; no Rust substitute."""
import json
import pathlib
import random
import subprocess
import sys
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import run

identity=json.loads(subprocess.check_output([sys.executable,str(run.ROOT/'tools/oracle/run.py'),'--identity']))
prng=random.Random(601)
cases=[]
for i in range(64):
    state={'seed':prng.randrange(1<<32),'calls':0xffffffff if i%2 else 0,'resets':0xffffffff,'lock_count':0xffffffff if i%3==0 else 0,'synchronous':bool(i%2)}
    steps=[{'op':'rng_get','bound':n} for n in [0,1,2,100,65535,65536,65537,0xffffffff]]
    steps += [{'op':'rng_lock'},{'op':'rng_unlock'},{'op':'rng_unlock'},{'op':'rng_reset','seed':42},{'op':'rng_initialize','seed':7}]
    cases.append({'protocol_version':1,'expected_engine':identity,'case':{'id':f'native-{i}','initial_state':{'rng':state},'steps':steps}})
payload=''.join(json.dumps(c)+'\n' for c in cases)
process=subprocess.run([sys.executable,str(run.ROOT/'tools/oracle/run.py')],input=payload,text=True,capture_output=True,check=True)
responses=[json.loads(line) for line in process.stdout.splitlines()]
assert len(responses)==len(cases)
for req,res in zip(cases,responses):
    assert res['status']=='ok',res
    state=dict(req['case']['initial_state']['rng'])
    for action,actual in zip(req['case']['steps'],res['steps']):
        op=action['op']
        if op=='rng_get':
            state['seed']=(state['seed']*1103515245+12345)&0xffffffff
            state['calls']=(state['calls']+1)&0xffffffff
            assert actual['rng_trace'][0]['result']==((state['seed']>>16)*(action['bound']&65535))>>16
        elif op=='rng_lock':state['lock_count']=(state['lock_count']+1)&0xffffffff
        elif op=='rng_unlock':state['lock_count']=max(0,state['lock_count']-1)
        elif op=='rng_reset':state.update(seed=action['seed'],resets=(state['resets']+1)&0xffffffff)
        elif op=='rng_initialize':state.update(seed=action['seed'],calls=0,resets=0,lock_count=0)
        assert actual['snapshot']=={'rng':state},(action,actual,state)
unsupported=dict(cases[0]);unsupported['case']={'id':'unsupported','initial_state':{},'steps':[{'op':'end_turn'}]}
r=subprocess.run([sys.executable,str(run.ROOT/'tools/oracle/run.py')],input=json.dumps(unsupported)+'\n',capture_output=True,text=True,check=True)
assert json.loads(r.stdout)['status']=='unsupported'
r=subprocess.run([sys.executable,str(run.ROOT/'tools/oracle/run.py'),'--dll',str(run.ROOT/'README.md')],input=json.dumps(cases[0])+'\n',capture_output=True,text=True,check=True)
assert json.loads(r.stdout)['status']=='unavailable' and 'SHA256 mismatch' in json.loads(r.stdout)['message']
print(f'PASS: {len(cases)} original-DLL cases, {sum(len(c["case"]["steps"]) for c in cases)} method calls; overflow, bounds, logging flag, unsupported action and wrong-DLL gate')
# Independently constructed original objects must not share seed/counters/locks.
streams={'protocol_version':1,'expected_engine':identity,'case':{'id':'interleaved-native-streams','initial_state':{'rng':{'seed':1},'rng_streams':{'1':{'seed':2},'4294967295':{'seed':3}}},'steps':[{'op':'rng_get','bound':100,'stream_id':1},{'op':'rng_reset','seed':42,'stream_id':4294967295},{'op':'rng_lock','stream_id':0},{'op':'rng_get','bound':65535,'stream_id':4294967295},{'op':'rng_get','bound':100,'stream_id':0}]}}
r=subprocess.run([sys.executable,str(run.ROOT/'tools/oracle/run.py')],input=json.dumps(streams)+'\n',capture_output=True,text=True,check=True)
observed=json.loads(r.stdout)
assert observed['status']=='ok',observed
first=observed['steps'][0]['snapshot']
assert first['rng']['seed']==1 and first['rng']['calls']==0
assert first['rng_streams']['1']['calls']==1 and first['rng_streams']['4294967295']['seed']==3
last=observed['steps'][-1]['snapshot']
assert last['rng']['calls']==1 and last['rng']['lock_count']==1
assert last['rng_streams']['1']['seed']==first['rng_streams']['1']['seed']
assert last['rng_streams']['4294967295']['calls']==1 and last['rng_streams']['4294967295']['resets']==2
streams['case']['steps'].append({'op':'rng_get','bound':1,'stream_id':99})
r=subprocess.run([sys.executable,str(run.ROOT/'tools/oracle/run.py')],input=json.dumps(streams)+'\n',capture_output=True,text=True,check=True)
assert json.loads(r.stdout)['status']=='error' and json.loads(r.stdout)['steps']==[]
print('PASS: interleaved three-instance original-DLL isolation and unknown stream preflight')
