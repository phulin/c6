#!/usr/bin/env python3
"""Exact Linux static lifecycle contracts; no original execution or state mutation."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from skip_turn_state_aftermath_audit import audit as aftermath_audit

ROOT = Path(__file__).resolve().parents[2]
GAME_DEFAULT = ROOT.parent/'c6-ghidra/evidence/linux-rules/asl-launch-xp2-2/libGameCore_XP2.so'
HOST_DEFAULT = ROOT.parent/'c6-assets-linux/Civ6'

def sha(b):
    return hashlib.sha256(b).hexdigest()

def rel(elf, at, target, opcode=0xe9):
    b=elf.read(at,5)
    if b[0]!=opcode or at+5+struct.unpack_from('<i',b,1)[0]!=target:
        raise ValueError('relative transfer differs: '+hex(at))

def audit(module, host_path):
    game,host=Elf(module,GAME_SHA),Elf(host_path,HOST_SHA)
    parent=aftermath_audit(module,host_path)
    lines=subprocess.check_output(['nm','-D','-S','--defined-only',str(module)],text=True).splitlines()
    tables=[(int(x.split()[0],16),x.split()[-1]) for x in lines
            if '_ZTVN8GameCore9Reporting12Notification' in x and not x.split()[-1].endswith('7ManagerE')]
    if len(tables)!=39: raise ValueError('notification subtype inventory differs')
    slots={8:'deleting_destructor',0x20:'release',0x28:'is_expired',0x30:'is_needed',0x40:'auto_expires',0x50:'get_blocking',0x78:'initialize',0x88:'activate',0x98:'dismiss'}
    dwarf=subprocess.check_output(['gdb','-q','-nx','-batch',str(module),'-ex',"ptype /o 'GameCore::Reporting::Notification::Instance'",'-ex',"ptype /o 'GameCore::Reporting::Notification::PlayerNotifications'",'-ex',"ptype /o 'GameCore::Reporting::Notification::Manager'",'-ex',"ptype /o 'GameCore::Reporting::Notification::PoolType'"],text=True)
    for field in ['m_szMessage','m_szSummary','m_eDefaultEndTurnBlockingType','m_pkParent','m_iLockCount','m_bDismissed','m_bIsExpired','total size (bytes):  416','total size (bytes):  864','total size (bytes):  576']:
        if field not in dwarf: raise ValueError('exact target DWARF field missing: '+field)
    bindings=[]
    for address,name in tables:
        row=dict(vtable_rva=hex(address),symbol=name,slots={})
        for slot,kind in slots.items():
            target=game.relocated_pointer(address+16+slot)
            row['slots'][kind]=hex(target)
            if kind=='release' and target!=0x2d2de30: raise ValueError('Release override requires review')
            if kind=='auto_expires' and target!=0x2d2e2b0: raise ValueError('AutoExpires override requires review')
            if kind=='dismiss' and target!=0x2d2e450: raise ValueError('Dismiss override requires review')
            if kind=='activate' and target not in [0x2d2dd80,0x2d2bfd0,0x2d314a0]: raise ValueError('Activate override requires review')
        destructor=int(row['slots']['deleting_destructor'],16)
        game.call(destructor+4,0x1eeae40 if destructor==0x2d31e20 else 0x1e36190)
        rel(game,destructor+13,0x1e79f60)
        row['deleting_destructor_sha256']=sha(game.read(destructor,18))
        bindings.append(row)
    # Exact branches, scalar widths/order and virtual dispatch surfaces.
    checks=[('game',0x2cd0310,'488d4720c3','message getter points20'),
            ('game',0x2cd0320,'488d87a0000000c3','summary getter pointsa0'),
            ('game',0x2d2fa70,'488b4708482b0748c1e803c3','manager vector pointer stride8'),
            ('game',0x2d2e8d0,'488b4708482b0748c1e803c3','pool vector pointer stride8'),
            ('game',0x2722db0,'488b074883c010c3','list node payload16'),
            ('game',0x2722e00,'488b07488b004889074889f8c3','list iterator next0'),
            ('game',0x2d2de38,'488bbb38010000','pool pointer load'),
            ('game',0x2d2de42,'7420','null pool deletes'),
            ('game',0x2d2de53,'48c7833801000000000000','clear live pool after transfer'),
            ('game',0x2d2de6f,'ff6008','nonpooled deleting destructor dispatch'),
            ('game',0x2d2e5d7,'7415','empty pool invokes allocator'),
            ('game',0x2d2e60a,'ff5078','reuse calls actual Initialize'),
            ('game',0x2d2d87b,'48c7839001000001000000','reuse count=1 lock=0'),
            ('game',0x2d2da76,'c7833001000000000000','reuse blocking zero before definition'),
            ('game',0x2d2e2b0,'83bf8c01000000','AutoExpires tests expiry delta'),
            ('game',0x2d2e2b7,'0f9fc0','AutoExpires signed positive'),
            ('game',0x2d2e26b,'8b8b8c010000038b88010000','expiry wrapping added turn plus delta'),
            ('game',0x2d2e279,'0f9dc0','expiry signed current >= sum'),
            ('game',0x2d2e291,'80bf9a01000000','IsNeeded flag19a'),
            ('game',0x2d2e29d,'ff5028','IsNeeded actual expiration'),
            ('game',0x2d2e3b8,'80bb9801000000','blocking dismissed gate'),
            ('game',0x2d2e3ce,'8bab30010000','omitted blocking field130'),
            ('game',0x2d310c1,'ff5030','blocking actual IsNeeded'),
            ('game',0x2d310ce,'ff5050','blocking actual GetEndTurnBlocking'),
            ('game',0x2d310ee,'7603','unsigned severity first tie retained'),
            ('game',0x2d31115,'ff5050','final chosen blocking reevaluation'),
            ('game',0x2d2e45a,'c6839801000001','Dismiss writes dismissed first'),
            ('game',0x2d2e466,'be20c0d19a','Dismiss event9ad1c020'),
            ('game',0x2d314a7,'80bb9c01000000','PlayerDefeated one-shot native19c gate'),
            ('game',0x2d2bfda,'ff5028','DiplomacySession expiration precondition'),
            ('host',0x3964e01,'443933','FindEntry compares key in vector order'),
            ('host',0x3964e04,'75ea','FindEntry first match retained'),
            ('host',0x3212d1b,'3d3dc34764','activation translator EventData type'),
            ('host',0x3212d46,'3d00040000','activation ID custom type1024'),
            ('host',0x3212d65,'be46f9a278','activation bool key78a2f946'),
            ('host',0x321cae7,'740b','null prepublish skips synchronous callback')]
    for source,address,expected,meaning in checks:
        if (game if source=='game' else host).read(address,len(bytes.fromhex(expected))).hex()!=expected:
            raise ValueError('branch differs: '+meaning)
    calls=[('game',0x2d2de4e,0x1e2c9d0),('game',0x2d2e5dc,0x1e79f10),('game',0x2d2e5e7,0x1ed81c0),
           ('game',0x2d2e613,0x1e408a0),('game',0x2d2d799,0x1fb89a0),('game',0x2d2d7a5,0x1e2f960),
           ('game',0x2d2e488,0x1eaf290),('game',0x2d31508,0x1f5d780),
           ('host',0x3964e87,0x3964dd0),('host',0x3212d8c,0x3216580),('host',0x3212d97,0x321cac0),
           ('host',0x321caef,0x3237c40),('host',0x321cafa,0x3237c80),('host',0x321cb01,0x2fd7e20)]
    for source,address,target in calls: (game if source=='game' else host).call(address,target)
    rel(game,0x2d2d7b2,0x1e2f960)
    rel(game,0x2d2c04a,0x1f9bfc0)
    methods=[('game','Instance.Release',0x2d2de30,0x42),('game','Instance.Destruct',0x2d2d780,0x37),
             ('game','Instance.CreatePooled',0x2d2e5b0,0x7a),('game','Instance.Initialize',0x2d2d7e0,0x520),
             ('game','Pool.Clear',0x2d2e670,0x50),('game','Instance.IsExpired',0x2d2e250,0x32),
             ('game','Instance.AutoExpires',0x2d2e2b0,0xb),('game','Instance.IsNeeded',0x2d2e290,0x1c),
             ('game','Instance.GetBlocking',0x2d2e3b0,0x2d),('game','PlayerNotifications.GetBlocking',0x2d31060,0xc9),
             ('game','Instance.Dismiss',0x2d2e450,0x45),('game','Instance.GetLock',0x2d2e4d0,0x10),
             ('game','Instance.ReleaseLock',0x2d2e4e0,0x18),('game','PlayerDefeated.Activate',0x2d314a0,0x7d),
             ('game','DiplomacySession.Activate',0x2d2bfd0,0x84),('game','PromoteUnit.Destruct',0x2d31df0,0x27),
             ('game','TypedVariant.Bool',0x263d350,0x87),('host','TypedVariant.Bool',0x305ed60,0x87),
             ('host','VectorMap.FindEntry',0x3964dd0,0x3e),('host','VectorMap.GetVariant',0x3964e80,0x35),
             ('host','VectorMap.Destruct',0x3964cc0,0x50),('host','TypedVariant.DestroySelf',0x395dd10,0xb6),
             ('host','Localization.Destruct',0x3be0800,0x3a),('host','Translate.NotificationActivated',0x3212d00,0xa4)]
    return dict(schema='c6-linux-notification-lifecycle-static-v1',checks_passed=True,static_only=True,native_execution=False,
        gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,source_audit_sha256=sha(Path(__file__).read_bytes()),
        inherited_aftermath_source_sha256=sha((ROOT/'tools/linux/skip_turn_state_aftermath_audit.py').read_bytes()),
        inherited_selection_contract=parent['selection'],update_order=parent['update_order'],
        virtual_bindings=bindings,checked_branches=[dict(module=m,rva=hex(a),meaning=n) for m,a,_,n in checks],
        checked_calls=[dict(module=m,rva=hex(a),target=hex(t)) for m,a,t in calls],
        methods=[dict(module=m,name=n,rva=hex(a),bytes=z,sha256=sha((game if m=='game' else host).read(a,z))) for m,n,a,z in methods],
        target_dwarf_sha256=sha(dwarf.encode()),
        layout=dict(instance_bytes=416,player_notifications_bytes=864,manager_bytes=576,pool_bytes=32,
          instance=dict(identity8='i16 owner/i16 type/u32 id',message='0x20',summary='0xa0',default_blocking='0x130',parent_pool='0x138',lock_count='0x194'),
          flags=dict(offset198='Dismissed',offset199='CanUserDismiss',offset19a='BlocksTurnAdvancement',offset19b='IsAutoNotify',offset19c='IsExpired',offset19d='IsAutoActivate',offset19e='HasBeenActivated'),
          player_notifications=dict(owner=0,next_id=4,list_sentinel=8,node_next=0,node_prev=8,node_payload=16,size='computed by traversal, not a stored count'),
          manager_players=dict(vector_offset=24,begin=0,end=8,element_bytes=8),pool_vector=dict(begin=0,end=8,element_bytes=8),
          text_correction='Historical wire adapter/codec metadata reversed message/summary labels; target DWARF plus compiled getters prove first locale20=message, seconda0=summary. Raw bytes and cleanup classification unchanged.',
          already_dismissed_classification='unchanged: Cleanup uses198 IsDismissed, not19c IsExpired. Base IsExpired does not read19c.'),
        cleanup=dict(order='native list order; Release before erase; retain surviving order and next ID',
          pooled='append same logical object to its pool, clear its pool link; retain payload until reuse or Clear',
          unpooled='call concrete deleting destructor; base payload destroys parameter map, message and summary',
          destructor_overrides='38 base-payload-only deleting wrappers; PromoteUnit additionally destroys ComponentID vector at1a0',
          native_addresses_required=False,logical_ownership_and_order_required=True,
          observed_pool_ownership=False,allocator_only_exclusion_proven_for_all_subtypes=False),
        reuse=dict(order=['pool empty: allocate through provided creator; else pop last object','actual virtual Initialize','SetPool'],
          base_reset=['lock194=0/count190=1','blocking130=0','dismissed198=false/userdismiss199=true/flag19a=false','19c/19d/19e=false','added188=-1/delta18c=0','type/severity/group','location/target/parameters/summary/message'],
          native_allocator_addresses_irrelevant=True,
          limit='Logical retained ownership can be modeled without native addresses. Full pool-state elimination requires initialized subtype/reset and ID assignment/lifetime proof; no such blanket exclusion is claimed.'),
        expiration=dict(base='delta18c>0 && signed current_game_turn >= wrapping_i32(added188+delta18c)',
                        native_rng_directly_called=False,subtype_overrides=30,base_vtables=9),
        blocking=dict(base_needed='!(flag19a && actual virtual IsExpired)',
          base_value='dismissed || actual virtual IsExpired ? 0 : live blocking130',
          reduction='first nonzero blocking among actual IsNeeded candidates; replace only with strictly greater UNSIGNED severity; evaluate selected blocking again',
          omitted_instance_field='signed enum/raw int130',omitted_player_field='old blocking7a8',
          event='55f15a7b old,new only when changed and owner is local'),
        activation=dict(base_vtables=37,base='write19e=true; general event025f6cfd(ID eab69e5a, incoming bool78a2f946)',
          overrides=dict(PlayerDefeated='19c one-shot gate; optional RequestSession(player,local,statement07ad627c,null); actual virtual Dismiss',
                         DiplomacySession='not expired, to-player alive, !(human && paused), turn-active: set19c then ActivateStatement; otherwise return'),
          native_rng_directly_called_in_base=False,transitive_callback_rng_coverage=False,
          reporting='enabled/not-suspended -> synchronous Cache.Dispatcher -> EngineUtility -> registered translator -> optional synchronous prepublish -> queued event257 KeepOrder',
          typed_override_lookup='first matching key in vector order; absent default variant; canonical bool low byte !=0 if nonarray type1',
          complete_subscriber_effects_proven=False),
        required_live_inputs=parent['required_inputs']+['Instance end-turn blocking130','Logical pool ownership/order and supported subtype destructor/reinitialize contract'],
        rust_handoff=dict(supported_candidate='explicit base-method profile with real live lock/blocking/definition inputs, ordered logical ownership and synchronous effect preflight',
          unsupported_before_mutation=['unknown subtype virtual methods','unknown variant/payload destruction support','unbound event effects when claiming full action','missing live locks/blocking/configuration/definition inputs'],
          diagnostic_projection='may compare wire effects with scenario inputs, but must not promote them to observed native live inputs or full command acceptance'),
        limits='Static contracts and existing frozen wire effects only. No new native runs. Complete gameplay state/action/turns, all gameplay RNG and event subscribers remain unsupported. Raw Save→Load 89,198-byte failure remains unclassified and unmasked.',
        full_action_acceptance=0,gameplay_parity=False)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--module',type=Path,default=GAME_DEFAULT);p.add_argument('--host',type=Path,default=HOST_DEFAULT);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    result=audit(a.module,a.host);a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,virtual_tables=len(result['virtual_bindings']),methods=len(result['methods']))))
if __name__=='__main__': main()
