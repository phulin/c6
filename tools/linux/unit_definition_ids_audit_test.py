#!/usr/bin/env python3
"""Synthetic metadata rejection tests; never original-code/native evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import zlib
from unit_definition_ids_audit import audit_runtime, ROOT, TARGET, HOST, sha


class AuditGates(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.capture=Path(self.temp.name)/'save'
        self.capture.write_bytes(b'synthetic control only')
        self.contract=json.loads((ROOT/'evidence/linux/unit-definition-ids-static.json').read_text())
        entries=[]
        for i,row in enumerate([0,7,101]):
            name=f'SYNTHETIC_{i}'.encode()
            h=(~zlib.crc32(name))&0xffffffff
            entries.append(dict(index=i,database_rowid=row,hash=h,find_index=i,native_string_hash=h,type_hex=name.hex()))
        self.query=dict(registry='Units',count=3,entries=entries,index_zero=0,at_count_is_null=True,missing_hash=0xffffffff,missing_result=-1,empty_string_hash=0xffffffff,lowercase_warrior_hash=(~zlib.crc32(b'unit_warrior'))&0xffffffff)
        self.config=dict(schema='c6-linux-mod-activation-v1',assets={'Civ6':HOST,'libGameCore_XP2.so':TARGET},lower_core_unit_definition_registry_observer=self.contract,cleanup_requested=True,sources={p:sha((ROOT/p).read_bytes()) for p in ['native/linux/unit_definition_ids.c','native/linux/unit_definition_ids.h']})
        self.runtime=dict(configuration=self.config,returncode=0,framework_cleanup_proven=True,client_cleanup_proven=True,game_saves={k:dict(sha256=sha(self.capture.read_bytes()),bytes=self.capture.stat().st_size) for k in ['save','repeat']})

    def check(self):
        self.runtime['configuration_sha256']=sha(json.dumps(self.config,sort_keys=True,separators=(',',':')).encode())
        self.runtime['stderr']='C6_LINUX_GAME_LOAD module /synthetic/libGameCore_XP2.so context_initialized 1 control\nC6_LINUX_UNIT_DEFINITION_IDS '+json.dumps(self.query)
        return audit_runtime(self.runtime,self.contract,self.capture,self.capture)

    def test_zero_and_gapped_rowids_preserved(self):
        self.assertEqual([e['database_rowid'] for e in self.check()['mapping']['entries']],[0,7,101])

    def test_cleanup_and_alternate_schema_rejected(self):
        for key in ['framework_cleanup_proven','client_cleanup_proven']:
            for value in [False,1,'true',None]:
                with self.subTest(key=key,value=value):
                    self.runtime[key]=value
                    with self.assertRaises(ValueError):self.check()
            self.runtime[key]=True
        self.runtime.pop('returncode')
        self.runtime['exit_code']=0
        with self.assertRaises(ValueError):self.check()

    def test_named_assets_gate_source_and_abi(self):
        initial=copy.deepcopy(self.config)
        changes=[lambda c:c['assets'].update(Civ6='0'*64),lambda c:c['assets'].pop('libGameCore_XP2.so'),lambda c:c.update(schema='alternate'),lambda c:c.update(cleanup_requested=1),lambda c:c['sources'].clear(),lambda c:c.pop('lower_core_unit_definition_registry_observer')]
        for change in changes:
            self.config=copy.deepcopy(initial)
            self.runtime['configuration']=self.config
            change(self.config)
            with self.assertRaises(ValueError):self.check()

    def test_bad_mapping_and_boundaries_rejected(self):
        initial=copy.deepcopy(self.query)
        for field,value in [('index',True),('index',7),('hash',0),('database_rowid',False),('find_index',-1)]:
            self.query=copy.deepcopy(initial)
            self.query['entries'][0][field]=value
            with self.assertRaises(ValueError):self.check()
        self.query=copy.deepcopy(initial)
        self.query['at_count_is_null']=1
        with self.assertRaises(ValueError):self.check()

    def test_changed_capture_rejected(self):
        self.capture.write_bytes(b'changed')
        with self.assertRaises(ValueError):self.check()


if __name__=='__main__':unittest.main()
