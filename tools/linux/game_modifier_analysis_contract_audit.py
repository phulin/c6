#!/usr/bin/env python3
"""Read-only, exact-SHA Linux GameModifierAnalysis v17 contract audit.

No original execution, proprietary byte archive, hook inference or gameplay claim.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA

METHODS = [(33341744,
  14,
  'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)',
  '2a3fc110a86f71122b45f03f192ddbc4499241d298e49b843ea750bd49d5e1cf'),
 (33341760,
  7,
  'GameCore::Player::Instance::GetID() const',
  'e997d06cf0d3f9b3bb89701ec15a7b1b096198fdc4b0dc87fc0ca29c37f6dadc'),
 (33341776,
  14,
  'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)',
  '9731ca1d2959dc0f406a684267a29d425c9bf7ea2f127425123508fbc2ed2d3b'),
 (33341856,
  14,
  'Serialization::IStream& Serialization::IStream::operator>><int>(int&)',
  'c6cf9d3175d5134bd8d7b3d396184fd7b9abd3f7ca364cbdc38a1794ce742775'),
 (33341872,
  12,
  'GameCore::Player::Manager::GetPlayer(GameCore::PlayerTypes)',
  '47fe7c239b1abbb63533198084d4b834d08e20f997a7b46ccce8700c65b4a2fb'),
 (33341888,
  14,
  'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)',
  '5edd23b31cc38a2cc74b61c4b33889da2f35c69f9ce10ee24f2899eb7e91af01'),
 (33347392,
  32,
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >::clear()',
  '1c9847eddcf42328846620077a04e7cab3fa7bc7ae77f461c05184f2cc41a2a0'),
 (33347536,
  5,
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >::begin() const',
  '5042e5f30d6a690da58e89ba23dbce53117a565d68ae61f3c26261671c92a765'),
 (33347552,
  4,
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >::size() const',
  '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
 (33348384,
  25,
  'Serialization::IStream::ReadRequest::ReadRequest(unsigned long, unsigned long)',
  '43f291f25ae0e59e98bf6b96e5341ed0345d1627c75c056bac01ada2cfd2f796'),
 (33362048,
  4,
  'GameCore::Player::ComponentID::GetID() const',
  '4703e829cd2c504bae1968b685216f9f191e08f6e2e2a6f3be0a737a92c43721'),
 (33431440,
  14,
  'Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)',
  '441a140d71c552a6ced859e0e9efa0994c519cad451b975cbb5fc13e8025cd3f'),
 (33431456,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resource> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resource> const&)',
  '87ca14d4e4fc124b181ba99258a0860e40ec36d111b53045467e0f01bba6d5d1'),
 (33431504,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> const&)',
  'b430e29bbc8350689b66d38971c62532dbbe0a9c882dbb96cf258ebacd87b84f'),
 (33431552,
  14,
  'Serialization::IStream& Serialization::IStream::operator>><bool>(bool&)',
  '8354dc947302541cc1e97c714b223e16c8e9f086c7d09ad77b437291a622809d'),
 (33432048,
  14,
  'GameCore::Player::ComponentID::ComponentID()',
  'a5184cacf555e2da043cf526b6d3b6116038743d94e93bc28ca61ecea202dbfc'),
 (33432064,
  26,
  'GameCore::City::IInstance::GetComponentID() const',
  'daa8152880488f56c4abaf01bd542858aa6f35fd0a18ebec401d9073ba78057a'),
 (33447072,
  30,
  'void Serialization::EnumFromStream<GameCore::PlayerTypes>(Serialization::IStream&, '
  'GameCore::PlayerTypes&)',
  '64a2263c6cb95736e735600c61a22e07f12969b393675bb416f45dcabee8ff95'),
 (33447552,
  28,
  'Serialization::IStream& Serialization::ReadVersioned<bool, bool>(Serialization::IStream&, bool&, int, '
  'int&&, bool&&)',
  '57e8e97c7be2741a1dc218f74049b9cf7a99191cffd41785ce1aa9dd4c6518e5'),
 (33450864,
  4,
  'GameCore::Player::ComponentID::GetOwner() const',
  'eea57f4eeb9f601ec3b1154cc785380880eae152e709e8156a9039bd8e939aa9'),
 (33454336,
  14,
  'Serialization::OStream::WriteObject(bool const&)',
  '0b101fff740deefceb76f32a685baf980dce55ee6f804dd16993e783e33aebba'),
 (33456544,
  75,
  'Serialization::IStream::ReadObject(bool&)',
  '1b35f5e559410da6d1adcde91f94b8b0ee9a16f84d871535a2d18c3ae544487b'),
 (33511168,
  44,
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >::Vector()',
  '20648c93f23bf061670f3db3a143675a7005dc4825da856b487574684cbe0349'),
 (33513968,
  14,
  'Serialization::IStream& Serialization::operator>><unsigned int, Types::LinearAlloc::Default<1u, 0u> '
  '>(Serialization::IStream&, Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >&)',
  '7afbe9a950c1d79e1fd93aabdaf26e16cf5e89ff7b89de4a62add3a7598b29b9'),
 (33513984,
  125,
  'void Serialization::SerializeToSequenceContainer<unsigned int, Types::Vector<unsigned int, '
  'Types::LinearAlloc::Default<1u, 0u> > >(Serialization::IStream&, Types::Vector<unsigned int, '
  'Types::LinearAlloc::Default<1u, 0u> >&)',
  'db836a9369ae025a9cc36324802e64d01fbfe230f74a28c57f041c41d33813e6'),
 (33514112,
  19,
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >::reserve(unsigned long)',
  'ccc8ffb5b7c4e7bc3603f56555e04b28081b5e33ec9f8f2564611922085ca8ea'),
 (33514144,
  44,
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >::push_back()',
  'f8781a64edb0f70e84c79f700646c795673685e86d8e15653e0085361ed98988'),
 (33514192,
  21,
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >::back()',
  '632a345563dd8ea8f5d2cf040d1e684774c41eb0244e0a48577a592a26ccd617'),
 (33514224,
  91,
  'Serialization::OStream& Serialization::operator<< <unsigned int, Types::LinearAlloc::Default<1u, 0u> '
  '>(Serialization::OStream&, Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> > const&)',
  '476fbcac3fff471c3959fc4a414507918bb1f6ea71fdf889b874c81a46421ce3'),
 (33514320,
  71,
  'Serialization::SerializeFromSequenceContainer<unsigned int const, Types::Vector<unsigned int, '
  'Types::LinearAlloc::Default<1u, 0u> > const> eastl::for_each<unsigned int const*, '
  'Serialization::SerializeFromSequenceContainer<unsigned int const, Types::Vector<unsigned int, '
  'Types::LinearAlloc::Default<1u, 0u> > const> >(unsigned int const*, unsigned int const*, '
  'Serialization::SerializeFromSequenceContainer<unsigned int const, Types::Vector<unsigned int, '
  'Types::LinearAlloc::Default<1u, 0u> > const>)',
  '3550beaf4fe9a53c57b5e48302397f9c00fd4e3911cd8145dd4cd86a536af78c'),
 (33514400,
  18,
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> >::end() const',
  'd24556e12caef78b88a8aed299be346071cdddeecbe2745e3719d47202f6db33'),
 (33514432,
  46,
  'Serialization::SerializeFromSequenceContainer<unsigned int const, Types::Vector<unsigned int, '
  'Types::LinearAlloc::Default<1u, 0u> > const>::SerializeFromSequenceContainer(Serialization::OStream&, '
  'Types::Vector<unsigned int, Types::LinearAlloc::Default<1u, 0u> > const&)',
  '7fd2b9fa1d7e744e641d8b921c3d9ba728d1b9ae0f8af8ca666892e6ba0e84c1'),
 (33514480,
  8,
  'Serialization::SerializeFromSequenceContainer<unsigned int const, Types::Vector<unsigned int, '
  'Types::LinearAlloc::Default<1u, 0u> > const>::operator()(unsigned int const&)',
  'c4a2bafd5a2b4e3f2ab880a5b036ef521543e3530632dbc2506a743d23bfe41a'),
 (33525376,
  21,
  'GameCore::Player::ComponentID::ComponentID(GameCore::Player::ComponentID const&)',
  '574ca5ba9db429a6a5a90e019d12cd702ad1b683b676123b9edeb6e55212c0d6'),
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
 (33563888,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> const&)',
  'f607bba3c44cfadb81e87f8bee69f39e86eb834d2fc0b6ca043bce343e4df2cd'),
 (33564496,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Unit> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Unit> const&)',
  'f9c016f8938c1e5bd223d7dc032ddd32298e58b21dcdd57f1371a102854e09c5'),
 (33564544,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Unit> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Unit> const&)',
  '474cea602fe4e91719bec7b48293e2dbb2a31c0616e983ff51dc33cf37583f98'),
 (33566912,
  19,
  'void Serialization::EnumToStream<GameCore::PlayerTypes>(Serialization::OStream&, GameCore::PlayerTypes '
  'const&)',
  '53e00befd8655db0c427919039335f3534f6be53858eb37a84c25490081ba08b'),
 (33574848,
  51,
  'GameCore::Player::Manager::IsValid(GameCore::PlayerTypes) const',
  '31c2b843f442ca71b8fd27edeccb2888940a331a9f50686c2a1e078cd25be038'),
 (33592304,
  14,
  'Serialization::IStream& Serialization::IStream::operator>><float>(float&)',
  '51003a632d7bc3a8e1c9922c292a112ef2b1fbea2b76377069657aaa9ddc35a3'),
 (33592320,
  75,
  'Serialization::IStream::ReadObject(float&)',
  '162448178ea2853cec3bc120666181791a5fcd9db6c09a632121344a61a9c261'),
 (33611280,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Building> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Building> const&)',
  '1939672915ff2e1e1959ad3cb086cd005b04c5645547e821f3467cc46aec1555'),
 (33611424,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Building> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Building> const&)',
  '9c052610b9e747d5ba228307932c28b24648909ede4dfa6660a7d4073661d79c'),
 (33612288,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  '175d86dbe583455f87afaeec42db117ecf32444a8129b3fbd0f1e2306ca31abc'),
 (33612544,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Civic> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Civic> const&)',
  '7dffe823af43a1ef66fd8005148909a935ee64615854520f37db17064376084b'),
 (33612608,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Civic> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Civic> const&)',
  '30966cd539108e54de0b986564596f601bea6de6428eb6fe4e1e72eaa9c6189f'),
 (33772688,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatWorkObjectType> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatWorkObjectType> '
  'const&)',
  '8aefd90e0f2647e88fa7dc3d124a56baff5d6a5e7819d1054c8f39f3919367c3'),
 (33773104,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Policy> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Policy> const&)',
  'd437eb0126eb7cfa477b825d656e1ba8961da5ac2c579f6f66aec4831c68617e'),
 (33773152,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentSlot> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentSlot> const&)',
  'e0bfcbde489df2d82efc9fe9edb5035da3bfcf5fcf3fc4c1570a9f15e3d13923'),
 (33773248,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentSlot> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentSlot> const&, '
  'int, int&&, int&&)',
  '88e15c0ce1f66c1edd265340f3ab88585e4d102b4a7a138eb77cb79eaa68fb39'),
 (33773392,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> const&)',
  '67cbdda95ab1e62dd546173c095c798de3dfc38b6c45cfeee8e4d3d70794c250'),
 (33773440,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> const&)',
  '022092141a2bc75b9de63a7cb602f1169131ad2a3a53e9ad03abc2e9be64792d'),
 (34485856,
  997,
  'Serialization::operator<<(Serialization::OStream&, GameCore::AI::GameModifierAnalysis const&)',
  '4b572f3b68823acb99ec5c237a8a255934a93cd3cd8a96196d5f4c01b5412f5e'),
 (34486864,
  1523,
  'Serialization::operator>>(Serialization::IStream&, GameCore::AI::GameModifierAnalysis&)',
  '18c5e8104f560248549006784219bddca5beab68d5326cdc2fe169c8bc8c2a89'),
 (34503872,
  156,
  'GameCore::AI::GameModifierAnalysis::GameModifierAnalysis()',
  '84f2a1bd368775116b15b067617ca9b271d8ee9ccc1d92c0c3940a8fc1809e4d'),
 (34504912,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Belief> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Belief> const&)',
  '0ca83f046480dcd37cbda7d104720efaeb6a8cc9689c7f16e41167417a67f25e'),
 (34504960,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Difficulty> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Difficulty> const&)',
  '3b29c91d0a99feb27b4807e5d2e35f6b4dfe0b1c9995bc14728b955435e4e7de'),
 (34505008,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> const&)',
  'a560edd5df226a25c3074ae777c6e5a5a0e6e45f73a7418ebc348b671fa69bf8'),
 (34505056,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> const&)',
  '2a0ffdb40f495fa6c05ce7a9f0b19767be589f826a14de8ba7b086d3e7234811'),
 (34505104,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Governor> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Governor> const&)',
  'aed00456573c6f22bf893e0b97c3a08d26eb8e918347b7ffd9956bfb92863261'),
 (34505152,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Government> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Government> const&)',
  '3dfc2e8e724fb95f48ccbf1e15fd73518e03f352fb8942902768d0ae24938ec2'),
 (34505200,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Project> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Project> const&)',
  '12545b05a4862e4985988aeca84dd2bc2b599740b47b5a8d5b007730b4ec318e'),
 (34505248,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonClass> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonClass> const&)',
  'be5fa6e8bd93ef5c4812c3ef44ca17df213d840b557521d1d14456415db16034'),
 (34505296,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation> const&)',
  '49d50d829c3dd5a764b13b4395e63b81560960c8e05bc10a0a5ee8208d488cf1'),
 (34505344,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> const&)',
  'c3455c01ec6f10476e969f484c0b179f0946eee1faa29479657134922725224d'),
 (34505392,
  19,
  'void Serialization::EnumToStream<GameCore::ReligionTypes>(Serialization::OStream&, '
  'GameCore::ReligionTypes const&)',
  '50ff09c522f898ea8aa2a28e13e2baa91561357e0924d58c26fc755e3623e02e'),
 (34505424,
  14,
  'Serialization::OStream& Serialization::OStream::operator<< <float>(float const&)',
  'fcd58fecc874fad08c98ce8808408c091b35b1b461e1c1f99d801848224e8d07'),
 (34505440,
  26,
  'GameCore::Unit::IInstance::GetComponentID() const',
  '4dd57984270bbcfbc8ea3273d4dce7454e1a80b27cc975a263084961359155b2'),
 (34505472,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Belief> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Belief> const&)',
  'cce3a95b3aadb3e96ed3075085c48dec059c4e3757e6ae90a950255d452ebcc7'),
 (34505520,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Feature> const&, int, '
  'int&&, int&&)',
  '26f29cbd4d46edfb44ca38ea5263deeb2abe8f3ca64a48186353953dab672cdf'),
 (34505568,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Difficulty> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Difficulty> const&, int, '
  'int&&, int&&)',
  'f57452b399b38f6de2bf9b452e25829f9b37a3ef2c783b80a87c884d2a22b80b'),
 (34505616,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> const&, int, '
  'int&&, int&&)',
  'd48ac53327348609697a4a6593aed7d19a00bfff514eb002c691a55f71a87758'),
 (34505664,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  'f845c681d586b1747a7ef50ff6e12d05359e9200b67d2b8f0a060216e7c0aa90'),
 (34505712,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> const&)',
  '9edaab095db829ce45ed51319d3bb74c4a7a8814ebe823d651c17d7ef93c0a7d'),
 (34505760,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resource> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resource> const&, int, '
  'int&&, int&&)',
  '7d0acf6ad0030647e2bec720ff97ac04ea14594dab79bbd7985537bac418ff97'),
 (34505808,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatWorkObjectType> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatWorkObjectType> '
  'const&, int, int&&, int&&)',
  '4a1d320026bd77544c5eb6464822fba57bd02afdeb86b71e4b4cb1ab8a072e8a'),
 (34505856,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Policy> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Policy> const&, int, int&&, '
  'int&&)',
  '75f9a48d9cc04a239afd4cc3515e4b2167e407e9d51be3bc444f184ec82888e2'),
 (34505904,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Governor> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Governor> const&, int, '
  'int&&, int&&)',
  '17880c73e040eb2a18cfbc74947ae009743f1a1c053fb3bf082605d9a44920d5'),
 (34505952,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Government> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Government> const&, int, '
  'int&&, int&&)',
  'f3a0278f1099b0b131d34616a4ce4c825028700dc12064484fdf42681af0544a'),
 (34506000,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Project> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Project> const&, int, '
  'int&&, int&&)',
  'a99b1b7d3ced8d822395cbb4bd9976243d8ccd5d283f6653e89bd1d2fc732f01'),
 (34506048,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonClass> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GreatPersonClass> const&, '
  'int, int&&, int&&)',
  '32b85a2e10f7cb646271a9f1f0f01de7d6ead07013054501fb4697088ae643da'),
 (34506096,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation> const&, int, '
  'int&&, int&&)',
  'b659a0a0524036e5e077c9d8aab9745f71dd42b347b8f5b1278cbde1b05bd21b'),
 (34506144,
  45,
  'int '
  'Serialization::ReadHashToIndexVersioned<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitPromotionClass> const&, '
  'int, int&&, int&&)',
  '8091556da083c71a30c3779d34b57ae2348eb98a58c3c541ce9ee06df6087c5c'),
 (34506192,
  30,
  'void Serialization::EnumFromStream<GameCore::ReligionTypes>(Serialization::IStream&, '
  'GameCore::ReligionTypes&)',
  '05282117b7d1592079ef112127dc4d29b42d63811255e76a929f31519fe4a442'),
 (34506224,
  34,
  'Serialization::IStream& Serialization::ReadVersioned<GameCore::Player::ComponentID, '
  'GameCore::Player::ComponentID>(Serialization::IStream&, GameCore::Player::ComponentID&, int, int&&, '
  'GameCore::Player::ComponentID&&)',
  '326beb2673b34d4b8fed34479e9d113a98e2a46d692e18b36c09d07c48acf535'),
 (34507728,
  19,
  'GameCore::AI::CollectionIndexData::CollectionIndexData()',
  'fb450f1646048d7c76ef02d8fcb9a3895c65b5fa60dd87f37c5e77265dbc916d'),
 (34512432,
  14,
  'Serialization::OStream::WriteObject(float const&)',
  '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'),
 (34883984,
  14,
  'Serialization::IStream& Serialization::IStream::operator>><short>(short&)',
  'fcb9926377812eb599dd6355ba5034296d824db7b711b95bf20e7d6805cf1b15'),
 (34884432,
  14,
  'Serialization::OStream& Serialization::OStream::operator<< <short>(short const&)',
  'b3a4ba0fe53a97a6f965db0493686a4226c6c361fae059c0f8d8b9ad2ae39fc0'),
 (35023648,
  5,
  'GameCore::Player::ComponentID::GetType() const',
  '95ee76ad5ef3f7beb741691cc3bd97bc5001575b0174515a9c173973dbfcba98'),
 (35592160,
  16,
  'GameCore::Definitions::GetResourceDefinitions()',
  '52a02d1fa4fec90cdc9212584a7305450af2c168f7b9839abafa2548916281e3'),
 (35731152,
  16,
  'GameCore::Definitions::GetBeliefDefinitions()',
  '3571d14d6a18c1fddb1c8b6de412730b45cc3cd9f646c61cf5c1e546714403a8'),
 (35731248,
  16,
  'GameCore::Definitions::GetBuildingDefinitions()',
  'd2851eafc27b252aa8bd74a7eb2b970b3e1d7e86b5136eb91d59256f6c6bdb3c'),
 (35731536,
  16,
  'GameCore::Definitions::GetCivicDefinitions()',
  '21d827e1b18ce1beb4961b2ac50ad247dbf0e2ad65fce7a36d2398946ffc8d96'),
 (35731856,
  16,
  'GameCore::Definitions::GetDifficultyDefinitions()',
  '17c0a12675d3b30b1f744519a4e850397a3703c1d290094ce22f7dc80da7ab5c'),
 (35732080,
  16,
  'GameCore::Definitions::GetDistrictDefinitions()',
  '76069daea712b3cc371592979e8763a1242553529523d5162fe6887ed2e45320'),
 (35732464,
  16,
  'GameCore::Definitions::GetFeatureDefinitions()',
  '645198b6de478f971e361793fcc0a551924cfd4946c0ad862e75c4a6e1242597'),
 (35732720,
  16,
  'GameCore::Definitions::GetGovernmentDefinitions()',
  '95b7ec070eb4981e30866ec36b79de752cdfddf07ddd75c40a8d88829556334c'),
 (35732784,
  16,
  'GameCore::Definitions::GetGovernmentSlotDefinitions()',
  '63a1354854ad39f2d7b019ebe2f6325ea264633e16bf3474d9a5ab233f2238a0'),
 (35732816,
  16,
  'GameCore::Definitions::GetGovernorDefinitions()',
  'b680f731edd8835d65717998b214c74416b8cc21e7f205b0b5a4a1dc1c908f39'),
 (35732976,
  16,
  'GameCore::Definitions::GetGreatPersonClassDefinitions()',
  '0a6c18917fd2dc9988dc8f248f85f72e2e92776310b97178aac414c6fb91e4cf'),
 (35733104,
  16,
  'GameCore::Definitions::GetGreatWorkObjectTypeDefinitions()',
  'a0b4005a870170fa9dd82a638d966a5517956a3f2ebd4b12764e09e19196c0e8'),
 (35733216,
  16,
  'GameCore::Definitions::GetImprovementDefinitions()',
  'e5dcaa6a4c7c61621d62bae40c50d047135862a38dc6b9ca106920b44c38c0b6'),
 (35733616,
  16,
  'GameCore::Definitions::GetMinorCivBonusDefinitions()',
  '7d1bb1ab18ce56823d5c30f1e0053213a7571e90a8c5d493a850b265fe6232f2'),
 (35734032,
  16,
  'GameCore::Definitions::GetPolicyDefinitions()',
  'b350e20390066f99aaef504608ef4d85090e6036f6f77c912d2ac3ba62ece73a'),
 (35734112,
  16,
  'GameCore::Definitions::GetProjectDefinitions()',
  '80b5cb4f9704c13ee0f21892c1f7caf955b6d63d1d21622d620b0dc30de8826a'),
 (35735280,
  16,
  'GameCore::Definitions::GetTechnologyDefinitions()',
  'e5f6b9ccc77ab304df2b37bd3addefa140b79bc50eacb658aecabeef2814251e'),
 (35735504,
  16,
  'GameCore::Definitions::GetUnitDefinitions()',
  '50972c441f7d9688de53a6ce66c2ea294e57945c392776946ce3ca7419bc4c65'),
 (35735696,
  16,
  'GameCore::Definitions::GetUnitOperationDefinitions()',
  '81ad939829e9d0aef09dc6a8231f2738c147082b41f1061727f64f0cc3dbb701'),
 (35735744,
  16,
  'GameCore::Definitions::GetUnitPromotionClassDefinitions()',
  '38fbd5dcad128f86109c71156097e6e678cc15d6148163d2a3267b5aaebe9494'),
 (35735920,
  16,
  'GameCore::Definitions::GetYieldDefinitions()',
  'b8ca8e6827d817397e9b8e220661a4b605b3b138aa25525476db4f02761e9218'),
 (40601504,
  21,
  'Serialization::IStream& operator>><GameCore::AI::GameModifierAnalysis, '
  'GameCore::AI::BehaviorTree::BehaviorTree>(Serialization::IStream&, '
  'FAutoVariable<GameCore::AI::GameModifierAnalysis, GameCore::AI::BehaviorTree::BehaviorTree>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (40601824,
  21,
  'Serialization::OStream& operator<< <GameCore::AI::GameModifierAnalysis, '
  'GameCore::AI::BehaviorTree::BehaviorTree>(Serialization::OStream&, '
  'FAutoVariable<GameCore::AI::GameModifierAnalysis, GameCore::AI::BehaviorTree::BehaviorTree> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (42392576,
  57,
  'GameCore::City::CManager::GetCity(GameCore::Player::ComponentID const&)',
  '5b7fb30d1c1ed64fa169933871cb06809ecf1f1b38a261a78bea94956b8fb281'),
 (42393776,
  93,
  'GameCore::City::CManager::GetDistrict(GameCore::Player::ComponentID const&)',
  'c1691db37b45fa3441151d0fc7494058a260d898fcba8ac052721df8728f27f2'),
 (43038880,
  16,
  'GameCore::Context::Globals::GetMap()',
  'f9cc044c211daf36f0dded8b1f757467b802002036f72bcadb87134ff66be7a8'),
 (43039024,
  22,
  'GameCore::Context::Globals::GetPlayer(GameCore::PlayerTypes)',
  '4f70fef1af78b8dea71c792f71f54727722e3e9f4509f9e764e536223739daaf'),
 (43111344,
  53,
  'GameCore::District::Instance::GetComponentID() const',
  '375fcfbe3325e22c56a6e49fa6540a14e874f5b423b91596f339746d9102dd58'),
 (45286528,
  7,
  'GameCore::Player::Instance::IsInitialized() const',
  'e1f84be58445280e096e10dd36a7c465c90745a4aaf7dcca8ce6a319e21c5d56'),
 (45429344,
  8,
  'GameCore::Player::Manager::GetInstance()',
  '3767e99c9e2a663768e78a24c0669349430acf05be01ce1ee8f273e1f62679e7'),
 (46225424,
  93,
  'GameCore::Unit::CManager::GetUnit(GameCore::Player::ComponentID const&)',
  '39ba0dbaa5365853cdddb366d64b750bb340c938b431d9c10fef5c5a57e29db9'),
 (46771104,
  33,
  'Serialization::operator<<(Serialization::OStream&, GameCore::PlayerTypes const&)',
  'e3aa2b85edbfcad11d2fe4b8d720bad7d662e88d8461f7994ba25444ae60c6d1'),
 (46771152,
  38,
  'Serialization::operator>>(Serialization::IStream&, GameCore::PlayerTypes&)',
  '2cc19dccb1ce5ba3727486d694c79a0b5c90c1d7658cf975ce07ade2572a6ee7'),
 (46771296,
  95,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::ComponentID const&)',
  'bd29bea4faf93d9428367ceceb01e636f498a5a209f6d011e3a69664a921fdff'),
 (46771392,
  76,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::ComponentID&)',
  '7021044d9575aaff00e08945164f5c91ba17cd8de1eedafd8c6781e3c69060be'),
 (46771472,
  95,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::CityID const&)',
  'a0462572e6b0309d6de60c35fd14f41c3cb054c915751ab1103d279b6f0d981c'),
 (46771648,
  95,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::UnitID const&)',
  'fac47f5edb84475fdb01bd75dd403a326ce14ed2f4062d541ba6f2495a36be32'),
 (46772480,
  11,
  'GameCore::Player::ComponentID::Set(GameCore::PlayerTypes, unsigned int, '
  'GameCore::Player::ComponentID::Type)',
  'cb4c434e7692669068d42f77251a91039bc80797654608f3bdbd62f534f7478f')]
BINDINGS = [(34485885, 32898000, 62729424, 33341744, 232),
 (34485892, 32130800, 62345824, 35731152, 232),
 (34485905, 31595760, 62078304, 34504912, 232),
 (34485913, 31965776, 62263312, 35731248, 232),
 (34485926, 31839904, 62200376, 33611424, 232),
 (34485934, 32648336, 62604592, 35731536, 232),
 (34485947, 32502480, 62531664, 33612608, 232),
 (34485955, 32651264, 62606056, 35732080, 232),
 (34485968, 32508400, 62534624, 33563840, 232),
 (34485976, 32099232, 62330040, 35732464, 232),
 (34485989, 32326832, 62443840, 33431504, 232),
 (34485997, 32102032, 62331440, 35731856, 232),
 (34486010, 31870368, 62215608, 34504960, 232),
 (34486018, 32462304, 62511576, 35733216, 232),
 (34486031, 32329312, 62445080, 33773392, 232),
 (34486039, 31596256, 62078552, 35733616, 232),
 (34486052, 32552672, 62556760, 34505008, 232),
 (34486060, 33010160, 62785504, 35735280, 232),
 (34486073, 31782336, 62171592, 33612288, 232),
 (34486081, 31534576, 62047712, 35735504, 232),
 (34486094, 32642560, 62601704, 33564496, 232),
 (34486102, 31534576, 62047712, 35735504, 232),
 (34486115, 32642560, 62601704, 33564496, 232),
 (34486123, 32894512, 62727680, 35735920, 232),
 (34486136, 33094864, 62827856, 34505056, 232),
 (34486144, 31768432, 62164640, 35592160, 232),
 (34486157, 32150192, 62355520, 33431456, 232),
 (34486165, 31925008, 62242928, 35733104, 232),
 (34486178, 33246096, 62903472, 33772688, 232),
 (34486186, 32069808, 62315328, 35734032, 232),
 (34486199, 31781136, 62170992, 33773104, 232),
 (34486207, 32859488, 62710168, 35732816, 232),
 (34486220, 32424816, 62492832, 34505104, 232),
 (34486228, 31714112, 62137480, 35732720, 232),
 (34486241, 32937680, 62749264, 34505152, 232),
 (34486249, 31936240, 62248544, 35732784, 232),
 (34486262, 31982208, 62271528, 33773152, 232),
 (34486270, 33187680, 62874264, 35734112, 232),
 (34486283, 31714304, 62137576, 34505200, 232),
 (34486291, 33074128, 62817488, 35732976, 232),
 (34486304, 32833344, 62697096, 34505248, 232),
 (34486312, 32242656, 62401752, 35735696, 232),
 (34486325, 32437840, 62499344, 34505296, 232),
 (34486333, 32635120, 62597984, 35735744, 232),
 (34486346, 32178624, 62369736, 34505344, 232),
 (34486358, 32150288, 62355568, 33566912, 232),
 (34486370, 32870832, 62715840, 34505392, 232),
 (34486382, 32898000, 62729424, 33341744, 232),
 (34486394, 32786160, 62673504, 46771296, 232),
 (34486409, 32898000, 62729424, 33341744, 232),
 (34486424, 32898000, 62729424, 33341744, 232),
 (34486439, 32890016, 62725432, 34505424, 232),
 (34486454, 32898000, 62729424, 33341744, 232),
 (34486469, 32898000, 62729424, 33341744, 232),
 (34486484, 32898000, 62729424, 33341744, 232),
 (34486499, 32783568, 62672208, 33514224, 232),
 (34486514, 31462048, 62011448, 33431440, 232),
 (34486529, 31462048, 62011448, 33431440, 232),
 (34486544, 32898000, 62729424, 33341744, 232),
 (34486564, 31462048, 62011448, 33431440, 232),
 (34486586, 33235888, 62898368, 33432064, 232),
 (34486597, 32190240, 62375544, 46771472, 232),
 (34486617, 31462048, 62011448, 33431440, 232),
 (34486639, 32182384, 62371616, 43111344, 232),
 (34486653, 32162016, 62361432, 33525376, 232),
 (34486664, 32786160, 62673504, 46771296, 232),
 (34486687, 31462048, 62011448, 33431440, 232),
 (34486704, 33228816, 62894832, 33341760, 232),
 (34486718, 32035680, 62298264, 46771104, 232),
 (34486741, 31462048, 62011448, 33431440, 232),
 (34486772, 32898000, 62729424, 33341744, 232),
 (34486795, 31462048, 62011448, 33431440, 232),
 (34486823, 32599024, 62579936, 34505440, 232),
 (34486834, 31971632, 62266240, 46771648, 232),
 (34486887, 31938512, 62249680, 33341856, 232),
 (34486892, 32130800, 62345824, 35731152, 232),
 (34486903, 32542032, 62551440, 34505472, 232),
 (34486911, 31965776, 62263312, 35731248, 232),
 (34486922, 32326912, 62443880, 33611280, 232),
 (34486931, 32648336, 62604592, 35731536, 232),
 (34486942, 32503920, 62532384, 33612544, 232),
 (34486951, 32651264, 62606056, 35732080, 232),
 (34486962, 33050192, 62805520, 33563888, 232),
 (34486971, 32099232, 62330040, 35732464, 232),
 (34487012, 32166912, 62363880, 34505520, 232),
 (34487021, 32102032, 62331440, 35731856, 232),
 (34487062, 33241200, 62901024, 34505568, 232),
 (34487071, 32462304, 62511576, 35733216, 232),
 (34487082, 32216848, 62388848, 33773440, 232),
 (34487091, 31596256, 62078552, 35733616, 232),
 (34487132, 32240080, 62400464, 34505616, 232),
 (34487141, 33010160, 62785504, 35735280, 232),
 (34487152, 33060016, 62810432, 34505664, 232),
 (34487161, 31534576, 62047712, 35735504, 232),
 (34487172, 33035952, 62798400, 33564544, 232),
 (34487181, 31534576, 62047712, 35735504, 232),
 (34487192, 33035952, 62798400, 33564544, 232),
 (34487201, 32894512, 62727680, 35735920, 232),
 (34487212, 31978064, 62269456, 34505712, 232),
 (34487221, 31768432, 62164640, 35592160, 232),
 (34487262, 33088960, 62824904, 34505760, 232),
 (34487271, 31925008, 62242928, 35733104, 232),
 (34487312, 31976016, 62268432, 34505808, 232),
 (34487328, 32069808, 62315328, 35734032, 232),
 (34487369, 32153760, 62357304, 34505856, 232),
 (34487378, 32859488, 62710168, 35732816, 232),
 (34487419, 33145952, 62853400, 34505904, 232),
 (34487428, 31714112, 62137480, 35732720, 232),
 (34487469, 32913088, 62736968, 34505952, 232),
 (34487478, 31936240, 62248544, 35732784, 232),
 (34487519, 33236320, 62898584, 33773248, 232),
 (34487528, 33187680, 62874264, 35734112, 232),
 (34487569, 32955040, 62757944, 34506000, 232),
 (34487578, 33074128, 62817488, 35732976, 232),
 (34487619, 31472160, 62016504, 34506048, 232),
 (34487628, 32242656, 62401752, 35735696, 232),
 (34487669, 33249536, 62905192, 34506096, 232),
 (34487678, 32635120, 62597984, 35735744, 232),
 (34487719, 31492608, 62026728, 34506144, 232),
 (34487735, 31822560, 62191704, 33447072, 232),
 (34487747, 32819264, 62690056, 34506192, 232),
 (34487759, 31938512, 62249680, 33341856, 232),
 (34487788, 32266624, 62413736, 33432048, 232),
 (34487809, 32923120, 62741984, 34506224, 232),
 (34487824, 31938512, 62249680, 33341856, 232),
 (34487839, 31938512, 62249680, 33341856, 232),
 (34487854, 31933200, 62247024, 33592304, 232),
 (34487869, 31938512, 62249680, 33341856, 232),
 (34487884, 31938512, 62249680, 33341856, 232),
 (34487929, 33253312, 62907080, 33563776, 232),
 (34487944, 32334656, 62447752, 33513968, 232),
 (34487986, 31828752, 62194800, 33447552, 232),
 (34488001, 32766320, 62663584, 33431552, 232),
 (34488016, 31938512, 62249680, 33341856, 232),
 (34488029, 32766320, 62663584, 33431552, 232),
 (34488049, 32266624, 62413736, 33432048, 232),
 (34488060, 33171248, 62866048, 46771392, 232),
 (34488065, 31987088, 62273968, 45429344, 232),
 (34488076, 32732160, 62646504, 33450864, 232),
 (34488086, 32870944, 62715896, 33574848, 232),
 (34488095, 31987088, 62273968, 45429344, 232),
 (34488108, 32732160, 62646504, 33450864, 232),
 (34488118, 32078528, 62319688, 33341872, 232),
 (34488126, 31596704, 62078776, 45286528, 232),
 (34488140, 32579184, 62570016, 42392576, 232),
 (34488157, 32766320, 62663584, 33431552, 232),
 (34488177, 32266624, 62413736, 33432048, 232),
 (34488188, 33171248, 62866048, 46771392, 232),
 (34488196, 32194400, 62377624, 42393776, 232),
 (34488213, 32766320, 62663584, 33431552, 232),
 (34488233, 33204544, 62882696, 46771152, 232),
 (34488242, 33050512, 62805680, 43039024, 232),
 (34488262, 32766320, 62663584, 33431552, 232),
 (34488282, 31938512, 62249680, 33341856, 232),
 (34488287, 31945264, 62253056, 43038880, 232),
 (34488320, 32766320, 62663584, 33431552, 232),
 (34488340, 32266624, 62413736, 33432048, 232),
 (34488351, 33171248, 62866048, 46771392, 232),
 (34488359, 31671072, 62115960, 46225424, 232),
 (46771309, 32732160, 62646504, 33450864, 232),
 (46771327, 32522480, 62541664, 34884432, 232),
 (46771335, 32847680, 62704264, 35023648, 232),
 (46771350, 32522480, 62541664, 34884432, 232),
 (46771358, 32984848, 62772848, 33362048, 232),
 (46771375, 31588976, 62074912, 33341776, 232),
 (46771407, 32697920, 62629384, 34883984, 232),
 (46771418, 32697920, 62629384, 34883984, 232),
 (46771431, 31815744, 62188296, 33341888, 232),
 (46771452, 31669824, 62115336, 46772480, 232),
 (46771485, 32732160, 62646504, 33450864, 232),
 (46771503, 32522480, 62541664, 34884432, 232),
 (46771511, 32847680, 62704264, 35023648, 232),
 (46771526, 32522480, 62541664, 34884432, 232),
 (46771534, 32984848, 62772848, 33362048, 232),
 (46771551, 31588976, 62074912, 33341776, 232),
 (46771661, 32732160, 62646504, 33450864, 232),
 (46771679, 32522480, 62541664, 34884432, 232),
 (46771687, 32847680, 62704264, 35023648, 232),
 (46771702, 32522480, 62541664, 34884432, 232),
 (46771710, 32984848, 62772848, 33362048, 232),
 (46771727, 31588976, 62074912, 33341776, 232),
 (33513972, 32915600, 62738224, 33513984, 232),
 (33514001, 32609424, 62585136, 33347392, 232),
 (33514022, 31815744, 62188296, 33341888, 232),
 (33514039, 31603152, 62082000, 33514112, 232),
 (33514067, 32715008, 62637928, 33514144, 232),
 (33514075, 32481760, 62521304, 33514192, 232),
 (33514086, 31815744, 62188296, 33341888, 232),
 (33514244, 31884208, 62222528, 33347536, 232),
 (33514255, 31487632, 62024240, 33514400, 232),
 (33514274, 32685120, 62622984, 33514432, 232),
 (33514295, 31541328, 62051088, 33514320, 232),
 (33514450, 32866768, 62713808, 33347552, 232),
 (33514467, 31588976, 62074912, 33341776, 232),
 (33514483, 31588976, 62074912, 33341776, 233),
 (33456575, 31471216, 62016032, 33348384, 232),
 (33592351, 31471216, 62016032, 33348384, 232),
 (34503894, 32266624, 62413736, 33432048, 232),
 (34503971, 33138160, 62849504, 33511168, 232),
 (34504023, 32848672, 62704760, 34507728, 233),
 (33566924, 32898000, 62729424, 33341744, 232),
 (34505404, 32898000, 62729424, 33341744, 232),
 (33447085, 31938512, 62249680, 33341856, 232),
 (34506205, 31938512, 62249680, 33341856, 232)]
DWARF_SHA = 'c6b87e6ca92ae1544908502974cf4d9fec0ac846b5cc3c27bc8bc8440ad87ba3'

HASH_FIELDS = [
    ('Belief', 0, 'Belief', None), ('Building', 4, 'Building', None),
    ('Civic', 24, 'Civic', None), ('District', 28, 'District', None),
    ('Feature', 32, 'Feature', 14), ('Handicap', 52, 'Difficulty', 3),
    ('Improvement', 56, 'Improvement', None), ('MinorCivType', 60, 'MinorCivBonus', 13),
    ('Technology', 80, 'Technology', None), ('Unit1', 92, 'Unit', None),
    ('Unit2', 96, 'Unit', None), ('Yield', 100, 'Yield', None),
    ('Resource', 76, 'Resource', 6), ('GreatWorkObject', 48, 'GreatWorkObjectType', 7),
    ('Policy', 68, 'Policy', 8), ('Governor', 36, 'Governor', 17),
    ('Government', 40, 'Government', 9), ('GovernmentSlot', 44, 'GovernmentSlot', 9),
    ('Project', 104, 'Project', 11), ('GreatPersonClass', 108, 'GreatPersonClass', 12),
    ('UnitOperation', 84, 'UnitOperation', 15), ('UnitPromotionClass', 88, 'UnitPromotionClass', 16),
]
FIELDS = [dict(name='version', wire='i32-le', current=17)] + [
    dict(name=n, object_offset=o, wire='definition-hash32-le', collection=c,
         reader_version_threshold=v, version_default=-1 if v else None)
    for n, o, c, v in HASH_FIELDS
] + [
    dict(name=n, object_offset=o, wire=w) for n, o, w in [
        ('Opponent', 64, 'i32-le'), ('Religion', 72, 'i32-le'), ('Count', 8, 'i32-le'),
        ('CityId', 16, 'component-id'), ('StartTurn', 180, 'i32-le'),
        ('TimeHorizon', 184, 'i32-le'), ('DiscountRate', 188, 'f32-raw-le'),
        ('ExpectedBonusValue', 192, 'i32-le'), ('ExpectedTurnsToMeet', 196, 'i32-le'),
        ('AnalysisDepth', 200, 'i32-le'), ('UnitList', 152, 'u32-count + ordered u32-le items'),
        ('BestImprovement', 176, 'raw-u8'), ('CollectionIndex.FoundItem', 204, 'raw-u8'),
        ('CollectionIndex.IteratorIndex', 208, 'i32-le'),
    ]
] + [dict(name=n, object_offset=o, wire='presence-u8 + payload iff nonzero', payload=w)
     for n, o, w in [('City', 112, 'component-id'), ('DistrictInstance', 120, 'component-id'),
                     ('Player', 128, 'i32-le'), ('Plot', 136, 'i32-le'), ('Unit', 144, 'component-id')]]


def relocations(game):
    raw = game.raw
    start = struct.unpack_from('<Q', raw, 40)[0]
    width, count = struct.unpack_from('<HH', raw, 58)
    sections = [struct.unpack_from('<IIQQQQIIQQ', raw, start + i * width) for i in range(count)]
    result = {}
    for section in sections:
        if section[1] != 4:
            continue
        symbols = sections[section[6]]
        for pos in range(section[4], section[4] + section[5], section[9]):
            target, info, addend = struct.unpack_from('<QQq', raw, pos)
            kind = info & 0xffffffff
            if kind == 8:
                result[target] = addend
            elif kind in (1, 7):
                symbol = symbols[4] + (info >> 32) * symbols[9]
                result[target] = struct.unpack_from('<Q', raw, symbol + 8)[0] + addend
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    path = args.assets / 'libGameCore_XP2.so'
    game = Elf(path, GAME_SHA)
    for address, size, name, digest in METHODS:
        if hashlib.sha256(game.read(address, size)).hexdigest() != digest:
            raise ValueError(f'method pin mismatch: {name}')
    pointers = relocations(game)
    for site, plt, slot, body, opcode in BINDINGS:
        raw = game.read(site, 5)
        if raw[0] != opcode or site + 5 + struct.unpack_from('<i', raw, 1)[0] != plt:
            raise ValueError(f'call/tail edge mismatch: {site:#x}')
        stub = game.read(plt, 6)
        if stub[:2] != b'\xff\x25' or plt + 6 + struct.unpack_from('<i', stub, 2)[0] != slot:
            raise ValueError(f'PLT slot mismatch: {plt:#x}')
        if pointers.get(slot) != body:
            raise ValueError(f'PLT body mismatch: {slot:#x}')
    if game.read(0x20e3670, 8) != bytes.fromhex('c744240c11000000'):
        raise ValueError('current version mismatch')
    dwarf = subprocess.check_output(['gdb', '-nx', '-batch', str(path), '-ex',
                                     "ptype /o 'GameCore::AI::GameModifierAnalysis'"])
    if hashlib.sha256(dwarf).hexdigest() != DWARF_SHA:
        raise ValueError('matching ELF DWARF layout mismatch')
    report = dict(
        schema='c6-linux-game-modifier-analysis-contract-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA,
        method='Read-only exact ELF method/call/PLT-slot/version and matching DWARF audit',
        writer=dict(address='0x20e3660', size=0x3e5, version=17),
        reader=dict(address='0x20e3a50', size=0x5f3),
        methods=[dict(address=hex(a), size=s, name=n, sha256=h) for a, s, n, h in METHODS],
        bindings=[dict(site=hex(a), plt=hex(p), slot=hex(s), body=hex(b), opcode=hex(o))
                  for a, p, s, b, o in BINDINGS],
        dwarf_sha256=DWARF_SHA, object_size=224, fields=FIELDS,
        component_id=dict(bytes=8, fields=['owner:i16-le', 'type:i16-le', 'id:u32-le']),
        v17_size='151 + 4*UnitList_count + 8*(City_present+DistrictInstance_present+Unit_present) + 4*(Player_present+Plot_present)',
        bool_semantics='ReadObject/WriteObject transfer one raw byte; presence writer generates 0/1; reader tests byte !=0',
        float_semantics='DiscountRate transfers four raw IEEE754 bytes, with no conversion or canonicalization',
        q8_fields=[], unit_list_semantics='u32 count; preserve unsigned item bits and order; native read clears vector before reserve/push',
        definition_semantics='WriteIndexToHash and ReadHashToIndex resolve through named definition collections; wire hashes must not be treated as native indices',
        version_limits='Only v17 grammar specified here; historical thresholds recorded as source leads; Policy read is specially skipped when version==9',
        not_serialized=['CollectionIndex.IteratorIndex2', 'CollectionIndex.IteratorIndex3',
                        'pointer addresses', 'allocation/capacity', 'object padding'],
        read_effects={
            'absent_optional': 'leave existing destination pointer unchanged',
            'City': 'read ComponentID; only assign GetCity result if owner IsValid and owner player IsInitialized; otherwise leave destination unchanged',
            'DistrictInstance': 'assign City::CManager::GetDistrict(ComponentID)',
            'Player': 'assign Context::Globals::GetPlayer(PlayerTypes)',
            'Plot': 'assign Globals::GetMap virtual slot +0x48 using signed plot index',
            'Unit': 'assign Unit::CManager::GetUnit(ComponentID)',
            'initialization': 'constructor separately initializes pointers to null; reader does not call constructor or reset'},
        fixtures=[], fixture_gap='No native execution or captured owner cuts in this research-only audit',
        original_execution=False, native_load_effects_reproduced=False, hook_counts_inferred=False,
        full_player=False, world_import=False, gameplay_state=False, actions=False, turns=False,
        planning_only_exclusion_proven=False, unknown_save_bytes_masked=False,
        recorded_raw_save_load_difference_bytes=89198, private_writes=False,
        audit_source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        attribution='Independent analysis of supplied pinned Linux ELF symbols, bytes and matching DWARF; no proprietary disassembly archived')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS), bindings=len(BINDINGS), fields=len(FIELDS))))


if __name__ == '__main__':
    main()
