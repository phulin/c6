#!/usr/bin/env python3
"""Independent offline review of the frozen consuming-World placement trial.

No original execution/build. Private artifacts and negative-case diagnostics
stay in --work-dir. Native answers are opened after candidate execution.
"""
import argparse
import collections
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
COMMIT = "70ff4a1868a4b43c66d7a7aa6e261fbc3517709c"
BINARY = "e24e0a65efa1340179934234f25069c93be524bf71f52e52620ed9706c2fbdee"
ARCHIVE = "55938bb8b89aa908fa6f5abdfcdcae1b92f6c61aeb247948fe3e47c18b725fe5"
EXPECTED = "bc65088165f1b57473738f6ec5f784c86b838060d3c8f2c87a8bebcd5240c882"
CORE = "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884"
GAME = "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
HOST = "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4"
PLACEMENT_FACTS = "b4922e0312310664d7ba59034d32b3118ab3ea78cdad45321926fa73e9cefc23"
MANAGER_FACTS = "3fd5b6e64b5f81e06fc06187b9dae53691ddb536a6fc0b03f6f728d759dea585"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def parse(data):
    def unique(items):
        result = {}
        for key, value in items:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result
    return json.loads(data, object_pairs_hook=unique)


def compare(actual, expected):
    require(actual["schema"] == "c6-loaded-placement-results-v1", "schema")
    require(actual["candidate_sha256"] == BINARY, "candidate")
    require(actual["source_core_sha256"] == CORE, "Core")
    require(actual["decode_role"] == "ReadInputCore", "Read role")
    require(actual["snapshot_source_role"] == "SavedOutputCore", "observed role")
    witness = actual["observed_snapshot"]
    require(witness["scope"] == "associated_saved_snapshot_diagnostic"
            and witness["snapshot_sha256"] == CORE
            and witness["observed_controls_source_role"] == "SavedOutputCore"
            and witness["decode_role_unchanged"] is True
            and witness["native_read_process_controls_claimed"] is False
            and witness["native3_live_state_acceptance"] is False, "snapshot provenance")
    require(witness["mutable_source_facts"] == {"placement": PLACEMENT_FACTS, "manager": MANAGER_FACTS}
            and actual["snapshot_facts_sha256"] == PLACEMENT_FACTS, "mutable fact attribution")
    require(actual["world_revision"] == 0, "mutation")
    for field in ["expected_answers_used_as_inputs", "native_execution", "full_action_parity", "native_read_equivalence"]:
        require(actual[field] is False, "scope " + field)
    require(expected[0]["schema"] == "c6-linux-placement-count-expected-v1"
            and expected[0]["answers_are_inputs"] is False, "native header")
    require(expected[-1] == {"kind": "summary", "status": 0}, "native footer")
    for field, native_kind, kind, size in [
        ("count_results", "count_expected", "count_result", 17584),
        ("unit_binding_results", "unit_binding_expected", "unit_binding_result", 1262),
    ]:
        original = [dict(row, kind=kind) for row in expected if row["kind"] == native_kind]
        require(len(original) == len(actual[field]) == size, "coverage " + field)
        require(original == actual[field], "ordered exact rows " + field)
    groups = collections.defaultdict(list)
    for row in actual["count_results"]:
        groups[(row["member_kind"], row["owner"], row["ordinal"], row["id"])].append(row["direction"])
        if row["plot_present"]:
            require(row["bool_argument"] is False, "native bool")
        else:
            require(not any(k in row for k in ["count", "plot", "bool_argument"]), "null plot fabricated")
    require(len(groups) == 1262 + 1250, "subject coverage")
    require(all(directions == list(range(-1, 6)) for directions in groups.values()), "direction schedule")


