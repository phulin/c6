#!/usr/bin/env python3
"""Pinned static metadata observer preparation; never executes original code."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'native/linux/unit_operation_manager_metadata.c'
HEADER = 'native/linux/unit_operation_manager_metadata.h'
AUDIT = 'tools/linux/unit_operation_manager_metadata_audit.py'
METHODS = [('gamecore',
  33570400,
  11,
  'GameCore::Context::Globals::GetConfiguration()',
  '2d3e1fadaf899ada28bed5a2c03be4245dddee7d006ab769e24e6db0b0e1baca'),
 ('gamecore',
  33342240,
  11,
  'GameCore::Context::Globals::GetGame()',
  '440d8ec0711321eeaafb3479cabd65b277a965561733cb61087722b09b561205'),
 ('gamecore',
  42879088,
  51,
  'GameCore::Configuration::Game::Instance::GetTurnPhaseType() const',
  '2ae899c84fc261748e99f2f6198cb0b9c6528623b1268aa8912cd7bfc8e36fc7'),
 ('gamecore',
  42879152,
  81,
  'GameCore::Configuration::Game::Instance::IsNetworkMultiplayer() const',
  'a785856ac0f0e8391e79e5cafeb5ee3b889d22211129933e6657a59fb4c4a57e'),
 ('gamecore',
  42882256,
  202,
  'GameCore::Configuration::Game::Instance::InitializeGameVariants()',
  '87bef5826bf9eaf25e0dd5116362e2577cf37d9e0e7743c269183e5a8db65ffe'),
 ('gamecore',
  42881936,
  132,
  'GameCore::Configuration::Game::Instance::AllocateMap()',
  'cfa57437582d728016a9551d4b3c14b0912f3b174b198ed175ad81fc5d54c36c'),
 ('gamecore',
  42878672,
  88,
  'GameCore::Configuration::Game::Instance::~Instance()',
  'b618b74134952a9abe73a202497a729603d95096d3b24227ef23b8e9025cc572'),
 ('gamecore',
  35735696,
  16,
  'GameCore::Definitions::GetUnitOperationDefinitions()',
  '81ad939829e9d0aef09dc6a8231f2738c147082b41f1061727f64f0cc3dbb701'),
 ('gamecore',
  34511808,
  5,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>::GetCount() '
  'const',
  '10b6b99e5adb80723195bcee32ed25b405c7afc2575bdd0d413f2c463cf8de2d'),
 ('gamecore',
  34511824,
  12,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>::size() '
  'const',
  '8375e78d364001210bb02186b4a5388e8e09554d7905f614c75f29f2692f7228'),
 ('gamecore',
  34290976,
  109,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>::Find(unsigned '
  'int) const',
  '6e5560f54d2973f7cffbc5de65eb89d2839e7478772244910ac8f21901dadf00'),
 ('gamecore',
  34292416,
  4,
  'GameCore::Definition::UnitOperation::GetCollectionIndex() const',
  'ff2bcadfbf055e508db5d12cdeb05fb8112c623032cf32072576f226e46e359a'),
 ('gamecore',
  34294016,
  4,
  'GameCore::Definition::UnitOperation::GetTypeHash() const',
  '580f63e5ed34152c56532cbfda2c4a0f74402790e9e026cd4888e06541f671e1'),
 ('gamecore',
  46318112,
  27,
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>::FindIndex(unsigned '
  'int) const',
  '0987f96defd19d195435710a642c1e5fcee85058aee80d49076836b0190e8f2d'),
 ('gamecore',
  34512752,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation> const&)',
  'b1857e1ade1de689270eacaa15b82957c4a6eaccbf045d053fe6771b1017cad5'),
 ('gamecore',
  47845984,
  26,
  'GameCore::Unit::Operations::Manager::Get()',
  '247a0c981c4169f399c86ecce00bb2cc63f7100c890337325450e592e3addcd4'),
 ('gamecore',
  47846992,
  82,
  'GameCore::Unit::Operations::Manager::GetHandler(GameCore::UnitOperationTypes) const',
  'a6c3ddb3a4c632585f054c52a76e6a0377372006cfa26f686dda45efe145ad13'),
 ('gamecore',
  47847200,
  86,
  'GameCore::Unit::Operations::Manager::GetHandlerIDFromHash(GameCore::Unit::Operations::Types::TypeHash) '
  'const',
  'e67620512f7440c1ca5486dc10569f19d23e2e7bc7ff46a97d7174146b9a7a86'),
 ('gamecore',
  47853360,
  12,
  'eastl::vector<GameCore::Unit::Operations::BaseHandler*, eastl::allocator>::size() const',
  '8375e78d364001210bb02186b4a5388e8e09554d7905f614c75f29f2692f7228'),
 ('gamecore',
  34473584,
  8,
  'GameCore::Game::Instance::GetPlayerManager()',
  '7a5e82ab0d9eeb4a98fea0624663246fda26b6bad0a9771e4fc3d236b7f2f300'),
 ('gamecore',
  33569088,
  39,
  'GameCore::Player::Manager::GetValidPlayer(GameCore::PlayerTypes) const',
  '58bb7297d06455fd64617c3472f7087054b4aa5fa1b28ab20ce94cb74db41648'),
 ('gamecore',
  33341760,
  7,
  'GameCore::Player::Instance::GetID() const',
  'e997d06cf0d3f9b3bb89701ec15a7b1b096198fdc4b0dc87fc0ca29c37f6dadc'),
 ('gamecore',
  45290976,
  41,
  'GameCore::Player::Instance::GetConfiguration() const',
  '7700c11c3c1ab92ec0cf51a3c668fa664a8a55a6682132d94a697a4b6fc7346d'),
 ('gamecore',
  42909296,
  41,
  'GameCore::Configuration::Player::Instance::GetSlotStatus() const',
  '22a29648f37247c1a316b7660cbef162162d34682133149854f2bf2ffbcd2ab9'),
 ('gamecore',
  33344624,
  8,
  'GameCore::Player::Instance::GetUnits() const',
  '881c20b86cb02a6b25219fefe1ac158db460204e9e4bc34be216221471420a51'),
 ('gamecore',
  33443552,
  12,
  'GameCore::Player::Units::GetCount() const',
  'b930b8a5f0d6c66958e0245758e8e2a791e6ed138d16ab757a53e77bfaa781df'),
 ('gamecore',
  33690704,
  12,
  'GameCore::Player::Units::Begin() const',
  '87a7388edb2ca1089c2f92e438b0559902f54df42b77aa83adc5040d7ab976b0'),
 ('gamecore',
  33690736,
  12,
  'GameCore::Player::Units::End() const',
  'c5e02649a25516f44f997a359dca4f57a10b74779f7e0d9f3d303d3aca711aa7'),
 ('gamecore',
  33690752,
  7,
  'RecycledIDContainer<GameCore::Unit::Instance, 8u, 16u>::const_iterator::operator->() const',
  '8b27de69f246bbea1ce9a72d955612ead4d947a493ae43bb249a7dda29e60023'),
 ('gamecore',
  33690768,
  7,
  'GameCore::Unit::IInstance::GetID() const',
  '317ef4dac77c526b04292f52727705f4b2857f4e0c5620bf48414d69903402e5'),
 ('gamecore',
  33690784,
  14,
  'RecycledIDContainer<GameCore::Unit::Instance, 8u, 16u>::const_iterator::operator++()',
  'ee14fc8eab331243e425499cf6bf96a37a747d268887d427a97eea2a287ec507'),
 ('gamecore',
  33572464,
  7,
  'GameCore::Unit::IInstance::GetOwner() const',
  '189fcf2dedb49f72260462d7135b85f6b2ee4f66daf2924b7e6167a33f461382'),
 ('gamecore',
  35067488,
  11,
  'GameCore::Context::Globals::GetScriptSystem()',
  '0d5bf15629235063deb60052f71390c42fd80e98a458e0775078bcd7bb77e07e'),
 ('gamecore',
  44388896,
  7,
  'GameCore::Phase::Processor::GetCurrentPhase()',
  '989aa79b1e94a1e41efd8bc5d6adef7557b769b9f0efd0180f0b6110098e2ea9'),
 ('gamecore',
  44388128,
  8,
  'GameCore::Phase::Processor::GetTurnPhases()',
  '082af7ef017cff0a4628c4aaeac8ad5683de65e89303698cee251537163a4124'),
 ('gamecore',
  36845472,
  4,
  'GameCore::Definition::TurnPhase::GetDatabaseRowId() const',
  '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
 ('gamecore',
  36845584,
  4,
  'GameCore::Definition::TurnSegment::GetDatabaseRowId() const',
  '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
 ('gamecore',
  44391664,
  4,
  'GameCore::Definition::TurnPhase::GetTurnPhaseTypeHash() const',
  '8e1eb1e8ee76f9d9fd9091b8e14772902a0c006899df4ef599bf389b61b64d35'),
 ('gamecore',
  44393152,
  4,
  'GameCore::Definition::TurnPhase::GetPhaseOrder() const',
  'de9f36dc140db6f626d63850679aba7ae4d76e6d68a564a1b411de985292fe18'),
 ('gamecore',
  44391328,
  5,
  'GameCore::Definition::TurnPhase::GetActiveSegment() const',
  'b811adb2d88317c4120dc7c33dd574a1678d7f98965dd45cb6cf6745cafe2e73'),
 ('gamecore',
  44391360,
  8,
  'GameCore::Definition::TurnSegment::IsAllowTacticalCommands() const',
  '06059c117d2525723825777083e470ee9220181f3044fd23f4b764be0ac6b038'),
 ('gamecore',
  46307296,
  73,
  'GameCore::Unit::Instance::IsOperationDisabled(GameCore::UnitOperationTypes) const',
  'de0f4942cf7c5e7e3e4fb4cd9ea1973ef50dc1b903add1ff673c871556839751'),
 ('gamecore',
  46398400,
  4,
  'GameCore::IndexList<bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>, '
  '&GameCore::Definitions::GetUnitOperationDefinitions, void>::edit()',
  'f84e915a39a42129d4d8c8f1c771b4a320ba7b1ab0c17754721be4ff7b835a21'),
 ('gamecore',
  33586992,
  4,
  'Types::Vector<bool, Types::LinearAlloc::Default<1u, 0u> >::size() const',
  '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
 ('gamecore',
  33450848,
  4,
  'Types::ptr64<bool>::operator bool*() const',
  'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'),
 ('gamecore',
  33689648,
  7,
  'GameCore::Player::Instance::IsTurnActive() const',
  'f44ea052578d46f191ac4e546446f30a3022dc5085b44ee6b9378719eabb8c5b'),
 ('gamecore',
  42055456,
  7,
  'GameCore::Game::Instance::IsPaused() const',
  'bf1c7941b3b62b57c2112fc90a57e28fbe0b8e7ab2b783e84a7c6731d061630f'),
 ('gamecore',
  45466560,
  7,
  'GameCore::Player::Processor::GetPlayerProcessingAutomation()',
  '750b0f58d460bdbe162c9ce1512bdd6a141a3527ce526bc67ef708695777aafb'),
 ('host',
  60158064,
  124,
  'Data::TypedVariantMap::TypedVariantMap()',
  '88ef146bf7cc119692ceb6363ed7eb31b05e1b25734903162c34b355bfca021e'),
 ('host',
  60158192,
  122,
  'Data::TypedVariantMap::TypedVariantMap(Data::TypedVariantManager*)',
  'e930cfe363710bf1526b918ee40fa709bdc4369fc7c406ed62e0bc1fd5c61188'),
 ('host',
  60159104,
  95,
  'Data::TypedVariantMap::GetVariant(unsigned int) const',
  'b264f2c22424685f2c0d51945a672854768f0363768ac961e0bd0de247274118'),
 ('host',
  52545280,
  27,
  'Data::TypedVariant::operator Data::TypedVariantMap const*() const',
  '8743eecda914a2c28d941ca5764c68b3fdfde23ea62bfe45992fbce0bc0c85df'),
 ('host',
  49689408,
  125,
  'Data::TypedVariant::operator int() const',
  'b63e903dd7b7cd54bb2164921354dee6844fb6477b7711e623fd3c2e090e6e6f'),
 ('host',
  49689536,
  125,
  'Data::TypedVariant::operator unsigned int() const',
  '380b88868b20051709d622ff110351a07618ef077476ba5699935aa0b031581c'),
 ('host',
  49689760,
  14,
  'Data::TypedVariant::IsArray() const',
  'f34dcf459773e7f857e1ad157d46af70e1597597c627d8f3d2ff62b456679a4a'),
 ('host',
  49691680,
  4,
  'Data::VariantType::GetType() const',
  '979fd779ee57a2411c6c3e4e9c6440050fee3571d0df0f0a9a95106de0ac1fb4'),
 ('host',
  60170336,
  51,
  'eastl::rbtree<unsigned int, eastl::pair<unsigned int const, Data::TypedVariant>, eastl::less<unsigned '
  'int>, eastl::allocator, eastl::use_first<eastl::pair<unsigned int const, Data::TypedVariant> >, true, '
  'true>::find(unsigned int const&) const',
  '3807ac541af5e04676e513023bfdfdb0a728d0dcbd4ab63d364b26abf20da6af'),
 ('host',
  53172176,
  19,
  'eastl::rbtree<unsigned int, eastl::pair<unsigned int const, Data::TypedVariant>, eastl::less<unsigned '
  'int>, eastl::allocator, eastl::use_first<eastl::pair<unsigned int const, Data::TypedVariant> >, true, '
  'true>::end() const',
  '4f2d6e9cfaf8c7b0f7a9a55178dbd270a6211d50a6870e8b05e160cac851a003'),
 ('host',
  53171600,
  10,
  'bool eastl::operator!=<eastl::pair<unsigned int const, Data::TypedVariant>, eastl::pair<unsigned int '
  'const, Data::TypedVariant> const*, eastl::pair<unsigned int const, Data::TypedVariant> '
  'const&>(eastl::rbtree_iterator<eastl::pair<unsigned int const, Data::TypedVariant>, eastl::pair<unsigned '
  'int const, Data::TypedVariant> const*, eastl::pair<unsigned int const, Data::TypedVariant> const&> '
  'const&, eastl::rbtree_iterator<eastl::pair<unsigned int const, Data::TypedVariant>, eastl::pair<unsigned '
  'int const, Data::TypedVariant> const*, eastl::pair<unsigned int const, Data::TypedVariant> const&> '
  'const&)',
  '4b032646ef472c6686889453fcf82f99a755f060d996a86069bb8e95c2093b1e'),
 ('host',
  53171632,
  8,
  'eastl::rbtree_iterator<eastl::pair<unsigned int const, Data::TypedVariant>, eastl::pair<unsigned int '
  'const, Data::TypedVariant> const*, eastl::pair<unsigned int const, Data::TypedVariant> '
  'const&>::operator*() const',
  '111da43af27d403431aff85821d27c6dfb0439f92612e03048c4c92b35072f05'),
 ('host',
  49691696,
  7,
  'Data::ArrayVariant::HasFlag(Data::ArrayVariant::ElementFlags) const',
  'f12bbd4f2af0ed16e1401bf73d72f99d1d715c19875a79c2c87e4c5db9e6ec47'),
 ('host',
  60170400,
  197,
  'eastl::rbtree<unsigned int, eastl::pair<unsigned int const, Data::TypedVariant>, eastl::less<unsigned '
  'int>, eastl::allocator, eastl::use_first<eastl::pair<unsigned int const, Data::TypedVariant> >, true, '
  'true>::find(unsigned int const&)',
  'c839f614702d43ba687762927ea76af24a47e7189b2d9d1d766a5826e0a596af'),
 ('host',
  52409248,
  7,
  'eastl::rbtree_iterator<eastl::pair<unsigned int const, Data::TypedVariant>, eastl::pair<unsigned int '
  'const, Data::TypedVariant> const*, eastl::pair<unsigned int const, Data::TypedVariant> '
  'const&>::rbtree_iterator(eastl::rbtree_iterator<eastl::pair<unsigned int const, Data::TypedVariant>, '
  'eastl::pair<unsigned int const, Data::TypedVariant>*, eastl::pair<unsigned int const, '
  'Data::TypedVariant>&> const&)',
  '87bbeb690e81cc8d3889e48426dcc9e4ba870d7e9147623b4a6c504e64719f2e'),
 ('host',
  53172160,
  4,
  'eastl::rbtree_iterator<eastl::pair<unsigned int const, Data::TypedVariant>, eastl::pair<unsigned int '
  'const, Data::TypedVariant> const*, eastl::pair<unsigned int const, Data::TypedVariant> '
  'const&>::rbtree_iterator(eastl::rbtree_node<eastl::pair<unsigned int const, Data::TypedVariant> > const*)',
  '539cfe51c3dc23be3a51b13f4cd6c55a914a4a8b4c38fc3a0789fb490f8d23ac'),
 ('host',
  60172064,
  4,
  'eastl::use_first<eastl::pair<unsigned int const, Data::TypedVariant> >::operator()(eastl::pair<unsigned '
  'int const, Data::TypedVariant> const&) const',
  '297ef56d5224205f2134280209526809023f7b23c5b97c6f0f85e9b2ac171036'),
 ('host',
  49810336,
  8,
  'eastl::less<unsigned int>::operator()(unsigned int const&, unsigned int const&) const',
  '73faf6b7167abaf28645c6408b6f9133c452b1fba3ead4ae01564bb6bbeb67da'),
 ('host',
  52409840,
  4,
  'eastl::rbtree_iterator<eastl::pair<unsigned int const, Data::TypedVariant>, eastl::pair<unsigned int '
  'const, Data::TypedVariant>*, eastl::pair<unsigned int const, '
  'Data::TypedVariant>&>::rbtree_iterator(eastl::rbtree_node<eastl::pair<unsigned int const, '
  'Data::TypedVariant> > const*)',
  '539cfe51c3dc23be3a51b13f4cd6c55a914a4a8b4c38fc3a0789fb490f8d23ac')]

def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def self_test():
    # No original code, native object construction or mapped game module.
    # Exercise the actual C extent/overflow guards, raw-byte preservation and
    # unsupported entry boundary; these checks do not constitute engine parity.
    with tempfile.TemporaryDirectory(prefix='c6-manager-metadata-') as work:
        work = Path(work)
        harness = work / 'check.c'
        harness.write_text('#include "' + str(ROOT / SOURCE) + '"\n' + r'''
#include <assert.h>
int main(void) {
 size_t n=99;
 assert(span(0,0,8,0,&n)&&n==0);
 assert(span(8,40,8,4,&n)&&n==4);
 assert(!span(40,8,8,4,&n));
 assert(!span(0,8,8,4,&n));
 assert(!span(8,41,8,5,&n));
 assert(!span(8,48,8,4,&n));
 assert(!span(8,8,0,0,&n));
 assert(!span(8,UINTPTR_MAX,8,65536,&n));
 assert(c6_linux_unit_operation_manager_metadata_probe(NULL,NULL)==241);
 unsigned char bytes[]={0,1,2,255};
 FILE *out=tmpfile();assert(out);raw(out,bytes,sizeof bytes);rewind(out);
 char text[11]={0};assert(fread(text,1,10,out)==10);
 assert(!strcmp(text,"\"000102ff\""));assert(!fclose(out));
 return 0;
}
''')
        command = ['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-Wconversion',
                   '-Wshadow', '-fsanitize=undefined', str(harness), '-ldl', '-o', str(work / 'check')]
        subprocess.run(command, check=True)
        subprocess.run([str(work / 'check')], check=True)
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-Wconversion',
                        '-Wshadow', '-fanalyzer', '-c', str(ROOT / SOURCE), '-o', str(work / 'observer.o')], check=True)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    images = {'gamecore': Elf(a.assets / 'libGameCore_XP2.so', GAME_SHA),
              'host': Elf(a.assets / 'Civ6', HOST_SHA)}
    methods = []
    for image, address, size, name, fingerprint in METHODS:
        require(sha(images[image].read(address, size)) == fingerprint, 'complete method differs: ' + name)
        methods.append(dict(image=image, address=hex(address), bytes=size, name=name, sha256=fingerprint))
    host, game = images['host'], images['gamecore']
    require(int.from_bytes(host.read(0x53b3690+0x70,8),'little') == 0x395f480,
            'constructed map const virtual binding differs')
    for address in [0x395f081,0x395f103]:
        raw = host.read(address,7)
        require(raw[:3] == bytes.fromhex('488d05') and address+7+struct.unpack_from('<i',raw,3)[0] == 0x53b3680,
                'map constructor vtable origin differs')
    # The whole module identity and full containing bodies above are primary
    # gates. These decisive reads additionally make the captured field ABI clear.
    instructions = [(0x2a55cf0,'8b4720c3'),(0x2a562c0,'8b4728c3'),
                    (0x23237a0,'8b4710c3'),(0x2323810,'8b4710c3'),
                    (0x2a55ba0,'488b4760c3'),(0x2021030,'8a8790020000c3'),
                    (0x2004670,'8b8730010000c3'),(0x2007f30,'8b4710c3'),
                    (0x1fe6b60,'488b07c3')]
    # Automation's RIP displacement is checked by its full body and decoded
    # target below, avoiding an unrelated neighboring build's literal bytes.
    for address, expected in instructions:
        require(game.read(address,len(expected)//2).hex()==expected,'raw field ABI differs: '+hex(address))
    automation = game.read(0x2b5c3c0,7)
    require(automation[:2]==bytes.fromhex('8b05') and 0x2b5c3c0+6+struct.unpack_from('<i',automation,2)[0]==0x3c06598,
            'automation global target differs')
    source = (ROOT / SOURCE).read_text()
    for part in ['U1 count_get=', 'II find_index=', 'int32_t (*hash_index)',
                 'c6_linux_unit_operation_manager_metadata_quiescent', 'fopen(path,"wx")',
                 'size>capacity', 'data>UINTPTR_MAX-size', 'map_binding(root,host)',
                 '(uintptr_t)as_map!=host+0x321c700', 'as_map(child)!=map',
                 'raw(out,vector,32)', 'raw(out,list,32)', 'raw(out,(const void *)data,size)',
                 'script_callbacks_inspected\\\":false']:
        require(part in source,'required source boundary absent: '+part)
    for forbidden in ['2da20f0', '2dac950', 'CallTestAll', 'CreateArgs', 'DestroyArgs', 'RequestOperation']:
        require(forbidden not in source,'forbidden gameplay/callback boundary: '+forbidden)
    dwarf = subprocess.check_output(['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so'),
        '-ex',"ptype 'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>'",
        '-ex',"ptype /o 'GameCore::IndexList<bool, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>, &GameCore::Definitions::GetUnitOperationDefinitions, void>'",
        '-ex',"ptype /o 'Types::LinearAlloc::LinearBlock<bool>'",
        '-ex',"ptype /o 'eastl::VectorBase<GameCore::Definition::TurnPhase const*, eastl::allocator>'"], text=True)
    for part in ['uint32_t GetCount() const','uint32_t size() const','int32_t FindIndex(uint32_t) const',
                 's_Purgatory','nElements','mpBegin','mpEnd','mpCapacity','mAllocator','total size (bytes):   32']:
        require(part in dwarf,'DWARF contract differs: '+part)
    self_test()
    report = dict(schema='c6-linux-unit-operation-manager-metadata-preparation-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,original_execution_performed=False,
        native_capture_available=False,new_profile_required=True,methods=methods,
        source_sha256={name:sha((ROOT/name).read_bytes()) for name in [SOURCE,HEADER,AUDIT]},
        configured_phase_provider=dict(status='closed_for_concrete_standard_map_binding_only',
            root_offset=128,game_map_offset=136,initializer='0x28e54d0',const_virtual_slot=112,
            host_map_vptr='0x53b3690',host_const_lookup='0x395f480',host_const_map_conversion='0x321c700',
            lifetime='borrowed retained Configuration/Game root owns Key_Game child; retain root and its real variant manager until observer returns',
            runtime_required=['nonnull actual root and game maps','both concrete vptr/const lookup bindings','root Key_Game const conversion equals actual configuration+0x88','exact host conversion binding'],
            unknown_override='unsupported before lookup; never substituted'),
        captured_inputs=['ordered actual operation definition index/hash and numeric/hash FindIndex/HashToIndex results plus registry IDs',
            'all actual Unit disabled IndexList size/capacity/default rawbyte/ordered raw values',
            'current raw phase enum and actual configured phase hash plus raw nullable ordered 32-byte phase vector',
            'phase/segment rowids, phase type hash, signed phase order and raw segment flags',
            'Game pause rawbyte and automation player rawenum','actual Player IDs, turn-active rawbyte and slot status',
            'ScriptSystem availability and actual vptr; callbacks remain uninspected'],
        scalar_abi=dict(definition_GetCount='u32',definition_size='u32',FindIndex='i32',HashToIndex='i32',
            phase_order='i32',current_phase='i32',automation_player='i32',phase_vector_bytes=32,
            disabled_list_bytes=32,disabled_default_offset=24,disabled_size_offset=16,disabled_capacity_offset=8),
        validation=['67 complete original method fingerprints including full host const-lookup/conversion leaf closure',
            'pinned host/game whole ELF hashes','original concrete host map virtual slot and constructor origin',
            'target DWARF widths and storage layouts','strict C warnings/analyzer','UBSan actual guard/raw-byte/preflight checks without original execution'],
        unresolved=['new source/profile-specific admission and quiescent guard','new runner integration and immutable candidate identity',
            'same-context native raw facts and independent saved Unit/PlayerHeader/rules crosschecks',
            'four internal plus two outer raw Saves, two known RNG checks, repeat capture and cleanup',
            'script callback registration/effects and complete gameplay RNG/state ownership'],
        full_manager_parity=False,full_action_supported=False,complete_state_action_rng_turns_supported=False,
        historical_raw_save_load_failure_bytes=89198)
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,complete_method_contracts=len(methods),original_execution=False,native_capture=False)))


if __name__ == '__main__':
    main()
