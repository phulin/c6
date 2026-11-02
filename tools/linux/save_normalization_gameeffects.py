#!/usr/bin/env python3
"""Audit the pinned Linux fixture's GameEffects ID differences without masking.

This is a bounded wire/evidence reader, not a game loader or an equivalence test.
Offsets come from the original forwarding serializer owner trace. It parses the
entire RequirementSystem v3 section and only the Team suffix of the object pool.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct


MODULE_SHA = "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
INITIAL_SHA = "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884"
RELOADED_SHA = "85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8"
INSTANCE_START = 73314265
REQUIREMENT_START = 73741108
REQUIREMENT_END = 74930233
INSTANCE_END = 76388443
TEAM_START = 73740688
TEAM_GUID = bytes.fromhex("693155cebc1a9341971200e179548dea")
FUNCTIONS = {
    "Instance_Write": (0x23F4B20, 0x244),
    "Instance_Read": (0x23F5E70, 0x35A),
    "RequirementSystem_Write": (0x23F54C0, 0x284),
    "RequirementSystem_Read": (0x23F7D20, 0x2B5),
    "Team_Write": (0x23EFE00, 0x3E),
    "Team_Read": (0x23EFE40, 0x156),
    "Team_MakeObject": (0x233EF40, 0x1F),
    "Team_DefaultMakeObject": (0x242DE20, 0xAB),
    "ObjectPool_SetNextInstanceId": (0x23350D0, 3),
    "ObjectPool_Share": (0x23353A0, 0x182),
    "ObjectPool_Append": (0x2335530, 0x235),
    "ModifierObject_GetInstanceId": (0x2335940, 4),
    "ModifierObject_SetInstanceId": (0x2335950, 4),
    "VictoryManager_Read": (0x2A31870, 0x22B),
    "VictoryManager_InitializePlayer": (0x2A32420, 0x8DB),
    "Game_Instance_Read": (0x2D7AAA0, 0x9EC),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def function_hashes(module):
    require(module[:6] == b"\x7fELF\x02\x01", "expected little-endian ELF64")
    phoff = struct.unpack_from("<Q", module, 32)[0]
    size, count = struct.unpack_from("<HH", module, 54)
    segments = [struct.unpack_from("<IIQQQQQQ", module, phoff + i * size)
                for i in range(count)]
    out = {}
    for name, (rva, length) in FUNCTIONS.items():
        segment = next(s for s in segments if s[0] == 1 and s[3] <= rva
                       and rva + length <= s[3] + s[5])
        offset = segment[2] + rva - segment[3]
        out[name] = {"rva": hex(rva), "bytes": length,
                     "sha256": sha(module[offset:offset + length])}
    return out


def requirements(data):
    require(data[REQUIREMENT_START] == 3, "unsupported requirement version")
    count = struct.unpack_from("<I", data, REQUIREMENT_START + 1)[0]
    pos = REQUIREMENT_START + 5
    out = []
    for index in range(count):
        length = struct.unpack_from("<I", data, pos)[0]
        require(pos + 4 + length + 9 <= REQUIREMENT_END, "record exceeds section")
        name = data[pos + 4:pos + 4 + length].decode("ascii")
        fields = pos + 4 + length
        context, subject, state = struct.unpack_from("<iiB", data, fields)
        out.append({"index": index, "offset": pos, "requirementId": name,
                    "contextId_offset": fields, "contextId": context,
                    "subjectId_offset": fields + 4, "subjectId": subject,
                    "state_offset": fields + 8, "state": state})
        pos = fields + 9
    require(pos == REQUIREMENT_END, "requirement section not fully consumed")
    return out


def teams(data):
    out = []
    for pos in range(TEAM_START, REQUIREMENT_START, 30):
        instance, present, version = struct.unpack_from("<iBB", data, pos)
        require((present, version) == (1, 1), "unexpected object wrapper")
        require(data[pos + 6:pos + 22] == TEAM_GUID, "unexpected Team type GUID")
        payload_version, team = struct.unpack_from("<Ii", data, pos + 22)
        require(payload_version == 1, "unsupported Team payload version")
        out.append({"instanceId_offset": pos, "instanceId": instance,
                    "team_offset": pos + 26, "eTeam": team})
    require(TEAM_START + len(out) * 30 == REQUIREMENT_START, "Team suffix boundary")
    return out


def audit(module, initial, reloaded):
    require(sha(module) == MODULE_SHA, "module identity mismatch")
    require(sha(initial) == INITIAL_SHA, "initial fixture identity mismatch")
    require(sha(reloaded) == RELOADED_SHA, "reloaded fixture identity mismatch")
    before, after = requirements(initial), requirements(reloaded)
    require(len(before) == len(after), "requirement count changed")
    for data in (initial, reloaded):
        require(data[INSTANCE_START:INSTANCE_START + 16]
                == b"\x0c\0\0\0GAME_EFFECTS", "Instance guard mismatch")
        require(struct.unpack_from("<BI", data, INSTANCE_START + 16) == (4, 10211),
                "Instance version/count mismatch")
    old_teams, new_teams = teams(initial), teams(reloaded)
    id_map = {old["instanceId"]: new["instanceId"]
              for old, new in zip(old_teams, new_teams)}
    require(len(id_map) == len(old_teams)
            and len(set(id_map.values())) == len(old_teams), "non-bijective observed ID map")
    changed = [i for i in range(INSTANCE_START, INSTANCE_END)
               if initial[i] != reloaded[i]]
    accounted = set()
    remap = []
    ids = {t["instanceId"] for t in old_teams}
    refs = []
    for old, new in zip(old_teams, new_teams):
        require(old["eTeam"] == new["eTeam"], "Team stable key changed")
        pos = old["instanceId_offset"]
        accounted.update(i for i in range(pos, pos + 4) if initial[i] != reloaded[i])
        remap.append({"eTeam": old["eTeam"], "offset": pos,
                      "initial_instanceId": old["instanceId"],
                      "reloaded_instanceId": new["instanceId"]})
    for old, new in zip(before, after):
        require(old["requirementId"] == new["requirementId"], "definition changed")
        require(old["state"] == new["state"], "state changed")
        for field in ("contextId", "subjectId"):
            if old[field] in ids:
                require(new[field] == id_map[old[field]], "Team reference remap mismatch")
                require(old["requirementId"] == "DEFAULT_VICTORY_REQUIREMENT"
                        and old["state"] == 1, "unexpected affected requirement record")
                refs.append({"record_index": old["index"],
                             "requirementId": old["requirementId"], "field": field,
                             "offset": old[field + "_offset"],
                             "initial": old[field], "reloaded": new[field],
                             "state": old["state"]})
            if old[field] != new[field]:
                require(old[field] in ids, "changed reference outside Team ID map")
                pos = old[field + "_offset"]
                accounted.update(i for i in range(pos, pos + 4)
                                 if initial[i] != reloaded[i])
    require(set(changed) == accounted and len(changed) == 42, "unaccounted changes")
    require(len(refs) == 28, "unexpected reference count")
    return {
        "schema": "c6-linux-save-normalization-gameeffects-v1",
        "module_sha256": MODULE_SHA, "initial_sha256": INITIAL_SHA,
        "reloaded_sha256": RELOADED_SHA, "bytes": len(initial),
        "raw_save_load_different_bytes": sum(a != b for a, b in zip(initial, reloaded)),
        "gameeffects_different_bytes": len(changed), "changed_offsets": changed,
        "native_functions": function_hashes(module),
        "boundary_evidence": {
            "owner_map_benchmark_2_full_sha256": "91f6e2dd80878e11cf9c05b6a103ce04b244d3714fa56dec5b8dc63dceb134a6",
            "trace_benchmark_2_sha256": "8d0c31395b4180872adf015775f4f14104dcbbcaad93309fbad0f4040f0e2598",
            "Instance_start": INSTANCE_START, "Instance_end": INSTANCE_END,
            "Instance_version": 4, "Instance_object_count": 10211,
            "Team_suffix_start": TEAM_START,
        },
        "wire_contract": {
            "Instance": "guard string, u8 version, u32 object count; each object: i32 GetInstanceId(), shared IModifierObject payload; RequirementSystem; ModifierSystem; guard string",
            "Team_suffix": "i32 instanceId; bool present=1; u8 wrapper version=1; 16-byte type GUID; u32 Team version=1; i32 eTeam",
            "RequirementSystem": "u8 version=3; u32 count; each record: u32 string byte length, requirementId bytes, i32 contextId, i32 subjectId, u8 RequirementState",
            "reference_names": "Matching target DWARF locals contextId, subjectId, requirementId, state at 0x23f7df5; native writer uses GetContext and GetSubject IDs.",
        },
        "requirements": {"version": 3, "count": len(before),
                         "start": REQUIREMENT_START, "end": REQUIREMENT_END,
                         "fully_consumed": True},
        "team_guid_wire_hex": TEAM_GUID.hex(), "team_id_remap": remap,
        "all_requirement_section_references_to_initial_team_ids": refs,
        "native_load_contract": [
            "Instance Read sets pool m_NextInstanceId to each serialized i32 before reading its object (0x23f5fb3..0x23f5fcd). Setter directly assigns pool+0 (0x23350d0).",
            "Team Read restores eTeam then MakeObject(eTeam) if present in WasEverAlive player list (0x23efe40..0x23eff96). MakeObject calls DefaultMakeObject then pool Share; Share reuses equal live objects or Append assigns current next ID and increments it.",
            "RequirementSystem Read resolves contextId then subjectId with ObjectPool::FindObject; GetRequirement(context,subject,requirementId), SetState(state), AddTemporaryRequirement preserve them while modifiers load (0x23f7d20).",
            "Instance Read calls ReleaseTemporaryRequirements (0x23f6085), destroys temporaryObjects strong references (0x23f60b6), then pool Trim (0x23f60d1). Pool entries themselves hold weak_ptrs.",
            "Game Read loads GameEffects (0x2d7af5b), then VictoryManager (0x2d7afb2). VictoryManager Read destroys its existing victory requirement list (0x2a31a09..0x2a31a80).",
            "Game Read itself later calls VictoryManager InitializeGlobalDefeats (0x2d7b307), then InitializePlayer (0x2d7b351); InitializePlayer calls MakeObject(eTeam) (0x2a3298c), GetRequirementSet(team,team,definition) (0x2a32a8d).",
        ],
        "allocation_counters": {
            "ObjectPool_m_NextInstanceId_offset": 0,
            "ObjectPool_m_NextTemporaryInstanceId_offset": 56,
            "persistence": "Instance Write does not serialize either final allocator counter separately. It serializes each live object's ID. Read assigns next permanent counter before each object. Neither counter's final runtime value is captured by these save bytes.",
        },
        "conclusion": "The 42 changed bytes are exact persistent Team instance IDs and their requirement context/subject references. Original Game Read includes post-load victory reconstruction and permits release/recreation of these objects. A missing external post-load finalization call is not established. Exact runtime allocation/reuse/destruction sequence producing +24 has not been observed.",
        "limits": [
            "This parses all RequirementSystem records and all 14 Team suffix records, not all object variants or all serialized references in other sections. No claim of a complete global reference census.",
            "All changed GameEffects bytes are accounted for without altering either stream. Raw 89,198-byte failure remains; no bytes are masked or normalized.",
            "Stable eTeam keys and matching requirement state do not prove IDs semantically interchangeable or future behavior equivalent. Ordering, allocation counters, ties and external observers remain in scope.",
            "No original-code execution, allocation-stage observer, native boundary fixture, simulator implementation or complete state/action/turn equivalence is supplied by this audit.",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--module", type=Path, required=True)
    parser.add_argument("--initial", type=Path, required=True)
    parser.add_argument("--reloaded", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = audit(args.module.read_bytes(), args.initial.read_bytes(), args.reloaded.read_bytes())
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"output": str(args.output), "different_bytes": report["gameeffects_different_bytes"]}))


if __name__ == "__main__":
    main()
