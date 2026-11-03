#!/usr/bin/env python3
"""Verify bounded movement-world resolver contracts; never execute native code."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf, GAME_SHA

METHODS = [[35213776,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Terrain> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Terrain> const&)',
  '7673111b478a7794d6ea86c5453e31cd479c4063c6008ba12808f19fb31fe9fc'],
 [33456672,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> const&)',
  '032d4223226a530de41426be2de99309e0f8663a81e492c5bd5a1d7f27ad3bd0'],
 [46311648,
  149,
  'bool Serialization::ReadIndexHashArray<bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Terrain> '
  '>(Serialization::IStream&, bool*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Terrain> const&)',
  '3b00754a0ac9e4cf4e460b88fcde735885d394f3317cbd16c52388c86377c173'],
 [42332000,
  149,
  'bool Serialization::ReadIndexHashArray<bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> '
  '>(Serialization::IStream&, bool*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> const&)',
  'ff162f3d7981b85dcd3837c07df2702626062a3eed91dab244957fb27840ad48'],
 [46281200,
  3011,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Unit::Instance&)',
  'e39f2c7b9a358527bbf3ceed7ff41d9c5f5dccfcf6cb3f5802bafbc0d0f2d202'],
 [46284224,
  809,
  'GameCore::Unit::Instance::Reset()',
  'fab99e5056724f13df7ff04fc63deb460993434b984b0e5ac28682bcb97f076b'],
 [46300352,
  64,
  'GameCore::Unit::Instance::GetIgnoreTerrain(GameCore::TerrainTypes) const',
  '58d498d9c6c20e597b0d6ff82ee9ce56427df417dcc8847d3e1eeb5056adb91d'],
 [46300416,
  60,
  'GameCore::Unit::Instance::GetIgnoreFeature(GameCore::FeatureTypes) const',
  '68a3bf023667bc21abc7693740231ceef26618a0a4a3f989c69c081257a7d087'],
 [43248112,
  4,
  'GameCore::Unit::MovementModifiers::HasEscortMobility() const',
  '58b763300bb29ecc23f387d8b3f124621e2a847cbd2da8f7a062c036f9dbbb3e'],
 [43248128,
  4,
  'GameCore::Unit::MovementModifiers::CanIgnoreShores() const',
  'afa00825fc979d9be84f79e9640fefd5b357bf511000f681685961a91d03e020'],
 [43248160,
  4,
  'GameCore::Unit::MovementModifiers::CanIgnoreRivers() const',
  'b6faa9466eeb3016f1b0ec36a9ed63a4dd9b1f94fda0104cfc8bf81565aa965b'],
 [46067616,
  7,
  'GameCore::Formation::Instance::CanEmbark() const',
  'cdfcf7560575a6ccc85f406a8cf74e3102b0d6848e844c729c7f1a2e2b919499'],
 [33432592,
  5,
  'GameCore::Map::Plot::Instance::GetFeatureType() const',
  'e15cd107dd900e9a5cde4942b652e914b465e14bc34f4a9dac54d3d7864faa1d'],
 [33432624,
  5,
  'GameCore::Map::Plot::Instance::GetImprovementType() const',
  '2e364ec2b0b375cab6dbdd31e0d641f899ed78ab8c1e4c59df24ea436add56f4'],
 [34485664,
  5,
  'GameCore::Map::Plot::Instance::GetTerrainType() const',
  'aa944c3c94a1601bb7dcb02a65bb632b6eea536336fcc9795e16687e6337491c'],
 [34485616,
  8,
  'GameCore::Map::Plot::Instance::IsRoute() const',
  '04bf99a354ea1e96e60f7c3203ffba8e423dd494555be95550973fd9804f4f4c'],
 [45914064,
  80,
  'GameCore::Map::Plot::Instance::IsWater() const',
  'c935f8d2ca7eddae13a2d74b29312b175126f21390531088f67149d29098383b'],
 [45935120,
  8,
  'GameCore::Map::Plot::Instance::IsRoutePillaged() const',
  'a6baa22b323f77fd4f20f2719002e617a930af2d76e7e7ba2b7be0a728e05ea8'],
 [46067568,
  9,
  'GameCore::Map::Plot::Instance::IsRouteBridged() const',
  '708157ba3a3736e516febfc86b3dbb6e5c7545856316902a9702e1a53bc01431'],
 [45908336,
  50,
  'GameCore::Map::Plot::CityAttributes::IsCity() const',
  'ece6fcce1b5e82142d85e93b9298936a6dc23473380bb7b769dcdcf4b030f9be'],
 [40570592,
  156,
  'GameCore::Map::IInstance::GetDirectionToPlot(int, int, int, int) const',
  '23296a440fa97eee1da1f6d20cbcd75441eebac00d16737810b8b46bab355f60'],
 [40570752,
  19,
  'GameCore::Map::Plot::Instance::IsRiverCrossing(GameCore::DirectionTypes) const',
  'e064a29ae57a1a06285aef75827dd9b4054069729c2b62f230b1ad93bb8b2742'],
 [40570928,
  266,
  'GameCore::Map::IsAdjacentPlotCoord(GameCore::PlotCoord const&, GameCore::PlotCoord const&, int, '
  'int, bool, bool)',
  '1a760ac48688249fd4f1a98b11388f894a4d7fd7580e999a1f83169a7c12027a'],
 [33451056,
  27,
  'GameCore::Map::Hexspace::PlotXToX(int, int)',
  'ef7fa80e635393c23fb28022e3b5776e0726ac8ba056e1e34756e729abd8a692'],
 [33451120,
  27,
  'GameCore::Map::Hexspace::XToPlotX(int, int)',
  'ecd5d792efcce060e5d9a9c8cf81309197f8be32e680337c624bde85ae851aba'],
 [35027792,
  29,
  'GameCore::Map::WrapCoordDifference(int, unsigned int, bool)',
  '92b11eea1a422e36046ce00a5c0fdfe531f48e73d59f98cbf55fed757c57851d'],
 [37993216,
  28,
  'GameCore::Map::CoordRange(int, int, bool)',
  'dc7e6d453df9764c167593b8c1b68adccf891a0f82332a4c04809768fd3b3136'],
 [45766016,
  12,
  'GameCore::Player::Units::Find(unsigned int) const',
  'd198a658e55262febda648055233f9b1d2069441377f889fb95313bd318f7091'],
 [45786704,
  84,
  'RecycledIDContainer<GameCore::Unit::Instance, 8u, 16u>::Find(unsigned int) const',
  '23187b5ffe8639ccf9e9f356f40d40c171a4e81ef1d4488b412aa1262744a899']]

def verify_relative_call(elf, address, target):
    """Read five original ELF bytes and verify the relative CALL operand only."""
    raw = elf.read(address, 5)
    if raw[0] != 0xe8 or address + 5 + struct.unpack_from('<i', raw, 1)[0] != target:
        raise ValueError(f'relative call bytes mismatch: {address:#x}')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    methods = []
    for address, size, name, digest in METHODS:
        actual = hashlib.sha256(game.read(address, size)).hexdigest()
        if actual != digest:
            raise ValueError(f'movement resolver method mismatch: {name}')
        methods.append(dict(rva=hex(address), size=size, name=name, sha256=actual))
    x = struct.unpack('<6i', game.read(0x3c00480, 24))
    y = struct.unpack('<6i', game.read(0x3c004a0, 24))
    if x != (0, 1, 1, 0, -1, -1) or y != (1, 0, -1, -1, 0, 1):
        raise ValueError('direction constants mismatch')
    verify_relative_call(game, 0x2c23211, 0x1e609e0)
    for address in [0x2c23e26, 0x2c23e69, 0x2c23eac]:
        verify_relative_call(game, address, 0x1e1d4e0)
    for address in [0x2c23e52, 0x2c23e95, 0x2c23ed8]:
        verify_relative_call(game, address, 0x1f250d0)
    for address in [0x2c23e43, 0x2c23e86, 0x2c23ec9]:
        if game.read(address, 5) != bytes.fromhex('c644240400'):
            raise ValueError('Unit reset false argument mismatch')
    report = dict(schema='c6-linux-movement-world-static-v1',
                  gamecore_sha256=GAME_SHA, static_checks_passed=True,
                  original_code_executed=False, native_differential_acceptance=False,
                  methods=methods, direction_x=x, direction_y=y,
                  unit_reset_default=False,
                  unit_array_remapping='native collection count, sequential Find/hash overlay, unknown hashes skipped, last duplicate wins',
                  terrain_ignore_bounds='invalid-terrain vector; reads ignore-terrain vector',
                  plot_water_builtin_indexes=[15, 16],
                  city_district_center_hash='0x8a32007d',
                  coordinate_profile='positive dimensions; actual in-map coordinates; signed wrapping arithmetic',
                  unresolved=['runtime owning-player district lookup and complete/type state',
                              'CityAttributes validDomains', 'nonempty portal sparse-table reconstruction',
                              'active native rules/property exports and identity gate',
                              'ZOC, legality, action execution, complete state and turns'],
                  raw_save_load_different_bytes_retained=89198,
                  unknown_bytes_masked=0, full_gameplay_parity=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(static_checks_passed=True, methods=len(methods), native_execution=False)))

if __name__ == '__main__':
    main()
