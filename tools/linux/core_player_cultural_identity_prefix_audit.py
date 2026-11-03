#!/usr/bin/env python3
"""Check byte-zero Rust Core/first-Player wire import; never execute original code."""
import argparse
import gzip
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from core_stream_index import Reader, index_core, read_stream, sha, METHODS as PREFIX_METHODS
from city_wire_audit import METHODS as CITY_METHODS
from player_components_audit import METHODS as PLAYER_METHODS
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from player_treasury_audit import METHODS as TREASURY_METHODS
from player_units_audit import METHODS as UNITS_METHODS
from player_eras_audit import METHODS as ERAS_METHODS
from player_trade_audit import METHODS as TRADE_METHODS
from player_great_people_points_audit import METHODS as GREAT_PEOPLE_POINTS_METHODS
from player_influence_audit import METHODS as INFLUENCE_METHODS
from player_quests_audit import METHODS as QUESTS_METHODS
from player_wmds_audit import METHODS as WMDS_METHODS
from player_improvements_audit import METHODS as IMPROVEMENTS_METHODS
from player_governors_audit import METHODS as GOVERNORS_METHODS
from player_cultural_identity_audit import METHODS as CULTURAL_IDENTITY_METHODS
from player_espionage_audit import METHODS as ESPIONAGE_METHODS
from player_religion_audit import METHODS as RELIGION_METHODS
from player_resources_audit import METHODS as RESOURCES_METHODS
from player_stats_audit import METHODS as STATS_METHODS
from tracked_value_audit import METHODS as TRACKED_METHODS
from player_techs_audit import METHODS as TECHS_METHODS
from player_name_manager_audit import METHODS as NAME_METHODS
from player_culture_audit import METHODS as CULTURE_METHODS
from player_header_control_audit import METHODS as HEADER_METHODS
from player_diplomacy_audit import METHODS as DIPLOMACY_METHODS
from player_diplomacy_blackboard_audit import METHODS as BLACKBOARD_METHODS
from player_districts_audit import METHODS as DISTRICT_METHODS, HOST_METHODS as DISTRICT_HOST_METHODS
ROOT = Path(__file__).resolve().parents[2]
TAIL_METHODS = [(44399056,
  231,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Cities const&)',
  '851d98eaead6cf9eeb90630105eee3f34304248fd01b2dbbfd6c0b49755d9720'),
 (44415040,
  374,
  'Serialization::OStream& Serialization::operator<< <GameCore::City::Instance, 8u, '
  '16u>(Serialization::OStream&, RecycledIDContainer<GameCore::City::Instance, 8u, 16u> const&)',
  '2819e602eddaaf3faebfcdc440a62e75550c30d02047c90cc976adb0b81a41bc'),
 (44426736,
  15,
  'FAutoVariable<Types::Vector<GameCore::BuildingTypes, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Cities>::save(Serialization::OStream&) const',
  '437d3dfd1d458cf0ec746a617925d09a4b316d9265aebe03196ba49545e0831d'),
 (44429344,
  91,
  'Serialization::OStream& Serialization::operator<< <GameCore::BuildingTypes, '
  'Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, Types::Vector<GameCore::BuildingTypes, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)',
  'b14438ba2c3861dc83eef37a9aa0caecd558027db941ccf83839a7274416006c'),
 (44430336,
  15,
  'FAutoVariable<eastl::vector<eastl::pair<GameCore::BuildingTypes, GameCore::BuildingTypes>, '
  'eastl::allocator>, GameCore::Player::Cities>::save(Serialization::OStream&) const',
  'f44b737b77ae09a4baa385b50947df2a01254520ebd9a4dd28164a28cf180586'),
 (44433680,
  91,
  'Serialization::OStream& Serialization::operator<< <eastl::pair<GameCore::BuildingTypes, '
  'GameCore::BuildingTypes> >(Serialization::OStream&, eastl::vector<eastl::pair<GameCore::BuildingTypes, '
  'GameCore::BuildingTypes>, eastl::allocator> const&)',
  'a6860b83b8d2cc6c6f20508907c0af6548f39043007a78a53a0da2547641aa1d'),
 (44433920,
  41,
  'Serialization::OStream& Serialization::operator<< <GameCore::BuildingTypes, '
  'GameCore::BuildingTypes>(Serialization::OStream&, eastl::pair<GameCore::BuildingTypes, '
  'GameCore::BuildingTypes> const&)',
  'd8e10e33aa9d40d7fcc1d9d3a3176621ccc77e8d1a4bd4fc1477e7272afbbaa2'),
 (44434944,
  15,
  'FAutoVariable<GameCore::ContinentTypes, GameCore::Player::Cities>::save(Serialization::OStream&) const',
  '2e2d3f31d2f94ecf4f59311aa8db2939d5f0f8f58b3a2c61c63b9c21daab97fc'),
 (44437600,
  15,
  'FAutoVariable<Types::Vector<GameCore::ContinentTypes, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Cities>::save(Serialization::OStream&) const',
  '20c52f5ff14b646509cc5497d0121e33e7c90834a1a1d3fc5a9177f94aa8daa3'),
 (44440512,
  91,
  'Serialization::OStream& Serialization::operator<< <GameCore::ContinentTypes, '
  'Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, Types::Vector<GameCore::ContinentTypes, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)',
  '6a27ea41bac074bd019d49f3c5ef3b441e409174d93d41ce460304d8c6fdc6cc'),
 (44441456,
  15,
  'FAutoVariable<int, GameCore::Player::Cities>::save(Serialization::OStream&) const',
  '78e44338c48bfe1c65dcc1ec38d07f000bcc5cf1f930be428d5ab3e6d9ea9b5f'),
 (44444064,
  15,
  'FAutoVariable<Types::Vector<GameCore::DistrictTypes, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Cities>::save(Serialization::OStream&) const',
  '801e6eccf726166ff5ebb5b1b70fd9dc4ab29a54e31ed1ed198b1bc3f25e8237'),
 (44446896,
  91,
  'Serialization::OStream& Serialization::operator<< <GameCore::DistrictTypes, '
  'Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, Types::Vector<GameCore::DistrictTypes, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)',
  'f9ab653cfaa61d00851893670ce87b3e835fe19b66b19335c45433bfea3b2770'),
 (44447872,
  15,
  'FAutoVariable<Types::Vector<GameCore::ProjectTypes, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::Player::Cities>::save(Serialization::OStream&) const',
  '0396602029440838254c96aaff4cb020978424c9c2a6fd4a7eb4e578d317b9e4'),
 (44450736,
  91,
  'Serialization::OStream& Serialization::operator<< <GameCore::ProjectTypes, '
  'Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, Types::Vector<GameCore::ProjectTypes, '
  'Types::LinearAlloc::Default<1u, 0u> > const&)',
  '3f1637e9d6801ee58ea926bc2c1103e0b1423f6a66b6c79cd8c8ee9a553c3c4f'),
 (47509456,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::ContinentTypes const&)',
  '1c85c3f68de14611703be986efc1bb71cba41343c9938d635db006ac2dd274c3')]
