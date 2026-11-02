#!/usr/bin/env python3
"""Validate original native serialization bytes, prefix consumption and unsupported guards."""
import json
import pathlib
import random
import struct
import subprocess
import sys
ROOT=pathlib.Path(__file__).resolve().parents[2]
COMMAND=[sys.executable,str(ROOT/'tools/oracle/serialization.py')]
identity=json.loads(subprocess.check_output(COMMAND+['--identity']))
random_source=random.Random(60118)
requests=[]
expected=[]
for i in range(256):
    fields=[random_source.randrange(1<<32) for _ in range(4)]
    raw_sync=i
    state=dict(zip(('seed','calls','resets','lock_count'),fields),synchronous_byte=raw_sync)
    encoded=struct.pack('<IIIIBB',1,*fields[:3],raw_sync,0)
    requests.append({'id':f'write-{i}','op':'write','state':state,'expected_engine':identity})
    expected.append({'bytes_hex':encoded.hex()})
    incoming=struct.pack('<I',random_source.randrange(1<<32))+encoded[4:]
    trailing=bytes(range(i%18))
    requests.append({'id':f'read-{i}','op':'read','bytes_hex':(incoming+trailing).hex(),'state':{'lock_count':fields[3]},'expected_engine':identity})
    expected.append({'state':state,'consumed':18,'trailing_hex':trailing.hex()})
for size in range(18):
    requests.append({'id':f'short-{size}','op':'read','bytes_hex':bytes(size).hex(),'expected_engine':identity})
    expected.append({'status':'unsupported'})
requests.append({'id':'legacy-true','op':'read','bytes_hex':(bytes(17)+b'\x01').hex(),'expected_engine':identity});expected.append({'status':'unsupported'})
r=subprocess.run(COMMAND,input=''.join(json.dumps(request)+'\n' for request in requests),capture_output=True,text=True,check=True)
responses=[json.loads(line) for line in r.stdout.splitlines()]
assert len(responses)==len(expected)
for response,wanted in zip(responses,expected):
    assert response['status']==wanted.get('status','ok'),response
    for key,value in wanted.items():assert response[key]==value,(response,wanted)
print(f'PASS: {len(responses)} original serialization cases: all256 raw sync bytes, arbitrary versions, preserved locks, prefix trailers,18 short inputs and legacy-true guards')
