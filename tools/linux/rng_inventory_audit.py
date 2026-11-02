#!/usr/bin/env python3
"""Hash-gated Linux RNG ownership audit; static tools only, no native execution."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess

from inspect_elf import sections

ROOT = Path(__file__).resolve().parents[2]
XP2 = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
PREFIXES = ('GameCore::Random::', 'GameCore::ScopedRandom::',
            'GameCore::Game::Instance::GetRandNum(',
            'GameCore::Game::Instance::GetSyncRandom(',
            'GameCore::Game::Instance::EditSyncRandom(',
            'GameCore::Game::Instance::SetRandomSeed(',
            'FAutoVariable<GameCore::Random, GameCore::Game::Instance>::',
            'GameCore::Cache::Lua::IGame::GetRandNum(',
            'GameCore::Cache::Lua::IGame::RegisterMembers(',
            'GameCore::Context::Instance::InitializeSyncRandomSeed(',
            'GameCore::Game::Initializer::InitializeGameInstance(',
            'GameCore::Game::Initializer::InitializeNewGame(',
            'GameCore::Game::Instance::Initialize()',
            'GameCore::Map::Generator::GetRandom(',
            'GameCore::Map::Generator::GenerateRandomMap(',
            'GameCore::Map::Generator::AssignStartingPositions(',
            'GameCore::Lua::IMapTerrainBuilder::lGetRandomNumber(')
CALL_PREFIXES = ('GameCore::Random::', 'GameCore::ScopedRandom::',
                 'GameCore::Game::Instance::GetRandNum(',
                 'GameCore::Map::Generator::GetRandom(', 'Math::RandomPCG32::')


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def run(*args):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT)


def read_at(path, address, size):
    for row in sections(path).values():
        start = int(row['address'], 16)
        if start <= address and address + size <= start + row['bytes']:
            with path.open('rb') as source:
                source.seek(row['offset'] + address - start)
                result = source.read(size)
            if len(result) != size:
                raise ValueError('truncated ELF read')
            return result
    raise ValueError('unmapped ELF address')


def methods(path, prefixes):
    result = []
    for line in run('nm', '-D', '-C', '-S', '--defined-only', str(path)).splitlines():
        match = re.match(r'([0-9a-f]+) ([0-9a-f]+) [TW] (.*)', line)
        if match and match[3].startswith(prefixes):
            address, size = int(match[1], 16), int(match[2], 16)
            row = dict(name=match[3], address=hex(address), size=size,
                       sha256=hashlib.sha256(read_at(path, address, size)).hexdigest())
            if row not in result:
                result.append(row)
    return sorted(result, key=lambda row: (int(row['address'], 16), row['name']))


def audit(assets, private):
    module, host = assets/'libGameCore_XP2.so', assets/'Civ6'
    if digest(module) != XP2 or digest(host) != HOST:
        raise ValueError('unsupported original Linux GameCore/host identity')
    private.mkdir(parents=True, exist_ok=True)
    commands = ['gdb', '-q', '-nx', '-batch', str(module)]
    for name in ('GameCore::Game::Instance', 'GameCore::Random',
                 'GameCore::ScopedRandom', 'GameCore::Map::Generator', 'Math::RandomPCG32'):
        commands += ['-ex', "ptype /o '" + name + "'"]
    dwarf = run(*commands)
    (private/'inventory.dwarf').write_text(dwarf)
    checks = [r'/\*\s+232\s+\|\s+48\s+\*/[^\n]*FAutoVariable<GameCore::Random',
              r'/\*\s+248\s+\|\s+32\s+\*/\s+ObjectType m_value;',
              r'/\*\s+8\s+\|\s+4\s+\*/\s+SeedType m_ulRandomSeed;',
              r'/\*\s+20\s+\|\s+4\s+\*/\s+uint m_uiLockCount;',
              r'/\*\s+0\s+\|\s+8\s+\*/\s+uint64_t state;',
              r'/\*\s+8\s+\|\s+8\s+\*/\s+uint64_t inc;']
    if not all(re.search(pattern, dwarf) for pattern in checks):
        raise ValueError('RNG owner/live-state DWARF contract differs')
    core_methods = methods(module, PREFIXES)
    host_methods = methods(host, ('Math::RandomPCG32::',))
    starts = {int(row['address'], 16): row['name'] for row in core_methods}
    calls, inline, raw = [], [], []
    owner, owner_address, selected = '', 0, False
    proc = subprocess.Popen(['objdump', '-d', '-C', str(module)], stdout=subprocess.PIPE, text=True)
    assert proc.stdout is not None
    for line in proc.stdout:
        match = re.match(r'^([0-9a-f]+) <(.*)>:', line)
        if match:
            owner_address, owner = int(match[1], 16), match[2]
            selected = owner_address in starts and '@plt' not in owner
        if selected:
            raw.append(line)
        if '@plt' in owner:
            continue
        branch = re.search(r'^\s*([0-9a-f]+):.*\b(call|jmp)\s+([0-9a-f]+) <(.*)>', line)
        if branch and branch[4].startswith(CALL_PREFIXES) and '+' not in branch[4]:
            calls.append(dict(caller=owner, caller_address=hex(owner_address),
                              site=hex(int(branch[1], 16)), target=branch[4]))
        if '0x41c64e6d' in line:
            inline.append(dict(owner=owner, site=hex(int(line.split(':')[0], 16))))
    if proc.wait() != 0:
        raise ValueError('objdump failed')
    (private/'inventory-selected.asm').write_text(''.join(raw))
    (private/'inventory-direct-calls.json').write_text(json.dumps(calls, indent=2)+'\n')
    pcg_asm = run('objdump', '-d', '-C', '--start-address=0x3c087b4',
                  '--stop-address=0x3c08896', str(host))
    (private/'inventory-host-pcg.asm').write_text(pcg_asm)
    bodies = ''.join(raw)
    required = ['add    $0xe8,%rdi', 'add    $0x10,%rbx',
                'GameCore::Random::Get(unsigned short, char const*)@plt',
                'GameCore::Random::Reseed(unsigned int)@plt',
                'GameCore::Random::Lock()@plt', 'GameCore::Random::Unlock()@plt',
                'div    %ebx', 'Platform::GetTime(bool, bool)@plt',
                '# 3c02ac0 <GameCore::Cache::Lua::ms_Random>']
    if not all(fragment in bodies for fragment in required):
        raise ValueError('selected target instruction contract differs')
    if len(inline) != 2 or set(row['owner'] for row in inline) != {
            'GameCore::Random::Get(unsigned short, char const*)',
            'GameCore::ScopedRandom::Get(unsigned short, char const*)'}:
        raise ValueError('inline immediate census changed')
    counts = Counter(row['target'] for row in calls)
    return dict(schema='c6-linux-rng-inventory-v1', checks_passed=True,
                gamecore_sha256=XP2, host_sha256=HOST, native_execution=False,
                complete_rng_inventory=False, complete_state_action_turn_parity=False,
                source_sha256=digest(Path(__file__).resolve()),
                dwarf_sha256=hashlib.sha256(dwarf.encode()).hexdigest(),
                methods=core_methods, host_methods=host_methods,
                direct_call_counts=dict(sorted(counts.items())), direct_calls=calls,
                lcg_immediate_occurrences=inline,
                streams=[
                    dict(id='game_sync', owner='GameCore::Game::Instance::m_kSyncRandom',
                         wrapper_offset=232, value_offset=248, live_bytes=32,
                         live_fields={'seed':8,'calls':12,'resets':16,'locks':20,'raw_sync':24},
                         normal_record_bytes=18, normal_record_fields=['version','seed','calls','resets','raw_sync','legacy_flag'],
                         normal_record_omits=['vptr','locks','padding'], scope='gameplay; combat draw path verified',
                         lifecycle='embedded in registered FAutoVariable; edit marks owner archive; InitializeSyncRandomSeed initializes; zero seed fallback Reset; SetRandomSeed uses Reseed'),
                    dict(id='map_generator', owner='GameCore::Map::Generator::ms_kRandom',
                         address='0x3c072f0', live_bytes=32, scope='gameplay initialization/map generation',
                         lifecycle='module static; Reset from Configuration::Map random seed in InitializeNewGame',
                         serialization='Random codec exists; no owner-level persistence proof from this direct-call audit'),
                    dict(id='cache_lua_pcg32', owner='GameCore::Cache::Lua::ms_Random',
                         address='0x3c02ac0', live_bytes=16, live_fields={'state':0,'inc':8},
                         lifecycle='module static constructor; RegisterMembers reseeds from low32 Platform::GetTime(false,true)',
                         draw='IGame::GetRandNum: host PCG32 Rand then unsigned remainder; zero bound divides by zero after draw',
                         serialization='no named PCG Read/Write/Reset methods identified; complete caller/data-flow ownership unresolved',
                         scope='unclassified; Cache namespace does not prove presentation-only'),
                    dict(id='scoped_random', owner='caller-provided four-byte object', live_bytes=4,
                         methods={'constructor':'0x2d4d030','reset':'0x2d4d040','draw':'0x2d4d050','get_seed':'0x2d4d070'},
                         lifecycle='explicit seed ctor/Reset; no destructor or Read/Write symbols identified',
                         scope='unclassified; no direct Get callers in scanned XP2, indirect/other-module use unresolved')],
                limitations=[
                    'Static direct branches and one exact LCG immediate scan do not enumerate indirect calls, function pointers, scripts or every RNG family.',
                    'No thread-local RNG or fixed-hash pseudorandom helper ownership has been proven; absence from selected symbols is not exclusion.',
                    'Host rendering/other RNG families and dependency RNG objects have not been exhaustively classified.',
                    'Existing native state observer snapshots only game_sync and map_generator; this audit runs no native fixture.',
                    'An 18-byte normal record omits live lock count and archive change tracking; equal stream records do not prove complete live state.',
                    'No native PCG32/ScopedRandom oracle or Rust parity is established by this static audit.',
                    'Unknown save bytes remain unclassified/unmasked; raw 89198-byte Save-to-Load failure remains open.'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--private-output', type=Path, default=ROOT.parent/'c6-ghidra/evidence/linux-rng-inventory')
    parser.add_argument('--output', type=Path, default=ROOT/'evidence/linux/rng-inventory.json')
    args = parser.parse_args()
    result = audit(args.assets, args.private_output)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=result['checks_passed'], methods=len(result['methods']),
                          direct_call_sites=len(result['direct_calls']), complete_rng_inventory=False)))


if __name__ == '__main__':
    main()
