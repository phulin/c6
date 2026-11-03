#!/usr/bin/env python3
"""SHA-gated typed current Linux GlobalAi wire audit. No original execution.

Native owner extents are comparison-only, never production parser inputs.
"""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from player_wmds_audit import Elf, GAME_SHA, FIXTURES
ROOT = Path(__file__).resolve().parents[2]
def sha(data): return hashlib.sha256(data).hexdigest()
METHODS = [[33517824, 67, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions>(Serialization::IStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void>&)', '0e9219a8584cc587034d055d9e56c635518a6da8fc6d5680c65426d3bf5c1deb'], [33518720, 56, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions>(Serialization::OStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void> const&)', '954fa84b4e80371c8ba4c831bfa45daa3988189b5b98d6b58f04557516908166'], [33518784, 141, 'bool Serialization::WriteIndexHashArray<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> >(Serialization::OStream&, FixedPointT<8u> const*, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield> const&)', 'ae762e77e970a800a6f7b28cc5b1ee80eca91896a4089b386f53a7b2c85cea6e'], [33521776, 67, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::PseudoYield>, &GameCore::Definitions::GetPseudoYieldDefinitions>(Serialization::IStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::PseudoYield>, &GameCore::Definitions::GetPseudoYieldDefinitions, void>&)', 'd366be49d5cc7734b9f0eb7d5391f42ee77ac13335280fb4d7680bfc049a8a9c'], [33522704, 56, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::PseudoYield>, &GameCore::Definitions::GetPseudoYieldDefinitions>(Serialization::OStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::PseudoYield>, &GameCore::Definitions::GetPseudoYieldDefinitions, void> const&)', 'c797c5459da81cd330622f4b8ce20e204a4944dc4cdd3eb30c8a8198bafdbba6'], [33564592, 91, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u> >(Serialization::OStream&, eastl::vector<FixedPointT<8u>, eastl::allocator> const&)', '200dd9bec38b0ae9a68b5cb2cd9e4326dafffa46deedc06bec01c0da57a54fda'], [33564688, 14, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u> >(Serialization::IStream&, eastl::vector<FixedPointT<8u>, eastl::allocator>&)', 'baa464afec68870e781fc4ceb5c699ba2f94c7905e18531f6470020a56821544'], [33653328, 66, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Control::Handle const&)', '59ad235e32b730f4bf09f479a96e80ab43548c5f4ed03b1fe0154073e9f9bf0d'], [33653408, 58, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Control::Handle&)', 'a36d9e7c1cf7422f37e527ea91da51367bf4a6ba7c69900ba41d787a1c49d1b6'], [33671312, 50, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Control::Factory const&)', '0ae77b5aa4633ba016fd4f732fc14c9fa296e911cc31e2f578e89b3b08d18bbe'], [33671376, 42, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Control::Factory&)', 'dd2e4753457e25366487841a8f806a25cd2341a5153c09e211c6ac25b8155355'], [33995008, 56, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::DiplomaticAction>, &GameCore::Definitions::GetDiplomaticActionDefinitions>(Serialization::OStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::DiplomaticAction>, &GameCore::Definitions::GetDiplomaticActionDefinitions, void> const&)', '58abf81c72ffb4eb93a95256304d5599e747c13bf5525c50a81d729be1c22e7f'], [34016320, 67, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::DiplomaticAction>, &GameCore::Definitions::GetDiplomaticActionDefinitions>(Serialization::IStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::DiplomaticAction>, &GameCore::Definitions::GetDiplomaticActionDefinitions, void>&)', 'bf26d31374708c27fd7b2eff85857ff38d540dcd77ecdef00d5b5d587dbc2801'], [34272864, 82, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::EspionageCityAnalysis const&)', '6460ea48bb05930322406ec2a4f56c1aac5f056d5059acdaac553a0f6a8ce674'], [34272960, 206, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::EspionageCityAnalysis&)', '36408869a1bb0525262e1a793b53e4875b3f2864deac9cf373c163d0a986648f'], [34273168, 173, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Espionage const&)', '335840503a98b838ce609ca7f00d3fc19f2f642b754a35b9aeb93a129a2a8142'], [34273344, 330, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Espionage&)', '05a904050ee98316431f15ab220a9780e5e2d00883f4c93dda2e29a880d93498'], [34290160, 83, 'bool Serialization::WriteArray<FixedPointT<8u> >(Serialization::OStream&, FixedPointT<8u> const*, unsigned long)', '68bd8def1b41479a779454b9c03c172ef03231d4a0589eb6302197924166a4a3'], [34312704, 121, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::EspionageManager const&)', '7228e7436092c69fee5c1993c53fd7238ca5336a6b40be39755f0e6df050099b'], [34312832, 326, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::EspionageManager&)', 'f91799e33f130271039a5fc5b93c276379c70084a49fb445782e79b93950e403'], [34554176, 50, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::MayhemTracker const&)', '8c7e925d1f966fe768305dada063786e0da9ecb5eaba4c2afc294baf082362f6'], [34554240, 42, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::MayhemTracker&)', 'a1f852b63072cb687901236b626d7f0f36cf31f958839d718946227d0afa6404'], [34554288, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::GlobalAi const&)', 'a55d6f37fd3a96fbfeb0f0c55b3dc0b65d4e54e69a4610c590275b7a33b708fb'], [34554320, 222, 'GameCore::AI::GlobalAi::Write(Serialization::OStream&) const', '8253c5ed5c116cd9aa02b0de1748cc3461cfe5d2ecb913ead5717aea839f6283'], [34554544, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::GlobalAi&)', '452bcbde734d5320080963e22ec8c982076f9fa0b65bfdc40fa353d26572f57d'], [34554576, 820, 'GameCore::AI::GlobalAi::Read(Serialization::IStream&)', '575a90c970c4fbed5e7835f1ceaecff6c6625cbdc93df38a5e6b0dfada0d266a'], [34922272, 67, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiScoutUse>, &GameCore::Definitions::GetAiScoutUseDefinitions>(Serialization::IStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiScoutUse>, &GameCore::Definitions::GetAiScoutUseDefinitions, void>&)', '188fbc8a626e8966ee8c556db3f24db51ef09814574b5ab09d0ceec4bf2c8ecd'], [34958224, 56, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiScoutUse>, &GameCore::Definitions::GetAiScoutUseDefinitions>(Serialization::OStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiScoutUse>, &GameCore::Definitions::GetAiScoutUseDefinitions, void> const&)', 'd30dcbf902867146fd2aaccc9a8070d7aaf795ea5ca5bb164ed669f31a81a41e'], [35067520, 129, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::BeliefAnalysis const&)', '53c75b4e6cf622bcb832419932133911dba5b8f2d8221505d74c60ba10f54885'], [35067664, 127, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::BeliefAnalysis&)', '0ad1489ff2ef13473c50d5ce40cd6842de4e6f2ef69746c84525a02c4719a15e'], [35068096, 137, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::CityReligionAnalysis const&)', 'dbb20507cdeb1a3ee4fb46511dab549f0cfc5a699343fefe4e563aff3eba3119'], [35068240, 237, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::CityReligionAnalysis&)', '92195563a6404710dc77eb01fdcbd004239246f30c6010aef652cf2c185f6523'], [35068480, 271, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Religious const&)', '13ec46a54aae7b25fe69436bd766f6673862b7e86dbf286d1ad869e90760a10a'], [35068752, 524, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Religious&)', '2112fa0d3dd5eefd00149280f6e6ec021f3ac301771d7d701b6e9041e68b8e11'], [35099408, 141, 'bool Serialization::WriteIndexHashArray<GameCore::AI::BeliefAnalysis, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Belief> >(Serialization::OStream&, GameCore::AI::BeliefAnalysis const*, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Belief> const&)', '787205f58039bfa067753bf9771c7d5790fcfb0df736d016923e6cbc533ff6de'], [35137536, 121, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::ReligiousManager const&)', '07281e2e1aa192d3c328e6d19a4a1899cdec72e45c0491242bdbf028599fbc5c'], [35137664, 322, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::ReligiousManager&)', 'aa99dde20ae1f0ceb3406d3fdabe924e1e1ef419d2a5d1d5756796d7a0e980dd'], [40656112, 14, 'Serialization::IStream& Serialization::operator>><GameCore::PlotCoord>(Serialization::IStream&, GameCore::AI::BehaviorTree::BlackboardVector<GameCore::PlotCoord>&)', 'd446b7f21de6942ba9f529e4260a968be35988848895493c4ac3263545d6d5a4'], [40656176, 14, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u> >(Serialization::IStream&, GameCore::AI::BehaviorTree::BlackboardVector<FixedPointT<8u> >&)', 'e12776b03a5a83bb445832b9dd36551855873228969de296f08fc22c25dec5be'], [40656304, 14, 'Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::BlackboardVector<GameCore::PlotCoord> const&)', '93909769961bd1a66dfeb861a59c3c365442395d18679f359537f691c01d421b'], [40656368, 14, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u> >(Serialization::OStream&, GameCore::AI::BehaviorTree::BlackboardVector<FixedPointT<8u> > const&)', '95f6de30077fae82128a5dc0ad7397cdc37bb321a527f6cd0f9b68a742fcfa69'], [40688544, 14, 'Serialization::IStream& Serialization::operator>><GameCore::PlotCoord>(Serialization::IStream&, GameCore::AI::BehaviorTree::BlackboardData<GameCore::PlotCoord>&)', '1e92a1a8f17621dfcc3579e137a3dcac9d1daea36df6b00f3fc36b70b648ec95'], [40689824, 14, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u> >(Serialization::IStream&, GameCore::AI::BehaviorTree::BlackboardData<FixedPointT<8u> >&)', 'c985d2707ea93c263e793f15e38b1be3a48d6804aabafe74d176379b41039ec1'], [40693984, 14, 'Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::BlackboardData<GameCore::PlotCoord> const&)', '5340b8ac7f27dca7ab3a8d588bbf6216dd1eca8ea161bac1de99a9b54655306c'], [40696320, 14, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u> >(Serialization::OStream&, GameCore::AI::BehaviorTree::BlackboardData<FixedPointT<8u> > const&)', 'd15944cd0d89739f7d466182f7eeaf5c902c1eab2fe86a1f414cdfe5d430bdbb'], [40820576, 20, 'Serialization::IStream& Serialization::operator>><GameCore::PlotCoord const>(Serialization::IStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const>&)', '2a207083788a0868fdb3091bc4f2faac7b2ada5d1d4191168f04453c45b6c386'], [40820896, 20, 'Serialization::IStream& Serialization::operator>><GameCore::PlotCoord>(Serialization::IStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord>&)', '523fd3048041b0a516529096a2c58eb2558814305c8a004c24e897b0d2e4b33c'], [40820960, 20, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u> const>(Serialization::IStream&, GameCore::AI::BehaviorTree::SharedData<FixedPointT<8u> const>&)', 'f31275538abdd9dfedb5ff5b7eb5ee889d0e01ed82cca4622e50716a087f4757'], [40822768, 20, 'Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)', '8f2e3375fe5b62c2a1b54e276ac735246a20b87ba543355f3dcc672fb6001227'], [40823056, 20, 'Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord> const&)', 'ebe727a063f96a592d8ff89e006a0a05705482ee6382a8f19d9eae164848d2a8'], [40823120, 20, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<FixedPointT<8u> const> const&)', 'eff603305e02c85536935ebcc2c251d7fcc2debfdbcc30766414d2bef4a206d7'], [44943296, 91, 'Serialization::OStream& Serialization::operator<< <long, Types::StaticArray<FixedPointT<8u>, 64ul> >(Serialization::OStream&, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator> const&)', 'eb308c6c5ed1a112f8c9ca6636051f416bddda66b96ca0d47054cf65d5964cf4'], [44943680, 14, 'Serialization::IStream& Serialization::operator>><long, Types::StaticArray<FixedPointT<8u>, 64ul> >(Serialization::IStream&, eastl::map<long, Types::StaticArray<FixedPointT<8u>, 64ul>, eastl::less<long>, eastl::allocator>&)', '16f06c4c3884ed9007a8620a3f4255676f006e1c8b5f24317bb6657057eaa2af'], [44947568, 41, 'Serialization::OStream& Serialization::operator<< <long, Types::StaticArray<FixedPointT<8u>, 64ul> >(Serialization::OStream&, eastl::pair<long, Types::StaticArray<FixedPointT<8u>, 64ul> > const&)', 'b096ccd07cb7df66f19b8105d2e6722f102660785511baf6ae0023253d6cb544'], [44947632, 57, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u>, 64ul>(Serialization::OStream&, Types::StaticArray<FixedPointT<8u>, 64ul> const&)', '1e8cf5135526b76bb870c41d7b0254cca4cb66b382893372fec26c117b7e4be5'], [44949248, 41, 'Serialization::IStream& Serialization::operator>><long, Types::StaticArray<FixedPointT<8u>, 64ul> >(Serialization::IStream&, eastl::pair<long, Types::StaticArray<FixedPointT<8u>, 64ul> >&)', '8f5099c6d9362f2771ef6469957e224f24ed181348e150c7708eda80b343bc8f'], [44949408, 57, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u>, 64ul>(Serialization::IStream&, Types::StaticArray<FixedPointT<8u>, 64ul>&)', 'b9d8a6dfabcd1c4570b7dd4a094e06bcfe63e339f9178bf88df136c1d3eff8ba'], [45314528, 14, 'Serialization::IStream& Serialization::operator>><GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void> >(Serialization::IStream&, eastl::vector<GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void>, eastl::allocator>&)', '2b34dacd8c64460e234b3973e26df552b03c424b1ea012bef7e3cff5f3d9c376'], [45315984, 91, 'Serialization::OStream& Serialization::operator<< <GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void> >(Serialization::OStream&, eastl::vector<GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void>, eastl::allocator> const&)', '047177ca20f8d2b2f4f96a8f642664c834fd44b6e2e2625bc41d34dad590d293'], [45583872, 67, 'Serialization::IStream& Serialization::operator>><FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resource>, &GameCore::Definitions::GetResourceDefinitions>(Serialization::IStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resource>, &GameCore::Definitions::GetResourceDefinitions, void>&)', 'd8534702ed3606db7e4986ab498146262a28b36e88732b0cde838a0dd6123a72'], [45583984, 56, 'Serialization::OStream& Serialization::operator<< <FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resource>, &GameCore::Definitions::GetResourceDefinitions>(Serialization::OStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resource>, &GameCore::Definitions::GetResourceDefinitions, void> const&)', 'c4d09befa9e8c8a47f7d466759932ee7d5f307bc3f8758c9f0aebd3f16a47788'], [45711280, 141, 'bool Serialization::WriteIndexHashArray<GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> >(Serialization::OStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void> const*, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> const&)', '3a5b91f4063a2d1bfe9d16a94e279daa5a6bbbdeb761ee4d5739ebb6d37ee61f'], [45711424, 141, 'bool Serialization::WriteIndexHashArray<GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Terrain> >(Serialization::OStream&, GameCore::IndexList<FixedPointT<8u>, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Yield>, &GameCore::Definitions::GetYieldDefinitions, void> const*, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Terrain> const&)', 'f0d87beba7da244ce452e035802f454972dedb5d8c103a340fb650b5bf873ec9'], [46771472, 95, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::CityID const&)', 'a0462572e6b0309d6de60c35fd14f41c3cb054c915751ab1103d279b6f0d981c'], [46771568, 76, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::CityID&)', '95c0f2a3adabb22a1ccc987faac9ec466d51c1c76898180eef6caa3a2d279e07'], [46772000, 41, 'Serialization::operator<<(Serialization::OStream&, GameCore::PlotCoord const&)', '09befdb0e6703290977db65b34b4a9d237076caa3199171f77138d55361c355d'], [46772048, 41, 'Serialization::operator>>(Serialization::IStream&, GameCore::PlotCoord&)', 'e3a5651a332e533a0ab691c102a6058de6b19f5da3cbd89404986c0e8deffda2'], [46772192, 42, 'Serialization::operator<<(Serialization::OStream&, FixedPointT<8u> const&)', 'bbd0553ff77e4be5a896850082327f68662e870164f4771d58a0e5c73c7d5704'], [46772240, 43, 'Serialization::operator>>(Serialization::IStream&, FixedPointT<8u>&)', '5d47fcdfb3868524b3d8da8a2fab95de165678efa05f23c615963da12ddf010d'], [47512336, 33, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Control::ControlPriority const&)', '777ee93239f8610f836cd9f3c9c65848ebaa5659917e6f6e8684fd194634207f'], [47512384, 38, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Control::ControlPriority&)', 'ce4eaf66311f2f1300fe5c9cedd30e38cb022b02153a9ce762aa725d9c4e6d7b'], [48142416, 91, 'Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, eastl::vector<GameCore::PlotCoord, eastl::allocator> const&)', '83d78418687fe2b355c017b9ee46d753ec56155556af69f38e0796fa88e7a8ab'], [48142528, 14, 'Serialization::IStream& Serialization::operator>><GameCore::PlotCoord>(Serialization::IStream&, eastl::vector<GameCore::PlotCoord, eastl::allocator>&)', '1c6a247b967fb11e6d6f321c2597b3bb073301a5a915cc13e2fa81fad50aa76a']]

