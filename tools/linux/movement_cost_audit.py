#!/usr/bin/env python3
"""Check movement-cost contracts in pinned Linux ELF bytes; never execute game code."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

METHODS = [[46061408,
  293,
  'GameCore::Rules::Movement::Instance::AlongRoute(GameCore::Formation::MovementCost::Values&)',
  '5000b9962213bc6a6090f6571bf0dc1d9fa07c2eb5fbc1b9eb8c9be951345e31'],
 [46060960,
  70,
  'GameCore::Rules::Movement::Instance::ToPlotCity(GameCore::Formation::MovementCost::Values&)',
  '729216c76745b54e0a3c92de5307b9faca0dc7ecfd11f771fe5a17d933eed678'],
 [46062512,
  443,
  'GameCore::Rules::Movement::Instance::Disembarking(GameCore::Formation::MovementCost::Values&)',
  '487f5047a89a542f21965179bc322b0c7daee7fbfbec3f58237b49c610d0d273'],
 [46061712,
  91,
  'GameCore::Rules::Movement::Instance::CrossingRiver(GameCore::Formation::MovementCost::Values&)',
  'c50b33e9117d8d7260cf305b20c987c073bcb4a9a80758ef37d273fe0a9d1a99'],
 [46061184,
  123,
  'GameCore::Rules::Movement::Instance::ToPlotFeature(GameCore::Formation::MovementCost::Values&)',
  '6881cff7897eb3ed8b756bb9fa4ceaf6842d745d635bcea26d82e242d24b58ea'],
 [46061040,
  136,
  'GameCore::Rules::Movement::Instance::ToPlotTerrain(GameCore::Formation::MovementCost::Values&)',
  'c719c27af20500fef89d8ad731b5a7659dd331611291de4850d3ffc3882e530b'],
 [46061808,
  253,
  'GameCore::Rules::Movement::Instance::CrossingPortal(GameCore::Formation::MovementCost::Values&)',
  '8733ffded244beaa59662feb06f72fb0262198cde712c0393127d1f852cf0290'],
 [46060464,
  164,
  'GameCore::Rules::Movement::Instance::IsRiverCrossing(GameCore::Map::Plot::Instance const*, '
  'GameCore::Map::Plot::Instance const*)',
  '058e188bf62381d5cf654ca7152deda8908d8f51907afebc1bce0bb8a71f1170'],
 [46061312,
  86,
  'GameCore::Rules::Movement::Instance::ToPlotImprovement(GameCore::Formation::MovementCost::Values&)',
  '188c49f63afec8724fcd55cda72dfffb98be73576471c41b200c9736653d661a'],
 [46060640,
  131,
  'GameCore::Rules::Movement::Instance::IsBridgeBetweenPlots(GameCore::Map::Plot::Instance const*, '
  'GameCore::Map::Plot::Instance const*)',
  '47cfe7f4683f14f2bff7cb383302bd1e2c333f3a4928dcb3432f7f310f039b1b'],
 [46062064,
  437,
  'GameCore::Rules::Movement::Instance::Embarking(GameCore::Formation::MovementCost::Values&)',
  '615c751c7ae459a3b722a124e811b35170f5f7b8bb2ec96de4fa6f4ef2c18863'],
 [43277936,
  9,
  'GameCore::Formation::MovementCost::Values::ChangeRegularCost(FixedPointT<8u>)',
  '298590d1ce62a125eb46adc9cdeaf702fb40d2db270809da4ae86c69e023a0d4'],
 [43278048,
  9,
  'GameCore::Formation::MovementCost::Values::ChangeToPlotExitCost(FixedPointT<8u>)',
  '987e6e029348fd6371cfaa305a7b178516d6325fb90f2a29e12d045ef870572a'],
 [43277952,
  48,
  'GameCore::Formation::MovementCost::Values::UpdateHardMinimumCost(FixedPointT<8u>)',
  'ce0db4b52b43e4a90796a7b849925e08ab310c3cf6044f4c83439172a8e55486'],
 [43278000,
  48,
  'GameCore::Formation::MovementCost::Values::UpdateSoftMaximumCost(FixedPointT<8u>)',
  '74a564fc7ceaa82adc7f0dedaaa893c9055ac1b52d48d7f3e163bb0d39ad34f8'],
 [43277632,
  148,
  'GameCore::Formation::MovementCost::Values::Values(GameCore::Formation::Instance const*, '
  'GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance const*, int, FixedPointT<8u>, '
  'FixedPointT<8u>, FixedPointT<8u>, GameCore::Formation::MovementCost::Values::ZOCValue)',
  '22e8008e5e3b4f13f57a08bf4ea10cff7e5c32739290fe3af62b37f28bf33dc0'],
 [43277504,
  120,
  'GameCore::Formation::MovementCost::Values::Values()',
  '691b1584e7307ea3a2f31504b2c7235ae2c3d97e3415d9cf7a79aa39acd4fc23'],
 [43287776,
  192,
  'GameCore::Formation::Movement::MovementCost(GameCore::Formation::Instance const*, '
  'GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance '
  'const*, int, FixedPointT<8u>, FixedPointT<8u>, unsigned int)',
  '5ed79d5b4197d5497244f9ca10c2d31007d5ee030a258a4693961013fcbcc824'],
 [43290992,
  188,
  'GameCore::Formation::Movement::MovementCost(GameCore::Formation::MovementCost::Values&, '
  'GameCore::Map::Plot::Instance const*, unsigned int)',
  '43b03228fec8f99f1d2b640ef0c692d680f1793ebafdbda1d9f5097a1b6f2555'],
 [43292480,
  246,
  'GameCore::Formation::Movement::IsSlowedByZOC(GameCore::Formation::Instance const*, '
  'GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance '
  'const*)',
  '3557f6be33f2c7daa0713a78c44cd183625cf19180798007c4af4ec95ab63b14'],
 [43290720,
  257,
  'GameCore::Formation::Movement::IsSlowedByZOC(GameCore::Formation::Instance const*, '
  'GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance '
  'const*, GameCore::Pathfinder::Instance*)',
  'daed895cd245d74ddfe8e6671f667bd8570f7d6b4aae52f1e4bc489564bdab23'],
 [43287600,
  170,
  'GameCore::Formation::Movement::MovementCostNoZOC(GameCore::Formation::Instance const*, '
  'GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance const*, GameCore::Map::Plot::Instance '
  'const*, int, FixedPointT<8u>, FixedPointT<8u>, unsigned int)',
  '5a403994301b30bf43a22552c95411be34a7fbcdae8e091f0da8718eb7b745d4'],
 [43292752,
  121,
  'GameCore::Formation::Movement::ApplyMovementRules(GameCore::Formation::MovementCost::Values&)',
  '16a77c160767a93b2bbc3f57c51f29e25b18a1a6514ceecbb2f78c698b43e20b'],
 [43277792,
  140,
  'GameCore::Formation::MovementCost::Values::GetFinalCost() const',
  'b76d004b229001badcf6e0d60e55d03a161fcecbcf53bb01325ff6ddcfef3ce1'],
 [43278064,
  18,
  'GameCore::Formation::MovementCost::Values::GetToPlotExitCost() const',
  'd73bac889143cd51cd1d6c488a838d684ed97790725ca4856feee5d7762dbae8'],
 [33352560,
  64,
  'FixedPointT<8u>::operator=(float)',
  'c640b5871f9c6e63fa0091b2623c4e63923c161f53dcba21a6c489238b361402'],
 [33348624,
  9,
  'FixedPointT<8u>::operator=(int)',
  '289ee59e91d4f68ddbe5f570b5002092702cd7aae484d25f21ca0576db55b96f'],
 [33444656,
  8,
  'FixedPointT<8u>::operator+=(FixedPointT<8u> const&)',
  '81e58c42b7d805beeed4b70b2cfd4565f3eaabb16e37e1baefa538f55502258f'],
 [46059328,
  852,
  'GameCore::Rules::Movement::Instance::IsInEnemyZOC(GameCore::PlayerTypes, GameCore::DomainTypes, '
  'GameCore::ReligionTypes, GameCore::LayeredPlotCoord, GameCore::Map::Plot::Instance const*, bool)',
  '7928c9bd2a468d5b392bde01e1531419347b26e42f1b3c056ad845723b488f45'],
 [33434416,
  8,
  'FixedPointT<8u>::operator>(FixedPointT<8u> const&) const',
  'ab00271575857734f9deafa1606796d9d87586ae02bc14e6486fe4c79f0cb86d'],
 [33442736,
  8,
  'FixedPointT<8u>::operator<(FixedPointT<8u> const&) const',
  '77df761629e8161df4d4d36cd655ed9337ef3b449ded7debc34049e32c284b77'],
 [43242896,
  192,
  'GameCore::Formation::Instance::GetIgnoreCost(GameCore::FeatureTypes) const',
  'b3d99573a43a6451c7f5fd4cd9d46829cc060477b7dc1cab1361e715265fb778'],
 [43242704,
  192,
  'GameCore::Formation::Instance::GetIgnoreCost(GameCore::TerrainTypes) const',
  '4b476bf066684ac658fc768cd256e8596ec55d63b364241aaa6952174059d64e'],
 [43243568,
  170,
  'GameCore::Formation::Instance::GetIgnoreRivers() const',
  'ea33c8a9016a9e35fd02c7e9d3810482bba30a0fcaa8239e50d3a0fa30614110'],
 [43243216,
  170,
  'GameCore::Formation::Instance::GetIgnoreShores() const',
  '573d70a2f1180afe15049990cf2ad6f611535d707ff10cb44de05151085507c2'],
 [33352528,
  5,
  'FixedPointT<8u>::FixedPointT(float)',
  'a8c01c4c23b78b6af9865efb0e3a9dbc7f7cfb973fc9bebd7cb33d4e75e36354'],
 [33343664,
  5,
  'FixedPointT<8u>::FixedPointT(int)',
  'a5050360463a3fe04639a9ad47d842d6a2c680138f54cfbbbe95eaf24f73e912']]

# Original call/tail-call sites in ApplyMovementRules, in execution order.
PHASES = [
    (0x2949865, 0x1f1b5f0, 'city'),
    (0x2949870, 0x1f1a7e0, 'terrain'),
    (0x294987b, 0x1ec05a0, 'feature'),
    (0x2949886, 0x1f71120, 'improvement'),
    (0x2949891, 0x1f36b60, 'route'),
    (0x294989c, 0x1f8de80, 'river'),
    (0x29498a7, 0x1ea3f60, 'portal'),
    (0x29498b2, 0x1ed6920, 'embark'),
    (0x29498c4, 0x1eb0400, 'disembark'),
]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    game = Elf(a.assets / 'libGameCore_XP2.so', GAME_SHA)
    Elf(a.assets / 'Civ6', HOST_SHA)
    methods = []
    for address, size, name, expected in METHODS:
        actual = hashlib.sha256(game.read(address, size)).hexdigest()
        if actual != expected:
            raise ValueError(f'movement method fingerprint mismatch at {address:#x}')
        methods.append(dict(address=hex(address), size=size, name=name, sha256=actual))
    for address, target, name in PHASES:
        raw = game.read(address, 5)
        if raw[0] != (0xe9 if name == 'disembark' else 0xe8):
            raise ValueError(f'phase opcode mismatch: {name}')
        if address + 5 + struct.unpack_from('<i', raw, 1)[0] != target:
            raise ValueError(f'phase target mismatch: {name}')
    for address, target in [(0x1fceb50, 0x1f49130), (0x1fcc8b0, 0x1f3afa0)]:
        raw = game.read(address, 5)
        if raw[0] != 0xe9 or address + 5 + struct.unpack_from('<i', raw, 1)[0] != target:
            raise ValueError('fixed-point constructor tail delegation mismatch')
    plot_slots = [(0x38, 0x2bca780, 'GetCityAttributes'), (0x48, 0x2bca7b0, 'HasCityAttributes')]
    for slot, target, name in plot_slots:
        if game.relocated_pointer(0x3ae2390 + 16 + slot) != target:
            raise ValueError(f'Plot virtual contract mismatch: {name}')
    if struct.unpack('<f', game.read(0x2dfac68, 4))[0] != 256.0:
        raise ValueError('fixed-point fraction multiplier mismatch')
    report = dict(
        schema='c6-linux-movement-cost-static-v1',
        game_sha256=GAME_SHA, host_sha256=HOST_SHA,
        scope='static phase/query contracts; no native execution or full movement acceptance',
        reference=dict(author='cru121', revision='5af389445118bed2bdebf7a8a42b219c8fba5175',
                       license='CC BY 4.0', attribution='docs/reference.md'),
        rust_source_sha256=hashlib.sha256((Path(__file__).resolve().parents[2] / 'crates/c6-core/src/movement.rs').read_bytes()).hexdigest(),
        methods=methods,
        phase_order=[name for _, _, name in PHASES],
        plot_virtual_slots=[dict(slot=hex(s), target=hex(t), name=n) for s, t, n in plot_slots],
        layout_basis='matching original Linux DWARF, corroborated by fingerprinted field-access methods',
        values_layout=dict(size=64, formation=0, from_plot=8, to_plot=16,
                           base_moves=24, max_moves=28, moves_remaining=32,
                           exit_moves=36, zoc=40, hard_minimum=44,
                           soft_maximum=48, regular=52, to_plot_exit=56),
        final_cost='max(min(regular, soft_maximum), hard_minimum); if <=0, replace with raw256',
        initial_accumulators=dict(hard_minimum=0, soft_maximum=2147483392, regular=0, to_plot_exit=0),
        gaps=['live loaded-unit differential corpus', 'complete affected-state and RNG comparison',
              'typed world/rules/modifier query integration', 'spatial ZOC callback and transitive queries',
              'movement legality and state mutation are separate and unimplemented'])
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(output=str(a.output), verified_methods=len(methods), phases=len(PHASES), native_execution=False)))


if __name__ == '__main__':
    main()
