#!/usr/bin/env python3
"""Independent instruction-level geometry model checked against the original PE."""
import ctypes, hashlib, itertools, json, pathlib, random
ROOT=pathlib.Path(__file__).resolve().parents[2]
DLL=pathlib.Path('/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll')
cfg=json.loads((ROOT/'tools/oracle/config.json').read_text())
assert hashlib.sha256(DLL.read_bytes()).hexdigest()==cfg['dll_sha256']
lib=ctypes.CDLL(str(ROOT/'native/oracle/build/libc6_oracle.so'))
lib.c6_load.argtypes=[ctypes.c_char_p]; assert lib.c6_load(str(DLL).encode())==0
lib.c6_plot_distance.argtypes=[ctypes.c_int32,ctypes.c_uint8]+[ctypes.c_int32]*4
lib.c6_plot_distance.restype=ctypes.c_int32
lib.c6_plot_valid.argtypes=[ctypes.c_int32]*2;lib.c6_plot_valid.restype=ctypes.c_uint8

def i32(x): return ctypes.c_int32(x).value

def half(y):
    if y>=0: return y>>1
    a=i32(y-1)
    return i32(a-(a>>31))>>1

def metric(q,r):
    aq,ar=i32(abs(q)),i32(abs(r))
    return i32(aq+ar) if (q>=0)==(r>=0) else max(aq,ar)

def model(w,wrap,x1,y1,x2,y2):
    dq=i32(i32(x2-half(y2))-i32(x1-half(y1)))
    dr=i32(y2-y1)
    ds=[metric(dq,dr)]
    if wrap: ds.extend([metric(i32(dq-w),dr),metric(i32(dq+w),dr)])
    return min(ds)

rng=random.Random(0x15ec0)
edge=[-2147483648,-2147483647,-9999,-3,-2,-1,0,1,2,3,9999,2147483646,2147483647]
cases=itertools.chain(((w,z,*c) for w in [0,1,2,80,-1,-2147483648,2147483647] for z in [0,1,255] for c in itertools.product(edge,repeat=4)),((rng.randrange(-2**31,2**31),rng.randrange(2),*(rng.randrange(-2**31,2**31) for _ in range(4))) for _ in range(100000)))
count=0
for c in cases:
    actual=lib.c6_plot_distance(*c);expected=model(*c)
    assert actual==expected,(c,actual,expected)
    count+=1
for x,y in itertools.product(edge,repeat=2): assert bool(lib.c6_plot_valid(x,y))==(x>-9999 and y>-9999)
print(json.dumps({'distance_cases':count,'valid_cases':len(edge)**2,'mismatches':0,'dll_sha256':cfg['dll_sha256'],'scope':'isolated original code with synthetic map fields; full state/action/end-turn untested'}))
