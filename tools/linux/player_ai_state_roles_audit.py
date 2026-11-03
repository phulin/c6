#!/usr/bin/env python3
"""Read-only Linux Player AI suffix role triage; no original-code execution.

Full method hashes gate structural leads. Only Planning.Manager wire schema is
closed here; direct-call RNG screening is not a transitive RNG proof.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
from player_wmds_audit import Elf, GAME_SHA, FIXTURES, relocation_symbols

ROOT = Path(__file__).resolve().parents[2]
def sha(data):
    return hashlib.sha256(data).hexdigest()

METHODS = [[33673056, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Control::Manager const&)', '61732d6dca66075fe04edb9b66bcaa7e89893a762bba9d50e0cb74ac9b372daf'], [33673376, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Control::Manager&)', '04c901c1de77ce3842013ffc484aaaea9839ad3a5d018a82103651c4f75e1ab9'], [33673408, 401, 'GameCore::AI::Control::Manager::Read(Serialization::IStream&)', 'd593b24d73bcc457218164da8b971daf2fc46b4345724c1eb158abb2272bab41'], [33673088, 279, 'GameCore::AI::Control::Manager::Write(Serialization::OStream&) const', '2ec55fb0cae521cfd78e3a52ad0621c3a5056de61942c9534d7ad0891e16c139'], [34868736, 72, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::InfluenceMap const&)', 'a0a16314da6af675604dcd79e64135e1b6924929848d3a91ed40a08418f252d3'], [34868816, 81, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::InfluenceMap&)', 'dd1bd2f89a8d60da3a5aa2a714d61346c2f9557ed7667d7ec3bc592dd66ccc89'], [35353952, 65, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Planning::Manager const&)', '550f5525c70c92d35a15473263690427dc4ef816166ec2176bdf9f18c91827d0'], [35354032, 80, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Planning::Manager&)', '9ad35ccc1cdb875cf76b58dc28e3349e9a93395eaf466fb3a87bf0a030d4e4d8'], [33355024, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::CityManager const&)', '08b40fbcfc07dd6c90917d2225754af52ead90ca7f6c73ac77f9213963b71cbd'], [33355136, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::CityManager&)', 'a370e54d1664d91b88120b4fcc955994adff7704a94e8768be39209cbc5fb990'], [33355168, 156, 'GameCore::AI::CityManager::Read(Serialization::IStream&)', 'f8a5c8ec602a903a31be61bed9f95bd7e06a5bcfaf73f81be10d3619fd8a5a08'], [33355056, 70, 'GameCore::AI::CityManager::Write(Serialization::OStream&) const', '897e1944daecb1fe76a25b2cbf05e021552c596d7f9bf6f3fa9c036a0cc37866'], [33721440, 444, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::CultureAi const&)', '45c6a6d52c4e9a042b4e0c188e61c95d4169fcad784c3a7cc1bf4244598da3b1'], [33721888, 1227, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::CultureAi&)', '6b4f8de366f68cd51529bb1df13246923c96379bf3949f24a823a6136bf49264'], [33929248, 615, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Diplomatic const&)', '5601e1aa6aa12e68757f0d062aea882f8c897191603b3615ab5be7d76410a137'], [33929872, 2254, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Diplomatic&)', '584b0962ae024fac076ac163e0337c23211f81dd900ec81dac78727714532b60'], [34222320, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Economic const&)', '2350cdbd7d76ca81c9d10f845c85a3b8d7d5ebf51a70be9439ea4b73780b30e1'], [34222656, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Economic&)', 'f655e529d57d5ed8ea34bf5674030d5caed2f41261373f4254668f3b1f8275a9'], [34222688, 581, 'GameCore::AI::Economic::Read(Serialization::IStream&)', 'cac17576ae90edb8c4bdf532491dc993fa3c870526f8928c6036e0f1e40f9f2f'], [34222352, 289, 'GameCore::AI::Economic::Write(Serialization::OStream&) const', '685bd29662bf9a9991314ca30cd3c13b365a8221437befa8190df94c54d99edf'], [34692000, 153, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Governors const&)', '31e289fe1af236eadc6dc10127343f2ecd47c7c8e3b395f23809387c5495ba24'], [34692160, 249, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Governors&)', 'e11c822eecee64f50f79c07119cd6bc07fbf8ef053fd9cc08e47867120e12d86'], [34726240, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::GrandStrategic const&)', 'b5af5b8296aa07070e997fba022da1dd02a093c60312b08a366d4cbfbb8b6636'], [34726544, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::GrandStrategic&)', '5cb3d06063e3743ee0c23bab44a23f5a1c8e8604036d814d4bfa46d63842e15f'], [34726576, 1027, 'GameCore::AI::GrandStrategic::Read(Serialization::IStream&)', 'b666911c1ed849812f6cd838ec9409cca4e1e543043124eadc46b9ad2207199b'], [34726272, 260, 'GameCore::AI::GrandStrategic::Write(Serialization::OStream&) const', '6cce3c9bed153c54f49f344cb2451c03a9942e780d610a849a04f05b756a9570'], [34818144, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Homeland const&)', '5392410370bc45f65b6faa22712923d89bf6c588cf581cf24c2ea39c120df51a'], [34818288, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Homeland&)', 'e8198b5339780eda18c31bcb018c50d6af7237b1bf6c8d5c4e9652ab0d409e82'], [34818320, 613, 'GameCore::AI::Homeland::Read(Serialization::IStream&)', '9bbce19b6aa9a87c6c98f58ccd14b4b4c3450098186d7b6c902149c2f02968a6'], [34818176, 110, 'GameCore::AI::Homeland::Write(Serialization::OStream&) const', 'd3348153729698c960267d393f738d84d18ffe149cd5437e7f19eb2dcf818490'], [34886640, 551, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Military const&)', '3f934803bfe326f35a629683ab2462fd0141671377b0d61797b0c40a8a489c20'], [34887200, 1472, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Military&)', '34f26f5196c12039960ac4fb8581e560a7f30b59228cee65a8b1cdf754d01f9f'], [35142912, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Research const&)', '90c09ef6b51923c6614abc61c1ee01be629741db414e16cef1a433b1ea08a725'], [35143200, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Research&)', '8b7b24c38c48c5ca9a3d684bf8bde978317808629b7dd7ed76b13f835662f472'], [35143232, 473, 'GameCore::AI::Research::Read(Serialization::IStream&)', '7920af77ab2594085727dce5587ae51dc393a93d9ab0b8e2921a89147a944cfb'], [35142944, 241, 'GameCore::AI::Research::Write(Serialization::OStream&) const', 'ded8c844d3235828ac71e40b77e8cf6b086d069a9953a5e6031fa3a217fb38dd'], [35214432, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Tactical const&)', '0f90e9e8e0e5c95083dfcbb50708dc17c9128f72ff88066b9956e194505d7ef6'], [35214576, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Tactical&)', 'b5bc2461223ab001e681b4a024816e22c183f437b26dd7f25a1fffd2ab218679'], [35214608, 771, 'GameCore::AI::Tactical::Read(Serialization::IStream&)', '7c35f6123dae527d19815139daf5334e9ac7e1638b1433a4a2ae9badfab16115'], [35214464, 110, 'GameCore::AI::Tactical::Write(Serialization::OStream&) const', '3405dabaaa3fdecaff7c74d3d2f7c0df81f71c59ae67cec359126d3ae3dc05b1'], [35289536, 188, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Targets const&)', 'f025d76c80f94f28c6cea72cbd76968e65d1dcde71e6beebdd6ca401f61cd94c'], [35289728, 377, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Targets&)', 'c0512a227d0384de94fbb960e459cc84f115da90022963e40051c1d0c62fba1d'], [40779840, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::BehaviorTree::Manager const&)', '4dde4561fe5520a1b16f69a33c29c580d2d2df07eedbbb3d259acc63d4978551'], [40779968, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::BehaviorTree::Manager&)', 'c8ca79358c24f4f76e24bcbbee4668878304dc380746d7753764e1a265e6f29e'], [40780000, 103, 'GameCore::AI::BehaviorTree::Manager::Read(Serialization::IStream&)', '0dfb0dcdb7901d93a0533f8588eccc3bd20a234b9174a50a035b5f2733fb95db'], [40779872, 95, 'GameCore::AI::BehaviorTree::Manager::Write(Serialization::OStream&) const', '53fb600830e5b240f7ac06eb8ad900d410411a15d89a3365e6b55caf4c7c75c7'], [33341856, 14, 'Serialization::IStream& Serialization::IStream::operator>><int>(int&)', 'c6cf9d3175d5134bd8d7b3d396184fd7b9abd3f7ca364cbdc38a1794ce742775'], [33341888, 14, 'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)', '5edd23b31cc38a2cc74b61c4b33889da2f35c69f9ce10ee24f2899eb7e91af01'], [33341776, 14, 'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)', '9731ca1d2959dc0f406a684267a29d425c9bf7ea2f127425123508fbc2ed2d3b'], [46771104, 33, 'Serialization::operator<<(Serialization::OStream&, GameCore::PlayerTypes const&)', 'e3aa2b85edbfcad11d2fe4b8d720bad7d662e88d8461f7994ba25444ae60c6d1'], [33943392, 3429, 'GameCore::AI::Diplomatic::DoTurnActivate(GameCore::AI::Control::ControlPriority, bool)', '14e8fdd2afa83fd154d16772385388e7af512035023c4cfc0e7fead62e1bca2a'], [34869808, 2484, 'GameCore::AI::InfluenceMap::UpdateInfluenceMap()', '248ca60a6f22f6a94769a6f04001a7fe1ca5e35f2c12e07f03045a592c1e4da4'], [35354400, 175, 'GameCore::AI::Planning::Manager::InitArchive()', '1216791c9de5bd65a8a17f9f619550ca9f3882afd829b08b1d2de3c98bb7d1f3'], [35361408, 384, 'GameCore::AI::Planning::Manager::DoTurnActivate(GameCore::AI::Control::ControlPriority, bool)', '2df794b54ee6a535f0a97df5a12bc73753531029f6785edf8e5003654197670c'], [35361792, 175, 'GameCore::AI::Planning::Manager::DoTurnDeactivate(GameCore::AI::Control::ControlPriority)', '49abf0e9ef0f5d5ac3a548f923877e63eb9febb58d225c0c5b895e8d644de015'], [35354112, 275, 'GameCore::AI::Planning::Manager::InitializeSignals()', 'e2afaa468cfcc03e628a3399c8a6bedc01dc878a74a2069bfc575b2b633889a0'], [35355936, 1237, 'GameCore::AI::Planning::Instance::DoCombatSimulations()', '7473443eb83c271a7448a87229e125f93b10f8076b0295d007814df572db9c9f'], [35152144, 764, 'GameCore::AI::Research::QueueNextTech(GameCore::TechTypes)', '43083232b2dd8be98852f956a92ee0c54b86098bb5276627d70a5711aa4c7404'], [35150448, 1693, 'GameCore::AI::Research::DoTurnActivate(GameCore::AI::Control::ControlPriority, bool)', '28b95cddc1af99cb4e1f6613e0cc4dfb1638c631d6792a5c59552e55f3af4aa3'], [33444416, 39, 'GameCore::Game::Instance::GetRandNum(unsigned short, char const*)', 'dc5512d444383e583abab29ae301710fcd783ed9f508fc1977a5c3024e69bd1a'], [35168000, 75, 'GameCore::Utilities::WeightedVector<GameCore::TechTypes, 10>::ChooseAtRandom(fastdelegate::FastDelegate2<unsigned short, char const*, unsigned short>*, char const*)', 'f6151d61034060bc450d5c88f4958f4879580addda42e27d835306a8dd337358'], [33341760, 7, 'GameCore::Player::Instance::GetID() const', 'e997d06cf0d3f9b3bb89701ec15a7b1b096198fdc4b0dc87fc0ca29c37f6dadc']]
BLOCKS = {'Control::Manager': [33673056, 33673376, 33673408, 33673088], 'InfluenceMap': [34868736, 34868816], 'Planning::Manager': [35353952, 35354032], 'CityManager': [33355024, 33355136, 33355168, 33355056], 'CultureAi': [33721440, 33721888], 'Diplomatic': [33929248, 33929872], 'Economic': [34222320, 34222656, 34222688, 34222352], 'Governors': [34692000, 34692160], 'GrandStrategic': [34726240, 34726544, 34726576, 34726272], 'Homeland': [34818144, 34818288, 34818320, 34818176], 'Military': [34886640, 34887200], 'Research': [35142912, 35143200, 35143232, 35142944], 'Tactical': [35214432, 35214576, 35214608, 35214464], 'Targets': [35289536, 35289728], 'BehaviorTree::Manager': [40779840, 40779968, 40780000, 40779872]}
VERSIONS = {'Control::Manager': 2, 'InfluenceMap': 2, 'Planning::Manager': 1, 'CityManager': 2, 'CultureAi': 15, 'Diplomatic': 18, 'Economic': 4, 'Governors': 3, 'GrandStrategic': 12, 'Homeland': 1, 'Military': 14, 'Research': 4, 'Tactical': 2, 'Targets': 4, 'BehaviorTree::Manager': 1}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root', type=Path, default=ROOT.parent/'c6-ghidra')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    game = Elf(args.assets/'libGameCore_XP2.so', GAME_SHA)
    relocations = relocation_symbols(game)
    methods = []
    for address, size, name, digest in METHODS:
        if sha(game.read(address, size)) != digest:
            raise ValueError('method mismatch: '+name)
        disassembly = subprocess.check_output(['objdump', '-d', '-C',
            '--start-address='+hex(address), '--stop-address='+hex(address+size),
            str(args.assets/'libGameCore_XP2.so')], text=True)
        edges = []
        indirect = []
        for line in disassembly.splitlines():
            m = re.search(r'^\s*([0-9a-f]+):.*?\b(call|jmp)\s+([0-9a-f]+) <(.*)>$', line)
            if m:
                site, target = int(m[1], 16), int(m[3], 16)
                if address <= target < address+size:
                    continue
                body = target
                stub = game.read(target, 6)
                if stub[:2] == b'\xff\x25':
                    body = relocations.get(target+6+struct.unpack_from('<i', stub, 2)[0])
                edges.append(dict(site=hex(site), target=hex(target), body=hex(body) if body is not None else None, name=m[4]))
            elif re.search(r'\b(call|jmp)\s+\*', line):
                indirect.append(hex(int(line.split(':')[0].strip(), 16)))
        methods.append(dict(address=hex(address), size=size, name=name, sha256=digest,
            direct_edges=edges, indirect_sites=indirect,
            direct_rng_named_edges=[x for x in edges if re.search(r'Random|RNG|Rand\(', x['name'])]))
    if struct.unpack('<I', game.read(0x2dfbc20, 4))[0] != 2:
        raise ValueError('CityManager version mismatch')
    reloc_text = subprocess.check_output(['readelf', '-rW', str(args.assets/'libGameCore_XP2.so')], text=True)
    if not re.search(r'^0000000003b1c648\s+\S+\s+R_X86_64_GLOB_DAT\s+0000000001fe5240\s+_ZN8GameCore4Game8Instance10GetRandNumEtPKc', reloc_text, re.M):
        raise ValueError('research random delegate relocation mismatch')
    dwarf = subprocess.check_output(['gdb', '-nx', '-batch', str(args.assets/'libGameCore_XP2.so'),
        '-ex', "ptype /o 'GameCore::AI::Planning::Manager'"], text=True)
    for field in ['m_Initialized', 'm_player', 'm_priority', 'm_unitPlanners']:
        if field not in dwarf:
            raise ValueError('missing DWARF field '+field)
    fixtures = []
    for identity, trace_name, trace_sha in FIXTURES:
        data = gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
        trace = gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
        if sha(data) != identity or sha(trace) != trace_sha:
            raise ValueError('fixture identity mismatch')
        owners = {x['call']: x for x in map(json.loads, trace.splitlines()) if x.get('event') == 'owner' and x.get('save') == 1}
        records = []
        for row in sorted(owners.values(), key=lambda x: x['start']):
            if row['owner'] != 'GameCore::AI::Planning::Manager':
                continue
            parent = owners[row['parent']]
            raw = data[row['start']:row['end']]
            if len(raw) != 8 or parent['owner'] != 'GameCore::Player::Instance':
                raise ValueError('Planning record boundary mismatch')
            version, player = struct.unpack('<Ii', raw)
            parent_version, parent_player = struct.unpack_from('<ii', data, parent['start'])
            if version != 1 or parent_version != 47 or player != parent_player:
                raise ValueError('Planning identity mismatch')
            if struct.pack('<Ii', version, player) != raw:
                raise ValueError('Planning typed scalar reconstruction differs')
            records.append(dict(start=row['start'], end=row['end'], player=player, sha256=sha(raw)))
        if len(records) != 43:
            raise ValueError('Planning owner count mismatch')
        fixtures.append(dict(core_sha256=identity, trace_sha256=trace_sha, records=records))
    blocks = [dict(order=i, name='GameCore::AI::'+n, current_version=VERSIONS[n],
        methods=[hex(a) for a in addresses],
        whole_block_exclusion_proven=False,
        complete_wire_schema=n == 'Planning::Manager',
        role='required player identity' if n == 'Planning::Manager' else 'unclassified; named planning data does not exclude gameplay consumers')
        for i, (n, addresses) in enumerate(BLOCKS.items(), 1)]
    report = dict(schema='c6-linux-player-ai-state-roles-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, audit_source_sha256=sha(Path(__file__).read_bytes()),
        methods=methods, blocks=blocks, planning_dwarf_sha256=sha(dwarf.encode()),
        planning_schema=['u32 version = 1', 'i32 player_id'], planning_bytes=8,
        planning_load_effects=['GetPlayer(player_id) -> m_player +0xa0', 'InitializeSignals', 'InitArchive', 'm_Initialized +0x98 = true'],
        planning_unsaved_fields=['m_priority', 'm_unitPlanners'],
        research_policy_rng=dict(phase='Research::DoTurnActivate', caller_site='0x2185eb0', policy='0x2186110', random_call_site='0x2186360', callback_relocation='0x3b1c648', callback_body='0x1fe5240', game_random_wrapper_offset='0xe8', gameplay_mutation_sites=['0x2186224', '0x218638d'], replacing_policy_rng_contract_validated=False),
        fixtures=fixtures, original_execution=False, private_writes=False,
        native_read_side_effects_reproduced=False, transitive_rng_audit_complete=False,
        planning_only_block_exclusions=[], unknown_bytes_masked=0,
        recorded_raw_save_load_difference_bytes=89198,
        full_player=False, gameplay_state=False, actions=False, turns=False,
        reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175', reference_role='discovery only')
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, methods=len(methods), blocks=len(blocks), planning_records=sum(len(x['records']) for x in fixtures))))


if __name__ == '__main__':
    main()
