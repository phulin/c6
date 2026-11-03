#!/usr/bin/env python3
"""Pin read-only reporting-provider replay checkpoints; never execute original code."""
import argparse
from collections import Counter
import hashlib
import json
import re
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
    ("game", "Event.SendVectorMap", 0x2d4a500, 0x8f),
    ("game", "Cache.StaticThunk", 0x2708590, 0x1f),
    ("game", "Cache.StaticFunctionGetter", 0x27085f0, 5),
    ("game", "Cache.StaticFunctionCast", 0x2708600, 4),
    ("game", "Cache.Unit.OnUpdate", 0x27ff430, 9),
    ("game", "Cache.Unit.PublishUpdate", 0x27ff860, 0x72),
    ("game", "Cache.Unit.Add", 0x27ff2d0, 0xde),
    ("game", "YieldMonitor.OnMessagesProcessed", 0x2c66430, 0x20),
    ("game", "YieldMonitor.CheckPlot", 0x2c665f0, 0x14d),
    ("game", "YieldMonitor.CheckTrade", 0x2c668b0, 0x52),
    ("host", "Translate.UnitActivityChanged", 0x320b670, 0x1d0),
    ("host", "Translate.OperationAdded", 0x320cb30, 0x7b),
    ("host", "Translate.OperationStarted", 0x320cbb0, 0x7b),
    ("host", "Translate.OperationDeactivated", 0x320ccb0, 0x7b),
    ("host", "Translate.OperationsCleared", 0x320cd30, 0x7b),
    ("host", "Translate.NotificationActivated", 0x3212d00, 0xa4),
    ("host", "DirtyComponents.Add", 0x32b5950, 0x56),
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
        ("game", 0x2708600, "4889f8c3"),
        ("game", 0x2d4a521, "84c0754a"),
        ("host", 0x3228600, "488b4708488b5710c3"),
    ]
    for image, address, expected in gates + [(i, a, b) for i, a, b, _, _ in CHECKPOINTS]:
        if images[image].read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError(f"original byte gate differs: {image}+{address:#x}")
    return dict(
        schema="c6-linux-reporting-provider-static-v2", checks_passed=True,
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
            limits=["Executed vector-map overload bypasses chosen map checkpoint; actual flag values were not captured",
                    "Disabled reporting path misses suspended-return checkpoint",
                    "Missing translator registration misses registered-translator checkpoint",
                    "Pre-publish subscriber pointer proves registration, not actual callback invocation",
                    "Raw delegate resolution requires exact target ABI and does not classify transitive writes",
                    "Hardware stops change scheduling; replay must compare command, streams, typed state and cleanup",
                    "Loader instrumentation must be reported separately"]),
        kernel_admission=dict(complete_reporting_effects_proven=False, accept_live_action=False,
            requirement="Bind actual enabled/suspended/cache flags, invoked cache handler targets and receivers, translator identities, pre-publish subscribers and all transitive effects; reject unsupported environments before mutation."),
        full_host_admission=False, gameplay_parity=False,
        limits="Static plan for one conditional Linux component replay. Complete gameplay state/action/turns remain unimplemented. Unserialized live state and complete RNG remain unresolved; the raw Save-to-Load 89198-byte failure remains unclassified and unmasked.")


