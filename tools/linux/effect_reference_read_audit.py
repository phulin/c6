#!/usr/bin/env python3
"""Pin static Linux GameEffects reference-reconstruction evidence.

Never runs game code or rewrites save bytes. Optional canonical fixtures are
checked by the existing typed reader; static findings remain manual instruction
analysis anchored to whole-module identity and exact function byte hashes.
"""

import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct

from save_normalization_gameeffects import MODULE_SHA, INITIAL_SHA, RELOADED_SHA
from save_normalization_gameeffects import FUNCTIONS as CLOSED_FUNCTIONS
from save_normalization_gameeffects import audit as fixture_audit

HOST_SHA = "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4"
EXTRA_FUNCTIONS = {
    "ObjectPool_Constructor": (0x2335030, 0x53),
    "ObjectPool_ClearObjects": (0x23289F0, 0x9D),
    "ObjectPool_Trim": (0x23350E0, 0x156),
    "ObjectPool_FindObject": (0x2335770, 0x68),
    "Team_Equal": (0x23EFA90, 0x5B),
    "Team_IsTemporary": (0x23EFC40, 3),
    "ModifierObject_CacheRequirement": (0x2335D50, 0x121),
    "ModifierObject_GetRequirement": (0x2335FB0, 0xA1),
    "ModifierObject_LookupKeyHasher": (0x2336110, 0x1A),
    "RequirementSystem_GetRequirement": (0x2336A40, 0x467),
    "RequirementSystem_ReleaseTemporaryRequirements": (0x2338BF0, 0x56),
    "RequirementSystem_Constructor": (0x2337D00, 0xC4),
    "VictoryManager_InitializeGlobalDefeats": (0x2A31F00, 0x3FE),
    "VictoryManager_CullRequirements": (0x2A32320, 0xFE),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def function_hashes(module):
    require(module[:6] == b"\x7fELF\x02\x01", "expected ELF64 little endian")
    require(struct.unpack_from("<H", module, 16)[0] == 3, "expected ET_DYN module")
    phoff = struct.unpack_from("<Q", module, 32)[0]
    width, count = struct.unpack_from("<HH", module, 54)
    segments = [struct.unpack_from("<IIQQQQQQ", module, phoff + i * width)
                for i in range(count)]
    functions = dict(CLOSED_FUNCTIONS, **EXTRA_FUNCTIONS)
    out = {}
    for name, (address, length) in functions.items():
        segment = next(s for s in segments if s[0] == 1 and s[1] & 1
                       and s[3] <= address and address + length <= s[3] + s[5])
        offset = segment[2] + address - segment[3]
        out[name] = {"rva": hex(address), "bytes": length,
                     "sha256": sha(module[offset:offset + length])}
    return out


def audit(module, host, closed_bytes):
    require(sha(module) == MODULE_SHA, "module identity mismatch")
    require(sha(host) == HOST_SHA, "host identity mismatch")
    require(host[:6] == b"\x7fELF\x02\x01"
            and struct.unpack_from("<H", host, 16)[0] == 2,
            "expected absolute-VA ET_EXEC Linux host")
    closed = json.loads(closed_bytes)
    require(closed["module_sha256"] == MODULE_SHA
            and closed["initial_sha256"] == INITIAL_SHA
            and closed["reloaded_sha256"] == RELOADED_SHA,
            "closed typed evidence identity mismatch")
    require(closed["gameeffects_different_bytes"] == 42
            and closed["raw_save_load_different_bytes"] == 89198,
            "closed difference accounting changed")
    functions = function_hashes(module)
    for name, expected in closed["native_functions"].items():
        require(functions[name] == expected, "closed function mismatch: " + name)
    return {
        "schema": "c6-linux-effect-reference-read-v1",
        "module_sha256": MODULE_SHA,
        "host_sha256": HOST_SHA,
        "host_address_kind": "ET_EXEC absolute virtual addresses; no host RVA conversion",
        "closed_typed_report_sha256": sha(closed_bytes),
        "native_execution": False,
        "native_functions": functions,
        "fixture_identity": {"initial_sha256": INITIAL_SHA,
                             "reloaded_sha256": RELOADED_SHA,
                             "bytes": closed["bytes"],
                             "raw_save_load_different_bytes": 89198,
                             "gameeffects_different_bytes": 42},
        "static_findings": [
            {"source": "0x2335030; 0x23289f0; 0x23f5f46",
             "finding": "Pool construction seeds permanent next=1 and temporary next=-1; ClearObjects clears both weak index sets without resetting counters. Instance v4 Read begins with Trim, not ClearObjects. Startup live roots and per-type weak-vector order remain relevant."},
            {"source": "0x23353a0; 0x23efa90; 0x23efc40",
             "finding": "Share selects permanent or temporary type map, locks weak entries in vector order and returns the first live equal object. Team is permanent; equality tests GUID and eTeam at object+0x110. A reused object retains its old instance ID; no Append or counter increment occurs."},
            {"source": "0x2335530..0x233574f",
             "finding": "Append selects type vector, may Trim on capacity exhaustion, assigns selected counter via virtual SetInstanceId, appends ID/weak tuple and type weak entry, then increments permanent or decrements temporary counter. Temporary branch rebases rbx by +0x38 at 0x23356a8 before common counter update at 0x2335744."},
            {"source": "0x2335770..0x23357d8",
             "finding": "FindObject(0) returns empty; positive IDs search permanent ID tuples; negative IDs search temporary tuples using negated ID. equal_range returns the first matching entry, then one weak lock; no search of later duplicates if that first entry expired."},
            {"source": "0x23f5fba; 0x23f5fcd; 0x23efee6",
             "finding": "Instance Read seeds permanent next with each serialized ID, then reads the object. Team Read requires eTeam!=-1 and membership in current WasEverAlive players before MakeObject. It does not directly force the returned object's ID or install an arbitrary old-to-new mapping."},
            {"source": "0x23f7df5..0x23f7f45; 0x2336a40..0x2336ac5",
             "finding": "Requirement v3 Read resolves context then subject by exact pool ID. GetRequirement rejects null context/subject or empty definition ID, then queries subject cache with context's instance ID. Reader restores state and temporarily roots a nonnull requirement only when virtual predicate slot+0x78 returns false; true skips both operations."},
            {"source": "0x2335d50; 0x2335fb0; 0x2336110; 0x2336bd8..0x2336c27",
             "finding": "Subject requirement cache key contains context instance ID, definition hash and definition string; the custom hasher consumes ID and hash. Successful new factory construction assigns and increments RequirementSystem's separate counter at system+0, caches requirement and appends its weak list. Cache hits bypass creation and this increment."},
            {"source": "0x2338bf0; 0x23f6085; 0x23f60b6; 0x23f60d1",
             "finding": "ReleaseTemporaryRequirements destroys roots in reverse vector order, then Instance destroys temporary object roots in reverse vector order, then Trim removes expired weak entries while retaining survivor order. Objects retained elsewhere survive; final save IDs do not identify which roots kept them alive."},
            {"source": "0x2d7af5b..0x2d7b351",
             "finding": "After effects, Game Read loads deals, notifications, summary and VictoryManager, restores map-derived data, initializes yield/tech/culture/heroes/analysis, invokes WasEverAlive player postload, rules and barbarian postload, then global defeats and Alive-order victory InitializePlayer. Any nested creation in these stages must be accounted for."},
            {"source": "0x2a32420; 0x2a3298c; 0x2a32a8d",
             "finding": "Victory InitializePlayer depends on WorldBuilder active, player validity/barbarian/free-city/major classification, existing victory entries and definition iteration. It creates/shares player and Team objects and requirements/sets, installs subscriptions and retains successful entries. Team creation order cannot be inferred from serialized Team order alone."},
        ],
        "conditional_allocation_constraint": {
            "last_initial_serialized_team_id": 87494,
            "first_reloaded_serialized_team_id": 87505,
            "observed_team_id_delta": 24,
            "inference": "If last restored Team appends once (next=87495), no later setter/reset occurs, and first recreated Team is a new Append assigned 87505, exactly ten intervening permanent appends are required. None of these antecedents or their callers is established by final save bytes alone."},
        "passive_observation_protocol": {
            "scope": "Capture both known fixtures independently from fresh process startup through Core Load and two Saves, without executing native AI decisions.",
            "events": [
                "Global monotonically sequenced thread/call-parent/stage events for pool constructor, ClearObjects, SetNextInstanceId, Share entry/return, Append entry/return, Trim entry/return and Team destruction; include all object types, not only Team.",
                "Record pool identity, both counter values, requested saved ID/stream offset, actual GUID/eTeam when applicable, object/control-block identity and object instance ID before/after. Record candidate weak-vector order and actual Share selection, including expired entries; preserve every ID exactly.",
                "At Game Read entry, Instance entry, requirements/modifiers entry/return, temporary-root release, Victory Read clear and each postload stage, snapshot weak indices, roots, RequirementSystem counter, requirement/cache identities and ordered Alive/WasEverAlive player/team lists.",
                "Observe RequirementSystem GetRequirement/GetRequirementSet cache hit or factory creation, assigned runtime IDs, actual context/subject IDs, state restoration guard/result, temporary-root release and victory subscriptions. Trace actual last-strong-owner destruction; pointer reuse must use control-block/allocation generation, not bare address equality.",
                "Record exact input/rules/configuration/provider identities and compare all affected state, including complete RNG records and observed RNG access/call counts across reconstruction. Factory/virtual callback and logging/service behavior must remain attributed dependencies."],
            "observer_constraints": [
                "Forward original calls once with untouched arguments, returns and exceptions; do not replace allocations or object/reference IDs.",
                "Use bounded preallocated observer storage. Never call weak_ptr::lock, MakeObject, FindObject, gameplay getters with unknown side effects, or native GameEffects logging solely to inspect state. Observer-created strong references would alter precisely the lifetime being measured.",
                "Retain instrumentation/code hashes and complete trace privately; public report contains digests, typed conclusions and gaps. Reject overflow, missing/nested event mismatch, thread loss or observer changes to known output hashes/cleanup."],
            "acceptance": "Both uninstrumented known Save hashes and successful cleanup must reproduce; every counter write, reuse decision and destruction linking imported IDs to final IDs must be explained. Then derive source-closed Rust reconstruction and compare the full affected state in boundary fixtures, including future allocation/iteration observations."},
        "implementation_decision": "No Rust identity reconstruction is justified yet: native ownership/factory/postload allocation sequence and complete affected-state oracle are missing. A +24 transform, generic provider trait, or stable-key isomorphism would not implement original Load.",
        "limits": [
            "Static findings are reviewed target instruction semantics, not a runtime allocation trace or native differential pass.",
            "Only the existing typed scope covers all 42 changed bytes; this adds no global reference census and excludes no AI-namespaced gameplay state.",
            "ID-bearing cache keys establish a real identity dependency, not a proven future RNG/iteration divergence. Transitive factories/callbacks require further evidence.",
            "Raw 89198-byte failure remains unmasked. Exact state import, full gameplay state/actions/turns and future behavior equivalence remain unimplemented."],
    }


def fixture_bytes(path):
    return gzip.decompress(path.read_bytes()) if path.suffix == ".gz" else path.read_bytes()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--module", type=Path, required=True)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--closed-report", type=Path,
                        default=Path("evidence/linux/save-normalization-gameeffects.json"))
    parser.add_argument("--initial", type=Path)
    parser.add_argument("--reloaded", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(bool(args.initial) == bool(args.reloaded), "supply both canonical fixtures")
    module = args.module.read_bytes()
    report = audit(module, args.host.read_bytes(), args.closed_report.read_bytes())
    if args.initial:
        typed = fixture_audit(module, fixture_bytes(args.initial), fixture_bytes(args.reloaded))
        require(typed == json.loads(args.closed_report.read_bytes()),
                "canonical typed audit differs from closed report")
        report["canonical_fixture_recheck"] = "complete existing typed audit reproduced exactly"
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"output": str(args.output), "native_functions": len(report["native_functions"]),
                      "native_execution": False, "raw_different_bytes": 89198}))


if __name__ == "__main__":
    main()
