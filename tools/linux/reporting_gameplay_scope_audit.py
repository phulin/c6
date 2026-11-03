#!/usr/bin/env python3
"""Check scoped desktop dirty-component consumers; never execute original code."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [{'address': '0x32b6520',
  'bytes': 10,
  'name': 'bool eastl::operator==<eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>, eastl::pair<unsigned int '
          'const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>*, '
          'eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>&, eastl::pair<unsigned int '
          'const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>*, '
          'eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>&>(eastl::rbtree_iterator<eastl::pair<unsigned '
          'int const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>, '
          'eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>*, eastl::pair<unsigned int '
          'const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>&> const&, '
          'eastl::rbtree_iterator<eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>, eastl::pair<unsigned int '
          'const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>*, '
          'eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>&> const&)',
  'sha256': 'eb0bb8fcfb7acf4cf64bda1925b26421c44526881c7acd0a81ab699ff90c628a'},
 {'address': '0x32b6590',
  'bytes': 10,
  'name': 'bool eastl::operator!=<eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>, eastl::pair<unsigned int '
          'const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>*, '
          'eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>&>(eastl::rbtree_iterator<eastl::pair<unsigned '
          'int const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>, '
          'eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>*, eastl::pair<unsigned int '
          'const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>&> const&, '
          'eastl::rbtree_iterator<eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>, eastl::pair<unsigned int '
          'const, GameCore::Utility::DirtyComponentsManager::ComponentsArray>*, '
          'eastl::pair<unsigned int const, '
          'GameCore::Utility::DirtyComponentsManager::ComponentsArray>&> const&)',
  'sha256': '4b032646ef472c6686889453fcf82f99a755f060d996a86069bb8e95c2093b1e'},
 {'address': '0x32b7730',
  'bytes': 71,
  'name': 'int GameCore::Lua::MethodWrapper<GameCore::Utility::Lua::IDirtyComponents, '
          'GameCore::Utility::DirtyComponents>::BasicLuaMethod<unsigned int>(lua_State*, unsigned '
          'int (GameCore::Utility::DirtyComponents::*)() const)',
  'sha256': '2b21668a1f107df5351464c432584bc08a4c3fa93bd3a802b8966f0ed074f65f'},
 {'address': '0x32b5940',
  'bytes': 13,
  'name': 'GameCore::Utility::DirtyComponents::RemoveEvent(unsigned int)',
  'sha256': '96fd8e3223589eaaa292adaff4a323f66ab73b93f21062a88a3a28e66bc694ec'},
 {'address': '0x32b5950',
  'bytes': 86,
  'name': 'GameCore::Utility::DirtyComponents::Add(GameCore::Player::ComponentID const&)',
  'sha256': '1f4235247163eba5152a3010fbb5f27a940bf74284dd888757f30869529dd3c2'},
 {'address': '0x32b5a10',
  'bytes': 5,
  'name': 'GameCore::Utility::DirtyComponents::Clear()',
  'sha256': '9fde4c4da22929e96642b8720f7445a01a3607ca73fe8b736cba87fa0995c08a'},
 {'address': '0x32b59b0',
  'bytes': 86,
  'name': 'GameCore::Utility::DirtyComponents::Remove(GameCore::Player::ComponentID const&)',
  'sha256': 'd70198f1622403ce682c31bfbb3e2bb701c83fda6fe91fd95566d0d74fd4b8ac'},
 {'address': '0x32b5930',
  'bytes': 13,
  'name': 'GameCore::Utility::DirtyComponents::AddEvent(unsigned int)',
  'sha256': '82a7d6aa9cd467eccbfdf5f0231798a9613a270814f8a4f1ea8c3745060410f9'},
 {'address': '0x32b65f0',
  'bytes': 5,
  'name': 'GameCore::Utility::DirtyComponents::DirtyComponents()',
  'sha256': 'f51f85a75831afc4e2c94aad727a14e696a5c01fa738248befe57cfff6cd8eb0'},
 {'address': '0x32b6600',
  'bytes': 5,
  'name': 'GameCore::Utility::DirtyComponents::~DirtyComponents()',
  'sha256': '48d0f4e1d3ac6ff9cdbc9af0178f55d9978e4fe8b7ad9a58cd40019db9880ef2'},
 {'address': '0x32b5ec0',
  'bytes': 41,
  'name': 'GameCore::Utility::DirtyComponentsManager::ComponentsArray::Add(GameCore::Utility::DirtyComponents*)',
  'sha256': '3505f300feccea7e45edea492126c9cbb8d819bc4a42bc2ed5f4e5f72842bd60'},
 {'address': '0x32b5f40',
  'bytes': 76,
  'name': 'GameCore::Utility::DirtyComponentsManager::ComponentsArray::Remove(GameCore::Utility::DirtyComponents*)',
  'sha256': '8b19e87d7ddd7b8d262de4d8a9450b05c87edf2ff59fd153322bfc66ea56305f'},
 {'address': '0x32b5f90',
  'bytes': 66,
  'name': 'GameCore::Utility::DirtyComponentsManager::ComponentsArray::SetDirty(GameCore::Player::ComponentID '
          'const&)',
  'sha256': 'ae43abdb887ee6fd52a14001c01dca33c7c46ac9a7998e10134913cc240408b3'},
 {'address': '0x32b6bc0',
  'bytes': 5,
  'name': 'GameCore::Utility::DirtyComponentsManager::ComponentsArray::ComponentsArray(GameCore::Utility::DirtyComponentsManager::ComponentsArray '
          'const&)',
  'sha256': 'ec7d2aa54ab353ae3e298f2752ca2bce4eae1ebdb8c1ddeefde03cd044576859'},
 {'address': '0x32b6350',
  'bytes': 5,
  'name': 'GameCore::Utility::DirtyComponentsManager::ComponentsArray::ComponentsArray()',
  'sha256': '97fe96b8568b19c3d9bef4f89725d47f0f91769ff3c97752c5213d74e29dc4e1'},
 {'address': '0x32b6270',
  'bytes': 12,
  'name': 'GameCore::Utility::DirtyComponentsManager::GetRequestCount()',
  'sha256': '3c8f9d667f805fe9778ade0ca7c78cbbdbae204973945d1b0ce0051e1f1953ea'},
 {'address': '0x32b5fe0',
  'bytes': 243,
  'name': 'GameCore::Utility::DirtyComponentsManager::RegisterForEvent(unsigned int, '
          'GameCore::Utility::DirtyComponents*)',
  'sha256': '25a20e8e048bd180be88aeac2cf867db4cfb69de40d34460d32d7d071f50aa9a'},
 {'address': '0x32b6200',
  'bytes': 100,
  'name': 'GameCore::Utility::DirtyComponentsManager::GetEventCompoments(unsigned int)',
  'sha256': '9c914c37466f2772d71d3e074e95aec9fb3256dbe2a7c8b8972558811ad06328'},
 {'address': '0x32b60e0',
  'bytes': 111,
  'name': 'GameCore::Utility::DirtyComponentsManager::UnregisterForEvent(unsigned int, '
          'GameCore::Utility::DirtyComponents*)',
  'sha256': 'cb4c5dcef4cad8ad135b6ca8dde8b71b110e36456a4edc51acf81608fd9d8362'},
 {'address': '0x32b6150',
  'bytes': 162,
  'name': 'GameCore::Utility::DirtyComponentsManager::UnregisterForAllEvents(GameCore::Utility::DirtyComponents*)',
  'sha256': '44712ea75fc00f5d512b2e48d461341d94b97ea193870d431ce7a4462c80a197'},
 {'address': '0x32b6280',
  'bytes': 130,
  'name': 'GameCore::Utility::DirtyComponentsManager::Create()',
  'sha256': '51531c8b299da6d3a797fe7878da24b1dfddb4eb2f7d69bb49958ca7eee12189'},
 {'address': '0x32b6310',
  'bytes': 51,
  'name': 'GameCore::Utility::DirtyComponentsManager::Destroy(GameCore::Utility::DirtyComponents*)',
  'sha256': '0a1012a9a6d54576fc5222dc6a3c3570b457908f2eb2116d38ca650ba434faf8'},
 {'address': '0x32b77d0',
  'bytes': 8,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::GetTypeName()',
  'sha256': '03297539e9c6caf8cbf8a740c7610adcb30d8199eb5961f96c3254285d67320c'},
 {'address': '0x32b7050',
  'bytes': 275,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::PushMethods(lua_State*, int)',
  'sha256': '0a799490681b1f7edf66aba9115d43e469d9e166e88faa8ca92496d211491d90'},
 {'address': '0x32b71c0',
  'bytes': 310,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::lMembersAux(lua_State*)',
  'sha256': 'f22912269c19a04549787bb1413a3d80ded3a6cf9badae357f72a7c91161f145'},
 {'address': '0x32b7340',
  'bytes': 498,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::lAddComponent(lua_State*)',
  'sha256': 'db16c49178edc11a8c070d8987812ad7487cbd97efbf6616ce08799324413757'},
 {'address': '0x32b7540',
  'bytes': 5,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::HandleMissingInstance(lua_State*)',
  'sha256': 'd228fa3863e1d89c8e1143c99efea41151ab00372e66725f9b659c02c5cd2014'},
 {'address': '0x32b7600',
  'bytes': 14,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::lClear(lua_State*)',
  'sha256': 'd0bc8945b2fa2372ca5f8422cda234dcb71030246cdd92c358e3c7029071a6ce'},
 {'address': '0x32b7170',
  'bytes': 66,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::lMembers(lua_State*)',
  'sha256': 'd1f2f66f453f3f69c0e87dd02dc08d4b3455660988ff189ef7ad3925dd925b02'},
 {'address': '0x32b7300',
  'bytes': 62,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::lAddEvent(lua_State*)',
  'sha256': '11953c5f8d10c08eaa6bdf372ad9e9116d1eb789750245d28f3d5d4c972cac20'},
 {'address': '0x32b75f0',
  'bytes': 14,
  'name': 'GameCore::Utility::Lua::IDirtyComponents::lGetCount(lua_State*)',
  'sha256': 'ccb2bd8cf7b2c99817b207b5e3b826b9d2c70d8dc4e4bc0be5254d32ccb13edc'},
 {'address': '0x32b77e0',
  'bytes': 3,
  'name': 'GameCore::Utility::Lua::IDirtyComponentsManager::GetInstance(lua_State*, int)',
  'sha256': '251447ee91a9067dcd6ab96703133f617565974cd6c4819021760c4688c91abf'},
 {'address': '0x36af680',
  'bytes': 8,
  'name': 'GameCore::Utility::Lua::IDirtyComponentsManager::GetInstanceName()',
  'sha256': 'ef8d673a8da2be8548d8fec1d2c07b99e934faedd703746ae50ca5d870add80d'},
 {'address': '0x32b77f0',
  'bytes': 46,
  'name': 'GameCore::Utility::Lua::IDirtyComponentsManager::RegisterMembers(lua_State*)',
  'sha256': '6a092c53e455910ffaa0bf6371bc44b540307eee540340098cacf89c09d09b6a'},
 {'address': '0x32b7820',
  'bytes': 27,
  'name': 'GameCore::Utility::Lua::IDirtyComponentsManager::lCreate(lua_State*)',
  'sha256': '2062ab7726e52c1cbda69984688bd7c14497a3d386528959c34c4ab566c603ea'},
 {'address': '0x32b7840',
  'bytes': 28,
  'name': 'GameCore::Utility::Lua::IDirtyComponentsManager::lDestroy(lua_State*)',
  'sha256': 'cc0f28ef537121171975863858ab91cdf6bbe5cdfb551106d1d89013c209da76'},
 {'address': '0x32b76f0',
  'bytes': 4,
  'name': 'GameCore::Utility::DirtyComponents::GetArray() const',
  'sha256': 'f84e915a39a42129d4d8c8f1c771b4a320ba7b1ab0c17754721be4ff7b835a21'},
 {'address': '0x32b7780',
  'bytes': 5,
  'name': 'GameCore::Utility::DirtyComponents::GetCount() const',
  'sha256': 'f2caa9e32b1dce79ce64efba5806a764e131b9d00f8f88603f6b8002f83abcf3'},
 {'address': '0x32b5ef0',
  'bytes': 66,
  'name': 'GameCore::Utility::DirtyComponentsManager::ComponentsArray::Contains(GameCore::Utility::DirtyComponents*) '
          'const',
  'sha256': 'f220f44bc3da71e1e8cfb38517ba6583b5b90760cf62e30031e9357c5014c275'},
 {'address': '0x32b65e0',
  'bytes': 5,
  'name': 'GameCore::Utility::DirtyComponentsManager::ComponentsArray::GetCount() const',
  'sha256': 'f51f85a75831afc4e2c94aad727a14e696a5c01fa738248befe57cfff6cd8eb0'}]
REFERENCES = [{'caller': '0x2c64420', 'site': '0x2c84021', 'target': '0x649dac0'},
 {'caller': '0x2c64420', 'site': '0x2c84066', 'target': '0x649dae0'},
 {'caller': '0x32b4260', 'site': '0x32b427a', 'target': '0x32b6200'},
 {'caller': '0x32b5f90', 'site': '0x32b5fb6', 'target': '0x32b5950'},
 {'caller': '0x32b5fe0', 'site': '0x32b5ff2', 'target': '0x649dae0'},
 {'caller': '0x32b5fe0', 'site': '0x32b6059', 'target': '0x649dae0'},
 {'caller': '0x32b60e0', 'site': '0x32b60f2', 'target': '0x649dae0'},
 {'caller': '0x32b6150', 'site': '0x32b615e', 'target': '0x649dae0'},
 {'caller': '0x32b6150', 'site': '0x32b6199', 'target': '0x649dae0'},
 {'caller': '0x32b6200', 'site': '0x32b620d', 'target': '0x649dae0'},
 {'caller': '0x32b6270', 'site': '0x32b6270', 'target': '0x649dac0'},
 {'caller': '0x32b6280', 'site': '0x32b62bc', 'target': '0x32b65f0'},
 {'caller': '0x32b6280', 'site': '0x32b62c1', 'target': '0x649dac0'},
 {'caller': '0x32b6310', 'site': '0x32b6321', 'target': '0x649dac0'},
 {'caller': '0x32b71c0', 'site': '0x32b7232', 'target': '0x32b76f0'},
 {'caller': '0x32b71c0', 'site': '0x32b7246', 'target': '0x32b76f0'},
 {'caller': '0x32b71c0', 'site': '0x32b725a', 'target': '0x32b76f0'},
 {'caller': '0x32b71c0', 'site': '0x32b7270', 'target': '0x32b76f0'},
 {'caller': '0x32b7300', 'site': '0x32b732f', 'target': '0x32b5930'},
 {'caller': '0x32b7340', 'site': '0x32b73bb', 'target': '0x32b5950'},
 {'caller': '0x32b7340', 'site': '0x32b73f4', 'target': '0x32b5950'},
 {'caller': '0x32b7340', 'site': '0x32b742a', 'target': '0x32b5950'},
 {'caller': '0x32b7340', 'site': '0x32b749c', 'target': '0x32b5950'},
 {'caller': '0x32b75f0', 'site': '0x32b75f0', 'target': '0x32b7780'},
 {'caller': '0x32b7600', 'site': '0x32b7600', 'target': '0x32b5a10'},
 {'caller': '0x32b7820', 'site': '0x32b7824', 'target': '0x32b6280'},
 {'caller': '0x32b7840', 'site': '0x32b7853', 'target': '0x32b6310'}]
LUA_SOURCES = [{'path': 'Base/assets/ui/unitflagmanager.lua',
  'sha256': '053c28decfefda03ba198bb49e45d8fd3daaba82b1f9846e26d1463511db267f'},
 {'path': 'Base/assets/ui/worldview/citybannermanager.lua',
  'sha256': 'b151d5570fa1fb148c48757242943ffdeab1e0c42a674cd143b8784c85d6d298'},
 {'path': 'DLC/expansion1/ui/citybanners/citybannermanager.lua',
  'sha256': '83e0ef08ce0c8d6b9c265ec3ef7ec9d723827bad21ad9f93c4b427babcdba91a'},
 {'path': 'DLC/expansion2/ui/citybanners/citybannermanager.lua',
  'sha256': 'd5567524fa954fa0dd2f255bba2f2ab4230bb27546019de01a67e0a963bf5f9d'},
 {'path': 'DLC/polandscenario/ui/unitflagmanager.lua',
  'sha256': 'c2b7e8b15185692750b6bf734c56e1eab739f6021335a5bc7896b79953ed1195'}]


def reference_census(host_path):
    targets = {int(r['target'], 16) for r in REFERENCES}
    # Includes zero-hit APIs: removal, Clear, GetRequestCount and constructor.
    targets.update([0x32b5940, 0x32b59b0, 0x32b6270])
    result = []
    caller = None
    with tempfile.TemporaryFile(mode='w+t') as output:
        subprocess.run(['objdump', '-d', '-C', '--no-show-raw-insn', str(host_path)],
                       stdout=output, check=True)
        output.seek(0)
        for line in output:
            label = re.match(r'([0-9a-f]+) <.*>:', line)
            if label:
                caller = hex(int(label[1], 16))
                continue
            if 'DirtyComponents' not in line:
                continue
            instruction = re.match(r'\s*([0-9a-f]+):\s+(.*)', line)
            if not instruction:
                continue
            for target in targets:
                if re.search(r'\b' + format(target, 'x') + r' <', instruction[2]):
                    result.append(dict(site=hex(int(instruction[1], 16)),
                                       target=hex(target), caller=caller))
    return result


def audit(assets):
    host = Elf(assets / 'Civ6', HOST_SHA)
    Elf(assets / 'libGameCore_XP2.so', GAME_SHA)
    for method in METHODS:
        actual = hashlib.sha256(host.read(int(method['address'], 16), method['bytes'])).hexdigest()
        if actual != method['sha256']:
            raise ValueError('method changed: ' + method['name'])
    references = reference_census(assets / 'Civ6')
    if references != REFERENCES:
        raise ValueError('selected dirty-component direct references changed')
    lua_sources = []
    for path in sorted(assets.rglob('*.lua')):
        raw = path.read_bytes()
        if b'DirtyComponents' in raw:
            lua_sources.append(dict(path=str(path.relative_to(assets)),
                                    sha256=hashlib.sha256(raw).hexdigest()))
    if lua_sources != LUA_SOURCES:
        raise ValueError('supplied Lua dirty-component consumer set changed')
    registrations = {}
    for source in LUA_SOURCES:
        text = (assets / source['path']).read_bytes().decode('latin-1')
        registrations[source['path']] = re.findall(r':AddEvent\(\s*"([^"]+)"\s*\)', text)
    return dict(
        stock_lua_event_registrations=registrations,
        skip_turn_dirty_consumer_classification=[
            dict(event='UnitActivityChanged', hash='e23b112d', calls=3, stock_consumer='UnitFlag visual refresh'),
            dict(event='UnitOperationDeactivated', hash='a60328c5', calls=2, stock_consumer='UnitFlag visual refresh'),
            dict(event='UnitOperationsCleared', hash='383669f1', calls=2, stock_consumer='No literal stock registration'),
            dict(event='UnitOperationAdded', hash='15c35863', calls=1, stock_consumer='No literal stock registration'),
            dict(event='UnitOperationStarted', hash='3d4006f2', calls=1, stock_consumer='No literal stock registration'),
            dict(event='NotificationActivated', hash='025f6cfd', calls=1, stock_consumer='No literal stock registration'),
            dict(event='Queue complete', hash='9ea4c222', calls=1, stock_consumer='No literal stock registration'),
        ],
        schema='c6-linux-reporting-gameplay-scope-v1', checks_passed=True,
        native_execution=False, gameplay_parity=False, full_host_admission=False,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        methods=METHODS, selected_direct_references=references, lua_sources=lua_sources,
        proven_visual_receiver=dict(
            source='Base/assets/ui/unitflagmanager.lua',
            register_events=['UNIT_OPERATION_DEACTIVATED', 'UNIT_ACTIVITY_CHANGED',
                             'UNIT_MOVEMENT_POINTS_CHANGED', 'UNIT_EMBARK_CHANGED'],
            drain='OnEventPlaybackComplete at1877 iterates Members; UpdateFlagType at593 writes four textures; UpdateReadyState at377 sets local dim flag and control alpha via SetDim at530 and UpdateDimmedState at546; then Clear',
            lifetime='RegisterDirtyEvents at1968 creates receiver; OnShutdown at2098 destroys it; playback handler added at2036 and removed at1992',
            excluded='The concrete stock UnitFlag receiver list, textures, local dim flag and alpha refresh are desktop presentation state; they need not be reconstructed in the headless gameplay state'),
        native_lifetime=dict(
            create='lCreate at32b7824 calls Create at32b6280; allocates32B vector, constructs, adds pointer to active requests',
            register='AddEvent forwards event hash and receiver to event-map registration; records receiver pointer without callback',
            read='GetArray has four direct callers, all MembersAux32b71c0; GetCount address used by Lua wrapper32b75f0; iteration returns owner/id and updates Lua cursor only',
            destroy='Destroy32b6310 unregisters all events, removes active pointer, destroys vector and deletes; Clear32b5a10 clears IDs independently of registration'),
        admission_change='Do not require recreation/capture of the concrete stock desktop UnitFlag dirty receiver as a headless action prerequisite. No Rust gameplay transition is missing on that receiver path. In the supplied stock sources the three CityBanner variants register only CITY_POPULATION_CHANGED and CITY_RELIGION_CHANGED, neither present in the eleven-call SkipTurn inventory. Both UnitFlag variants have the same visual drain. Therefore receiver ID-vector capture is not a headless SkipTurn admission gate for a verified stock source profile without additional consumers.',
        scope_limits=[
            'Direct references are a selected API/storage census, not proof against arbitrary indirect calls, aliases or dynamically loaded scripts.',
            'Five supplied Lua source files are identified by hashes, not by an asserted active mod/configuration identity.',
            'City banner dirty drains call UpdateStats; its transitive numerical getters and all UI native getter effects are not classified by this audit.',
            'Actual registrations and receiver vector contents of the historical native replay remain uncaptured; do not assert they are empty or all belong to UnitFlag.',
            'Other Core cache delegates, translators, queued event consumers, scripting gameplay callbacks and RNG remain independent obligations.',
            'Core tracked archive dirty history is distinct and remains in gameplay initialization scope.',
        ],
        private_raw_evidence='../c6-ghidra/evidence/linux-reporting-gameplay-scope/host-dirty-components.disassembly.txt',
        save_load_raw_failure_bytes=89198, unknown_save_bytes_masked=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.assets)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(checks_passed=True, method_gates=len(METHODS),
                         direct_references=len(REFERENCES), native_execution=False)))


if __name__ == '__main__':
    main()
