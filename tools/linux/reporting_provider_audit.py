#!/usr/bin/env python3
"""Pin read-only reporting-provider replay checkpoints; never execute original code."""
import argparse
import hashlib
import json
from pathlib import Path

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
CONFIGURATION = "b6ca58d53073abc2afa9176a4ceba350fafe3f91ba12dbf84fd08e25a3be9a5b"
CHECKPOINTS = [
    ("game", 0x2d4a4c6, "84c0740c", "suspended_return",
     "AL actual IsSuspended result; EBP event hash; RBX map; read enabled game+3c00649/cache updates game+3bffce8"),
    ("game", 0x2724777, "ffe0", "cache_delegate_tail",
     "RAX resolved callback; RDI adjusted receiver; ESI event; RDX EventData; RCX map; retain stack return and backtrace"),
    ("host", 0x3214cb0, "4885c0740c", "registered_translator",
     "RAX actual translator including null; event u32[RSP+0xc]; R15 EventData; R14 map"),
    ("host", 0x2fd7db0, "488b4710c3", "prepublish_getter",
     "RDI DispatchData; subscriber u64[RDI+0x10]; caller u64[RSP]; raw delegate24 and backtrace when nonnull"),
]
METHODS = [
    ("game", "Event.IsEnabled", 0x2d49180, 7),
    ("game", "Event.IsSuspended", 0x2d49190, 5),
    ("game", "Globals.IsUnserializing", 0x290bb30, 0x2d),
    ("game", "Cache.AreUpdatesEnabled", 0x2723640, 7),
    ("game", "Event.SendMap", 0x2d4a4b0, 0x4f),
    ("game", "Cache.Send", 0x2723550, 0xee),
    ("game", "Cache.FastDelegate3.Invoke", 0x2724730, 0x49),
    ("host", "EngineUtility.SendEvent", 0x32b4260, 0x9bc),
    ("host", "Translate.Dispatch", 0x3214c50, 0x81),
    ("host", "Publish71", 0x32176b0, 0x50),
    ("host", "Publish257", 0x321cac0, 0x50),
    ("host", "PrePublish.Get", 0x2fd7db0, 5),
    ("host", "PrePublish71.Invoke", 0x3228580, 0x37),
    ("host", "PrePublish257.Invoke", 0x3237c40, 0x37),
    ("host", "PrePublish71.GetThis", 0x32285f0, 4),
    ("host", "PrePublish71.GetMember", 0x3228600, 9),
]


def audit(assets):
    images = dict(game=Elf(assets / "libGameCore_XP2.so", GAME_SHA),
                  host=Elf(assets / "Civ6", HOST_SHA))
    gates = [
        ("game", 0x2d49180, "8a05c374eb00c3"),
        ("game", 0x2723640, "8a05a2c64d01c3"),
        ("host", 0x32285f0, "488b07c3"),
        ("host", 0x3228600, "488b4708488b5710c3"),
    ]
    for image, address, expected in gates + [(i, a, b) for i, a, b, _, _ in CHECKPOINTS]:
        if images[image].read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError(f"original byte gate differs: {image}+{address:#x}")
    return dict(
        schema="c6-linux-reporting-provider-static-v1", checks_passed=True,
        static_only=True, native_execution=False, live_callback_inventory_observed=False,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        source_audit_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        parent_configuration_sha256=CONFIGURATION,
        methods=[dict(image=i, name=n, rva=hex(a), bytes=z,
                      sha256=hashlib.sha256(images[i].read(a, z)).hexdigest()) for i, n, a, z in METHODS],
        passive_capture=dict(maximum_simultaneous_hardware_breakpoints=4,
            game_code_patches=False, inferior_calls=False, inferior_field_writes=False,
            checkpoints=[dict(image=i, rva=hex(a), instruction_bytes=b, name=n, capture=c)
                         for i, a, b, n, c in CHECKPOINTS],
            preserve=["Raw callback/receiver/object addresses", "Module ASLR bases and pointer RVAs separately",
                      "Unclassified bytes and cross-process differences", "Read errors and missed checkpoint coverage"],
            limits=["Disabled reporting path misses suspended-return checkpoint",
                    "Missing translator registration misses registered-translator checkpoint",
                    "Pre-publish subscriber pointer proves registration, not actual callback invocation",
                    "Raw delegate resolution requires exact target ABI and does not classify transitive writes",
                    "Hardware stops change scheduling; replay must compare command, streams, typed state and cleanup",
                    "Loader instrumentation must be reported separately"]),
        kernel_admission=dict(complete_reporting_effects_proven=False, accept_live_action=False,
            requirement="Bind actual enabled/suspended/cache flags, invoked cache handler targets and receivers, translator identities, pre-publish subscribers and all transitive effects; reject unsupported environments before mutation."),
        full_host_admission=False, gameplay_parity=False,
        limits="Static plan for one conditional Linux component replay. Complete gameplay state/action/turns remain unimplemented. Unserialized live state and complete RNG remain unresolved; the raw Save-to-Load 89198-byte failure remains unclassified and unmasked.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, default=ROOT.parent / "c6-assets-linux")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = audit(args.assets)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
