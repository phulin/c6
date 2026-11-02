#!/usr/bin/env python3
"""Acquire the pinned public reference; game binaries are never downloaded."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = Path(__file__).with_name('manifest.json')


def acquire(destination):
    manifest = json.loads(MANIFEST.read_text())
    revision = manifest['commit']
    if not destination.exists():
        subprocess.run(['git', 'clone', '--no-checkout', manifest['repository'], str(destination)], check=True)
    actual_repo = subprocess.check_output(['git', '-C', str(destination), 'remote', 'get-url', 'origin'], text=True).strip()
    if actual_repo != manifest['repository']:
        raise SystemExit(f'Refusing an unexpected repository at {destination}')
    subprocess.run(['git', '-C', str(destination), 'fetch', 'origin', revision], check=True)
    subprocess.run(['git', '-C', str(destination), 'checkout', '--detach', revision], check=True)
    for name, expected in manifest['sha256'].items():
        actual = hashlib.sha256((destination / name).read_bytes()).hexdigest()
        if actual != expected:
            raise SystemExit(f'Checksum mismatch: {name}')
    print(f'Verified {len(manifest["sha256"])} artifacts at {revision}: {destination}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--destination', type=Path, default=ROOT / 'evidence/reference/upstream')
    acquire(parser.parse_args().destination.resolve())
