#!/usr/bin/env python3
"""Bounded decoder for passive Diplomacy.Manager observations, not execution admission."""
import argparse
import hashlib
import json
import re
import struct
from pathlib import Path

TARGET = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
MAX_BYTES = 16 * 1024 * 1024
MAX_RECORDS = 40000
U64 = (1 << 64) - 1


class InvalidCapture(ValueError):
    pass


def require(condition, reason):
    if not condition:
        raise InvalidCapture(reason)


def number(value, low=0, high=U64):
    require(type(value) is int and low <= value <= high, 'integer range/type')
    return value


def pointer(value):
    require(isinstance(value, str) and re.fullmatch(r'0x[0-9a-f]{1,16}', value), 'pointer format')
    return int(value, 16)


def add(base, offset):
    require(base != 0 and offset <= U64 - base, 'pointer overflow/null')
    return base + offset


def raw(row, expected):
    require(row.get('raw') == expected.hex(), 'raw field projection differs')


def pairs(values):
    out = {}
    for key, value in values:
        require(key not in out, 'duplicate JSON key')
        out[key] = value
    return out


def load_records(data):
    require(len(data) <= MAX_BYTES, 'capture byte limit')
    lines = data.splitlines()
    require(2 <= len(lines) <= MAX_RECORDS, 'record count')
    rows = []
    for line in lines:
        require(len(line) <= 65536, 'record byte limit')
        try:
            row = json.loads(line, object_pairs_hook=pairs,
                             parse_constant=lambda _: (_ for _ in ()).throw(InvalidCapture('JSON constant')))
        except (ValueError, RecursionError) as error:
            raise InvalidCapture('invalid JSON record') from error
        require(type(row) is dict and type(row.get('kind')) is str, 'record object/kind')
        rows.append(row)
    return rows


class Cursor:
    def __init__(self, rows):
        self.rows, self.pos = rows, 0

    def take(self, kind, container=None):
        require(self.pos < len(self.rows), 'missing record')
        row = self.rows[self.pos]
        self.pos += 1
        require(row['kind'] == kind, 'record order: expected ' + kind)
        if container is not None:
            require(row.get('container') == container, 'container order')
        return row


