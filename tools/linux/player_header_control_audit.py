#!/usr/bin/env python3
"""Read-only pinned Player47 alive bridge; offline canonical first-header checks."""
import argparse
import gzip
import json
from pathlib import Path
import re
import struct
import subprocess
from core_stream_index import METHODS as CORE_METHODS, Reader, index_core, sha
from player_districts_audit import FIXTURES
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    (0x2003cb0, 7, 'Player.Instance.IsAlive', '3002fe1bd5eaf1a0e1fa05cede54870c8999f7e31479aef1a8dfdcc00460032d'),
    (0x2b502b0, 21, 'Player bool operator<<', '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
    (0x2b50590, 21, 'Player bool operator>>', '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
    (0x2b3e140, 15, 'Player bool load', '6cec740fce0026f8bfec042c04cef3c6787a67ca02d451b186eece9a4c2943ac'),
    (0x2b3e180, 15, 'Player bool save', '52db51217991d6485b972e2fd724ce7fe3c20ab77e9c25fc13ae01d04404515c'),
    (0x2b31af0, 15, 'Player.Instance.GetTeam', '91b3ec5a8477dc9a77ac40ae060e9bd8eef5f4fc57051363a323183f8eea15f9'),
    (0x2b31530, 32, 'Player.Instance.IsHuman', '7b956c7055aea66551b2309c8d001e35a098fc6e19355bd959b5a46241245c95'),
    (0x28ec230, 14, 'Configuration.Player.Instance.IsHuman', 'a98088b62436c799ea309acd50c5ee744fd7ad1846e1be89d12429e6ffb9015a'),
] + [(a, n, name, h) for module, a, n, name, h in CORE_METHODS if a in (0x2b4ee00, 0x2b4f490)]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root', type=Path, default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    game = Elf(args.assets/'libGameCore_XP2.so', GAME_SHA)
    Elf(args.assets/'Civ6', HOST_SHA)
    for a, n, name, digest in METHODS:
        if sha(game.read(a, n)) != digest:
            raise ValueError('method differs: '+name)
    if struct.unpack('<I', game.read(0x2f65d64, 4))[0] != 47:
        raise ValueError('Player version differs')
    # Exact source address and call sites: first bool follows EndTurnBlocking.
    for a in (0x2b4ef24, 0x2b4f86f):
        if game.read(a, 7) != bytes.fromhex('498db650020000'):
            raise ValueError('alive wrapper source differs')
    calls = [(0x2b4ef1f, 0x1e4cec0), (0x2b4ef2e, 0x1e45260),
             (0x2b4f86a, 0x1f2dc00), (0x2b4f879, 0x1f66c30)]
    for a, target in calls:
        game.call(a, target)
    # Itanium ABI address point is vtable symbol +16; wrapper virtual slots +16/+40.
    for a, target in ((0x3addb48, 0x2b3e140), (0x3addb60, 0x2b3e180)):
        if game.relocated_pointer(a) != target:
            raise ValueError('bool vtable bridge differs')
    dwarf = subprocess.check_output(['gdb', '-nx', '-batch', str(args.assets/'libGameCore_XP2.so'),
        '-ex', "ptype /o 'GameCore::Player::Instance'",
        '-ex', "ptype /o 'FAutoVariable<bool, GameCore::Player::Instance>'"], text=True)
    if not re.search(r'/\*\s+592\s+\|\s+24\s+\*/.*?\}\s+m_bIsAlive;', dwarf, re.S):
        raise ValueError('alive DWARF wrapper differs')
    if not re.search(r'/\*\s+16\s+\|\s+1\s+\*/\s+ObjectType m_value;', dwarf):
        raise ValueError('bool DWARF value offset differs')
    fixtures = []
    for identity, _, _ in FIXTURES:
        data = gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
        if sha(data) != identity:
            raise ValueError('canonical Core differs')
        indexed = index_core(data, first_formations=True)
        section = next(x for x in indexed['sections'] if x['name'] == 'first_player_prefix')
        # Start from a structurally located section, then parse each typed field.
        r = Reader(data[section['start']:section['end']])
        r.version(47)
        owner = r.take('i')
        starting_plot = [r.take('i'), r.take('i')]
        scalars = [r.take('i') for _ in range(15)]
        flags = [r.boolean() for _ in range(6)]
        tail = [r.u32() for _ in range(4)]
        if r.pos != section['bytes']:
            raise ValueError('header not exhausted')
        fixtures.append(dict(core_sha256=identity, first_player_owner=owner,
            header_start=section['start'], header_bytes=r.pos, header_sha256=section['sha256'],
            starting_plot=starting_plot, ordered_scalars=scalars, ordered_flags=flags,
            is_alive=flags[0], trailing_words=tail, canonical_bool_checks=6))
    report = dict(schema='c6-linux-player-header-control-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, player_version=47,
        methods=[dict(address=hex(a), size=n, name=name, sha256=h) for a,n,name,h in METHODS],
        direct_calls=[dict(address=hex(a), target=hex(t)) for a,t in calls],
        dwarf_sha256=sha(dwarf.encode()), audit_source_sha256=sha(Path(__file__).read_bytes()),
        alive=dict(source_field='m_bIsAlive', wrapper_offset=592, value_offset=608,
            ordered_flags_index=0, accessor_contract='ordered_flags[0]',
            native_read_applies_wire_value=True, native_post_read_fixups_validated=False),
        external_configuration=dict(team=True, human=True, human_slot_status=3,
            exact_core_header_bridge_proven=False), fixtures=fixtures,
        original_execution=False, private_writes=False, full_player_roster=False,
        gameplay_state=False, actions=False, turns=False, unknown_bytes_masked=0,
        recorded_raw_save_load_difference_bytes=89198)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS), fixtures=len(fixtures), original_execution=False)))


if __name__ == '__main__':
    main()
