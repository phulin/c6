#!/usr/bin/env python3
"""Pinned Linux Blackboard contracts and offline saved-record roundtrips; no original execution."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
ROOT = Path(__file__).resolve().parents[2]
def sha(b): return hashlib.sha256(b).hexdigest()
METHODS = [[44863648,
  79,
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, '
  'GameCore::Player::Diplomacy>::dummyLoadDelta(Serialization::IStream&)',
  'ae7e6d54aa8a3056fbdd87ac5ab1bba47abaddef477e8c2eaf805b86ad8636ea'],
 [44863616,
  15,
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'b58646a1af9d88d3bf29b331da31cda6e5f0f1c6d1d635fd332fb947ea2a6663'],
 [44863632,
  9,
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, '
  'GameCore::Player::Diplomacy>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'],
 [44943856,
  19,
  'void Serialization::EnumToStream<GameCore::DiplomaticVisibilityTypes>(Serialization::OStream&, '
  'GameCore::DiplomaticVisibilityTypes const&)',
  '8b5ec31424cd8e791a3b4643899b9ee5418475c3ff550fe8cad4a68da4ca51b5'],
 [44943936,
  30,
  'void Serialization::EnumFromStream<GameCore::DiplomaticVisibilityTypes>(Serialization::IStream&, '
  'GameCore::DiplomaticVisibilityTypes&)',
  'ec61280a55d011602bb51b614f963852e48c9d50750aab3e440fc0a376fabfd3'],
 [44952032,
  125,
  'void Serialization::SerializeToSequenceContainer<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator>, '
  'Types::Vector<eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned '
  'int>, eastl::allocator>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::IStream&, '
  'Types::Vector<eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned '
  'int>, eastl::allocator>, Types::LinearAlloc::Default<1u, 0u> >&)',
  'ed8314994bcc09f0264521225791a04a45d89356bf4171288af1da13099d4b9e'],
 [44948528,
  46,
  'Serialization::SerializeFromSequenceContainer<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const, '
  'Types::Vector<eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned '
  'int>, eastl::allocator>, Types::LinearAlloc::Default<1u, 0u> > '
  'const>::SerializeFromSequenceContainer(Serialization::OStream&, Types::Vector<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator>, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)',
  '496718026bcb62d895fa744086673ab4e5dcf237b4ea582ec3cc7b168e990de6'],
 [44948576,
  8,
  'Serialization::SerializeFromSequenceContainer<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const, '
  'Types::Vector<eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned '
  'int>, eastl::allocator>, Types::LinearAlloc::Default<1u, 0u> > const>::operator()(eastl::map<unsigned '
  'int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const&)',
  'cf1a47e3b4d694e577f2a621c55d8cbcb07ea172989f71072499d592b1bfb848'],
 [44952448,
  205,
  'void Serialization::SerializeToAssociativeContainer<eastl::pair<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> '
  '>(Serialization::IStream&, eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, '
  'eastl::less<unsigned int>, eastl::allocator>&)',
  '1b4729167f961f24a4b3bb70fd7f3a122d5a270a3d6c71fca9ac81bedc4b911f'],
 [44949056,
  151,
  'void Serialization::SerializeToAssociativeContainer<eastl::pair<long, Types::StaticArray<FixedPointT<8u>, '
  '64ul> >, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator> '
  '>(Serialization::IStream&, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, '
  'eastl::allocator>&)',
  '6c5269d014aa4237e3914b512277d473e5b8b182542a2e7e46d23d98c341e03f'],
 [44951120,
  151,
  'void Serialization::SerializeToAssociativeContainer<eastl::pair<long, Types::StaticArray<bool, 64ul> >, '
  'eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, eastl::allocator> '
  '>(Serialization::IStream&, eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, '
  'eastl::allocator>&)',
  '9a753e8e514c2e7cba8ab5b61b6031017f17f1c7dc7cf60a2384bef84ef2df84'],
 [44950176,
  151,
  'void Serialization::SerializeToAssociativeContainer<eastl::pair<long, Types::StaticArray<unsigned int, '
  '64ul> >, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, eastl::allocator> '
  '>(Serialization::IStream&, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, '
  'eastl::allocator>&)',
  '3a579fafc291e88e09a0b77ff175107ba9d93072f0190852d6920a12be1c226d'],
 [44948880,
  46,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> '
  'const>::SerializeFromAssociativeContainer(Serialization::OStream&, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const&)',
  '10f7ac8fd2aaa2df43d39f2c0f60148f6da296d6e080aafc93516716c7abbace'],
 [44948928,
  8,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> '
  'const>::operator()(eastl::pair<unsigned int, GameCore::Player::DiplomaticModifierTracker> const&)',
  'd6ca1d6367d107a34af4e8b0074681485582e3a0afde18182b2a62e021d87558'],
 [44947472,
  46,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<FixedPointT<8u>, '
  '64ul> >, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator> '
  'const>::SerializeFromAssociativeContainer(Serialization::OStream&, eastl::map<long, '
  'Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator> const&)',
  'fb59c3f5f01c6e0b29c7cabd8bab578633a9d7d8b42de929ec0ef409c29a6cfa'],
 [44947520,
  8,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<FixedPointT<8u>, '
  '64ul> >, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator> '
  'const>::operator()(eastl::pair<long, Types::StaticArray<FixedPointT<8u>, 64ul> > const&)',
  '90f2dcb03d085b5a820a8dd3442b9ad183ae37da3aa8861d1f2703f60566a728'],
 [44948272,
  46,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<bool, 64ul> >, '
  'eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, eastl::allocator> '
  'const>::SerializeFromAssociativeContainer(Serialization::OStream&, eastl::map<long, '
  'Types::StaticArray<bool, 64ul>, eastl::less<long>, eastl::allocator> const&)',
  '544e3ee5647a9466fa6ad881da4dcb1655495e7bd0335a1173a7450ea3594512'],
 [44948320,
  8,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<bool, 64ul> >, '
  'eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, eastl::allocator> '
  'const>::operator()(eastl::pair<long, Types::StaticArray<bool, 64ul> > const&)',
  '3cfc93e271b9db9a9cd20f8d8047c9e8e50f4d465b6556c955471f16b91c5b16'],
 [44947888,
  46,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<unsigned int, 64ul> '
  '>, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, eastl::allocator> '
  'const>::SerializeFromAssociativeContainer(Serialization::OStream&, eastl::map<long, '
  'Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, eastl::allocator> const&)',
  '5565c71af4b2c99e030aebffd88580b67c87ebf8ee68283e3f37a34c05fd6a9c'],
 [44947936,
  8,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<unsigned int, 64ul> '
  '>, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, eastl::allocator> '
  'const>::operator()(eastl::pair<long, Types::StaticArray<unsigned int, 64ul> > const&)',
  'a462be7d363a393b4bef3dd8f20862ac4eaf38263114c350547c552dacd6c5bb'],
 [44949472,
  75,
  'Serialization::IStream::ReadObject(long&)',
  '5fee11815e1d068e54481092f72bd687140adcc68dd59830fd84910d17abfcab'],
 [44947696,
  14,
  'Serialization::OStream::WriteObject(long const&)',
  '0a72c29ed2372d7654b65a5833f26a5d2942142eb2b1a1af85c06046e1b3ef8e'],
 [46772192,
  42,
  'Serialization::operator<<(Serialization::OStream&, FixedPointT<8u> const&)',
  'bbd0553ff77e4be5a896850082327f68662e870164f4771d58a0e5c73c7d5704'],
 [44934032,
  267,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DiplomacyBlackboard const&)',
  '85fb0ee056686346afad55e874c5c8681f4497e325e2fdc8b5db119cd5865869'],
 [44935104,
  167,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DiplomaticModifierTracker const&)',
  'f5421c788022689e8558c9caad1ca04cbf4c4bfac432df94276e06e739f38465'],
 [44947632,
  57,
  'Serialization::OStream& Serialization::operator<< <FixedPointT<8u>, 64ul>(Serialization::OStream&, '
  'Types::StaticArray<FixedPointT<8u>, 64ul> const&)',
  '1e8cf5135526b76bb870c41d7b0254cca4cb66b382893372fec26c117b7e4be5'],
 [44943584,
  91,
  'Serialization::OStream& Serialization::operator<< <eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator>, '
  'Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, Types::Vector<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator>, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)',
  '6cb3e11dce0f67cc5a26bbe5c33e7fbdcfe76c84a07dc4a19294e786a114e720'],
 [33510448,
  81,
  'Serialization::OStream& Serialization::operator<< <Platform::StaticHeapAllocator<5, 0>, '
  '(String::Encoding)0>(Serialization::OStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, '
  '(String::Encoding)0> const&)',
  '045813e437c3874ae9899508e422a1126cbd16d136f2af8ce3eff6603cb99cca'],
 [44830304,
  57,
  'Serialization::OStream& Serialization::operator<< <bool, 64ul>(Serialization::OStream&, '
  'Types::StaticArray<bool, 64ul> const&)',
  '1c718422d7b2169aaebd4794682fe49b52d1d7bbb6fefa28def835130cafe9e1'],
 [42607696,
  57,
  'Serialization::OStream& Serialization::operator<< <int, 64ul>(Serialization::OStream&, '
  'Types::StaticArray<int, 64ul> const&)',
  '1160044aac8668625b62e0243935cc139c38a02e730c536d3f29b877d8e5d554'],
 [44948032,
  57,
  'Serialization::OStream& Serialization::operator<< <unsigned int, 64ul>(Serialization::OStream&, '
  'Types::StaticArray<unsigned int, 64ul> const&)',
  '985bfe70a0eecafb00022b9ef5e539b995a61d80dfd3d176facf0a8d2ab84224'],
 [44948592,
  91,
  'Serialization::OStream& Serialization::operator<< <unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>(Serialization::OStream&, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const&)',
  '5f32ffd16b610567211ff1e63dd24f0ae551f95af3c2af1b53a00128731ca6af'],
 [44948992,
  37,
  'Serialization::OStream& Serialization::operator<< <unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>(Serialization::OStream&, eastl::pair<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker> const&)',
  '75a5220bfdbf6892ff264e784b5a0d844091648538d591ef73ce8811e1a3e39e'],
 [44943296,
  91,
  'Serialization::OStream& Serialization::operator<< <long, Types::StaticArray<FixedPointT<8u>, 64ul> '
  '>(Serialization::OStream&, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, '
  'eastl::allocator> const&)',
  'eb308c6c5ed1a112f8c9ca6636051f416bddda66b96ca0d47054cf65d5964cf4'],
 [44947568,
  41,
  'Serialization::OStream& Serialization::operator<< <long, Types::StaticArray<FixedPointT<8u>, 64ul> '
  '>(Serialization::OStream&, eastl::pair<long, Types::StaticArray<FixedPointT<8u>, 64ul> > const&)',
  'b096ccd07cb7df66f19b8105d2e6722f102660785511baf6ae0023253d6cb544'],
 [44943488,
  91,
  'Serialization::OStream& Serialization::operator<< <long, Types::StaticArray<bool, 64ul> '
  '>(Serialization::OStream&, eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, '
  'eastl::allocator> const&)',
  '77cf170582c622064a29537a195823e09715c06c8b86c2fbed65d4fefaeeea09'],
 [44948384,
  41,
  'Serialization::OStream& Serialization::operator<< <long, Types::StaticArray<bool, 64ul> '
  '>(Serialization::OStream&, eastl::pair<long, Types::StaticArray<bool, 64ul> > const&)',
  'd78329a337b7926c81954b97b33eb9c7dafe941cb82fa5cb1d82e6d3cdcddfb5'],
 [44943392,
  91,
  'Serialization::OStream& Serialization::operator<< <long, Types::StaticArray<unsigned int, 64ul> '
  '>(Serialization::OStream&, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, '
  'eastl::allocator> const&)',
  '66dd754becf1995f1e081a79a14cae1e50ee96a3e550fea17d9f3e47f16bb56e'],
 [44947984,
  41,
  'Serialization::OStream& Serialization::operator<< <long, Types::StaticArray<unsigned int, 64ul> '
  '>(Serialization::OStream&, eastl::pair<long, Types::StaticArray<unsigned int, 64ul> > const&)',
  'de3020d36481a0a50b5578143a641628fd1a6b36b7c9ec53e93f57631504dfc3'],
 [46772240,
  43,
  'Serialization::operator>>(Serialization::IStream&, FixedPointT<8u>&)',
  '5d47fcdfb3868524b3d8da8a2fab95de165678efa05f23c615963da12ddf010d'],
 [44934304,
  791,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DiplomacyBlackboard&)',
  '6df4db2ba941c143f90f015a9dd2eb98cf963a2975eef6841980ffee0df51075'],
 [44935280,
  186,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DiplomaticModifierTracker&)',
  'c93982679cf586bd7d4b790b43f2135621782007cd9135c1c72edc9481741b1a'],
 [44949408,
  57,
  'Serialization::IStream& Serialization::operator>><FixedPointT<8u>, 64ul>(Serialization::IStream&, '
  'Types::StaticArray<FixedPointT<8u>, 64ul>&)',
  'b9d8a6dfabcd1c4570b7dd4a094e06bcfe63e339f9178bf88df136c1d3eff8ba'],
 [44943728,
  14,
  'Serialization::IStream& Serialization::operator>><eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator>, '
  'Types::LinearAlloc::Default<1u, 0u> >(Serialization::IStream&, Types::Vector<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator>, '
  'Types::LinearAlloc::Default<1u, 0u> >&)',
  '43a561bee4f1205e5fd916edfbe04fe2c28b59e868f3529ee7be7d7e1bf3f7f4'],
 [33508640,
  298,
  'Serialization::IStream& Serialization::operator>><Platform::StaticHeapAllocator<5, 0>, '
  '(String::Encoding)0>(Serialization::IStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, '
  '(String::Encoding)0>&)',
  '7bef00c45e9a3eccd9989b48d1c8dbfe0b5ee20adb2ce7c5922995b76d6172af'],
 [44778560,
  57,
  'Serialization::IStream& Serialization::operator>><bool, 64ul>(Serialization::IStream&, '
  'Types::StaticArray<bool, 64ul>&)',
  '1ba69f9d8b0cdf64bcf3f2be80372b869a740e96a352b5b229c79f577c18bee3'],
 [42586816,
  57,
  'Serialization::IStream& Serialization::operator>><int, 64ul>(Serialization::IStream&, '
  'Types::StaticArray<int, 64ul>&)',
  '87942a1adf64ea74dd3b0cf2e1457098aff6df9ff9ead874df6b73ee1c6b7e1c'],
 [44950448,
  57,
  'Serialization::IStream& Serialization::operator>><unsigned int, 64ul>(Serialization::IStream&, '
  'Types::StaticArray<unsigned int, 64ul>&)',
  '7960c6161ac617d002b840721267105cfd29809540535e5b0b08ada8c009c8b8'],
 [44952304,
  14,
  'Serialization::IStream& Serialization::operator>><unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>(Serialization::IStream&, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator>&)',
  'fd9e05d1c378760863911e987df5a29d3960bd09220feb3fa4bf48819a83c717'],
 [44952704,
  37,
  'Serialization::IStream& Serialization::operator>><unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>(Serialization::IStream&, eastl::pair<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>&)',
  '48f8517952bf56a643151297222b9d6e635300aedbabd602329a548ec73dfbdf'],
 [44943680,
  14,
  'Serialization::IStream& Serialization::operator>><long, Types::StaticArray<FixedPointT<8u>, 64ul> '
  '>(Serialization::IStream&, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, '
  'eastl::allocator>&)',
  '16f06c4c3884ed9007a8620a3f4255676f006e1c8b5f24317bb6657057eaa2af'],
 [44949248,
  41,
  'Serialization::IStream& Serialization::operator>><long, Types::StaticArray<FixedPointT<8u>, 64ul> '
  '>(Serialization::IStream&, eastl::pair<long, Types::StaticArray<FixedPointT<8u>, 64ul> >&)',
  '8f5099c6d9362f2771ef6469957e224f24ed181348e150c7708eda80b343bc8f'],
 [44943712,
  14,
  'Serialization::IStream& Serialization::operator>><long, Types::StaticArray<bool, 64ul> '
  '>(Serialization::IStream&, eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, '
  'eastl::allocator>&)',
  'f6cdea75212780a2a3cd74dbc852b1c3ff1cdbe0f37d898e4336e5df63742208'],
 [44951312,
  41,
  'Serialization::IStream& Serialization::operator>><long, Types::StaticArray<bool, 64ul> '
  '>(Serialization::IStream&, eastl::pair<long, Types::StaticArray<bool, 64ul> >&)',
  '4f962527596dee2ba1e469ef4e6cc26a1d4028cedb60723b4455df6c9f9533a4'],
 [44943696,
  14,
  'Serialization::IStream& Serialization::operator>><long, Types::StaticArray<unsigned int, 64ul> '
  '>(Serialization::IStream&, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, '
  'eastl::allocator>&)',
  '6d81e14c32b620067e94034b87085e0a6011895bc522d9abab02c1ca2aeb1943'],
 [44950352,
  41,
  'Serialization::IStream& Serialization::operator>><long, Types::StaticArray<unsigned int, 64ul> '
  '>(Serialization::IStream&, eastl::pair<long, Types::StaticArray<unsigned int, 64ul> >&)',
  '11e09cec37f8262c7da1c3093b58ddfc1424e5871ef17421441c3ffb0619ef7c'],
 [44948688,
  184,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const> '
  'eastl::for_each<eastl::rbtree_iterator<eastl::pair<unsigned int const, '
  'GameCore::Player::DiplomaticModifierTracker>, eastl::pair<unsigned int const, '
  'GameCore::Player::DiplomaticModifierTracker> const*, eastl::pair<unsigned int const, '
  'GameCore::Player::DiplomaticModifierTracker> const&>, '
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const> '
  '>(eastl::rbtree_iterator<eastl::pair<unsigned int const, GameCore::Player::DiplomaticModifierTracker>, '
  'eastl::pair<unsigned int const, GameCore::Player::DiplomaticModifierTracker> const*, eastl::pair<unsigned '
  'int const, GameCore::Player::DiplomaticModifierTracker> const&>, '
  'eastl::rbtree_iterator<eastl::pair<unsigned int const, GameCore::Player::DiplomaticModifierTracker>, '
  'eastl::pair<unsigned int const, GameCore::Player::DiplomaticModifierTracker> const*, eastl::pair<unsigned '
  'int const, GameCore::Player::DiplomaticModifierTracker> const&>, '
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker>, eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const>)',
  '566f7b4433e107e613d9ad137776847046a19a6f8bc1230f8f71070115e41af8'],
 [44947312,
  158,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<FixedPointT<8u>, '
  '64ul> >, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator> '
  'const> eastl::for_each<eastl::rbtree_iterator<eastl::pair<long const, Types::StaticArray<FixedPointT<8u>, '
  '64ul> >, eastl::pair<long const, Types::StaticArray<FixedPointT<8u>, 64ul> > const*, eastl::pair<long '
  'const, Types::StaticArray<FixedPointT<8u>, 64ul> > const&>, '
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<FixedPointT<8u>, '
  '64ul> >, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator> '
  'const> >(eastl::rbtree_iterator<eastl::pair<long const, Types::StaticArray<FixedPointT<8u>, 64ul> >, '
  'eastl::pair<long const, Types::StaticArray<FixedPointT<8u>, 64ul> > const*, eastl::pair<long const, '
  'Types::StaticArray<FixedPointT<8u>, 64ul> > const&>, eastl::rbtree_iterator<eastl::pair<long const, '
  'Types::StaticArray<FixedPointT<8u>, 64ul> >, eastl::pair<long const, Types::StaticArray<FixedPointT<8u>, '
  '64ul> > const*, eastl::pair<long const, Types::StaticArray<FixedPointT<8u>, 64ul> > const&>, '
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<FixedPointT<8u>, '
  '64ul> >, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator> '
  'const>)',
  '4c0833e885d215d69876c75d930a210a2b311bde86ba5dc32d137f02e4fee2eb'],
 [44948112,
  155,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<bool, 64ul> >, '
  'eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, eastl::allocator> const> '
  'eastl::for_each<eastl::rbtree_iterator<eastl::pair<long const, Types::StaticArray<bool, 64ul> >, '
  'eastl::pair<long const, Types::StaticArray<bool, 64ul> > const*, eastl::pair<long const, '
  'Types::StaticArray<bool, 64ul> > const&>, '
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<bool, 64ul> >, '
  'eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, eastl::allocator> const> '
  '>(eastl::rbtree_iterator<eastl::pair<long const, Types::StaticArray<bool, 64ul> >, eastl::pair<long '
  'const, Types::StaticArray<bool, 64ul> > const*, eastl::pair<long const, Types::StaticArray<bool, 64ul> > '
  'const&>, eastl::rbtree_iterator<eastl::pair<long const, Types::StaticArray<bool, 64ul> >, '
  'eastl::pair<long const, Types::StaticArray<bool, 64ul> > const*, eastl::pair<long const, '
  'Types::StaticArray<bool, 64ul> > const&>, '
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<bool, 64ul> >, '
  'eastl::map<long, Types::StaticArray<bool, 64ul>, eastl::less<long>, eastl::allocator> const>)',
  'f1230631de0f687e979bcf04fb39d21121f6cee1b082b4267dbdcf69c776f1ab'],
 [44947728,
  158,
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<unsigned int, 64ul> '
  '>, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, eastl::allocator> const> '
  'eastl::for_each<eastl::rbtree_iterator<eastl::pair<long const, Types::StaticArray<unsigned int, 64ul> >, '
  'eastl::pair<long const, Types::StaticArray<unsigned int, 64ul> > const*, eastl::pair<long const, '
  'Types::StaticArray<unsigned int, 64ul> > const&>, '
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<unsigned int, 64ul> '
  '>, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, eastl::allocator> const> '
  '>(eastl::rbtree_iterator<eastl::pair<long const, Types::StaticArray<unsigned int, 64ul> >, '
  'eastl::pair<long const, Types::StaticArray<unsigned int, 64ul> > const*, eastl::pair<long const, '
  'Types::StaticArray<unsigned int, 64ul> > const&>, eastl::rbtree_iterator<eastl::pair<long const, '
  'Types::StaticArray<unsigned int, 64ul> >, eastl::pair<long const, Types::StaticArray<unsigned int, 64ul> '
  '> const*, eastl::pair<long const, Types::StaticArray<unsigned int, 64ul> > const&>, '
  'Serialization::SerializeFromAssociativeContainer<eastl::pair<long, Types::StaticArray<unsigned int, 64ul> '
  '>, eastl::map<long, Types::StaticArray<unsigned int, 64ul>, eastl::less<long>, eastl::allocator> const>)',
  '65aaf5a34db32c076930ea5c5a38e48b1ee514290a16a825bffb464c0286a8a2'],
 [44948448,
  71,
  'Serialization::SerializeFromSequenceContainer<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const, '
  'Types::Vector<eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned '
  'int>, eastl::allocator>, Types::LinearAlloc::Default<1u, 0u> > const> eastl::for_each<eastl::map<unsigned '
  'int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const*, '
  'Serialization::SerializeFromSequenceContainer<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const, '
  'Types::Vector<eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned '
  'int>, eastl::allocator>, Types::LinearAlloc::Default<1u, 0u> > const> >(eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const*, '
  'eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, '
  'eastl::allocator> const*, Serialization::SerializeFromSequenceContainer<eastl::map<unsigned int, '
  'GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned int>, eastl::allocator> const, '
  'Types::Vector<eastl::map<unsigned int, GameCore::Player::DiplomaticModifierTracker, eastl::less<unsigned '
  'int>, eastl::allocator>, Types::LinearAlloc::Default<1u, 0u> > const>)',
  'd3c83ede0d0ad97f8936ab05cdd049cead4c0f6bee3bbe5e4969d7c65b05a104'],
 [44863728,
  15,
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  'e29324d7c538f3cfa8aaaac65e2c8e85ee026fdeb730e69fae66d6ae1edae5c8'],
 [44863888,
  1775,
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, '
  'GameCore::Player::Diplomacy>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&) const',
  'b434caf64e3c35cc5d6b3f16898cb90849eb0eb1be49f2f94c25729f36935a58'],
 [44863744,
  9,
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, '
  'GameCore::Player::Diplomacy>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'],
 [44778000,
  21,
  'Serialization::OStream& operator<< <GameCore::Player::DiplomacyBlackboard, '
  'GameCore::Player::Diplomacy>(Serialization::OStream&, '
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, GameCore::Player::Diplomacy> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'],
 [44779040,
  21,
  'Serialization::IStream& operator>><GameCore::Player::DiplomacyBlackboard, '
  'GameCore::Player::Diplomacy>(Serialization::IStream&, '
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, GameCore::Player::Diplomacy>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55']]
CALLS = [[44934049, 32898000, 62729424, 33341744],
 [44934061, 32347840, 62454344, 44943296],
 [44934073, 32452560, 62506704, 44943392],
 [44934085, 31980576, 62270712, 44943488],
 [44934100, 32522128, 62541488, 44943584],
 [44934115, 32035680, 62298264, 46771104],
 [44934130, 33099232, 62830040, 46772192],
 [44934145, 33099232, 62830040, 46772192],
 [44934160, 32372592, 62466720, 42607696],
 [44934175, 32372592, 62466720, 42607696],
 [44934190, 32372592, 62466720, 42607696],
 [44934205, 32372592, 62466720, 42607696],
 [44934220, 32372592, 62466720, 42607696],
 [44934235, 32372592, 62466720, 42607696],
 [44934250, 32372592, 62466720, 42607696],
 [44934265, 32372592, 62466720, 42607696],
 [44934283, 32372592, 62466720, 42607696],
 [44934332, 31938512, 62249680, 33341856],
 [44934344, 31451392, 62006120, 44943680],
 [44934356, 31981792, 62271320, 44943696],
 [44934368, 31884448, 62222648, 44943712],
 [44934383, 31986000, 62273424, 44943728],
 [44934428, 32239904, 62400376, 33360608],
 [44934462, 32860128, 62710488, 33343664],
 [44934483, 31563328, 62062088, 33566272],
 [44934520, 32860128, 62710488, 33343664],
 [44934541, 31563328, 62062088, 33566272],
 [44934582, 32818528, 62689688, 44943744],
 [44934613, 32818528, 62689688, 44943744],
 [44934644, 32818528, 62689688, 44943744],
 [44934679, 32818528, 62689688, 44943744],
 [44934714, 32818528, 62689688, 44943744],
 [44934749, 32818528, 62689688, 44943744],
 [44934784, 32818528, 62689688, 44943744],
 [44934822, 32818528, 62689688, 44943744],
 [44934848, 32998480, 62779664, 33342240],
 [44934864, 33264848, 61982280, 33444416],
 [44934878, 32989408, 62775128, 33865424],
 [44934928, 32985648, 62773248, 44943792],
 [44934961, 32818528, 62689688, 44943744],
 [44934996, 32818528, 62689688, 44943744],
 [44935031, 32818528, 62689688, 44943744],
 [44935069, 32818528, 62689688, 44943744],
 [44935121, 32898000, 62729424, 33341744],
 [44935132, 31462048, 62011448, 33431440],
 [44935144, 33099232, 62830040, 46772192],
 [44935156, 33099232, 62830040, 46772192],
 [44935168, 33099232, 62830040, 46772192],
 [44935180, 33099232, 62830040, 46772192],
 [44935192, 32898000, 62729424, 33341744],
 [44935204, 32898000, 62729424, 33341744],
 [44935216, 32898000, 62729424, 33341744],
 [44935228, 33099232, 62830040, 46772192],
 [44935240, 32498736, 62529792, 33510448],
 [44935255, 33217184, 62889016, 44943856],
 [44935295, 31938512, 62249680, 33341856],
 [44935306, 32766320, 62663584, 33431552],
 [44935318, 32963584, 62762216, 46772240],
 [44935330, 32963584, 62762216, 46772240],
 [44935342, 32963584, 62762216, 46772240],
 [44935354, 32963584, 62762216, 46772240],
 [44935366, 31938512, 62249680, 33341856],
 [44935378, 31938512, 62249680, 33341856],
 [44935390, 31938512, 62249680, 33341856],
 [44935402, 32963584, 62762216, 46772240],
 [44935435, 33240336, 62900592, 44943888],
 [44935450, 32537392, 62549120, 44943936]]

def binding(game, address):
    start = struct.unpack_from('<Q', game.raw, 40)[0]
    width, count = struct.unpack_from('<HH', game.raw, 58)
    sections = [struct.unpack_from('<IIQQQQIIQQ', game.raw, start+i*width) for i in range(count)]
    for section in sections:
        if section[1] != 4: continue
        for pos in range(section[4], section[4]+section[5], section[9]):
            target, info, addend = struct.unpack_from('<QQq', game.raw, pos)
            if target != address: continue
            if info & 0xffffffff == 8: return addend
            if info & 0xffffffff not in (1,6,7): raise ValueError('unsupported binding')
            symbols = sections[section[6]]
            return struct.unpack_from('<Q', game.raw, symbols[4]+(info>>32)*symbols[9]+8)[0]+addend
    raise ValueError('missing binding')

HARNESS = r'''pub use c6_core::unit;
#[path="MODULE_PATH"]
pub mod player_diplomacy_blackboard;
use player_diplomacy_blackboard::{DiplomacyBlackboard as B,DiplomaticModifierTracker as T};
use unit::Limits;
use std::io::Read;
fn main(){
 let mut data=Vec::new();std::fs::File::open(std::env::args().nth(1).unwrap()).unwrap().read_to_end(&mut data).unwrap();
 let mut p=0;let mut count=0;let mut bytes=0;let mut items=0;let mut trackers=0;let mut maps=0;let mut array_entries=0;let mut names=0;
 while p<data.len(){let kind=data[p];p+=1;let n=u64::from_le_bytes(data[p..p+8].try_into().unwrap())as usize;p+=8;let record=&data[p..p+n];p+=n;
 let (encoded,used)=if kind==0 {let b=B::decode(record,Limits::default()).unwrap();trackers+=b.diplomatic_modifiers.iter().map(Vec::len).sum::<usize>();maps+=b.diplomatic_modifiers.len();array_entries+=b.fixed_point_arrays.len()+b.uint_arrays.len()+b.bool_arrays.len();b.encode_with_items(Limits::default()).unwrap()}else{let t=T::decode(record,Limits::default()).unwrap();names+=t.modifier_name.len();t.encode_with_items(Limits::default()).unwrap()};
 assert_eq!(encoded,record);count+=1;bytes+=n;items+=used;
 }
 println!("{{\"records\":{count},\"bytes\":{bytes},\"items\":{items},\"nested_trackers\":{trackers},\"nested_map_slots\":{maps},\"array_map_entries\":{array_entries},\"standalone_tracker_name_bytes\":{names}}}");
}
'''

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    game=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    Elf(args.assets/'Civ6',HOST_SHA)
    for address,size,name,digest in METHODS:
        if sha(game.read(address,size))!=digest: raise ValueError('method differs: '+name)
    for site,plt,got,target in CALLS:
        game.call(site,plt)
        raw=game.read(plt,6)
        if raw[:2]!=b'\xff%' or plt+6+struct.unpack('<i',raw[2:])[0]!=got or binding(game,got)!=target:
            raise ValueError('overload binding differs')
    if game.read(0x2f5d4fc,4)!=struct.pack('<i',13): raise ValueError('version differs')
    dwarf=subprocess.check_output(['gdb','-nx','-batch',str(args.assets/'libGameCore_XP2.so'),'-ex',"ptype /o 'GameCore::Player::DiplomacyBlackboard'",'-ex',"ptype /o 'GameCore::Player::DiplomaticModifierTracker'"],text=True)
    for token in ['m_fixedPointArrays','m_uintArrays','m_boolArrays','m_diplomaticModifiers','m_warmongerScratchValue','m_aiOneSidedTradeValue','m_isActive','m_modifierName','m_reduction','m_cooldownTurns','2496','56']:
        if token not in dwarf: raise ValueError('DWARF field differs: '+token)
    reference=ROOT/'evidence/reference/upstream/reference/data/layouts.json'
    if sha(reference.read_bytes())!='3c2fedab23ea2f2d16f7c0d62af477c12ff5073ada991aa10d945e8929b670cf': raise ValueError('pinned reference layouts identity differs')
    layouts=[r for r in json.loads(reference.read_text()) if r['cpp'] in ['GameCore::Player::DiplomacyBlackboard','GameCore::Player::DiplomaticModifierTracker']]
    if [(r['cpp'],r['size']) for r in layouts]!=[('GameCore::Player::DiplomacyBlackboard',2496),('GameCore::Player::DiplomaticModifierTracker',56)]: raise ValueError('reference layout lead differs')
    fixtures=[('ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884','trace-benchmark-1.jsonl.gz','528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),('85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8','trace-captured-1.jsonl.gz','027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')]
    source_paths=[ROOT/p for p in subprocess.check_output(['git','ls-files','Cargo.toml','Cargo.lock','crates/c6-core/Cargo.toml','crates/c6-core/src'],cwd=ROOT,text=True).splitlines() if p.endswith(('.rs','.toml','.lock'))]
    owned_module=ROOT/'crates/c6-core/src/player_diplomacy_blackboard.rs'
    if owned_module not in source_paths: source_paths.append(owned_module)
    sources={str(p.relative_to(ROOT)):sha(p.read_bytes()) for p in source_paths}
    with tempfile.TemporaryDirectory(prefix='c6-blackboard-audit-') as work:
        work=Path(work);(work/'src').mkdir()
        (work/'Cargo.toml').write_text('[package]\nname="c6-blackboard-audit"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nc6-core={path='+json.dumps(str(ROOT/'crates/c6-core'))+'}\n')
        harness=HARNESS.replace('MODULE_PATH',str(ROOT/'crates/c6-core/src/player_diplomacy_blackboard.rs'))
        (work/'src/main.rs').write_text(harness)
        command=['cargo','test','--offline','--manifest-path',str(work/'Cargo.toml')]
        subprocess.run(command,check=True,cwd=ROOT)
        subprocess.run(['cargo','clippy','--offline','--manifest-path',str(work/'Cargo.toml'),'--all-targets','--','-D','warnings'],check=True,cwd=ROOT)
        subprocess.run(['cargo','build','--offline','--manifest-path',str(work/'Cargo.toml')],check=True,cwd=ROOT)
        candidate=work/'target/debug/c6-blackboard-audit'
        cases=[]
        for identity,trace_name,trace_sha in fixtures:
            data=gzip.open(args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz'),'rb').read()
            if sha(data)!=identity: raise ValueError('canonical stream differs')
            trace=gzip.open(args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name,'rb').read()
            if sha(trace)!=trace_sha: raise ValueError('owner trace differs')
            owners=[json.loads(line) for line in trace.splitlines()]
            records=[r for r in owners if r.get('event')=='owner' and r.get('save')==1 and r.get('owner') in ['GameCore::Player::DiplomacyBlackboard','GameCore::Player::DiplomaticModifierTracker']]
            batch=bytearray();blackboards=0;trackers=0;payload_hashes=[set(),set()];wire_bytes=[0,0]
            for r in records:
                kind=int(r['owner'].endswith('DiplomaticModifierTracker'));blackboards+=kind==0;trackers+=kind==1
                payload=data[r['start']:r['end']]
                payload_hashes[kind].add(sha(payload));wire_bytes[kind]+=len(payload)
                batch.extend(struct.pack('<BQ',kind,len(payload)));batch.extend(payload)
            (work/'records').write_bytes(batch)
            result=json.loads(subprocess.check_output([str(candidate),str(work/'records')],text=True))
            cases.append(dict(stream_sha256=identity,trace_sha256=trace_sha,blackboards=blackboards,standalone_trackers=trackers,unique_blackboard_payloads=len(payload_hashes[0]),unique_tracker_payloads=len(payload_hashes[1]),blackboard_wire_bytes=wire_bytes[0],tracker_wire_bytes=wire_bytes[1],record_batch_sha256=sha(batch),**result))
        report=dict(schema='c6-linux-player-diplomacy-blackboard-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
                    methods=[dict(address=hex(a),size=n,name=name,sha256=s) for a,n,name,s in METHODS],
                    overload_bindings=[dict(call=hex(a),plt=hex(p),got=hex(g),target=hex(t))for a,p,g,t in CALLS],
                    audit_sha256=sha(Path(__file__).read_bytes()),version=13,dwarf_sha256=sha(dwarf.encode()),reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175',reference_layouts_sha256=sha(json.dumps(layouts,sort_keys=True).encode()),
                    candidate_sha256=sha(candidate.read_bytes()),candidate_harness_sha256=sha(harness.encode()),candidate_sources=sources,
                    rustc=subprocess.check_output(['rustc','--version'],text=True).strip(),cases=cases,item_policy='dynamic container entries and raw string bytes; fixed arrays uncharged',
                    original_game_execution=False,offline_rust_candidate_execution=True,unknown_bytes_masked=0,
                    scope='Current wire records only. No native Read side effects, live diplomacy, planning exclusion, complete Core load or gameplay state/action/turn equivalence.')
    if sources!={str(p.relative_to(ROOT)):sha(p.read_bytes())for p in source_paths}: raise ValueError('candidate sources changed during validation; rerun')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods=len(METHODS),bindings=len(CALLS),cases=cases)))
if __name__=='__main__': main()
