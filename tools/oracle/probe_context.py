#!/usr/bin/env python3
"""Bounded original QueryInterface introspection; no actual game context is created."""
import ctypes as C
import hashlib
import json
import pathlib
import subprocess
import sys
import run

DLL = run.ROOT.parent/'c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll'
if len(sys.argv)>1: DLL=pathlib.Path(sys.argv[1])
if hashlib.sha256(DLL.read_bytes()).hexdigest()!=run.CONFIG['dll_sha256']:
    raise SystemExit('DLL version mismatch')
source=run.ROOT/'native/oracle/context_probe.c'
output=run.ROOT/'native/oracle/build/libc6_context_probe.so'
subprocess.run(['cc','-O2','-Wall','-Wextra','-Werror','-fPIC','-shared','-o',str(output),str(source)],check=True)
loader=C.CDLL(str(run.ROOT/'native/oracle/build/libc6_oracle.so'))
loader.c6_host_source_sha256.restype=C.c_char_p
if loader.c6_host_source_sha256().decode()!=run.CONFIG['host_source_sha256']:
    raise SystemExit('Stale native loader')
loader.c6_load.argtypes=[C.c_char_p]
if loader.c6_load(str(DLL).encode()):raise SystemExit('Original PE image mapping failed')
probe=C.CDLL(str(output))
probe.c6_context_query_interface.argtypes=[C.c_size_t,C.POINTER(C.c_uint8)]
probe.c6_context_query_interface.restype=C.c_size_t
# Deliberately a synthetic identity pointer: QueryInterface never dereferences it.
identity=0x12345678
guids=['ae1400d760d96846bd9788a9c5b36b9a','e8e4f51860ef8d4cbb1b3834cc571031','00000000000000000000000000000000']
results=[]
for i,guid in enumerate(guids):
    native=(C.c_uint8*16).from_buffer_copy(bytes.fromhex(guid))
    actual=probe.c6_context_query_interface(identity,native)
    expected=identity if i<2 else 0
    assert actual==expected,(guid,actual,expected)
    results.append({'guid_bytes':guid,'returns_self':actual==identity,'returns_null':actual==0})
print(json.dumps({'status':'ok','mode':'isolated-original-QueryInterface','real_context_created':False,'dll_sha256':run.CONFIG['dll_sha256'],'rva':'0x5907a0','bridge_source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'results':results},sort_keys=True))
