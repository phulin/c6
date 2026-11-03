#!/usr/bin/env python3
"""Check profile-specific plot-yield query construction/effect contracts; never run native code."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sqlite3
import struct
import zlib

from map_state_audit import Reader
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
MAP_SHA = 'cd2ec75b4e342a6ebccc1b96889335352657b9ea67f83d5f6d56f6a94eaf2de9'
PARSER_SHA = 'f43b2712ef67a4e2fdb1ab0ba578bc4796adc6241d232cb020ed9a35cf13f96b'
RULES_REPORT_SHA = 'f23bfb47cf729bc44f44e1683c88f17ac75f191433dbeef1e5bd1b4e747d99ec'
YIELD_EXPORT_SHA = 'fa8c6d0919280b6cc6b9c557aab81b7b8879fb21c3ef9c037a3ac16bdc53146f'
DATABASE_SHA = 'c880adaedcfd6da7808302b3b76bb9cad7d3188b35ae78e780cff7b56e04477e'
PARENT_SHA = '504571fe4cb8f194189bccc86940b820720d2f375de74cffedb9ccdb525c2029'
METHODS = [
    ('game', '0x2bc8820', 1077, 'Plot.Read', 'd8c6bd76add824f31a23a544617d6b211f486a0854537d69ff4ff08eb78f0276'),
    ('game', '0x2bca850', 123, 'Plot.EditProperties.load_construction', 'dedae75283379affd021f3acd5366ea38a63ad12e59932884764b2036a6276be'),
    ('game', '0x2bca8d0', 199, 'Plot.EditPropertiesKind.construction', '6eb511ba7f7e290e9c082a28d49692cdc9ae321f0043d55ebc07156411bad9cd'),
    ('game', '0x2bca7f0', 19, 'Plot.GetProperties.const', 'c15288f31fc8f4fec71681360c57dfd1e606001b8613d3625d9f08cc1f4929e6'),
    ('game', '0x2bca810', 52, 'Plot.GetPropertiesKind.const', 'd449003c2237ab81da7dc9508ed69c88ea1b18815acb8dbdc0d1c4c02bcc1947'),
    ('game', '0x2a3e7d0', 17, 'Improvement.GetProperties', '28f33affda3b468ee1e695f5ffa7c255f77cc47b5147c2036bd536fad5207c19'),
    ('game', '0x28e3e40', 27, 'TypedVariant.to_map.const', 'cc7583b6bf5d0bb161ba009dca97725dd0ad5a282f06717f1628f0eb5b5691a9'),
    ('game', '0x2bf5ad0', 3498, 'Rules.GetYield', '5249f66e85d4f15aaac4e11747aa594dce491cdf8d30e9dc8301aac63606c293'),
    ('game', '0x2a3ed90', 94, 'Improvement.GetYieldFrom', '645eedbaa72e5279a96ffba3e0ced962cc4ce943dd35c5b769f8e3e1a67f19ee'),
    ('game', '0x2a3ee50', 757, 'Improvement.GetPlayerYieldFrom', 'f75e801dbf601be97571a61978777c845cc21da43ae840992265ca1c3443e6b1'),
    ('game', '0x2a3f150', 2147, 'Improvement.GetPlayerYieldFrom.adjacency', '2bc83c3fa99ebe760d7076f6fa00ed2fc13baa475b5d7c8f6e7862b675315af1'),
    ('game', '0x2a3fd30', 267, 'Improvement.BonusTech', '4e88ea903a10e995089047c15fe4c64838ad782be69733321544cfcd54a18f98'),
    ('game', '0x2a3fe40', 267, 'Improvement.BonusCivic', '7fcdc9e73aac2cd148e91df4a58569b4cdca56063b27b3fa6a958617de44b9c1'),
    ('game', '0x2a3ff50', 572, 'Improvement.IsBonusValid', 'c2d8b04d7cc2473744d97e682c308ed0b41ad8e8195301a6ab541767584896b8'),
    ('game', '0x2bc96c0', 37, 'Plot.GetAppeal', '20737c86df0be1bd3e5d522749073fefd0fe53a79a244ec0ae79e16b89afed0f'),
    ('game', '0x21ff420', 1837, 'Improvement.BuildDefinition', '68872fab10190f5d8bd01bda9724e75b7039ab38365e75aff395bd90ab996843'),
    ('game', '0x21ffb60', 8, 'Improvement.BuilderSQL', '651145ae4543a0ba78cbd73ee3d03ca3745bd46f632bc83701dbd87ba721c354'),
    ('game', '0x21ff3a0', 120, 'Improvement.SetYieldFromAppeal.load_only', '1b7091134025f092bb749be8b194a7e8651cf1c77a3c4f022f87d7519c8c03f7'),
    ('game', '0x21f1070', 116, 'Definition.NameToYieldType', '4d81d635b36ffc7819c8a81a7d79aa7c8b90dea8f1a91572fbaa28210f214fb7'),
    ('game', '0x21e7f60', 8, 'AdjacentAppeal.BuilderSQL', '5b47965ca29e40ca98ac28da60e97cb4344e21d4215142a5d6e1e6444740fb0b'),
    ('game', '0x224a1c0', 285, 'AdjacentAppeal.BuildCollection', '800eb69e65b6a3f76b4e572d47c427f5ce95e3b248daedcb336608632a590ba8'),
    ('game', '0x2bc1990', 56, 'Visibility.GetRevealedResourceType', 'f8678e7ebf3d24bf4119383c4f77980da0f1c4b880167cab5f026a17a86dd28f'),
    ('game', '0x2bc18c0', 66, 'Visibility.GetVisibility', '26ab66b819e2442996e7a8229cfe986ce58b596267cbbbd1377b7432df46705f'),
    ('game', '0x27f50c0', 66, 'Visibility.GetLayerIndex', 'a3e36db2fa19c899755360cda6b4f4fb252f102e16fa7f13429eeb02055f74f9'),
    ('game', '0x2005450', 19, 'Visibility.Layer.GetCount', '4e5878e49e25c82d97e5e5806a7aca92cc237095459b2381ec39a142c876a1b2'),
    ('game', '0x2b8ac60', 79, 'Player.Techs.HasTech', 'cf4a3cabd3f3b09872b8bd16197980410160d0ae810aae1d63a423ea68c32ec3'),
    ('game', '0x2a73e70', 86, 'Player.Culture.HasCivic', 'd61b6ae94c24981349cecd699fed49fa57c413f81d7d15787f33a3996dfe4f68'),
    ('game', '0x2a2d160', 24, 'Game.Techs.HasTech', '15b73ac27200be281d4dc5657f341244ff1f86e80e95a10d7d3c0dfeda4125a9'),
    ('game', '0x2978310', 24, 'Game.Culture.HasCivic', '6b922359c9946249e8f6cb842ea7af3d46d23608c32d0a8855aa1178528e759f'),
    ('game', '0x1fe6b50', 14, 'BoolVector.index.const', 'f13ff4ca446e97d1c02ca5ef61a9e5e1703d4640688045c5ca379017e9371e79'),
    ('game', '0x2007f30', 4, 'BoolVector.size.const', '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
    ('game', '0x2bc81b0', 74, 'CityAttributes.GetDistrictType', 'f147b13b9c923cb7774f9f0ebe3858ef019da7c5ce7b4609fbaf6392251ea435'),
    ('game', '0x2bc8200', 71, 'CityAttributes.IsDistrictComplete', 'f39bdadcfd60d365e1f4d7f4329fb250774fd1be088cdb4786c0becc43da2a3a'),
    ('game', '0x2bc8250', 71, 'CityAttributes.IsDistrictPillaged', '0bc7f06c70d75bce80133b50fb15fe5327c35e216b0c7fed4eb10223c36741cc'),
    ('game', '0x2bc8100', 101, 'CityAttributes.IsWonderComplete', '37423cc523509373a9a51a98ee65593b80645e94693ebfdba93a3df852036ea0'),
    ('game', '0x2bc8320', 8, 'CityAttributes.GetDistrict.native_binding', 'e260cc42d331ac28d051f92f0f26d3c0a79ae6d9eacecbdb85efe784ca1cfdd0'),
    ('game', '0x286e0b0', 93, 'CityManager.GetDistrict', 'c1691db37b45fa3441151d0fc7494058a260d898fcba8ac052721df8728f27f2'),
    ('game', '0x2ae12e0', 12, 'Player.Districts.Find', 'a460913eb67325a8287de3200d8f7ffbafa73204508ea8b9e7ee045a2d34d50f'),
    ('game', '0x2ae4290', 84, 'District.IDContainer.Find', '851cd5df1a54fadad43fca32535d1e082d5159169247e367345f165b5a4c4419'),
    ('game', '0x1fe24b0', 7, 'District.IsComplete', '04de6938c59bb5009eaa15a67bf3dad4db82acf731c120a7b6fb5ba3ce334419'),
    ('game', '0x291eb20', 7, 'District.IsPillaged', 'd2e1e923f91a89ab76f04f17b4cc251b878d300e218ad876b1b9770fc9f26a17'),
    ('game', '0x1fe57d0', 7, 'District.GetType', 'ebf9ec5205f71d01bc09c02b78f779e470aced6d0adc6824d92a4d9fe716f035'),
    ('game', '0x1fe2450', 16, 'Map.GetAdjacentPlot.coord', 'd49aaa6a8e22ce6c82d1f979740d9ee339fe37d6bceaa260971abb2e1a9a1ebe'),
    ('game', '0x1fe6bb0', 117, 'Map.GetAdjacentPlot.xy', '20d1b6cd6f0a199ffc43585e40fed52480a603425383291e73ba00e68f8f8e6e'),
    ('game', '0x2a48a30', 168, 'Map.GetPlot.xy', '55e2269824af4db4e69fd3c1525d033284cbe8890485439af00e1b97db203839'),
    ('game', '0x1fe6c30', 27, 'Hexspace.PlotXToX', 'ef7fa80e635393c23fb28022e3b5776e0726ac8ba056e1e34756e729abd8a692'),
    ('game', '0x1fe6c70', 27, 'Hexspace.XToPlotX', 'ecd5d792efcce060e5d9a9c8cf81309197f8be32e680337c624bde85ae851aba'),
    ('game', '0x1fe6c50', 14, 'Direction.X', '6970d362e7b883fd0bc6155fe3e8327590c9cd99d9e1870580cd8d72e61b0986'),
    ('game', '0x1fe6c60', 14, 'Direction.Y', 'da454a67c04c94bd3a82f8a320cd534a2f8e8ac60ead40e66a3025f6ea10298e'),
    ('game', '0x27bbbf0', 32, 'CityAttributes.constructor', '678fcdd56f78ea2f47ce6b9d44f35e598b66e1072bd4b45a808bec4629268595'),
    ('game', '0x27bbc20', 39, 'CityAttributes.initialize', '8c63b3a6c242a1a59e3eecf7042bff0a94e03e0ba5d9c508ffe948370bc8799d'),
    ('game', '0x2a42500', 50, 'CityAttributes.lookup', 'd9b066e28f35789167afe32cdf72cfc7287e0678e5b4f1f5324427a3260b862d'),
    ('game', '0x1fe2440', 6, 'Plot.IsImprovementPillaged', 'af700d60344727ffe670fa9f8aa5e8b02a6fdbe9243c45d2428feced003839cc'),
    ('host', '0x395f070', 124, 'TypedVariantMap.constructor', '88ef146bf7cc119692ceb6363ed7eb31b05e1b25734903162c34b355bfca021e'),
    ('host', '0x395f0f0', 122, 'TypedVariantMap.manager_constructor', 'e930cfe363710bf1526b918ee40fa709bdc4369fc7c406ed62e0bc1fd5c61188'),
    ('host', '0x3961f40', 26, 'TypedVariantMap.stream_operator', 'cfba287e2777e4569362c54de11126a45f3f3b9c495fecb6c5cf6368a02e95db'),
    ('host', '0x3971330', 468, 'TypedKeyedContainer.Unserialize', '926471abf5f8aba34009e30e24c276adebac3f80a62beca98aaa8ad8bb9feaa8'),
    ('host', '0x395eff0', 96, 'TypedVariant.Unserialize', 'f1ec89541fec21e498a19c155c3842c0197633f9d497ebb085193e82183d5ed9'),
    ('host', '0x3968060', 1033, 'ArrayVariant.Unserialize', 'e0176b74687662be2b0c8941e82fdb88d604d35d1301dfcea20f6e21d65fc4b3'),
    ('host', '0x39673c0', 43, 'TypedVariant.to_keyed.const', 'e4491a75c74608682f2ae19fd73ffa487af024a7f8a632bb409197dcaedb3a8d'),
    ('host', '0x395f3b0', 90, 'TypedVariantMap.FindVariant.const', '2788514beee59158d6cd2b816679fb774a19df417cc647a83564e156c77f6b36'),
    ('host', '0x395f480', 95, 'TypedVariantMap.GetVariant.const', 'b264f2c22424685f2c0d51945a672854768f0363768ac961e0bd0de247274118'),
    ('game', '0x2a2c3f0', 26, 'Game.Techs.component_lookup', 'be1a5f2cdb26e2434e288a6cb37438b910f8e2fce0984649c912ea5e746c11fd'),
    ('game', '0x29772c0', 26, 'Game.Culture.component_lookup', 'a344f4b32312ca9961f5184e995ba93b5592886334ff8d09df8294516474409a'),
    ('game', '0x264c060', 42, 'Game.Component.lookup', 'e864d685fed8e09fe6089b219d16d885221ad68c732a5c70a0d3a9d2167fac11'),
    ('game', '0x2660650', 8, 'Game.GetInstance', 'f2da8e6d1cfad107964503183788ee3a3940ed4000767c930e4617053d2e59fb'),
    ('game', '0x2b53270', 8, 'Player.Manager.Get', 'f272c8eeb341e5a4f0e8ddc7c942700a242845c100e7d826fc8b012207bdc8e2'),
    ('game', '0x2003940', 39, 'Player.Manager.GetValidPlayer', '58bb7297d06455fd64617c3472f7087054b4aa5fa1b28ab20ce94cb74db41648'),
    ('game', '0x290b930', 22, 'Globals.GetPlayer', '4f70fef1af78b8dea71c792f71f54727722e3e9f4509f9e764e536223739daaf'),
    ('game', '0x286dc00', 57, 'CityManager.GetCity.ID', '5b7fb30d1c1ed64fa169933871cb06809ecf1f1b38a261a78bea94956b8fb281'),
    ('game', '0x286dc40', 60, 'CityManager.GetCity.owner_ID', 'dc94e53ae027e4fbbc2ebceb12ba12939c76adbe5faeaf3b5ce0e6e4362939b6'),
    ('game', '0x2a597e0', 12, 'Player.Cities.Find', 'b22cb52aa75d16eb1e1f3e664d1ff67096a8a3683c2c7f2d54c009fa22a3e7a6'),
    ('game', '0x2a5cc90', 84, 'City.IDContainer.Find', '3e400de7612ac963b66e6e0630fd735acbe9c1f2588f74c1f26df97c50df008d'),
    ('game', '0x2863e70', 22, 'Buildings.HasBuilding', 'a22810f0b2c8b67074499e681e299cb418bdaba0c8bd82823abb9f3ef26b9d77'),
    ('game', '0x2863e90', 52, 'Buildings.GetBuildingLocation', '83299fac67aad89284b4a3629c58868974a0f575e431a388b1ab17a4897529c2'),
    ('game', '0x2bc8310', 8, 'CityAttributes.GetCity.native_binding', '7d7ecf22e246b5e8c6db17f7bef9c58503b4d34ed920241b8aac33f7b84664a3'),
    ('game', '0x243bb00', 28, 'Map.CoordRange', 'dc7e6d453df9764c167593b8c1b68adccf891a0f82332a4c04809768fd3b3136'),
    ('game', '0x2143670', 60, 'Map.IsPlot', 'dd606c96cc4998d41447193d97ac79a88d424a7e5be21e46b94caa548f51ffd9'),
    ('game', '0x21436b0', 24, 'Map.GetPlotIndex', 'f8a2e8693206a4f3f2fc492be80dbe1ddcceaa3ac67e629f37edc25cd7191e7d'),
]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_identity(path, expected):
    if sha(path) != expected:
        raise ValueError('input identity differs: ' + str(path))


def type_hash(name):
    return zlib.crc32(name.encode()) ^ 0xffffffff


def audit(args):
    for path, expected in [(args.native_map, MAP_SHA), (args.database, DATABASE_SHA),
                           (args.parent_runtime, PARENT_SHA),
                           (ROOT / 'tools/linux/map_state_audit.py', PARSER_SHA),
                           (args.rules_report, RULES_REPORT_SHA),
                           (ROOT / 'evidence/linux/definition-ids-native-1.json', YIELD_EXPORT_SHA)]:
        check_identity(path, expected)
    images = dict(game=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA),
                  host=Elf(args.assets/'Civ6',HOST_SHA))
    for image,address,size,name,expected in METHODS:
        if hashlib.sha256(images[image].read(int(address,16),size)).hexdigest() != expected:
            raise ValueError('original method differs: ' + name)
    gates = [
        ('game',0x2bf6118,'4531c0','NULL contribution list from actual GetYield'),
        ('game',0x2a3efaa,'480f44d8','NULL contribution selects NULL tile-output vector'),
        ('game',0x2a3efd9,'48891c24','tile-output pointer forwarded on stack'),
        ('game',0x2a3efdd,'4531c9','NULL hypothetical map changes'),
        ('game',0x2a3f016,'7460','NULL contribution skips list append'),
        ('game',0x2a3f097,'7424','NULL contribution skips live temporary modifiers'),
        ('game',0x2a3f961,'83fbff','only yield -1 enters adjacency normalization'),
        ('game',0x2a3f964,'7532','specific yield skips normalization'),
        ('game',0x2bf61d6,'bef0957b87','query improvement property key'),
        ('game',0x2bf61e4,'743c','missing property skips deeper yield-map conversion/read'),
        ('game',0x2bc8c16,'ff5060','original plot load obtains mutable map'),
        ('game',0x2bc8c1f,'e8bc1530ff','original plot load reads typed map'),
        ('host',0x3968174,'be00000004','typed map payload flag'),
        ('host',0x39681b5,'e8b66effff','nested payload directly constructs concrete typed map'),
        ('host',0x39681c7,'ff90f0010000','nested map uses constructed read virtual'),
        ('game',0x21ffaf4,'eb17','SQL NULL passes null appeal string to setter'),
        ('game',0x21ff3f3,'31c0','null appeal string produces zero hash'),
        ('game',0x21f1070,'b8ffffffff','unknown appeal hash resolves yield -1'),
        ('game',0x2bf614d,'7542','nonmatching appeal yield skips GetAppeal'),
        ('game',0x224a24b,'743f','empty definition query skips all row construction'),
    ]
    for image,address,expected,name in gates:
        if images[image].read(address,len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('original branch differs: ' + name)
    bindings = [('game',0x3ae2390,0x60,0x2bca850,'native plot mutable map construction'),
                ('game',0x3ae2390,0x50,0x2bca7f0,'native plot const map access'),
                ('game',0x3ae2390,0x58,0x2bca810,'native plot keyed const access'),
                ('game',0x3ae2350,0x8,0x2bc8320,'native CityAttributes district lookup'),
                ('game',0x3ae2350,0x0,0x2bc8310,'native CityAttributes city lookup'),
                ('game',0x3ad5350,0x68,0x2a48a30,'native Map xy lookup'),
                ('host',0x53b3680,0x60,0x395f3b0,'constructed typed map const FindVariant'),
                ('host',0x53b3680,0x70,0x395f480,'constructed typed map const GetVariant'),
                ('host',0x53b3680,0x1f0,0x3971330,'constructed typed map read')]
    for image,table,slot,target,name in bindings:
        elf=images[image]
        actual=elf.relocated_pointer(table+16+slot) if image=='game' else int.from_bytes(elf.read(table+16+slot,8),'little')
        if actual != target:
            raise ValueError('original binding differs: '+name)
    # Both actual constructors store this concrete vptr, independent of manager.
    for address in [0x395f081,0x395f103]:
        raw=images['host'].read(address,7)
        if raw[:3] != bytes.fromhex('488d05') or address+7+struct.unpack_from('<i',raw,3)[0] != 0x53b3680:
            raise ValueError('constructed map vtable origin differs')
    reader=Reader(args.native_map.read_bytes())
    native_map=reader.map()
    if reader.pos != reader.end:
        raise ValueError('typed Map not fully consumed')
    plots=native_map['plots']
    db=sqlite3.connect('file:'+str(args.database.resolve())+'?mode=ro',uri=True)
    db.row_factory=sqlite3.Row
    improvements={type_hash(r['ImprovementType']):dict(r) for r in db.execute('SELECT rowid,* FROM Improvements ORDER BY rowid')}
    present={r['improvement_hash'] for r in plots if r['improvement_hash'] != 0xffffffff}
    if not present <= improvements.keys():
        raise ValueError('Map improvement hashes lack exported definitions')
    present_names={improvements[h]['ImprovementType'] for h in present}
    base_yields={(r[0],r[1]) for r in db.execute('SELECT ImprovementType,YieldType FROM Improvement_YieldChanges')}
    adjacencies=[dict(r) for r in db.execute('SELECT a.ImprovementType,b.* FROM Improvement_Adjacencies a JOIN Adjacency_YieldChanges b ON a.YieldChangeId=b.ID ORDER BY a.rowid') if r['ImprovementType'] in present_names and (r['ImprovementType'],r['YieldType']) in base_yields]
    excluded_ids={r[0] for r in db.execute('SELECT YieldChangeId FROM ExcludedAdjacencies')}
    relevant_ids={r['ID'] for r in adjacencies}
    if relevant_ids & excluded_ids:
        raise ValueError('profile enters separately unsupported trait exclusion path')
    unowned=[r for r in plots if r['city'] is None or r['city']['owner']==-1]
    unowned_improved=[r for r in unowned if r['improvement_hash'] != 0xffffffff]
    unowned_appeal=[r for r in unowned_improved if improvements[r['improvement_hash']]['YieldFromAppeal'] is not None]
    adjacent_appeal=db.execute('SELECT count(*) FROM Adjacent_AppealYieldChanges').fetchone()[0]
    if unowned_appeal or adjacent_appeal:
        raise ValueError('profile reaches unowned appeal domain requiring its general contract')
    properties=[r['properties'] for r in plots if r['properties'] is not None]
    for prop in properties:
        if len(prop['entries']) != 1:
            raise ValueError('unexpected plot properties in bounded fixture')
        row=prop['entries'][0]
        if row['key'] != 0xd9a34c75 or row['value']['kind'] != 10 or row['value']['value']['flags'] != 0x05000000 or row['value']['value']['payload']['entries']:
            raise ValueError('nonempty/foreign improvement property graph requires wider contract')
    # Same rules-input provenance, without pretending the diagnostic configuration IDs are equal.
    rules=json.loads(args.rules_report.read_text())
    parent_report=json.loads(args.parent_runtime.read_text())
    parent=parent_report['configuration']
    exported=json.loads((ROOT/'evidence/linux/definition-ids-native-1.json').read_text())['native_query']
    source=exported['source_identity']
    inventory=parent['original_filesystem_profile']['validated_launch']['validated_source_inventory_sha256']
    if not (inventory==rules['profile']['content_recipe_sha256']==source['original_filesystem_profile']['validated_launch']['validated_source_inventory_sha256'] and
            parent['saved_configuration_sha256']==rules['saved_configuration_source_sha256']==source['saved_configuration_sha256'] and
            parent['assets']==source['assets'] and parent['update_callback_profile']==source['update_callback_profile']):
        raise ValueError('rules/definition export input profile differs')
    if parent_report['native_activation_selection'] != rules['native_activation_selection'] or parent_report['native_activation_selection'] != exported['native_activation_selection']:
        raise ValueError('native content selection differs across rules/export/action runs')
    mapping=exported['mapping']
    yield_rows=[r[0] for r in db.execute('SELECT YieldType FROM Yields ORDER BY rowid')]
    if [bytes.fromhex(r['type_hex']).decode() for r in mapping['entries']] != yield_rows:
        raise ValueError('native exported Yield mapping differs from rules')
    db.close()
    return dict(schema='c6-linux-reporting-yield-query-closure-v1',checks_passed=True,
        source_sha256=sha(Path(__file__)),gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
        inputs=dict(map_sha256=MAP_SHA,map_parser_sha256=PARSER_SHA,rules_database_sha256=DATABASE_SHA,
                    rules_report_sha256=sha(args.rules_report),parent_runtime_sha256=PARENT_SHA,
                    native_yield_export_sha256=sha(ROOT/'evidence/linux/definition-ids-native-1.json'),
                    shared_source_inventory_sha256=inventory,shared_saved_configuration_sha256=parent['saved_configuration_sha256'],
                    observation_configuration='515f95429b6823d51248351d1c1731f548057ac62d15b142e3c57d4dc08a34b8',
                    separate_diagnostic_configuration_ids=True,native_activation_selections_equal=True,live_definition_fields_captured_by_this_audit=False),
        methods=[dict(image=i,rva=a,bytes=z,name=n,sha256=h) for i,a,z,n,h in METHODS],
        instruction_contracts=[dict(image=i,rva=hex(a),bytes=b,name=n) for i,a,b,n in gates],
        virtual_bindings=[dict(image=i,table=hex(t),slot=hex(s),target=hex(a),name=n) for i,t,s,a,n in bindings],
        properties=dict(serialized_plot_maps=len(properties),outer_and_improvement_map_concrete_binding='host TypedVariantMap vtable+16 at0x53b3680',
            provenance='original Plot.Read -> EditDynamicProperties -> direct concrete constructor; nested ArrayVariant.Unserialize directly constructs same class',
            improvement_key='d9a34c75',queried_yield_modifier_key='877b95f0',improvement_maps_empty=True,
            deeper_yield_map_lookup_excluded=True,const_query_allocates_or_inserts=False,
            loading_mutations='construction, insertion and ownership setup occur during load, separately from const GetYield',
            scope='deserialized native property graph; arbitrary externally rebound subclasses are unsupported'),
        improvement=dict(present_definition_hashes=len(present),adjacency_references_after_caller_yield_guard=len(adjacencies),
            adjacency_reference_projection_sha256=hashlib.sha256(json.dumps(adjacencies,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
            excluded_trait_reference_intersection=0,requested_yields=[r['index'] for r in mapping['entries']],
            actual_NUM_YIELD_TYPES_reobserved=False,nonnull_contribution_list=False,hypothetical_map_changes=False,tile_output_vector=False,
            normalize_yields_path_excluded=True,temporary_modifier_path_excluded=True,
            source_mutations_in_supported_query_path='none; explicit stores target stack temporaries or NULL-guarded optional output',
            leaf_reads=['existing player/global tech and civic masks','existing revealed-resource layer signed16 values',
                        'original map geometry/plot fields','existing district IDs, definitions and completion/pillage bits'],
            numeric_inputs_still_required=True,query_result_implemented_in_rust=False),
        appeal=dict(unowned_plots=len(unowned),unowned_improved_plots=len(unowned_improved),
            unowned_improvement_names=sorted({improvements[r['improvement_hash']]['ImprovementType'] for r in unowned_improved}),
            unowned_improvement_appeal_reference_candidates=len(unowned_appeal),adjacent_appeal_definition_rows=adjacent_appeal,
            unowned_recomputation_excluded_under_definition_input_gate=True,
            owned_appeal='direct signed16 cached read; exact requested yield and percent guards remain numeric inputs',
            general_unowned_appeal_transitive_closure=False),
        effect_contract=dict(supported='original monitor caller + typed native Map/property classes + matching exported definition inputs',
            focused_query_source_writes=False,focused_query_rng_consumption=False,focused_query_reporting_or_signals=False,
            monitor_cache_and_reporting='separate caller-owned IndexList comparisons/updates/reporting; not a universal no-op',
            unsupported='reject differing relevant property graph, definition profile or concrete provider bindings before mutation'),
        remaining=['Numeric implementation needs exact definitions/references, native integer/FixedPoint order and current player/global tech/civic/visibility/district inputs',
                   'Rules exports are separately identified original runs sharing source/configuration inputs; this audit does not capture every live definition field',
                   'General nonnull contribution/planning outputs, custom property subclasses and unowned appeal in other definitions remain separate domains',
                   'Other GetYield rule groups retain their existing contracts; this is not whole-engine getter purity or complete state/RNG proof'],
        native_execution=False,private_evidence_local_only=True,full_gameplay_state_actions_turns_implemented=False,
        full_host_action_admission=False,raw_save_load_failure_bytes_unclassified=89198)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--native-map',type=Path,default=Path('/tmp/c6-core-map-indexed-1.bin'))
    p.add_argument('--database',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-2/gameplay.sqlite')
    p.add_argument('--parent-runtime',type=Path,default=Path('/tmp/c6-core-component-event-execute-1.json'))
    p.add_argument('--rules-report',type=Path,default=ROOT/'evidence/linux/rules-xp2-request-none-snapshot.json')
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    args.output.write_text(json.dumps(audit(args),indent=2,sort_keys=True)+'\n')


if __name__=='__main__':
    main()
