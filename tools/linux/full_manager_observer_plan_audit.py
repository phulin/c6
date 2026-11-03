#!/usr/bin/env python3
"""Test offline preload extraction/valueflow without loading original code."""
import argparse
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import subprocess

import full_manager_observer_plan as producer
from full_manager_observer_runner import ROOT

DRAFT_SHA='7c49002e25cda6c9efc3bad78d7c7957f6d2158c8780cecbbd7a1714496ecf70'


class FlowTests(unittest.TestCase):
    def selected(self,extra=(),between=()):
        elf=producer.OwnElf.__new__(producer.OwnElf)
        lines=['lea 0x0(%rip),%rax # 100 <selector>','call 200 <dlsym@plt>',
               'mov %rax,-0x8(%rbp)']+list(extra)+['mov -0x8(%rbp),%rax']+list(between)+['call *%rax']
        elf.code={'f':[dict(address=index,size=1,raw='90',return_pc=index+1,asm=line) for index,line in enumerate(lines)]}
        elf.cstring=lambda address:'target'
        return elf.selected_pointer_call('f','target')
    def test_plain_slot_value_flow(self):
        rows,proof=self.selected(extra=['cmpq $0x0,-0x8(%rbp)'])
        self.assertEqual(len(rows),1);self.assertEqual(proof['result_slot'],'-0x8')
    def test_pointer_slot_overwrite_refused(self):
        with self.assertRaisesRegex(ValueError,'overwritten'):
            self.selected(extra=['movq $0x0,-0x8(%rbp)'])
    def test_partial_register_clobber_refused(self):
        with self.assertRaisesRegex(ValueError,'never reaches'):
            self.selected(between=['xor %eax,%eax'])
    def test_call_clobber_refused(self):
        with self.assertRaisesRegex(ValueError,'never reaches'):
            self.selected(between=['call 300 <other>'])
    def test_slot_escape_refused(self):
        with self.assertRaisesRegex(ValueError,'escaped'):
            self.selected(extra=['lea -0x8(%rbp),%rdi'])
    def test_manager_partial_register_alias(self):
        self.assertTrue(producer.writes(dict(asm='xor %r10d,%r10d'),'r10'))
    def test_instruction_exact_return(self):
        row=producer.instructions('  1fb81:\t41 ff d2\tcall *%r10')[0]
        self.assertEqual(row['return_pc'],0x1fb84)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--preload',type=Path,default=ROOT/'target/linux/full-manager-draft/preload-draft.so')
    parser.add_argument('--expected-sha256',default=DRAFT_SHA)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    result=unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(FlowTests))
    if not result.wasSuccessful():raise SystemExit(1)
    fragment,elf=producer.extract(args.preload,args.expected_sha256)
    assert len(fragment['compiled_symbols'])==12 and len(fragment['caller_suffix_frames'])==8
    assert set(fragment['marker_parent_frames'])==set(map(str,range(1,10)))
    for chains in fragment['marker_parent_frames'].values():
        for chain in chains:assert chain[-1]==fragment['caller_suffix_frames'][-1]
    assert fragment['window_parent_prefixes']=={} and not fragment['complete_runtime_plan']
    try:
        producer.OwnElf(args.preload,'0'*64)
    except ValueError:pass
    else:raise AssertionError('wrong ELF identity accepted')
    # Mutate only our copied preload, then deliberately update its outer hash
    # to reach the separate Manager value-flow rejection boundary.
    manager=fragment['indirect_call_proofs']['Manager']['call']
    field=manager['field_load'];code=elf.code['invoke'];index=next(i for i,row in enumerate(code) if row['address']==field['address'])
    after=code[index+1];assert after['size']==4 and after['asm']=='mov -0x80(%rbp),%rax'
    load=next(row for row in elf.loads if row['address']<=after['address']<row['address']+row['file_size'])
    offset=load['offset']+after['address']-load['address']
    mutated=bytearray(elf.raw);mutated[offset:offset+4]=bytes.fromhex('4531d290') # xor r10d,r10d; nop
    with tempfile.TemporaryDirectory(prefix='c6-plan-mutation-') as temporary:
        path=Path(temporary)/'mutated-own-preload.so';path.write_bytes(mutated)
        try:producer.extract(path,hashlib.sha256(mutated).hexdigest())
        except ValueError as error:assert 'Bindings.can_start' in str(error)
        else:raise AssertionError('clobbered Manager target accepted')
        # Source pathname replacement after verification cannot alter the
        # immutable snapshot supplied to objdump or retained in OwnElf.
        path.write_bytes(elf.raw);actual=producer.subprocess.check_output
        def replace_then_disassemble(command,**kwargs):
            path.write_bytes(b'replaced after verification')
            return actual(command,**kwargs)
        with mock.patch.object(producer.subprocess,'check_output',side_effect=replace_then_disassemble):
            retained=producer.OwnElf(path,args.expected_sha256)
        assert producer.sha(retained.raw)==args.expected_sha256
        assert retained.table['invoke']==elf.table['invoke']
    report=dict(schema='c6-linux-full-manager-observer-plan-source-v1',original_execution=False,
                preload_sha256=args.expected_sha256,extractor_sha256=producer.sha(Path(producer.__file__).read_bytes()),
                audit_sha256=producer.sha(Path(__file__).read_bytes()),
                checks=dict(valueflow_cases=result.testsRun,actual_compiled_symbol_bodies=12,
                    actual_own_caller_frames=8,actual_marker_phase_prefixes=9,
                    wrong_identity_refused=True,mutated_Manager_register_clobber_refused=True,
                    disassembly_uses_immutable_verified_snapshot=True),
                symbols=fragment['compiled_symbols'],marker=fragment['marker_contract'],
                main_entry_witness=fragment['main_entry_witness'],
                caller_suffix_frames=fragment['caller_suffix_frames'],
                complete_runtime_plan=False,native_profile_admitted=False,
                remaining=fragment['remaining'],full_manager_parity=False,complete_state_actions_rng_turns=False)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')


if __name__=='__main__':main()
