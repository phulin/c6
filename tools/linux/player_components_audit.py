#!/usr/bin/env python3
"""Static target contracts + offline production wire checks; never run the original game."""
import argparse
import gzip
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from core_stream_index import index_core, read_stream, Reader, sha
from save_lifecycle_audit import Elf, GAME_SHA
ROOT = Path(__file__).resolve().parents[2]
METHODS = [(33801856,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resolution> '
  '>(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resolution> '
  'const&)',
  'dce42c0b096f0e7f4e76d0944f197dc280a66f1798268b8f9540569b672254a7'),
 (34512576,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> const&)',
  'dfde79b7a8af6bbc7495834b67e5b19a6914f364cc44ca4d91ca9423fb73da0b'),
 (44395568,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentBonusName> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentBonusName> '
  'const&)',
  '3a112e0f4bfa51c84c32d305cfcf66fadc37a44833a5131cce0ddb20805927b2'),
 (45121024,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GoodyHut> '
  '>(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GoodyHut> '
  'const&)',
  '8efc287e27df8440632273884ff854a8f867619ddd6782466813f74658728fe1'),
 (44464288,
  149,
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> const&)',
  'dc6ede9840be91d7326506fc0d881ce157b12f00c3e6fce3afa0c932ec7f1a9f'),
 (44395072,
  149,
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentBonusName> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentBonusName> '
  'const&)',
  '7de7ad65ff3106c9998e5aca7eb477d51f1a3ae731f92e7d7aa387fb8038386d'),
 (45119520,
  149,
  'bool Serialization::ReadIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GoodyHut> '
  '>(Serialization::IStream&, int*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GoodyHut> const&)',
  '312a0b198096a8dfeab87127493f4794fd974dbc6c425a767ccdd01ce1e65d99'),
 (44462624,
  141,
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus> const&)',
  '146b3a329896713e56718144571e45269ffd143115e9ad5241590de936652c61'),
 (44394832,
  141,
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentBonusName> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentBonusName> '
  'const&)',
  '13e1adabab122e362d5da335c5c26c254dbf03ab624882498d87d4a2b5559d91'),
 (45119376,
  141,
  'bool Serialization::WriteIndexHashArray<int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GoodyHut> '
  '>(Serialization::OStream&, int const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GoodyHut> const&)',
  '26cfa5bc03670f5a0a38ca37fdc7372447e52e6ebc2f7b9d3f4eaf9fb5867b30'),
 (44471696,
  125,
  'void Serialization::SerializeToSequenceContainer<GameCore::Player::ResolutionEffectRefund, '
  'eastl::vector<GameCore::Player::ResolutionEffectRefund, eastl::allocator> >(Serialization::IStream&, '
  'eastl::vector<GameCore::Player::ResolutionEffectRefund, eastl::allocator>&)',
  '9360cd286e243dbf22b86bfa0a5424edd0368602886bd0d13b439c9fcaa6c4c6'),
 (46101920,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::ResolutionTypes const&)',
  '9009d884a1086c2a63080bf6c4b68fede66e04ee74144220d040b55f61de9d98'),
 (44454880,
  71,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::ResolutionEffectRefund const&)',
  '3aefa90b3d7423bc09225e92910c7ffecbd3237441f3f2ba28c04c5200437997'),
 (44393184,
  67,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Bonuses const&)',
  '95f939679204999668433bc91b657ea57ed3653e5c5b55308c4c0204d2f35f1b'),
 (44455040,
  381,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Congress const&)',
  'b2e7763824d6e85473c63a0af93bbaa982d0afa5647fa045adf41938db2c6650'),
 (45113648,
  67,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Goody_Hut const&)',
  'e061e322d00f9ed207b9905ddf43dae3ef8317b2e9c867ea825782c221875cb1'),
 (44472880,
  91,
  'Serialization::OStream& Serialization::operator<< '
  '<GameCore::Player::ResolutionEffectRefund>(Serialization::OStream&, '
  'eastl::vector<GameCore::Player::ResolutionEffectRefund, eastl::allocator> const&)',
  '0c995d69ff73127c577cc3fcea84732ab4421ecec0672fea401abcbc91fe93cc'),
 (46101968,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::ResolutionTypes&)',
  '78d21c377bbb161482d1febe92920d30a7763ce5e335e63dfdaa3e0cb4320288'),
 (44454960,
  69,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::ResolutionEffectRefund&)',
  '3168ac2b0826f07e7a65d963ffcc54e326d1f3912dfa3e15da4e00deae947fe4'),
 (44393264,
  295,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Bonuses&)',
  '481d6e6e4a173a1df3a324f97d842d092b736ab5f0d6f02ad4f25408e7dbe792'),
 (44455424,
  908,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Congress&)',
  'db763f9e00fe3d3789d452633ab1d52a7124b8a6e239c8e092d56c6aa6149e61'),
 (45113728,
  73,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Goody_Hut&)',
  'b5370f82e290af8c3714f4deb47fdfc662e3ad449bd0b1ca569232a5bc455303'),
 (33921488,
  102,
  'Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >::resize(unsigned long)',
  '4789193996d4c60a067663ca162949befba01cfa621fb5cc7ee5dd3bcf619710'),
 (44394576,
  84,
  'GameCore::Player::Bonuses::Reset()',
  '959675d43e12ae899dee6c1a9fd422084248b43723d30bfa9112ff5e5069591d'),
 (44394016,
  234,
  'GameCore::Player::Bonuses::Bonuses()',
  '63d1042498381220994e6cc5de0320b0f49e646de5c5687d4e70a8a2ea14e6b6'),
 (44459264,
  385,
  'GameCore::Player::Congress::Reset()',
  '62ae1eb27c51f6b050159af098bdc4fcbb81dd766f013fb8dd763c75bdd106be'),
 (44456784,
  1996,
  'GameCore::Player::Congress::Congress()',
  'eb69b960b93d278f68307dcce7dcadd740394d365bdf00a6fd953a0ad59feee9'),
 (45114144,
  91,
  'GameCore::Player::Goody_Hut::Reset()',
  '26fd02eb7b2ca47b09fddd451d47b0a35d67a056406c495d9548ec07626d8a7a'),
 (45113808,
  181,
  'GameCore::Player::Goody_Hut::Goody_Hut(GameCore::Player::Instance*)',
  '5bb4e043ed69961fca13c3c98deca3af1460865591894d8702d4fe0fec69033e'),
 (33801904,
  4,
  'GameCore::Definition::Resolution::GetCollectionIndex() const',
  'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a'),
 (34512624,
  4,
  'GameCore::Definition::MinorCivBonus::GetCollectionIndex() const',
  'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a'),
 (43370944,
  4,
  'GameCore::Definition::GovernmentBonusName::GetCollectionIndex() const',
  'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a'),
 (33786080,
  109,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Resolution>::Find(unsigned '
  'int) const',
  'dd2f55696970696aae8c36ceef9f1bb9601b3c3ba381a890d3d31301902f30c4'),
 (34510288,
  109,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::MinorCivBonus>::Find(unsigned '
  'int) const',
  '504f4c041e7147db646c24c8dd78ef96c9890a2779c27d03c7ad4c74557c0902'),
 (43370832,
  109,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GovernmentBonusName>::Find(unsigned '
  'int) const',
  '7ab974f41526c00a3e88cdc9772f96fb684ec1c3ce9d249d0ba713e2471b6b34'),
 (43966368,
  109,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::GoodyHut>::Find(unsigned '
  'int) const',
  'bd326768db1c20741032ba270aa18b0358db33756015542c93ad0d88501f16fd'),
 (45121072,
  4,
  'GameCore::Definition::GoodyHut::GetCollectionIndex() const',
  'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a')]

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output',type=Path,required=True)
    args = p.parse_args()
    module=args.assets/'libGameCore_XP2.so'
    game=Elf(module,GAME_SHA)
    for a,n,name,digest in METHODS:
        if sha(game.read(a,n))!=digest: raise ValueError('original provider differs: '+name)
    for a,version in [(0x2f611d0,1),(0x2f569a0,4),(0x2f571f0,12)]:
        if struct.unpack('<i',game.read(a,4))[0]!=version: raise ValueError('version differs')
    for a,raw in [(0x2b0635e,'c744240c00000000'),(0x2a56887,'c744240c00000000'),
                  (0x2a5634b,'ff5020'),(0x2059a20,'4889dfe8a8dbebff83c5ff75f3')]:
        if game.read(a,len(bytes.fromhex(raw))).hex()!=raw: raise ValueError('read/reset byte contract differs')
    virtuals=[(0x3ad5a30,0x20,0x2a56850),(0x3ad63d0,0x20,0x2a66500)]
    for table,slot,target in virtuals:
        if game.relocated_pointer(table+16+slot)!=target: raise ValueError('concrete Reset virtual differs')
    map_parser=ROOT/'tools/linux/map_state_audit.py'
    if sha(map_parser.read_bytes())!='f43b2712ef67a4e2fdb1ab0ba578bc4796adc6241d232cb020ed9a35cf13f96b':
        raise ValueError('reviewed structural Map parser differs')
    dwarf=subprocess.check_output(['gdb','-nx','-batch',str(module),
        '-ex',"ptype /o 'GameCore::Player::ResolutionEffectRefund'",'-ex',"ptype /o 'GameCore::Player::Congress'",
        '-ex',"ptype /o 'GameCore::Player::Goody_Hut'",'-ex',"ptype /o 'GameCore::Player::Bonuses'"],text=True)
    for token in ['eResolution','iWhichEffect','iRefundPercent','total size (bytes):   12',
                  'm_applyingGoody','m_aiPlayerGoodies','m_iFlatBonuses','m_iFavorFromGovernmentSlots',
                  'm_iLastTurnConsideredCongress','m_iCongressStageCompleted']:
        if token not in dwarf: raise ValueError('matching DWARF differs: '+token)
    sources=[ROOT/'crates/c6-core/Cargo.toml',ROOT/'Cargo.toml',ROOT/'Cargo.lock',
             ROOT/'crates/c6-core/examples/player_components_codec.rs',*sorted((ROOT/'crates/c6-core/src').rglob('*.rs'))]
    identities={str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in sources}
    subprocess.run(['cargo','build','--offline','-p','c6-core','--example','player_components_codec'],cwd=ROOT,check=True)
    if identities!={str(x.relative_to(ROOT)):sha(x.read_bytes()) for x in sources}: raise ValueError('source changed during build')
    candidate=ROOT/'target/debug/examples/player_components_codec'
    fixtures=['ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884',
              '85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8']
    traces=[('trace-benchmark-1.jsonl.gz','528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
            ('trace-captured-1.jsonl.gz','027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')]
    cases=[]
    with tempfile.TemporaryDirectory(prefix='c6-player-wire-') as directory:
        directory=Path(directory)
        def check(kind,payload,label):
            source=directory/(label+'.bin'); output=directory/(label+'.out')
            source.write_bytes(payload)
            observation=json.loads(subprocess.check_output([str(candidate),kind,str(source),str(output)],text=True))
            if output.read_bytes()!=payload or observation['wire_roundtrip_equal'] is not True: raise ValueError('Rust roundtrip differs')
            return dict(kind=kind,bytes=len(payload),sha256=sha(payload),items=observation['items'])
        for number,(identity,(trace_name,trace_digest)) in enumerate(zip(fixtures,traces)):
            data=read_stream(args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz'))
            if sha(data)!=identity: raise ValueError('fixture identity differs')
            indexed=index_core(data,first_player_components=True)
            rows={x['name']:x for x in indexed['sections'] if x['name'].startswith('first_player_')}
            with gzip.open(args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name,'rb') as f: trace=f.read()
            if sha(trace)!=trace_digest: raise ValueError('original trace differs')
            owners=[json.loads(line) for line in trace.splitlines()]
            owners=[x for x in owners if x.get('event')=='owner' and x.get('save')==1]
            congresses=[x for x in owners if x.get('owner')=='GameCore::Player::Congress']
            cities=[x for x in owners if x.get('owner')=='GameCore::Player::Cities']
            first=rows['first_player_congress']
            if not any((x['start'],x['end'])==(first['start'],first['end']) for x in congresses): raise ValueError('Congress native boundary differs')
            if not any(x['start']==indexed['parsed_end'] for x in cities): raise ValueError('stop is not original Cities start')
            direct=[]
            for kind in ['goody_hut','bonuses','congress']:
                row=rows['first_player_'+kind]
                direct.append(check(kind,data[row['start']:row['end']],str(number)+'-'+kind))
            all_congress=[check('congress',data[x['start']:x['end']],str(number)+'-all-'+str(i)) for i,x in enumerate(congresses)]
            # Counted-pair length changes relocate following records structurally.
            goody=rows['first_player_goody_hut']; altered=bytearray(data[goody['start']:goody['end']])
            struct.pack_into('<I',altered,4,struct.unpack_from('<I',altered,4)[0]+1)
            altered.extend(struct.pack('<Ii',0xdeadbeef,-123))
            shifted=index_core(data[:goody['start']]+altered+data[goody['end']:],first_player_components=True)
            shifted_congress=next(x for x in shifted['sections'] if x['name']=='first_player_congress')
            if shifted['parsed_end']!=indexed['parsed_end']+8 or shifted_congress['sha256']!=first['sha256']: raise ValueError('count shift not followed')
            # Current source-derived nonempty nested Refund, regardless of native fixture counts.
            sample=struct.pack('<i6i',12,*range(6))+struct.pack('<I',0)*5+struct.pack('<4iI',7,8,9,10,1)+struct.pack('<iIii',12,0xfedcba98,-1,150)+struct.pack('<Bi',1,11)
            reader=Reader(sample); parsed=reader.congress()
            if reader.pos!=len(sample) or len(parsed['refunds'])!=1: raise ValueError('nonempty Refund schema differs')
            check('congress',sample,str(number)+'-synthetic-refund')
            cases.append(dict(stream_sha256=identity,trace_sha256=trace_digest,first_player=indexed['first_player'],
                component_counts=indexed['first_player_component_counts'],components=direct,
                congress_native_boundaries_matched=len(all_congress),congress_codec_records=all_congress,
                city_start=indexed['parsed_end'],next_unframed_schema=indexed['next_unframed_schema'],predecessor_length_shift_passed=True))
    report=dict(schema='c6-linux-player-components-v1',checks_passed=True,gamecore_sha256=GAME_SHA,
        methods=[dict(address=hex(a),size=n,name=name,sha256=digest) for a,n,name,digest in METHODS],
        virtual_bindings=[dict(table=hex(t),slot=hex(s),target=hex(a)) for t,s,a in virtuals],
        audit_source_sha256=sha(Path(__file__).read_bytes()),map_parser_sha256=sha(map_parser.read_bytes()),
        matching_dwarf_sha256=sha(dwarf.encode()),candidate_sha256=sha(candidate.read_bytes()),candidate_sources=identities,
        indexer_sha256=sha((ROOT/'tools/linux/core_stream_index.py').read_bytes()),fixtures=cases,
        wire_versions=dict(goody_hut=1,bonuses=4,congress=12,refund=12),raw_ordered_hash_pairs_preserved=True,
        read_semantics=dict(goody_hut='preserve prior values and applyingGoody; no Reset',
            bonuses='concrete Reset zeros to current definition count, then overlay',
            congress='resize preserves known prefix; newly exposed values remain unavailable unless supplied',
            duplicates='last mapped pair wins',unknown_hashes='consume without writing',
            omitted_congress_runtime_fields='actual owner state must retain them; no exclusion'),
        original_game_execution=False,offline_candidate_execution=True,trace_required_by_indexer=False,fixture_offsets_used_by_indexer=False,
        full_player_import=False,full_unit_roster_index=False,complete_city_codec=False,gameplay_parity=False,unknown_bytes_masked=0)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods=len(METHODS),original_streams=len(cases),congress_records=sum(x['congress_native_boundaries_matched'] for x in cases),original_game_execution=False)))
if __name__=='__main__': main()