HARNESS = r'''pub use c6_core::unit;
use c6_core::game_global_ai::{GlobalAi as Manager, Error};
fn main() {
    let input = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let mut pos = 0;
    let mut records = 0;
    while pos < input.len() {
        let size = u32::from_le_bytes(input[pos..pos+4].try_into().unwrap()) as usize;
        let items = u32::from_le_bytes(input[pos+4..pos+8].try_into().unwrap()) as usize;
        pos += 8;
        let nv = u32::from_le_bytes(input[pos..pos+4].try_into().unwrap()) as usize;
        let nc = u32::from_le_bytes(input[pos+4..pos+8].try_into().unwrap()) as usize;
        pos += 8;
        let sites:Vec<usize> = input[pos..pos+4*(nv+nc)].chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).collect();
        pos += 4*(nv+nc);
        let raw = &input[pos..pos+size]; pos += size;
        let limits = unit::Limits { max_bytes: size, max_items: items };
        let value = Manager::decode(raw, limits).unwrap();
        for offset in sites {
            let mut bad = raw.to_vec();
            bad[offset..offset+4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(Manager::decode(&bad, limits).is_err());
        }
        assert_eq!(value.encode(limits).unwrap(), raw);
        assert_eq!(value.encode_with_items(limits).unwrap(), (raw.to_vec(), items));
        assert_eq!(Manager::decode_prefix(raw, limits).unwrap(), (value.clone(), size));
        for end in (0..size.min(1024)).chain((0..1024).map(|i| i * size / 1024)).chain([size-1]) { assert!(Manager::decode(&raw[..end], limits).is_err()); }
        let short = unit::Limits { max_bytes: size-1, ..limits };
        assert_eq!(Manager::decode(raw, short), Err(Error::ResourceLimit));
        assert_eq!(value.encode(short), Err(Error::ResourceLimit));
        if items > 0 {
            let short = unit::Limits { max_items: items-1, ..limits };
            assert_eq!(Manager::decode(raw, short), Err(Error::ResourceLimit));
            assert_eq!(value.encode(short), Err(Error::ResourceLimit));
        }
        let mut suffix = raw.to_vec(); suffix.push(77);
        assert_eq!(Manager::decode(&suffix, limits), Err(Error::Trailing));
        assert_eq!(Manager::decode_prefix_with_items(&suffix, limits).unwrap(), (value, size, items));
        records += 1;
    }
    println!("{records}");
}
'''

