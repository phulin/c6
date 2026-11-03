#!/usr/bin/env python3
"""Read-only pinned Linux numerical plot-yield and Feature storage audit.

This performs no native execution or numerical differential acceptance.
"""
import argparse
import hashlib
import json
from pathlib import Path

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    (0x2bf5ad0, 3498, 'Rules.Yields.GetYield', '5249f66e85d4f15aaac4e11747aa594dce491cdf8d30e9dc8301aac63606c293'),
    (0x21f1070, 116, 'NameToYieldType', '4d81d635b36ffc7819c8a81a7d79aa7c8b90dea8f1a91572fbaa28210f214fb7'),
    (0x2dd39b0, 29, 'Feature.GetPlotYield', 'ce54380b0273ebbdad5d3cb579efb33db7bfba1bf25b967e0ccdc8501f972b8a'),
    (0x2194fb0, 67, 'YieldIndexList.Read', '8c02f2ee47616cdbde2f284fa49559f9923c6b3068a660e9658f78804629cc0b'),
    (0x2194fa0, 5, 'YieldIndexList.Reset', '323602b65b354a24027856e729344b0cb29766a2be6a004660cdbfd49f4d7b25'),
    (0x2195000, 57, 'YieldIndexList.DoReset', '3a0faf2f771f989278a510161bf8a37fc4129aa0757249ce094288ccfb444ba9'),
    (0x2002e80, 149, 'Yield.ReadIndexHashArray', '94dd2bf8590edc193df1efd16e6b7ac79e302dfcd5e400314f193bd55e591af6'),
    (0x1ff7260, 34, 'Yield.HashToIndex', '97ccdf99110784245628a451ab9a17a107a9d6db0e9a11ee4812c603ecf1f454'),
    (0x212a1d0, 55, 'YieldIndexList.const_index', '48a45b23243b69c3170acca5e18e353c06a80ed36d6a9e461fa56d8b62de3a42'),
    (0x2dd91f0, 95, 'Feature.SparsePlotData.const_index', '26c64b904a1e821660e796e45d201ced4095ceac66fa5e8b924e7aa0bd0a9fcd'),
    (0x1fe25f0, 5, 'CityAttributes.GetWonderType', '4bb35de4f90a0d35d73c48bd802c0038cb98b5f55c665c02855ec492a4acb379'),
]
CONTRACTS = [
    (0x1fe25f0, '0fbf471ec3', 'wonder definition index is signed16'),
    (0x2195008, 'c7431800000000', 'IndexList reset default is zero'),
    (0x2002ed7, '85c0', 'hash overlay rejects missing negative index'),
    (0x2002edd, '498d3486', 'hash overlay writes supplied native index'),
    (0x2002ef0, '4889df4c89ee', 'unknown hash consumes value into temporary'),
    (0x2bf5e33, '41bc01000000', 'terrain multiplier starts at one'),
    (0x2bf5e39, '41bd02000000', 'double-adjacent multiplier is two'),
    (0x2bf5eb0, '450f45e5', 'neighbor doubling replaces multiplier rather than accumulating'),
    (0x2bf5ecd, '450faffc', 'terrain integer multiply wraps at32 bits'),
    (0x2bf5f23, '440f45f9', 'natural wonder replaces terrain contribution with zero'),
    (0x2bf6118, '4531c0', 'improvement receives null contribution output'),
    (0x2bf615d, '410fafc7', 'appeal percent multiplies at32 bits before division'),
    (0x2bf6269, '4139c4440f4ce0', 'city minimum uses signed maximum'),
    (0x2bf6342, '01442410', 'neighbor feature matching rows accumulate'),
    (0x2bf6674, '0fafc3', 'flood bonus multiply wraps at32 bits'),
    (0x2bf67b2, '0fafc3', 'terrain-class bonus multiply wraps at32 bits'),
    (0x2bf6857, '29c30f48d9', 'drought subtraction wraps then clamps signed negative to zero'),
]
CALLS = [
    (0x2bf5fb0, 0x1f625e0, 'GetYield queries actual Feature stored IndexList'),
    (0x2194fbd, 0x1e52000, 'wire IndexList reader resets before overlay'),
    (0x2194fe3, 0x1ec8b80, 'wire IndexList reads ordered hash/value overlay'),
    (0x2bf6126, 0x1e4c6a0, 'GetYield uses improvement wrapper'),
    (0x2bf6187, 0x1edd680, 'appeal integer quotient passes through fixed-point AsInt'),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    core = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    Elf(args.assets / 'Civ6', HOST_SHA)
    for address, size, name, expected in METHODS:
        actual = hashlib.sha256(core.read(address, size)).hexdigest()
        if actual != expected:
            raise ValueError('original numerical yield method differs: ' + name)
    for address, expected, name in CONTRACTS:
        if core.read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('original numerical yield contract differs: ' + name)
    for address, target, _ in CALLS:
        core.call(address, target)
    implementations = ['crates/c6-core/src/plot_yield.rs', 'crates/c6-core/src/improvement_yield.rs',
                       'crates/c6-core/src/yields.rs', 'crates/c6-core/src/world.rs',
                       'crates/c6-core/src/city_district.rs']
    result = dict(
        schema='c6-linux-plot-yield-source-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        methods=[dict(address=hex(a), bytes=n, name=s, sha256=h) for a,n,s,h in METHODS],
        instruction_contracts=[dict(address=hex(a), bytes=b, contract=s) for a,b,s in CONTRACTS],
        calls=[dict(address=hex(a), target=hex(t), contract=s) for a,t,s in CALLS],
        implementations=[dict(path=p, sha256=hashlib.sha256((ROOT/p).read_bytes()).hexdigest()) for p in implementations],
        implemented=['concrete Plot Rules.GetYield numerical branch sequence over typed rules and explicit live query providers',
                     'Feature manager sparse native IndexList reconstruction from owned World Map component',
                     'native six-name yield conversion over supplied actual globals',
                     'null-contribution improvement arithmetic composed with existing tech/civic helpers',
                     'known absent and native typed-map missing improvement yield-property keys'],
        unsupported=['uncaptured City/Player/resource/climate live query inputs',
                     'uncaptured original direction-ordered adjacency and improvement adjacency rules',
                     'foreign Plot/property subclasses and unimplemented variant conversion domains',
                     'nonnull contribution output and hypothetical map changes'],
        source_tests='cargo test -p c6-core plot_yield; cargo test -p c6-core improvement_yield',
        native_execution_performed=False, native_differential_acceptance=False,
        complete_state_action_turn_support=False,
        unknown_save_bytes='unclassified and unmasked; independent recorded89198-byte Save-to-Load failure remains',
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS), instruction_contracts=len(CONTRACTS),
                          native_execution_performed=False, output=str(args.output))))


if __name__ == '__main__':
    main()
