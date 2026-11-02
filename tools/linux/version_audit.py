#!/usr/bin/env python3
"""Compare installed Civ VI depot identities with anonymous Valve appinfo.

Read-only: never executes game code or copies account metadata. Obtain appinfo
with SteamCMD +login anonymous +app_info_update 1 +app_info_print 289070 +quit.
"""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import re


def parse_vdf(text, root):
    tokens = [a or b for a, b in re.findall(r'"([^"\n]*)"|([{}])', text)]
    start = next(i for i, token in enumerate(tokens[:-1])
                 if token == root and tokens[i + 1] == '{')

    def object_at(index):
        result = {}
        index += 1
        while tokens[index] != '}':
            key = tokens[index]
            index += 1
            if tokens[index] == '{':
                value, index = object_at(index)
            else:
                value = tokens[index]
                index += 1
            result[key] = value
        return result, index + 1

    return object_at(start + 1)[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('assets', type=Path)
    parser.add_argument('appinfo', type=Path)
    parser.add_argument('--observed-at', required=True, help='UTC retrieval timestamp')
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    installed = parse_vdf((args.assets / 'steamapps/appmanifest_289070.acf').read_text(), 'AppState')
    public = parse_vdf(args.appinfo.read_text(), '289070')['depots']
    rows = []
    for depot, info in installed['InstalledDepots'].items():
        current = public.get(depot, {})
        manifest = current.get('manifests', {}).get('public', {}).get('gid')
        rows.append({'depot': depot, 'oslist': current.get('config', {}).get('oslist'),
                     'installed_manifest': info['manifest'], 'public_manifest': manifest,
                     'matches': info['manifest'] == manifest})
    branch = public['branches']['public']
    hashes = {}
    for name in ['Civ6', 'libGameCore_XP2.so']:
        with (args.assets / name).open('rb') as stream:
            hashes[name] = hashlib.file_digest(stream, 'sha256').hexdigest()
    result = {
        'schema': 1, 'observed_at_utc': args.observed_at,
        'source': 'Valve SteamCMD anonymous app_info_update 1; app_info_print 289070',
        'source_url': 'https://developer.valvesoftware.com/wiki/SteamCMD',
        'installed_build_id': installed['buildid'], 'public_build_id': branch['buildid'],
        'public_branch_updated_utc': datetime.datetime.fromtimestamp(
            int(branch['timeupdated']), datetime.timezone.utc).isoformat(),
        'aspyr_version': json.loads((args.assets / 'com.aspyr.civ6.version.json').read_text()),
        'artifact_sha256': hashes, 'installed_depots': rows,
        'all_installed_depot_manifests_match_public': all(row['matches'] for row in rows),
        'limits': ['Manifest identity comparison does not verify every installed file against Valve depot content.',
                   'Public branch update time does not date every platform binary.',
                   'Current Linux release identity does not establish Windows ABI or behavior equivalence.',
                   'No game execution, initialized gameplay or action/turn parity is tested.']}
    args.output.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