def current_record(raw):
    pos, items = 0, 0
    reconstructed = bytearray()
    children = []
    versions, counts = [], []
    def take(fmt):
        nonlocal pos
        size = struct.calcsize('<'+fmt)
        values = struct.unpack_from('<'+fmt, raw, pos)
        reconstructed.extend(struct.pack('<'+fmt, *values)); pos += size
        return values[0] if len(values) == 1 else list(values)
    def version(expected):
        versions.append(pos)
        if take('I') != expected: raise ValueError('unsupported version')
    def vector(fn, minimum):
        nonlocal items
        counts.append(pos)
        n = take('I')
        if n > (len(raw)-pos)//minimum: raise ValueError('hostile count')
        items += n
        return [fn() for _ in range(n)]
    def city(): return take('hhI')
    def handle():
        version(1); return take('iI')
    def hash_q8(): return take('Ii')
    def esp_city():
        version(3)
        city_id=city(); values=vector(lambda:take('i'),4)
        if len(values)!=11:raise ValueError('current fixed act array length differs')
        return dict(city=city_id, values=values, value=take('i'))
    def espionage():
        begin=pos; version(4)
        value=dict(player=take('i'),priority=take('i'), cities=vector(esp_city,64),flag=take('B'),
            targets=vector(city,8), a=vector(lambda:take('i'),4), b=vector(lambda:take('i'),4), c=vector(lambda:take('i'),4))
        children.append(dict(owner='GameCore::AI::Espionage',start=begin,end=pos))
        return value
    def belief():
        key=take('I'); version(1)
        return dict(hash=key,player=take('i'),belief=take('I'),belief_class=take('i'),flag=take('B'),value=take('i'))
    def religious_city():
        version(4)
        return dict(city=city(), xy=take('ii'),xy10=take('ii'),a=take('i'),b=take('i'),c=take('i'),flag=take('B'),values=vector(lambda:take('I'),4))
    def religious():
        begin=pos;version(6);flag=take('B')
        value=dict(flag=flag)
        if flag:
            value.update(player=take('i'),religion=take('I'),flag118=take('B'),beliefs=vector(belief,25),
                handles=vector(handle,12),handle=handle(),flag130=take('B'),cities=vector(religious_city,45),yields=vector(hash_q8,8))
        children.append(dict(owner='GameCore::AI::Religious',start=begin,end=pos))
        return value
    version(4);wrapped=take('I');version(1);mayhem=take('I');version(1);factory=take('I')
    version(1);esp=vector(espionage,37)
    version(2);religion=vector(religious,5)
    scalar=take('i');values=vector(lambda:take('i'),4);a=vector(hash_q8,8);b=vector(hash_q8,8)
    if pos != len(raw) or reconstructed != raw: raise ValueError('extent/typed reconstruction mismatch')
    projection=dict(wrapped=wrapped,mayhem_f32_bits=mayhem,factory=factory,espionage=esp,religious=religion,scalar_raw_q8=scalar,values=values,yields_a8=a,yields_d0=b)
    return dict(bytes=pos,items=items,children=children,version_offsets=versions,count_offsets=counts,
                projection_sha256=sha(json.dumps(projection,sort_keys=True,separators=(',',':')).encode()),
                espionage=len(esp),religious=len(religion),religious_initialized=sum(x['flag']!=0 for x in religion))

