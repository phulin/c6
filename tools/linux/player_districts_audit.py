#!/usr/bin/env python3
"""Pinned read-only Linux District contract and offline wire comparisons. No original execution."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from core_stream_index import sha
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
ROOT = Path(__file__).resolve().parents[2]
METHODS = [(33566336,
  149,
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> const&)',
  '94dd2bf8590edc193df1efd16e6b7ac79e302dfcd5e400314f193bd55e591af6'),
 (33567040,
  141,
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> const&)',
  'dba8d3f40b5faed3e3068fae08dc1b175b82035fe9be875b6e286b035753ce68'),
 (33567360,
  141,
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> const&)',
  'e0c72d445008f5bf338a0638e7bf47e84dfedf057538aaedf62aaf6efb7287c8'),
 (33581856,
  149,
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> const&)',
  'a0e07cea319b67f30b550c208e2cfbf72fb3d461929b95834e839be738521489'),
 (34759536,
  141,
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> const&)',
  'c78f8f69cf7da06801da2a3cce0b49cd64bf1ae3e1d9853e33e9ad574704c8a0'),
 (34773616,
  149,
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> const&)',
  '534a0ecacdbc6d5187361e1faa453b586ed2716783501298d6c5915dd9c57c2d'),
 (35211264,
  56,
  'Serialization::OStream& Serialization::operator<< <int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions>(Serialization::OStream&, GameCore::IndexList<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions, void> const&)',
  '297d39dc5168d2b4833c6d1fdcbf6b77b67b25169e6706634e1887aabfeba543'),
 (35211392,
  301,
  'void GameCore::IndexSerializer::YieldTypes_ReadArray<int>(Serialization::IStream&, '
  'GameCore::IndexList<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions, void>&, bool)',
  '108f40da8fc32a3e88adcb8b64f4bf5a945361a976f2d7fcac0d6370a06250d7'),
 (43109392,
  86,
  'Serialization::operator<<(Serialization::OStream&, GameCore::District::AdjacencyBonusMirror const&)',
  '76cc4547c4a67f5521257e46836fe0dd3bb49ee304de1be08d02e98dd217c122'),
 (43109488,
  83,
  'Serialization::operator>>(Serialization::IStream&, GameCore::District::AdjacencyBonusMirror&)',
  '378946fab84b69402680f56845a69a12f9a7116a107db2b2fc1b0626ee2cd77d'),
 (43109744,
  487,
  'Serialization::operator<<(Serialization::OStream&, GameCore::District::Instance const&)',
  'c86a88b70f37fbf88c31c4be0f5fc1e84a1c9d929be471a219ce3985a6698149'),
 (43110240,
  1103,
  'Serialization::operator>>(Serialization::IStream&, GameCore::District::Instance&)',
  '8946937f50875d90d39f6bd7b6c15a9960da7e1e81fbd09aac06b698569360b4'),
 (43125952,
  21,
  'Serialization::OStream& operator<< <eastl::vector<GameCore::District::AdjacencyBonusMirror, '
  'eastl::allocator>, GameCore::District::Instance>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>, '
  'GameCore::District::Instance> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (43125984,
  141,
  'bool Serialization::WriteIndexHashArray<FixedPointT<8u>, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonClass> '
  '>(Serialization::OStream&, FixedPointT<8u> const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonClass> const&)',
  'ff70064f3a9cadc72e964d01d85e1916ca0943fa274245975fb0fad593b65c4f'),
 (43126448,
  21,
  'Serialization::IStream& operator>><eastl::vector<GameCore::District::AdjacencyBonusMirror, '
  'eastl::allocator>, GameCore::District::Instance>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>, '
  'GameCore::District::Instance>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (43129760,
  157,
  'bool Serialization::ReadIndexHashArray<FixedPointT<8u>, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonClass> '
  '>(Serialization::IStream&, FixedPointT<8u>*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonClass> const&)',
  '3067cfe665f0590e78ed733e29c61802cd221a059fec8a77d9cafed462f67afe'),
 (43135744,
  15,
  'FAutoVariable<GameCore::DistrictTypes, GameCore::District::Instance>::load(Serialization::IStream&)',
  '3eb116914c84c8491eafd6fdf15adecaa3f54a85b9d2f7fbb2b6c766918aed8a'),
 (43135808,
  15,
  'FAutoVariable<GameCore::DistrictTypes, GameCore::District::Instance>::save(Serialization::OStream&) const',
  '16f9ac7ad8b1d687fb0d10603637640aca8fd7de48ef0c77d1add392f0717510'),
 (43138304,
  15,
  'FAutoVariable<GameCore::IndexList<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions, void>, '
  'GameCore::District::Instance>::load(Serialization::IStream&)',
  '0f2f6f0d90e7e28862f3dca5dea8d1b61a655e6e1a26f61d4f864ffe504e3c73'),
 (43138416,
  15,
  'FAutoVariable<GameCore::IndexList<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions, void>, '
  'GameCore::District::Instance>::save(Serialization::OStream&) const',
  'd20206949a5d7e5ac9cd41fcb1e64fa7ab4c97b49811e0d52da676bc40e57a33'),
 (43141040,
  15,
  'FAutoVariable<eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>, '
  'GameCore::District::Instance>::load(Serialization::IStream&)',
  '94c702d9800dbc57c809232ac58da880839af1d928623f32e9b313000c645396'),
 (43141056,
  9,
  'FAutoVariable<eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>, '
  'GameCore::District::Instance>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (43141072,
  73,
  'FAutoVariable<eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>, '
  'GameCore::District::Instance>::dummyLoadDelta(Serialization::IStream&)',
  'f48aa66099b11264021b466b375f25788c681bf2d70212b52d32f6f08742ac23'),
 (43141152,
  15,
  'FAutoVariable<eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>, '
  'GameCore::District::Instance>::save(Serialization::OStream&) const',
  '5763a7e135a91b1b44bf4b352cd04eb491d538b6df89c6af3c6112995e7147e8'),
 (43141168,
  9,
  'FAutoVariable<eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>, '
  'GameCore::District::Instance>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (43141312,
  1775,
  'FAutoVariable<eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>, '
  'GameCore::District::Instance>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&) const',
  'fdf4668f0a32529db65ba36b5b7a6d09bbb236f3e1ed1909820a74ebfb140131'),
 (43143744,
  14,
  'Serialization::IStream& '
  'Serialization::operator>><GameCore::District::AdjacencyBonusMirror>(Serialization::IStream&, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>&)',
  'aa1540e555f8363c65be07eab9040e18f418745064585d936e91da23a2b0c25a'),
 (43143760,
  125,
  'void Serialization::SerializeToSequenceContainer<GameCore::District::AdjacencyBonusMirror, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator> >(Serialization::IStream&, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator>&)',
  'e87565500cb8949a6a79854b9d060475bc7b7b8d6d70cc0235cff721296ec1f6'),
 (43145168,
  91,
  'Serialization::OStream& Serialization::operator<< '
  '<GameCore::District::AdjacencyBonusMirror>(Serialization::OStream&, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator> const&)',
  '2336c0807e8fd9e307fe214b266f6b29ce6c8018a8a54dc78b81c56f4c470a3c'),
 (43145264,
  71,
  'Serialization::SerializeFromSequenceContainer<GameCore::District::AdjacencyBonusMirror const, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator> const> '
  'eastl::for_each<GameCore::District::AdjacencyBonusMirror const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::District::AdjacencyBonusMirror const, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator> const> '
  '>(GameCore::District::AdjacencyBonusMirror const*, GameCore::District::AdjacencyBonusMirror const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::District::AdjacencyBonusMirror const, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator> const>)',
  'd25887a5113e3541603f3c8212a93987e804eabf85b0b66ead411aa197289019'),
 (43145344,
  46,
  'Serialization::SerializeFromSequenceContainer<GameCore::District::AdjacencyBonusMirror const, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator> '
  'const>::SerializeFromSequenceContainer(Serialization::OStream&, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator> const&)',
  '3fb980844c868d05db8a5ec4602835f812a38997bd1c727781452912dca737c1'),
 (43145392,
  8,
  'Serialization::SerializeFromSequenceContainer<GameCore::District::AdjacencyBonusMirror const, '
  'eastl::vector<GameCore::District::AdjacencyBonusMirror, eastl::allocator> '
  'const>::operator()(GameCore::District::AdjacencyBonusMirror const&)',
  '078ad4adf5486a0c14fd9ff18ac6684d25e82bb0b93897bb2eef48651b3dfdf6'),
 (43156800,
  15,
  'FAutoVariable<Data::TypedVariantMap, GameCore::District::Instance>::load(Serialization::IStream&)',
  'c6e91d6274bef13e31c7aea93aa4011388bcbf1b3d57301e8303b13799e97976'),
 (43156912,
  15,
  'FAutoVariable<Data::TypedVariantMap, GameCore::District::Instance>::save(Serialization::OStream&) const',
  'b3876a3c09dac2fc5cf74ba11f92a9077af25b190bf73b3845fdf971a4e2785d'),
 (44956816,
  107,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DistrictCreateYield const&)',
  '33148589d4198daf0dae37457864ea74e3720f0e1e2bee48b6c3d7ab47a1f995'),
 (44956928,
  105,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DistrictCreateYield&)',
  'a3ba16664c71e39418ed6219f5c9955653db9c1227b4c7cc313190601111cb99'),
 (44957040,
  59,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DistrictCreateUnit const&)',
  '277a05914fce952690f8d9df3d0bde6d4922c066d2d774bffc380edd16b6b4ec'),
 (44957104,
  57,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DistrictCreateUnit&)',
  '29992f5db2e4835cffc5e08ed770f63354c56c82accb172e243b83ed7d588980'),
 (44957168,
  71,
  'Serialization::operator<<(Serialization::OStream&, '
  'GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass const&)',
  '68054dd080c33cbed10b41d6907059589b10a6fe4f31233c096fa35052a72129'),
 (44957248,
  69,
  'Serialization::operator>>(Serialization::IStream&, '
  'GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass&)',
  '8536f4ca8f6dd523521f1927a691707faa41ed80a35146911af10b3bc6961f38'),
 (44957328,
  59,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DistrictUnlockTech const&)',
  'be54803dd7b3ea9f881b52826d1ab17149513956528f85fcf994c6f329fd594a'),
 (44957392,
  57,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DistrictUnlockTech&)',
  '7581ea2918090eacf21174d31d21d994c48d810db1cf18950f143b0b744d4378'),
 (44957456,
  59,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DistrictUnlockCivic const&)',
  'f158e635ffe1d567f7f58629e9e5ee1d7ee8ddc5d7e9b0f6e4d28f4233523d38'),
 (44957520,
  57,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DistrictUnlockCivic&)',
  'dd1b756ee47b41f6049f1eeb15cedef45cb37eb4cf1d562f3b8dd1efd1c6079d'),
 (44957584,
  325,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Districts const&)',
  'b3794b9c902ddf8ceae45f8aa7c4e0eefe0245cc1a3b73b797328427a45b1548'),
 (44957920,
  762,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Districts&)',
  'a2e0a23de368df51aee0402a116378239cf4c6be714f310e981645bf97252208'),
 (44969984,
  374,
  'Serialization::OStream& Serialization::operator<< <GameCore::District::Instance, 8u, '
  '16u>(Serialization::OStream&, RecycledIDContainer<GameCore::District::Instance, 8u, 16u> const&)',
  'acceb05731d494327fcd195730d8dbc965048c1e4a4d69202ba18ab6ced51883'),
 (44970400,
  21,
  'Serialization::OStream& operator<< <eastl::vector<GameCore::Player::DistrictCreateYield, '
  'eastl::allocator>, GameCore::Player::Districts>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>, '
  'GameCore::Player::Districts> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (44970432,
  21,
  'Serialization::OStream& operator<< <eastl::vector<GameCore::Player::DistrictCreateUnit, '
  'eastl::allocator>, GameCore::Player::Districts>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (44970464,
  21,
  'Serialization::OStream& operator<< '
  '<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, eastl::allocator>, '
  'GameCore::Player::Districts>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (44970496,
  21,
  'Serialization::OStream& operator<< <eastl::vector<GameCore::Player::DistrictUnlockTech, '
  'eastl::allocator>, GameCore::Player::Districts>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>, '
  'GameCore::Player::Districts> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (44970528,
  21,
  'Serialization::OStream& operator<< <eastl::vector<GameCore::Player::DistrictUnlockCivic, '
  'eastl::allocator>, GameCore::Player::Districts>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>, '
  'GameCore::Player::Districts> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (44970560,
  386,
  'Serialization::IStream& Serialization::operator>><GameCore::District::Instance, 8u, '
  '16u>(Serialization::IStream&, RecycledIDContainer<GameCore::District::Instance, 8u, 16u>&)',
  'bac737dc6a535e94d28ca9bc0a1d5795cd8caf2fe19d2cfdf446c4e126bf76b4'),
 (44971040,
  21,
  'Serialization::IStream& operator>><eastl::vector<GameCore::Player::DistrictCreateYield, '
  'eastl::allocator>, GameCore::Player::Districts>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>, '
  'GameCore::Player::Districts>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (44971072,
  21,
  'Serialization::IStream& operator>><eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (44971104,
  21,
  'Serialization::IStream& '
  'operator>><eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (44971136,
  34,
  'Serialization::IStream& Serialization::ReadVersioned<eastl::vector<GameCore::Player::DistrictUnlockTech, '
  'eastl::allocator>, eastl::vector<GameCore::Player::DistrictUnlockTech, '
  'eastl::allocator>&>(Serialization::IStream&, eastl::vector<GameCore::Player::DistrictUnlockTech, '
  'eastl::allocator>&, int, int&&, eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>&)',
  '8f1b01e8a304e9bd874bd0f8448f1a0cd3524a39db74565711a779a0ee73d30d'),
 (44971248,
  34,
  'Serialization::IStream& Serialization::ReadVersioned<eastl::vector<GameCore::Player::DistrictUnlockCivic, '
  'eastl::allocator>, eastl::vector<GameCore::Player::DistrictUnlockCivic, '
  'eastl::allocator>&>(Serialization::IStream&, eastl::vector<GameCore::Player::DistrictUnlockCivic, '
  'eastl::allocator>&, int, int&&, eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>&)',
  '104b409fe23e116662e5e2a119608bf1a6dc136806d04476015284824f8579ee'),
 (44978736,
  14,
  'Serialization::IStream& '
  'Serialization::operator>><GameCore::Player::DistrictUnlockTech>(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>&)',
  '2620efa6a615f80d23972ae4ad2cf94559c3dda657764d4f9cad127b3b9ff41e'),
 (44979392,
  125,
  'void Serialization::SerializeToSequenceContainer<GameCore::Player::DistrictUnlockTech, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator> >(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>&)',
  '625bcb0fd940c9561aef6aa0b9fa389114bfa78f49e25d1ffc0d1f9e92c97508'),
 (44980704,
  14,
  'Serialization::IStream& '
  'Serialization::operator>><GameCore::Player::DistrictUnlockCivic>(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>&)',
  '6de9e3c241632adcca5d9f41695cb265ee7f50ec8b2d3f11809c969e91a7c6ba'),
 (44981360,
  125,
  'void Serialization::SerializeToSequenceContainer<GameCore::Player::DistrictUnlockCivic, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator> >(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>&)',
  'f820d7e1b0f518ae7f4016367fec2e3c13fc22132657fcefcfc3600951e9877e'),
 (44991184,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>, '
  'GameCore::Player::Districts>::load(Serialization::IStream&)',
  '598dac86b776b0af7c6b5b42b8f6e94aa2ae51a773cf526c4df9e20e7823c0d5'),
 (44991200,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>, '
  'GameCore::Player::Districts>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (44991216,
  73,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>, '
  'GameCore::Player::Districts>::dummyLoadDelta(Serialization::IStream&)',
  'dc5fce9ce6162c6e69c50492e247eff1a6e60dc7a6c4c9b85babc9ef4b6e23cb'),
 (44991296,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>, '
  'GameCore::Player::Districts>::save(Serialization::OStream&) const',
  'cd067e7badeaead7e65118c45b1dbce10166a616465c16cea84a31467709592e'),
 (44991312,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>, '
  'GameCore::Player::Districts>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (44991456,
  1775,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>, '
  'GameCore::Player::Districts>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&) const',
  'f620987b89cef3dbffeb775eea2983f8f4f280822fbeb3d16fc2ab50279c6193'),
 (44993888,
  14,
  'Serialization::IStream& '
  'Serialization::operator>><GameCore::Player::DistrictCreateYield>(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>&)',
  '2d8be939a021992bc28a206fb30f33cb998d9a62ce4218338e12c4d7669f1ff9'),
 (44993904,
  125,
  'void Serialization::SerializeToSequenceContainer<GameCore::Player::DistrictCreateYield, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator> >(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator>&)',
  '730eb38fbbd15842ba277ff6bb84ebb0d4b38871035b3643dfba64913e0c6ae7'),
 (44995312,
  91,
  'Serialization::OStream& Serialization::operator<< '
  '<GameCore::Player::DistrictCreateYield>(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator> const&)',
  '987ccded4a08691e37f6b20e08be077844664e068337d66f9c07f5a6d8d876dd'),
 (44995408,
  71,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateYield const, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator> const> '
  'eastl::for_each<GameCore::Player::DistrictCreateYield const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateYield const, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator> const> '
  '>(GameCore::Player::DistrictCreateYield const*, GameCore::Player::DistrictCreateYield const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateYield const, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator> const>)',
  '71dbbd696c708848b97714913ae5aba212d7ddc0dd8b3d5245c0c95f7269fc06'),
 (44995488,
  46,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateYield const, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator> '
  'const>::SerializeFromSequenceContainer(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator> const&)',
  'beca1b43fa66fd10e9861428e5292adf100752297a9168cf0d490f1c81f6496b'),
 (44995536,
  8,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateYield const, '
  'eastl::vector<GameCore::Player::DistrictCreateYield, eastl::allocator> '
  'const>::operator()(GameCore::Player::DistrictCreateYield const&)',
  'fb454a039cacdb4086af0819a49231ebcbf101023a40d37963c5d1c10bb57236'),
 (44996416,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts>::load(Serialization::IStream&)',
  'a9ea9709d7ccce79bbacefbe7182933225f1ad3f5e3ec58840d24258a1e552dd'),
 (44996432,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (44996448,
  73,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts>::dummyLoadDelta(Serialization::IStream&)',
  '584dc5f4262759eb22f6dd77de29af36f3ea146b290321a275d3f6630f452526'),
 (44996528,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts>::save(Serialization::OStream&) const',
  '76aeec9c6f39b8e3faab8563190da1570c1882b573ff6c7ca331d27c4588b79c'),
 (44996544,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (44996688,
  1775,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>, '
  'GameCore::Player::Districts>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&) const',
  'e999b3a10c73f42ac50b28a2134a8306b3a5873713f40edec97600397907bb8e'),
 (44999120,
  14,
  'Serialization::IStream& '
  'Serialization::operator>><GameCore::Player::DistrictCreateUnit>(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>&)',
  '877b152c7c0632059939c9142012ee9e987b041867618182b732493364cb95ea'),
 (44999136,
  125,
  'void Serialization::SerializeToSequenceContainer<GameCore::Player::DistrictCreateUnit, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator> >(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator>&)',
  '9addc64b6b81424841201599b357c3277e6a9a2270a3c3ccf4446c382f7a9f53'),
 (45000496,
  91,
  'Serialization::OStream& Serialization::operator<< '
  '<GameCore::Player::DistrictCreateUnit>(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator> const&)',
  '75b4f10ce6aed4c15c913d075c15eab003afdea3d68c2b9ebae1670465f99758'),
 (45000592,
  71,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateUnit const, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator> const> '
  'eastl::for_each<GameCore::Player::DistrictCreateUnit const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateUnit const, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator> const> '
  '>(GameCore::Player::DistrictCreateUnit const*, GameCore::Player::DistrictCreateUnit const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateUnit const, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator> const>)',
  '08ab526e807d4f78d57f75ecee845ae2106a7ae1e9704d858f7acf436a091115'),
 (45000672,
  46,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateUnit const, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator> '
  'const>::SerializeFromSequenceContainer(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator> const&)',
  '40a71a3d51a46bd1ee44066b58f220bb70013565132f4260eae2c3f63148f4de'),
 (45000720,
  8,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictCreateUnit const, '
  'eastl::vector<GameCore::Player::DistrictCreateUnit, eastl::allocator> '
  'const>::operator()(GameCore::Player::DistrictCreateUnit const&)',
  '349bcab13f32704fb10ca912f8907984200d643c89feed4f5f311235912c033b'),
 (45001552,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts>::load(Serialization::IStream&)',
  '7b638efbf958f8075c757205f9093f31b55bd6653da07e6ddb0c0a2906d629c9'),
 (45001568,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (45001584,
  73,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts>::dummyLoadDelta(Serialization::IStream&)',
  '10a663e6613b2a0e6b5231a291ed78535ab8b9eec7b6f218b3c969b53d6d0782'),
 (45001664,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts>::save(Serialization::OStream&) const',
  '6094b2da1359f09ba79ec8ee672163bad29d6bfb8e4ce863c84a9004b368ebeb'),
 (45001680,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (45001824,
  1775,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator>, GameCore::Player::Districts>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&) const',
  '9745d2f01383edf0b094fea133fe564c703a9bba5915ca90f630cf13fbd7592a'),
 (45004256,
  14,
  'Serialization::IStream& '
  'Serialization::operator>><GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass>(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, eastl::allocator>&)',
  '750f9fe13b9be78d86659537807e323199b11ab9481ae1aad25915451b92bb64'),
 (45004272,
  125,
  'void '
  'Serialization::SerializeToSequenceContainer<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, eastl::allocator> '
  '>(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, eastl::allocator>&)',
  '481ba86c2661779ce15a2086e8e128837de8a27ad1d05b81e832b8555727e20e'),
 (45005680,
  91,
  'Serialization::OStream& Serialization::operator<< '
  '<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass>(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, eastl::allocator> '
  'const&)',
  '2d3297a60b3a121c6c93ce1faf638cb4b50a0b77eca1b80b3ec4c02991eab5bb'),
 (45005776,
  71,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass '
  'const, eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator> const> '
  'eastl::for_each<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass '
  'const, eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator> const> >(GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass const*, '
  'GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass '
  'const, eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator> const>)',
  'f8c0bc122164469912970573012386ee7e2138f1825a972ce4cf36b67f6cb1f8'),
 (45005856,
  46,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass '
  'const, eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator> const>::SerializeFromSequenceContainer(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, eastl::allocator> '
  'const&)',
  '89a08b91553048e44c9db951d3f07ec49a71aa94277f1821999aec916229354a'),
 (45005904,
  8,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass '
  'const, eastl::vector<GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass, '
  'eastl::allocator> const>::operator()(GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass '
  'const&)',
  '06103b5ad61182ad754819d5a64b846b3355ac9e8484b9a59aa6f4f6f99c43e8'),
 (45006784,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>, '
  'GameCore::Player::Districts>::load(Serialization::IStream&)',
  '317a79c40a10a910a1ec8cf0e2fa7d1acf80b80d6b87337bfacd143c1156ec5e'),
 (45006800,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>, '
  'GameCore::Player::Districts>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (45006816,
  73,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>, '
  'GameCore::Player::Districts>::dummyLoadDelta(Serialization::IStream&)',
  '38a0b1e78b5f0faa7dee1241e0ec93153404a54ec8b9131318707f40d68640db'),
 (45006896,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>, '
  'GameCore::Player::Districts>::save(Serialization::OStream&) const',
  '1da4245f74a99910655dda1501b05add05722b121a23c0b2136093d107deafa1'),
 (45006912,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>, '
  'GameCore::Player::Districts>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (45007056,
  1775,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator>, '
  'GameCore::Player::Districts>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&) const',
  '7f7ef33428dfee5dc8eec76c4fe23ba6bb4bab02c45daf71404ec49857cfd5e0'),
 (45009568,
  91,
  'Serialization::OStream& Serialization::operator<< '
  '<GameCore::Player::DistrictUnlockTech>(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator> const&)',
  '8454aa9ea4fc652729a6c64f57087bf5f52a3c77b3e53f10b4366acbfc6dd0bb'),
 (45009664,
  71,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockTech const, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator> const> '
  'eastl::for_each<GameCore::Player::DistrictUnlockTech const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockTech const, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator> const> '
  '>(GameCore::Player::DistrictUnlockTech const*, GameCore::Player::DistrictUnlockTech const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockTech const, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator> const>)',
  'b19616f08416a0468e07edbf15bb5201660a8f215955487e9a94320de7bd805f'),
 (45009744,
  46,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockTech const, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator> '
  'const>::SerializeFromSequenceContainer(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator> const&)',
  'f01eb264c29d40bdbd755dc9419c5a7c1c8dd4c57f9b4aa540bb5a456d2277b4'),
 (45009792,
  8,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockTech const, '
  'eastl::vector<GameCore::Player::DistrictUnlockTech, eastl::allocator> '
  'const>::operator()(GameCore::Player::DistrictUnlockTech const&)',
  'ebd82d01bfbbc0b05c6d3b9b3a4f86da417c21a7482a7c8e08e5f32459b04297'),
 (45010608,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>, '
  'GameCore::Player::Districts>::load(Serialization::IStream&)',
  'e3ff6e97d1ed7533e45d8a1ffe0464d051eacaf8cd0c21cc83e75cbd20358a18'),
 (45010624,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>, '
  'GameCore::Player::Districts>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (45010640,
  73,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>, '
  'GameCore::Player::Districts>::dummyLoadDelta(Serialization::IStream&)',
  '061bf06d3a64941b69c6eb4826945163403aa2a6cdac53c052e951f4f4a4a551'),
 (45010720,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>, '
  'GameCore::Player::Districts>::save(Serialization::OStream&) const',
  'b73800715ffca17a36f4733bf2956230585b9491fc31abad7b9e929ae2a5c141'),
 (45010736,
  9,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>, '
  'GameCore::Player::Districts>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (45010880,
  1775,
  'FAutoVariable<eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator>, '
  'GameCore::Player::Districts>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&) const',
  '6ea2ea321a227c9d6e0941f20a2d25af361a96aff92c047729760499d5447bd5'),
 (45013392,
  91,
  'Serialization::OStream& Serialization::operator<< '
  '<GameCore::Player::DistrictUnlockCivic>(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator> const&)',
  '2eb7718db94b07028b5ab0a137fc93feb1d3c4fd4bdc485efbfabb621f49266a'),
 (45013488,
  71,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockCivic const, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator> const> '
  'eastl::for_each<GameCore::Player::DistrictUnlockCivic const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockCivic const, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator> const> '
  '>(GameCore::Player::DistrictUnlockCivic const*, GameCore::Player::DistrictUnlockCivic const*, '
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockCivic const, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator> const>)',
  '6d9619410e455c35856f5c4df6c82b4cc806f9e086bbd1e819441c62f1c2e726'),
 (45013568,
  46,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockCivic const, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator> '
  'const>::SerializeFromSequenceContainer(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator> const&)',
  '1741e60bf224d7450f972d0474570c2ead2728e7dda6444d58dfc7d280326f78'),
 (45013616,
  8,
  'Serialization::SerializeFromSequenceContainer<GameCore::Player::DistrictUnlockCivic const, '
  'eastl::vector<GameCore::Player::DistrictUnlockCivic, eastl::allocator> '
  'const>::operator()(GameCore::Player::DistrictUnlockCivic const&)',
  '1292cd51cd0aa313f528de4ce088b3b28777de6bc747a18475a922ae50fbcfb7'),
 (47508336,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::CivicTypes const&)',
  '259301d4abef819cc377acf6436d4036a2241bb2b57f9241fe4aea3a13ac0f02'),
 (47508384,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::CivicTypes&)',
  '23ed427d7285eff6b631c259b039d49a30273a5e0f8af7daa84500f258d2588f'),
 (47508448,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::DistrictTypes const&)',
  'dc40e9bc43e0fc8aec2e195881b9a1c8eab8c366272b24f46990d7fb1231425c'),
 (47508496,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::DistrictTypes&)',
  '8f2608b388942db0f3d64ebc8e0e8f9cedd91acf351ae7e3d5cbdf710883f92d'),
 (47509008,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::ImprovementTypes const&)',
  '553cc1de61effac943a18e8873ec9df614ca5f5c40d8631d2077791eb977a632'),
 (47509056,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::ImprovementTypes&)',
  '2f6a637a7c44e832fe7172a40794a4f17093b31eceb0f307447e72e9cb1c357b'),
 (47509232,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::TechTypes const&)',
  'e8b21f56f96030386a4c9aac4582d7fce63120854b0a642cf15392ac5210cdad'),
 (47509280,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::TechTypes&)',
  '047050d264acc31d6a4477873412f54329d63a12b971850fa8ca7602c1cfa31c'),
 (47510224,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::UnitAbilityTypes const&)',
  '2f885c059351bfe8decd9c94aa6474fdcc3991fa5b27336df27d18d50eaae03d'),
 (47510272,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::UnitAbilityTypes&)',
  'eb2eec5fafba1439f8cc549711cf96580f60febd055716ea95b863f07e01d978'),
 (47512432,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::YieldTypes const&)',
  'b971caad466bb8cb3baf0b50ec89d1f83d536c9d25f15145cc664077fdd4a5c7'),
 (47512480,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::YieldTypes&)',
  '8f3a33cbd2aca7db1a0c6d4e9fae3b8ea968171068bccd693ea5ca8982362a12'),
 (47514352,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::UnitPromotionClassTypes const&)',
  '821b0ff788c56d14d16ee196521bb6c3123fad502e2cb692d159c6b7c0c59e32'),
 (47514400,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::UnitPromotionClassTypes&)',
  'b35a49271ce4cac08084f04204907ee4a853537a44ceb38d57799593e05b6a48')]
HOST_METHODS = [(60232496, 480, '5e28e9973784847874797906847e9c5e2093d6245c60eec23b804bcbfc4a0369'), (60169808, 196, 'e45d5808a5c8cc683ef5a06060228f21ad81ed49d6d12a9043e4255e0236b1da')]
VERSIONS = [('District',0x2f30b08,15),('CreateYield',0x2f5d770,1),('CreateUnit',0x2f5d774,1),
 ('CreateUnitWithAbility',0x2f5d778,1),('UnlockTech',0x2f5d77c,1),('UnlockCivic',0x2f5d780,1),
 ('PlayerDistricts',0x2f5d784,8),('Recycled',0x2f5d788,65537)]
FIXTURES = [('ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884','trace-benchmark-1.jsonl.gz','528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
 ('85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8','trace-captured-1.jsonl.gz','027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')]
HARNESS = r'''
pub use c6_core::{map,unit};
#[path="MODULE"] pub mod player_districts;
use player_districts::{District,Districts};
fn main() {
 let args:Vec<_>=std::env::args().collect();
 let input=std::fs::read(&args[2]).unwrap();
 let limits=unit::Limits{max_bytes:64*1024*1024,max_items:4_000_000};
 if args[1]=="district" {
   let (v,n,items)=District::decode_prefix_with_items(&input,limits).unwrap();
   assert_eq!(n,input.len()); let (out,write_items)=v.encode_with_items(limits).unwrap(); assert_eq!(items,write_items);
   std::fs::write(&args[3],out).unwrap();
   println!("{} {} {} {} {} {} {} {} {}",n,items,v.id,v.location.0,v.location.1,v.city_id,v.district_hash,u8::from(v.is_complete),v.yield_adjacency_mirrors.len());
 } else {
   let (v,n,items)=Districts::decode_prefix_with_items(&input,limits).unwrap(); assert_eq!(n,input.len());
   let (out,write_items)=v.encode_with_items(limits).unwrap(); assert_eq!(items,write_items);
   std::fs::write(&args[3],out).unwrap();
   println!("{} {} {} {} {} {} {} {} {} {} {} {} {}",n,items,v.records.capacity,v.records.entries.len(),v.records.current_id_bits,v.records.free_head,v.records.free_tail,v.records.free_links.len(),v.creation_yields.len(),v.creation_units.len(),v.district_and_buildings_creation_units.len(),v.unlock_techs.len(),v.unlock_civics.len());
 }
}
'''

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    game=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    host=Elf(args.assets/'Civ6',HOST_SHA)
    for a,n,name,digest in METHODS:
        if sha(game.read(a,n))!=digest:raise ValueError('method differs: '+name)
    for name,a,v in VERSIONS:
        if struct.unpack('<I',game.read(a,4))[0]!=v:raise ValueError('version differs: '+name)
    bindings=[(0x53b3680,0x1f0,0x3971330),(0x53b3680,0x1f8,0x3961e50)]
    for table,slot,target in bindings:
        if struct.unpack('<Q',host.read(table+16+slot,8))[0]!=target:raise ValueError('typed property virtual binding differs')
    # Complete inherited providers, shared with City and Unit property transport.
    host_methods=[]
    for a,n,digest in HOST_METHODS:
        if sha(host.read(a,n))!=digest:raise ValueError('host property method differs')
        host_methods.append(dict(address=hex(a),size=n,sha256=digest))
    checks=[(0x291ce1d,'488dbb80030000'),(0x291ce2b,'ba01000000'),(0x291ce33,'ff90f8010000'),
            (0x291d029,'ff91f0010000')]
    for a,raw in checks:
        if game.read(a,len(bytes.fromhex(raw))).hex()!=raw:raise ValueError('District properties concrete call differs')
    types=['GameCore::District::Instance','GameCore::District::AdjacencyBonusMirror','GameCore::Player::Districts',
        'GameCore::Player::DistrictCreateYield','GameCore::Player::DistrictCreateUnit',
        'GameCore::Player::DistrictAndBuildingsCreateUnitWithAbilityByClass','GameCore::Player::DistrictUnlockTech','GameCore::Player::DistrictUnlockCivic']
    command=['gdb','-nx','-batch',str(args.assets/'libGameCore_XP2.so')]
    for name in types:command+=['-ex',"ptype /o '"+name+"'"]
    dwarf=subprocess.check_output(command,text=True)
    for name in ['m_iGarrisonDamage','m_aYieldAdjacencyMirrors','m_iWithinOneHexEspionageDefenseBonus',
        'm_aAppealYieldChanges','m_districtBuildMap','m_iMirrorCount','m_bMustReplaceImprovement','m_ePromotionClass','m_eTechnology','m_eCivic']:
        if name not in dwarf:raise ValueError('matching DWARF differs: '+name)
    source_paths=[ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'crates/c6-core/Cargo.toml',*sorted((ROOT/'crates/c6-core/src').rglob('*.rs'))]
    sources={str(x.relative_to(ROOT)):sha(x.read_bytes())for x in source_paths}
    subprocess.run(['cargo','build','--offline','-p','c6-core'],cwd=ROOT,check=True)
    if sources!={str(x.relative_to(ROOT)):sha(x.read_bytes())for x in source_paths}:raise ValueError('sources changed during dependency build')
    reports=[]
    with tempfile.TemporaryDirectory(prefix='c6-player-districts-')as tmp:
        tmp=Path(tmp);module=ROOT/'crates/c6-core/src/player_districts.rs'
        harness=HARNESS.replace('MODULE',str(module));(tmp/'candidate.rs').write_text(harness)
        base=['rustc','--edition=2021',str(tmp/'candidate.rs'),'--extern','c6_core='+str(ROOT/'target/debug/libc6_core.rlib'),'-L','dependency='+str(ROOT/'target/debug/deps')]
        subprocess.run(base+['-o',str(tmp/'candidate')],check=True)
        subprocess.run(base+['--test','-o',str(tmp/'tests')],check=True)
        subprocess.run([str(tmp/'tests')],check=True)
        candidate_sha=sha((tmp/'candidate').read_bytes())
        for identity,trace_name,trace_sha in FIXTURES:
            data=gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
            if sha(data)!=identity:raise ValueError('original Core differs')
            trace=gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
            if sha(trace)!=trace_sha:raise ValueError('original owner trace differs')
            rows=[json.loads(x)for x in trace.splitlines()]
            rows=[x for x in rows if x.get('event')=='owner'and x.get('save')==1]
            record_counts={};total_bytes={};observations={};input_digests={}
            for kind,owner in [('district','GameCore::District::Instance'),('districts','GameCore::Player::Districts')]:
                cuts=sorted([x for x in rows if x['owner']==owner],key=lambda x:x['start'])
                obs=[];digests=[];count=0
                for cut in cuts:
                    payload=data[cut['start']:cut['end']]
                    (tmp/'in').write_bytes(payload)
                    result=subprocess.check_output([str(tmp/'candidate'),kind,str(tmp/'in'),str(tmp/'out')],text=True)
                    if (tmp/'out').read_bytes()!=payload:raise ValueError('original record roundtrip differs')
                    words=[int(x)for x in result.split()]
                    if words[0]!=len(payload):raise ValueError('native validation cut end differs')
                    if kind=='district':
                        # Independent prefix read; no table-owner inference.
                        expected=list(struct.unpack_from('<IiiII',payload,4))+[payload[44]]
                        if words[2:8]!=expected:raise ValueError('typed District prefix differs')
                    count+=len(payload);obs.append(words);digests.append(sha(payload))
                record_counts[kind]=len(cuts);total_bytes[kind]=count
                observations[kind]=sha(json.dumps(obs,separators=(',',':')).encode())
                input_digests[kind]=sha(json.dumps(digests,separators=(',',':')).encode())
            reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=record_counts,
                record_bytes=total_bytes,typed_observations_sha256=observations,ordered_input_record_digests_sha256=input_digests))
    report=dict(schema='c6-linux-player-districts-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
        methods=[dict(address=hex(a),size=n,name=name,sha256=digest)for a,n,name,digest in METHODS],
        versions=[dict(name=name,address=hex(a),value=v)for name,a,v in VERSIONS],host_methods=host_methods,
        host_property_bindings=[dict(table=hex(a),slot=hex(s),target=hex(t))for a,s,t in bindings],
        property_call_checks=[dict(address=hex(a),bytes=raw)for a,raw in checks],dwarf_sha256=sha(dwarf.encode()),
        audit_source_sha256=sha(Path(__file__).read_bytes()),candidate_sources=sources,candidate_sha256=candidate_sha,
        harness_sha256=sha(harness.encode()),fixtures=reports,original_execution=False,private_writes=False,
        complete_current_district_wire=True,complete_current_player_districts_wire=True,
        native_read_fixups=False,definition_remapping=False,actual_district_owner_reconstructed=False,
        full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,
        unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(output=str(args.output),methods=len(METHODS),fixtures=reports,original_execution=False)))
if __name__=='__main__':main()
