#!/usr/bin/env python3
"""Fixture-specific read-only native Trade graph decoder; no simulator state loading."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

INITIAL = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
RELOADED = '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8'
INITIAL_TRACE = '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
RELOADED_TRACE = '027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369'
START, END, SIZE = 71027800, 71497438, 80657462


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


class Reader:
    def __init__(self, data):
        self.data, self.offset, self.routes = data, START, []

    def value(self, fmt):
        size = struct.calcsize(fmt)
        if self.offset + size > END:
            raise ValueError('Trade field exceeds observed original graph boundary')
        result = struct.unpack_from(fmt, self.data, self.offset)[0]
        self.offset += size
        return result

    def i32(self):
        return self.value('<i')

    def u32(self):
        return self.value('<I')

    def boolean(self):
        result = self.value('<B')
        if result not in (0, 1):
            raise ValueError('unsupported native bool representation')
        return bool(result)

    def count(self):
        result = self.i32()
        if not 0 <= result <= 1000000:
            raise ValueError('unsupported or corrupt original count')
        return result

    def version(self, expected):
        actual = self.i32()
        if actual != expected:
            raise ValueError(f'unsupported original serialization version {actual}, expected {expected}')

    def vertex(self):
        self.version(1)
        return [self.i32(), self.i32(), self.i32()]

    def yields(self):
        count = self.count()
        if count != 6:
            raise ValueError('unsupported fixture Yield definition count')
        pairs = [[self.u32(), self.i32()] for _ in range(count)]
        if len({x[0] for x in pairs}) != count:
            raise ValueError('duplicate yield type hash')
        return pairs

    def component_id(self):
        return dict(owner=self.value('<h'), component_type=self.value('<h'), id=self.u32())

    def route(self):
        start = self.offset
        self.version(4)
        result = dict(id=self.i32(), start_turn=self.i32(), trader_unit=self.component_id(),
                      resource_type_hash=self.u32(), resource_quantity=self.i32(),
                      origin_city=self.component_id(), destination_city=self.component_id(),
                      origin_yields=self.yields(), destination_yields=self.yields())
        self.routes.append((start, self.offset))
        return result

    def edge(self):
        self.version(3)
        result = dict(id=self.i32(), most_recent_route=self.route(), has_active_route=self.boolean(),
                      routes_started=self.i32(), routes_completed=self.i32())
        result['most_recent_path'] = [self.i32() for _ in range(self.count())]
        result.update(path_traversal_index=self.i32(), path_traversal_going_forward=self.boolean(), from_vertex=self.vertex(), to_vertex=self.vertex())
        return result

    def graph(self):
        self.version(1)
        total_created, count = self.i32(), self.count()
        origins, edges, origin_keys = [], {}, set()
        for _ in range(count):
            origin = self.vertex()
            origin_key = tuple(origin)
            if origin_key in origin_keys:
                raise ValueError('duplicate actual outer origin vertex')
            origin_keys.add(origin_key)
            destinations = []
            for _ in range(self.count()):
                destination, edge = self.vertex(), self.edge()
                if edge['from_vertex'] != origin or edge['to_vertex'] != destination:
                    raise ValueError('native Graph origin/destination index invariant failed')
                key = json.dumps([origin, destination], separators=(',', ':'))
                if key in edges:
                    raise ValueError('duplicate actual graph origin/destination pair')
                edges[key] = edge
                destinations.append(destination)
            origins.append(dict(vertex=origin, destinations_in_wire_order=destinations))
        if self.offset != END:
            raise ValueError('typed decoder did not consume exact observed native graph range')
        if len({x['id'] for x in edges.values()}) != len(edges):
            raise ValueError('duplicate actual native edge ID')
        return dict(total_edges_created=total_created, origins_in_wire_order=origins,
                    edge_ids_in_wire_order=[edge['id'] for edge in edges.values()], edges_by_vertices=edges)


def decode(path, expected, trace, expected_trace):
    if digest(trace) != expected_trace:
        raise ValueError('unsupported exact original native owner trace')
    if path.stat().st_size != SIZE or digest(path) != expected:
        raise ValueError('unsupported canonical original native output')
    reader = Reader(path.read_bytes())
    graph = reader.graph()
    native_routes = []
    with trace.open() as source:
        for line in source:
            if '"owner":"GameCore::Trade::Route"' not in line:
                continue
            row = json.loads(line)
            if row['save'] == 1 and START <= row['start'] and row['end'] <= END:
                native_routes.append((row['start'], row['end']))
    if sorted(native_routes) != sorted(reader.routes):
        raise ValueError('typed Route record ranges differ from actual original serializer trace')
    return graph


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--initial', type=Path, required=True)
    parser.add_argument('--reloaded', type=Path, required=True)
    parser.add_argument('--initial-trace', type=Path, required=True)
    parser.add_argument('--reloaded-trace', type=Path, required=True)
    parser.add_argument('--private-details', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    a = decode(args.initial, INITIAL, args.initial_trace, INITIAL_TRACE)
    b = decode(args.reloaded, RELOADED, args.reloaded_trace, RELOADED_TRACE)
    ka, kb = a['edges_by_vertices'], b['edges_by_vertices']
    oa = {tuple(x['vertex']) for x in a['origins_in_wire_order']}
    ob = {tuple(x['vertex']) for x in b['origins_in_wire_order']}
    common = ka.keys() & kb.keys()
    initial_origins = [x['vertex'] for x in a['origins_in_wire_order']]
    reloaded_origins = [x['vertex'] for x in b['origins_in_wire_order']]
    initial_destinations = {tuple(x['vertex']): x['destinations_in_wire_order'] for x in a['origins_in_wire_order']}
    reloaded_destinations = {tuple(x['vertex']): x['destinations_in_wire_order'] for x in b['origins_in_wire_order']}
    changed = [dict(key=k, initial=ka[k], reloaded=kb[k]) for k in sorted(common) if ka[k] != kb[k]]
    details = dict(initial=a, reloaded=b, changed_edges=changed,
                   initial_only_keys=sorted(ka.keys()-kb.keys()), reloaded_only_keys=sorted(kb.keys()-ka.keys()))
    with args.private_details.open('x') as destination:
        destination.write(json.dumps(details, indent=2)+'\n')
    result = dict(schema='c6-linux-save-normalization-typed-trade-v1', initial_sha256=INITIAL,
                  reloaded_sha256=RELOADED, parser_source_sha256=digest(Path(__file__)),
                  native_trace_sha256=[digest(args.initial_trace), digest(args.reloaded_trace)], graph_start=START, graph_end=END,
                  total_edges_created=[a['total_edges_created'], b['total_edges_created']],
                  origins=[len(a['origins_in_wire_order']), len(b['origins_in_wire_order'])],
                  edges=[len(ka), len(kb)], initial_only_origin_keys=len(oa-ob), reloaded_only_origin_keys=len(ob-oa),
                  initial_only_edge_keys=len(ka.keys()-kb.keys()),
                  reloaded_only_edge_keys=len(kb.keys()-ka.keys()), changed_keyed_edge_payloads=len(changed),
                  keyed_values_equal=oa == ob and ka == kb and a['total_edges_created'] == b['total_edges_created'],
                  serialized_iteration_order_equal=a['origins_in_wire_order'] == b['origins_in_wire_order'],
                  changed_origin_wire_positions=sum(x != y for x, y in zip(initial_origins, reloaded_origins)),
                  origins_with_changed_destination_order=sum(initial_destinations[k] != reloaded_destinations[k] for k in oa & ob),
                  changed_edge_id_wire_positions=sum(x != y for x, y in zip(a['edge_ids_in_wire_order'], b['edge_ids_in_wire_order'])),
                  private_details=dict(filename=args.private_details.name, sha256=digest(args.private_details), bytes=args.private_details.stat().st_size),
                  limits='Strict fixture-only read-only decoder, actual native Route boundaries checked. Key equality does not discard native container iteration order or prove future gameplay/action equivalence. Route finalizing live field is not serialized.')
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
