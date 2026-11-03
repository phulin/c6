#!/usr/bin/env python3
"""Read-only original Linux prerequisite owner contracts; no native execution."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [(45653904, 242, 'Player.Techs.Write', 'a1eee5b85bf91e347826bbf1fc76bdaa2c7f54e983e55d9bde95e9b36055e8d6'),
 (45654160, 494, 'Player.Techs.Read', 'eadb0d0289bb1cdefd1870b3bccac5a6022119147595fdd84327e634156b6553'),
 (45654656, 384, 'Player.Techs.Reset', 'b8eba90de6208478dccacd31d781e2f77f7de6c8e06d11f87db9816d11927ec3'),
 (44491568, 1357, 'Player.Culture.Write', '9f823e183f89f4a5e93421be6d5eefab371538923b3cfc193d11dfb413707232'),
 (44492928, 3414, 'Player.Culture.Read', '65066739c39f896f5058cf0b5c11ff19085e7d2eae23930a9695bda1cec380b2'),
 (44496352, 2026, 'Player.Culture.Reset', 'bbd64b805d50c69fe2ee3ff5179d4b568c85a3cbf52ff509ea04d50d54a6f225'),
 (35192160,
  149,
  'Technology.ReadIndexHashArray.bool',
  '40b036252c1824179cc68526958d444b6d22db6d9791ec360d73dfacc11e3083'),
 (35192400,
  138,
  'Technology.WriteIndexHashArray.bool',
  'ff63bebb8b64f6198886ad14a9b928b9e9201b097a4a00784c99e9f8335d6249'),
 (44559056,
  149,
  'Civic.ReadIndexHashArray.bool',
  '393963ad90a53b3b31abf3ba73c0856089a60449af713aeecd5a6a66793aa06a'),
 (44556752,
  138,
  'Civic.WriteIndexHashArray.bool',
  'fd4403786ee9ac4d96074a7a7f83146088d30a65230239a871263b22a4ad1df2'),
 (33645696, 34, 'Technology.HashToIndex', 'bbba3422fa0bd6194a1c30c88ee7b8e77f8f7f0733c020e55d5c967202a1dd21'),
 (33648976, 34, 'Civic.HashToIndex', '748e53de45220156b20d66f313d3a803b68d269be59bcac16833d8a36347aab2'),
 (44224848, 15, 'Game.GetNumTechs', 'b7cc5f4e936d8d76f805906b752cfedcd703ebf43ded08244f8be2a7dbc6d9ae'),
 (43484096, 15, 'Game.GetNumCivics', '4572387a33de0b4f82f7b37e06ea0bf5f13dd6494a92187786695ebdbda01827'),
 (44224896, 132, 'Game.GetTechNode', 'a7ba4456eb131bf96c471355b9e7266eeca1fb924e828b1dfa32670b15614095'),
 (43483952, 132, 'Game.GetCivicNode', 'dc69e6d710859a001da3691f848c32cfffd1db53be66ab275f12b3d3520dfdaf'),
 (44229728, 4, 'TechNode.IsActive', 'b3f029064691ffb01edfa268341c425a57f45a1174d9b787a33bff7f42083fc4'),
 (43493712, 4, 'CivicNode.IsActive', 'b3f029064691ffb01edfa268341c425a57f45a1174d9b787a33bff7f42083fc4'),
 (33456544, 75, 'IStream.ReadObject.bool', '1b35f5e559410da6d1adcde91f94b8b0ee9a16f84d871535a2d18c3ae544487b')]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    elf = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for address, size, name, expected in METHODS:
        if hashlib.sha256(elf.read(address, size)).hexdigest() != expected:
            raise ValueError('original method differs: ' + name)
    calls = [
        (0x2b8a0ac, 0x1f24d60, 'Techs Read invokes Reset before arrays'),
        (0x2a6e8a1, 0x1fb91a0, 'Culture Read invokes Reset before arrays'),
        (0x2b8a138, 0x1efadf0, 'first technology bool hash overlay'),
        (0x2a6ec98, 0x1f23060, 'first civic bool hash overlay'),
        (0x2b8a33d, 0x1f250d0, 'Techs acquired explicit-false resize'),
        (0x2a6f8bd, 0x1f250d0, 'Culture acquired explicit-false resize'),
        (0x218fdd6, 0x1f3f970, 'known or unknown hash consumes one bool'),
    ]
    for address, target, _ in calls:
        elf.call(address, target)
    instructions = [
        (0x2b8a330, 'c6042400', 'Techs Reset explicit false byte'),
        (0x2a6f8b3, 'c6042400', 'Culture Reset explicit false byte'),
        (0x2a2e460, '8a4704c3', 'TechNode active byte at offset4'),
        (0x297a950, '8a4704c3', 'CivicNode active byte at offset4'),
        (0x1fe81b2, 'be01000000', 'bool ReadObject requests one byte'),
    ]
    for address, expected, name in instructions:
        if elf.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('original instruction differs: ' + name)
    result = dict(
        schema='c6-linux-player-prerequisites-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA,
        methods=[dict(address=hex(a), bytes=n, name=name, sha256=h) for a,n,name,h in METHODS],
        calls=[dict(address=hex(a), target=hex(t), contract=c) for a,t,c in calls],
        instruction_contracts=[dict(address=hex(a), bytes=b, contract=c) for a,b,c in instructions],
        acquired_vectors=dict(technology_raw_offset='0x140', technology_auto_offset='0x130', civic_raw_offset='0x360', civic_auto_offset='0x350', array='first bool hash array in each owner', reset='clear and resize false to actual definition count', wire='u32 pair count then u32 hash and byte bool in original order; unknown hashes consumed, later duplicates overwrite', remapping='HashToIndex calls actual native definition Find then GetCollectionIndex'),
        global_nodes=dict(owner_raw_offset='0xc0', type='signed32 native enum', active_offset=4, completeness='whole ordered post-Read/Validate live node vector', lookup='minusone absent; matching positional fast path; otherwise first matching node; absent/inactive false'),
        remaining_capture=['structural location of acquired arrays through complete preceding unframed Player components, or actual complete live acquired vectors', 'complete original live Game.Techs/Culture nodes after Validate', 'native-order actual technology/civic definition registries and full module/rules/DLC/mod/configuration identity', 'initialized native differential output and affected-state comparison'],
        private_evidence='../c6-ghidra/evidence/linux-player-prerequisites',
        source_sha256=hashlib.sha256((ROOT/'crates/c6-core/src/player_prerequisites.rs').read_bytes()).hexdigest(),
        native_execution=False, native_differential_acceptance=False,
        complete_owner_codec=False, complete_core_import=False,
        gameplay_parity=False, action_parity=False, turn_parity=False,
        raw_save_load_difference_bytes=89198,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS), native_execution=False)))


if __name__ == '__main__':
    main()
