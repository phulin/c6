#!/usr/bin/env python3
"""Static pinned Linux Diplomacy audit and original-record wire roundtrips.

Original modules are only read. Raw records stay in memory and anonymous pipes;
no proprietary record, disassembly or DWARF dump is written by this audit.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
ROOT = Path(__file__).resolve().parents[2]
def sha(value): return hashlib.sha256(value).hexdigest()
METHODS = [(35211264,
  56,
  'Serialization::OStream& Serialization::operator<< <int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions>(Serialization::OStream&, GameCore::IndexList<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions, void> const&)',
  '297d39dc5168d2b4833c6d1fdcbf6b77b67b25169e6706634e1887aabfeba543'),
 (42607696,
  57,
  'Serialization::OStream& Serialization::operator<< <int, 64ul>(Serialization::OStream&, Types::StaticArray<int, '
  '64ul> const&)',
  '1160044aac8668625b62e0243935cc139c38a02e730c536d3f29b877d8e5d554'),
 (44678304,
  71,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DiploVisibilityAddition const&)',
  '1e71cb96cc3af4223dd0668db4588dd95c06a292c67ca569fc5c12b9576c488f'),
 (44678384,
  69,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DiploVisibilityAddition&)',
  'ca20f0300c1f3120ff182d07fd6098191c6a0280a94df0108f138be4a9a7fb72'),
 (44678464,
  41,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::CityWarWeariness const&)',
  '969c1fbc660ae448fb51562021152fb7ab873c7288e0b1fbc2bc07ac51827e79'),
 (44678512,
  41,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::CityWarWeariness&)',
  '0c8de869215fa34d8df8a91033ec6c547266a6592c4436c2ffc697a92ccf167b'),
 (44678688,
  107,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::PromiseData const&)',
  'd646d6c18c6483029a695ef92cdb696f3e8c99f1af87a5f1599dbaf05e894ed9'),
 (44678800,
  162,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::PromiseData&)',
  'ff4a152786e846d7576cd2c88a649f45132da109742307a3afa5cc84041eb4a4'),
 (44678976,
  14,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::IncursionData const&)',
  '85893b2d8838d0e55246b180d6f0518e49ddfbd14f83ae51468e7942ff71d0e1'),
 (44678992,
  14,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::IncursionData&)',
  '35874ca6518b6514993a2070384195a85e9c4313f343ed2e5bb98b220c8e1957'),
 (44679008,
  143,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::CasusBelliData const&)',
  '3e50c499b23a11dc6dfe3162056e8301867242990eb329bd198bd486a46855a9'),
 (44679152,
  225,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::CasusBelliData&)',
  '55cc616835ee528218f5dc113f78bb53d32ef5187b6ef3a8768d34e53fa362cc'),
 (44679392,
  89,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::MessageData const&)',
  '55b6a527bb28504cdcf1cb1345f78ff31006e9cead906243763ea4aa72a715e4'),
 (44679488,
  81,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::MessageData&)',
  '70013695bc7d446245e862d6186ed8ad160327b0102a1fb389bf402a2c24bd56'),
 (44679584,
  101,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DiplomaticYieldModifier const&)',
  '79650b99dc8fb82f8c8c8da7ba99402506f49ac91d9c982648581110be821cb4'),
 (44679696,
  123,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DiplomaticYieldModifier&)',
  '38b68f06a22218550aab4d02513d7093c5889c5ecab74e76f480ad4cec0c9374'),
 (44679824,
  77,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DiplomaticMovementModifier const&)',
  'da94f01a1d80ded2fe15d05a814211db07ef28e0724bbe0180bf5abbef7c2015'),
 (44679904,
  69,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DiplomaticMovementModifier&)',
  'a7793210da1eb61cd200bd53a311b7baf54d5dbba965dde62d019a024ef0ecbd'),
 (44679984,
  89,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DiplomaticCombatModifier const&)',
  'fa4be82788930cdb91d2c58573fa33a1817416bcfab9bd0551e20c8bd504e483'),
 (44680080,
  81,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DiplomaticCombatModifier&)',
  '5a177051cfce216d20ca65a45df405c13bf48e34e647bc46fbe55a67604732c3'),
 (44680176,
  77,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::DiploActionPrereqOverride const&)',
  'b16a1072d4cb1086404dec5bc3fe540381a6bf555b4e87b3a633723507cb7d28'),
 (44680256,
  69,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::DiploActionPrereqOverride&)',
  '199bdd17c28be20c223d252067e5aa29cd06eff61d7d9b19c675fdaab595d6c6'),
 (44680336,
  65,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::BannedDiploActionsSpecificCivilizations '
  'const&)',
  'c13b68eb90e5a8fa5ad1bccf124f423769ae1f7fd0a9d69ec0a59c8a7606c070'),
 (44680416,
  57,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::BannedDiploActionsSpecificCivilizations&)',
  '94c40ba5fd7212c353a6e892931e8cf6d571664d99e9f367fa0260753c530cc4'),
 (44680480,
  1165,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Diplomacy const&)',
  '2bd45eea8a3ae0386543fffe264a6b124da80b759909c1d8fa0c822c6879a6bd'),
 (44681648,
  4513,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Diplomacy&)',
  '78df3c3745bf3876644964e906e2ca822bb7ca32289c5c5029f66d3402658f22'),
 (44688800,
  113,
  'Serialization::operator<<(Serialization::OStream&, eastl::vector_map<GameCore::GossipTypes, unsigned int, '
  'eastl::less<GameCore::GossipTypes>, eastl::allocator> const&)',
  '5221fb02fe9c1be3a22f36dd6dc60ba4318cbaf84b79bc4217e6c8ec7ba91f25'),
 (44688928,
  120,
  'Serialization::operator>>(Serialization::IStream&, eastl::vector_map<GameCore::GossipTypes, unsigned int, '
  'eastl::less<GameCore::GossipTypes>, eastl::allocator>&)',
  '2cf1d5900c0237fa9dc76c8c148f29274313e408e8c635e0c239738f4843694a'),
 (44689408,
  41,
  'Serialization::operator<<(Serialization::OStream&, eastl::pair<GameCore::PromiseTypes, '
  'GameCore::Player::PromiseData> const&)',
  'ee19a7d07facb004074fa6872d4b57a0c8525d36a7ba1cd41355dde8b455c0ac'),
 (44689504,
  113,
  'Serialization::operator<<(Serialization::OStream&, eastl::vector_map<GameCore::PromiseTypes, '
  'GameCore::Player::PromiseData, eastl::less<GameCore::PromiseTypes>, eastl::allocator> const&)',
  'b152d645825fd438caf68f0c167d81dfb1ec77f815210ac67be83d0130a28036'),
 (44689632,
  122,
  'Serialization::operator>>(Serialization::IStream&, eastl::vector_map<GameCore::PromiseTypes, '
  'GameCore::Player::PromiseData, eastl::less<GameCore::PromiseTypes>, eastl::allocator>&)',
  '7a0d6c9075d06a0e330476426b115bb475be7b7aa217b60a05955c9e74d079f5'),
 (44689760,
  41,
  'Serialization::operator<<(Serialization::OStream&, eastl::pair<GameCore::PromiseTypes, '
  'Types::Vector<GameCore::Player::IncursionData, Types::LinearAlloc::Default<1u, 0u> > > const&)',
  '1a062f5d60550eb575a89a3672640b17b3c11bfc3cf1fb40ab6f074db210d57a'),
 (44689856,
  113,
  'Serialization::operator<<(Serialization::OStream&, eastl::vector_map<GameCore::PromiseTypes, '
  'Types::Vector<GameCore::Player::IncursionData, Types::LinearAlloc::Default<1u, 0u> >, '
  'eastl::less<GameCore::PromiseTypes>, eastl::allocator> const&)',
  '11edfe82bcaa119bf8afeacfae0f4eaacd5e81f957a61927f5364c268620fdc2'),
 (44689984,
  151,
  'Serialization::operator>>(Serialization::IStream&, eastl::vector_map<GameCore::PromiseTypes, '
  'Types::Vector<GameCore::Player::IncursionData, Types::LinearAlloc::Default<1u, 0u> >, '
  'eastl::less<GameCore::PromiseTypes>, eastl::allocator>&)',
  '82feb48a7f4c869b304532066a7f381f4457f86ddf88589f3e80ddde568f213f'),
 (44827776,
  15,
  'FAutoVariable<Types::StaticArray<bool, 64ul>, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'ad55e282e274fa29a9930df87a762b5b760ca55685a6355bbdb4ff318644cf14'),
 (44827840,
  15,
  'FAutoVariable<Types::StaticArray<bool, 64ul>, GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '4314a68cb811bf76d9506f34623404474e2a3715034f683019e2943e164526f8'),
 (44830304,
  57,
  'Serialization::OStream& Serialization::operator<< <bool, 64ul>(Serialization::OStream&, Types::StaticArray<bool, '
  '64ul> const&)',
  '1c718422d7b2169aaebd4794682fe49b52d1d7bbb6fefa28def835130cafe9e1'),
 (44830704,
  15,
  'FAutoVariable<Types::StaticArray<int, 64ul>, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '71ae2f34619f98c162a17b2b2cf333c0cc5843ca2d56a5b83e4b538247a87084'),
 (44830768,
  15,
  'FAutoVariable<Types::StaticArray<int, 64ul>, GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  'f1db5b6ef7f015c79e314c2088d5a68d1905c83fc878435a0eb652748729be62'),
 (44833344,
  15,
  'FAutoVariable<bool, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'da5b4928c0247f261986172da3caa7c204af50afd4725a040d7e915d9e833ddc'),
 (44833408,
  15,
  'FAutoVariable<bool, GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '01b11193daf839c4e67ad069a5f846432efa6e3c7974f87693599461dc2f71b4'),
 (44835888,
  15,
  'FAutoVariable<GameCore::IndexList<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions, void>, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '9823d8594880c6927d53b68f113c6c65cafa12260271f363e2f46fd89220b700'),
 (44836000,
  15,
  'FAutoVariable<GameCore::IndexList<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, '
  '&GameCore::Definitions::GetYieldDefinitions, void>, GameCore::Player::Diplomacy>::save(Serialization::OStream&) '
  'const',
  '12ae9852ce527e8246d87cd7db738b73a153f657b55ace3b05107de36a51044c'),
 (44838608,
  15,
  'FAutoVariable<int, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '9a96503637fad8d485ba81cc2317fca3c87499b825b515ca28ce1ba0f6b41e60'),
 (44838672,
  15,
  'FAutoVariable<int, GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '3e651c46ec8b55dfc21c195e6c9421dcdd30a95e3fea9f781ccda3af31863626'),
 (44841168,
  15,
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'af62d81fce6d97873431d0f2ff4d7053ef09717420700e365ce4d0c0c54ffebd'),
 (44841248,
  15,
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '5a9a1288beed6eeaa69d1edc7e20a98cef86520769ef7b129bf8ffb330ee6c74'),
 (44843872,
  15,
  'FAutoVariable<Types::Vector<GameCore::DiplomaticVisibilityTypes, '
  'Types::LinearAlloc::DefaultInline<GameCore::DiplomaticVisibilityTypes, 64u, 1u, 0u> >, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'a98a76d6cfce008fe75cd72a97ad3934b484f3613d409693bd6cb66cc8853c75'),
 (44843984,
  15,
  'FAutoVariable<Types::Vector<GameCore::DiplomaticVisibilityTypes, '
  'Types::LinearAlloc::DefaultInline<GameCore::DiplomaticVisibilityTypes, 64u, 1u, 0u> >, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  'dfda906a13f945ae665441d9f8e5ad23f95f38e1660f0180cd5a8ba2505535f1'),
 (44848576,
  15,
  'FAutoVariable<Types::Vector<GameCore::DiplomaticVisibilitySources, '
  'Types::LinearAlloc::DefaultInline<GameCore::DiplomaticVisibilitySources, 64u, 1u, 0u> >, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'a79151e9aaa42e57974b828f73ee3684999b2f0110487ba41e0dc70b964b0943'),
 (44848688,
  15,
  'FAutoVariable<Types::Vector<GameCore::DiplomaticVisibilitySources, '
  'Types::LinearAlloc::DefaultInline<GameCore::DiplomaticVisibilitySources, 64u, 1u, 0u> >, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '1ca9c5782d2b4a1fb1abe45a76d52ece4f2db09f3f3e6105ba25e594e2c3b332'),
 (44852368,
  15,
  'FAutoVariable<eastl::map<GameCore::PlayerTypes, eastl::vector_map<GameCore::GossipTypes, unsigned int, '
  'eastl::less<GameCore::GossipTypes>, eastl::allocator>, eastl::less<GameCore::PlayerTypes>, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'ad50accdd0cc2eac9fcf09799ad37191adf700a1be2e1622e946bb3722390423'),
 (44852512,
  15,
  'FAutoVariable<eastl::map<GameCore::PlayerTypes, eastl::vector_map<GameCore::GossipTypes, unsigned int, '
  'eastl::less<GameCore::GossipTypes>, eastl::allocator>, eastl::less<GameCore::PlayerTypes>, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  'a2e347c95bc8ea28835728d0e2887065c8524abe65b4605747301dd458c9400b'),
 (44858560,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiploVisibilityAddition, eastl::allocator>, '
  'GameCore::Player::Instance>::load(Serialization::IStream&)',
  '502539f10ab1d85f79a0a8ed40a82f740f861b66f0a3bb5395921de62dbcb613'),
 (44858672,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiploVisibilityAddition, eastl::allocator>, '
  'GameCore::Player::Instance>::save(Serialization::OStream&) const',
  'b4547053379f02f93945a96f0177dfa06e63fbc54ac2defb4a039c9c1ced8485'),
 (44863616,
  15,
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'b58646a1af9d88d3bf29b331da31cda6e5f0f1c6d1d635fd332fb947ea2a6663'),
 (44863728,
  15,
  'FAutoVariable<GameCore::Player::DiplomacyBlackboard, GameCore::Player::Diplomacy>::save(Serialization::OStream&) '
  'const',
  'e29324d7c538f3cfa8aaaac65e2c8e85ee026fdeb730e69fae66d6ae1edae5c8'),
 (44870224,
  15,
  'FAutoVariable<Types::Vector<GameCore::Player::CityWarWeariness, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'd9c399aa8acd1a08f161f36504d253bc88f7be23ac38ea1c1638763a4bca2958'),
 (44870336,
  15,
  'FAutoVariable<Types::Vector<GameCore::Player::CityWarWeariness, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '6f85c80eaeb37bf4c91cfeb4d4947c0fc5f9e698737f06d838afe83c96012095'),
 (44873920,
  15,
  'FAutoVariable<Types::StaticArray<GameCore::Player::CasusBelliData, 64ul>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'e6a7acb05efda7a87b817095c193eb1451ade9029adc7cb30ffba00b055aed87'),
 (44874000,
  15,
  'FAutoVariable<Types::StaticArray<GameCore::Player::CasusBelliData, 64ul>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  'f252f418bbf10a1b263373670d6c0ffaebf6f50ebb99ac0411c3ac097bf1b442'),
 (44876544,
  57,
  'Serialization::OStream& Serialization::operator<< <GameCore::Player::CasusBelliData, 64ul>(Serialization::OStream&, '
  'Types::StaticArray<GameCore::Player::CasusBelliData, 64ul> const&)',
  '8d685dc8d82f778e0008421e08c9a066d41f72542bec541f6cfa94c06ff73d48'),
 (44876992,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiplomaticYieldModifier, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'da0c1c86585310d6e8e71a329d0b8ae1ea223e33cd16dc0b1541880fbaa1cee2'),
 (44877104,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiplomaticYieldModifier, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '76fda84c614eaa5a5121073889d5fdcc7046ede009ea7d3386a27f5bd5eb80b8'),
 (44880768,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiplomaticMovementModifier, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'aafb42c38eb88609edd519dcf54271a79aa0d3ac75e253c586c3909ec88773b5'),
 (44880880,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiplomaticMovementModifier, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '4fef5cb7ec528634e0ae919fae5728ea0c5af1d003bedf8ade30bd75872c2044'),
 (44884624,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiplomaticCombatModifier, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '805c6fa79d6adc6d2ee537910a7a1a17e3d30d5089b49e68a9af761952b8f0cd'),
 (44884736,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiplomaticCombatModifier, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '239550f215f645be4629d9ece92e97cab6a81a198ec0f814e1eef644a6eb29bc'),
 (44888448,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiploActionPrereqOverride, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '9fa2513986b832e810785540613f9abf02a445058703ecb2f1e8734e3fb824b6'),
 (44888560,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::DiploActionPrereqOverride, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  'd54f9ae778d2e8fabc0dea5bd5de1a3ad96871c13fbfa6a4ac6712a28098b146'),
 (44892304,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::BannedDiploActionsSpecificCivilizations, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '634082e30405838fdf589be786e07083ed9fe2a6ea23a53f8f0d3f356c8bde34'),
 (44892416,
  15,
  'FAutoVariable<eastl::vector<GameCore::Player::BannedDiploActionsSpecificCivilizations, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '41a8e39655a17431af59e08084020b262d6add81eee1c60269d47723107cb84b'),
 (44896080,
  15,
  'FAutoVariable<Types::Vector<GameCore::Player::MessageData, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '67836934811a6be7ab8c23aefcff81ff22f33a31fb0cf166a697f3f4d8fc6e28'),
 (44896192,
  15,
  'FAutoVariable<Types::Vector<GameCore::Player::MessageData, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  'f976257fae6bdddd9a5eab064bf66685c800b8e25bbcb890ec9d7be909e7e844'),
 (44899712,
  15,
  'FAutoVariable<eastl::map<GameCore::PlayerTypes, eastl::vector_map<GameCore::PromiseTypes, '
  'GameCore::Player::PromiseData, eastl::less<GameCore::PromiseTypes>, eastl::allocator>, '
  'eastl::less<GameCore::PlayerTypes>, eastl::allocator>, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  'f337099bd8ab610cc42941ed05b6bfb4e55ab8f85b41f8e2d12f0a84ae74a3b6'),
 (44899856,
  15,
  'FAutoVariable<eastl::map<GameCore::PlayerTypes, eastl::vector_map<GameCore::PromiseTypes, '
  'GameCore::Player::PromiseData, eastl::less<GameCore::PromiseTypes>, eastl::allocator>, '
  'eastl::less<GameCore::PlayerTypes>, eastl::allocator>, GameCore::Player::Diplomacy>::save(Serialization::OStream&) '
  'const',
  'aeb1ac138dca44e37d2685c870b7c63d157fdffb3848c4f5f3b58636bb9e1bc8'),
 (44904032,
  15,
  'FAutoVariable<eastl::map<GameCore::PlayerTypes, eastl::vector_map<GameCore::PromiseTypes, '
  'Types::Vector<GameCore::Player::IncursionData, Types::LinearAlloc::Default<1u, 0u> >, '
  'eastl::less<GameCore::PromiseTypes>, eastl::allocator>, eastl::less<GameCore::PlayerTypes>, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '3f5804e51be8052d29170acad1310541cb6d34fc47e7abed2543528ab75babb4'),
 (44904176,
  15,
  'FAutoVariable<eastl::map<GameCore::PlayerTypes, eastl::vector_map<GameCore::PromiseTypes, '
  'Types::Vector<GameCore::Player::IncursionData, Types::LinearAlloc::Default<1u, 0u> >, '
  'eastl::less<GameCore::PromiseTypes>, eastl::allocator>, eastl::less<GameCore::PlayerTypes>, eastl::allocator>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '38d5029e45e170ebc79be8b91ed2ca1c25e165302e71fae92288a9c26f046661'),
 (44908592,
  15,
  'FAutoVariable<Types::Vector<bool, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '9f7eb5302527856420202ee3fad523d247d95edccdc75e3254d3e7f0dcc53483'),
 (44908704,
  15,
  'FAutoVariable<Types::Vector<bool, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '76ad4a0b49f2df7df487a39d338fbf4daa6c7cc6a0ba0cabe28e1944b23b9a15'),
 (44911360,
  15,
  'FAutoVariable<eastl::vector<int, eastl::allocator>, GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '9da67ad0728110db2369ba7e880a839fb2bee1aeb74e45b294c3be584391ee97'),
 (44911472,
  15,
  'FAutoVariable<eastl::vector<int, eastl::allocator>, GameCore::Player::Diplomacy>::save(Serialization::OStream&) '
  'const',
  '0cc52fcb4fe9b5d52d86743757d214b2f10481d67001df84e863b0827a25b78f'),
 (44914176,
  15,
  'FAutoVariable<Types::StaticArray<GameCore::IndexList<bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Discussion>, '
  '&GameCore::Definitions::GetDiscussionDefinitions, void>, 64ul>, '
  'GameCore::Player::Diplomacy>::load(Serialization::IStream&)',
  '55753d92f1a6cfccc3c714021404d17180bc2d89180ff6b3fac498660a98250f'),
 (44914288,
  15,
  'FAutoVariable<Types::StaticArray<GameCore::IndexList<bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Discussion>, '
  '&GameCore::Definitions::GetDiscussionDefinitions, void>, 64ul>, '
  'GameCore::Player::Diplomacy>::save(Serialization::OStream&) const',
  '25cd02f626b4be387eb7b39b54a267b2528ff5ce0e026487bc36fec28d34f1fe'),
 (46771104,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::PlayerTypes const&)',
  'e3aa2b85edbfcad11d2fe4b8d720bad7d662e88d8461f7994ba25444ae60c6d1'),
 (46771152,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::PlayerTypes&)',
  '2cc19dccb1ce5ba3727486d694c79a0b5c90c1d7658cf975ce07ade2572a6ee7'),
 (46772288,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::DiplomaticVisibilityTypes const&)',
  'd2d6ffd3c11ef7d8a27751ea476f900ab91ed41f7b5096809a8ce1a1cb45fb63'),
 (46772336,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::DiplomaticVisibilityTypes&)',
  '02ed683c15087dcf7f3dd7f89396316d0bf8d3a28cc8a8f0b9926429274990e7'),
 (46772384,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::DiplomaticSourceTypes const&)',
  '1ffc8ff7a0abd5eb403a8456ab82e0787b5e62746c061fef8fb7660344c6a6ed'),
 (46772432,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::DiplomaticSourceTypes&)',
  '87234f60e8c5223c9c1689c1f0093c0f10e80e890f2637c926da2b7655210630'),
 (47508336,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::CivicTypes const&)',
  '259301d4abef819cc377acf6436d4036a2241bb2b57f9241fe4aea3a13ac0f02'),
 (47508384,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::CivicTypes&)',
  '23ed427d7285eff6b631c259b039d49a30273a5e0f8af7daa84500f258d2588f'),
 (47509232,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::TechTypes const&)',
  'e8b21f56f96030386a4c9aac4582d7fce63120854b0a642cf15392ac5210cdad'),
 (47509280,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::TechTypes&)',
  '047050d264acc31d6a4477873412f54329d63a12b971850fa8ca7602c1cfa31c'),
 (47511232,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::AI::DiplomaticActions const&)',
  'b21d9e9d756259a46b9db2f4840b84152532b6e72572d04e9401c534df44d715'),
 (47511280,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::AI::DiplomaticActions&)',
  '6d269a75b310726cb1644ecce39fad8f2eb14ae1a99be5935272dae2670a6633'),
 (47511792,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::DiplomaticVisibilitySources const&)',
  '88ceccfdd4467e9fdd18ba9d60580aad53a46d49270bb7bd36a03791015d19a7'),
 (47511840,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::DiplomaticVisibilitySources&)',
  '3049f249e27569a5bd1a0939914aca7faf8d629e6861d8b2a26f2690bad89100'),
 (47511904,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::CivilizationTypes const&)',
  'd32b17ab228ea716cd7ba43ff537885bb7941489a31ffbf8769bc7612dc068fe'),
 (47511952,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::CivilizationTypes&)',
  'd75f2a773fc176d231456fd29d3a39f343e0ad866a931825dd2c365be7b62542'),
 (47512432,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::YieldTypes const&)',
  'b971caad466bb8cb3baf0b50ec89d1f83d536c9d25f15145cc664077fdd4a5c7'),
 (47512480,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::YieldTypes&)',
  '8f3a33cbd2aca7db1a0c6d4e9fae3b8ea968171068bccd693ea5ca8982362a12'),
 (47513872,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::GossipTypes const&)',
  'f232c14d755e4f388e8814d8956e09ca4cd4e00b6b5a1270540949abadadb0da'),
 (47513920,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::GossipTypes&)',
  '341d2599899e758e2f0f220e465a43ce3bbba2dc5f607aa104a312d5b2c2b61d'),
 (47513968,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::PromiseTypes const&)',
  '9a91a6f2495136f92160ada6f40854b8313c6bad439487b2bb31b2dc0068a946'),
 (47514016,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::PromiseTypes&)',
  '6946fc71a0eb54541a81ab0e5f4fd83dbe1497fc3b6524f28934e3cbe68256fe'),
 (47514064,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::DiplomaticYieldSources const&)',
  '5fdc835bac7a1de605992965dbfaeb7b66227d6f945f331c8d568e39dd4a702c'),
 (47514112,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::DiplomaticYieldSources&)',
  '0086a02957d3850d8cc0e8cd75f39c3e22a989832be111fe3bce9dfcfb548a54')]
VTABLES = [(61702312, 40, 44841248), (61702312, 16, 44841168), (61703656, 40, 44874000), (61703656, 16, 44873920), (61705768, 40, 44914288), (61705768, 16, 44914176), (61701352, 40, 44827840), (61701352, 16, 44827776), (61701544, 40, 44830768), (61701544, 16, 44830704), (61702504, 40, 44843984), (61702504, 16, 44843872), (61702696, 40, 44848688), (61702696, 16, 44848576), (61704808, 40, 44896192), (61704808, 16, 44896080), (61703464, 40, 44870336), (61703464, 16, 44870224), (61705384, 40, 44908704), (61705384, 16, 44908592), (61702888, 40, 44852512), (61702888, 16, 44852368), (61705192, 40, 44904176), (61705192, 16, 44904032), (61705000, 40, 44899856), (61705000, 16, 44899712), (61703080, 40, 44858672), (61703080, 16, 44858560), (61703848, 40, 44877104), (61703848, 16, 44876992), (61704232, 40, 44884736), (61704232, 16, 44884624), (61704424, 40, 44888560), (61704424, 16, 44888448), (61704040, 40, 44880880), (61704040, 16, 44880768), (61704616, 40, 44892416), (61704616, 16, 44892304), (61705576, 40, 44911472), (61705576, 16, 44911360), (61703272, 40, 44863728), (61703272, 16, 44863616), (61701928, 40, 44836000), (61701928, 16, 44835888), (61701736, 40, 44833408), (61701736, 16, 44833344), (61702120, 40, 44838672), (61702120, 16, 44838608)]
FIXTURES = [
    ('ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884', 'trace-benchmark-1.jsonl.gz', '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
    ('85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8', 'trace-captured-1.jsonl.gz', '027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369'),
]
RUNNER = r'''#![allow(dead_code)]
use std::io::{Read, Write};
mod unit { pub use c6_core::unit::*; }
mod player_diplomacy_blackboard { pub use c6_core::player_diplomacy_blackboard::*; }
#[path = "@ROOT@/crates/c6-core/src/player_diplomacy.rs"]
mod player_diplomacy;
fn main() {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    let limits = unit::Limits::default();
    let (v, used, items) = player_diplomacy::Diplomacy::decode_prefix_with_items(&input, limits).unwrap();
    let (encoded, encoded_items) = v.encode_with_items(limits).unwrap();
    assert_eq!(items, encoded_items);
    let row = format!("{{\"consumed\":{used},\"items\":{items},\"weariness\":{},\"yield_modifiers\":{},\"movement_modifiers\":{},\"combat_modifiers\":{},\"prereq_overrides\":{},\"banned_civilizations\":{},\"visibility_additions\":{},\"gossip_players\":{},\"messages\":{},\"promise_players\":{},\"incursion_players\":{},\"last_promise_players\":{},\"unit_hash_pairs\":{},\"action_hash_pairs\":{}}}\n",
        v.city_war_weariness.len(), v.yield_modifiers.len(), v.movement_modifiers.len(), v.combat_modifiers.len(), v.action_prereq_overrides.len(), v.banned_action_civilizations.len(), v.visibility_additions.len(), v.gossip_reports.len(), v.held_messages.len(), v.promises_made.len(), v.incursions_made.len(), v.last_promises_made.len(), v.units_blocked_entry.len(), v.banned_action_counts.len());
    let mut out = std::io::stdout().lock();
    out.write_all(row.as_bytes()).unwrap();
    out.write_all(&encoded).unwrap();
}
'''
def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root', type=Path, default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    game = Elf(args.assets/'libGameCore_XP2.so', GAME_SHA)
    Elf(args.assets/'Civ6', HOST_SHA)
    methods = []
    calls = []
    for address, size, name, expected in METHODS:
        actual = sha(game.read(address, size))
        if actual != expected: raise ValueError('target method differs: '+name)
        methods.append(dict(address=hex(address), size=size, name=name, sha256=actual))
        disassembly = subprocess.check_output(['objdump','-d','--start-address='+str(address), '--stop-address='+str(address+size), str(args.assets/'libGameCore_XP2.so')], text=True)
        for line in disassembly.splitlines():
            m = re.match(r'\s*([0-9a-f]+):.*\bcall\s+([0-9a-f]+)\s+<(.*)>', line)
            if m:
                site, target, symbol = m.groups()
                site, target = int(site,16), int(target,16)
                game.call(site,target)
                calls.append(dict(site=hex(site), target=hex(target), symbol=symbol))
    virtual_calls = []
    for table, slot, target in VTABLES:
        if game.relocated_pointer(table+16+slot) != target: raise ValueError('FAutoVariable virtual binding differs')
        virtual_calls.append(dict(vtable=hex(table), vptr_slot=hex(slot), target=hex(target)))
    for address, expected in [(0x2f59980,50),(0x2f59984,1),(0x2f5998c,1),(0x2f59990,1)]:
        if struct.unpack('<I',game.read(address,4))[0] != expected: raise ValueError('current version differs')
    dwarf = subprocess.check_output(['gdb','-nx','-batch',str(args.assets/'libGameCore_XP2.so'), '-ex', "ptype /o 'GameCore::Player::Diplomacy'", '-ex', "ptype /o 'GameCore::Player::CasusBelliData'"], text=True)
    for token in ['m_blackboard', 'm_GossipReportMap', 'm_PromisesMadeMap', 'm_LastPromisesMadeMap', 'm_IncursionsMadeMap', 'm_bIdeologicalWar']:
        if token not in dwarf: raise ValueError('target DWARF differs: '+token)
    source_paths = [ROOT/'crates/c6-core/src/player_diplomacy.rs', ROOT/'crates/c6-core/src/player_diplomacy_blackboard.rs', ROOT/'crates/c6-core/src/unit.rs', ROOT/'crates/c6-core/Cargo.toml', ROOT/'Cargo.toml', ROOT/'Cargo.lock']
    sources = {str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in source_paths}
    runner = RUNNER.replace('@ROOT@', str(ROOT))
    fixtures = []
    with tempfile.TemporaryDirectory(prefix='c6-diplomacy-audit-') as directory:
        directory = Path(directory)
        (directory/'src').mkdir()
        (directory/'src/main.rs').write_text(runner)
        (directory/'Cargo.toml').write_text('[package]\nname="c6-diplomacy-audit"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nc6-core={path='+json.dumps(str(ROOT/'crates/c6-core'))+'}\n')
        subprocess.run(['cargo','build','--offline','--manifest-path',str(directory/'Cargo.toml')], check=True)
        candidate = directory/'target/debug/c6-diplomacy-audit'
        candidate_sha = sha(candidate.read_bytes())
        for identity, trace_name, trace_sha in FIXTURES:
            data = gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
            if sha(data) != identity: raise ValueError('original Core identity differs')
            trace = gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
            if sha(trace) != trace_sha: raise ValueError('historical owner trace differs')
            owners = [json.loads(x) for x in trace.splitlines()]
            rows = [x for x in owners if x.get('event')=='owner' and x.get('save')==1 and x.get('owner')=='GameCore::Player::Diplomacy']
            records = []
            for row in rows:
                raw = data[row['start']:row['end']]
                output = subprocess.check_output([str(candidate)], input=raw)
                header, encoded = output.split(b'\n',1)
                result = json.loads(header)
                if result['consumed'] != len(raw) or encoded != raw: raise ValueError('original record wire roundtrip differs')
                result.update(start=row['start'], end=row['end'], sha256=sha(raw))
                records.append(result)
            fixtures.append(dict(stream_sha256=identity, trace_sha256=trace_sha, records=records, count=len(records), bytes=sum(x['consumed'] for x in records)))
        if sources != {str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in source_paths}: raise ValueError('sources changed during audit')
    report = dict(schema='c6-linux-player-diplomacy-wire-v1', checks_passed=True, gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        methods=methods, direct_call_bindings=calls, virtual_call_bindings=virtual_calls, dwarf_sha256=sha(dwarf.encode()), candidate_sources=sources, candidate_sha256=candidate_sha,
        runner_sha256=sha(runner.encode()), audit_source_sha256=sha(Path(__file__).read_bytes()), fixtures=fixtures,
        discovery_reference_commit='5af389445118bed2bdebf7a8a42b219c8fba5175',
        reference_layouts_sha256=sha((ROOT/'evidence/reference/upstream/reference/data/layouts.json').read_bytes()),
        full_current_diplomacy_wire_records=True, fixed_arrays_uncounted=True, ordered_maps_preserved=True,
        current_outer_version=50, native_read_application=False, constructor_defaults_reproduced=False,
        complete_player_import=False, full_core_import=False, full_gameplay_state=False, action_execution=False, turn_execution=False,
        original_game_execution=False, original_read_oracle=False, offline_rust_candidate_execution=True, unknown_bytes_masked=0,
        next_player_dependency='GameCore::Player::Districts')
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, methods=len(methods), calls=len(calls), records=sum(x['count'] for x in fixtures), bytes=sum(x['bytes'] for x in fixtures), original_game_execution=False)))
if __name__ == '__main__': main()
