#!/usr/bin/env python3
"""Summarize original XP2 lower Core Load/Save observations, never gameplay parity."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def summarize(report, outputs):
    stderr = report['stderr']
    loads = [dict(result=int(a), offset=int(b), size=int(c)) for a, b, c in re.findall(
        r'C6_LINUX_STREAM load_return result (-?\d+) offset (\d+) size (\d+)', stderr)]
    saves = [dict(result=int(a), size=int(b)) for a, b in re.findall(
        r'C6_LINUX_STREAM save_return result (-?\d+) size (\d+)', stderr)]
    files = [{'bytes': p.stat().st_size, 'sha256': digest(p)} if p.is_file() else None for p in outputs]
    completed = report['returncode'] == 0 and 'C6_LINUX_GAME_LOAD boundary_return 0 ' in stderr
    saved = len(saves) == len(files) == 2 and all(
        artifact is not None and native['result'] == 0 and artifact['bytes'] == native['size']
        for native, artifact in zip(saves, files))
    return {
        'schema': 'c6-linux-game-load-observation-v1',
        'configuration_sha256': report['configuration_sha256'],
        'returncode': report['returncode'],
        'core_loads': loads, 'core_saves': saves, 'private_save_files': files,
        'lower_core_boundary_completed': completed,
        'complete_input_consumed': bool(loads) and all(x['result'] == 0 and x['offset'] == x['size'] for x in loads),
        'consecutive_save_bytes_equal': completed and saved and files[0] == files[1],
        'framework_cleanup_proven': report.get('framework_cleanup_proven', False),
        'host_load_finalization_proven': False, 'initialized_gameplay_fixture': False, 'gameplay_parity': False,
        'limits': 'Lower Context StartupGame/Load/Save diagnostic using constructor-state Update callback. No saved RequestLoad/Begin, postload cache/PostInitialize/active-user lifecycle, action or turn equivalence. Core Save omits live queues, caches, jobs and session state; consecutive equality is not branch-restorable full-state proof.',
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-report', type=Path, required=True)
    parser.add_argument('--save-output', type=Path, required=True)
    parser.add_argument('--save-repeat-output', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    report = json.loads(args.native_report.read_text())
    result = summarize(report, [args.save_output, args.save_repeat_output])
    result['native_report_sha256'] = digest(args.native_report)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
