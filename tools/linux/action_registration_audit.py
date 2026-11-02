#!/usr/bin/env python3
"""Gate the original inactive-core host registration handshake providers."""
import argparse
import hashlib
import json
from pathlib import Path
from action_delivery_audit import METHODS
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA


BORROWED_GETTERS = [(50610944, 5, 'Network.GetSession', '8e3eaf4f4b235f364076d47e3b5b95d9e60a71cf19dd62030d0fa20e80858d2e'), (60629396, 5, 'FNetSession.GetSyncManager', '142aede9b1c13ca9a9e85fa39b4ea11d5b695b949b3df7b174f93ae9608f6ad2'), (60742466, 42, 'FNetSession.GetHostPlayerID', '1432b5724843e6d7b25f1e7ea4352eda569433da29860f32ce1f0518c112e887'), (60742424, 42, 'FNetSession.GetLocalPlayerID', '68ada90d2c34f08e3ac707cc8723cfb8fa485cd9536a74d4c927a415bd3d18ce'), (60742754, 43, 'FNetSession.IsHost', '4329eccdab7df608d7fc10b83faedd22eff97614c54ad7216e20e66156295aaf'), (60742508, 59, 'FNetSession.GetPlayer', 'ea78b2ff0029742b4745a356180c402ec212f8e6188315ba83fcaf4855cd6f7d')]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for module, address, size, name, digest in METHODS:
        elf = host if module == 'host' else game
        if hashlib.sha256(elf.read(address, size)).hexdigest() != digest:
            raise ValueError('original delivery provider differs: ' + name)
    for address, size, name, digest in BORROWED_GETTERS:
        if hashlib.sha256(host.read(address, size)).hexdigest() != digest:
            raise ValueError('original borrowed getter differs: ' + name)
    getters = [(0x33e2170, '8a471dc3', 'FNetPlayer.GetAppRegistered'),
               (0x33e0ff0, '8a471ec3', 'FNetPlayer.GetNeedsSnapshot')]
    for address, expected, name in getters:
        if host.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('original peer getter differs: ' + name)
    report = dict(schema='c6-linux-original-registration-handshake-abi-v1',
                  host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA, checks_passed=True,
                  delivery_provider_methods_checked=len(METHODS),
                  borrowed_session_getters=[dict(address=hex(a),size=n,name=k,sha256=d) for a,n,k,d in BORROWED_GETTERS],
                  peer_getters=[dict(address=hex(address), bytes=expected, name=name) for address, expected, name in getters],
                  entry='existing real host session; original CoreActive must remain false until app registration finishes',
                  protocol='original Network.Update processes existing NetRegister1208; original inactive-core callback publishes event55; original handler constructs and broadcasts NetAppRegister1209(false); original host PlayerAppRegistered clears NeedsSnapshot',
                  completion='actual original GetAppRegistered=true, GetNeedsSnapshot=false, IsIgnoreIncomingMessages=false, known original provider queue empty; bounded to eight original updates',
                  direct_registration_call=False, fabricated_events_or_messages=False, flag_writes=False,
                  native_execution=False, gameplay_parity=False,
                  limits='Original event subscribers remain guarded but not proven globally policy-free. This audit does not replace the dynamic handshake/control/Save/RNG case.')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, native_execution=False)))


if __name__ == '__main__':
    main()
