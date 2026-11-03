#!/usr/bin/env python3
"""Read-only SHA-gated Player.Religion v20 contracts and offline source records.

Never executes original code; owner traces supply validation cuts only.
"""
import argparse
import gzip
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from core_stream_index import sha
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from player_districts_audit import FIXTURES
ROOT = Path(__file__).resolve().parents[2]
METHODS = [(33341744,
  14,
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)',
  '2a3fc110a86f71122b45f03f192ddbc4499241d298e49b843ea750bd49d5e1cf'),
 (33341776,
  14,
  'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)',
  '9731ca1d2959dc0f406a684267a29d425c9bf7ea2f127425123508fbc2ed2d3b'),
 (33341856,
  14,
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)',
  'c6cf9d3175d5134bd8d7b3d396184fd7b9abd3f7ca364cbdc38a1794ce742775'),
 (33341888,
  14,
  'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)',
  '5edd23b31cc38a2cc74b61c4b33889da2f35c69f9ce10ee24f2899eb7e91af01'),
 (33348288,
  14,
  'Serialization::OStream::WriteObject(int const&)',
  '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'),
 (33348304,
  75,
  'Serialization::IStream::ReadObject(int&)',
  '09ddfd4d815a2204789075821ff06a3e8d881dc2bae4791ff3f746206deb3230'),
 (33348384,
  25,
  'Serialization::IStream::ReadRequest::ReadRequest(unsigned long, unsigned long)',
  '43f291f25ae0e59e98bf6b96e5341ed0345d1627c75c056bac01ada2cfd2f796'),
 (33348512,
  14,
  'Serialization::OStream::WriteObject(unsigned int const&)',
  '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'),
 (33348528,
  75,
  'Serialization::IStream::ReadObject(unsigned int&)',
  'b5c97ed9421b7f58e0ea8a7e2989ec1c05dd9c16a79416af11d59bd45e57dc3e'),
 (33431440,
  14,
  'Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)',
  '441a140d71c552a6ced859e0e9efa0994c519cad451b975cbb5fc13e8025cd3f'),
 (33431552,
  14,
  'Serialization::IStream& Serialization::IStream::operator>><bool>(bool&)',
  '8354dc947302541cc1e97c714b223e16c8e9f086c7d09ad77b437291a622809d'),
 (33447552,
  28,
  'Serialization::IStream& Serialization::ReadVersioned<bool, bool>(Serialization::IStream&, bool&, int, '
  'int&&, bool&&)',
  '57e8e97c7be2741a1dc218f74049b9cf7a99191cffd41785ce1aa9dd4c6518e5'),
 (33454336,
  14,
  'Serialization::OStream::WriteObject(bool const&)',
  '0b101fff740deefceb76f32a685baf980dce55ee6f804dd16993e783e33aebba'),
 (33456544,
  75,
  'Serialization::IStream::ReadObject(bool&)',
  '1b35f5e559410da6d1adcde91f94b8b0ee9a16f84d871535a2d18c3ae544487b'),
 (33488400,
  14,
  'Serialization::IStream& Serialization::operator>><int, Types::LinearAlloc::Default<1u, 0u> '
  '>(Serialization::IStream&, Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >&)',
  'be5ba0a1913c43eb3a03e109b552c1b3d7d559821096d6f9c59b0ae72f08e878'),
 (33488416,
  125,
  'void Serialization::SerializeToSequenceContainer<int, Types::Vector<int, Types::LinearAlloc::Default<1u, '
  '0u> > >(Serialization::IStream&, Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >&)',
  '0906c9608ac801fcde663528b0adc6569442b13dcb95725cc55a877fa7cfb8c4'),
 (33536976,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  'const&)',
  'd33e4a5fb4dadf218364b8a20da36838ab796c11f42bc2b47327f41c0ff5a465'),
 (33563776,
  28,
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)',
  '23f9d1493d61ddb5d8df9b32f1d08543a6eabe7d8b75c1ff9463c7772bc52442'),
 (33563840,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> const&)',
  'dafb3aecb2908606aaceb1a0e7e4829c3b36c03c4127d40bef8cbf7fd7918c6d'),
 (33576096,
  66,
  'unsigned int '
  'Serialization::IndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  'const&)',
  '7db73dfe0669efcaf4e9ca33d27a67f3d2e51aed3f6cbdc00134176a2c5199b1'),
 (33645696,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  'const&)',
  'bbba3422fa0bd6194a1c30c88ee7b8e77f8f7f0733c020e55d5c967202a1dd21'),
 (34512800,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> const&)',
  'bb8836acfe149431c085ea284f1a3022e8e1508430ae8d79bd99f8d3ac731d95'),
 (42478512,
  30,
  'Serialization::IStream& Serialization::ReadVersioned<bool, int>(Serialization::IStream&, bool&, int, '
  'int&&, int&&)',
  '1c8f8e7ace5176888574feb86c69b85069de8d1ce361bb96017c63e7ba32d50f'),
 (45469728,
  95,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::BeliefYield const&)',
  '6690d8aea786ec5de17ecad88af5682f943d16104b685d391195646b643fe1b1'),
 (45469824,
  126,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::BeliefYield&)',
  '461d49f60f250bfb319c2345d7b6eef37035b3ce82fc3a933e902e1b6773bcea'),
 (45469952,
  71,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::ReligionBuildingYieldModifier '
  'const&)',
  '4d8e76c443f40aa91603a1ae06b0d5d7163e58a8b7596e7aecb79595aa18ff24'),
 (45470032,
  69,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::ReligionBuildingYieldModifier&)',
  'e32afc2aec5868e5a99be1fd5d5c61555f27403603da87e446e099320a1a28ca'),
 (45470112,
  549,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Religion const&)',
  '9c9c41864bb815fe6a6795a6aa04dd2b1a66f9d65fb582d6ca0ae6f1d25d2666'),
 (45470672,
  1499,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Religion&)',
  '9f4c31dea0ee55306a5e2175de420ba1c5400e3c2423a6fdc19f253f088109f4'),
 (45488176,
  28,
  'Serialization::IStream& Serialization::ReadVersioned<GameCore::DistrictTypes, '
  'GameCore::DistrictTypes>(Serialization::IStream&, GameCore::DistrictTypes&, int, int&&, '
  'GameCore::DistrictTypes&&)',
  '4ebfd652e697103dc16305452941c3f783da98df9adf0db38d72e0850007c259'),
 (45488208,
  21,
  'Serialization::OStream& operator<< <FixedPointT<8u>, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488240,
  21,
  'Serialization::OStream& operator<< <GameCore::BeliefTypes, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<GameCore::BeliefTypes, '
  'GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488272,
  21,
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488304,
  21,
  'Serialization::OStream& operator<< <GameCore::PlotCoord, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<GameCore::PlotCoord, '
  'GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488336,
  21,
  'Serialization::OStream& operator<< <eastl::vector<GameCore::Player::BeliefYield, eastl::allocator>, '
  'GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::BeliefYield, eastl::allocator>, GameCore::Player::Religion> '
  'const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488368,
  21,
  'Serialization::OStream& operator<< <eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, '
  'eastl::allocator>, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488400,
  21,
  'Serialization::OStream& operator<< <bool, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<bool, GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488432,
  21,
  'Serialization::OStream& operator<< <GameCore::TechTypes, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<GameCore::TechTypes, '
  'GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488464,
  141,
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> const&)',
  '38a67fc5ee9a62ef4939bb6228b95fe113b3d9bbad8520c974029105a6e722e7'),
 (45488608,
  21,
  'Serialization::OStream& operator<< <GameCore::GreatPersonIndividualTypes, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<GameCore::GreatPersonIndividualTypes, '
  'GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488640,
  21,
  'Serialization::OStream& operator<< <Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<Types::Vector<int, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Religion> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45488672,
  21,
  'Serialization::IStream& operator>><FixedPointT<8u>, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Religion>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45488704,
  21,
  'Serialization::IStream& operator>><GameCore::BeliefTypes, '
  'GameCore::Player::Religion>(Serialization::IStream&, FAutoVariable<GameCore::BeliefTypes, '
  'GameCore::Player::Religion>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45488736,
  21,
  'Serialization::IStream& operator>><int, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<int, GameCore::Player::Religion>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45488768,
  21,
  'Serialization::IStream& operator>><GameCore::PlotCoord, '
  'GameCore::Player::Religion>(Serialization::IStream&, FAutoVariable<GameCore::PlotCoord, '
  'GameCore::Player::Religion>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45488800,
  21,
  'Serialization::IStream& operator>><eastl::vector<GameCore::Player::BeliefYield, eastl::allocator>, '
  'GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::BeliefYield, eastl::allocator>, '
  'GameCore::Player::Religion>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45488832,
  38,
  'Serialization::IStream& '
  'Serialization::ReadVersioned<FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, '
  'eastl::allocator>, GameCore::Player::Religion>, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&, int, int&&, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&)',
  '3009f12db8649c49020f3657452442db3f15a8f5d025b992c8b5e2f14b3ba616'),
 (45488880,
  34,
  'Serialization::IStream& Serialization::ReadVersioned<FAutoVariable<int, GameCore::Player::Religion>, '
  'int>(Serialization::IStream&, FAutoVariable<int, GameCore::Player::Religion>&, int, int&&, int&&)',
  '7ee907bfa0d46c8109b141cb2741815a6eb6f160fb5c88cbafade0742b7532d2'),
 (45488992,
  28,
  'Serialization::IStream& Serialization::ReadVersioned<GameCore::TechTypes, '
  'GameCore::TechTypes>(Serialization::IStream&, GameCore::TechTypes&, int, int&&, GameCore::TechTypes&&)',
  'f27d96297351852a5bfa33d5504e849fc1e3cb81faaba267512d1d2f3b7a90d4'),
 (45489088,
  120,
  'bool Serialization::ReadIndexHashArrayVersioned<int, int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> const&, '
  'int, int&&, int&&)',
  '51087e3d4bcc7ed69cc4235e19582667c89d2ff2261cf024d6e152a337864d05'),
 (45489216,
  28,
  'Serialization::IStream& Serialization::ReadVersioned<GameCore::GreatPersonIndividualTypes, '
  'GameCore::GreatPersonIndividualTypes>(Serialization::IStream&, GameCore::GreatPersonIndividualTypes&, '
  'int, int&&, GameCore::GreatPersonIndividualTypes&&)',
  '952071b6e4f2f4e4ebe7bd144741f46ffcbdc8c274fd7112e01579e2493a20ba'),
 (45489312,
  34,
  'Serialization::IStream& Serialization::ReadVersioned<Types::Vector<int, Types::LinearAlloc::Default<1u, '
  '0u> >, Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> > const&>(Serialization::IStream&, '
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >&, int, int&&, Types::Vector<int, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)',
  'eedd2b6127cac69516cd9131d81a92e198afc6f25b597558358b00e801a80fd7'),
 (45492400,
  21,
  'Serialization::IStream& operator>><eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, '
  'eastl::allocator>, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45493456,
  149,
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> const&)',
  'b2c51a5ab2fe26b6cc9ee999a1c3557d13ef86b39c1e419f62dc43a077527b2f'),
 (47508448,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::DistrictTypes const&)',
  'dc40e9bc43e0fc8aec2e195881b9a1c8eab8c366272b24f46990d7fb1231425c'),
 (47508496,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::DistrictTypes&)',
  '8f2608b388942db0f3d64ebc8e0e8f9cedd91acf351ae7e3d5cbdf710883f92d'),
 (47509280,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::TechTypes&)',
  '047050d264acc31d6a4477873412f54329d63a12b971850fa8ca7602c1cfa31c'),
 (47510496,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::GreatPersonIndividualTypes&)',
  '9fe820dbadcfc907d2a583f88ab9c21f808e8a5dfb5c66cd581531b2a8c81337'),
 (47512432,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::YieldTypes const&)',
  'b971caad466bb8cb3baf0b50ec89d1f83d536c9d25f15145cc664077fdd4a5c7'),
 (47512480,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::YieldTypes&)',
  '8f3a33cbd2aca7db1a0c6d4e9fae3b8ea968171068bccd693ea5ca8982362a12'),
 (47512528,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::BeliefYieldTypes const&)',
  'd32af052f0218f8e73b49a8ff4a081be91e03f201aea4102b9fb56b0045e45e6'),
 (47512576,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::BeliefYieldTypes&)',
  '5a3d1cfd0bfa3feebdbdb35d924a19ae813f26e643664a076bcddb2fefebbb3d'),
 (47518256,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonIndividual> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonIndividual> '
  'const&)',
  '73ea400f90b93c6d1ce899be91b4dfac2fd300085888d8d34ed601916c2874a2')]
