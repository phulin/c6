#!/usr/bin/env python3
"""Read-only SHA-gated Linux request conversion and transport ABI evidence."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA

METHODS = [('gamecore', 0x2c7e2c0, 13, 'Exports.Context.RequestUnitOperation', 'c4f01b05985fc9af021e6220373fa2bc710a31a6f4ccf60abad2f9ada0116dbe'), ('gamecore', 41968032, 212, 'Cache.Unit.Manager.RequestOperation', '218e087b8d1253ca44100a3d5fd016c1452d5b4af9e80f4123620c225b1068ae'), ('gamecore', 47852272, 81, 'Unit.Operations.Manager.MakeRequest', 'df4289c1de49d2ca7ed1a2fa74c6e97fd092f0b296e70c279f14ddd89441b911'), ('gamecore', 47802144, 246, 'Unit.Operations.BaseHandler.MakeRequest', 'f251ebcecbd03408fb08ca8a3d2fcf4f377c6002a1c21457955a62b77f711cd2'), ('gamecore', 47332848, 664, 'Messages.Manager.BroadcastMessage', '26bd66fc177e4f08efd1aa0a47f871be695a253e48fbaf721415e23b8467bead'), ('host', 53169408, 73, 'EngineUtility.SendMessage(stream)', '66519012830a77b9b6afbd6d7b2acc459ef6947e2c4c53bffbc17349e8f69bca'), ('host', 54254544, 29, 'Network.Manager.BroadcastMessage', '1bf7b49e98b7ac064a3aad55944e9a77bcae26d3a0623d03b3189a96b362349f'), ('host', 60721704, 2563, 'FNetSession.BroadcastMessage', '0c887cfa3eed625673c1ba4276851a055026bbca6004f1985242c5537a1c0356'), ('host', 54196208, 252, 'NetGameCoreMessage.Execute', '25657c19fb08579acb8dc79477586b1bef2467c5fac37d483e44d7a4a7b351ac')]
SLOTS = [
    ('gamecore', 0x3aff870, 0x100, 0x2da2af0, 'Manager.MakeRequest'),
    ('gamecore', 0x3aff870, 0xa0, 0x2da1720, 'Manager.GetHandlerIDFromHash'),
    ('gamecore', 0x3aff870, 0x88, 0x2da1650, 'Manager.GetHandler'),
    ('gamecore', 0x3b00da0, 0x78, 0x2d96720, 'SkipTurn.MakeRequest'),
    ('host', 0x53a6c80, 0x78, 0x32b4d00, 'EngineUtility.SendMessage(stream)'),
    ('host', 0x53b5600, 0xc0, 0x39e8a28, 'FNetSession.BroadcastMessage'),
    ('host', 0x53b5600, 0x280, 0x39d2194, 'FNetSession.GetSyncManager'),
    ('host', 0x53b5600, 0x278, 0x39d218e, 'FNetSession.GetPlayerManager'),
]
CALLS = [
    ('gamecore', 0x280622e, 0x1fb9798, 'request_broadcast'),
    ('gamecore', 0x2d24040, 0x1f63c10, 'broadcast_serialization'),
    ('host', 0x32b4d0d, 0x33af670, 'bridge_construct_net_message'),
    ('host', 0x32b4d1d, 0x33bdbd0, 'bridge_broadcast'),
    ('host', 0x33af804, 0x33d4150, 'receive_gamecore_active_gate'),
]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    elves = {'host': Elf(a.assets/'Civ6', HOST_SHA),
             'gamecore': Elf(a.assets/'libGameCore_XP2.so', GAME_SHA)}
    for module, address, size, name, digest in METHODS:
        if hashlib.sha256(elves[module].read(address, size)).hexdigest() != digest:
            raise ValueError('method identity differs: '+name)
    for module, table, offset, target, name in SLOTS:
        elf = elves[module]
        actual = (struct.unpack('<Q', elf.read(table+16+offset, 8))[0]
                  if module == 'host' else elf.relocated_pointer(table+16+offset))
        if actual != target:
            raise ValueError('virtual binding differs: '+name)
    for module, address, target, _ in CALLS:
        elves[module].call(address, target)
    if elves['gamecore'].read(0x1fb9798,6) != bytes.fromhex('ff252285b601'):
        raise ValueError('broadcast GOT binding differs')
    for module, address, expected in [('host', 0x3044300, '488b4708c3'),
                                      ('host', 0x39d2194, '488b4738c3'),
                                      ('host', 0x39d218e, '488b4740c3')]:
        if elves[module].read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('session getter bytes differ')
    if elves['gamecore'].read(0x2c7e2c0, 13) != bytes.fromhex('4889f789d64889cae9034033ff'):
        raise ValueError('Exports.Context member-to-static forwarding ABI differs')
    result = dict(schema='c6-linux-action-delivery-abi-v1', checks_passed=True,
        host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA,
        method='Read-only original Linux bytes and original virtual bindings',
        methods=[dict(module=m,address=hex(v),size=n,name=s,sha256=h) for m,v,n,s,h in METHODS],
        virtual_bindings=[dict(module=m,table=hex(t),offset=hex(o),target=hex(v),name=s) for m,t,o,v,s in SLOTS],
        direct_calls=[dict(module=m,address=hex(v),target=hex(t),name=s) for m,v,t,s in CALLS],
        broadcast_plt='0x1fb9798', broadcast_got='0x3b21cc0',
        conversion_abi='bool(receiver RDI, hash ESI, const TypedVariantMap* RDX, Request* RCX); result AL',
        cache_request_operation_abi='static void(ComponentID* RDI, hash ESI, const TypedVariantMap* RDX)',
        exports_context_request_unit_operation_abi='member void(Context* RDI, ComponentID* RSI, hash EDX, const TypedVariantMap* RCX); forwards RSI to RDI, EDX to ESI, RCX to RDX then tailcalls static Cache.Manager via PLT 0x1fb22d0',
        engine_transport_abi='receiver RDI, MemoryOStream* RSI; original bridge always returns EAX=0',
        session_broadcast_abi='void(receiver RDI, FNetMessage* RSI, bool EDX=1, bool ECX=0)',
        transport_gates=['Network.Manager+8 session nonnull', 'FNetSession.GetSyncManager nonnull',
                         'FNetSession+0x28 nonnull'],
        receive_gates=['Network.IsGameCoreActive', 'nonempty NetGameCoreMessage payload', 'nonnull Context'],
        native_execution=False, action_executed=False, gameplay_parity=False,
        scope='Static delivery ABI only; runtime failing gate remains unobserved')
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods_checked=len(METHODS),native_execution=False)))


if __name__ == '__main__':
    main()
