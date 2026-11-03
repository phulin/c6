#!/usr/bin/env python3
"""Offline pinned-Linux Hero rebuild source, synthetic kernel and fixture audit.

Never loads or executes original code. Candidates compile only extracted complete
committed source preimages, with immutable content-named artifacts.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import struct
import subprocess
import tarfile

from player_heroes_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
RAW = 'a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa'
SAVED = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE = '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
DATABASE = 'c880adaedcfd6da7808302b3b76bb9cad7d3188b35ae78e780cff7b56e04477e'
METHODS = [
    (0x2655ed0, 14, 'HeroesManager.InitializePostLoad'),
    (0x2655ee0, 219, 'HeroesManager.RebuildValidHeroClasses'),
    (0x2655fc0, 202, 'HeroesManager.ValidatePlayerHeroDiscoveryOrders'),
    (0x2655200, 20, 'HeroesManager.IsActive'),
    (0x26560b0, 137, 'HeroesManager.IsAllowedToUse'),
    (0x269bb60, 44, 'Player.Heroes.IsAllowedToUse'),
    (0x269be80, 45, 'Player.Heroes.IsHeroDiscovered'),
    (0x269c6f0, 475, 'Player.Heroes.RebuildHeroDiscoveryOrder'),
    (0x269c8d0, 38, 'Player.Heroes.GetCurrentEra'),
    (0x269c900, 445, 'Player.Heroes.CouldClaimHeroNow'),
    (0x269cac0, 5, 'discovery_sort'),
    (0x269e060, 114, 'discovery_quick_sort'),
    (0x269e0e0, 176, 'discovery_quick_sort_impl'),
    (0x269e190, 185, 'discovery_insertion_sort'),
    (0x269e450, 10, 'signed_weight_comparator'),
    (0x21febd0, 8, 'HeroClassCollectionBuilder.GetBuilderCountSql'),
    (0x21febe0, 8, 'HeroClassCollectionBuilder.GetBuilderSql'),
    (0x2254960, 285, 'HeroClassCollectionBuilder.BuildCollection'),
    (0x2246d30, 15, 'HeroClassCollectionWithTypeIndex.constructor'),
    (0x2213eb0, 16, 'Definitions.GetHeroClassDefinitions'),
    (0x269b3a0, 283, 'Player.Heroes.Read_without_Reset'),
    (0x269ba50, 270, 'Player.Heroes.Reset'),
    (0x269b9a0, 172, 'Player.Heroes.Initialize'),
    (0x265b700, 34, 'HeroClass.HashToIndex'),
    (0x265b730, 109, 'HeroClass.Find_small_index_before_hash'),
    (0x265b900, 149, 'HeroClass.ReadIndexHashArray_bool_last_write'),
    (0x2953330, 19, 'HeroClass.GetIndex'),
    (0x2959e30, 94, 'HeroClass.GetTypeIndexT'),
    (0x2b4fc2d, 26, 'Player.Read_Heroes_version47_guard'),
]
OWNED_SOURCES = [
    'crates/c6-core/src/hero_discovery_order.rs',
    'tools/linux/hero_discovery_order_check.rs',
    'tools/linux/hero_discovery_order_audit.py',
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(args, **kwargs):
    return subprocess.check_output(args, **kwargs)


def c_string(elf, address):
    return elf.read(address, 512).split(b'\0', 1)[0]


def source_gate(module):
    elf = Elf(module, GAME_SHA)
    require(elf.read(0x2655ed6, 2) == b'\x31\xf6', 'post-load force=false differs')
    elf.call(0x269c815, 0x1fb94d0)
    require(elf.read(0x269c806, 5) == b'\xbe\x14\x00\x00\x00', 'draw bound differs')
    require(elf.read(0x269e450, 10).hex() == '8b47043b46040f9fc0c3', 'signed weight comparator differs')
    require(elf.read(0x269e0fd, 6).hex() == '483de1000000', 'partition threshold differs')
    count = c_string(elf, 0x2e3cb62)
    rows = c_string(elf, 0x2e3cb83)
    require(count == b'SELECT count(*) from HeroClasses', 'class count query differs')
    require(rows.endswith(b'FROM HeroClasses ORDER BY rowid'), 'class builder order differs')
    require(elf.read(0x2246d30, 15).hex() == '0f57c00f1147200f1147100f1107c3', 'fresh class collection differs')
    require(elf.read(0x2b4fc2d, 7).hex() == '837c240c2f7c13', 'Player47 Hero guard differs')
    elf.call(0x2b4fc42, 0x1f5edd0)
    elf.call(0x269b3de, 0x1eb9050)
    elf.call(0x265b952, 0x1e81570)
    elf.call(0x269b9f9, 0x1eb6f80)
    return elf, dict(count_sql_sha256=sha(count), rows_sql_sha256=sha(rows),
                     table='HeroClasses', order='rowid', fresh_collection_pointer_storage_zero=True,
                     zero_rows_append_zero_definitions=True,
                     initialized_runtime_collection_identity_admitted=False)


def candidate(commit):
    pin = run(['git', 'rev-parse', commit+'^{commit}'], cwd=ROOT, text=True).strip()
    for name in OWNED_SOURCES + ['tools/linux/player_heroes_audit.py']:
        require(run(['git', 'show', pin+':'+name], cwd=ROOT) == (ROOT/name).read_bytes(),
                'driver/owned source must equal committed preimage: '+name)
    archive = run(['git', 'archive', '--format=tar', pin], cwd=ROOT)
    location = ROOT/'target/linux/hero-discovery-order'/('candidate-'+sha(archive))
    location.mkdir(parents=True, exist_ok=True)
    path = location/'source.tar'
    if path.exists():
        require(path.read_bytes() == archive, 'immutable archive collision')
    else:
        path.write_bytes(archive)
    source = location/'source'
    if not source.exists():
        source.mkdir()
        with tarfile.open(path) as stream:
            stream.extractall(source, filter='data')
        # A #[path] standalone module changes the child lookup location. This is
        # an exact duplicate of a retained committed preimage, not edited source.
        shutil.copyfile(source/'crates/c6-core/src/rng/native_serialization.rs',
                        source/'crates/c6-core/src/native_serialization.rs')
    rustc = Path(shutil.which('rustc')).resolve()
    compiler = run(['rustc', '-vV'], text=True)
    sysroot = Path(run(['rustc', '--print', 'sysroot'], text=True).strip())
    host = next(line.split(': ', 1)[1] for line in compiler.splitlines() if line.startswith('host: '))
    std = sorted((sysroot/'lib/rustlib'/host/'lib').glob('libstd-*.rlib'))
    require(len(std) == 1, 'ambiguous compiler standard library')
    checker = source/'tools/linux/hero_discovery_order_check.rs'
    exe, test = location/'check', location/'check-tests'
    # Never overwrite a previously retained executable/report.
    for target, flags in [(exe, []), (test, ['--test'])]:
        if not target.exists():
            subprocess.run(['rustc', '--edition=2024', *flags, str(checker), '-o', str(target)], check=True)
    tests = run([str(test)], text=True)
    require('test result: ok.' in tests, 'synthetic/source tests failed')
    test_result = location/'tests.txt'
    if test_result.exists():
        # Rust test scheduling and elapsed time can differ; retain original.
        require('test result: ok.' in test_result.read_text(), 'old test result not passing')
    else:
        test_result.write_text(tests)
    return exe, dict(source_commit=pin, complete_source_archive_sha256=sha(archive),
                     complete_source_archive=str(path.relative_to(ROOT)),
                     extracted_committed_tree=True, concurrent_wip_included=False,
                     generated_source_duplicate=dict(source='crates/c6-core/src/rng/native_serialization.rs',
                         destination='crates/c6-core/src/native_serialization.rs',
                         sha256=sha((source/'crates/c6-core/src/native_serialization.rs').read_bytes())),
                     compiler=compiler, compiler_executable_sha256=sha(rustc.read_bytes()),
                     standard_library_rlib_sha256=sha(std[0].read_bytes()),
                     candidate_sha256=sha(exe.read_bytes()), test_candidate_sha256=sha(test.read_bytes()),
                     tests_result=next(line for line in tests.splitlines() if line.startswith('test result:')))


def saved_fixture(private, executable):
    data = gzip.decompress((private/'evidence/linux-game-load/canonical'/(SAVED+'.bin.gz')).read_bytes())
    trace = gzip.decompress((private/'evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz').read_bytes())
    require(sha(data) == SAVED and sha(trace) == TRACE, 'saved fixture/trace identity differs')
    owners = {r['call']: r for r in map(json.loads, trace.splitlines()) if r.get('event') == 'owner' and r.get('save') == 1}
    cuts = sorted((r for r in owners.values() if r['owner'] == 'GameCore::Player::Quests'), key=lambda r:r['start'])
    require(len(cuts) == 43, 'Player.Quests owner coverage differs')
    observations = []
    payload = executable.parent/'fixture-input.bin'
    for cut in cuts:
        parent = owners[cut['parent']]
        require(parent['owner'] == 'GameCore::Player::Instance', 'wrong Player owner')
        version, count = struct.unpack_from('<II', data, cut['end'])
        require(version == 1, 'WMD grammar differs')
        start = cut['end']+8+8*count
        version, count = struct.unpack_from('<II', data, start)
        require(version == 1, 'Improvements grammar differs')
        start += 12+4*count
        require(start < parent['end'] <= len(data), 'parent extent invalid')
        payload.write_bytes(data[start:parent['end']])
        skip, size, order, discovered, claims = map(int, run([str(executable), str(payload)], text=True).split())
        start += skip
        observations.append(dict(player=struct.unpack_from('<i', data, parent['start']+4)[0],
            range=[start,start+size], record_sha256=sha(data[start:start+size]),
            discovery_order_records=order, discovered_records=discovered, claims_records=claims,
            validate_false_rebuild_required=order == 0))
    payload.unlink()
    return dict(stream_sha256=SAVED, trace_sha256=TRACE, surface='post-load SavedOutput; writer owner cuts only',
                records=observations, zero_orders=sum(r['discovery_order_records'] == 0 for r in observations),
                raw_read_guard_inference=False, native_rebuild_observed=False)


def first_player_fixtures(private, executable):
    """Actual byte-zero source grammar, with no SavedOutput-offset borrowing."""
    location = executable.parent
    source = location/'source'
    cargo_target = location/'cargo-target'
    subprocess.run(['cargo', 'build', '--offline', '-p', 'c6-core', '--lib',
                    '--target-dir', str(cargo_target)], cwd=source, check=True)
    checker = location/'check-first-player'
    if not checker.exists():
        subprocess.run(['rustc', '--edition=2024', '--cfg', 'feature="full-core"',
            str(source/'tools/linux/hero_discovery_order_check.rs'),
            '--extern', 'c6_core='+str(cargo_target/'debug/libc6_core.rlib'),
            '-L', 'dependency='+str(cargo_target/'debug/deps'), '-o', str(checker)], check=True)
    rows = []
    for identity, path, compressed in [
        (RAW, private/'evidence/saves/xp2-gamecore-stream.bin', False),
        (SAVED, private/'evidence/linux-game-load/canonical'/(SAVED+'.bin.gz'), True),
    ]:
        data = path.read_bytes()
        if compressed:
            data = gzip.decompress(data)
        require(sha(data) == identity, 'byte-zero fixture identity differs')
        incoming, output = location/(identity+'.input.bin'), location/(identity+'.heroes.bin')
        incoming.write_bytes(data)
        row = json.loads(run([str(checker), str(incoming), str(output)], text=True))
        incoming.unlink()
        row.update(stream_sha256=identity,
                   heroes_encoded_sha256=sha(output.read_bytes()) if row['parsed'] else None,
                   byte_zero_source_grammar_attempted=True, offsets_borrowed_from_other_fixture=False,
                   native_rebuild_reached_or_skipped='not observed; header/order parsing does not establish manager/configuration gates')
        rows.append(row)
    return dict(records=rows, candidate_sha256=sha(checker.read_bytes()),
                dependency_lock_sha256=sha((source/'Cargo.lock').read_bytes()),
                exact_first_player_heroes_records_equal=(rows[0]['heroes_encoded_sha256'] == rows[1]['heroes_encoded_sha256']) if all(r['parsed'] for r in rows) else None,
                remaining_players_raw_read_heroes_framing=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--private-root', type=Path, default=ROOT.parent/'c6-ghidra')
    parser.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    elf, builder = source_gate(args.assets/'libGameCore_XP2.so')
    Elf(args.assets/'Civ6', HOST_SHA)
    raw = (args.private_root/'evidence/saves/xp2-gamecore-stream.bin').read_bytes()
    require(sha(raw) == RAW and len(raw) == 76556938, 'raw incoming Read fixture differs')
    databases = []
    for suffix in ('1', '2'):
        path = args.private_root/('evidence/linux-rules/mod-activation-xp2-constructor-final-'+suffix)/'gameplay.sqlite'
        require(sha(path.read_bytes()) == DATABASE, 'activated database identity differs')
        with sqlite3.connect(path.resolve().as_uri()+'?mode=ro', uri=True) as database:
            rows = database.execute(c_string(elf, 0x2e3cb83).decode()).fetchall()
        require(not rows, 'expected empty HeroClasses builder result differs')
        databases.append(dict(sha256=DATABASE, native_builder_rows=len(rows), readonly=True))
    exe, candidate_identity = candidate(args.source_commit)
    saved = saved_fixture(args.private_root, exe)
    first = first_player_fixtures(args.private_root, exe)
    report = dict(schema='c6-linux-hero-discovery-order-source-v1', checks_passed=True,
        original_execution=False, gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        methods=[dict(address=hex(a), size=n, name=name, sha256=sha(elf.read(a,n))) for a,n,name in METHODS],
        candidate=candidate_identity, saved_fixture=saved, byte_zero_first_player_comparison=first,
        incoming_raw_read=dict(sha256=RAW, bytes=len(raw), first_player_heroes_source_framing_available=first['records'][0]['parsed'],
            all_players_heroes_owner_framing_available=False,
            actual_rebuild_reached_or_skipped='unresolved; retain exact byte-zero parser support result and native manager/configuration/registry gaps'),
        builder=builder, activated_databases=databases,
        empty_class_collection_rng_proof='If the runtime collection is the fresh collection populated solely by the pinned zero-row builder result, every rebuild consumes zero RNG draws, independent of earlier gate outcomes.',
        runtime_collection_database_binding_admitted=False,
        bounded_kernel=dict(maximum_output_records=28, exact_native_insertion_sort_ties=True,
            owned_heroes_and_game_random_mutation=True, transactional_missing_inputs=True,
            native_archive_edits='ordered diagnostic effects; physical pointer trees not required for local value mutation',
            source_queries='fallible interfaces; synthetic tests confer no captured configuration/eligibility authority',
            production_loaded_state_bridge=False, original_native_rebuild_oracle=False),
        discovered_read_projection=dict(live_indexed_snapshot_required=True,
            existing_indexed_baseline_required=True, read_resets_baseline=False,
            reset_baseline_only_with_separately_proven_initialize_or_reset=True,
            carrier_less_than_collection_count_is_direct_index=True,
            duplicate_mapped_entries='last write wins in wire order',
            missing_carriers='read into discard slot; raw wire entries retained',
            complete_contiguous_native_collection_indices_required=True,
            empty_collection_requires_no_discovered_snapshot=True),
        legacy_player_heroes_guard=dict(player_version_minimum=47, compare='0x2b4fc2d',
            signed_less_branch='0x2b4fc32', skipped_destination='0x2b4fc47',
            heroes_read_call='0x2b4fc42', skipped_branch_resets_heroes=False,
            raw_player_version_not_inferred_from_game_version=True),
        full_state_actions_turns=False, save_writer=False, unknown_bytes_masked=False,
        recorded_raw_save_load_failure_bytes=89198)
    require(not args.output.exists(), 'refusing to overwrite retained report')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, saved_records=len(saved['records']), empty_saved_orders=saved['zero_orders'], incoming_read_first_player_heroes_framing=first['records'][0]['parsed'])))


if __name__ == '__main__':
    main()
