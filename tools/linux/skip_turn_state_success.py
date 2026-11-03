#!/usr/bin/env python3
"""Classify the closed quiescent SkipTurn fixture's entire serialized footprint.

No original code is executed. Original before-owner boundaries, production
typed codecs, exact framing and equality of every intervening byte establish
alignment. This fixture profile rejects any extra changed field/region.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile

from core_component_check import validate, require, digest, raw_live_differences, typed_observation
from save_lifecycle_audit import Elf, GAME_SHA
from skip_turn_state_audit import DECODER, ROOT
from unit_codec_check import TRACES, SAVES


NOTIFICATIONS = r'''
use c6_core::unit::variants::{Limits,DecodeBudget,TypedVariantVectorMap};
use std::{fs,env};
struct R<'a>{b:&'a[u8],p:usize}
impl<'a> R<'a>{
 fn take(&mut self,n:usize)->&'a[u8]{let b=&self.b[self.p..self.p+n];self.p+=n;b}
 fn u32(&mut self)->u32{u32::from_le_bytes(self.take(4).try_into().unwrap())}
 fn i32(&mut self)->i32{self.u32() as i32}
 fn byte(&mut self)->u8{self.take(1)[0]}
 fn string(&mut self)->(usize,usize){
  assert_eq!(self.u32(),u32::MAX);assert_eq!(self.byte(),1);
  let n=self.u32() as usize;let start=self.p;self.take(n);
  assert_eq!(self.byte(),0,"unsupported localized arguments");
  assert_eq!(self.byte(),0,"unsupported named arguments");(start,n)
 }
}
fn main()->Result<(),Box<dyn std::error::Error>>{
 let a:Vec<_>=env::args().collect();let b=fs::read(&a[1])?;let mut r=R{b:&b,p:0};
 assert_eq!(r.u32(),1);let owner=r.i32();let next=r.u32();let count=r.u32();
 println!("{{\"kind\":\"header\",\"owner\":{owner},\"next_id\":{next},\"count\":{count}}}");
 for index in 0..count{
  let start=r.p;let ty=r.u32();assert_eq!(r.u32(),5);
  let owner=i16::from_le_bytes(r.take(2).try_into().unwrap());
  let component_type=i16::from_le_bytes(r.take(2).try_into().unwrap());let id=r.u32();
  assert_eq!(r.u32(),ty);let severity=r.i32();let summary=r.string();let message=r.string();
  let location=[r.i32(),r.i32()];let component=r.take(8).to_vec();let ints=[r.i32(),r.i32()];
  let flags=r.take(3).to_vec();let i190=r.i32();let flags2=r.take(2).to_vec();
  let mut budget=DecodeBudget::new(Limits::default());
  let parameter_start=r.p;let (map,n)=TypedVariantVectorMap::decode_prefix(&r.b[r.p..],&mut budget)?;
  assert_eq!(map.encode(Limits::default())?.0,r.b[r.p..r.p+n]);r.p+=n;
  let flags3=r.take(2).to_vec();
  assert!(flags.iter().chain(&flags2).chain(&flags3).all(|x|*x<=1));
  println!("{{\"kind\":\"notification\",\"index\":{index},\"start\":{start},\"end\":{},\"type_hash\":{ty},\"owner\":{owner},\"component_type\":{component_type},\"id\":{id},\"severity\":{severity},\"summary_range\":[{},{}],\"message_range\":[{},{}],\"location\":{:?},\"component_bytes\":{:?},\"native_188_18c\":{:?},\"native_198_199_19a\":{:?},\"native_190\":{i190},\"native_19b_19c\":{:?},\"native_19d_19e\":{:?},\"is_dismissed\":{},\"has_been_activated\":{},\"parameters_range\":[{parameter_start},{}],\"parameters_debug\":{:?}}}",r.p,summary.0,summary.0+summary.1,message.0,message.0+message.1,location,component,ints,flags,flags2,flags3,flags[0]!=0,flags3[1]!=0,parameter_start+n,format!("{map:?}"));
 }
 assert_eq!(r.p,b.len());Ok(())
}
'''


def sha(data):
    return hashlib.sha256(data).hexdigest()


def original_contract(module):
    elf = Elf(module, GAME_SHA)
    require(elf.read(0x2d313c0, 7).hex() == '8a879e010000c3', 'HasBeenActivated field differs')
    require(elf.read(0x2722dd0, 7).hex() == '8a8798010000c3', 'IsDismissed field differs')
    require(elf.read(0x2d2dd8e, 7).hex() == 'c6839e01000001', 'Activate tracked bool store differs')
    require(elf.read(0x2ab70e0, 11).hex() == '83bf94010000000f9fc0c3', 'omitted live lock count predicate differs')
    require(struct.unpack('<I', elf.read(0x2f86e40, 4))[0] == 5, 'Notification version differs')
    require(struct.unpack('<I', elf.read(0x2f86f7c, 4))[0] == 1, 'PlayerNotifications version differs')
    for call, target in [(0x2d305d8, 0x1f5fbd0), (0x2d305e3, 0x1e8d360),
                         (0x2d305ef, 0x1e20270), (0x2d3060c, 0x1f5fbd0),
                         (0x2d3066a, 0x1e20270), (0x2d2e1ad, 0x1e24a60),
                         (0x2d2dde4, 0x1e578a0), (0x2d3123b, 0x1fb8d00)]:
        elf.call(call, target)
    require(elf.read(0x2d30680, 3).hex() == 'ff5070', 'notification virtual writer slot differs')
    selected_report_calls = [(0x2da1897, 0x1fb5910, 'e23b112d', 'SetActivityType void epilogue'),
                             (0x2d9650b, 0x1eea180, 'a60328c5', 'Immediate xor eax,eax returns false'),
                             (0x2da1d5d, 0x1eaf290, '383669f1', 'PopFront void epilogue'),
                             (0x2da263f, 0x1fb5910, '15c35863', 'Original GetSize call overwrites eax before compare'),
                             (0x2dac91d, 0x1fb5910, '3d4006f2', 'Original Manager.Edit call overwrites rax before use')]
    for call, target, _, _ in selected_report_calls:
        elf.call(call, target)
    require(elf.read(0x2d96510, 2).hex() == '31c0', 'Deactivate report return consumption differs')
    elf.call(0x2da2647, 0x1f22ec0)
    elf.call(0x2dac922, 0x1fb5340)
    symbols = subprocess.check_output(['nm', '-D', '--defined-only', str(module)], text=True)
    tables = [(int(line.split()[0], 16), line.split()[-1]) for line in symbols.splitlines()
              if '_ZTVN8GameCore9Reporting12Notification' in line]
    writers = []
    for address, name in tables:
        if name.endswith('7ManagerE'):
            continue
        writer = elf.relocated_pointer(address + 16 + 0x70)
        require(writer == 0x2d2e0b0, 'notification writer override needs separate framing: ' + name)
        writers.append(dict(vtable_rva=hex(address), symbol=name, write_rva=hex(writer)))
    require(len(writers) >= 30, 'notification writer inheritance coverage missing')
    methods = [('PlayerNotifications.Write', 0x2d305c0, 0xf6),
               ('Notification.Write', 0x2d2e0b0, 0x12a), ('Localization.String.Write', 0x2d4e3c0, 0x315),
               ('Notification.Activate', 0x2d2dd80, 0x86), ('Notification.Cleanup', 0x2d311e0, 0xcb),
               ('Player.UpdateEndTurnBlocking', 0x2b5bd70, 0x9b), ('Notification.Update', 0x2d2f360, 0x67),
               ('Notification.AcceptsNotifications', 0x2d2f000, 0x3a), ('Notification.SendAutoNotify', 0x2d2f4b0, 0xec),
               ('Notification.ProcessAutoActivate', 0x2d2f5a0, 0x97), ('Notification.FindAutoActivate', 0x2d30ec0, 0x110),
               ('Notification.IsAutoActivate', 0x2d2e1f0, 0x58), ('Notification.IsLocked', 0x2ab70e0, 0xb),
               ('Notification.DismissExpired', 0x2d31130, 0xa1)]
    return dict(gamecore_sha256=GAME_SHA, inherited_writers=writers,
                methods=[dict(name=name, rva=hex(rva), bytes=size, sha256=sha(elf.read(rva, size)))
                         for name, rva, size in methods],
                general_activation_event_hash='025f6cfd', activation_event_overload_captured=False,
                selected_identity_reporting_calls=[dict(call_rva=hex(call), plt_rva=hex(target), event=event, subsequent_use=use)
                                                   for call, target, event, use in selected_report_calls],
                omitted_live_lock_count=dict(offset='0x194', predicate='signed i32 > 0', serialized=False),
                unclassified_scalar_names='Native offsets retain unknown semantics; no guessed bool/int labels.')


def formation_end(data, start):
    require(struct.unpack_from('<I', data, start)[0] == 1, 'unsupported Formation version')
    nodes = struct.unpack_from('<I', data, start + 36)[0]
    require(nodes <= 100000, 'Formation path resource bound')
    count_pos = start + 48 + nodes * 20
    ids = struct.unpack_from('<I', data, count_pos)[0]
    require(ids <= 100000, 'Formation membership resource bound')
    return count_pos + 4 + ids * 4


def compile_tool(source, name, directory, target):
    source_path, binary = directory / (name + '.rs'), directory / name
    source_path.write_text(source)
    subprocess.run(['rustc', '--edition=2021', str(source_path), '--extern',
                    'c6_core=' + str(target / 'libc6_core.rlib'), '-L',
                    'dependency=' + str(target / 'deps'), '-o', str(binary)], check=True)
    return binary


def audit(runtime, private_root, module):
    component, _ = validate(runtime)
    require(component['mode'] == 'execute' and component['conditional_component_completed'], 'successful component missing')
    run = json.loads(runtime.read_text())
    paths = run['execution_paths']['core_component']
    before, after = (Path(paths[k]).read_bytes() for k in ('before', 'after'))
    require(sha(before) == SAVES[0], 'before owner-trace identity differs')
    trace_bytes = gzip.decompress((private_root / 'evidence/linux-save-normalization/checkpoint-203-owners' / TRACES[0][0]).read_bytes())
    require(sha(trace_bytes) == TRACES[0][1], 'before original serializer trace differs')
    rows = [json.loads(line) for line in trace_bytes.splitlines()]
    rows = [r for r in rows if r.get('save') == 1 and 'start' in r]
    initial = json.loads((ROOT / 'evidence/linux/skip-turn-state-execute-failure-1.json').read_text())['typed_initial_state']
    require((component['selected']['owner'], component['selected']['id']) ==
            (initial['unit']['owner'], initial['unit']['id']), 'different actual selected actor needs new initial contract')
    spans = {k: initial[k]['native_range'] for k in ('formation', 'unit')}
    for key, owner in [('formation', 'GameCore::Formation::Instance'), ('unit', 'GameCore::Unit::Instance')]:
        start, end = spans[key]
        require(any(r['owner'] == owner and [r['start'], r['end']] == [start, end] for r in rows), 'initial owner boundary absent')
        require(sha(before[start:end]) == initial[key]['record_sha256'], 'initial typed record identity differs')
    notification_rows = [r for r in rows if r['owner'] == 'GameCore::Reporting::Notification::PlayerNotifications']
    # This exact fixture's selected owner's container is established by its
    # original trace range and its actual decoded owner header, not an ID search.
    notify = next(r for r in notification_rows if r['start'] == 76400879)
    spans['notifications'] = [notify['start'], notify['end']]
    fstart, fend = spans['formation']
    require(formation_end(before, fstart) == fend, 'before Formation extent differs from native trace')
    new_fend = formation_end(after, fstart)
    fdelta = new_fend - fend
    ustart, uend = spans['unit']
    # The already-attributed current Unit has only two changed fields. Its old
    # typed queue is52 bytes; observed empty queue framing is8. Production codec
    # decoding/roundtrip and all73field comparisons below verify this candidate.
    queue = next(f for f in initial['unit_fields'] if f['name'] == 'm_kOperations')
    old_queue_size = queue['end'] - queue['start']
    new_ustart, new_uend = ustart + fdelta, uend + fdelta + 8 - old_queue_size
    udelta = (new_uend - new_ustart) - (uend - ustart)
    nstart, nend = spans['notifications']
    new_nstart = nstart + fdelta + udelta
    # End is independently constrained by the unchanged complete suffix and
    # then checked by exact sequential notification count/instance consumption.
    new_nend = nend + len(after) - len(before)
    paired = [('formation', [fstart, fend], [fstart, new_fend]),
              ('unit', [ustart, uend], [new_ustart, new_uend]),
              ('notifications', [nstart, nend], [new_nstart, new_nend])]
    gaps = []
    old_pos = new_pos = 0
    for name, old_range, new_range in paired + [('end', [len(before), len(before)], [len(after), len(after)])]:
        a, b = before[old_pos:old_range[0]], after[new_pos:new_range[0]]
        require(a == b, 'unclassified bytes outside typed changed owners before ' + name)
        gaps.append(dict(before=[old_pos, old_range[0]], after=[new_pos, new_range[0]], bytes=len(a), sha256=sha(a)))
        old_pos, new_pos = old_range[1], new_range[1]
    contract = original_contract(module)
    subprocess.run(['cargo', 'build', '-p', 'c6-core', '--lib'], cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1', '--no-deps'], cwd=ROOT))
    target = Path(metadata['target_directory']) / 'debug'
    decoded, notifications = {}, {}
    with tempfile.TemporaryDirectory(prefix='c6-skip-success-') as temporary:
        directory = Path(temporary)
        decoder = compile_tool(DECODER, 'unit_formation', directory, target)
        notification_decoder = compile_tool(NOTIFICATIONS, 'notifications', directory, target)
        for side, data, range_index in [('before', before, 1), ('after', after, 2)]:
            records = []
            for name, old_range, new_range in paired:
                lo, hi = [None, old_range, new_range][range_index]
                path = directory / f'{side}-{name}.bin'
                path.write_bytes(data[lo:hi])
                if name != 'notifications':
                    records.append(f'{name} {path} 0')
                else:
                    notifications[side] = [json.loads(line) for line in subprocess.check_output([str(notification_decoder), str(path)], text=True).splitlines()]
            manifest = directory / (side + '-records')
            manifest.write_text('\n'.join(records))
            decoded[side] = [json.loads(line) for line in subprocess.check_output([str(decoder), str(manifest), str(component['selected']['owner']), str(component['selected']['id'])], text=True).splitlines()]
    units = {side: next(r for r in records if r['kind'] == 'unit') for side, records in decoded.items()}
    formations = {side: next(r for r in records if r['kind'] == 'formation') for side, records in decoded.items()}
    fields = {side: [r for r in records if r['kind'] == 'field'] for side, records in decoded.items()}
    require(len(fields['before']) == len(fields['after']) == 73, 'incomplete Unit field inventory')
    changed_fields = []
    for a, b in zip(fields['before'], fields['after']):
        require(a['name'] == b['name'], 'Unit field order differs')
        ar = [ustart + a[k] for k in ('start', 'end')]
        br = [new_ustart + b[k] for k in ('start', 'end')]
        av, bv = before[ar[0]:ar[1]], after[br[0]:br[1]]
        if av != bv:
            changed_fields.append(dict(name=a['name'], before_range=ar, after_range=br,
                                       before_bytes=len(av), after_bytes=len(bv), before_sha256=sha(av), after_sha256=sha(bv)))
    require([r['name'] for r in changed_fields] == ['m_eActivity', 'm_kOperations'], 'unexpected additional Unit field effects')
    require(units['after']['activity'] == 0x047c048b and units['after']['queue_size'] == 0, 'typed SkipTurn result differs')
    require(formations['before']['path_nodes'] == 5 and formations['after']['path_nodes'] == 0
            and formations['after']['cache_location'] == [-9999, -9999], 'typed Formation clear differs')
    # Other Formation fields must be identical: fixed prefix36bytes and final
    # membership vector. The count/path/cache middle is decoded above.
    require(before[fstart:fstart+36] == after[fstart:fstart+36]
            and before[fend-8:fend] == after[new_fend-8:new_fend], 'other Formation fields changed')
    notification_details = {}
    for side, data, origin in [('before', before, nstart), ('after', after, new_nstart)]:
        header, *items = notifications[side]
        require(header['owner'] == component['selected']['owner'], 'notification owner differs')
        for item in items:
            lo, hi = item['start'], item['end']
            item['native_range'] = [origin+lo, origin+hi]
            item['bytes'] = hi-lo
            item['record_sha256'] = sha(data[origin+lo:origin+hi])
            for field in ('summary_range', 'message_range', 'parameters_range'):
                a, b = item[field]
                item[field.removesuffix('_range') + '_sha256'] = sha(data[origin+a:origin+b])
        notification_details[side] = dict(header=header, records=items)
    old_items, new_items = (notification_details[s]['records'] for s in ('before', 'after'))
    old_by_id, new_by_id = ({r['id']: r for r in items} for items in (old_items, new_items))
    removed = [r for r in old_items if r['id'] not in new_by_id]
    require([r['id'] for r in removed] == [23, 42, 44]
            and all(r['is_dismissed'] and r['type_hash'] == 0xe90d955a for r in removed), 'dismissed cleanup footprint differs')
    require(sum(r['bytes'] for r in removed) == 1064, 'notification size decrease unaccounted')
    require([r['id'] for r in new_items] == [r['id'] for r in old_items if r['id'] in new_by_id], 'surviving notification order differs')
    changed_notifications = []
    for item in new_items:
        old = old_by_id[item['id']]
        ar, br = old['native_range'], item['native_range']
        a, b = before[ar[0]:ar[1]], after[br[0]:br[1]]
        differences = [i for i, (x, y) in enumerate(zip(a, b)) if x != y]
        if differences:
            require(len(a) == len(b) and differences == [len(a)-1] and a[-1] == 0 and b[-1] == 1
                    and item['id'] == 47 and item['has_been_activated'], 'additional notification changes need classification')
            changed_notifications.append(dict(id=47, field='HasBeenActivated', native_offset='0x19e', before=False, after=True,
                                              before_offset=ar[1]-1, after_offset=br[1]-1))
    require(len(changed_notifications) == 1, 'notification activation effect missing')
    require(fdelta == -100 and udelta == -44 and len(after)-len(before) == -1208, 'unexpected serialized size accounting')
    all_ranges = [dict(owner=name, before_range=a, after_range=b, before_bytes=a[1]-a[0], after_bytes=b[1]-b[0],
                       before_sha256=sha(before[a[0]:a[1]]), after_sha256=sha(after[b[0]:b[1]])) for name,a,b in paired]
    # Large named regions are covered by exact gap equality. This is byte
    # equality at shifted offsets, not a live semantics/behavior claim.
    unchanged_regions = []
    for name, lo, hi in [('TradeGraph', 71027800, 71497438), ('RequirementSystem', 73741108, 74930233)]:
        require(before[lo:hi] == after[lo-144:hi-144], 'unexpected ' + name + ' byte change')
        unchanged_regions.append(dict(owner=name, before_range=[lo,hi], after_range=[lo-144,hi-144], sha256=sha(before[lo:hi])))
    return dict(schema='c6-linux-skip-turn-quiescent-state-v1', artifact_checks_passed=True,
                configuration_sha256=component['configuration_sha256'], runtime_report_sha256=digest(runtime),
                original_core_saves=component['original_core_saves'], selected=component['selected'],
                native_quiescence_proven=True, native_cleanup_proven=True,
                conditional_component_completed=True, host_admission_proven=False, full_action_acceptance=0, gameplay_parity=False,
                raw_core_difference=component['raw_core_difference'], masked_bytes=0, unknown_bytes_classified=False,
                complete_serialized_byte_footprint_accounted=True, unaccounted_serialized_changed_regions=0,
                changed_owner_regions=all_ranges, unchanged_gaps=gaps,
                before_original_serializer_trace_sha256=TRACES[0][1],
                after_alignment='Production Unit/Formation codec framing and roundtrips; original Notification writer sequential count/record framing; exact equality of entire intervening regions and suffix. No identity byte search or stale-offset reuse.',
                unit_before=units['before'], unit_after=units['after'], unit_changed_fields=changed_fields,
                unit_unchanged_fields=71,
                old_queue=[r for r in decoded['before'] if r['kind'] == 'queue'],
                formation_before=formations['before'], formation_after=formations['after'],
                formation_path_before=[r for r in decoded['before'] if r['kind'] == 'formation_path_node'],
                formation_only_changed_fields=['m_kLastPath', 'm_kLastPathCacheLocation'],
                notification_container=notification_details, removed_notifications=[r['id'] for r in removed],
                changed_notification_fields=changed_notifications, notification_contract=contract,
                unchanged_named_serialized_regions=unchanged_regions,
                bounded_live_observations=component['bounded_live_observations'], reporting_events=component['reporting_events'],
                complete_reporting_inventory=False, complete_live_affected_state=False, complete_rng_inventory=False,
                historical_identity_report_wrappers_return_abi_correct=False,
                observer_purity_proven=False,
                no_direct_ai_policy_guard_observed=True, universal_external_policy_boundary_proven=False,
                historical_raw_save_load_failure_bytes=89198,
                codec_source_sha256={name:digest(ROOT / name) for name in
                                     ('crates/c6-core/src/unit.rs', 'crates/c6-core/src/unit/state.rs',
                                      'crates/c6-core/src/unit/variants.rs')},
                audit_source_sha256=digest(Path(__file__)), rust_decoder_sha256=sha(DECODER.encode()), notification_decoder_sha256=sha(NOTIFICATIONS.encode()),
                limits='Complete byte footprint of this conditional fixture is classified by original writer ownership and typed records. Complete live affected state, Formation dirty history, notification activation event/subscribers, all gameplay RNG and universal external policy boundaries remain incomplete. No full-host acceptance, Rust SkipTurn transition parity, complete gameplay state/action/turn equivalence or presentation-only exclusion.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-report', type=Path, required=True)
    parser.add_argument('--private-root', type=Path, default=ROOT.parent / 'c6-ghidra')
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-rules/asl-launch-xp2-2/libGameCore_XP2.so')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repeat-runtime-report', type=Path)
    args = parser.parse_args()
    result = audit(args.runtime_report, args.private_root, args.module)
    if args.repeat_runtime_report:
        repeated, _ = validate(args.repeat_runtime_report)
        require(repeated['runtime_report_sha256'] != result['runtime_report_sha256'], 'repeat reused same runtime')
        require(repeated['mode'] == 'execute' and repeated['configuration_sha256'] == result['configuration_sha256']
                and repeated['original_core_saves'] == result['original_core_saves'] and repeated['selected'] == result['selected'],
                'repeat serialized footprint or identified configuration/actor differs')
        require(all(typed_observation(a) == typed_observation(b) for a, b in
                    zip(result['bounded_live_observations'], repeated['bounded_live_observations'])),
                'independent repeat typed observations differ')
        result['isolated_repeat_runtime_sha256'] = repeated['runtime_report_sha256']
        result['isolated_repeat_identical_serialized_footprint'] = True
        result['isolated_repeat_raw_live_differences'] = {
            name: raw_live_differences(result['bounded_live_observations'][index], repeated['bounded_live_observations'][index])
            for name, index in [('before', 0), ('after', 2)]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ('artifact_checks_passed', 'complete_serialized_byte_footprint_accounted', 'unaccounted_serialized_changed_regions', 'full_action_acceptance')}))


if __name__ == '__main__':
    main()