def candidate(pin, records):
    pin = subprocess.check_output(['git', 'rev-parse', pin+'^{commit}'], cwd=ROOT, text=True).strip()
    archive = subprocess.check_output(['git', 'archive', pin], cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-global-ai-wire-') as directory:
        tmp = Path(directory)
        source = tmp/'source'
        source.mkdir()
        subprocess.run(['tar', '-x', '-C', str(source)], input=archive, check=True)
        driver = source/'tools/linux/game_global_ai_audit.py'
        if driver.read_bytes() != Path(__file__).read_bytes():
            raise ValueError('actual audit driver must be committed with candidate')
        module = source/'crates/c6-core/src/game_global_ai.rs'
        lib = source/'crates/c6-core/src/lib.rs'
        if 'pub mod game_global_ai;' not in lib.read_text():
            raise ValueError('candidate archive must already export game_global_ai')
        source_paths = ['Cargo.toml', 'Cargo.lock', 'crates/c6-core/Cargo.toml',
                        'tools/linux/game_global_ai_audit.py', 'tools/linux/player_wmds_audit.py']
        source_paths += [str(path.relative_to(source)) for path in
                         sorted((source/'crates/c6-core/src').rglob('*.rs'))]
        source_hashes = {}
        for path in source_paths:
            archived = (source/path).read_bytes()
            committed = subprocess.check_output(['git', 'show', pin+':'+path], cwd=ROOT)
            if archived != committed:
                raise ValueError('archived source differs from Git pin: '+path)
            source_hashes[path] = sha(archived)
        subprocess.run(['cargo', 'test', '--offline', '-p', 'c6-core', 'game_global_ai::tests'], cwd=source, check=True)
        subprocess.run(['cargo', 'clippy', '--offline', '-p', 'c6-core', '--all-targets', '--', '-D', 'warnings'], cwd=source, check=True)
        subprocess.run(['cargo', 'build', '--offline', '--release', '-p', 'c6-core'], cwd=source, check=True)
        harness = HARNESS
        (tmp/'candidate.rs').write_text(harness)
        command = ['rustc', '--edition=2021', '-Dwarnings', '-O', str(tmp/'candidate.rs'),
                   '--extern', 'c6_core='+str(source/'target/release/libc6_core.rlib'),
                   '-L', 'dependency='+str(source/'target/release/deps')]
        subprocess.run(command+['-o', str(tmp/'candidate')], check=True)
        (tmp/'records').write_bytes(records)
        n = int(subprocess.check_output([str(tmp/'candidate'), str(tmp/'records')], text=True).strip())
        if n != 2:
            raise ValueError('candidate record count differs')
        for path, digest in source_hashes.items():
            if sha((source/path).read_bytes()) != digest:
                raise ValueError('candidate build mutated archived source: '+path)
        return dict(source_commit=pin, archive_sha256=sha(archive),
                    module_sha256=sha(module.read_bytes()), driver_sha256=sha(driver.read_bytes()),
                    harness_sha256=sha(harness.encode()), executable_sha256=sha((tmp/'candidate').read_bytes()),
                    records=n, source_tests=3, strict_clippy=True,
                    isolated_export_added=False, source_files_sha256=source_hashes,
                    every_source_hash_verified_against_git_pin=True, actual_exported_crate_tests=True,
                    candidate_uses_exported_crate=True, archive_source_mutations=False,
                    exact_byte_and_shared_item_caps=True, bounded_native_hostile_counts_and_versions=True, bounded_native_truncations_rejected=True,
                    short_byte_and_item_caps_rejected=True, strict_trailing_rejected=True,
                    exact_prefix_consumption=True)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--source-commit')
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    elf=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    for a,n,name,digest in METHODS:
        if __import__("hashlib").sha256(elf.read(a,n)).hexdigest() != digest: raise ValueError('method identity differs: '+name)
    corpus=bytearray();fixtures=[]
    for identity,trace_name,trace_digest in FIXTURES:
        data=gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
        trace=gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
        if sha(data)!=identity or sha(trace)!=trace_digest:raise ValueError('fixture identity differs')
        owners=[r for r in map(json.loads,trace.splitlines()) if r.get('event')=='owner' and r.get('save')==1]
        rows=[r for r in owners if r['owner']=='GameCore::AI::GlobalAi']
        if len(rows)!=1:raise ValueError('GlobalAi cardinality differs')
        row=rows[0];raw=data[row['start']:row['end']];result=current_record(raw)
        children=[dict(owner=r['owner'],start=r['start']-row['start'],end=r['end']-row['start']) for r in owners if r['parent']==row['call']]
        if result.pop('children')!=children:raise ValueError('typed child extents differ')
        next_rows=[r for r in owners if r['start']==row['end'] and r['owner']=='GameCore::Barbarian::Manager']
        if len(next_rows)!=1:raise ValueError('next Barbarian boundary differs')
        versions=result['version_offsets'];counts=result['count_offsets']
        versions=[versions[i] for i in sorted({0,len(versions)-1}|{i*len(versions)//64 for i in range(64)})]
        counts=[counts[i] for i in sorted({0,len(counts)-1}|{i*len(counts)//64 for i in range(64)})]
        corpus.extend(struct.pack('<IIII',len(raw),result['items'],len(versions),len(counts)))
        for site in versions+counts:corpus.extend(struct.pack('<I',site))
        corpus.extend(raw)
        result['version_sites']=len(result.pop('version_offsets'));result['count_sites']=len(result.pop('count_offsets'))
        result.update(core_sha256=identity,trace_sha256=trace_digest,wire_sha256=sha(raw),start=row['start'],end=row['end'],direct_children=dict(Counter(c['owner'] for c in children)))
        fixtures.append(result)
    report=dict(schema='c6-linux-game-global-ai-v1',checks_passed=True,gamecore_sha256=GAME_SHA,
        methods=[dict(address=hex(a),size=n,name=s,sha256=h) for a,n,s,h in METHODS],fixtures=fixtures,
        candidate=candidate(args.source_commit,corpus) if args.source_commit else None,
        complete_current_global_ai_wire=True,original_execution=False,private_writes=False,
        native_read_side_effects=False,full_player=False,gameplay_state=False,world_import=False,actions=False,turns=False,
        unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198,next_component='GameCore::Barbarian::Manager')
    args.output.write_text(json.dumps(report,indent=2)+'\n')
if __name__=='__main__':main()
