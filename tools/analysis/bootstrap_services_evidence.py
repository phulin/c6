#!/usr/bin/env python3
"""Reproduce bounded host service ABI evidence; raw bytes stay in private research tree."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

IDENTITIES = {
    "GameCore_XP2_FinalRelease.dll": "324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c",
    "CivilizationVI.exe": "e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63",
    "DatabaseDLL_FinalRelease.dll": "e6577e873f946224df65c02f6c4bc3b385607e2a82d957b0fd613e01cbe78b0c",
}
RANGES = {
    "GameCore_XP2_FinalRelease.dll": [(0x45050,0x45100),(0x45d90,0x45ecc),(0x1a1400,0x1a15e3),(0x590440,0x590460),(0x5905f0,0x590600),(0x591750,0x591758),(0x5fda50,0x5fda60),(0x5fe1c0,0x5fe250),(0x5e34e0,0x5e3640),(0x46df0,0x470b0),(0x14930,0x149e0),(0x9908f0,0x990914),(0x994970,0x99498d),(0x639210,0x639244),(0x422f0,0x42320),(0x995630,0x995749),(0x45ed0,0x45f1f),(0x590600,0x59060b),(0x92a290,0x92a339),(0x979630,0x979676),(0x7503e0,0x750401)],
    "CivilizationVI.exe": [(0x1c9620,0x1c9640),(0x1c9720,0x1c9740),(0x1c9a60,0x1c9a80),(0x641b0,0x641b8),(0x64650,0x64658),(0xbcbe0,0xbcbef),(0xbcb60,0xbcbc8),(0xbf980,0xbf9f1),(0xbc880,0xbc948),(0xdba4f0,0xdba525),(0xdbae40,0xdbaf12),(0x19d350,0x19d358),(0x982080,0x982087),(0x1c9710,0x1c9718),(0x24de00,0x24dee8)],
    "DatabaseDLL_FinalRelease.dll": [(0x1020,0x10fb),(0x1120,0x1142),(0x13c0,0x1432),(0x1ae0,0x1be6),(0x1bf0,0x1d30)],
}
def pe_fields(data):
    pe = struct.unpack_from("<I", data, 0x3c)[0]
    count = struct.unpack_from("<H", data, pe+6)[0]
    optional = pe+24
    base = struct.unpack_from("<Q", data, optional+24)[0]
    sections = optional+struct.unpack_from("<H", data, pe+20)[0]
    return base, [(struct.unpack_from("<IIII", data, sections+40*i+8)) for i in range(count)]
def at_rva(data, rva, count):
    for size, va, raw_size, offset in pe_fields(data)[1]:
        if va <= rva and rva+count <= va+raw_size:
            return data[offset+rva-va:offset+rva-va+count]
    raise ValueError(f"RVA not backed by file bytes: {rva:#x}")
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, default=Path("../c6-assets"))
    parser.add_argument("--output", type=Path, default=Path("../c6-ghidra/evidence/bootstrap/services"))
    args = parser.parse_args()
    paths = {
        "GameCore_XP2_FinalRelease.dll": args.assets/"DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll",
        "CivilizationVI.exe": args.assets/"Base/Binaries/Win64Steam/CivilizationVI.exe",
        "DatabaseDLL_FinalRelease.dll": args.assets/"Base/Binaries/Win64Steam/DatabaseDLL_FinalRelease.dll",
    }
    # Verify every module before emitting partial evidence.
    data = {name: path.read_bytes() for name,path in paths.items()}
    for name, blob in data.items():
        if hashlib.sha256(blob).hexdigest() != IDENTITIES[name]:
            raise SystemExit(f"unsupported target identity: {name}")
    args.output.mkdir(parents=True, exist_ok=True)
    for name,path in paths.items():
        base = pe_fields(data[name])[0]
        with (args.output/(name+".asm")).open("w") as output:
            for start,end in RANGES[name]:
                subprocess.run(["objdump","-d","-M","intel",f"--start-address={base+start:#x}",f"--stop-address={base+end:#x}",str(path)], stdout=output,check=True)
    strings = {f"{rva:#x}": at_rva(data["CivilizationVI.exe"],rva,100).split(b"\0")[0].decode("ascii") for rva in [0xfb30c8,0xfb35b0,0xfb3030,0xfb3040,0xfb3060]}
    constants = {"engine_guid_raw": at_rva(data["GameCore_XP2_FinalRelease.dll"],0x9bfba8,16).hex(), "host_strings":strings, "identities":IDENTITIES}
    (args.output/"constants.json").write_text(json.dumps(constants,indent=2)+"\n")
    print(json.dumps({"output":str(args.output),"module_count":len(paths),"engine_guid":constants["engine_guid_raw"]}))
if __name__ == "__main__":
    main()
