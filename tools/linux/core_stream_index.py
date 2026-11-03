#!/usr/bin/env python3
"""Structural prefix index/extractor for the pinned Linux Core stream grammar.

No trace, fixture offsets, byte-pattern search, engine calls or inferred opaque
unframed skips. A supported Map is complete; unparsed downstream state is explicit.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
from map_state_audit import Reader as MapReader

GAME_SHA = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST_SHA = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
MAX_STREAM = 256 * 1024 * 1024


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Reader(MapReader):
    def __init__(self, data):
        super().__init__(data)
        self.sections = []
        self.variant_nodes = 0

    def section(self, name, function):
        start = self.pos
        value = function()
        self.sections.append(dict(name=name, start=start, end=self.pos,
                                  bytes=self.pos-start, sha256=sha(self.data[start:self.pos])))
        return value

    def boolean(self):
        value = self.take('B')
        if value not in (0, 1):
            raise ValueError('noncanonical bool at ' + str(self.pos - 1))
        return bool(value)

    def string8(self):
        return self.raw(self.count())

    def static_array(self, fmt, count=64):
        # Native Types::StaticArray writes each element; it has NO wire count.
        if fmt == '?':
            return [self.boolean() for _ in range(count)]
        return [self.take(fmt) for _ in range(count)]

    def untyped_map(self, depth=0):
        if depth > 64:
            raise ValueError('variant depth budget')
        scalar = [(self.u32(), self.u32()) for _ in range(self.count())]
        arrays = [(self.u32(), self.array_variant(depth + 1)) for _ in range(self.count())]
        return dict(scalars=scalar, arrays=arrays)

    def typed_variant(self, depth):
        # Native framing does not switch on type/modifier tags.
        self.variant_nodes += 1
        if depth > 64 or self.variant_nodes > 1_000_000:
            raise ValueError('variant node/depth budget')
        kind, modifiers = self.take('H'), self.take('H')
        return dict(kind=kind, modifiers=modifiers, value=self.array_variant(depth, kind))

    def array_variant(self, depth, kind=None):
        if depth > 64:
            raise ValueError('variant depth budget')
        flags, width = self.u32(), self.u32()
        branch = flags & 0x5e000000
        if branch.bit_count() > 1:
            raise ValueError('unsupported variant branch at ' + str(self.pos - 8))
        if branch == 0x04000000 or branch == 0x40000000:
            payload = self.variant_map(depth + 1)
        elif branch == 0x02000000:
            payload = self.untyped_map(depth + 1)
        elif branch == 0x10000000:
            payload = [self.typed_variant(depth + 1) for _ in range(self.count())]
        elif branch == 0x08000000:
            payload = None
        elif flags & 0x01000000:
            # Explicit native element count/width framing only; no payload semantics.
            size = (flags & 0xffffff) * width
            if size > 0xffffffff:
                raise ValueError('unsupported overflowing raw-array byte count')
            payload = self.raw(size)
        else:
            payload = self.raw(8 if flags & 0x80000000 else 4)
        return dict(flags=flags, element_size=width, payload=payload)

    def rules(self):
        self.version(1)
        count = self.count()
        if count != 1:
            raise ValueError('unsupported unframed Rules count: ' + str(count))
        if self.u32() != 0xf3e6941e:
            raise ValueError('unsupported unframed Rules provider')
        self.version(1)
        return dict(placement=True)

    def player_processor(self):
        self.version(2)
        flags = [self.static_array('?') for _ in range(6)]
        counts = [self.static_array('I') for _ in range(3)]
        return dict(flags=flags, counts=counts, active_count=self.u32(), human_first=self.boolean())

    def phase(self):
        self.version(3)
        current, time_limit = self.take('i'), self.u32()
        disabled = [(self.u32(), self.boolean()) for _ in range(self.count())]
        return dict(current=current, time_limit=time_limit, disabled=disabled, last_segment=self.take('i'))

    def processor(self):
        self.version(2)
        last_initial_ai = self.static_array('i')
        turn_order = self.static_array('i')
        last_ai = self.take('i')
        flags = [self.boolean() for _ in range(3)]
        players = self.section('player_processor', self.player_processor)
        phase = self.section('phase_processor', self.phase)
        return dict(last_initial_ai=last_initial_ai, turn_order=turn_order,
                    last_ai=last_ai, flags=flags, players=players, phase=phase)

    def components(self):
        # Reuse all Map field parsing, but index arbitrary named LENGTH-framed payloads.
        envelope_start = self.pos
        self.version(1)
        components = []
        for _ in range(self.count()):
            name = self.string8().decode('ascii')
            size = self.count()
            start = self.pos
            self.raw(size)
            row = dict(name='map_component/' + name, start=start, end=self.pos,
                       bytes=size, sha256=sha(self.data[start:self.pos]), semantics_decoded=False)
            self.sections.append(row)
            components.append(row)
        self.sections.append(dict(name='map_components', start=envelope_start, end=self.pos,
            bytes=self.pos-envelope_start, sha256=sha(self.data[envelope_start:self.pos])))
        return components

    def religion_record(self):
        self.version(7)
        self.raw(3 * 4 + 2 * 4)  # Religion hash, player, founded turn, PlotCoord.
        self.string8()
        for _ in range(3):
            self.raw(self.count() * 4)  # Belief/Building/Unit hashes.
        self.take('i')
        self.static_array('?', 3)
        self.take('i')

    def religion(self):
        self.version(7)
        self.raw(8)
        count = self.count()
        for _ in range(count):
            self.religion_record()
        return dict(records=count)

    def first_player_prefix(self):
        self.version(47)
        owner = self.take('i')
        self.raw(8)  # Starting PlotCoord.
        self.raw(15 * 4)  # Original ordered scalar fields through EndTurnBlocking.
        self.static_array('?', 6)
        self.raw(4 * 4)  # Handicap hash, District hash, Project hash, final int.
        return owner

    def formation(self):
        start = self.pos
        self.version(1)
        self.raw(32)  # Existing current Formation v1 fixed prefix after version.
        self.raw(self.count() * 20)  # Native cached path nodes.
        self.raw(8)  # Cached destination coordinate.
        self.raw(self.count() * 4)  # Member unit IDs.
        return dict(start=start, end=self.pos)

    def first_formations(self):
        self.version(1)
        return self.recycled(lambda: self.section('formation', self.formation))

    def indexed_ints(self):
        # Current WriteIndexHashArray<int>: count then ordered hash/value pairs.
        return [(self.u32(), self.take('i')) for _ in range(self.count())]

    def goody_hut(self):
        self.version(1)
        return dict(player_goodies=self.indexed_ints())

    def bonuses(self):
        self.version(4)
        return dict(flat_bonuses=self.indexed_ints())

    def congress_refund(self):
        self.version(12)
        return dict(resolution_hash=self.u32(), which_effect=self.take('i'), refund_percent=self.take('i'))

    def congress(self):
        self.version(12)
        favor = self.static_array('i', 6)
        suzerain = self.indexed_ints()
        influence = self.indexed_ints()
        trade = self.indexed_ints()
        slots = [self.take('i') for _ in range(self.count())]
        modifier = self.indexed_ints()
        modifiers = self.static_array('i', 4)
        refunds = [self.congress_refund() for _ in range(self.count())]
        wants, vp = self.boolean(), self.take('i')
        return dict(favor=favor, suzerain=suzerain, influence=influence, trade=trade,
                    government_slots=slots, trade_modifier=modifier, modifiers=modifiers,
                    refunds=refunds, wants_congress=wants, diplo_vp_last_turn=vp)


def index_core(data, *, first_formations=False, first_player_components=False):
    first_formations = first_formations or first_player_components
    if len(data) > MAX_STREAM:
        raise ValueError('Core stream resource limit')
    r = Reader(data)
    r.version(5)
    metadata = r.section('context_metadata', r.variant_map)
    r.section('index_serializer_version', lambda: r.version(1))
    r.section('game_version', lambda: r.version(22))
    r.section('rules', r.rules)
    processor = r.section('game_processor', r.processor)
    game_scalars = r.section('game_scalars', lambda: r.static_array('i', 3))
    r.section('game_properties', r.untyped_map)
    world = r.section('map', r.map)
    religion, players, first = None, None, None
    component_counts = None
    if first_formations:
        religion = r.section('game_religion', r.religion)
        r.version(1)
        players = r.count()
        if players:
            external_owner = r.take('i')
            owner = r.section('first_player_prefix', r.first_player_prefix)
            if owner != external_owner:
                raise ValueError('player manager identity differs from player record')
            formations = r.section('first_player_formations', r.first_formations)
            first = dict(owner=owner, formations=len(formations['entries']))
            if first_player_components:
                goody = r.section('first_player_goody_hut', r.goody_hut)
                bonus = r.section('first_player_bonuses', r.bonuses)
                congress = r.section('first_player_congress', r.congress)
                component_counts = dict(goody_hut_pairs=len(goody['player_goodies']),
                    bonus_pairs=len(bonus['flat_bonuses']),
                    congress_indexed_pairs=[len(congress[k]) for k in ('suzerain','influence','trade','trade_modifier')],
                    government_slot_values=len(congress['government_slots']),
                    refund_records=len(congress['refunds']))

    return dict(schema='c6-linux-core-prefix-index-v1', module_contract_sha256=GAME_SHA,
        stream_sha256=sha(data), stream_bytes=len(data), sections=r.sections,
        context_metadata_entries=len(metadata['entries']),
        current_turn=game_scalars[0], max_turns=game_scalars[1], local_player=game_scalars[2],
        current_phase=processor['phase']['current'], map_width=world['header']['width'],
        map_height=world['header']['height'], plots=len(world['plots']),
        religion_records=religion['records'] if religion else None, player_count=players, first_player=first,
        parsed_end=r.pos, unparsed_tail_bytes=len(data)-r.pos,
        next_unframed_schema=(('Player.Cities' if first_player_components else 'Player.Goody_Hut') if players else 'AI.GlobalAi') if first_formations else 'Game.Religion',
        first_player_components_requested=first_player_components, first_player_component_counts=component_counts,
        first_formations_requested=first_formations,
        unit_location_status='Unsupported: unframed player subsystems precede Units',
        complete_map_index=True, complete_unit_index=False, complete_formation_roster_index=False,
        complete_core_index=False, full_world_import=False, gameplay_parity=False,
        unknown_bytes_masked=0, trace_required=False, fixture_offsets_used=False)


def read_stream(path):
    with (gzip.open(path, 'rb') if path.suffix == '.gz' else path.open('rb')) as source:
        data = source.read(MAX_STREAM + 1)
    if len(data) > MAX_STREAM:
        raise ValueError('Core stream resource limit')
    return data


def main():
    import sys
    if len(sys.argv) > 1 and sys.argv[1] == 'audit':
        audit_main(sys.argv[2:])
        return
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('stream', type=Path)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--extract-map', type=Path)
    p.add_argument('--first-formations', action='store_true', help='Extend past Map through the first player Formation collection only')
    p.add_argument('--first-player-components', action='store_true', help='Extend through first Player Goody_Hut, Bonuses and complete current Congress; stop at Cities')
    args = p.parse_args()
    if args.output.resolve() == args.stream.resolve():
        raise ValueError('refuse to overwrite the input Core stream')
    data = read_stream(args.stream)
    result = index_core(data, first_formations=args.first_formations, first_player_components=args.first_player_components)
    if args.extract_map and args.extract_map.resolve() == args.output.resolve():
        raise ValueError('Map extraction and index output must be distinct')
    if args.extract_map:
        row = next(s for s in result['sections'] if s['name'] == 'map')
        with args.extract_map.open('xb') as output:
            output.write(data[row['start']:row['end']])
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: result[k] for k in ('complete_map_index', 'plots', 'parsed_end', 'complete_core_index')}))


METHODS = [('gamecore',
  43039728,
  161,
  'GameCore::Context::Instance::Write(Serialization::OStream&) const',
  '96913cdfb9ded76f82d174aee6a76dda5636bac24c69532258b7d5c7e5279763'),
 ('gamecore',
  43039904,
  310,
  'GameCore::Context::Instance::Read(Serialization::IStream&)',
  '81dc994f9f6ad8bece5e2b421974aa6b3e78284d692e3c3323c417ad8ac7e371'),
 ('gamecore',
  47399552,
  12,
  'GameCore::IndexSerializer::SaveVersion(Serialization::OStream&)',
  '49010e3b612c9b4de140c013c3e725ff3a66df236b02af883f40d88ea071c961'),
 ('gamecore',
  47399568,
  12,
  'GameCore::IndexSerializer::LoadVersion(Serialization::IStream&)',
  '814a4c9eb1631167536e0a585e613e070b286b3815881d2b9b01950a1ab2df61'),
 ('gamecore',
  47687616,
  728,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Game::Instance const&)',
  'fe25d7b118dade6882a0e8d3e2ebba13ce8e32fb0f7dafd78f67add672753722'),
 ('gamecore',
  47688352,
  2540,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Game::Instance&)',
  '917d430f7b9c105932f394d62b2b75c83f698437ba89e7ce0ece979dad395873'),
 ('gamecore',
  46056720,
  67,
  'GameCore::Rules::Manager::Write(Serialization::OStream&)',
  '18f5b653a66aa34019ee1f755543380f431292c91b3d1991fc168f78c4cb6df4'),
 ('gamecore',
  46056544,
  170,
  'GameCore::Rules::Manager::Read(Serialization::IStream&)',
  'b2eb2d0ff67390c4c497af8e6473ca97a146d6a7f8af5e6706525abbdef1636e'),
 ('gamecore',
  46068064,
  52,
  'GameCore::Rules::Placement::Instance::Write(Serialization::OStream&)',
  'f9eaa1c3340d8560a36a9eb52bbd7b6fac1a11afd682309f855ad4d11e6df4e2'),
 ('gamecore',
  46068048,
  15,
  'GameCore::Rules::Placement::Instance::Read(Serialization::IStream&)',
  '658922a23b4b7b6bbb9d3ca33bef8006fb29efbd24fb1997b453153cf419b911'),
 ('gamecore',
  46076096,
  6,
  'GameCore::Rules::Placement::Instance::GetID() const',
  '6b2134e380690354dc806cbfa18ca4dbda16a8cd86079ea5a96bb75e9e28f4b1'),
 ('gamecore',
  44085696,
  123,
  'GameCore::Game::Processor::Write(Serialization::OStream&)',
  '030659a92d26a848c66c4ab0a524fa20e08f067080a64817fb1b33196cf1f39e'),
 ('gamecore',
  44085536,
  146,
  'GameCore::Game::Processor::Read(Serialization::IStream&)',
  '82ede96443daa4e7325e746c534ba2af575975c63e8d97898b31af9dba70cf35'),
 ('gamecore',
  45459184,
  182,
  'GameCore::Player::Processor::Write(Serialization::OStream&)',
  '24b7f4e9c244b6b7da58ecadeaf07f7feffc6ce0d67c575b8e416dcaf37eea7b'),
 ('gamecore',
  44386240,
  77,
  'GameCore::Phase::Processor::Write(Serialization::OStream&)',
  'b9363887b710fd26f64ce935b5116d60d72e5ac7c2b4a2d34a1a9d2980275851'),
 ('gamecore',
  44385936,
  103,
  'GameCore::Phase::Processor::Read(Serialization::IStream&)',
  'f070df7132e25dddc26fcd1d063e0b3a8936eeb83f45addeca3d441a8f98ef56'),
 ('gamecore',
  42607696,
  57,
  'Serialization::OStream& Serialization::operator<< <int, 64ul>(Serialization::OStream&, '
  'Types::StaticArray<int, 64ul> const&)',
  '1160044aac8668625b62e0243935cc139c38a02e730c536d3f29b877d8e5d554'),
 ('gamecore',
  44830304,
  57,
  'Serialization::OStream& Serialization::operator<< <bool, 64ul>(Serialization::OStream&, '
  'Types::StaticArray<bool, 64ul> const&)',
  '1c718422d7b2169aaebd4794682fe49b52d1d7bbb6fefa28def835130cafe9e1'),
 ('gamecore',
  44948032,
  57,
  'Serialization::OStream& Serialization::operator<< <unsigned int, 64ul>(Serialization::OStream&, '
  'Types::StaticArray<unsigned int, 64ul> const&)',
  '985bfe70a0eecafb00022b9ef5e539b995a61d80dfd3d176facf0a8d2ab84224'),
 ('gamecore',
  44391136,
  56,
  'Serialization::OStream& Serialization::operator<< <bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::TurnSegment>, '
  '&GameCore::Definitions::GetTurnSegmentDefinitions>(Serialization::OStream&, GameCore::IndexList<bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::TurnSegment>, '
  '&GameCore::Definitions::GetTurnSegmentDefinitions, void> const&)',
  '2c581206b5046f56f78c3ba263d658a240ba58b32629ddafd1ad51765ad58da3'),
 ('gamecore',
  44330480,
  508,
  'GameCore::Map::Instance::Write(Serialization::OStream&) const',
  '756252461e75aa7699cb5610203d69d7b63c32fbc59064ac13d9c746e924870e'),
 ('gamecore',
  44331024,
  555,
  'GameCore::Map::Instance::Read(Serialization::IStream&)',
  '203be99fb7e29951094b8c224aa37d5bec3e1fc7cc0b15ff95cef082fb810261'),
 ('gamecore',
  44128192,
  81,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Game::Religion const&)',
  '68b7dad39e6e046ebab3058ba8c187ac3bbbe79330c39613fe11ffab4a275a83'),
 ('gamecore',
  44128288,
  79,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Game::Religion&)',
  'be558b177e9f3bcca1dbd306694b1ba79eb44301d4c20976c4a9d3364cd5b8fb'),
 ('gamecore',
  44127584,
  206,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Game::ReligionData const&)',
  '2e16af4145ff5675c66243e79eb219e100e0c78463a0553c6fab7245253fd6ba'),
 ('gamecore',
  45420640,
  136,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Manager const&)',
  '9a04b5b429cb4694c382e6dd096f28539267d17645cff0538e425cd693bbb8f6'),
 ('gamecore',
  45420784,
  132,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Manager&)',
  'ab965247cffca1fbce8acb41ba7d7397770a9681a361847088e0feafcbc9e55d'),
 ('gamecore',
  45411840,
  1679,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Instance const&)',
  'cbeacc8a90c5fa8853e90d6971174683990e8bbee2c91c00a9be1bb2023d8593'),
 ('gamecore',
  45413520,
  3518,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Instance&)',
  '6cafd8cbdf97f8acc05be3175d60823c706339e01e84c9b757aa63009ef0f3ed'),
 ('gamecore',
  45108592,
  48,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Formations const&)',
  '21b2c38c924d60fa5c1889cfa842ed08019a9fc41de4de589ebe5441bf35a260'),
 ('gamecore',
  45108640,
  46,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Formations&)',
  'b3c00e618a9221d5a5164f12671722e89c6a1ca5e165d58ca9fe9ab97483bd31'),
 ('gamecore',
  43234368,
  276,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Formation::Instance const&)',
  'b2f6335d6e18fc88f201fe39aaed436c6748757bc088516188f747c2af8dd114'),
 ('gamecore',
  43234656,
  385,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Formation::Instance&)',
  '5b123a18e008411f37fc3b7848a0929eab7dff3370a0a66107d373021913b82c'),
 ('gamecore',
  45750912,
  986,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Units const&)',
  '457582f7f2b696f50af168fd435b140a5fc71f2229617bbe40c48c1cc46c12ca'),
 ('host',
  60194208,
  283,
  'Data::ArrayVariant::Serialize(Serialization::OStream&, Data::VariantSerializationOptions) const',
  '18bbc35d2364538d8788e6583261355a3a19293d9e22c37b73621c2f071b187e'),
 ('host',
  60194912,
  1033,
  'Data::ArrayVariant::Unserialize(Serialization::IStream&, Data::VariantManager*)',
  'e0176b74687662be2b0c8941e82fdb88d604d35d1301dfcea20f6e21d65fc4b3'),
 ('host',
  60194496,
  364,
  'Data::operator<<(Serialization::OStream&, Data::VariantMap const&)',
  'a356395aa95c564acd8f5a545d6cf602d2adaf097adc371ab883b420b863bd51'),
 ('host',
  60195952,
  223,
  'Data::operator>>(Serialization::IStream&, Data::VariantMap&)',
  'fa39a32bfb35884cf4b734bad96a8e111dca2ca2dfc596dfa50f3935e57c6895'),
 ('host',
  60232976,
  110,
  'Data::TypedVariantKeyedContainer::SerializeHeader(Serialization::OStream&, '
  'Data::VariantSerializationOptions) const',
  '37f5b792c5c47ebf533c5b2eaade032cbcdfd4cfd3ff622bfd66c357a7d3168a'),
 ('host',
  60233088,
  206,
  'Data::TypedVariantKeyedContainer::SerializeMember(Serialization::OStream&, unsigned int, '
  'Data::TypedVariant const&, Data::VariantSerializationOptions) const',
  'fd104b06d05b0a86de839f8b58adab3aff58be58286212c248f9a5e6a942c99b'),
 ('host',
  60232496,
  468,
  'Data::TypedVariantKeyedContainer::Unserialize(Serialization::IStream&, Data::TypedVariantManager*)',
  '926471abf5f8aba34009e30e24c276adebac3f80a62beca98aaa8ad8bb9feaa8')]

METHODS += [('gamecore', 45458976, 196, 'GameCore::Player::Processor::Read(Serialization::IStream&)', 'ffec19bb07201ca77505530adffa491cc333ff99b9eabcd622afa0b0150d286e')]

def audit_main(argv):
    from save_lifecycle_audit import Elf
    p = argparse.ArgumentParser(description='Read-only original serializer audit and structural index tests')
    p.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    p.add_argument('--private-root', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-ghidra')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args(argv)
    modules = dict(gamecore=Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA),
                   host=Elf(args.assets / 'Civ6', HOST_SHA))
    for module, address, size, name, expected in METHODS:
        if sha(modules[module].read(address, size)) != expected:
            raise ValueError('original serializer differs: ' + name)
    game = modules['gamecore']
    versions = [('context', 0x2f2f0b0, 5), ('game', 0x2f90220, 22),
        ('index', 0x2f87528, 1), ('rules', 0x2f6de10, 1), ('placement', 0x2f6e2b0, 1),
        ('game_processor', 0x2f52660, 2), ('player_processor', 0x2f66384, 2),
        ('phase', 0x2f568e0, 3), ('religion', 0x2f52ca0, 7),
        ('player_manager', 0x2f65d68, 1), ('player', 0x2f65d64, 47),
        ('player_formations', 0x2f611c8, 1)]
    for name, address, expected in versions:
        if int.from_bytes(game.read(address, 4), 'little') != expected:
            raise ValueError('version differs: ' + name)
    virtuals = [(0x3ae3808, 0x40, 0x2bec510), (0x3ae3a40, 0x20, 0x2bf10c0),
                (0x3ae3a40, 0x48, 0x2bef160)]
    for table, slot, target in virtuals:
        if game.relocated_pointer(table + 16 + slot) != target:
            raise ValueError('Rules virtual provider differs')
    parser = Path(__file__).with_name('map_state_audit.py')
    if sha(parser.read_bytes()) != 'f43b2712ef67a4e2fdb1ab0ba578bc4796adc6241d232cb020ed9a35cf13f96b':
        raise ValueError('reviewed Map field parser identity differs')
    canonical = args.private_root / 'evidence/linux-game-load/canonical'
    identities = ['ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884',
                  '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8']
    # Old owner evidence is used ONLY after parsing, as an independent boundary check.
    trace_dir = args.private_root / 'evidence/linux-save-normalization/checkpoint-203-owners'
    traces = [('trace-benchmark-1.jsonl.gz', '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
              ('trace-captured-1.jsonl.gz', '027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')]
    fixtures = []
    baseline = None
    for identity, (trace_name, trace_digest) in zip(identities, traces):
        data = read_stream(canonical / (identity + '.bin.gz'))
        if sha(data) != identity:
            raise ValueError('original validation fixture identity differs')
        indexed = index_core(data, first_formations=True)
        rows = indexed['sections']
        outer = next(x for x in rows if x['name'] == 'map')
        with gzip.open(trace_dir / trace_name, 'rb') as f:
            raw_trace = f.read()
        if sha(raw_trace) != trace_digest:
            raise ValueError('native validation trace identity differs')
        owners = [json.loads(line) for line in raw_trace.splitlines()]
        owners = [x for x in owners if x.get('event') == 'owner' and x.get('save') == 1]
        known_map = [x for x in owners if x['owner'] == 'GameCore::Map::Instance']
        if len(known_map) != 1 or (outer['start'], outer['end']) != (known_map[0]['start'], known_map[0]['end']):
            raise ValueError('structural Map boundary differs from independent native owner')
        first = next(x for x in rows if x['name'] == 'first_player_formations')
        actual = [(x['start'], x['end']) for x in owners
                  if x['owner'] == 'GameCore::Formation::Instance' and first['start'] <= x['start'] < first['end']]
        computed = [(x['start'], x['end']) for x in rows if x['name'] == 'formation']
        if sorted(actual) != computed or not computed:
            raise ValueError('structural Formation boundaries differ from native owners')
        # Complete extracted section, exhausted by the independent typed Map decoder.
        payload = data[outer['start']:outer['end']]
        check = MapReader(payload)
        decoded = check.map()
        if check.pos != len(payload) or len(decoded['plots']) != indexed['plots']:
            raise ValueError('standalone extracted Map does not exhaust typed decoder')
        fixtures.append(dict(stream_sha256=identity, stream_bytes=len(data),
            native_owner_trace_sha256=sha(raw_trace), map=outer,
            indexed_sections=len(rows), first_player=indexed['first_player'],
            formation_boundaries_matched=len(computed), parsed_end=indexed['parsed_end'],
            unparsed_tail_bytes=indexed['unparsed_tail_bytes'], next_unframed_schema=indexed['next_unframed_schema']))
        if baseline is None:
            baseline = (data, indexed)
    data, indexed = baseline
    def span(name):
        return next(x for x in indexed['sections'] if x['name'] == name)
    original_map = span('map')
    # Different valid structural prefix lengths must relocate Map by parsed lengths.
    metadata = span('context_metadata')
    prefix = bytearray(data[:metadata['end']])
    if struct.unpack_from('<I', prefix, metadata['start'])[0] & 0x80000000:
        raise ValueError('unexpected synthetic-test setup header')
    count = struct.unpack_from('<I', prefix, metadata['start'])[0]
    struct.pack_into('<I', prefix, metadata['start'], count + 1)
    prefix.extend(struct.pack('<IHHIII', 0xdeadbeef, 3, 0, 0, 4, 123))
    shifted = index_core(bytes(prefix) + data[metadata['end']:])
    shifted_map = next(x for x in shifted['sections'] if x['name'] == 'map')
    if shifted_map['start'] != original_map['start'] + 20 or shifted_map['sha256'] != original_map['sha256']:
        raise ValueError('metadata-size shift was not structurally followed')
    props = span('game_properties')
    replacement = struct.pack('<III', 1, 0x12345678, 5) + struct.pack('<IIII', 1, 0x87654321, 0x21000005, 1) + b'abcde'
    shifted = index_core(data[:props['start']] + replacement + data[props['end']:])
    shifted_map = next(x for x in shifted['sections'] if x['name'] == 'map')
    if shifted_map['start'] != original_map['start'] + len(replacement) - props['bytes'] or shifted_map['sha256'] != original_map['sha256']:
        raise ValueError('property-map-size shift was not structurally followed')
    envelope = span('map_components')
    prefix = bytearray(data[:envelope['end']])
    count = struct.unpack_from('<I', prefix, envelope['start'] + 4)[0]
    struct.pack_into('<I', prefix, envelope['start'] + 4, count + 1)
    name, opaque = b'UnknownFramedTest', b'no-gameplay-claim'
    prefix.extend(struct.pack('<I', len(name)) + name + struct.pack('<I', len(opaque)) + opaque)
    framed = index_core(bytes(prefix) + data[envelope['end']:])
    if not any(x['name'] == 'map_component/UnknownFramedTest' and x['bytes'] == len(opaque) for x in framed['sections']):
        raise ValueError('explicitly length-framed unknown component not indexed')
    rejected = []
    cases = [('truncated_metadata', data[:metadata['end']-1]),
             ('truncated_map', data[:original_map['end']-1]),
             ('unsupported_context', struct.pack('<I', 6) + data[4:])]
    for name, malformed in cases:
        try:
            index_core(malformed)
        except (ValueError, struct.error):
            rejected.append(name)
        else:
            raise ValueError('unsupported/truncated input accepted: ' + name)
    report = dict(schema='c6-linux-core-stream-layout-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        methods=[dict(module=m, address=hex(a), size=n, name=s, sha256=h) for m,a,n,s,h in METHODS],
        versions={n:v for n,_,v in versions},
        rules_virtuals=[dict(table=hex(t), slot=hex(s), target=hex(a)) for t,s,a in virtuals],
        sources_sha256={str(Path(__file__).relative_to(Path(__file__).resolve().parents[2])):sha(Path(__file__).read_bytes()),
                       'tools/linux/map_state_audit.py':sha(parser.read_bytes())},
        original_fixtures=fixtures,
        structural_tests=dict(metadata_growth_shift_bytes=20, property_map_growth=True,
            unknown_length_framed_component=True, rejected=rejected,
            synthetic_cases_are_not_original_native_fixtures=True),
        parser_requires_trace=False, parser_requires_fixture_offsets=False,
        parser_requires_known_stream_hash=False, parser_uses_byte_pattern_search=False,
        complete_map_index=True, first_player_formation_index=True,
        complete_unit_index=False, complete_formation_roster_index=False, complete_core_index=False,
        unit_predecessors_after_first_formations=['Goody_Hut', 'Bonuses', 'Congress', 'Cities',
            'Diplomacy', 'Districts', 'NameManager', 'Culture', 'Espionage', 'Religion',
            'Resources', 'Stats', 'Techs', 'Treasury'],
        unknown_bytes_masked=0, opaque_framed_payload_semantics_proven=False,
        full_world_import=False, native_execution=False, gameplay_parity=False)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS), original_fixtures=len(fixtures),
        complete_map_index=True, native_execution=False)))


if __name__ == '__main__':
    main()