def file_digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def passive_summary(directory):
    metadata_path = directory / "metadata.json"
    observations_path = directory / "observations.jsonl"
    metadata = json.loads(metadata_path.read_text())
    validation_path = directory / "validation.json"
    validation = json.loads(validation_path.read_text())
    if validation["runtime_sha256"] != file_digest(directory / "runtime.json"):
        raise ValueError("passive validation runtime identity differs")
    if not validation["all_five_save_captures_and_command_match_parent"]:
        raise ValueError("passive parent comparison failed")
    if metadata["parent_configuration_sha256"] != CONFIGURATION:
        raise ValueError("passive replay parent configuration differs")
    if metadata["observations_sha256"] != file_digest(observations_path):
        raise ValueError("passive observations digest differs")
    if metadata["gdb_returncode"] != 0 or metadata["inferior_exit"]["inferior_exitcode"] != 0:
        raise ValueError("passive replay did not close successfully")
    rows = [json.loads(line) for line in observations_path.read_text().splitlines()]
    if any(row.get("checkpoint_bytes_match") is False or row.get("read_error") for row in rows):
        raise ValueError("passive checkpoint/read failure")
    selected = [r for r in rows if r.get("trace_armed") and r.get("trace_phase") == 5]
    caches = [r for r in selected if r["stage"] == "cache_delegate_tail"]
    translations = [r for r in selected if r["stage"] == "host_translation_pointer"]
    publishes = [r for r in selected if r["stage"] == "host_pre_publish_getter"]
    if (len(caches), len(translations), len(publishes)) != (8, 10, 17):
        raise ValueError("selected replay checkpoint counts differ")
    if any(not r["caller_is_audited_cache_dispatch"] for r in caches):
        raise ValueError("cache tail caller differs")
    base = int(caches[0]["game_base"], 16)
    observed_cache = [(int(r["resolved_callback"]["pointer"], 16) - base,
                       int(r["adjusted_receiver"], 16) - base) for r in caches]
    if observed_cache[:7] != [(0x2708590, 0x27ff430)] * 7 or observed_cache[7][0] != 0x2c66430:
        raise ValueError("actual selected callback identity differs")
    expected_translators = Counter({"0x320b670": 3, "0x320ccb0": 2, "0x320cd30": 2,
                                   "0x320cb30": 1, "0x320cbb0": 1, "0x3212d00": 1})
    if Counter(r["registered_translator"]["pointer"] for r in translations) != expected_translators:
        raise ValueError("actual selected translator identity differs")
    if any(int(r["pre_publish_subscriber"], 16) != 0 for r in publishes):
        raise ValueError("selected pre-publish registration changed")
    event_classes = Counter(re.search(r"GameCoreEvent_Template<(\d+)u", r["caller"]["symbol"]).group(1)
                           for r in publishes)
    save_rows = []
    for name, expected in [(name, "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884")
                           for name in ("before.bin", "save.bin")] + [
            (name, "be08f44a4b83bd7eaa8d15a24443731764705c32961c570697bc3bb1736097bc")
            for name in ("after.bin", "after-repeat.bin", "repeat.bin")]:
        actual = file_digest(directory / name)
        if actual != expected:
            raise ValueError("passive Save identity differs: " + name)
        save_rows.append(dict(name=name, sha256=actual, bytes=(directory / name).stat().st_size))
    if file_digest(directory / "command.bin") != "5c262073a0b62b252c10ca562ca765faff535bc0e6bdf78c4f41f331498f288a":
        raise ValueError("passive command differs")
    return dict(schema="c6-linux-reporting-provider-passive-classification-v1",
        native_execution_observed=True, native_exit=0, gdb_exit=0, timing_equivalent=False,
        metadata_sha256=file_digest(metadata_path), observations_sha256=file_digest(observations_path),
        validation_sha256=file_digest(validation_path), runtime_sha256=validation["runtime_sha256"],
        total_hits=len(rows), selected_phase=5, selected_armed=True, selected_hits=len(selected),
        selected_counts=dict(Counter(r["stage"] for r in selected)),
        direct_flag_observation=False, missing_map_checkpoint_hits=True,
        derived_branch_facts="Original vector-map sender gates require enabled=true and suspended=false on paths that reached cache dispatch/provider; invoked cache delegates require cache updates enabled. This is path evidence, not a direct flag capture.",
        cache_callbacks=[dict(sequence=r["sequence"], event_hash=hex(r["event_hash"]),
            target=r["resolved_callback"], adjusted_receiver=r["adjusted_receiver"],
            continuation_rva="0x27ff430" if n < 7 else "0x2c66430",
            classification="Unit snapshot cache allocation plus host CacheUpdated event275" if n < 7 else
                           "Conditional yield-monitor cache comparison/update and recursive reporting; current branch and transitive gameplay effects unresolved")
            for n, r in enumerate(caches)],
        translators=[dict(sequence=r["sequence"], event_hash=hex(r["event_hash"]),
                          target=r["registered_translator"], target_elf_virtual_address=r["registered_translator"]["pointer"]) for r in translations],
        prepublish=dict(observed_calls=len(publishes), all_observed_pointers_null=True,
                       host_event_counts=dict(event_classes), universal_absence_proven=False),
        saves=save_rows, command_sha256=file_digest(directory / "command.bin"),
        callback_receiver_state_observed=False, complete_transitive_effects_proven=False,
        authoritative_core_mutation_from_callback_proven=False, pure_reporting_proven=False,
        full_host_admission=False, rust_live_action_admission=False,
        limits="Raw pointer/ASLR values retained. Translator pointer checkpoint precedes the indirect call. Null pre-publish registration is scoped to actual getter hits. Cache snapshot Fill methods, yield-monitor branch/live cache, host dirty registrations and activity-facade virtual calls remain unresolved. No blanket cache/AI/presentation exclusion.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, default=ROOT.parent / "c6-assets-linux")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--passive-replay", type=Path)
    args = parser.parse_args()
    report = audit(args.assets)
    if args.passive_replay:
        report["passive_replay"] = passive_summary(args.passive_replay)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
