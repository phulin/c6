#!/usr/bin/env python3
"""Dedicated original Random binary serialization oracle, independent of game envelopes."""
import argparse
import ctypes as C
import hashlib
import json
import pathlib
import subprocess
import sys

ROOT=pathlib.Path(__file__).resolve().parents[2]
CONTRACT=pathlib.Path(__file__).with_name('serialization-config.json')
FIELDS=('seed','calls','resets','lock_count','synchronous_byte')
class Random(C.Structure):
    _fields_=[('vptr',C.c_uint64),('seed',C.c_uint32),('calls',C.c_uint32),('resets',C.c_uint32),('lock_count',C.c_uint32),('synchronous_byte',C.c_uint8),('padding',C.c_uint8*7)]

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def state_object(fields):
    r=Random()
    for key in FIELDS:
        value=fields.get(key,1 if key=='resets' else 0)
        limit=255 if key=='synchronous_byte' else 0xffffffff
        if isinstance(value,bool) or not isinstance(value,int) or not 0<=value<=limit:raise ValueError('invalid '+key)
        setattr(r,key,value)
    return r

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dll',type=pathlib.Path,default=ROOT.parent/'c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll')
    parser.add_argument('--identity',action='store_true')
    args=parser.parse_args()
    config=json.loads(CONTRACT.read_text())
    identity={'kind':'original_dll_serialization','binary_sha256':config['dll_sha256'],'configuration_sha256':digest(CONTRACT)}
    if args.identity:print(json.dumps(identity));return
    failure=None
    try:
        for source,expected in config['sources'].items():
            if digest(ROOT/source)!=expected:raise ValueError('source identity mismatch: '+source)
        if digest(args.dll)!=config['dll_sha256']:raise ValueError('DLL SHA256 mismatch')
        library=ROOT/'native/oracle/build/libc6_serialization.so'
        subprocess.run(['cc','-O2','-Wall','-Wextra','-Werror','-fPIC','-shared','-o',str(library),str(ROOT/'native/oracle/serialization_host.c')],check=True)
        loader=C.CDLL(str(ROOT/'native/oracle/build/libc6_oracle.so'))
        loader.c6_host_source_sha256.restype=C.c_char_p
        if loader.c6_host_source_sha256().decode()!=config['sources']['native/oracle/image_host.c']:raise ValueError('stale native loader')
        loader.c6_load.argtypes=[C.c_char_p]
        if loader.c6_load(str(args.dll).encode()):raise ValueError('PE mapping failed')
        host=C.CDLL(str(library))
        host.c6_random_write.argtypes=[C.POINTER(Random),C.POINTER(C.c_uint8)]
        host.c6_random_read.argtypes=[C.POINTER(Random),C.POINTER(C.c_uint8),C.c_uint32]
    except (OSError,ValueError,subprocess.CalledProcessError) as e:failure=str(e)
    for line in sys.stdin:
        response={'engine':identity}
        try:
            request=json.loads(line)
            response['id']=request['id']
            if request.get('expected_engine')!=identity:raise ValueError('expected_engine identity mismatch')
            if failure:response.update(status='unavailable',message=failure)
            else:
                r=state_object(request.get('state',{}))
                op=request['op']
                if op=='write':
                    out=(C.c_uint8*18)()
                    if host.c6_random_write(C.byref(r),out):raise ValueError('native write stream contract mismatch')
                    response.update(status='ok',bytes_hex=bytes(out).hex())
                elif op=='read':
                    data=bytes.fromhex(request['bytes_hex'])
                    if len(data)<18 or data[17]!=0:
                        response.update(status='unsupported',message='bounded normal path requires at least18 bytes and legacy flag0; no native state mutation executed')
                    else:
                        buffer=(C.c_uint8*len(data)).from_buffer_copy(data)
                        if host.c6_random_read(C.byref(r),buffer,len(data)):raise ValueError('native read stream contract mismatch')
                        response.update(status='ok',state={key:getattr(r,key) for key in FIELDS},consumed=18,trailing_hex=data[18:].hex())
                else:response.update(status='unsupported',message='supported ops: write/read')
        except (ValueError,TypeError,KeyError) as e:response.update(status='error',message=str(e))
        print(json.dumps(response,sort_keys=True),flush=True)
if __name__=='__main__':main()
