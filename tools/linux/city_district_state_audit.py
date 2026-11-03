#!/usr/bin/env python3
"""Read-only exact Linux City/District movement registry contract; no game execution."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [(35026096,
  23,
  'GameCore::Map::Plot::CityAttributes::GetDistrict() const',
  'ece97c1bdc8cd9379b256fd1aa042223588b76c87f9d69a5453181140796b248'),
 (33432672,
  23,
  'GameCore::Map::Plot::CityAttributes::GetOwningCity() const',
  '8afcd0793002a2f5984462ec24c87fb2e553fd92fef70b28ed42bed2d3199d98'),
 (34878560,
  23,
  'GameCore::Map::Plot::CityAttributes::GetWorkingCity() const',
  '7ec4115274085cf2fd24148f5e5b5782ed15101e42e038d8fdd0854a4c76aab3'),
 (33571104,
  5,
  'GameCore::Map::Plot::CityAttributes::GetPlayer() const',
  '5358f81f711f3168f8bd75591000cf17b4eceda6ba11925083fa88182fda3867'),
 (41663472,
  32,
  'GameCore::Map::Plot::CityAttributes::CityAttributes()',
  '678fcdd56f78ea2f47ce6b9d44f35e598b66e1072bd4b45a808bec4629268595'),
 (41663520,
  39,
  'GameCore::Map::Plot::CityAttributes::Initialize()',
  '8c63b3a6c242a1a59e3eecf7042bff0a94e03e0ba5d9c508ffe948370bc8799d'),
 (45908016,
  89,
  'GameCore::Map::Plot::CityAttributes::Write(Serialization::OStream&) const',
  'e2637861638ec67c101276ba571c635f98caa194836029752d2ed212550b507a'),
 (45908112,
  91,
  'GameCore::Map::Plot::CityAttributes::Read(Serialization::IStream&, int)',
  '4735021680e42fe397e22a42fcde4e37114f3a69045747faae2e7a3e29378f77'),
 (45908208,
  16,
  'GameCore::Map::Plot::CityAttributes::Update(GameCore::Map::Plot::Reflection::CityAttributes1 const&)',
  'fd6b61eee446ebca204402ca257237ea444ee42b87e9f4b2076d46c9e38b4cd5'),
 (45908336,
  50,
  'GameCore::Map::Plot::CityAttributes::IsCity() const',
  'ece6fcce1b5e82142d85e93b9298936a6dc23473380bb7b769dcdcf4b030f9be'),
 (45908400,
  74,
  'GameCore::Map::Plot::CityAttributes::GetDistrictType() const',
  'f147b13b9c923cb7774f9f0ebe3858ef019da7c5ce7b4609fbaf6392251ea435'),
 (45908480,
  71,
  'GameCore::Map::Plot::CityAttributes::IsDistrictComplete() const',
  'f39bdadcfd60d365e1f4d7f4329fb250774fd1be088cdb4786c0becc43da2a3a'),
 (45908752,
  8,
  'GameCore::Map::Plot::CityAttributes::GetCity(GameCore::Player::CityID) const',
  '7d7ecf22e246b5e8c6db17f7bef9c58503b4d34ed920241b8aac33f7b84664a3'),
 (45908768,
  8,
  'GameCore::Map::Plot::CityAttributes::GetDistrict(GameCore::Player::DistrictID) const',
  'e260cc42d331ac28d051f92f0f26d3c0a79ae6d9eacecbdb85efe784ca1cfdd0'),
 (45908784,
  10,
  'GameCore::Map::Plot::CityAttributes::IsDomainValid(GameCore::DomainMask) const',
  '963df09b8cfda69b159a2cc8fd0c42d7339aa94e2306d98ac04c8cf3264a0d45'),
 (45908816,
  20,
  'GameCore::Map::Plot::CityAttributes::IsDomainValid(GameCore::DomainTypes) const',
  'aa94b9e935091bc509d768556124eacb14865e3338f42fd71f2db093a7fe8169'),
 (45908848,
  25,
  'GameCore::Map::Plot::CityAttributes::GetDomainMask(GameCore::DomainTypes) const',
  '16b028f1d852ff0059b080d174fd3453e30ab797f1984112b533acef3461a6be'),
 (45908880,
  23,
  'GameCore::Map::Plot::CityAttributes::SetDomainValid(GameCore::DomainMask, bool)',
  '586928f865bc6c6950507556026a70982e878067b2fc6717804de6f330f7f067'),
 (42392576,
  57,
  'GameCore::City::CManager::GetCity(GameCore::Player::ComponentID const&)',
  '5b7fb30d1c1ed64fa169933871cb06809ecf1f1b38a261a78bea94956b8fb281'),
 (42392640,
  60,
  'GameCore::City::CManager::GetCity(GameCore::PlayerTypes, unsigned int)',
  'dc94e53ae027e4fbbc2ebceb12ba12939c76adbe5faeaf3b5ce0e6e4362939b6'),
 (42393776,
  93,
  'GameCore::City::CManager::GetDistrict(GameCore::Player::ComponentID const&)',
  'c1691db37b45fa3441151d0fc7494058a260d898fcba8ac052721df8728f27f2'),
 (42393872,
  60,
  'GameCore::City::CManager::GetDistrict(GameCore::PlayerTypes, unsigned int)',
  'ad8229e662d7953d567fe4702b559bbab0214bf2942609462b3762a9875f66ce'),
 (34138592,
  21,
  'GameCore::Player::ComponentID::IsValid() const',
  '6b7ab1333e17d6c73ef1445fc18db84c1ff4ecc644a31115c46534e071f1d771'),
 (33441552,
  11,
  'GameCore::Player::ComponentID::ComponentID(GameCore::PlayerTypes, unsigned int, '
  'GameCore::Player::ComponentID::Type)',
  'cb4c434e7692669068d42f77251a91039bc80797654608f3bdbd62f534f7478f'),
 (34009840,
  10,
  'GameCore::Player::DistrictID::DistrictID(GameCore::PlayerTypes, unsigned int)',
  'e789b6ceb72594da7166287a7547e312502da72175aae92b322520ef3eb69512'),
 (33450784,
  10,
  'GameCore::Player::CityID::CityID(GameCore::PlayerTypes, unsigned int)',
  '6b39d991e8ef1a8dfa4511ac0174b83e2932bac35d346fbb0007dcbdb3f85edc'),
 (33569088,
  39,
  'GameCore::Player::Manager::GetValidPlayer(GameCore::PlayerTypes) const',
  '58bb7297d06455fd64617c3472f7087054b4aa5fa1b28ab20ce94cb74db41648'),
 (33574848,
  51,
  'GameCore::Player::Manager::IsValid(GameCore::PlayerTypes) const',
  '31c2b843f442ca71b8fd27edeccb2888940a331a9f50686c2a1e078cd25be038'),
 (33574912,
  7,
  'GameCore::Player::Manager::IsValidRange(GameCore::PlayerTypes) const',
  '1bda6fddb3180619888eb42cbb75a05d0877b5498c502ebb72ec289b4e0ba7a1'),
 (44406752,
  12,
  'GameCore::Player::Cities::Find(unsigned int) const',
  'b22cb52aa75d16eb1e1f3e664d1ff67096a8a3683c2c7f2d54c009fa22a3e7a6'),
 (44962528,
  12,
  'GameCore::Player::Districts::Find(unsigned int) const',
  'a460913eb67325a8287de3200d8f7ffbafa73204508ea8b9e7ee045a2d34d50f'),
 (44420240,
  84,
  'RecycledIDContainer<GameCore::City::Instance, 8u, 16u>::Find(unsigned int) const',
  '3e400de7612ac963b66e6e0630fd735acbe9c1f2588f74c1f26df97c50df008d'),
 (44974736,
  84,
  'RecycledIDContainer<GameCore::District::Instance, 8u, 16u>::Find(unsigned int) const',
  '851cd5df1a54fadad43fca32535d1e082d5159169247e367345f165b5a4c4419'),
 (44973488,
  343,
  'RecycledIDContainer<GameCore::District::Instance, 8u, 16u>::Add(GameCore::District::Instance**)',
  '3acd06f60f87ad028e90a4b22be172c8cde2627bc9caa290c7c5ed6ac5d0ccfa'),
 (44418624,
  343,
  'RecycledIDContainer<GameCore::City::Instance, 8u, 16u>::Add(GameCore::City::Instance**)',
  '144990a55a4141308d095802b2be86a5d7ff6c56a06abbf6a853bb418dd7447a'),
 (33432064,
  26,
  'GameCore::City::IInstance::GetComponentID() const',
  'daa8152880488f56c4abaf01bd542858aa6f35fd0a18ebec401d9073ba78057a'),
 (33432208,
  7,
  'GameCore::City::IInstance::GetOwner() const',
  'e997d06cf0d3f9b3bb89701ec15a7b1b096198fdc4b0dc87fc0ca29c37f6dadc'),
 (33433376,
  7,
  'GameCore::City::IInstance::GetID() const',
  '17b3e45097637e3e3e83c8af90d6981e0756c7452f3b935a52116daf90fb1292'),
 (43111344,
  53,
  'GameCore::District::Instance::GetComponentID() const',
  '375fcfbe3325e22c56a6e49fa6540a14e874f5b423b91596f339746d9102dd58'),
 (43116096,
  47,
  'GameCore::District::Instance::GetPlot() const',
  '18129455757d4e5368269a4ea7c6c6fb0522cd1dec354341fe9980bc37ebfef4'),
 (43116144,
  15,
  'GameCore::District::Instance::GetOwner() const',
  '258e9059f2f83cb3fb38cf5cf84a3e40e613ec752b3ef4c50cc9b48d1227e875'),
 (43116272,
  56,
  'GameCore::District::Instance::GetCity() const',
  'bab5f1e6454974f4f3dcc82cf0205c5254d33535fc7919653d52e529d6facaf6'),
 (43116336,
  7,
  'GameCore::District::Instance::GetCityID() const',
  'e997d06cf0d3f9b3bb89701ec15a7b1b096198fdc4b0dc87fc0ca29c37f6dadc'),
 (35272384,
  7,
  'GameCore::District::Instance::GetID() const',
  '93ff8d41c83ba5892a73273b9f5771ba588ff8ac1b171055e62bf57aff7a1eae'),
 (33445840,
  7,
  'GameCore::District::Instance::GetType() const',
  'ebf9ec5205f71d01bc09c02b78f779e470aced6d0adc6824d92a4d9fe716f035'),
 (33432752,
  7,
  'GameCore::District::Instance::IsComplete() const',
  '04de6938c59bb5009eaa15a67bf3dad4db82acf731c120a7b6fb5ba3ce334419'),
 (43117360,
  63,
  'GameCore::District::Instance::GetTypeHash() const',
  '7b40c5962121ed6c9611b6231929b1f15f2cc57acb0831e95bc874b32879fa3e'),
 (43114528,
  731,
  'GameCore::District::Instance::Initialize()',
  '6f55421231001a52e01bcc5519fcf18c081b8a78ade523c407de1c989957eee5'),
 (43116416,
  927,
  'GameCore::District::Instance::SetComplete(bool, bool)',
  'ae141d44de082967aeb320f105385e32a908d708878d89f35cdb2ec2dc192ae1'),
 (43109744,
  487,
  'Serialization::operator<<(Serialization::OStream&, GameCore::District::Instance const&)',
  'c86a88b70f37fbf88c31c4be0f5fc1e84a1c9d929be471a219ce3985a6698149'),
 (43110240,
  1103,
  'Serialization::operator>>(Serialization::IStream&, GameCore::District::Instance&)',
  '8946937f50875d90d39f6bd7b6c15a9960da7e1e81fbd09aac06b698569360b4'),
 (42536752,
  1159,
  'Serialization::operator<<(Serialization::OStream&, GameCore::City::Instance const&)',
  '9c4c4675b1773116af80f0fe81053cbe2948f8e729c379a6a42e521185d68650'),
 (42537920,
  3059,
  'Serialization::operator>>(Serialization::IStream&, GameCore::City::Instance&)',
  '209dea94038e9b8d301adc1eb27d763388b2ea6653fa2e84d0bb07cc4006adfd'),
 (43135808,
  15,
  'FAutoVariable<GameCore::DistrictTypes, GameCore::District::Instance>::save(Serialization::OStream&) const',
  '16f9ac7ad8b1d687fb0d10603637640aca8fd7de48ef0c77d1add392f0717510'),
 (43135744,
  15,
  'FAutoVariable<GameCore::DistrictTypes, GameCore::District::Instance>::load(Serialization::IStream&)',
  '3eb116914c84c8491eafd6fdf15adecaa3f54a85b9d2f7fbb2b6c766918aed8a'),
 (47508448,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::DistrictTypes const&)',
  'dc40e9bc43e0fc8aec2e195881b9a1c8eab8c366272b24f46990d7fb1231425c'),
 (47508496,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::DistrictTypes&)',
  '8f2608b388942db0f3d64ebc8e0e8f9cedd91acf351ae7e3d5cbdf710883f92d'),
 (45918080,
  12,
  'GameCore::Map::Plot::Instance::GetCityAttributes() const',
  'e1a598f85f49a28e29144ddf86739e2601b0294590320845586249ce08532b8a'),
 (45918128,
  9,
  'GameCore::Map::Plot::Instance::HasCityAttributes() const',
  '465525d3e6485a0fa870265793fdd0e49577850a4fb46c4514eb2bb42fd126d2'),
 (44311808,
  50,
  'GameCore::Map::CityPlots::GetAttributes(unsigned int)',
  'd9b066e28f35789167afe32cdf72cfc7287e0678e5b4f1f5324427a3260b862d'),
 (33432528,
  41,
  'GameCore::Map::Plot::Instance::GetOwner() const',
  'd40d1bd008ab26ed6eac1cc16bfd19ecf84925b3ce6bf6ef79d1d97e1520ce05'),
 (44309008,
  111,
  'GameCore::Map::CityPlots::RestoreValidDomainData()',
  'a81bc7e65ff09e78b7fdbc341764f99184940739215e444f62b7a1510396927a'),
 (44309120,
  178,
  'GameCore::Map::CityPlots::SetValidDomains(GameCore::Map::Plot::Instance*)',
  '2a4b78123d7faffdefd2c0e7c1841ff9a72ef95454eb2b4859482ac9e388e74b'),
 (44309312,
  100,
  'GameCore::Map::CityPlots::IsValidForDomain(GameCore::Map::Plot::Instance const*, GameCore::DomainTypes) '
  'const',
  '9718c9e103997e5f4518fa18acef51f56bdedd06f1434e325ea4b003c083f3c6'),
 (47688352,
  2540,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Game::Instance&)',
  '917d430f7b9c105932f394d62b2b75c83f698437ba89e7ce0ece979dad395873'),
 (45966688,
  165,
  'GameCore::Rules::Cities::Instance::MarkAdjacentCitiesNavigable(GameCore::Map::Plot::Instance*)',
  '0e55899eac1833b3ebe4a697dfed0ad27de88703e308951e9348b166c557a5ec')]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def district_projection(data):
    """A prefix of an independently owner-bounded District v15, not a full codec."""
    if len(data) < 45:
        raise ValueError('truncated District projection')
    version, identity, x, y, city, type_hash = struct.unpack_from('<IIiiII', data)
    if version != 15 or data[44] not in (0, 1):
        raise ValueError('unsupported District prefix version/complete byte')
    return dict(id=identity, location=[x,y], city_id=city, type_hash=type_hash, complete=bool(data[44]))


def city_projection(data):
    """A prefix of an independently owner-bounded City v51, not a full codec."""
    if len(data) < 20:
        raise ValueError('truncated City projection')
    version, identity, x, y, owner = struct.unpack_from('<IIiii', data)
    if version != 51:
        raise ValueError('unsupported City prefix version')
    return dict(id=identity, location=[x,y], owner=owner)


def slot_lookup(owner, identity, valid_players, records):
    # Exact lookup semantics: no ComponentID type test or high16 generation check.
    if owner == -1 or identity == 0xffffffff or not 0 <= owner < 65 or owner not in valid_players:
        return None
    return records.get((owner, identity & 0xffff))


def domain_valid(bits, domain):
    mask = (1,2,4,8,16)[domain] if 0 <= domain < 5 else 8
    signed = bits if bits < 0x8000 else bits - 0x10000
    return signed & mask != 0


def fixtures(private):
    from core_stream_index import index_core, read_stream
    from map_state_audit import Reader
    output = []
    canonical = private / 'evidence/linux-game-load/canonical'
    traces = private / 'evidence/linux-save-normalization/checkpoint-203-owners'
    inputs = [
        ('ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884',
         'trace-benchmark-1.jsonl.gz','528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
        ('85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8',
         'trace-captured-1.jsonl.gz','027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')]
    for identity, trace_name, trace_digest in inputs:
        data = read_stream(canonical / (identity + '.bin.gz'))
        if sha(data) != identity:
            raise ValueError('original Core fixture differs')
        with gzip.open(traces / trace_name, 'rb') as f:
            trace = f.read()
        if sha(trace) != trace_digest:
            raise ValueError('original owner trace differs')
        rows = [json.loads(line) for line in trace.splitlines()]
        rows = [x for x in rows if x.get('event') == 'owner' and x.get('save') == 1]
        indexed = index_core(data)
        span = next(s for s in indexed['sections'] if s['name'] == 'map')
        reader = Reader(data, span['start'], span['end'])
        world = reader.map()
        if reader.pos != span['end']:
            raise ValueError('Map not exhausted')
        width, height = world['header']['width'], world['header']['height']
        def plot(location):
            x,y = location
            if not 0 <= x < width or not 0 <= y < height:
                raise ValueError('observed object location outside Map')
            return world['plots'][y*width+x]
        cities, districts = [], []
        for row in rows:
            if not 0 <= row['start'] < row['end'] <= len(data):
                raise ValueError('invalid independently attributed owner span')
            payload = data[row['start']:row['end']]
            if row['owner'] == 'GameCore::City::Instance':
                cities.append(city_projection(payload))
            elif row['owner'] == 'GameCore::District::Instance':
                d = district_projection(payload)
                attributes = plot(d['location'])['city']
                d['owner'] = attributes['owner'] if attributes else -1
                districts.append(d)
        if len(cities) != 245 or len(districts) != 1284:
            raise ValueError('observed registry fixture cardinality differs')
        city_registry = {(c['owner'],c['id']&0xffff):c for c in cities}
        district_registry = {(d['owner'],d['id']&0xffff):d for d in districts}
        if len(city_registry) != len(cities) or len(district_registry) != len(districts):
            raise ValueError('ambiguous observed owner/slot registry')
        players = {c['owner'] for c in cities} | {d['owner'] for d in districts}
        stats = Counter()
        for p in world['plots']:
            a = p['city']
            if not a:
                continue
            stats['attributes'] += 1
            for name in ('working','owning'):
                if a[name] != 0xffffffff:
                    found = slot_lookup(a['owner'],a[name],players,city_registry)
                    stats[name + ('_resolved' if found else '_unresolved')] += 1
                    if found and found['id'] != a[name]:
                        stats[name+'_requested_actual_id_differ'] += 1
            if a['district'] != 0xffffffff:
                found = slot_lookup(a['owner'],a['district'],players,district_registry)
                stats['district_resolved' if found else 'district_unresolved'] += 1
                if found:
                    stats['district_complete' if found['complete'] else 'district_incomplete'] += 1
                    stats['is_city_type_hash'] += found['type_hash'] == 0x8a32007d
                    if found['id'] != a['district']:
                        stats['district_requested_actual_id_differ'] += 1
        projection = json.dumps(dict(cities=cities,districts=districts),sort_keys=True,separators=(',',':')).encode()
        output.append(dict(stream_sha256=identity, owner_trace_sha256=trace_digest,
            city_records=len(cities), district_records=len(districts),
            serialized_projection_sha256=sha(projection), map_links=dict(sorted(stats.items())),
            runtime_type_ordinals_observed=False, runtime_valid_domains_observed=False,
            complete_city_or_district_state_decoded=False,
            owner_trace_used_for_record_bounds_only=True,
            city_type_comparison_scope='serialized type hash projection; native runtime definition ordinal still needs export'))
    return output

BYTES = [
    (0x2bc8194,'3d7d00328a','IsCity compares district definition City Center hash'),
    (0x2bc81eb,'b8ffffffff','missing District returns NO_DISTRICT'),
    (0x2bc823b,'31c0','missing District returns incomplete'),
    (0x2ae42a1,'440fb7fd4489fbc1eb03','District lookup low16 then block index'),
    (0x2a5cca1,'440fb7fd4489fbc1eb03','City lookup low16 then block index'),
    (0x2ae3eea,'b8000001000343288943284409e0','District Add counter plus 65536 OR slot'),
    (0x2a5c77a,'b8000001000343288943284409e0','City Add counter plus 65536 OR slot'),
    (0x2005000,'83fe410f92c0c3','unsigned player range is below65'),
    (0x27bbc40,'66c747140000','CityAttributes Initialize clears omitted domain mask'),
    (0x291ce0e,'488db3f00300004c89f7e813635eff','District Write serializes completion wrapper'),
    (0x2a41a93,'6641c746140000','SetValidDomains first clears runtime mask'),
]


# Core compatibility metadata is not a definition-index registry.
METHODS += [(43040224, 286, 'GameCore::Context::Instance::MakeMetaData', '14a02ebd6f619fd85ed06f5b3d86b8f07f2e9021296900fbdb53e2074564fc9a'), (43040512, 246, 'GameCore::Context::Instance::ReadMetaData', '4362a58cea537d57fcd8815e6b1071c065278365709672169640b0a393ed9d29'), (47399552, 12, 'GameCore::IndexSerializer::SaveVersion', '49010e3b612c9b4de140c013c3e725ff3a66df236b02af883f40d88ea071c961'), (47399568, 12, 'GameCore::IndexSerializer::LoadVersion', '814a4c9eb1631167536e0a585e613e070b286b3815881d2b9b01950a1ab2df61')]

# Concrete District/Building serialized hash remapping providers.
METHODS += [(33536304, 34, 'int Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Building> >(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Building> const&)', 'aef8a22d38a41aa430f31d3a7b9315abb5de415033479baf1722557685f5d707'), (33536976, 34, 'int Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> >(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District> const&)', 'd33e4a5fb4dadf218364b8a20da36838ab796c11f42bc2b47327f41c0ff5a465'), (33536352, 109, 'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Building>::Find(unsigned int) const', '81c8f59f047a39dbd88358cb829ef615f0fb9db2c737051e03bbf333f5a3ba06'), (33537024, 109, 'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::District>::Find(unsigned int) const', 'b720a1edc9281bc19df2a5fa2d1db43b2e3c09493372d0f970d1fe598f6181d8'), (33344608, 4, 'GameCore::Definition::Building::GetCollectionIndex() const', 'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a'), (33344752, 4, 'GameCore::Definition::District::GetCollectionIndex() const', 'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a')]

BYTES += [
    (0x2d7b13d,'e85e770fff','Canal Wonder uses original Building Find after packed type narrowing'),
    (0x2d7b1eb,'e8b0760fff','Bridge independently uses original Building Find'),
    (0x2d7b12a,'83fdff','Only Canal Wonder prechecks packed type != -1'),
]

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output',type=Path,required=True)
    args = p.parse_args()
    module = args.assets/'libGameCore_XP2.so'
    game = Elf(module,GAME_SHA)
    for address,size,name,expected in METHODS:
        if sha(game.read(address,size)) != expected:
            raise ValueError('native method differs: '+name)
    for address,expected,name in BYTES:
        if game.read(address,len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('native branch/field differs: '+name)
    domains = struct.unpack('<5i',game.read(0x2f6b180,20))
    if domains != (1,2,4,8,16):
        raise ValueError('native DomainTypes mask mapping differs')
    branch_table = 0x2f55f9c
    branches = [branch_table+d for d in struct.unpack('<5i',game.read(branch_table,20))]
    if branches != [0x2a41b60,0x2a41b9a,0x2a41b8d,0x2a41b89,0x2a41b9a]:
        raise ValueError('native base domain rules differ')
    for address,expected in [(0x2f30b08,15),(0x2f22c60,51)]:
        if struct.unpack('<I',game.read(address,4))[0] != expected:
            raise ValueError('City/District writer version differs')
    slots = [(0x3ae2350,0,0x2bc8310),(0x3ae2350,8,0x2bc8320),
             (0x3ac5108,0x28,0x2923340),(0x3ac5108,0x10,0x2923300)]
    for table,slot,target in slots:
        if game.relocated_pointer(table+16+slot) != target:
            raise ValueError('native concrete provider differs')
    dwarf = subprocess.check_output(['gdb','-nx','-batch',str(module),
        '-ex',"ptype /o 'GameCore::Map::Plot::CityAttributes'",
        '-ex',"ptype /o 'GameCore::Player::ComponentID'",
        '-ex',"ptype 'GameCore::DomainTypes'",
        '-ex',"ptype 'GameCore::DomainMask'"],text=True)
    for token in ('/*     20      |       2 */','m_validDomains','total size (bytes):   32',
                  'uint64 m_bits','DOMAIN_SEA','DOMAIN_MASK_HOVER = 16'):
        if token not in dwarf:
            raise ValueError('matching target DWARF differs: '+token)
    record = dict(id=0x12340007,type_index=3,complete=False)
    records = {(0,7):record,(64,7):record}
    if slot_lookup(0,0x56780007,{0,64},records) is not record:
        raise ValueError('generation-alias contract test failed')
    for owner,identity in [(-1,7),(65,7),(0,0xffffffff),(1,7)]:
        if slot_lookup(owner,identity,{0,64},records) is not None:
            raise ValueError('missing/invalid registry contract test failed')
    if slot_lookup(64,7,{0,64},records) is not record:
        raise ValueError('player64 registry test failed')
    if not domain_valid(8,-1) or domain_valid(0,0) or not all(domain_valid(31,d) for d in range(5)):
        raise ValueError('native domain-mask contract test failed')
    observed = fixtures(args.private_root)
    report = dict(schema='c6-linux-city-district-movement-state-v1',checks_passed=True,
        gamecore_sha256=GAME_SHA,
        production_registry=dict(
            source='crates/c6-core/src/city_district.rs',
            source_sha256=sha((ROOT/'crates/c6-core/src/city_district.rs').read_bytes()),
            lookup='borrowed sorted native low16 slots; preserve requested/resolved IDs and actual owner',
            unavailable_distinct_from_complete_missing=True,
            movement_runtime_adapter=True,
            initialization_rebuild='base then XP2 augmentation, plot order; bounded fallible mask allocation',
            external_rebuild_dependencies=['IsWater','IsCoastalLand(1)','IsWonderComplete','original directional neighbors'],
            later_incremental_updates_implemented=False,
            complete_city_district_codec=False,native_comparison=False),
        core_prefix_metadata=dict(definition_enumeration_mappings=False,
            metadata='two numeric compatibility values and one GUID string',
            keys=['fd1bbf76','4e33fe49','f4e540d8'],
            index_serializer_version=1,index_version_payload='one i32; not a mapping table',
            required_definition_identity='actual runtime registry ordinal plus stable definition hash'),
        methods=[dict(address=hex(a),size=n,name=s,sha256=h)for a,n,s,h in METHODS],
        byte_contracts=[dict(address=hex(a),contract=s)for a,_,s in BYTES],
        virtual_bindings=[dict(table=hex(t),slot=hex(s),target=hex(a))for t,s,a in slots],
        identity=dict(component_id_bytes=8,component_id_alignment=8,owner_signed16=True,
            city_tag=2,district_tag=3,invalid_id='0xffffffff',
            generic_validity='owner16 != 0xffff and id32 != 0xffffffff; no component tag test',
            player_validity='unsigned owner<65 and actual manager mapping[owner]!=-1',
            container_key='owner plus (requested_id & 0xffff)',block_index='slot >>3',block_element='slot &7',
            generation_match_checked_by_Find=False,
            allocation='wrapping u32 counter+=0x10000; returned_id=counter OR allocated_slot',
            retain_full_actual_id_and_allocator_metadata=True),
        city_attributes=dict(live_bytes=32,wire_bytes=17,wire=['working u32','owning u32','district u32','player i8','wonder definition hash u32'],
            field_offsets=dict(working=8,owning=12,district=16,valid_domains=20,plot_yield_effect=24,player=28,wonder=30),
            valid_domains_wire=False,plot_yield_effect_wire=False,
            absent_attribute_index=0,invalid_attribute_index_reads_default_index0=True),
        district=dict(live_type='native definition collection index i32 at+0xf8',
            wire_type='definition hash u32, remapped by original HashToIndex',
            id_offset='0xa8',city_id_offset='0xe0',complete_offset='0x400',
            wire_version=15,serialized_projection_prefix_bytes=45,
            owner='derived from District.GetPlot -> Plot.GetOwner, not a separate stored District owner',
            absent_lookup_type=-1,absent_lookup_complete=False),
        city=dict(wire_version=51,id_offset='0xb0',owner_offset='0xe0',serialized_projection_prefix_bytes=20),
        is_city=dict(predicate='resolved District definition type hash ==0x8a32007d',
            requires_completion=False,requires_resolved_City_instance=False,
            different_from_District_IsCityCenter_location_predicate=True,
            missing_definition_for_nonnegative_type='unsupported invalid context; original IsCity has no null guard'),
        domains=dict(ordinals=['sea','air','land','immobile','hover'],masks=list(domains),
            outside_0_through_4_mask=8,raw_mask_bits=16,native_predicate='(signextend_i16(mask) & requested_mask)!=0',
            initial_value=0,wire_omitted=True,
            base_restore=dict(sea='water OR (IsCity AND IsCoastalLand(1))',air=True,land='NOT water',immobile=False,hover=True),
            XP2_load_augmentation=['complete Canal: sea; adjacent City plots sea',
                'completed internal-only District with CanalWonder definition: sea and adjacent cities sea; no IsWonderComplete test',
                'completed internal-only District plus completed BridgeWonder: land'],
            incremental_gameplay_writers=['district completion/create/destroy/pillage','building completion/pillage','city transfer'],
            current_loaded_mask_must_be_observed_or_exactly_reconstructed=True),
        native_probe=dict(obligations=['same quiescent source/configuration and original pointers',
            'HasCityAttributes separate from default/fallback object',
            'requested DistrictID original hidden-return construction',
            'null/present actual lookup; actual full ID; runtime type index/hash; completion',
            'actual raw validDomains i16 and actual formation-domain query',
            'exact original live getters/virtual bindings, source hashes and before/after state'],
            hidden_return_ABI='GetDistrict/GetOwningCity/GetComponentID: resultstorage RDI, receiver RSI; keep storage8-aligned',
            lookup_ABI='static CManager.GetDistrict/GetCity(ComponentID const* RDI) -> borrowed pointer RAX',
            original_native_execution_performed_by_this_task=False),
        original_fixtures=observed,
        fixture_prefix_projections_equal=observed[0]['serialized_projection_sha256']==observed[1]['serialized_projection_sha256'],
        registry_contract_tests=['stale high16 aliases same native slot','player64 may resolve','invalid owner/id/mapping and missing slot',
            'domain ordinal/mask fallback and raw query bits'],
        production_Rust_types_changed=False,full_Player_parser_required_for_targeted_registry=False,
        native_runtime_type_or_mask_export_completed=False,
        complete_city_simulation=False,complete_district_simulation=False,
        complete_world_import=False,native_execution=False,gameplay_parity=False,
        full_gameplay_state_action_turn_supported=False,unknown_bytes_masked=0)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods=len(METHODS),fixture_count=len(observed),native_execution=False)))


if __name__=='__main__':main()