def decode(data, expected_profile=None):
    rows = load_records(data)
    require(rows[0]['kind'] == 'snapshot_begin' and rows[-1]['kind'] == 'snapshot_end', 'missing frame')
    require(sum(row['kind'] == 'snapshot_begin' for row in rows) == 1
            and sum(row['kind'] == 'snapshot_end' for row in rows) == 1, 'multiple snapshots')
    begin, end = rows[0], rows[-1]
    require(begin.get('schema') == 'c6-linux-diplomacy-capture-v1', 'schema')
    require(begin.get('semantic_complete') is False and begin.get('gameplay_parity') is False, 'scope marker')
    require(type(end.get('complete')) is bool, 'completion type')
    require(number(end.get('read_bytes'), high=8 * 1024 * 1024) >= 0, 'read budget')
    number(end.get('items'), high=65536)
    # A failed bounded read is retained as partial evidence, never fabricated
    # empty state. The original JSONL must remain with this summarized result.
    if not end['complete']:
        require(isinstance(end.get('failure'), str) and 0 < len(end['failure']) <= 128, 'failure marker')
        return dict(schema='c6-linux-diplomacy-decoded-v1', complete=False,
                    failure=end['failure'], raw_records=rows, source_sha256=hashlib.sha256(data).hexdigest(),
                    semantic_complete=False, native_execution_admitted=False)
    require(end.get('failure') is None, 'contradictory completion')
    require(begin.get('core_sha256') == TARGET, 'target identity')
    profile = begin.get('profile_sha256')
    require(isinstance(profile, str) and re.fullmatch('[0-9a-f]{64}', profile), 'profile identity')
    if expected_profile is not None:
        require(profile == expected_profile, 'profile differs')
    require(isinstance(begin.get('stage'), str) and re.fullmatch('[A-Za-z0-9_.-]{1,64}', begin['stage']), 'stage')
    number(begin.get('main_tid'), low=1, high=(1 << 63) - 1)
    base, manager = pointer(begin.get('core_base')), pointer(begin.get('player_manager'))
    for name in ['module', 'context', 'game']:
        require(pointer(begin.get(name)) != 0, 'missing owner identity')
    require(base and manager, 'null base/manager')
    cursor = Cursor(rows)
    cursor.take('snapshot_begin')
    sessions = {}

    def session(expected_pointer):
        row = cursor.take('session')
        require(pointer(row.get('pointer')) == expected_pointer and expected_pointer, 'session identity')
        number(row.get('from'), -(1 << 31), (1 << 31) - 1)
        number(row.get('to'), -(1 << 31), (1 << 31) - 1)
        prev, nxt = pointer(row.get('pending_prev')), pointer(row.get('pending_next'))
        allocator = pointer(row.get('pending_allocator_raw'))
        require(row.get('pending_count') == 'unknown_not_captured', 'unobserved pending count')
        require(row.get('pending_header_raw') == struct.pack('<QQQ', nxt, prev, allocator).hex(), 'pending header projection')
        # This nested EASTL list has no count field: +16 is its allocator.
        # It is intentionally not traversed, and unknown statements stay so.
        add(expected_pointer, 0x28)
        require(row.get('statement_payload') == 'unknown'
                and row.get('pending_items') == 'not_captured_layout_unclosed', 'unknown payload marker')
        if expected_pointer in sessions:
            require(sessions[expected_pointer] == row, 'changed session during snapshot')
        sessions[expected_pointer] = row
        return row

    def linked_list(name, offset):
        header = cursor.take('list_header', name)
        sentinel = pointer(header.get('address'))
        require(sentinel == add(base, offset), 'global list address')
        prev, nxt = pointer(header.get('prev')), pointer(header.get('next'))
        count = number(header.get('count'), high=4096)
        raw(header, struct.pack('<QQQ', prev, nxt, count))
        nodes, prior, current, seen = [], sentinel, nxt, set()
        for ordinal in range(count):
            row = cursor.take('list_item', name)
            require(number(row.get('ordinal')) == ordinal, 'list ordinal')
            node = pointer(row.get('node'))
            require(node and node != sentinel and node == current and node not in seen, 'list cycle/link')
            require(pointer(row.get('prev')) == prior, 'list previous link')
            seen.add(node)
            value = pointer(row.get('session'))
            row_session = session(value)
            nodes.append(dict(node=node, session=value, fields=row_session))
            prior, current = node, pointer(row.get('next'))
        require(current == sentinel and prev == prior, 'list count/tail')
        return dict(header=header, entries=nodes)

    open_list = linked_list('open', 0x3c052f8)
    closed_list = linked_list('closed', 0x3c05310)
    owned_sessions = [e['session'] for group in [open_list, closed_list] for e in group['entries']]
    require(len(set(owned_sessions)) == len(owned_sessions), 'duplicate owned session')
    active = cursor.take('active_session')
    active_pointer = pointer(active.get('pointer'))
    if active_pointer:
        session(active_pointer)

    def vector(name, address, stride, limit):
        row = cursor.take('vector_header', name)
        require(pointer(row.get('address')) == address, 'vector address')
        start, finish, capacity = [pointer(row.get(k)) for k in ['begin', 'end', 'capacity']]
        require(start <= finish <= capacity and (start != 0 or capacity == 0), 'vector span')
        require((finish - start) % stride == 0 and (capacity - start) % stride == 0, 'vector width')
        require((finish - start) // stride <= limit and (capacity - start) // stride <= 65536, 'vector cap')
        raw(row, struct.pack('<QQQ', start, finish, capacity))
        return row, (finish - start) // stride

    pending_header, count = vector('pending_actions', add(base, 0x3c05330), 12, 65536)
    allocator = cursor.take('pending_allocator')
    pointer(allocator.get('raw'))
    actions = []
    for ordinal in range(count):
        row = cursor.take('pending_action')
        require(number(row.get('ordinal')) == ordinal, 'action ordinal')
        action_id = number(row.get('id'), high=(1 << 32) - 1)
        source = number(row.get('from'), -(1 << 31), (1 << 31) - 1)
        destination = number(row.get('to'), -(1 << 31), (1 << 31) - 1)
        raw(row, struct.pack('<Iii', action_id, source, destination))
        actions.append(row)

    players = {}
    headers = {}
    for name, offset, limit in [('ever_alive', 0x3a8, 64), ('allocation_slots', 8, 65), ('id_slots', 0x20, 65)]:
        headers[name], count = vector(name, add(manager, offset), 8, limit)
        entries = []
        for ordinal in range(count):
            row = cursor.take('player', name)
            require(number(row.get('ordinal')) == ordinal, 'player ordinal')
            value = pointer(row.get('pointer'))
            if value:
                number(row.get('id'), -(1 << 31), (1 << 31) - 1)
                number(row.get('was_ever_alive_raw'), high=255)
                require(row.get('is_ai') == 'unknown_configuration_schema', 'AI must remain unknown')
            else:
                require(not any(k in row for k in ['id', 'was_ever_alive_raw', 'is_ai']), 'null player fields')
            entries.append(row)
        players[name] = entries
    cursor.take('snapshot_end')
    require(cursor.pos == len(rows), 'trailing records')
    # This is an attribution consistency check, not a gameplay predicate. Keep
    # unknown/inconsistent inventories explicit instead of guessing order/AI.
    issues = []
    allocation, id_slots = players['allocation_slots'], players['id_slots']
    if len(allocation) != 65 or len(id_slots) != 65:
        issues.append('initialized_65_slot_tables_unavailable')
    else:
        by_pointer = {}
        for row in allocation:
            value = pointer(row['pointer'])
            if not value:
                continue
            owner = row['id']
            if value in by_pointer or not 0 <= owner < 65 or pointer(id_slots[owner]['pointer']) != value:
                issues.append('allocation_identity_disagrees')
            by_pointer[value] = row
        for index, row in enumerate(id_slots):
            value = pointer(row['pointer'])
            if value and (row['id'] != index or value not in by_pointer):
                issues.append('id_identity_disagrees')
        expected = [r for r in allocation[:64] if pointer(r['pointer']) and r['was_ever_alive_raw'] != 0]
        actual = players['ever_alive']
        if [(r['pointer'], r.get('id'), r.get('was_ever_alive_raw')) for r in actual] != [
                (r['pointer'], r['id'], r['was_ever_alive_raw']) for r in expected]:
            issues.append('ever_alive_allocation_order_disagrees')
    return dict(schema='c6-linux-diplomacy-decoded-v1', complete=True,
                source_sha256=hashlib.sha256(data).hexdigest(), capture_identity=begin,
                open_sessions=open_list, closed_sessions=closed_list, active_session=active,
                pending_header=pending_header, pending_actions=actions, pending_allocator=allocator,
                player_headers=headers, players=players,
                inventory_consistent=not issues, inventory_issues=sorted(set(issues)),
                open_sessions_observed_empty=not open_list['entries'],
                statement_payloads='unknown', current_is_ai='unknown_configuration_schema',
                semantic_complete=False, native_execution_admitted=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=Path)
    parser.add_argument('--expected-profile')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(args.capture.stat().st_size <= MAX_BYTES, 'capture byte limit')
    result = decode(args.capture.read_bytes(), args.expected_profile)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({k: result[k] for k in ['complete', 'semantic_complete', 'native_execution_admitted']}))


if __name__ == '__main__':
    main()
