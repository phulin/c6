#!/usr/bin/env python3
"""Extract fingerprinted native script-library bridge evidence to private storage."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from bootstrap_services_evidence import IDENTITIES, at_rva, pe_fields

RANGES = {
    "CivilizationVI.exe": [(0xf8430, 0xf8472), (0xf8510, 0xf85d4),
                           (0x161160, 0x161296), (0x1614a0, 0x16154a),
                           (0x161a90, 0x161ca7), (0x64650, 0x6466c),
                           (0x2a0b40, 0x2a0ba0), (0x2a3440, 0x2a3535)],
    "GameCore_XP2_FinalRelease.dll": [(0x590440, 0x5904bb),
        (0x5a1a40, 0x5a1a5b), (0x5a1d30, 0x5a1dbc), (0x5bdac0, 0x5bdd80)],
}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, default=Path("../c6-assets"))
    parser.add_argument("--private-output", type=Path, default=Path("../c6-ghidra/evidence/script"))
    parser.add_argument("--report", type=Path, default=Path("evidence/bootstrap/script-binding.json"))
    args = parser.parse_args()
    paths = {"CivilizationVI.exe": args.assets / "Base/Binaries/Win64Steam/CivilizationVI.exe",
             "GameCore_XP2_FinalRelease.dll": args.assets / "DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll"}
    blobs = {name: path.read_bytes() for name, path in paths.items()}
    for name, blob in blobs.items():
        if hashlib.sha256(blob).hexdigest() != IDENTITIES[name]:
            raise SystemExit("unsupported target identity: " + name)
    core = blobs["GameCore_XP2_FinalRelease.dll"]
    slots = struct.unpack("<9Q", at_rva(core, 0xa126c8, 72))
    expected = [0x5a1d30, 0x8972d0, 0x5a1da0, 0x5a1db0, 0x5a1c50,
                0x5a1c90, 0x5a1ce0, 0x5a1a60, 0x5a1ab0]
    base = pe_fields(core)[0]
    if [slot - base for slot in slots] != expected:
        raise SystemExit("script utility vtable mismatch")
    args.private_output.mkdir(parents=True, exist_ok=True)
    for name, path in paths.items():
        image_base = pe_fields(blobs[name])[0]
        with (args.private_output / (name + ".binding.asm")).open("w") as output:
            for start, end in RANGES[name]:
                subprocess.run(["objdump", "-d", "-M", "intel",
                    f"--start-address={image_base + start:#x}",
                    f"--stop-address={image_base + end:#x}", str(path)], stdout=output, check=True)
    report = {
        "schema": "c6-native-script-binding-static-evidence-v1",
        "identities": {name: IDENTITIES[name] for name in paths},
        "validation": "fingerprinted_static_instructions_and_vtable_only",
        "gamecore": {"context_getter_slot": "0x68", "context_getter_rva": "0x590440",
            "static_utility_rva": "0xb8f0e0", "utility_vtable_rva": "0xa126c8",
            "utility_slot_rvas": {hex(i * 8): hex(rva) for i, rva in enumerate(expected)},
            "register_abi": "void(self, genuine_havok_state)",
            "register_native_implementation_rva": "0x5bdac0"},
        "host": {"gamecore_registration_callback_rva": "0x1614a0",
            "protected_dispatch_caller_rva": "0x161160",
            "context_provider_rva": "0xf8430", "cached_context_global_rva": "0x22d60c0",
            "context_factory_global_rva": "0x22d64e8", "native_module_loader_rva": "0xf8510",
            "gameinfo_callback_rva": "0x2a3440",
            "gameinfo_database_call_rva": "0x2a3504", "app_global_rva": "0x12c2c88",
            "gameinfo_database_getter_rva": "0x64660",
            "app_gameplay_database_field": "0x54d8",
            "module_loader_factory_symbol": at_rva(blobs["CivilizationVI.exe"], 0xfba318, 24).split(b"\0")[0].decode("ascii")},
        "limits": ["No original script-library registration executed by this extractor.",
            "No host cached-context or factory global overwritten.",
            "No VM, game initialization, action or turn equivalence established.",
            "Native host VM creation registers host libraries before GameCore library callback use; app singleton prerequisites remain separate."]}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"report": str(args.report), "private_output": str(args.private_output), "vtable_slots_verified": len(expected)}))

if __name__ == "__main__":
    main()
