#!/usr/bin/env python3
"""Read-only gameplay boundary/request-order evidence, not a native gameplay parity test."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

METHODS = [('gamecore', 41968032, 212, 'CacheUnit.RequestOperation', '218e087b8d1253ca44100a3d5fd016c1452d5b4af9e80f4123620c225b1068ae'), ('gamecore', 47332848, 664, 'Messages.Broadcast', '26bd66fc177e4f08efd1aa0a47f871be695a253e48fbaf721415e23b8467bead'), ('gamecore', 47333520, 140, 'Messages.HandleTurnUnready', 'f8e776e5bfb36d5c2f45c042884daa2bfa0e1434e6fb4f1c7eefcb9e437fc9df'), ('gamecore', 47353344, 3, 'UnitOperation.UnreadiesTurn', 'e18aacce29affcdddaa5f641ac1ec12552e0fe9d066e6408941b1792818b8619'), ('gamecore', 45291264, 50, 'Player.CanUnreadyTurn', 'e9c7cf9fb3972208e3699cf0aa53d22a5d3b4e9771a879936350224deac826c5'), ('gamecore', 45464320, 89, 'Player.RequestUnready', '47d3b5b3a4af49f22315b320036a84faac228e18a11b2b29cd7495f1bc095999'), ('gamecore', 44389008, 37, 'Phase.AllowUnready', 'd6a34a792b873c6d60a34a7f43095a68bd161eb34ee7599001f4b7091d9542a4'), ('gamecore', 47350208, 688, 'PlayerTurnUnready.Execute', '9cb05c7a8c75089db955c8fa47cee0f70f61e8e91fdfb732e74861de6f7a23e6'), ('gamecore', 46647792, 8, 'Context.SetEngineInterface', '0c271f377597ae9000f41b6f04dfaa5fd75d08514f5cdfca3e31c2d3c163190e'), ('gamecore', 47485296, 14, 'Reporting.Enable', '3072154cc27f6ac23e6a7f1a29dc4838eff35cd751ea64c044328f0d4beec166'), ('gamecore', 41041648, 14, 'Cache.EnableUpdates', '4b866522f69419dc340fbf4726eb83e7dc349fc077a47f030ff2c005a6ae8bc7'), ('gamecore', 46649984, 7, 'Context.EnableCacheUpdates', '0243d74274dcdbc554639b2563ec0f012e5a9ea7eb86437cdbe8c800bab3ffbc'), ('gamecore', 46555344, 26, 'YieldMonitor.Initialize', '44aaaaec133f5a8c06c89e466e2e48dea630c32d51c799fcfd5c9ac76b902c99'), ('gamecore', 46555824, 276, 'YieldMonitor.RegisterEvents', '71bfc965ffa3b480e51f9a8ca3260f8e69c95db900a0049cc3d5d92c8e8778e6'), ('gamecore', 46556112, 89, 'YieldMonitor.Uninitialize', '5a8c0e3be6f66506b423b0bbf32a63222f662ad5d7c61b999727839ca141175c'), ('host', 53165568, 1099, 'Engine.RequestUnready', 'c7bced8be81cccf7eee01a82ec4e0f0ec2771c43d0c4b799d0cbe3a6be6f9d77'), ('host', 53056624, 19, 'UI.SkipNextAutoEndTurn', 'b7723de37b934c060f00b654aa59736b2fdf794c1124a373b6903a5860271e05'), ('host', 49880400, 3, 'App.SetupDedicatedServer', 'e18aacce29affcdddaa5f641ac1ec12552e0fe9d066e6408941b1792818b8619'), ('host', 49908848, 12, 'App.SetUIOff', '714ee406a4fe0ca0bb36eb6a6fb2403970010f6140f658af2d3041a9dbd21a4d'), ('host', 53169184, 7, 'Engine.RegisterEvent', '327a0fd92d44623a9328591e30960e82cc73957eb763dc523e0b8e447311c0b6'), ('host', 53169200, 11, 'Engine.RegisterEventParameter', '3d9abe8ec987897eaed89fb87d7db6bd75ec6702396f212c98dcfe67dd72a6f3')]


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--assets',type=Path,default=Path(__file__).resolve().parents[3]/'c6-assets-linux');p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    modules={'gamecore':Elf(a.assets/'libGameCore_XP2.so',GAME_SHA),'host':Elf(a.assets/'Civ6',HOST_SHA)}
    for m,address,size,name,expected in METHODS:
        if hashlib.sha256(modules[m].read(address,size)).hexdigest()!=expected:raise ValueError('original boundary method differs: '+name)
    game=modules['gamecore']
    calls=[(0x2d23e1d,0x1e11220,'Broadcast handles unready before serialization'),
           (0x2d240be,0x1e3ec50,'Handle tests actual player CanUnready'),
           (0x2d240cf,0x1f14ff0,'Handle invokes phase/provider unready request'),
           (0x2d240e0,0x1e5d5a0,'Handle constructs genuine PlayerTurnUnready'),
           (0x2d240eb,0x1f2e170,'Handle sends unready through same native delegate'),
           (0x2b31704,0x1e317a0,'CanUnready tests live active first'),
           (0x2b5bb14,0x1e321f0,'Request checks current phase permission'),
           (0x2d28214,0x1e61790,'Unready message clears turn complete'),
           (0x2d28221,0x1f89000,'Unready message reactivates player')]
    for address,target,_ in calls:game.call(address,target)
    if game.read(0x2b3170b,2)!=bytes.fromhex('7518'):raise ValueError('active player CanUnready false branch differs')
    if game.read(0x2d28e00,3)!=bytes.fromhex('b001c3'):raise ValueError('UnitOperation unreadies property differs')
    report=dict(schema='c6-linux-gameplay-oracle-scope-v1',checks_passed=True,
        host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
        methods=[dict(module=m,address=hex(v),size=n,name=s,sha256=h)for m,v,n,s,h in METHODS],
        calls=[dict(address=hex(v),target=hex(t),contract=s)for v,t,s in calls],
        boundary_decision='Trusted authoritative external decisions -> exact gameplay request semantics -> original Core queue/dispatch/upkeep -> next external decision',
        full_desktop_ui_required_for_gameplay_semantics=False,full_network_peer_admission_required_for_trusted_local_boundary=False,
        request_side_effects_required=True,complete_logical_gameplay_state_and_rng_required=True,
        gameplay_affecting_callbacks_required=True,physical_heap_layout_equality_required=False,
        exclusions_require_ownership_consumers_effects_evidence=True,unknown_save_bytes_remain_unmasked=True,
        genuine_global_headless_equivalence_switch_proven=False,
        current_fixture_unready=dict(local_player_active_observed=True,can_unready_false_inferred_from_original_first_branch=True,native_CanUnready_direct_observation_available=False,
            proposed_observation='Read-only original CanUnreadyTurn on genuine local player at same quiescent pre-normalization boundary; expectfalse; preserve independent purity observations'),
        unsupported_operations_must_reject_before_mutation=True,full_gameplay_state_action_turn_supported=False,
        native_execution=False,gameplay_parity=False)
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(checks_passed=True,original_methods=len(METHODS),native_execution=False,gameplay_parity=False)))


if __name__=='__main__':main()