def overlay(source, destination, omit=()):
    destination.mkdir()
    for path in source.iterdir():
        if path.name not in omit:
            (destination / path.name).symlink_to(path.resolve(), target_is_directory=path.is_dir())


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--work-dir", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    work = a.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=False)
    freeze_dir = ROOT / "target/frozen/loaded-placement" / COMMIT
    freeze = parse((freeze_dir / "freeze.json").read_bytes())
    require(freeze["source_commit"] == COMMIT and freeze["source_files"] == 1553, "freeze source")
    require(freeze["candidate_sha256"] == BINARY and freeze["source_archive_sha256"] == ARCHIVE, "freeze hashes")
    binary = Path(freeze["candidate_path"])
    archive = Path(freeze["source_archive"])
    require(sha(binary.read_bytes()) == BINARY and sha(archive.read_bytes()) == ARCHIVE, "frozen artifacts")
    require(sha(subprocess.check_output(["git", "archive", "--format=tar", COMMIT], cwd=ROOT)) == ARCHIVE,
            "archive differs from committed tree")
    manifest = parse((freeze_dir / "source-manifest.json").read_bytes())
    require(manifest["source_commit"] == COMMIT and len(manifest["files"]) == 1553, "source manifest")
    with tarfile.open(archive) as tar:
        files = {member.name: sha(tar.extractfile(member).read()) for member in tar if member.isfile()}
        require(files == manifest["files"], "all archived sources")
        checker_bytes = tar.extractfile("tools/linux/loaded_placement_results.py").read()
        resource_source = tar.extractfile("crates/c6-data/src/loaded_actions/resources.rs").read()
        candidate_source = tar.extractfile("crates/c6-differential/src/bin/c6-loaded-placement-candidate.rs").read()
    require(sha(checker_bytes) == freeze["checker_sha256"], "checker")
    require(b"expected-counts" not in resource_source and b"expected-counts" not in candidate_source,
            "expected answer read in candidate resources")
    private = ROOT.parent / "c6-ghidra/evidence"
    core_manifest = private / "linux-military-initialization/native-3/candidate-paths-v2.json"
    runtime = core_manifest.parent / "runtime.json"
    placement = private / "linux-placement-metadata/native-1"
    manager = private / "linux-unit-operation-manager-metadata/native-1"
    require(file_sha(placement / "facts.jsonl") == PLACEMENT_FACTS
            and file_sha(manager / "facts.jsonl") == MANAGER_FACTS, "original facts")
    input_paths = parse(core_manifest.read_bytes())
    require(file_sha(core_manifest.parent / input_paths["target_module"]) == GAME
            and file_sha(core_manifest.parent / input_paths["target_host"]) == HOST, "original target")
    for name in ["before.bin", "control.bin", "after.bin", "after-repeat.bin", "save.bin", "repeat.bin"]:
        require(file_sha(placement / name) == CORE, "associated raw snapshot " + name)
    checker = work / "frozen-checker.py"
    checker.write_bytes(checker_bytes)
    base_command = [str(binary), str(core_manifest), str(runtime)]
    def run_candidate(name, directory):
        result = subprocess.run(base_command + [str(directory), str(manager)], capture_output=True, timeout=300)
        (work / (name + ".stdout")).write_bytes(result.stdout)
        (work / (name + ".stderr")).write_bytes(result.stderr)
        return result
    # This process has not opened the native expected file at this point.
    run = run_candidate("actual", placement)
    require(run.returncode == 0, "candidate failed")
    actual = parse(run.stdout)
    expected_bytes = (placement / "expected-counts.jsonl").read_bytes()
    require(sha(expected_bytes) == EXPECTED, "native expected identity")
    expected = [parse(line) for line in expected_bytes.splitlines()]
    compare(actual, expected)
    historical = parse((ROOT / "evidence/linux/placement-count-results-review.json").read_bytes())
    require(historical["expected_sha256"] == EXPECTED and historical["every_raw_unit_and_formation_membership_identity_matches_ff31_owner_trace"] is True,
            "previous original capture attribution")
    negative = {}
    for name, change in [
        ("count_value", lambda d: d["count_results"][0].__setitem__("count", 123456)),
        ("count_order", lambda d: d["count_results"].reverse()),
        ("count_coverage", lambda d: d["count_results"].pop()),
        ("binding_identity", lambda d: d["unit_binding_results"][0].__setitem__("formation_id", 123456)),
        ("decode_role", lambda d: d.__setitem__("decode_role", "SavedOutputCore")),
        ("snapshot_role", lambda d: d.__setitem__("snapshot_source_role", "ReadInputCore")),
        ("native_read_control_claim", lambda d: d["observed_snapshot"].__setitem__("native_read_process_controls_claimed", True)),
        ("world_revision", lambda d: d.__setitem__("world_revision", 1)),
        ("acceptance_scope", lambda d: d.__setitem__("native_read_equivalence", True)),
    ]:
        corrupt = copy.deepcopy(actual)
        change(corrupt)
        try:
            compare(corrupt, expected)
        except ValueError:
            negative[name] = "rejected"
        else:
            raise ValueError("accepted corruption: " + name)
    no_answers = work / "placement-no-answers"
    overlay(placement, no_answers, ["expected-counts.jsonl"])
    absent_run = run_candidate("no-native-answers", no_answers)
    require(absent_run.returncode == 0, "candidate requires native expected answers")
    compare(parse(absent_run.stdout), expected)
    negative["native_answers_absent_candidate_unchanged"] = "passed"
    # Now install a corrupt answer file: candidate must still run, but the
    # frozen author's checker must reject the pinned answer identity afterward.
    (no_answers / "expected-counts.jsonl").write_bytes(expected_bytes + b"\n")
    checked = subprocess.run(["python3", str(checker), "--freeze", str(freeze_dir / "freeze.json"),
        "--manifest", str(core_manifest), "--target-runtime", str(runtime), "--placement", str(no_answers),
        "--manager", str(manager), "--diagnostics", str(work / "corrupt-answer-diagnostics"),
        "--output", str(work / "corrupt-answer-result.json")], capture_output=True, timeout=300)
    (work / "corrupt-answer-checker.stderr").write_bytes(checked.stderr)
    require(checked.returncode != 0 and b"native expected answers changed" in checked.stderr,
            "frozen checker accepted corrupt native answers")
    require((work / "corrupt-answer-diagnostics/stdout.json").stat().st_size > 0,
            "checker did not run candidate before rejecting answers")
    negative["frozen_checker_native_expected_hash"] = "rejected"
    bad_facts = work / "placement-corrupt-facts"
    overlay(placement, bad_facts, ["facts.jsonl"])
    (bad_facts / "facts.jsonl").write_bytes((placement / "facts.jsonl").read_bytes() + b"\n")
    facts_run = run_candidate("corrupt-facts", bad_facts)
    require(facts_run.returncode != 0 and not facts_run.stdout, "candidate accepted corrupt facts")
    require(b"closed artifact hash" in facts_run.stderr, "unexpected corrupt facts failure")
    negative["candidate_resource_facts_hash"] = "rejected"
    rows = actual["count_results"]
    result = dict(schema="c6-loaded-placement-independent-review-v1", accepted=True,
        source_commit=COMMIT, source_archive_sha256=ARCHIVE, source_files_checked=1553,
        reviewer_source_sha256=file_sha(Path(__file__)),
        target_module_sha256=GAME, target_host_sha256=HOST,
        placement_facts_sha256=PLACEMENT_FACTS, manager_facts_sha256=MANAGER_FACTS,
        full_matching_associated_snapshot_images=6,
        candidate_sha256=BINARY, checker_sha256=sha(checker_bytes), native_expected_sha256=EXPECTED,
        input_manifest_sha256=sha(core_manifest.read_bytes()), target_runtime_sha256=sha(runtime.read_bytes()),
        source_core_sha256=CORE, decode_role=actual["decode_role"], snapshot_source_role=actual["snapshot_source_role"],
        exact_ordered_count_rows=len(rows), exact_ordered_binding_rows=len(actual["unit_binding_results"]),
        null_plot_rows=sum(not row["plot_present"] for row in rows),
        count_histogram=dict(collections.Counter(str(row["count"]) for row in rows if row["plot_present"])),
        layer_histogram=dict(collections.Counter(str(row["layer"]) for row in rows)),
        original_capture_attribution="pinned expected identity previously independently reviewed; no new original run",
        expected_opened_after_candidate=True, actual_output_sha256=sha(run.stdout),
        negative_checks=negative, world_revision=actual["world_revision"],
        diagnostic_directory=str(work), native_execution=False, native_read_equivalence=False,
        full_action_or_rng_or_turn_parity=False, historical_raw_save_load_difference_retained=89198)
    a.output.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
    print(json.dumps(result, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