CALLS = [(33341748, 32120032, 33348288, 'call', 'Serialization::OStream::WriteObject(int const&)'),
 (33341780, 32631936, 33348512, 'call', 'Serialization::OStream::WriteObject(unsigned int const&)'),
 (33341860, 31737840, 33348304, 'call', 'Serialization::IStream::ReadObject(int&)'),
 (33341892, 33039872, 33348528, 'call', 'Serialization::IStream::ReadObject(unsigned int&)'),
 (33348335,
  31471216,
  33348384,
  'call',
  'Serialization::IStream::ReadRequest::ReadRequest(unsigned long, unsigned long)'),
 (33348404,
  31796656,
  33348416,
  'jmp',
  'fastdelegate::FastDelegate1<Platform::MemoryBuffer&, void>::FastDelegate1()'),
 (33348559,
  31471216,
  33348384,
  'call',
  'Serialization::IStream::ReadRequest::ReadRequest(unsigned long, unsigned long)'),
 (33431444, 33083040, 33454336, 'call', 'Serialization::OStream::WriteObject(bool const&)'),
 (33431556, 31666816, 33456544, 'call', 'Serialization::IStream::ReadObject(bool&)'),
 (33447570,
  32766320,
  33431552,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><bool>(bool&)'),
 (33456575,
  31471216,
  33348384,
  'call',
  'Serialization::IStream::ReadRequest::ReadRequest(unsigned long, unsigned long)'),
 (33488404,
  32308032,
  33488416,
  'call',
  'void Serialization::SerializeToSequenceContainer<int, Types::Vector<int, Types::LinearAlloc::Default<1u, '
  '0u> > >(Serialization::IStream&, Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >&)'),
 (33488433, 31928560, 33445168, 'call', 'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::clear()'),
 (33488454,
  31815744,
  33341888,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)'),
 (33488471,
  31839232,
  33488544,
  'call',
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::reserve(unsigned long)'),
 (33488499,
  31608208,
  33488576,
  'call',
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::push_back()'),
 (33488507, 32488512, 33488624, 'call', 'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::back()'),
 (33488518,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (33536984,
  31459536,
  33537024,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District>::Find(unsigned '
  'int) const'),
 (33536998, 32759552, 33344752, 'jmp', 'GameCore::Definition::District::GetCollectionIndex() const'),
 (33563794,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (33563853,
  32748560,
  33576096,
  'call',
  'unsigned int '
  'Serialization::IndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  'const&)'),
 (33563870,
  31588976,
  33341776,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)'),
 (33576107,
  32275840,
  33432144,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District>::GetCount() '
  'const'),
 (33576121,
  32293968,
  33434368,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District>::At(unsigned int) '
  'const'),
 (33576135, 32029376, 33446784, 'jmp', 'GameCore::Definition::District::GetTypeHash() const'),
 (33576142,
  31459536,
  33537024,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District>::Find(unsigned '
  'int) const'),
 (33645704,
  32827680,
  33645744,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology>::Find(unsigned '
  'int) const'),
 (33645718, 31856512, 33344816, 'jmp', 'GameCore::Definition::Technology::GetCollectionIndex() const'),
 (34512808,
  32030032,
  34511968,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass>::Find(unsigned '
  'int) const'),
 (34512822,
  32187360,
  34214240,
  'jmp',
  'GameCore::Definition::UnitPromotionClass::GetCollectionIndex() const'),
 (42478532,
  32766320,
  33431552,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><bool>(bool&)'),
 (45469745,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (45469756,
  33184832,
  47512528,
  'call',
  'Serialization::operator<<(Serialization::OStream&, GameCore::BeliefYieldTypes const&)'),
 (45469768,
  32433776,
  47512432,
  'call',
  'Serialization::operator<<(Serialization::OStream&, GameCore::YieldTypes const&)'),
 (45469780,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (45469792,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (45469807,
  33152272,
  47508448,
  'call',
  'Serialization::operator<<(Serialization::OStream&, GameCore::DistrictTypes const&)'),
 (45469842,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (45469853,
  32304128,
  47512576,
  'call',
  'Serialization::operator>>(Serialization::IStream&, GameCore::BeliefYieldTypes&)'),
 (45469865,
  32704720,
  47512480,
  'call',
  'Serialization::operator>>(Serialization::IStream&, GameCore::YieldTypes&)'),
 (45469877,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (45469889,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (45469934,
  32700336,
  45488176,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<GameCore::DistrictTypes, '
  'GameCore::DistrictTypes>(Serialization::IStream&, GameCore::DistrictTypes&, int, int&&, '
  'GameCore::DistrictTypes&&)'),
 (45469969,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (45469980,
  32433776,
  47512432,
  'call',
  'Serialization::operator<<(Serialization::OStream&, GameCore::YieldTypes const&)'),
 (45469992,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (45470007,
  31462048,
  33431440,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)'),
 (45470047,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (45470058,
  32704720,
  47512480,
  'call',
  'Serialization::operator>>(Serialization::IStream&, GameCore::YieldTypes&)'),
 (45470070,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (45470085,
  32766320,
  33431552,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><bool>(bool&)'),
 (45470130,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (45470145,
  32546752,
  45488208,
  'call',
  'Serialization::OStream& operator<< <FixedPointT<8u>, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Religion> const&)'),
 (45470160,
  32546752,
  45488208,
  'call',
  'Serialization::OStream& operator<< <FixedPointT<8u>, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Religion> const&)'),
 (45470175,
  33058480,
  45488240,
  'call',
  'Serialization::OStream& operator<< <GameCore::BeliefTypes, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<GameCore::BeliefTypes, '
  'GameCore::Player::Religion> const&)'),
 (45470190,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470205,
  32318192,
  45488304,
  'call',
  'Serialization::OStream& operator<< <GameCore::PlotCoord, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<GameCore::PlotCoord, '
  'GameCore::Player::Religion> const&)'),
 (45470220,
  31754384,
  45488336,
  'call',
  'Serialization::OStream& operator<< <eastl::vector<GameCore::Player::BeliefYield, eastl::allocator>, '
  'GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::BeliefYield, eastl::allocator>, GameCore::Player::Religion> '
  'const&)'),
 (45470235,
  32278208,
  45488368,
  'call',
  'Serialization::OStream& operator<< <eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, '
  'eastl::allocator>, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion> const&)'),
 (45470250,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470265,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470280,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470295,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470310,
  32366576,
  45488400,
  'call',
  'Serialization::OStream& operator<< <bool, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<bool, GameCore::Player::Religion> const&)'),
 (45470325,
  32366576,
  45488400,
  'call',
  'Serialization::OStream& operator<< <bool, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<bool, GameCore::Player::Religion> const&)'),
 (45470340,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470355,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470370,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470385,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470400,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470415,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470430,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470445,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470460,
  32341312,
  45488432,
  'call',
  'Serialization::OStream& operator<< <GameCore::TechTypes, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<GameCore::TechTypes, '
  'GameCore::Player::Religion> const&)'),
 (45470475,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470490,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470505,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470520,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470535,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470547,
  33113120,
  33567184,
  'call',
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::front() const'),
 (45470555, 32635120, 35735744, 'call', 'GameCore::Definitions::GetUnitPromotionClassDefinitions()'),
 (45470569,
  32367200,
  45488464,
  'call',
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  'const&)'),
 (45470584,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470599,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470614,
  32490320,
  45488272,
  'call',
  'Serialization::OStream& operator<< <int, GameCore::Player::Religion>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Religion> const&)'),
 (45470629,
  33087920,
  45488608,
  'call',
  'Serialization::OStream& operator<< <GameCore::GreatPersonIndividualTypes, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<GameCore::GreatPersonIndividualTypes, '
  'GameCore::Player::Religion> const&)'),
 (45470647,
  32827552,
  45488640,
  'call',
  'Serialization::OStream& operator<< <Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Religion>(Serialization::OStream&, FAutoVariable<Types::Vector<int, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Religion> const&)'),
 (45470696,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (45470711,
  31453840,
  45488672,
  'call',
  'Serialization::IStream& operator>><FixedPointT<8u>, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Religion>&)'),
 (45470726,
  31453840,
  45488672,
  'call',
  'Serialization::IStream& operator>><FixedPointT<8u>, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Religion>&)'),
 (45470741,
  31531136,
  45488704,
  'call',
  'Serialization::IStream& operator>><GameCore::BeliefTypes, '
  'GameCore::Player::Religion>(Serialization::IStream&, FAutoVariable<GameCore::BeliefTypes, '
  'GameCore::Player::Religion>&)'),
 (45470756,
  32553392,
  45488736,
  'call',
  'Serialization::IStream& operator>><int, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<int, GameCore::Player::Religion>&)'),
 (45470771,
  32117984,
  45488768,
  'call',
  'Serialization::IStream& operator>><GameCore::PlotCoord, '
  'GameCore::Player::Religion>(Serialization::IStream&, FAutoVariable<GameCore::PlotCoord, '
  'GameCore::Player::Religion>&)'),
 (45470786,
  31503152,
  45488800,
  'call',
  'Serialization::IStream& operator>><eastl::vector<GameCore::Player::BeliefYield, eastl::allocator>, '
  'GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::BeliefYield, eastl::allocator>, '
  'GameCore::Player::Religion>&)'),
 (45470821,
  32352000,
  45488832,
  'call',
  'Serialization::IStream& '
  'Serialization::ReadVersioned<FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, '
  'eastl::allocator>, GameCore::Player::Religion>, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&, int, int&&, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&)'),
 (45470866,
  31602336,
  45488880,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<FAutoVariable<int, GameCore::Player::Religion>, '
  'int>(Serialization::IStream&, FAutoVariable<int, GameCore::Player::Religion>&, int, int&&, int&&)'),
 (45470881,
  32553392,
  45488736,
  'call',
  'Serialization::IStream& operator>><int, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<int, GameCore::Player::Religion>&)'),
 (45470926,
  31602336,
  45488880,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<FAutoVariable<int, GameCore::Player::Religion>, '
  'int>(Serialization::IStream&, FAutoVariable<int, GameCore::Player::Religion>&, int, int&&, int&&)'),
 (45470941,
  32553392,
  45488736,
  'call',
  'Serialization::IStream& operator>><int, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<int, GameCore::Player::Religion>&)'),
 (45470953, 31752640, 45488928, 'call', 'FAutoVariable<bool, GameCore::Player::Religion>::edit()'),
 (45470994,
  31649472,
  42478512,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<bool, int>(Serialization::IStream&, bool&, int, '
  'int&&, int&&)'),
 (45471006, 31752640, 45488928, 'call', 'FAutoVariable<bool, GameCore::Player::Religion>::edit()'),
 (45471044,
  31828752,
  33447552,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<bool, bool>(Serialization::IStream&, bool&, int, '
  'int&&, bool&&)'),
 (45471056, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471097,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471109, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471150,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471162, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471203,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471215, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471256,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471268, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471309,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471321, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471362,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471374, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471415,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471427, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471468,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471480,
  31637920,
  45489024,
  'call',
  'FAutoVariable<GameCore::TechTypes, GameCore::Player::Religion>::edit()'),
 (45471521,
  32985920,
  45488992,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<GameCore::TechTypes, '
  'GameCore::TechTypes>(Serialization::IStream&, GameCore::TechTypes&, int, int&&, GameCore::TechTypes&&)'),
 (45471533, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471574,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471586, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471627,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471639, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471680,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471692, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471733,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471745, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471786,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471791, 32635120, 35735744, 'call', 'GameCore::Definitions::GetUnitPromotionClassDefinitions()'),
 (45471809,
  32752528,
  37992304,
  'call',
  'FAutoVariable<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Religion>::edit()'),
 (45471820,
  32730448,
  34221920,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass>::GetCount() '
  'const'),
 (45471830,
  32315936,
  33921488,
  'call',
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::resize(unsigned long)'),
 (45471838,
  32752528,
  37992304,
  'call',
  'FAutoVariable<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Religion>::edit()'),
 (45471846, 32377088, 33566496, 'call', 'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::front()'),
 (45471890,
  31685856,
  45489088,
  'call',
  'bool Serialization::ReadIndexHashArrayVersioned<int, int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> const&, '
  'int, int&&, int&&)'),
 (45471902, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471943,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45471955, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45471996,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45472008, 31544752, 37992240, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::edit()'),
 (45472049,
  33253312,
  33563776,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, '
  'int&&)'),
 (45472061,
  33139312,
  45489248,
  'call',
  'FAutoVariable<GameCore::GreatPersonIndividualTypes, GameCore::Player::Religion>::edit()'),
 (45472102,
  31547376,
  45489216,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<GameCore::GreatPersonIndividualTypes, '
  'GameCore::GreatPersonIndividualTypes>(Serialization::IStream&, GameCore::GreatPersonIndividualTypes&, '
  'int, int&&, GameCore::GreatPersonIndividualTypes&&)'),
 (45472114,
  32752528,
  37992304,
  'call',
  'FAutoVariable<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Religion>::edit()'),
 (45472149,
  31847440,
  45489312,
  'call',
  'Serialization::IStream& Serialization::ReadVersioned<Types::Vector<int, Types::LinearAlloc::Default<1u, '
  '0u> >, Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> > const&>(Serialization::IStream&, '
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >&, int, int&&, Types::Vector<int, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)'),
 (45488194,
  32469952,
  47508496,
  'call',
  'Serialization::operator>>(Serialization::IStream&, GameCore::DistrictTypes&)'),
 (45488489,
  32730448,
  34221920,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass>::GetCount() '
  'const'),
 (45488506,
  31588976,
  33341776,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)'),
 (45488533,
  33138128,
  34511920,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass>::At(unsigned '
  'int) const'),
 (45488541, 33107296, 34511952, 'call', 'GameCore::Definition::UnitPromotionClass::GetTypeHash() const'),
 (45488556,
  31588976,
  33341776,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)'),
 (45488567,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (45488850,
  31680816,
  45492432,
  'call',
  'void CopyAssignment<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion> '
  '>(FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&, eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, '
  'eastl::allocator> const&)'),
 (45488860,
  31686752,
  45492400,
  'call',
  'Serialization::IStream& operator>><eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, '
  'eastl::allocator>, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<eastl::vector<GameCore::Player::ReligionBuildingYieldModifier, eastl::allocator>, '
  'GameCore::Player::Religion>&)'),
 (45488894, 31811536, 37990064, 'call', 'FAutoVariable<int, GameCore::Player::Religion>::operator=(int&&)'),
 (45488904,
  32553392,
  45488736,
  'call',
  'Serialization::IStream& operator>><int, GameCore::Player::Religion>(Serialization::IStream&, '
  'FAutoVariable<int, GameCore::Player::Religion>&)'),
 (45489010,
  32676368,
  47509280,
  'call',
  'Serialization::operator>>(Serialization::IStream&, GameCore::TechTypes&)'),
 (45489113,
  31662624,
  34484704,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass>::begin() '
  'const'),
 (45489124,
  32488960,
  34484720,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass>::end() '
  'const'),
 (45489158,
  32187360,
  34214240,
  'call',
  'GameCore::Definition::UnitPromotionClass::GetCollectionIndex() const'),
 (45489203,
  31825040,
  45493456,
  'jmp',
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  'const&)'),
 (45489234,
  31879680,
  47510496,
  'call',
  'Serialization::operator>>(Serialization::IStream&, GameCore::GreatPersonIndividualTypes&)'),
 (45489326,
  32512080,
  33582816,
  'call',
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::operator=(Types::Vector<int, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)'),
 (45489336,
  32271472,
  33488400,
  'call',
  'Serialization::IStream& Serialization::operator>><int, Types::LinearAlloc::Default<1u, 0u> '
  '>(Serialization::IStream&, Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >&)'),
 (45493484,
  31815744,
  33341888,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)'),
 (45493526,
  31815744,
  33341888,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)'),
 (45493538,
  33133616,
  34512800,
  'call',
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  'const&)'),
 (45493574,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (47508456, 32651264, 35732080, 'call', 'GameCore::Definitions::GetDistrictDefinitions()'),
 (47508469,
  32508400,
  33563840,
  'call',
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> const&)'),
 (47508515,
  31815744,
  33341888,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)'),
 (47508524, 32651264, 35732080, 'call', 'GameCore::Definitions::GetDistrictDefinitions()'),
 (47508534,
  32370640,
  33536976,
  'call',
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  'const&)'),
 (47509299,
  31815744,
  33341888,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)'),
 (47509308, 33010160, 35735280, 'call', 'GameCore::Definitions::GetTechnologyDefinitions()'),
 (47509318,
  32664000,
  33645696,
  'call',
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  'const&)'),
 (47510515,
  31815744,
  33341888,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)'),
 (47510524, 32700976, 35732992, 'call', 'GameCore::Definitions::GetGreatPersonIndividualDefinitions()'),
 (47510534,
  32565824,
  47518256,
  'call',
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonIndividual> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonIndividual> '
  'const&)'),
 (47512451,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (47512495,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (47512547,
  32898000,
  33341744,
  'call',
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)'),
 (47512591,
  31938512,
  33341856,
  'call',
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)'),
 (47518264,
  32985216,
  42376768,
  'call',
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonIndividual>::Find(unsigned '
  'int) const'),
 (47518278,
  31615616,
  33919440,
  'jmp',
  'GameCore::Definition::GreatPersonIndividual::GetCollectionIndex() const')]

def relocation_symbols(elf):
    raw=elf.raw
    start=struct.unpack_from('<Q',raw,40)[0]
    width,count=struct.unpack_from('<HH',raw,58)
    sections=[struct.unpack_from('<IIQQQQIIQQ',raw,start+i*width) for i in range(count)]
    result={}
    for section in sections:
        if section[1]!=4: continue
        symbols=sections[section[6]]
        for pos in range(section[4],section[4]+section[5],section[9]):
            target,info,_=struct.unpack_from('<QQq',raw,pos)
            if info & 0xffffffff != 7: continue
            symbol=symbols[4]+(info>>32)*symbols[9]
            result[target]=struct.unpack_from('<Q',raw,symbol+8)[0]
    return result

HARNESS=r'''pub use c6_core::unit;
#[path="MODULE"] pub mod player_religion;
fn main(){let args:Vec<_>=std::env::args().collect();let input=std::fs::read(&args[1]).unwrap();let limits=unit::Limits{max_bytes:64*1024*1024,max_items:4_000_000};let(v,n,items)=player_religion::Religion::decode_prefix_with_items(&input,limits).unwrap();assert_eq!(n,input.len());let exact=unit::Limits{max_bytes:n,max_items:items};let(out,witems)=v.encode_with_items(exact).unwrap();assert_eq!(items,witems);assert_eq!(v,player_religion::Religion::decode(&out,exact).unwrap());for short in [unit::Limits{max_bytes:n-1,..exact},unit::Limits{max_items:items-1,..exact}]{assert!(player_religion::Religion::decode(&input,short).is_err());assert!(v.encode(short).is_err());}std::fs::write(&args[2],out).unwrap();println!("{} {} {} {} {} {} {}",n,items,v.faith_balance_q8,v.belief_yields.len(),v.building_yield_modifiers.len(),v.enable_wall_attack_whole_game_same_religion_promotion_class.len(),v.units_religion_strength_by_religion.len());}
'''

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    game=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA)
    Elf(a.assets/'Civ6',HOST_SHA)
    symbols={}
    for row in subprocess.check_output(['nm','-S','-C',str(a.assets/'libGameCore_XP2.so')],text=True).splitlines():
        parts=row.split(' ',3)
        if len(parts)==4 and parts[2] in ('T','W'):
            symbols[parts[3]]=(int(parts[0],16),int(parts[1],16))
    for address,size,name,digest in METHODS:
        if symbols.get(name)!=(address,size):raise ValueError('symbol identity mismatch: '+name)
        if sha(game.read(address,size))!=digest:raise ValueError('full method mismatch: '+name)
    if struct.unpack('<I',game.read(0x2f66664,4))[0]!=20:raise ValueError('local current version changed')
    relocations=relocation_symbols(game)
    calls=CALLS+[(0x2b4f09a,0x1f8ff50,0x2b5d1a0,'call','Serialization::operator<<(Serialization::OStream&, GameCore::Player::Religion const&)'),(0x2b4f0ad,0x1eb9310,0x2b6b0f0,'call','Serialization::operator<<(Serialization::OStream&, GameCore::Player::Resources const&)')]
    game.call(0x2b4f0ad,0x1eb9310)
    for site,target,body,kind,name in calls:
        code=game.read(site,5)
        if code[0]!=(0xe8 if kind=='call' else 0xe9) or site+5+struct.unpack_from('<i',code,1)[0]!=target:raise ValueError('direct binding changed')
        if symbols[name][0]!=body:raise ValueError('bound symbol changed')
        if target!=body:
            stub=game.read(target,6)
            if stub[:2]!=b'\xff\x25':raise ValueError('unexpected PLT')
            got=target+6+struct.unpack_from('<i',stub,2)[0]
            if relocations.get(got)!=body:raise ValueError('PLT actual body changed')
    command=['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so')]
    for name in ['Religion','BeliefYield','ReligionBuildingYieldModifier']:command+=['-ex',"ptype /o 'GameCore::Player::"+name+"'"]
    dwarf=subprocess.check_output(command,text=True)
    if sha(dwarf.encode())!='1ae0c1843fc671f1353a31224a43d2e190fc96b9631f69d1d5e710a61afc7ac7':raise ValueError('full DWARF types changed')
    paths=[ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'crates/c6-core/Cargo.toml',*sorted((ROOT/'crates/c6-core/src').rglob('*.rs'))]
    sources={str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in paths}
    subprocess.run(['cargo','build','--offline','-p','c6-core'],cwd=ROOT,check=True)
    if sources!={str(x.relative_to(ROOT)):sha(x.read_bytes())for x in paths}:raise ValueError('source changed during build')
    reports=[]
    with tempfile.TemporaryDirectory(prefix='c6-religion-')as tmp:
        tmp=Path(tmp);harness=HARNESS.replace('MODULE',str(ROOT/'crates/c6-core/src/player_religion.rs'));(tmp/'candidate.rs').write_text(harness)
        cmd=['rustc','--edition=2021','-Dwarnings',str(tmp/'candidate.rs'),'--extern','c6_core='+str(ROOT/'target/debug/libc6_core.rlib'),'-L','dependency='+str(ROOT/'target/debug/deps')]
        subprocess.run(cmd+['-o',str(tmp/'candidate')],check=True)
        subprocess.run(cmd+['--test','-o',str(tmp/'tests')],check=True)
        subprocess.run([str(tmp/'tests')],check=True)
        candidate_sha=sha((tmp/'candidate').read_bytes())
        for identity,trace_name,trace_sha in FIXTURES:
            data=gzip.decompress((a.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
            if sha(data)!=identity:raise ValueError('Core identity mismatch')
            trace=gzip.decompress((a.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
            if sha(trace)!=trace_sha:raise ValueError('owner trace identity mismatch')
            rows=[json.loads(x)for x in trace.splitlines()]
            cuts=sorted([x for x in rows if x.get('event')=='owner'and x.get('save')==1 and x.get('owner')=='GameCore::Player::Religion'],key=lambda x:x['start'])
            if not cuts:raise ValueError('no Religion validation cuts')
            observations=[];digests=[];total=0
            for cut in cuts:
                payload=data[cut['start']:cut['end']];(tmp/'in').write_bytes(payload)
                words=[int(x)for x in subprocess.check_output([str(tmp/'candidate'),str(tmp/'in'),str(tmp/'out')],text=True).split()]
                if (tmp/'out').read_bytes()!=payload:raise ValueError('saved record differs')
                if words[0]!=len(payload)or words[2]!=struct.unpack_from('<i',payload,4)[0]:raise ValueError('typed prefix mismatch')
                total+=len(payload);observations.append(words);digests.append(sha(payload))
            reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(cuts),record_bytes=total,record_bytes_meaning='sum_of_all_records_in_this_stream',first_record_bytes=cuts[0]['end']-cuts[0]['start'],first_observed_range=[cuts[0]['start'],cuts[0]['end']],ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode()),typed_observations=observations))
    report=dict(schema='c6-linux-player-religion-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_version=20,version_address='0x2f66664',methods=[dict(address=hex(x),size=n,name=name,sha256=h)for x,n,name,h in METHODS],direct_bindings=[dict(address=hex(x),target=hex(y),actual_body=hex(z),kind=k,name=n)for x,y,z,k,n in calls],dwarf_sha256=sha(dwarf.encode()),candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=5,exact_native_record_byte_item_caps_checked=True,one_short_native_record_caps_rejected=True,allocation_policy='checked per-vector storage; aggregate consumed wire bytes and items',original_execution=False,private_writes=False,legacy_versions=False,attribution_reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175',reference_role='discovery_only',complete_current_religion_wire=True,definition_remapping=False,native_read_fixups=False,full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,next_player_component='Resources',unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
    a.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods=len(METHODS),fixtures=[dict(records=x['records'],record_bytes=x['record_bytes'])for x in reports])))
if __name__=='__main__':main()
