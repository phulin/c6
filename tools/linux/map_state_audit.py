#!/usr/bin/env python3
"""Independently decode native Linux Map v4 structure; never execute game code.

The original owner trace identifies the outer Map call only. Every inner boundary
is advanced from serializer field order and serialized lengths, never fixture offsets.
"""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import struct

XP2 = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
INITIAL = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
RELOADED = '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8'


class Reader:
    def __init__(self, data, start=0, end=None):
        self.data, self.pos, self.end = data, start, len(data) if end is None else end
        self.ranges = []
        self.owners = []

    def raw(self, size):
        if size < 0 or self.pos + size > self.end:
            raise ValueError(f'truncated at {self.pos}, request {size}, end {self.end}')
        result = self.data[self.pos:self.pos + size]
        self.pos += size
        return result

    def take(self, fmt):
        return struct.unpack('<' + fmt, self.raw(struct.calcsize('<' + fmt)))[0]

    def u32(self):
        return self.take('I')

    def count(self):
        n = self.u32()
        if n > 1_000_000:
            raise ValueError(f'count budget at {self.pos - 4}: {n}')
        return n

    def version(self, expected):
        actual = self.u32()
        if actual != expected:
            raise ValueError(f'version at {self.pos - 4}: {actual}, expected {expected}')
        return actual

    def array(self, fmt):
        return [self.take(fmt) for _ in range(self.count())]

    def hash_array(self):
        return [(self.u32(), self.take('i')) for _ in range(self.count())]

    def block(self, name, function, owner=None):
        start = self.pos
        result = function()
        self.ranges.append(dict(name=name, start=start, end=self.pos,
                                bytes=self.pos-start,
                                sha256=hashlib.sha256(self.data[start:self.pos]).hexdigest()))
        if owner:
            self.owners.append((owner, start, self.pos))
        return result

    def recycled(self, element):
        self.version(65537)
        bits, capacity, size = self.u32(), self.count(), self.count()
        if bits != 16 or size > capacity:
            raise ValueError('unsupported recycled container')
        entries = [(self.take('i'), element()) for _ in range(size)]
        current_id_bits, free_size, free_head, free_tail = self.u32(), self.count(), self.u32(), self.u32()
        free = [self.u32() for _ in range(free_size)]
        return dict(bits=bits, capacity=capacity, entries=entries, current_id_bits=current_id_bits,
                    free_head=free_head, free_tail=free_tail, free=free)

    def visibility(self):
        self.version(1)
        entries = []
        for _ in range(self.count()):
            player = self.take('i')
            entries.append((player, self.block('visibility_instance', self.visibility_instance,
                           'GameCore::Map::Player::Visibility::Instance')))
        return entries

    def visibility_instance(self):
        self.version(6)
        player, cached, stale = self.take('i'), self.u32(), self.take('B')
        revealed = self.array('B')
        layers = []
        for _ in range(self.count()):
            fields = [self.u32() for _ in range(4)]
            values = self.array('h')
            if len(values) != fields[2]:
                raise ValueError('visibility layer count differs')
            layers.append(dict(id=fields[0], index=fields[1], count=fields[2], width=fields[3], values=values))
        shared = []
        for _ in range(2):
            rows = []
            for _ in range(self.count()):
                self.version(6)
                rows.append((self.take('i'), self.u32()))
            shared.append(rows)
        return dict(player=player, cached=cached, stale=stale, revealed=revealed,
                    layers=layers, shared=shared)

    def area(self):
        self.version(2)
        scalars = [self.take('i') for _ in range(11)]
        players = [self.array('i') for _ in range(5)]
        if any(len(x) != 64 for x in players):
            raise ValueError('unsupported area player array width')
        yields = [self.hash_array() for _ in range(64)]
        resources, improvements = self.hash_array(), self.hash_array()
        bounds = [self.take('i') for _ in range(4)]
        flags = [self.take('B') for _ in range(3)]
        return dict(scalars=scalars, players=players, yields=yields, resources=resources,
                    improvements=improvements, bounds=bounds, flags=flags)

    def landmass(self):
        self.version(1)
        return dict(id=self.take('i'), count=self.take('i'), water=self.take('B'),
                    centroid_delta=[self.take('i'), self.take('i')])

    def region(self):
        self.version(1)
        return dict(id=self.u32(), count=self.take('i'), chokepoint=self.take('B'),
                    area=self.u32(), joined=self.array('I'))

    def chokepoint(self):
        self.version(1)
        return dict(id=self.u32(), initialized=self.take('B'), endpoints=self.coords(),
                    points=self.coords(), joined_areas=self.array('I'))

    def coords(self):
        return [(self.take('i'), self.take('i')) for _ in range(self.count())]

    def continent(self):
        self.version(1)
        return dict(id=self.take('i'), type_hash=self.u32(), count=self.take('i'), fertility=self.u32())

    def feature_layout(self):
        return dict(lowest_plot=self.take('H'), plots=self.array('H'))

    def variant_map(self, depth=0):
        if depth > 64:
            raise ValueError('property depth budget')
        header = self.u32()
        n = self.count() if header & 0x80000000 else header
        if n > 1_000_000:
            raise ValueError('property map count budget')
        rows = []
        for _ in range(n):
            key = self.u32()
            name = self.raw(self.take('H')).hex() if header & 0x80000000 and header & 1 else None
            rows.append(dict(key=key, name=name, value=self.typed_variant(depth + 1)))
        return dict(header=header, entries=rows)

    def typed_variant(self, depth):
        kind, modifiers = self.take('H'), self.take('H')
        if modifiers or kind not in (0,1,2,3,4,5,8,9,10,11,20,21,22):
            raise ValueError('unsupported property type/modifiers')
        return dict(kind=kind, value=self.array_variant(depth, kind))

    def array_variant(self, depth, kind=None):
        if depth > 64:
            raise ValueError('property depth budget')
        flags, element_size = self.u32(), self.u32()
        child = flags & 0x5e000000
        if child.bit_count() > 1 or flags & 0x20000000:
            raise ValueError('unsupported property branch')
        if child == 0x04000000 or child == 0x40000000:
            value = self.variant_map(depth + 1)
        elif child == 0x10000000:
            value = [self.typed_variant(depth + 1) for _ in range(self.count())]
        elif child == 0x08000000:
            value = []
        elif child == 0x02000000:
            scalars = [(self.u32(), self.u32()) for _ in range(self.count())]
            arrays = [(self.u32(),self.array_variant(depth + 1)) for _ in range(self.count())]
            value = dict(scalars=scalars,arrays=arrays)
        elif flags & 0x01000000:
            if kind != 5 or element_size != 1 or flags & 0x80000000:
                raise ValueError('unsupported property raw array')
            size = flags & 0xffffff
            value = self.raw(size).hex()
        else:
            value = self.raw(8 if flags & 0x80000000 else 4).hex()
        return dict(flags=flags, element_size=element_size, payload=value)

    def plot(self):
        fields = [('area_id','i'),('landmass_id','i'),('region_id','I'),('terrain_hash','I'),
                  ('feature_hash','I'),('feature_layout','H'),('continent_hash','I'),('unit_count','b'),
                  ('resource_hash','I'),('resource_count','H'),('improvement_hash','I'),
                  ('improvement_owner','b'),('route_type','b'),('route_era','b'),('appeal','h'),
                  ('river_e_flow','b'),('river_se_flow','b'),('river_sw_flow','b'),
                  ('river_crossing_count','b'),('river_crossing_mask','B'),('cliff_crossing_mask','B')]
        result = {name:self.take(fmt) for name,fmt in fields}
        flags = int.from_bytes(self.raw(3),'little')
        if flags >> 17:
            raise ValueError('unsupported noncanonical plot flag bits')
        result['flags'] = flags
        count = self.count()
        result['properties'] = self.variant_map() if count else None
        if count and count != len(result['properties']['entries']):
            raise ValueError('property count does not agree')
        if flags & (1 << 14):
            result['city'] = dict(working=self.u32(), owning=self.u32(), district=self.u32(),
                                  owner=self.take('b'), wonder=self.u32())
        else:
            result['city'] = None
        return result

    def unit_container(self):
        self.version(1)
        width, height = self.u32(), self.u32()
        layers = []
        for _ in range(self.count()):
            layer_id = self.u32()
            self.version(1)
            allocated, maximum = self.count(), self.u32()
            cells = []
            for _ in range(allocated):
                index = self.u32()
                cells.append((index, [(self.take('i'),self.u32()) for _ in range(self.count())]))
            layers.append(dict(id=layer_id, maximum=maximum, cells=cells))
        return dict(width=width, height=height, layers=layers)

    def sparse_plot_data(self, element):
        self.version(4)
        default = element()
        indices = self.array('i')
        values = [element() for _ in range(self.count())]
        return dict(default=default, indices=indices, values=values)

    def string8(self):
        return self.raw(self.count()).hex()

    def eruption(self):
        self.version(7)
        return dict(turn=self.take('i'), duration=self.take('i'), severity=self.take('i'))

    def volcano(self):
        self.version(5)
        return dict(name_hash=self.u32(), plot=self.take('i'), discovering_civ_hash=self.u32(),
                    turn_active=self.take('i'), custom_name=self.string8(),
                    eruptions=[self.eruption() for _ in range(self.count())],
                    affected_plots=self.array('i'), periods_raw=self.array('i'), chances_raw=self.array('i'))

    def natural_wonder_volcano(self):
        self.version(5)
        return dict(feature_hash=self.u32(), plots=self.array('i'),
                    eruptions=[self.eruption() for _ in range(self.count())])

    def ice(self):
        self.version(2)
        return dict(plot=self.take('i'), random_event_hash=self.u32())

    def feature_component(self):
        self.version(7)
        return dict(extra_yields=self.sparse_plot_data(self.hash_array),
                    volcanoes=[self.block('volcano',self.volcano,'GameCore::Map::Feature::Volcano') for _ in range(self.count())],
                    natural_wonders=[self.natural_wonder_volcano() for _ in range(self.count())],
                    plot_volcanoes=self.sparse_plot_data(lambda:self.array('i')),
                    ice=[self.ice() for _ in range(self.count())])

    def flood(self):
        self.version(5)
        return dict(turn=self.take('i'), duration=self.take('i'), severity=self.take('i'), mitigated=self.take('B'))

    def river(self):
        self.version(4)
        return dict(name_hash=self.u32(), id=self.take('i'), plots=self.array('i'),
                    floodplains=self.array('i'), discovering_civ_hash=self.u32(), custom_name=self.string8(),
                    floods=[self.flood() for _ in range(self.count())], edges=self.coords(),
                    periods_raw=self.array('i'), chances_raw=self.array('i'))

    def river_component(self):
        self.version(4)
        return dict(next_id=self.take('i'),
                    rivers=[self.block('river',self.river,'GameCore::Map::River::Instance') for _ in range(self.count())],
                    plot_rivers=self.sparse_plot_data(lambda:self.array('i')),
                    floodplain_rivers=self.sparse_plot_data(lambda:self.array('i')))

    def coastal_lowland(self):
        self.version(9)
        return dict(lowland_hash=self.u32(), plot=self.take('i'), starting_terrain_hash=self.u32(),
                    river_flow=[self.take('b') for _ in range(6)], river_crossing_count=self.take('b'),
                    river_crossing_mask=self.take('B'), river_edges=[self.take('B') for _ in range(6)],
                    flooded=self.take('B'), submerged=self.take('B'), protected=self.take('B'))

    def named_terrain(self, version, hashed):
        self.version(version)
        return dict(type_hash=self.u32() if hashed else self.take('i'),
                    discovering_civ_hash=self.u32(), territory=self.take('i'), custom_name=self.string8())

    def terrain_component(self):
        self.version(6)
        result=dict(coastal_lowlands=[self.coastal_lowland() for _ in range(self.count())])
        for name, version, hashed in [('deserts',6,True),('mountain_ranges',6,True),
                                      ('named_lakes',1,False),('named_oceans',1,False),('named_seas',1,False)]:
            result[name]=[self.named_terrain(version,hashed) for _ in range(self.count())]
        return result

    def route_component(self):
        self.version(1)
        self.version(1)
        width, height, count = self.u32(), self.u32(), self.count()
        portals=[]
        if count:
            self.version(2)
            for _ in range(count):
                portals.append(dict(entrance_plot=self.take('i'),exit_plots=self.array('i'),source_plots=self.array('i')))
        return dict(width=width,height=height,portals=portals)

    def localized_string(self, depth=0):
        if depth>64:
            raise ValueError('localized name depth budget')
        self.version(0xffffffff)
        if self.take('B')!=1:
            raise ValueError('unsupported localized string version')
        base=self.string8()
        args=[]
        for _ in range(self.take('B')):
            kind=self.take('B')
            if kind==0:
                value=None
            elif kind==1:
                value=self.take('i')
            elif kind==2:
                value=self.take('q')
            elif kind==3:
                value=self.u32()
            elif kind==4:
                value=self.take('Q')
            elif kind==5:
                value=self.localized_string(depth+1)
            else:
                raise ValueError('unsupported localized argument type')
            args.append(dict(kind=kind,value=value))
        names=[]
        for _ in range(self.take('B')):
            index=self.u32()
            if self.take('B')!=1:
                raise ValueError('unsupported localized argument name multiplicity')
            names.append(dict(index=index,name=self.string8()))
        return dict(base=base,args=args,names=names)

    def territory(self):
        self.version(2)
        return dict(id=self.take('i'),terrain_class_hash=self.u32(),
                    counts=[self.take('i') for _ in range(6)],is_lake=self.take('B'),is_sea=self.take('B'),
                    bounds=[self.take('i') for _ in range(4)],centroid_delta=[self.take('i') for _ in range(2)],
                    feature_counts=self.hash_array(),name=self.localized_string())

    def territories_component(self):
        self.version(1)
        return dict(territories=self.recycled(self.territory),plot_territories=self.array('I'))

    def components(self):
        self.version(1)
        result = []
        for _ in range(self.count()):
            name = self.raw(self.count()).decode('ascii')
            size = self.count()
            start = self.pos
            payload = self.raw(size)
            sub = Reader(self.data,start,self.pos)
            decoders = dict(Route=sub.route_component,Feature=sub.feature_component,
                            River=sub.river_component,Terrain=sub.terrain_component,
                            Territories=sub.territories_component,TerritoryBuilder=lambda:None)
            if name not in decoders:
                raise ValueError(f'unsupported map component {name}')
            value=decoders[name]()
            if sub.pos!=sub.end:
                raise ValueError(f'component {name} not exhausted {sub.pos}/{sub.end}')
            result.append(dict(name=name, start=start, end=self.pos, bytes=size,
                               sha256=hashlib.sha256(payload).hexdigest(), decoded=True, value=value))
        return result

    def map(self):
        def header():
            self.version(4)
            values = [self.take('i') for _ in range(4)]
            return dict(width=values[0],height=values[1],top_latitude=values[2],bottom_latitude=values[3],
                        wrap_x=self.take('B'),wrap_y=self.take('B'),size_hash=self.u32(),
                        land_plots=self.take('i'),owned_plots=self.take('i'),natural_wonders=self.take('i'),
                        resources=self.hash_array(),land_resources=self.hash_array())
        result = dict(header=self.block('header',header))
        result['visibility'] = self.block('visibility',self.visibility,'GameCore::Map::Player::Visibility::Manager')
        for name,element,owner in [('areas',self.area,'Area'),('landmasses',self.landmass,'Landmass')]:
            def container(element=element,owner=owner):
                self.version(1)
                return self.recycled(lambda:self.block(owner.lower(),element,f'GameCore::Map::{owner}::Instance'))
            result[name] = self.block(name,container)
        def regions():
            self.version(1)
            a = self.recycled(lambda:self.block('region',self.region,'GameCore::Map::Region::Instance'))
            b = self.recycled(self.chokepoint)
            return dict(regions=a,chokepoints=b)
        result['regions'] = self.block('regions',regions)
        result['continents'] = self.block('continents',lambda:[self.block('continent',self.continent,'GameCore::Map::Continent::Instance') for _ in range(self.count())])
        def features():
            self.version(1)
            return self.recycled(self.feature_layout)
        result['features'] = self.block('features',features)
        def plots():
            self.version(6)
            count = self.count()
            if count != result['header']['width'] * result['header']['height']:
                raise ValueError('plot count does not equal dimensions')
            return [self.block('plot',self.plot) for _ in range(count)]
        result['plots'] = self.block('plots',plots)
        result['units'] = self.block('units',self.unit_container,'GameCore::Map::UnitContainer')
        result['components'] = self.block('components',self.components)
        return result


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--module',type=Path,required=True)
    p.add_argument('--initial',type=Path,required=True)
    p.add_argument('--reloaded',type=Path,required=True)
    p.add_argument('--initial-trace',type=Path,required=True)
    p.add_argument('--reloaded-trace',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--private-details',type=Path,help='private directory for full decoded native values')
    args=p.parse_args()
    if digest(args.module)!=XP2:
        raise ValueError('native module identity differs')
    results=[]
    input_streams=[]
    for path,trace,expected in [(args.initial,args.initial_trace,INITIAL),(args.reloaded,args.reloaded_trace,RELOADED)]:
        data = gzip.decompress(path.read_bytes()) if path.suffix=='.gz' else path.read_bytes()
        input_streams.append(data)
        if len(data)!=80657462 or hashlib.sha256(data).hexdigest()!=expected:
            raise ValueError('canonical stream identity differs')
        rows=[json.loads(x) for x in trace.read_text().splitlines()]
        rows=[x for x in rows if x.get('event')=='owner' and x.get('save')==1]
        outer=[x for x in rows if x['owner']=='GameCore::Map::Instance']
        if len(outer)!=1:
            raise ValueError('expected one native Map owner')
        outer=outer[0]
        r=Reader(data,outer['start'],outer['end'])
        decoded=r.map()
        if r.pos!=r.end:
            raise ValueError(f'Map not exhausted {r.pos}/{r.end}')
        if args.private_details:
            args.private_details.mkdir(parents=True,exist_ok=True)
            (args.private_details/(expected+'.json.gz')).write_bytes(gzip.compress(json.dumps(decoded,separators=(',',':')).encode(),mtime=0))
        observed={(x['owner'],x['start'],x['end']) for x in rows}
        if any(x not in observed for x in r.owners):
            raise ValueError('decoded owner boundary does not agree with native trace')
        fields=['terrain_hash','feature_hash','resource_hash','improvement_hash','route_type','route_era','area_id','landmass_id']
        area_ids={x['scalars'][0] for _,x in decoded['areas']['entries']}
        landmass_ids={x['id'] for _,x in decoded['landmasses']['entries']}
        region_ids={x['id'] for _,x in decoded['regions']['regions']['entries']}
        if any(x['area_id'] not in area_ids or x['landmass_id'] not in landmass_ids or
               (x['region_id']!=0xffffffff and x['region_id'] not in region_ids) for x in decoded['plots']):
            raise ValueError('plot container ID link missing')
        comp={x['name']:x['value'] for x in decoded['components']}
        component_counts=dict(volcanoes=len(comp['Feature']['volcanoes']),
                              natural_wonder_volcanoes=len(comp['Feature']['natural_wonders']),
                              ice=len(comp['Feature']['ice']),rivers=len(comp['River']['rivers']),
                              territory_entries=len(comp['Territories']['territories']['entries']),
                              plot_territory_ids=len(comp['Territories']['plot_territories']))
        results.append(dict(stream_sha256=expected,trace_sha256=digest(trace),start=outer['start'],end=r.end,
                            map_sha256=hashlib.sha256(data[outer['start']:r.end]).hexdigest(),
                            header=decoded['header'],ranges=[x for x in r.ranges if x['name'] in ['header','visibility','areas','landmasses','regions','continents','features','plots','units','components']],
                            validated_native_owner_boundaries=len(r.owners),
                            counts=dict(visibility=len(decoded['visibility']),areas=len(decoded['areas']['entries']),
                                        landmasses=len(decoded['landmasses']['entries']),regions=len(decoded['regions']['regions']['entries']),
                                        chokepoints=len(decoded['regions']['chokepoints']['entries']),continents=len(decoded['continents']),
                                        feature_layouts=len(decoded['features']['entries']),plots=len(decoded['plots']),
                                        plots_with_properties=sum(x['properties'] is not None for x in decoded['plots']),
                                        plots_with_city_attributes=sum(x['city'] is not None for x in decoded['plots']),
                                        unit_layers=len(decoded['units']['layers'])),
                            component_counts=component_counts,
                            plot_links_resolved=True,
                            present_city_attribute_owner_histogram=dict(sorted(Counter(x['city']['owner'] for x in decoded['plots'] if x['city']).items())),
                            plot_histograms={f:dict(sorted(Counter(x[f] for x in decoded['plots']).items())) for f in fields},
                            components=[{k:v for k,v in c.items() if k!='value'} for c in decoded['components']]))
    private=Path(__file__).resolve().parents[3]/'c6-ghidra/evidence/linux-map-state'
    artifacts=[dict(name=x.name,sha256=digest(x),bytes=x.stat().st_size) for x in sorted(private.iterdir()) if x.is_file() and x.suffix in ('.asm','.txt')]
    result=dict(schema='c6-linux-map-state-v1',module_sha256=XP2,streams=results,private_static_evidence=artifacts,
                map_bytes_equal=results[0]['map_sha256']==results[1]['map_sha256'],
                source_only_nonempty_schemas=['Route portal v2','CoastalLowland v9','Desert v6','MountainRange v6','NamedTerritory v1','Territory v2','current recursive Localization::String'],
                source_only_limits='Recovered original writer/reader schemas; no original-code nonempty Route/Terrain/Territory fixture exists in this audit. Hand-authored branch fixtures are structural checks only.',
                raw_save_load_different_bytes=sum(a!=b for a,b in zip(*input_streams)),
                limits='Static original-code contract plus independent decode of original streams; no native execution, world reconstruction, movement, actions, turns, or gameplay RNG parity. Current-version Map component schemas decoded; nonempty Route/Terrain/Territory records are source-derived only and absent from these native fixtures. Historical versions remain unsupported. Raw 89,198-byte Save→Load failure remains required.')
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(output=str(args.output),streams=[dict(sha256=x['stream_sha256'],counts=x['counts']) for x in results])))


if __name__=='__main__':
    main()
