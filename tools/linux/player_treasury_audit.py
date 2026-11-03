#!/usr/bin/env python3
"""Pinned read-only Techs contracts and offline saved-record comparisons; no original execution."""
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
from player_religion_audit import relocation_symbols
ROOT=Path(__file__).resolve().parents[2]
SOURCE_PIN="d4fa2c3"
METHODS = [(45732784,
  37,
  'FAutoVariable<FixedPointT<8u>, '
  'GameCore::Player::Treasury>::dummyLoadDelta(Serialization::IStream&)',
  '5b9eddbf7104f1a285aefaef26eb926f9a0fcbe07ad439a8324a54da3360229e'),
 (45732752,
  15,
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Treasury>::load(Serialization::IStream&)',
  '9241b20b42d40cba649a389c776d3c9cdd0febbbc610e0c56de22fd4f927a916'),
 (45732768,
  9,
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Treasury>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (45737968,
  67,
  'FAutoVariable<Types::Vector<FixedPointT<8u>, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::dummyLoadDelta(Serialization::IStream&)',
  'cc99b8c99276c8e2540a6755f4077b3b9342c4630e08244dea933aa73601425e'),
 (45737936,
  15,
  'FAutoVariable<Types::Vector<FixedPointT<8u>, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::load(Serialization::IStream&)',
  '3d2fc4ac7b6ce083f38eafc84d2f6d0ed984e8fc419f24342818222b057d89ce'),
 (45737952,
  9,
  'FAutoVariable<Types::Vector<FixedPointT<8u>, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (45740784,
  67,
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::dummyLoadDelta(Serialization::IStream&)',
  '019ab1ef0c664c6bccc904fc91e4468c965372f8633dd0ae8b3eee27ff037dff'),
 (45740752,
  15,
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::load(Serialization::IStream&)',
  '71a3237b8348b127b8e293be1739d99e781d9740b9ac5bc2df6e29381da65955'),
 (45740768,
  9,
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (45735408,
  19,
  'FAutoVariable<int, GameCore::Player::Treasury>::dummyLoadDelta(Serialization::IStream&)',
  '861327e18ed8b9a46af1251a5973c7b81714b08079179a91dc3f77f6fb3cb861'),
 (45735376,
  15,
  'FAutoVariable<int, GameCore::Player::Treasury>::load(Serialization::IStream&)',
  '139a327af1d1e2a93c9b5917dec32c631884402e609224f44d0316c74f1a3f9f'),
 (45735392,
  9,
  'FAutoVariable<int, GameCore::Player::Treasury>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (33537648,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  'const&)',
  'e623a70d6c0c3dbd957189686d9b2d3c6c233f1591769dc739a223eb1c947756'),
 (33801920,
  66,
  'unsigned int '
  'Serialization::IndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  'const&)',
  '42e027fc18f1f887f3ae21b1febcd2deb94084bc9cdf0ca06b4dffa25f84e1e1'),
 (45728992,
  72,
  'Serialization::IStream& Serialization::ReadVersioned<FAutoVariable<FixedPointT<8u>, '
  'GameCore::Player::Treasury>, int>(Serialization::IStream&, FAutoVariable<FixedPointT<8u>, '
  'GameCore::Player::Treasury>&, int, int&&, int&&)',
  '2594718d47cfba76506cc1b48317693198c9a45150d3dc747fc0991ae84563f7'),
 (45729072,
  38,
  'Serialization::IStream& '
  'Serialization::ReadVersioned<FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned '
  'int>, Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Treasury>, '
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Treasury>&>(Serialization::IStream&, '
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Treasury>&, int, int&&, '
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Treasury>&)',
  '9b4556f301cb91904497d7a51919663a097ff9c90686d1c8e9d4dd5dbbd213b2'),
 (45728944,
  34,
  'Serialization::IStream& Serialization::ReadVersioned<FAutoVariable<int, '
  'GameCore::Player::Treasury>, int>(Serialization::IStream&, FAutoVariable<int, '
  'GameCore::Player::Treasury>&, int, int&&, int&&)',
  '8eac96acc835c425fcfd8d06332279f454160cc4297fbf29ef47660ec7283dcc'),
 (33537488,
  157,
  'bool Serialization::ReadIndexHashArray<FixedPointT<8u>, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  '>(Serialization::IStream&, FixedPointT<8u>*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  'const&)',
  '00d655a36e6d013e376e1a1b11626e3f239d98882e9179362d9df313c4f5fb36'),
 (33450016,
  141,
  'bool Serialization::WriteIndexHashArray<FixedPointT<8u>, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  '>(Serialization::OStream&, FixedPointT<8u> const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  'const&)',
  '04e9734af531d3e9ae398f18a19c9120b09eeaabc2757e502072f44fac278ef7'),
 (33448384,
  131,
  'bool Serialization::ReadIndexHashArrayVersioned<FixedPointT<8u>, FixedPointT<8u>, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  '>(Serialization::IStream&, FixedPointT<8u>*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> '
  'const&, int, int&&, FixedPointT<8u>&&)',
  'e106bfef12560b60a2bf2781fcb13f3391fcb9abce72fe07ad5e3dc133e4ba76'),
 (45743488,
  125,
  'void Serialization::SerializeToSequenceContainer<std::__1::pair<unsigned int, unsigned int>, '
  'Types::Vector<std::__1::pair<unsigned int, unsigned int>, Types::LinearAlloc::Default<1u, 0u> > '
  '>(Serialization::IStream&, Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >&)',
  '52793aa9638c2cc94cc7299ccf3bcef1fa1496e038e0aa6f32655b7ea5978904'),
 (45744016,
  46,
  'Serialization::SerializeFromSequenceContainer<std::__1::pair<unsigned int, unsigned int> const, '
  'Types::Vector<std::__1::pair<unsigned int, unsigned int>, Types::LinearAlloc::Default<1u, 0u> > '
  'const>::SerializeFromSequenceContainer(Serialization::OStream&, '
  'Types::Vector<std::__1::pair<unsigned int, unsigned int>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&)',
  'd7ecde059e916d9eeba49845c71fcf3b8cf60954a770050da20fc4d6917ed840'),
 (45744064,
  8,
  'Serialization::SerializeFromSequenceContainer<std::__1::pair<unsigned int, unsigned int> const, '
  'Types::Vector<std::__1::pair<unsigned int, unsigned int>, Types::LinearAlloc::Default<1u, 0u> > '
  'const>::operator()(std::__1::pair<unsigned int, unsigned int> const&)',
  '353a5ba592ae60f19c47fb202f7abc6107d590092c283017d4892231fc29f335'),
 (46772192,
  42,
  'Serialization::operator<<(Serialization::OStream&, FixedPointT<8u> const&)',
  'bbd0553ff77e4be5a896850082327f68662e870164f4771d58a0e5c73c7d5704'),
 (45716064,
  234,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Treasury const&)',
  'd4bc0f8b270e7a780b8842d351561b91131fa36521b2e0c99dd89f5f5c1d8c30'),
 (45743840,
  91,
  'Serialization::OStream& Serialization::operator<< <std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, '
  'Types::Vector<std::__1::pair<unsigned int, unsigned int>, Types::LinearAlloc::Default<1u, 0u> > '
  'const&)',
  'accd7fd639ca26e31d54e55c5712009954faec2165e740728ef7667c1e194877'),
 (45744080,
  41,
  'Serialization::OStream& Serialization::operator<< <unsigned int, unsigned '
  'int>(Serialization::OStream&, std::__1::pair<unsigned int, unsigned int> const&)',
  '94be20cfe7b40ef749db5a15bf740b9e7de25947c2100fd706f63ebbad013b00'),
 (46772240,
  43,
  'Serialization::operator>>(Serialization::IStream&, FixedPointT<8u>&)',
  '5d47fcdfb3868524b3d8da8a2fab95de165678efa05f23c615963da12ddf010d'),
 (45716304,
  544,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Treasury&)',
  '0006a96d8e2217d0ed7ef03720841ca71ea902729bf63d8fd7122e0577fc9ba9'),
 (45743472,
  14,
  'Serialization::IStream& Serialization::operator>><std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >(Serialization::IStream&, '
  'Types::Vector<std::__1::pair<unsigned int, unsigned int>, Types::LinearAlloc::Default<1u, 0u> '
  '>&)',
  '20a8289369b0f5c47f96b616dec71f53e9515f17acd1730d78d4c0e1c77f9b85'),
 (45743712,
  41,
  'Serialization::IStream& Serialization::operator>><unsigned int, unsigned '
  'int>(Serialization::IStream&, std::__1::pair<unsigned int, unsigned int>&)',
  '1e744041ddfa1c87c09b07f72abb3cf1ba9ff43811920874ba6d14c108b67759'),
 (45743936,
  71,
  'Serialization::SerializeFromSequenceContainer<std::__1::pair<unsigned int, unsigned int> const, '
  'Types::Vector<std::__1::pair<unsigned int, unsigned int>, Types::LinearAlloc::Default<1u, 0u> > '
  'const> eastl::for_each<std::__1::pair<unsigned int, unsigned int> const*, '
  'Serialization::SerializeFromSequenceContainer<std::__1::pair<unsigned int, unsigned int> const, '
  'Types::Vector<std::__1::pair<unsigned int, unsigned int>, Types::LinearAlloc::Default<1u, 0u> > '
  'const> >(std::__1::pair<unsigned int, unsigned int> const*, std::__1::pair<unsigned int, '
  'unsigned int> const*, Serialization::SerializeFromSequenceContainer<std::__1::pair<unsigned '
  'int, unsigned int> const, Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> > const>)',
  '585b8eae81f747d8dbd42b892bc78b605a9b119852e9f4d741967b63d5d4f6df'),
 (45718416,
  338,
  'GameCore::Player::Treasury::Reset()',
  '5dddc8aa195ea12fb286d74d3dd5805dd41f28d54abcedb5f3f762aa15aa8cc6'),
 (45732832,
  15,
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Treasury>::save(Serialization::OStream&) const',
  '2aadf7fa8b74835dfd62a8de6604a785cc967db9e6c1a13dd6943f46fcabea3c'),
 (45732992,
  1735,
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Treasury>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, '
  'Types::LinearAlloc::Default<1u, 0u> > const&) const',
  '3c7a733d3b3eb859badacc1ad20af6f5db357e1ca454e47ffaab1b9ecb5d1f59'),
 (45732848,
  9,
  'FAutoVariable<FixedPointT<8u>, GameCore::Player::Treasury>::saveDelta(Serialization::OStream&) '
  'const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (45738048,
  15,
  'FAutoVariable<Types::Vector<FixedPointT<8u>, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::save(Serialization::OStream&) const',
  'a4f9a9f9c8795915ff877416f18a6a36085b8bce2d94daaa72845b7849b419fc'),
 (45738208,
  1815,
  'FAutoVariable<Types::Vector<FixedPointT<8u>, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, '
  'Types::LinearAlloc::Default<1u, 0u> > const&) const',
  'e198e3b1e70deaf637fa43bb5f6c9f8aaa3e31a1adda89999844687cf3c49bcc'),
 (45738064,
  9,
  'FAutoVariable<Types::Vector<FixedPointT<8u>, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (45740864,
  15,
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::save(Serialization::OStream&) const',
  '55217d71be79713c300fe290f54dfe6bde3cd577784b8b62ceef4e17d20c42fd'),
 (45741024,
  1815,
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, '
  'Types::LinearAlloc::Default<1u, 0u> > const&) const',
  'baaebe5db4d0dd63ad39c8668d05781705241da82cd81521b1d5b70187dc14ee'),
 (45740880,
  9,
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Treasury>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (45735440,
  15,
  'FAutoVariable<int, GameCore::Player::Treasury>::save(Serialization::OStream&) const',
  '21cc80789cf70b2759be0fd9a0b33cbabd3990b3ab99c574d93e5d8176435637'),
 (45735600,
  1724,
  'FAutoVariable<int, GameCore::Player::Treasury>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, '
  'Types::LinearAlloc::Default<1u, 0u> > const&) const',
  '58c64283f5feabbfff24c0c0dc0994898fda592973b06c9c2639414c17420341'),
 (45735456,
  9,
  'FAutoVariable<int, GameCore::Player::Treasury>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (45728720,
  21,
  'Serialization::OStream& operator<< <FixedPointT<8u>, '
  'GameCore::Player::Treasury>(Serialization::OStream&, FAutoVariable<FixedPointT<8u>, '
  'GameCore::Player::Treasury> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45728784,
  21,
  'Serialization::OStream& operator<< <Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Treasury>(Serialization::OStream&, '
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Treasury> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45728752,
  21,
  'Serialization::OStream& operator<< <int, GameCore::Player::Treasury>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Treasury> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45728816,
  21,
  'Serialization::IStream& operator>><FixedPointT<8u>, '
  'GameCore::Player::Treasury>(Serialization::IStream&, FAutoVariable<FixedPointT<8u>, '
  'GameCore::Player::Treasury>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45731296,
  21,
  'Serialization::IStream& operator>><Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Treasury>(Serialization::IStream&, '
  'FAutoVariable<Types::Vector<std::__1::pair<unsigned int, unsigned int>, '
  'Types::LinearAlloc::Default<1u, 0u> >, GameCore::Player::Treasury>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45728912,
  21,
  'Serialization::IStream& operator>><int, GameCore::Player::Treasury>(Serialization::IStream&, '
  'FAutoVariable<int, GameCore::Player::Treasury>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55')]
# Defined module bodies only; imported library routines have no local body address.
BINDINGS = [(45732797, 31783152, 33352544, 'call'),
 (45732808, 32963584, 46772240, 'call'),
 (45732762, 32963584, 46772240, 'jmp'),
 (45737984, 32206560, 33441568, 'call'),
 (45737995, 32014336, 33492240, 'call'),
 (45738003, 31841952, 33441888, 'call'),
 (45738022, 31841952, 33441888, 'call'),
 (45737946, 32014336, 33492240, 'jmp'),
 (45740800, 31664304, 45740672, 'call'),
 (45740811, 32816256, 45743472, 'call'),
 (45740819, 32582768, 45743792, 'call'),
 (45740838, 32582768, 45743792, 'call'),
 (45740762, 32816256, 45743472, 'jmp'),
 (45735420, 31938512, 33341856, 'call'),
 (45735386, 31938512, 33341856, 'jmp'),
 (33537656, 31762656, 33537696, 'call'),
 (33537670, 33028768, 33344784, 'jmp'),
 (33801931, 32193584, 33432176, 'call'),
 (33801945, 32134272, 33446704, 'call'),
 (33801959, 32559872, 33537984, 'jmp'),
 (33801966, 31762656, 33537696, 'call'),
 (45729022, 32860128, 33343664, 'call'),
 (45729033, 32455712, 45729888, 'call'),
 (45729046, 31660240, 45728816, 'call'),
 (45729090, 32412816, 45731328, 'call'),
 (45729100, 32048336, 45731296, 'call'),
 (45728958, 32207648, 37995936, 'call'),
 (45728968, 31747264, 45728912, 'call'),
 (33537516, 31815744, 33341888, 'call'),
 (33537558, 31815744, 33341888, 'call'),
 (33537570, 32990352, 33537648, 'call'),
 (33537603, 31783152, 33352544, 'call'),
 (33537614, 32963584, 46772240, 'call'),
 (33450041, 32193584, 33432176, 'call'),
 (33450058, 31588976, 33341776, 'call'),
 (33450085, 32134272, 33446704, 'call'),
 (33450093, 32559872, 33537984, 'call'),
 (33450108, 31588976, 33341776, 'call'),
 (33450119, 33099232, 46772192, 'call'),
 (33448409, 31957728, 33433088, 'call'),
 (33448420, 31767664, 33433104, 'call'),
 (33448451, 33028768, 33344784, 'call'),
 (33448465, 31649728, 33348608, 'call'),
 (33448510, 32977984, 33537488, 'jmp'),
 (45743505, 31548416, 45729856, 'call'),
 (45743526, 31815744, 33341888, 'call'),
 (45743543, 32826064, 45743616, 'call'),
 (45743571, 31691408, 45743648, 'call'),
 (45743579, 33206144, 45743760, 'call'),
 (45743590, 32013936, 45743712, 'call'),
 (45744034, 32385584, 45731616, 'call'),
 (45744051, 31588976, 33341776, 'call'),
 (45744067, 32455568, 45744080, 'jmp'),
 (46772203, 32683728, 46772496, 'call'),
 (46772220, 32898000, 33341744, 'call'),
 (45716082, 32898000, 33341744, 'call'),
 (45716097, 33208944, 45728720, 'call'),
 (45716112, 33208944, 45728720, 'call'),
 (45716127, 33208944, 45728720, 'call'),
 (45716142, 33174544, 45728752, 'call'),
 (45716157, 33174544, 45728752, 'call'),
 (45716172, 33174544, 45728752, 'call'),
 (45716187, 33174544, 45728752, 'call'),
 (45716202, 33174544, 45728752, 'call'),
 (45716217, 33208944, 45728720, 'call'),
 (45716232, 33208944, 45728720, 'call'),
 (45716244, 31461680, 33450304, 'call'),
 (45716252, 32462304, 35733216, 'call'),
 (45716266, 32162800, 33450016, 'call'),
 (45716284, 33226464, 45728784, 'call'),
 (45743860, 32765152, 45729952, 'call'),
 (45743871, 33024560, 45729968, 'call'),
 (45743890, 32842336, 45744016, 'call'),
 (45743911, 32298128, 45743936, 'call'),
 (45744090, 31588976, 33341776, 'call'),
 (45744105, 31588976, 33341776, 'call'),
 (46772255, 31938512, 33341856, 'call'),
 (46772267, 32345808, 35393568, 'call'),
 (45716329, 31938512, 33341856, 'call'),
 (45716344, 31660240, 45728816, 'call'),
 (45716359, 31660240, 45728816, 'call'),
 (45716371, 32886000, 45728848, 'call'),
 (45716412, 32910240, 42331456, 'call'),
 (45716424, 32674384, 37991600, 'call'),
 (45716465, 33253312, 33563776, 'call'),
 (45716480, 31747264, 45728912, 'call'),
 (45716525, 33028688, 45728944, 'call'),
 (45716570, 33028688, 45728944, 'call'),
 (45716615, 33028688, 45728944, 'call'),
 (45716660, 32174160, 45728992, 'call'),
 (45716705, 32174160, 45728992, 'call'),
 (45716717, 32468912, 37990432, 'call'),
 (45716725, 32787760, 33447008, 'call'),
 (45716733, 32462304, 35733216, 'call'),
 (45716763, 32860128, 33343664, 'call'),
 (45716787, 32710304, 33448384, 'call'),
 (45716825, 33026784, 45729072, 'call'),
 (45743476, 32901488, 45743488, 'call'),
 (45743722, 31815744, 33341888, 'call'),
 (45743737, 31815744, 33341888, 'call'),
 (45743974, 32149584, 45744064, 'call'),
 (45718436, 32886000, 45728848, 'call'),
 (45718446, 32747424, 33348624, 'call'),
 (45718458, 32886000, 45728848, 'call'),
 (45718468, 32747424, 33348624, 'call'),
 (45718480, 32886000, 45728848, 'call'),
 (45718490, 32747424, 33348624, 'call'),
 (45718505, 32860128, 33343664, 'call'),
 (45718520, 32455712, 45729888, 'call'),
 (45718535, 32860128, 33343664, 'call'),
 (45718550, 32455712, 45729888, 'call'),
 (45718562, 32674384, 37991600, 'call'),
 (45718580, 32674384, 37991600, 'call'),
 (45718598, 32674384, 37991600, 'call'),
 (45718616, 32674384, 37991600, 'call'),
 (45718634, 32674384, 37991600, 'call'),
 (45718655, 32468912, 37990432, 'call'),
 (45718663, 32892720, 33445040, 'call'),
 (45718671, 32468912, 37990432, 'call'),
 (45718679, 32462304, 35733216, 'call'),
 (45718687, 32193584, 33432176, 'call'),
 (45718702, 32860128, 33343664, 'call'),
 (45718715, 32214752, 33995808, 'call'),
 (45718730, 31462416, 45729792, 'call'),
 (45718738, 31548416, 45729856, 'call'),
 (45732842, 33099232, 46772192, 'jmp'),
 (45733038, 31783152, 33352544, 'call'),
 (45733049, 32963584, 46772240, 'call'),
 (45733059, 33252640, 33462608, 'call'),
 (45733081, 33252640, 33462608, 'call'),
 (45733120, 33099232, 46772192, 'call'),
 (45733143, 33099232, 46772192, 'call'),
 (45733303, 32626224, 33462640, 'call'),
 (45733336, 32588624, 33342032, 'call'),
 (45733556, 32626224, 33462640, 'call'),
 (45733589, 32588624, 33342032, 'call'),
 (45733714, 32626224, 33462640, 'call'),
 (45733759, 32588624, 33342032, 'call'),
 (45733810, 32626224, 33462640, 'call'),
 (45733850, 32588624, 33342032, 'call'),
 (45733924, 32105152, 33343920, 'call'),
 (45733934, 32105152, 33343920, 'call'),
 (45733947, 31792832, 46772512, 'call'),
 (45733962, 31792832, 46772512, 'call'),
 (45734041, 32626224, 33462640, 'call'),
 (45734054, 32626224, 33462640, 'call'),
 (45734273, 32626224, 33462640, 'call'),
 (45734286, 32626224, 33462640, 'call'),
 (45734397, 32626224, 33462640, 'call'),
 (45734410, 32626224, 33462640, 'call'),
 (45734483, 32626224, 33462640, 'call'),
 (45734496, 32626224, 33462640, 'call'),
 (45734613, 32588624, 33342032, 'call'),
 (45734623, 32588624, 33342032, 'call'),
 (45734704, 32588624, 33342032, 'call'),
 (45734714, 32588624, 33342032, 'call'),
 (45738058, 32001664, 33492992, 'jmp'),
 (45738257, 32206560, 33441568, 'call'),
 (45738268, 32014336, 33492240, 'call'),
 (45738283, 33252640, 33462608, 'call'),
 (45738305, 33252640, 33462608, 'call'),
 (45738344, 32001664, 33492992, 'call'),
 (45738372, 32001664, 33492992, 'call'),
 (45738531, 32626224, 33462640, 'call'),
 (45738564, 32588624, 33342032, 'call'),
 (45738781, 32626224, 33462640, 'call'),
 (45738814, 32588624, 33342032, 'call'),
 (45738928, 32626224, 33462640, 'call'),
 (45738973, 32588624, 33342032, 'call'),
 (45739028, 32626224, 33462640, 'call'),
 (45739068, 32588624, 33342032, 'call'),
 (45739154, 32105152, 33343920, 'call'),
 (45739164, 32105152, 33343920, 'call'),
 (45739177, 31961424, 33493248, 'call'),
 (45739195, 31961424, 33493248, 'call'),
 (45739274, 32626224, 33462640, 'call'),
 (45739287, 32626224, 33462640, 'call'),
 (45739506, 32626224, 33462640, 'call'),
 (45739522, 32626224, 33462640, 'call'),
 (45739639, 32626224, 33462640, 'call'),
 (45739652, 32626224, 33462640, 'call'),
 (45739722, 32626224, 33462640, 'call'),
 (45739735, 32626224, 33462640, 'call'),
 (45739847, 32588624, 33342032, 'call'),
 (45739857, 32588624, 33342032, 'call'),
 (45739870, 31841952, 33441888, 'call'),
 (45739925, 32588624, 33342032, 'call'),
 (45739982, 32588624, 33342032, 'call'),
 (45739992, 32588624, 33342032, 'call'),
 (45740010, 31841952, 33441888, 'call'),
 (45740874, 33196480, 45743840, 'jmp'),
 (45741073, 31664304, 45740672, 'call'),
 (45741084, 32816256, 45743472, 'call'),
 (45741099, 33252640, 33462608, 'call'),
 (45741121, 33252640, 33462608, 'call'),
 (45741160, 33196480, 45743840, 'call'),
 (45741188, 33196480, 45743840, 'call'),
 (45741347, 32626224, 33462640, 'call'),
 (45741380, 32588624, 33342032, 'call'),
 (45741597, 32626224, 33462640, 'call'),
 (45741630, 32588624, 33342032, 'call'),
 (45741744, 32626224, 33462640, 'call'),
 (45741789, 32588624, 33342032, 'call'),
 (45741844, 32626224, 33462640, 'call'),
 (45741884, 32588624, 33342032, 'call'),
 (45741970, 32105152, 33343920, 'call'),
 (45741980, 32105152, 33343920, 'call'),
 (45741993, 33131728, 45744144, 'call'),
 (45742011, 33131728, 45744144, 'call'),
 (45742090, 32626224, 33462640, 'call'),
 (45742103, 32626224, 33462640, 'call'),
 (45742322, 32626224, 33462640, 'call'),
 (45742338, 32626224, 33462640, 'call'),
 (45742455, 32626224, 33462640, 'call'),
 (45742468, 32626224, 33462640, 'call'),
 (45742538, 32626224, 33462640, 'call'),
 (45742551, 32626224, 33462640, 'call'),
 (45742663, 32588624, 33342032, 'call'),
 (45742673, 32588624, 33342032, 'call'),
 (45742686, 32582768, 45743792, 'call'),
 (45742741, 32588624, 33342032, 'call'),
 (45742798, 32588624, 33342032, 'call'),
 (45742808, 32588624, 33342032, 'call'),
 (45742826, 32582768, 45743792, 'call'),
 (45735450, 32898000, 33341744, 'jmp'),
 (45735646, 31938512, 33341856, 'call'),
 (45735656, 33252640, 33462608, 'call'),
 (45735678, 33252640, 33462608, 'call'),
 (45735717, 32898000, 33341744, 'call'),
 (45735740, 32898000, 33341744, 'call'),
 (45735900, 32626224, 33462640, 'call'),
 (45735933, 32588624, 33342032, 'call'),
 (45736153, 32626224, 33462640, 'call'),
 (45736186, 32588624, 33342032, 'call'),
 (45736311, 32626224, 33462640, 'call'),
 (45736356, 32588624, 33342032, 'call'),
 (45736407, 32626224, 33462640, 'call'),
 (45736447, 32588624, 33342032, 'call'),
 (45736521, 32105152, 33343920, 'call'),
 (45736531, 32105152, 33343920, 'call'),
 (45736638, 32626224, 33462640, 'call'),
 (45736651, 32626224, 33462640, 'call'),
 (45736870, 32626224, 33462640, 'call'),
 (45736883, 32626224, 33462640, 'call'),
 (45736994, 32626224, 33462640, 'call'),
 (45737007, 32626224, 33462640, 'call'),
 (45737080, 32626224, 33462640, 'call'),
 (45737093, 32626224, 33462640, 'call'),
 (45737210, 32588624, 33342032, 'call'),
 (45737220, 32588624, 33342032, 'call'),
 (45737301, 32588624, 33342032, 'call'),
 (45737311, 32588624, 33342032, 'call')]
TYPES=["Treasury"]
DWARF_SHA='fa2e66b26f710638b350e708cd99f56baef7900e9484ee037e6511deff9d75cb'
HARNESS=r'''pub use c6_core::unit;
#[path="MODULE"] pub mod player_treasury;
fn main(){let args:Vec<_>=std::env::args().collect();let input=std::fs::read(&args[1]).unwrap();let limits=unit::Limits{max_bytes:64*1024*1024,max_items:4_000_000};let(v,n,items)=player_treasury::Treasury::decode_prefix_with_items(&input,limits).unwrap();assert_eq!(n,input.len());for k in 0..input.len(){assert!(player_treasury::Treasury::decode(&input[..k],limits).is_err());}let exact=unit::Limits{max_bytes:n,max_items:items};assert_eq!(player_treasury::Treasury::decode(&input,exact).unwrap(),v);assert_eq!(v.encode_with_items(exact).unwrap().1,items);for low in [unit::Limits{max_bytes:n-1,..exact},unit::Limits{max_items:items-1,..exact}]{assert!(player_treasury::Treasury::decode(&input,low).is_err());assert!(v.encode(low).is_err());}let(out,witems)=v.encode_with_items(limits).unwrap();assert_eq!(items,witems);std::fs::write(&args[2],out).unwrap();println!("{} {} {} {}",n,items,v.gold_dispersal_change_q8.len(),v.gold_diplomatic_deals.len());}
'''
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
 p.add_argument('--output',type=Path,required=True)
 a=p.parse_args()
 game=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA)
 Elf(a.assets/'Civ6',HOST_SHA)
 for address,size,name,digest in METHODS:
  if sha(game.read(address,size))!=digest:raise ValueError('method mismatch: '+name)
 if struct.unpack('<I',game.read(0x2f6937c,4))[0]!=10:raise ValueError('current version mismatch')
 calls=[(0x2b4f0ad,0x1eb9310),(0x2b4f0c0,0x1f4cb40),(0x2b4f0d3,0x1f84390),(0x2b4f0e6,0x1f66d50),(0x2b4f0f9,0x1f35f50)]
 for address,target in calls:game.call(address,target)
 relocations=relocation_symbols(game)
 for site,plt,body,kind in BINDINGS:
  code=game.read(site,5)
  if code[0]!=(0xe8 if kind=='call' else 0xe9) or site+5+struct.unpack_from('<i',code,1)[0]!=plt:raise ValueError('direct binding changed')
  stub=game.read(plt,6)
  if stub[:2]!=b'\xff\x25':raise ValueError('PLT changed')
  got=plt+6+struct.unpack_from('<i',stub,2)[0]
  if relocations.get(got)!=body:raise ValueError('concrete overload changed')
 command=['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so')]
 for name in TYPES:command+=['-ex',"ptype /o 'GameCore::Player::"+name+"'"]
 dwarf=subprocess.check_output(command,text=True)
 if sha(dwarf.encode())!=DWARF_SHA:raise ValueError('matching DWARF changed')
 pin=subprocess.check_output(['git','rev-parse',SOURCE_PIN+'^{commit}'],cwd=ROOT,text=True).strip()
 reports=[]
 with tempfile.TemporaryDirectory(prefix='c6-treasury-')as tmp:
  tmp=Path(tmp); source=tmp/'source';source.mkdir()
  archive=subprocess.check_output(['git','archive',pin],cwd=ROOT)
  subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
  paths=[source/'Cargo.toml',source/'Cargo.lock',source/'crates/c6-core/Cargo.toml',*sorted((source/'crates/c6-core/src').rglob('*.rs'))]
  sources={str(x.relative_to(source)):sha(x.read_bytes())for x in paths}
  subprocess.run(['cargo','build','--offline','-p','c6-core'],cwd=source,check=True)
  harness=HARNESS.replace('MODULE',str(source/'crates/c6-core/src/player_treasury.rs'));(tmp/'candidate.rs').write_text(harness)
  cmd=['rustc','--edition=2021','-Dwarnings',str(tmp/'candidate.rs'),'--extern','c6_core='+str(source/'target/debug/libc6_core.rlib'),'-L','dependency='+str(source/'target/debug/deps')]
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
   cuts=sorted([x for x in rows if x.get('event')=='owner'and x.get('save')==1 and x.get('owner')=='GameCore::Player::Treasury'],key=lambda x:x['start'])
   if not cuts:raise ValueError('no Treasury validation cuts')
   observations=[];digests=[];total=0
   for cut in cuts:
    payload=data[cut['start']:cut['end']];(tmp/'in').write_bytes(payload)
    result=subprocess.check_output([str(tmp/'candidate'),str(tmp/'in'),str(tmp/'out')],text=True)
    if (tmp/'out').read_bytes()!=payload:raise ValueError('saved record differs')
    words=[int(x)for x in result.split()]
    if words[0]!=len(payload):raise ValueError('typed prefix mismatch')
    total+=len(payload);observations.append(words);digests.append(sha(payload))
   reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(cuts),record_bytes=total,first_observed_range=[cuts[0]['start'],cuts[0]['end']],ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode()),typed_observations_sha256=sha(json.dumps(observations,separators=(',',':')).encode())))
 report=dict(schema='c6-linux-player-treasury-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_version=10,version_address='0x2f6937c',methods=[dict(address=hex(x),size=n,name=name,sha256=h)for x,n,name,h in METHODS],direct_calls=[dict(address=hex(x),target=hex(y))for x,y in calls],concrete_bindings=[dict(site=hex(a),plt=hex(b),body=hex(c),kind=k)for a,b,c,k in BINDINGS],dwarf_sha256=sha(dwarf.encode()),candidate_source_commit=pin,candidate_archive_sha256=sha(archive),candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=3,exact_native_record_byte_item_caps_checked=True,one_short_native_record_caps_rejected=True,every_native_record_prefix_rejected=True,allocation_policy='checked per-vector storage; aggregate consumed wire bytes and items',original_execution=False,private_writes=False,complete_current_treasury_wire=True,definition_remapping=False,native_read_fixups=False,full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,next_player_component='Units',unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
 a.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,methods=len(METHODS),fixtures=reports)))
if __name__=='__main__':main()
