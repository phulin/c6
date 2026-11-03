#!/usr/bin/env python3
"""Read-only pinned Linux improvement prerequisite numerical contracts; no native execution."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    (0x2a3fd30, 267, 'Improvement.BonusTech', '4e88ea903a10e995089047c15fe4c64838ad782be69733321544cfcd54a18f98'),
    (0x2a3fe40, 267, 'Improvement.BonusCivic', '7fcdc9e73aac2cd148e91df4a58569b4cdca56063b27b3fa6a958617de44b9c1'),
    (0x2b8ac60, 79, 'Player.Techs.HasTech', 'cf4a3cabd3f3b09872b8bd16197980410160d0ae810aae1d63a423ea68c32ec3'),
    (0x2a73e70, 86, 'Player.Culture.HasCivic', 'd61b6ae94c24981349cecd699fed49fa57c413f81d7d15787f33a3996dfe4f68'),
    (0x2a2d160, 24, 'Game.Techs.HasTech', '15b73ac27200be281d4dc5657f341244ff1f86e80e95a10d7d3c0dfeda4125a9'),
    (0x2978310, 24, 'Game.Culture.HasCivic', '6b922359c9946249e8f6cb842ea7af3d46d23608c32d0a8855aa1178528e759f'),
    (0x1fe6b50, 14, 'BoolVector.index.const', 'f13ff4ca446e97d1c02ca5ef61a9e5e1703d4640688045c5ca379017e9371e79'),
    (0x2007f30, 4, 'BoolVector.size.const', '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
    (0x1fcc8b0, 5, 'FixedPoint.integer_ctor', 'a5050360463a3fe04639a9ad47d842d6a2c680138f54cfbbbe95eaf24f73e912'),
    (0x1fcdc10, 9, 'FixedPoint.assign_integer', '289ee59e91d4f68ddbe5f570b5002092702cd7aae484d25f21ca0576db55b96f'),
    (0x1fe5330, 8, 'FixedPoint.add', '81e58c42b7d805beeed4b70b2cfd4565f3eaabb16e37e1baefa538f55502258f'),
    (0x2951db0, 19, 'Civic.GetIndex', '6f03228f444b3b6c1b4019e2d3471367abddad91fe407aa0259503531c9ad626'),
    (0x2955a70, 94, 'Civic.GetTypeIndex', '57e02c836734d6b9eb1a7d60e774b6a23bbf46e3dc2aa09582e75fc0217ae170'),
    (0x2004fc0, 51, 'Player.Manager.IsValid', '31c2b843f442ca71b8fd27edeccb2888940a331a9f50686c2a1e078cd25be038'),
    (0x2005000, 7, 'Player.Manager.IsValidRange', '1bda6fddb3180619888eb42cbb75a05d0877b5498c502ebb72ec289b4e0ba7a1'),
    (0x2a2d180, 132, 'Game.GetTechNode', 'a7ba4456eb131bf96c471355b9e7266eeca1fb924e828b1dfa32670b15614095'),
    (0x2978330, 132, 'Game.GetCivicNode', 'dc69e6d710859a001da3691f848c32cfffd1db53be66ab275f12b3d3520dfdaf'),
    (0x2a2e460, 4, 'TechNode.IsActive', 'b3f029064691ffb01edfa268341c425a57f45a1174d9b787a33bff7f42083fc4'),
    (0x297a950, 4, 'CivicNode.IsActive', 'b3f029064691ffb01edfa268341c425a57f45a1174d9b787a33bff7f42083fc4'),
    (0x2a414c0, 8, 'Improvement.BonusReferences', 'efa5016d731c94e18c5e74fee830e4c32594f708f3d1e27d39ed24fdc189f2a7'),
    (0x2a414d0, 4, 'BonusReference.begin', 'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'),
    (0x2a414e0, 5, 'BonusReference.end', '8e3eaf4f4b235f364076d47e3b5b95d9e60a71cf19dd62030d0fa20e80858d2e'),
    (0x2a414f0, 5, 'Bonus.YieldReference', '387fb5103487f0b5fee8adcd266e45ec84e7da2718fef1244b3fd5cabdf69c80'),
    (0x2a41500, 5, 'Bonus.TechReference', 'b811adb2d88317c4120dc7c33dd574a1678d7f98965dd45cb6cf6745cafe2e73'),
    (0x2a41510, 4, 'Bonus.IntegerChange', '580f63e5ed34152c56532cbfda2c4a0f74402790e9e026cd4888e06541f671e1'),
    (0x2a41520, 5, 'Bonus.CivicReference', '7ee12ad7b55e604f6ff9eb30affde4f85327c6ded57b8cd89cc32ca9dc3f18db'),
    (0x1fccd30, 4, 'Technology.CollectionIndex', 'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a'),
    (0x1fccdd0, 4, 'Civic.CollectionIndex', 'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a'),
    (0x1fe51e0, 4, 'Yield.CollectionIndex', 'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a'),
    (0x1ffbea0, 109, 'Improvement.Find', 'b7736652c5c3868bda281b29100d8049e96647fb6d9866f3f4daae99390c4538'),
]
# Bytes are small critical-instruction contracts, not distributed disassembly.
INSTRUCTIONS = [
    (0x1fcdc10, 'c1e6088937', 'signed32 integer conversion shifts low32 by8'),
    (0x1fe5330, '8b060107', 'load32 then wrapping add32'),
    (0x2b8ac68, '83fdff7439', 'technology minusone short circuit'),
    (0x2b8ac7c, '39e87626', 'technology unsigned mask bounds before global lookup'),
    (0x2a73e78, '83fdff7440', 'civic minusone short circuit'),
    (0x2a73e8c, '39e8762d', 'civic unsigned mask bounds before global lookup'),
    (0x2955a82, '39e8773d', 'civic resolved index below native definition count is identity'),
    (0x2a3fdc3, '4885ff7458', 'null technology bonus row skips'),
    (0x2a3fdd2, '4889c7', 'nonnull yield reference supplies original definition'),
    (0x2a3fdda, '4439e07541', 'technology requested yield guard'),
    (0x2a3fdea, '7434', 'null technology prerequisite skips'),
    (0x2a3fe20, '4883c308', 'technology original reference order, eightbyte entries'),
    (0x2a3fed3, '4885ff7458', 'null civic bonus row skips'),
    (0x2a3feea, '4439e07541', 'civic requested yield guard'),
    (0x2a3fefa, '7434', 'null civic prerequisite skips'),
    (0x2a3ff30, '4883c308', 'civic original reference order, eightbyte entries'),
    (0x2a2e460, '8a4704c3', 'actual technology node active byte'),
    (0x297a950, '8a4704c3', 'actual civic node active byte'),
]
CALLS = [
    (0x2a3fd47, 0x1f567e0, 'initialize hidden return FixedPoint to zero'),
    (0x2a3fd56, 0x1f59220, 'native player validity before definition'),
    (0x2a3fd7f, 0x1e4a8e0, 'find actual improvement hash'),
    (0x2a3fdf9, 0x1fb8900, 'technology HasTech'),
    (0x2a3fe0f, 0x1f567e0, 'convert matching integer change'),
    (0x2a3fe1a, 0x1e95d70, 'accumulate in native order'),
    (0x2a3fe57, 0x1f567e0, 'initialize civic hidden return'),
    (0x2a3fe66, 0x1f59220, 'civic native player validity'),
    (0x2a3ff09, 0x1fb9018, 'civic HasCivic'),
    (0x2a3ff1f, 0x1f567e0, 'civic integer conversion'),
    (0x2a3ff2a, 0x1e95d70, 'civic ordered accumulation'),
    (0x2b8ac8a, 0x1e19410, 'technology global availability before acquired bit'),
    (0x2a73e9a, 0x1ea7590, 'civic global availability before acquired bit'),
    (0x2a73ea5, 0x1ee13e0, 'native civic index normalization'),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    core = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for address, size, name, expected in METHODS:
        if hashlib.sha256(core.read(address, size)).hexdigest() != expected:
            raise ValueError('method differs: ' + name)
    for address, expected, name in INSTRUCTIONS:
        if core.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('instruction differs: ' + name)
    for address, target, _ in CALLS:
        core.call(address, target)
    result = dict(
        schema='c6-linux-yield-prerequisites-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA,
        methods=[dict(address=hex(a), bytes=n, name=name, sha256=h) for a,n,name,h in METHODS],
        instruction_contracts=[dict(address=hex(a), bytes=b, contract=n) for a,b,n in INSTRUCTIONS],
        calls=[dict(address=hex(a), target=hex(t), contract=n) for a,t,n in CALLS],
        abi='hidden sret RDI; receiver RSI; player EDX; improvement hash ECX; requested yield R8D; fourbyte signed FixedPointT<8> storage',
        contract=dict(
            invalid_native_player='zero before definition input',
            absent_improvement_in_complete_registry='zero',
            references='actual ordered bonus references; null row/yield/prerequisite skips; requested yield matches original collection index',
            prerequisite='minusone or unsigned index beyond acquired vector false; then actual global node presence and IsActive; then acquired bit',
            civic_index='resolved actual definition index below native count guarantees Types.GetIndex identity',
            arithmetic='for each accepted row signed32 integer wrapping shift8 then wrapping add32',
            mutation='read only supplied rules/world; local accumulator only',
            missing_inputs='explicit errors, never invented zero or false'),
        rust_source_sha256=hashlib.sha256((ROOT / 'crates/c6-core/src/yields.rs').read_bytes()).hexdigest(),
        source_audit_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        private_evidence=['../c6-ghidra/evidence/linux-reporting-yield-effects-local/branch-05.asm',
                          '../c6-ghidra/evidence/linux-reporting-yield-effects-local/branch-06.asm',
                          '../c6-ghidra/evidence/linux-reporting-yield-query-closure-local/c6-HasTech.asm',
                          '../c6-ghidra/evidence/linux-reporting-yield-query-closure-local/c6-HasCivic.asm'],
        native_execution=False, native_differential_acceptance=False,
        complete_numerical_helpers_over_supplied_typed_inputs=True,
        full_getyield=False, live_action=False, gameplay_parity=False, turn_parity=False,
        raw_save_load_failure_bytes_unclassified=89198,
        gaps=['No initialized original helper oracle or native differential fixtures executed',
              'Runtime native ordered improvement/bonus reference export and original global/player mask assembly remain required',
              'Other plot/improvement contributions and complete gameplay state/actions/turns remain unimplemented'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS), native_execution=False)))


if __name__ == '__main__':
    main()