BINDINGS = [(0x3ad5e48,0x2a60600),(0x3ad5f08,0x2a61060),(0x3ad5fc8,0x2a61f70),
    (0x3ad5cc8,0x2a5e5f0),(0x3ad6088,0x2a629a0),(0x3ad6148,0x2a63880),(0x3ad5d88,0x2a5f400)]
FIXTURES = ['ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884',
    '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8']
TRACES = [('trace-benchmark-1.jsonl.gz','528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
    ('trace-captured-1.jsonl.gz','027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')]

def first_difference(a,b,path='projection'):
    if type(a)!=type(b):return path
    if isinstance(a,dict):
        if a.keys()!=b.keys():return path+'.keys'
        for key in a:
            if a[key]!=b[key]:return first_difference(a[key],b[key],path+'.'+key)
    elif isinstance(a,list):
        if len(a)!=len(b):return path+'.length'
        for i,(x,y) in enumerate(zip(a,b)):
            if x!=y:return first_difference(x,y,path+'['+str(i)+']')
    return path

def hashes(r):
    return [r.u32() for _ in range(r.count())]

def religion_record(r):
    r.version(7)
    v = dict(religion_hash=r.u32(),founder=r.take('i'),founded_turn=r.take('i'),
        holy_city=[r.take('i'),r.take('i')],custom_name=r.string8().hex(),belief_hashes=hashes(r),
        building_hashes=hashes(r),unit_hashes=hashes(r),anyone_condemns_favor=r.take('i'))
    for key in ['condemn_prohibited','auto_spreads_to_new_cities','awards_envoy']:
        v[key] = r.boolean()
    v['awards_envoy_religious_pressure'] = r.take('i')
    return v

def recycled(r, read):
    r.version(65537)
    if r.u32() != 16:
        raise ValueError('container index width differs')
    capacity,n = r.u32(),r.count()
    slots,records = [],[]
    for _ in range(n):
        slot = r.take('i');slots.append(slot);records.append(read(slot))
    current,free,head,tail = r.u32(),r.count(),r.u32(),r.u32()
    links = [r.u32() for _ in range(free)]
    return dict(capacity=capacity,current_id_bits=current,free_head=head,free_tail=tail,
        free_links=links,slots=slots),records

def independent(data, owners):
    indexed = index_core(data,first_player_components=True)
    map_end = next(x['end'] for x in indexed['sections'] if x['name']=='map')
    r = Reader(data);r.pos = map_end;r.version(7)
    religion = dict(minimum_faith_for_next_pantheon=r.take('i'),next_pantheon_founder=r.take('i'))
    religion['records'] = [religion_record(r) for _ in range(r.count())]
    r.version(1);players = r.count();first = None;layout = {}
    if players:
        requested = r.take('i');r.version(47)
        header = dict(owner=r.take('i'),starting_plot=[r.take('i'),r.take('i')],
            ordered_scalars=[r.take('i') for _ in range(15)],ordered_flags=[r.boolean() for _ in range(6)],
            handicap_hash=r.u32(),district_hash=r.u32(),project_hash=r.u32(),final_scalar=r.take('i'))
        r.version(1)
        def formation(slot):
            start = r.pos;r.version(1);identity = r.u32();r.raw(8);owner = r.take('i');r.raw(16)
            r.raw(r.count()*20);r.raw(8);r.raw(r.count()*4)
            return dict(slot=slot,id=identity,owner=owner,bytes=r.pos-start,sha256=sha(data[start:r.pos]))
        fm,formations = recycled(r,formation)
        r.goody_hut();r.bonuses();r.congress();r.version(15)
        city_bounds = {x['start']:x['end'] for x in owners if x['owner']=='GameCore::City::Instance'}
        def city(slot):
            start = r.pos
            if start not in city_bounds:
                raise ValueError('computed City start differs from independent native owner')
            h = r.city_header();end = city_bounds[start]
            if r.pos > end:
                raise ValueError('typed City header exceeds native owner')
            # Validation-only native cuts cross-check the already reviewed full
            # City codec. They are never provided to the library or candidate.
            r.pos = end
            return dict(slot=slot,id=h['id'],owner=h['owner'],location=[h['location_x'],h['location_y']],
                population=h['population'],bytes=end-start,sha256=sha(data[start:end]))
        cm,cities = recycled(r,city)
        fresh,urban,home = r.boolean(),r.take('i'),r.u32()
        layout['settled_count'] = r.pos
        settled = hashes(r)
        layout['settled_end'] = r.pos
        tail = dict(fresh_water_housing_bonus=fresh,urban_defenses=urban,
            home_continent_hash=home,settled_continent_hashes=settled,peak_cities=r.u32(),
            extra_starting_population_off_home_continent=r.take('i'),population_after_conquest=r.take('i'),
            banned_building_hashes=hashes(r),banned_district_hashes=hashes(r),
            banned_district_building_hashes=hashes(r),banned_project_hashes=hashes(r),
            building_build_overrides=[[r.u32(),r.u32()] for _ in range(r.count())])
        first = dict(requested_owner=requested,header=header,formation_container=fm,formations=formations,
            city_container=cm,cities=cities,city_tail=tail)
        if not any(x['owner']=='GameCore::Player::Cities' and x['end']==r.pos for x in owners):
            raise ValueError('computed complete Cities end differs from original owner')
        if not any(x['owner']=='GameCore::Player::Diplomacy' and x['start']==r.pos for x in owners):
            raise ValueError('computed stop differs from original Diplomacy start')
        # New component codecs have independent complete-record audits. Native
        # cuts are validation-only bounds, never passed to the Rust candidate.
        diplomacy_start = r.pos
        matches = [x for x in owners if x['owner']=='GameCore::Player::Diplomacy' and x['start']==r.pos]
        if len(matches)!=1:raise ValueError('Diplomacy native boundary missing')
        r.pos = matches[0]['end']
        first['diplomacy'] = dict(bytes=r.pos-diplomacy_start,sha256=sha(data[diplomacy_start:r.pos]))
        districts_start = r.pos;r.version(8)
        district_bounds = {x['start']:x['end'] for x in owners if x['owner']=='GameCore::District::Instance'}
        def district(slot):
            start = r.pos;r.version(15);identity = r.u32()
            location = [r.take('i'),r.take('i')];city_id = r.u32();kind = r.u32()
            r.raw(5*4);complete = r.boolean()
            if start not in district_bounds or r.pos>district_bounds[start]:raise ValueError('District native boundary differs')
            r.pos = district_bounds[start]
            return dict(slot=slot,id=identity,location=location,city_id=city_id,district_hash=kind,
                is_complete=complete,bytes=r.pos-start,sha256=sha(data[start:r.pos]))
        dm,districts = recycled(r,district)
        r.take('i');r.take('i')
        for _ in range(5):r.indexed_ints()
        # Every nested tail record is versioned and structurally decoded.
        for _ in range(r.count()):
            r.version(1);r.u32();r.take('i');r.take('i');r.boolean();r.boolean();r.u32()
        for _ in range(r.count()):r.version(1);r.u32();r.u32()
        for _ in range(r.count()):r.version(1);r.u32();r.u32();r.u32()
        for _ in range(2):
            for _ in range(r.count()):r.version(1);r.u32();r.u32()
        if not any(x['owner']=='GameCore::Player::Districts' and x['start']==districts_start and x['end']==r.pos for x in owners):
            raise ValueError('complete Districts envelope boundary differs')
        # This historical trace has no NameManager hook. The exact Player
        # writer establishes that next call; do not invent an observed entry.
        first['districts'] = dict(bytes=r.pos-districts_start,sha256=sha(data[districts_start:r.pos]),container=dm,records=districts)
        name_start = r.pos;r.version(2);outer,inner = r.count(),r.count()
        if outer!=inner:raise ValueError('NameManager vector/WriteArray counts differ')
        names = [r.take('h') for _ in range(inner)]
        first['name_manager'] = dict(bytes=r.pos-name_start,sha256=sha(data[name_start:r.pos]),citizen_name_usage_counts=names)
        culture_start = r.pos
        matches = [x for x in owners if x['owner']=='GameCore::Player::Culture' and x['start']==r.pos]
        if len(matches)!=1:raise ValueError('computed NameManager end differs from native Culture start')
        culture_end = matches[0]['end'];r.version(45);r.raw(5*4);r.boolean()
        def bool_pairs():return [[r.u32(),r.boolean()] for _ in range(r.count())]
        bool_pairs();r.indexed_ints();r.indexed_ints()
        for _ in range(5):bool_pairs()
        for _ in range(3):
            outer,inner = r.take('i'),r.count()
            if outer!=inner:raise ValueError('Culture government-slot double counts differ')
            for _ in range(inner):r.u32();r.take('i')
        acquired = bool_pairs()
        if r.pos>culture_end:raise ValueError('acquired civic prefix exceeds complete Culture')
        r.pos = culture_end
        first['culture'] = dict(bytes=r.pos-culture_start,sha256=sha(data[culture_start:r.pos]),has_civic=acquired)
        first['header']['saved_is_alive'] = first['header']['ordered_flags'][0]
        if not any(x['owner']=='GameCore::Player::Espionage' and x['start']==r.pos for x in owners):
            raise ValueError('computed stop differs from native Espionage start')
        for name,key in [('Espionage','espionage'),('Religion','religion'),('Resources','resources'),('Stats','stats'),('Techs','techs')]:
            start=r.pos;matches=[x for x in owners if x['owner']=='GameCore::Player::'+name and x['start']==start]
            if len(matches)!=1:raise ValueError('native component validation boundary missing: '+name)
            r.pos=matches[0]['end'];part=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]))
            if key=='resources':layout['resources_end']=r.pos
            if key=='techs':
                projection=Reader(data);projection.pos=start;projection.version(6)
                projection.raw(projection.count()*4);projection.raw(3*4)
                part['has_tech']=[[projection.u32(),projection.boolean()] for _ in range(projection.count())]
                if projection.pos>r.pos:raise ValueError('acquired Tech projection exceeds native component')
            first[key]=part
        if not any(x['owner']=='GameCore::Player::Treasury' and x['start']==r.pos for x in owners):
            raise ValueError('computed stop differs from native Treasury entry')
        start=r.pos;matches=[x for x in owners if x['owner']=='GameCore::Player::Treasury' and x['start']==start]
        if len(matches)!=1:raise ValueError('native Treasury validation boundary missing')
        r.pos=matches[0]['end'];first['treasury']=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]))
        start=r.pos;matches=[x for x in owners if x['owner']=='GameCore::Player::Units' and x['start']==start]
        if len(matches)!=1:raise ValueError('native Units validation boundary missing')
        units_end=matches[0]['end'];r.version(22)
        unit_bounds={x['start']:x['end'] for x in owners if x['owner']=='GameCore::Unit::Instance'}
        def unit_record(slot):
            begin=r.pos;r.version(28);identity=r.u32();r.u32();r.raw(4*4)  # AI type/controller + PlotCoord goal
            location=[r.take('i'),r.take('i')];r.take('i');owner=r.take('i')
            if begin not in unit_bounds or r.pos>unit_bounds[begin]:raise ValueError('nested Unit source boundary differs')
            r.pos=unit_bounds[begin]
            return dict(slot=slot,id=identity,owner=owner,location=location,bytes=r.pos-begin,sha256=sha(data[begin:r.pos]))
        um,unit_records=recycled(r,unit_record)
        if r.pos>units_end:raise ValueError('Unit table exceeds native complete envelope')
        # Full typed Units tail is independently audited; its native owner cut
        # cross-checks the production decoder, never drives it.
        r.pos=units_end;layout['units_end']=r.pos
        first['units']=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]),container=um,records=unit_records)
        start=r.pos;r.version(2);era,game_era=r.u32(),r.u32()
        first['eras']=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]),era_hash=era,game_era_hash=game_era)
        start=r.pos;matches=[x for x in owners if x['owner']=='GameCore::Player::Trade' and x['start']==start]
        if len(matches)!=1:raise ValueError('scalar Era bracket differs from native Trade start')
        r.version(35);trade_player=r.take('i');r.pos=matches[0]['end']
        first['trade']=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]),player=trade_player)
        for key, owner, version in [('great_people_points','GreatPeoplePoints',13),('influence','Influence',22),('quests','Quests',1)]:
            start=r.pos
            matches=[x for x in owners if x['owner']=='GameCore::Player::'+owner and x['start']==start]
            if len(matches)!=1:raise ValueError('native '+owner+' comparison cut missing')
            r.version(version);r.pos=matches[0]['end']
            first[key]=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]))
        start=r.pos;r.version(1);r.raw(r.count()*8)
        first['wmds']=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]))
        start=r.pos;r.version(1);r.raw(r.count()*4);r.u32()
        first['improvements']=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]))
        # The envelope is unhooked. The independently audited Governors codec
        # supplies its complete grammar; this fixture comparator uses the actual
        # following CulturalIdentity owner entry, never as a library input.
        start=r.pos;r.version(6)
        matches=[x for x in owners if x['owner']=='GameCore::Player::CulturalIdentity' and x['start']>=r.pos]
        if not matches:raise ValueError('native CulturalIdentity comparison entry missing')
        r.pos=min(x['start'] for x in matches)
        first['governors']=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]))
        start=r.pos;r.version(10);words=[r.take('i') for _ in range(14)]
        matches=[x for x in owners if x['owner']=='GameCore::Player::CulturalIdentity' and x['start']==start]
        if len(matches)!=1 or matches[0]['end']!=r.pos:raise ValueError('CulturalIdentity native boundary differs')
        first['cultural_identity']=dict(bytes=r.pos-start,sha256=sha(data[start:r.pos]),ordered_words=words)
    return dict(religion=religion,player_count=players,first_player=first),map_end,r.pos,layout

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--source-commit',help='Exact commit for isolated candidate build')
    p.add_argument('--repository',type=Path,help='Original repository used only to verify immutable source blobs')
    p.add_argument('--frozen-profile',type=Path,help='Replay an immutable historical candidate/report/source archive instead of rebuilding current code')
    args = p.parse_args()
    build_pin=None
    if not args.frozen_profile:
        if not args.source_commit or not args.repository or ROOT.resolve()==args.repository.resolve():
            raise ValueError('candidate build requires an isolated archive plus --source-commit and --repository')
        build_pin=subprocess.check_output(['git','-C',str(args.repository),'rev-parse','--verify',args.source_commit+'^{commit}'],text=True).strip()
    modules = dict(gamecore=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA),host=Elf(args.assets/'Civ6',HOST_SHA))
    methods = list(dict.fromkeys(PREFIX_METHODS + [('gamecore',*x) for x in CITY_METHODS+PLAYER_METHODS+TAIL_METHODS+DIPLOMACY_METHODS+DISTRICT_METHODS+NAME_METHODS+CULTURE_METHODS+HEADER_METHODS+ESPIONAGE_METHODS+RELIGION_METHODS+RESOURCES_METHODS+STATS_METHODS+TRACKED_METHODS+TECHS_METHODS+TREASURY_METHODS+UNITS_METHODS+ERAS_METHODS+TRADE_METHODS+GREAT_PEOPLE_POINTS_METHODS+INFLUENCE_METHODS+QUESTS_METHODS+WMDS_METHODS+IMPROVEMENTS_METHODS+GOVERNORS_METHODS+CULTURAL_IDENTITY_METHODS]+[('gamecore',*x) for x in BLACKBOARD_METHODS]+[('host',a,n,'District property provider',h) for a,n,h in DISTRICT_HOST_METHODS]))
    for module,a,n,name,digest in methods:
        if sha(modules[module].read(a,n)) != digest:
            raise ValueError('original method differs: '+name)
    for table,target in BINDINGS:
        if modules['gamecore'].relocated_pointer(table+16+0x28) != target:
            raise ValueError('Cities concrete enum/vector save binding differs')
    paths = [ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'crates/c6-core/Cargo.toml',ROOT/'crates/c6-data/Cargo.toml',
        ROOT/'crates/c6-differential/Cargo.toml',ROOT/'crates/c6-differential/src/bin/c6-core-player-cultural-identity-prefix-candidate.rs',
        *sorted((ROOT/'crates/c6-core/src').rglob('*.rs')),*sorted((ROOT/'crates/c6-data/src').rglob('*.rs'))]
    if args.frozen_profile:
        archived = json.loads((args.frozen_profile/'report.json').read_text())
        if archived['schema']!='c6-linux-rust-core-first-player-units-prefix-v1' or not archived['checks_passed']:
            raise ValueError('wrong historical profile')
        sources = archived['candidate_sources'];build_pin=archived['source_commit'];audit_sources=archived['audit_sources']
        for name,digest in sources.items():
            if sha((args.frozen_profile/'sources'/name).read_bytes())!=digest:
                raise ValueError('frozen source differs: '+name)
        candidate = args.frozen_profile/'candidate'
        if sha(candidate.read_bytes())!=archived['candidate_sha256']:
            raise ValueError('frozen candidate identity differs')
    else:
        sources = {str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in paths}
        # All actual Rust/Cargo inputs and Python validation logic come from one
        # committed archive. The original repository is read only.
        audit_paths=sorted((ROOT/'tools/linux').glob('*.py'))
        audit_sources={str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in audit_paths}
        for name,digest in {**sources,**audit_sources}.items():
            blob=subprocess.check_output(['git','-C',str(args.repository),'show',build_pin+':'+name])
            if sha(blob)!=digest:raise ValueError('isolated source differs from commit: '+name)
        subprocess.run(['cargo','build','--offline','-p','c6-differential','--bin','c6-core-player-cultural-identity-prefix-candidate'],cwd=ROOT,check=True)
        if sources != {str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in paths}:
            raise ValueError('sources changed during build')
        candidate = ROOT/'target/debug/c6-core-player-cultural-identity-prefix-candidate'
    cases = [];inputs = []
    with tempfile.TemporaryDirectory(prefix='c6-core-player-cultural-identity-prefix-') as directory:
        directory = Path(directory)
        def run(data,label):
            source,output = directory/(str(label)+'.core'),directory/(str(label)+'.prefix')
            source.write_bytes(data)
            actual = json.loads(subprocess.check_output([str(candidate),str(source),str(output)],text=True))
            if output.read_bytes()!=data[actual['map_end']:actual['consumed']]:
                raise ValueError('complete prefix roundtrip differs')
            return actual
        layouts = [];actuals = []
        for i,(identity,(trace_name,trace_digest)) in enumerate(zip(FIXTURES,TRACES)):
            data = read_stream(args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz'))
            if sha(data) != identity:
                raise ValueError('original Core identity differs')
            with gzip.open(args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name,'rb') as f:
                raw = f.read()
            if sha(raw) != trace_digest:
                raise ValueError('native owner evidence identity differs')
            owners = [json.loads(x) for x in raw.splitlines()]
            owners = [x for x in owners if x.get('event')=='owner' and x.get('save')==1]
            expected,map_end,end,layout = independent(data,owners)
            actual = run(data,i)
            if {k:actual[k] for k in expected} != expected:
                raise ValueError('independent typed projection differs at '+first_difference({k:actual[k] for k in expected},expected))
            if (actual['map_end'],actual['consumed']) != (map_end,end):
                raise ValueError('dynamic byte-zero prefix boundary or complete wire roundtrip differs')
            if not actual['exact_and_one_short_caps']:raise ValueError('native exact-cap checks absent')
            if not actual['tail_borrowed'] or actual['tail_bytes']!=len(data)-end or actual['tail_sha256']!=sha(data[end:]):
                raise ValueError('borrowed original tail differs')
            if actual['boundary']!='PlayerHeroes' or actual['unfinished_player_entries']!=expected['player_count']:
                raise ValueError('partial boundary differs')
            for key in ['complete_player_import','complete_core_import','gameplay_parity','metadata_compatibility_checked','rng_reconstructed','original_game_execution']:
                if actual[key]:raise ValueError('partial wire scope overclaimed')
            cases.append(dict(stream_sha256=identity,stream_bytes=len(data),trace_sha256=trace_digest,
                map_end=map_end,consumed=end,post_map_bytes=actual['post_map_bytes'],post_map_items=actual['post_map_items'],
                total_items=actual['items_used'],post_map_sha256=actual['post_map_sha256'],
                player_count=expected['player_count'],first_player_formations=len(expected['first_player']['formations']),
                first_player_cities=len(expected['first_player']['cities']),
                first_player_districts=len(expected['first_player']['districts']['records']),
                first_player_civic_pairs=len(expected['first_player']['culture']['has_civic']),
                first_player_tech_pairs=len(expected['first_player']['techs']['has_tech']),
                internal_resources_end=layout['resources_end'],internal_units_end=layout['units_end'],
                first_player_units=len(expected['first_player']['units']['records']),exact_and_one_short_caps=True,
                saved_is_alive=expected['first_player']['header']['saved_is_alive'],
                typed_projection_sha256=sha(json.dumps(expected,sort_keys=True,separators=(',',':')).encode()),
                tail_bytes=actual['tail_bytes'],tail_sha256=actual['tail_sha256'],tail_borrowed=True))
            inputs.append(data);layouts.append(layout);actuals.append(actual)
        data,baseline = inputs[0],actuals[0]
        indexed = index_core(data)
        metadata = next(x for x in indexed['sections'] if x['name']=='context_metadata')
        prefix = bytearray(data[:metadata['end']])
        count = struct.unpack_from('<I',prefix,metadata['start'])[0]
        if count & 0x80000000:raise ValueError('unexpected synthetic metadata carrier')
        struct.pack_into('<I',prefix,metadata['start'],count+1)
        prefix.extend(struct.pack('<IHHIII',0xfffffffe,3,0,0,0,123))
        moved = run(bytes(prefix)+data[metadata['end']:],'metadata-shift')
        if moved['consumed']!=baseline['consumed']+20 or moved['map_end']!=baseline['map_end']+20 or moved['post_map_sha256']!=baseline['post_map_sha256'] or moved['tail_sha256']!=baseline['tail_sha256']:
            raise ValueError('whole-input metadata relocation differs')
        layout = layouts[0];altered = bytearray(data[:layout['settled_end']])
        count = struct.unpack_from('<I',altered,layout['settled_count'])[0]
        struct.pack_into('<I',altered,layout['settled_count'],count+1)
        altered.extend(struct.pack('<I',0xdeadbeef));altered.extend(data[layout['settled_end']:])
        moved = run(altered,'city-tail-shift')
        if moved['consumed']!=baseline['consumed']+4 or moved['map_end']!=baseline['map_end'] or moved['tail_sha256']!=baseline['tail_sha256'] or moved['first_player']['city_tail']['settled_continent_hashes'][-1]!=0xdeadbeef or moved['first_player']['cities']!=baseline['first_player']['cities']:
            raise ValueError('counted Cities tail relocation differs')
    report = dict(schema='c6-linux-rust-core-first-player-units-prefix-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
        methods=[dict(module=m,address=hex(a),size=n,name=name,sha256=d) for m,a,n,name,d in methods],
        save_bindings=[dict(table=hex(t),slot='0x28',target=hex(a)) for t,a in BINDINGS],
        source_commit=build_pin,isolated_committed_source_verified=True,audit_sources=audit_sources,
        frozen_candidate_replay=bool(args.frozen_profile),
        candidate_sha256=sha(candidate.read_bytes()),candidate_sources=sources,audit_source_sha256=sha(Path(__file__).read_bytes()),
        independent_indexer_sha256=sha((ROOT/'tools/linux/core_stream_index.py').read_bytes()),fixtures=cases,
        raw_stream_byte_differences=sum(a!=b for a,b in zip(*inputs))+abs(len(inputs[0])-len(inputs[1])),unknown_bytes_masked=0,
        source_only_relocations=dict(metadata_extra_bytes=20,settled_continent_extra_bytes=4,passed=True),
        direct_rust_byte0_parser=True,python_offsets_required_by_library=False,native_traces_required_by_library=False,
        native_city_owner_cuts_used_only_for_independent_validation=True,
        unhooked_outer_components_compared_by_source_order_and_adjacent_cuts=["WMDs","Improvements","Governors"],next_unparsed_section='Player.Heroes',
        next_schema_proven_by_writer_order=True,native_next_schema_entry_observed=False,
        complete_player_import=False,complete_core_import=False,metadata_compatibility_checked=False,
        rng_reconstructed=False,gameplay_parity=False,original_game_execution=False,offline_rust_candidate_execution=True)
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,original_raw_streams=len(cases),consumed=cases[0]['consumed'],
        first_player_cities=cases[0]['first_player_cities'],first_player_formations=cases[0]['first_player_formations'],
        raw_stream_byte_differences=report['raw_stream_byte_differences'],original_game_execution=False)))
if __name__=='__main__':main()
