#!/usr/bin/env python3
"""Read-only SHA-gated Linux request conversion and transport ABI evidence."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA

METHODS = [('gamecore', 46654144, 13, 'Exports.Context.RequestUnitOperation', 'c4f01b05985fc9af021e6220373fa2bc710a31a6f4ccf60abad2f9ada0116dbe'), ('gamecore', 41968032, 212, 'Cache.Unit.Manager.RequestOperation', '218e087b8d1253ca44100a3d5fd016c1452d5b4af9e80f4123620c225b1068ae'), ('gamecore', 47852272, 81, 'Unit.Operations.Manager.MakeRequest', 'df4289c1de49d2ca7ed1a2fa74c6e97fd092f0b296e70c279f14ddd89441b911'), ('gamecore', 47802144, 246, 'Unit.Operations.BaseHandler.MakeRequest', 'f251ebcecbd03408fb08ca8a3d2fcf4f377c6002a1c21457955a62b77f711cd2'), ('gamecore', 47332848, 664, 'Messages.Manager.BroadcastMessage', '26bd66fc177e4f08efd1aa0a47f871be695a253e48fbaf721415e23b8467bead'), ('host', 53169408, 73, 'EngineUtility.SendMessage(stream)', '66519012830a77b9b6afbd6d7b2acc459ef6947e2c4c53bffbc17349e8f69bca'), ('host', 54254544, 29, 'Network.Manager.BroadcastMessage', '1bf7b49e98b7ac064a3aad55944e9a77bcae26d3a0623d03b3189a96b362349f'), ('host', 60721704, 2563, 'FNetSession.BroadcastMessage', '0c887cfa3eed625673c1ba4276851a055026bbca6004f1985242c5537a1c0356'), ('host', 54196208, 252, 'NetGameCoreMessage.Execute', '25657c19fb08579acb8dc79477586b1bef2467c5fac37d483e44d7a4a7b351ac'), ('host', 54253760, 69, 'Network.Manager.Update(float)', '749e3b326c94cc434c32ffebf89f4f183291c4f69d0fcb40f1524afb0b84be6c'), ('host', 60725748, 4108, 'FNetSession.Update(float)', 'b9fa89e86ae8f288bea52fe40439ee5a18605a909f4b105e19b179a8b2794ea6'), ('host', 60724322, 173, 'FNetSession.SendAllMessageChunks', '0392606e0bb830588a68532fd5aeaec5d3cd0859ae3c39c1c3d05a3b0541fbe3'), ('host', 60678126, 92, 'FNetAccessENet.Send', '49ad117bacc3468ad2ccb2ec51168551e159b86213c379e078b69759d7ec1ddd'), ('host', 60678496, 143, 'FNetAccessENet.Update', '6d59ec62131850287bd2e1a7d1d37470474b9f25b685dce840edbb1cdb817546'), ('host', 60763954, 16, 'FNetSession.ProcessNetwork', 'd2fa063c94d5ee91fe0abc8fc2f10a4bdaf52ef3d00f948fd4baa89f560d4a50'), ('host', 60763418, 45, 'FNetSession.HandleMessage', 'b22448c44c0a89a12b62475650434192a8dd24ef9b46f33c1de2458b96a55520'), ('host', 60964070, 160, 'FNetMessageSync.Update', 'add3e7915c5082841803d285fb868775a075dba1e46b96e4c1cf20c5bac328be'), ('host', 63259526, 5, 'MemoryOStream.GetBytes', '8e3eaf4f4b235f364076d47e3b5b95d9e60a71cf19dd62030d0fa20e80858d2e'), ('host', 63259532, 5, 'MemoryOStream.GetOffset', '54fd6cfb47b3e150944357d067e1c7fcbe9bc8b8b20527db61840cc59da2b262'), ('host', 49690304, 4, 'ByteVector.size', '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'), ('host', 54197056, 14, 'ByteVector.const_index', 'f9cb6ea9abddfdfb70468af992da37bef4f2d9f8c3a375576095d9c31baac14c'), ('host', 49692048, 4, 'BytePtr64.conversion', 'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'), ('host', 60686874, 72, 'FNetMessage.constructor', '821c4cd19d694ce9ee9e067c4404e6f884f1b1114805839519b785b0b1172eb5'), ('host', 54195824, 157, 'NetGameCoreMessage.constructor', 'ba75825a7fa291a29dee00b17d797c75c96d61bc55a30963e9ddabf6f3a5c9a4'), ('host', 60694598, 44, 'FNetMessageFactory.CreateMessage', 'b57f983ab517a63e3521f91896d33c3720dea9da01c6b901958d014b747be6e8'), ('host', 54196608, 47, 'Register.NetGameCoreMessage', '0f212e2b5360f087a32a4445db498dfe61e6d6b44175500ac01ee420c39152e2'), ('host', 54197728, 55, 'NetGameCoreMessage.creator', '740a48e2d50140031e6bcfd334054a31f3c9935b71b32e243c8cc707644c419b'), ('host', 60760162, 3156, 'FNetSession.MessageReceived', '1b3a5a1167e1f84b1465b853b453824b5c35a477aba88d5ba44c384ec7e797cc'), ('host', 46694000, 12, 'Type3.registration_wrapper', '9f1ccd53a9c1e9337b6ed86331cae4288dbf6bbd169158e51b64b76182b9ad2b'), ('host', 46694016, 49, 'AppMessages.grouped_initializer', 'a30d022eae136c6607ae89c1116f3dadd830f4a13764f88b9a2305c113eeb304'), ('host', 54398960, 4, 'FNetPlayer.GetNeedsSnapshot', '0b3d4c9b850f6ce20def8d4daac7004ea47e0e917f9cbef7cec02789df401290'), ('host', 60695324, 17, 'FNetPlayer.SetNeedsSnapshot', '9f19deada9e8882d50b09842027aa6e0a449897be82ee69ca65ae59f35e2ee05'), ('host', 60695184, 58, 'FNetPlayer.constructor', 'abd4fa57f66c7de743ad4df2b6698672ca152e888652a79b12351919a71f14c0'), ('host', 60763318, 62, 'FNetSession.IsIgnoreIncomingMessages', '63cab8d4ea8fb85d3190dece7eb724e13bab34b6085057f106c10cf2a89b8e83'), ('host', 60768916, 732, 'FNetSession.PlayerAppRegistered', '44c868dedaada42294f0f2772f24bc5fb0b8fb60f181eb7fc9c3f48428d1562a')]
SLOTS = [
    ('host', 0x53cc788, 0x18, 0x3c5438c, 'MemoryOStream.length_is_GetOffset'),
    ('host', 0x53a9ad0, 0x28, 0x33af7f0, 'NetGameCoreMessage.Execute'),
    ('host', 0x53b5048, 0x10, 0x39ddfee, 'ENet.Send'),
    ('host', 0x53b5048, 0x38, 0x39de160, 'ENet.Update'),
    ('host', 0x53b4280, 0x10, 0x39ada8c, 'ENetEOS.Send'),
    ('host', 0x53b4280, 0x38, 0x39ad1fe, 'ENetEOS.Update'),
    ('host', 0x53b71b0, 0x10, 0x3a06b58, 'ENetSteam.Send'),
    ('host', 0x53b71b0, 0x38, 0x39de160, 'ENetSteam.Update'),
    ('gamecore', 0x3aff870, 0x100, 0x2da2af0, 'Manager.MakeRequest'),
    ('gamecore', 0x3aff870, 0xa0, 0x2da1720, 'Manager.GetHandlerIDFromHash'),
    ('gamecore', 0x3aff870, 0x88, 0x2da1650, 'Manager.GetHandler'),
    ('gamecore', 0x3b00da0, 0x78, 0x2d96720, 'SkipTurn.MakeRequest'),
    ('host', 0x53a6c80, 0x78, 0x32b4d00, 'EngineUtility.SendMessage(stream)'),
    ('host', 0x53b5600, 0xd8, 0x39e99f4, 'FNetSession.Update(float)'),
    ('host', 0x53b5600, 0x238, 0x39f2f32, 'FNetSession.ProcessNetwork'),
    ('host', 0x53b5600, 0x248, 0x39f2d1a, 'FNetSession.HandleMessage'),
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
    field_instructions = [(0x39de186, '49837f4000', 'provider_packet_count_u64_at_0x40'),
                          (0x39de1bb, '498b7f08', 'provider_callback_pointer_at_0x8'),
                          (0x39f29e7, '80782900', 'sync_deferral_byte_at_0x29'),
                          (0x39f2d1d, '817e1ce8030000', 'message_type_i32_at_0x1c'),
                          (0x39e0228, '89771c', 'constructor_message_type_i32_at_0x1c'),
                          (0x3c54386, '488b4708c3', 'stream_data_pointer_at_0x8'),
                          (0x3c5438c, '488b4710c3', 'stream_length_u64_at_0x10'),
                          (0x2f636c0, '8b4710c3', 'vector_size_u32_at_0x10'),
                          (0x2f63d90, '488b07c3', 'vector_data_pointer_at_0x0')]
    for address, expected, name in field_instructions:
        if elves['host'].read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('field instruction differs: '+name)
    if struct.unpack('<Q', elves['host'].read(0x535e160,8))[0] != 0x2c87e80:
        raise ValueError('host application-message loader initializer differs')
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
        field_instructions=[dict(address=hex(v),bytes=b,name=n,sha256=hashlib.sha256(bytes.fromhex(b)).hexdigest()) for v,b,n in field_instructions],
        passive_stream_capture=dict(data_pointer_offset=8,length_u64_offset=16,length_source='original GetOffset via virtual slot0x18'),
        passive_net_gamecore_capture=dict(vptr='0x53a9ae0',type_i32_offset=28,expected_type=3,data_pointer_offset=64,size_u32_offset=80),
        passive_message_factory=dict(singleton='0x653d530',bucket_pointer_offset=8,bucket_count_u32_offset=16,node_key_i32_offset=0,node_creator_pointer_offset=8,node_next_pointer_offset=16,expected_key=3,expected_creator='0x33afde0',init_array_entry='0x535e160',initializer='0x2c87e80'),
        message_received_entry='0x39f2062; receiver RDI, message RSI, bool DL; 0x39f29da is an interior branch',
        player_receive_gate=dict(needs_snapshot_byte_offset=30,needs_snapshot_getter='0x33e0ff0',needs_snapshot_setter='0x39e231c',adjacent_byte_offset=31,adjacent_byte_semantics='unclassified; set after original SendSnapshotData dispatch',ignore_incoming_getter='0x39f2cb6',constructor_needs_snapshot=True,constructor_adjacent_byte=0,original_app_registered='0x39f4294'),
        local_delivery='FNetAccessENet.Send(connection=0) queues packet; provider.Update drains via provider+8 callback slot0x30; absent callback destroys packet',
        synchronization_deferral='MessageReceived queues Sync.Add when sync+0x29 is nonzero and message+8 is zero',
        update_deferral='FNetMessageSync.Update false or pending connection queue update false skips session incoming/outgoing drains',
        session_start_policy_safety='Not established transitively; LaunchGame broadcasts NetLaunchGame and live callbacks require audit',
        native_execution=False, action_executed=False, gameplay_parity=False,
        scope='Static delivery ABI only; runtime failing gate remains unobserved')
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods_checked=len(METHODS),native_execution=False)))


if __name__ == '__main__':
    main()
