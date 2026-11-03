#!/usr/bin/env python3
"""Read-only exact Linux plot-yield effects and conditional branch audit."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

METHODS = [('gamecore', 33432112, 24, 'GameCore::Map::Plot::Instance::GetLocation() const', '8820929392df8cf5ef38d48429b5ac9e8b9b5be1ea4829b44857889a36f4e3bf'), ('gamecore', 33432528, 41, 'GameCore::Map::Plot::Instance::GetOwner() const', 'd40d1bd008ab26ed6eac1cc16bfd19ecf84925b3ce6bf6ef79d1d97e1520ce05'), ('gamecore', 33432592, 5, 'GameCore::Map::Plot::Instance::GetFeatureType() const', 'e15cd107dd900e9a5cde4942b652e914b465e14bc34f4a9dac54d3d7864faa1d'), ('gamecore', 33432624, 5, 'GameCore::Map::Plot::Instance::GetImprovementType() const', '2e364ec2b0b375cab6dbdd31e0d641f899ed78ab8c1e4c59df24ea436add56f4'), ('gamecore', 33432640, 6, 'GameCore::Map::Plot::Instance::IsImprovementPillaged() const', 'af700d60344727ffe670fa9f8aa5e8b02a6fdbe9243c45d2428feced003839cc'), ('gamecore', 34485664, 5, 'GameCore::Map::Plot::Instance::GetTerrainType() const', 'aa944c3c94a1601bb7dcb02a65bb632b6eea536336fcc9795e16687e6337491c'), ('gamecore', 34927504, 8, 'Game.EditMap', '60f333d9f21930ce93b03ef00947904c289d6d58010da000d11efa92c55ef289'), ('gamecore', 35536624, 11, 'Globals.EditGame', '1a62cdedb5d67a1a402ce77ae33f01e69b2789fb3e2d562419b6c8e61e993fcf'), ('gamecore', 36866128, 294, 'GameEffects::PlotYields::RemoveAllPlotYields()', '72b9ae80afd69757b41c0926efeea21f055aba34578cf2a86700833447cb7ca5'), ('gamecore', 36868544, 4, 'GameEffects::Instance::GetModifierSystem()', 'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'), ('gamecore', 36868800, 8, 'GameEffects.GetPlotYields', 'd28a535aeaeacb75ccc1ccd18583a7044c37f5b3c925356bbe81b9e3e0ad8fc0'), ('gamecore', 36900816, 124, 'GameEffects::ModifierSystem::ClearModifiers(std::__1::shared_ptr<GameEffects::IModifierObject> const&)', '127ed043b33349b5de0e4892529ec6769b00b172ca17198a1cb5e9919f011ae0'), ('gamecore', 36902240, 44, 'ModifierSystem.ObjectDestroyed', '2fe04653c2158ccaf7b655b6d03a3a4aa5ccbed6680356294312c637bfa45dda'), ('gamecore', 36919616, 4, 'GameEffects::Objects::ModifierObject::GetInstanceId() const', '580f63e5ed34152c56532cbfda2c4a0f74402790e9e026cd4888e06541f671e1'), ('gamecore', 36919632, 4, 'GameEffects::Objects::ModifierObject::SetInstanceId(int)', '16d722de6e6e0c0b5b7328dcabe0f5ecf0e1d191ce89414edf847c761c487641'), ('gamecore', 37674288, 227, 'Objects.PlotYields.C1', 'a4258beac250738ef36023b5a35919f99966a8ebaac2c725ad23a1cb6eab2ec1'), ('gamecore', 37689232, 120, 'PlotYields.C1', 'd7baaf33aaf80c9d48ff52e726add42b05802e5f97d4693d37805e502bc132a5'), ('gamecore', 37689408, 294, 'PlotYields.AddPlotYields', 'f7ecb549065bc474311930db7f0ae3ab089f81b45ebd20012e4f85139b4f1098'), ('gamecore', 37689712, 460, 'PlotYields.RemovePlotYields', '24304515b2bf4651707487e391aad831bd3ab09519e2b1cf6781405361490fef'), ('gamecore', 37690400, 86, 'PlotYields.GetPlotYields', '68d2756caf981354ebc28de793565dea8c9dda2e74f3a0c84e493aeb3e684fcc'), ('gamecore', 37690496, 26, 'PlotYieldData.C1', 'ccc498c9aad09347892f97d6e1d2e43e7bc213868889d7d50cd22f04ea76328d'), ('gamecore', 37863136, 143, 'ObjectAdded.Dispatch', '1a0447f6d5138bca73e7aa6c925611d30c9148b89f300c66e01b1c38bde066cb'), ('gamecore', 37948048, 8, 'GameEffects::Objects::ModifierObject::GetOnDestroyed()', 'ed09e751585a6947774095e65b448b6761b20980482541ff17ee77ae6df0bcd8'), ('gamecore', 38000496, 9, 'PlotYieldData.D2', '6f6d14ce929ee2b616e2ffcad9ccf287b08d2b44ea9ea2180d4149c3a73dbd9a'), ('gamecore', 40246576, 8, 'Game.GetGameEffects', '69ea70922a14067d1b731c0c3e7b6b4b96c12c568ebda1c4352f4685b119dacc'), ('gamecore', 41697680, 42, 'CoreComponentContainer<GameCore::Map::Component::IInstance>::EditComponentByIndex(unsigned int)', 'e864d685fed8e09fe6089b219d16d885221ad68c732a5c70a0d3a9d2167fac11'), ('gamecore', 42354128, 132, 'GameCore::City::Buildings::GetBuildingsAtLocation(int, Types::Vector<GameCore::BuildingTypes, Types::LinearAlloc::DefaultInline<GameCore::BuildingTypes, 3u, 1u, 0u> >&) const', '890c75f51439b3c5a167a34c3f48f7f09a5e0a7e65dd4d86f5855a4b0e3581b2'), ('gamecore', 42512480, 484, 'GameCore::City::Growth::HasDroughtPrevention() const', '9904e60d6e47e4511fd118a69737b0b45bf6d367226d556fa900e018649e9eca'), ('gamecore', 43038896, 16, 'GameCore::Context::Globals::EditMap()', '38f37272316971d72466ae3b66de873b38ddf16786c870218bef53a493b7e77e'), ('gamecore', 44296144, 17, 'GameCore::Map::Improvement::Instance::GetProperties() const', '28f33affda3b468ee1e695f5ffa7c255f77cc47b5147c2036bd536fad5207c19'), ('gamecore', 44297808, 757, 'GameCore::Map::Improvement::Yields::GetPlayerYieldFrom(GameCore::PlayerTypes, GameCore::ImprovementTypes, GameCore::YieldTypes, GameCore::Map::Plot::Instance const*, std::__1::list<GameCore::Map::Improvement::Yields::YieldContributions, std::__1::allocator<GameCore::Map::Improvement::Yields::YieldContributions> >*) const', 'f75e801dbf601be97571a61978777c845cc21da43ae840992265ca1c3443e6b1'), ('gamecore', 44298576, 2147, 'GameCore::Map::Improvement::Yields::GetPlayerYieldFrom(GameCore::PlayerTypes, GameCore::Definition::Adjacency_YieldChange const*, GameCore::YieldTypes, GameCore::Map::Plot::Instance const*, Types::Vector<GameCore::AI::AiMapChange, Types::LinearAlloc::Default<1u, 0u> > const*, std::__1::vector<int, std::__1::allocator<int> >*) const', '2bc83c3fa99ebe760d7076f6fa00ed2fc13baa475b5d7c8f6e7862b675315af1'), ('gamecore', 44301408, 199, 'GameCore::Map::Improvement::Yields::GetPlayerBonusYieldFromModifiers(GameCore::ImprovementTypes, GameCore::Map::Plot::Instance const*, GameCore::YieldTypes) const', '148be4cc89bf42514a44278586a21a733c2c2d55ccc6503b29131e141e6265ef'), ('gamecore', 44301616, 267, 'GameCore::Map::Improvement::Yields::GetPlayerBonusTechYieldFrom(GameCore::PlayerTypes, GameCore::ImprovementTypes, GameCore::YieldTypes) const', '4e88ea903a10e995089047c15fe4c64838ad782be69733321544cfcd54a18f98'), ('gamecore', 44301888, 267, 'GameCore::Map::Improvement::Yields::GetPlayerBonusCivicYieldFrom(GameCore::PlayerTypes, GameCore::ImprovementTypes, GameCore::YieldTypes) const', '7fcdc9e73aac2cd148e91df4a58569b4cdca56063b27b3fa6a958617de44b9c1'), ('gamecore', 44308624, 110, 'GameCore::Map::CityPlots::GetPlotYieldData(int) const', '5f3edd1380e04731affd03a1c7c8bfb39f4bd3264f100c0d2855ee7921fe1d2b'), ('gamecore', 44311808, 50, 'GameCore::Map::CityPlots::GetAttributes(unsigned int)', 'd9b066e28f35789167afe32cdf72cfc7287e0678e5b4f1f5324427a3260b862d'), ('gamecore', 44332400, 8, 'GameCore::Map::EditInstance()', '9ed2b7361cfabb71d887c7009c92d3a3f1e43aae2382c53c7e42fd534467081d'), ('gamecore', 44338080, 169, 'GameCore::Map::Instance::GetPlot(GameCore::PlotCoord const&) const', '44dca219db554b1884a343cdbdfe4e5f1ef3ca7695c848cb1e0fe72b933ad4ec'), ('gamecore', 45166816, 208, 'GameCore::Player::GreatPeoplePoints::CalculateYieldChangePerEarnedGreatPersonClassOnResource(GameCore::YieldTypes, GameCore::ResourceTypes) const', 'b0cf65dd52c74280bfef2935186c00bc39d7ccfb63543e0514777ab4efc5f772'), ('gamecore', 45614192, 461, 'GameCore::Player::Stats::GetNumDifferentTerrainTypesForCities() const', 'b3bf5aecb462d09cd78db1e6131af8277b9dbf175760899c9716ddb11ca88555'), ('gamecore', 45911424, 6, 'GameCore::Map::Plot::Instance::IsFinalKindOf(unsigned int) const', '1841c462c002d4015f18698e6f708f340efad3bbe9db6d6aae6823b5f7bd6793'), ('gamecore', 45913680, 52, 'Plot.GetYield.forward', '04b9421afd53b980c258b7a2fe67d09281b778fd591c9a26822f5d57017c80e6'), ('gamecore', 45913792, 37, 'GameCore::Map::Plot::Instance::GetAppeal() const', '20737c86df0be1bd3e5d522749073fefd0fe53a79a244ec0ae79e16b89afed0f'), ('gamecore', 45918080, 12, 'GameCore::Map::Plot::Instance::GetCityAttributes() const', 'e1a598f85f49a28e29144ddf86739e2601b0294590320845586249ce08532b8a'), ('gamecore', 45918192, 19, 'Plot.GetDynamicProperties', 'c15288f31fc8f4fec71681360c57dfd1e606001b8613d3625d9f08cc1f4929e6'), ('gamecore', 45918224, 52, 'Plot.GetDynamicProperties.kind', 'd449003c2237ab81da7dc9508ed69c88ea1b18815acb8dbdc0d1c4c02bcc1947'), ('gamecore', 45954496, 514, 'GameCore::Rules::Appeal::Instance::GetAppeal(GameCore::Map::Plot::Instance const*)', '577ae75b609362399bc6d1c17b7b849862743c0054037354d720a933c766fcd7'), ('gamecore', 45955760, 556, 'Appeal.local_accumulator_callback', '6752ef37700063eeec7a0d52f149144787cfa1d61438089b95f815a060f84919'), ('gamecore', 46095056, 3498, 'Rules.Yields.GetYield', '5249f66e85d4f15aaac4e11747aa594dce491cdf8d30e9dc8301aac63606c293'), ('gamecore', 46556656, 333, 'YieldMonitor.CheckPlot', '0f1a191dda92973708f6ec1de0ec41dbb829c192651d4d84aca1310738fb2dd2'), ('gamecore', 48048816, 26, 'GameCore::Map::Feature::Manager::Edit()', '8b45c9c83922809241de3bdfcb839bf7d157e3571ca278aaa1300d6a277a4323'), ('host', 60158896, 90, 'TypedVariantMap.FindVariant.const', '2788514beee59158d6cd2b816679fb774a19df417cc647a83564e156c77f6b36'), ('host', 60159104, 95, 'TypedVariantMap.GetVariant.const', 'b264f2c22424685f2c0d51945a672854768f0363768ac961e0bd0de247274118'), ('gamecore', 44340928, 14, 'Map.PlotVector.index.const', 'e43872283b0a27df38ac1c395d6da3b33206096b605d8374a9d57f3f39752492'), ('gamecore', 33432096, 8, 'Game.GetMap', '60f333d9f21930ce93b03ef00947904c289d6d58010da000d11efa92c55ef289')]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    core = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    for image, address, size, name, expected in METHODS:
        elf = core if image == 'gamecore' else host
        if hashlib.sha256(elf.read(address, size)).hexdigest() != expected:
            raise ValueError('original yield method differs: ' + name)
    calls = [(0x2c6668a,0x1e71110,'monitor calls Plot.GetYield'),
             (0x2bf6126,0x1e4c6a0,'rules calls improvement GetYieldFrom'),
             (0x2a3f0ab,0x1eedd50,'nonnull contribution branch calls temporary modifier calculation'),
             (0x2a3fcd5,0x1f72a90,'temporary calculation adds live PlotYields object'),
             (0x2a3fd10,0x1eea0e0,'temporary calculation removes object'),
             (0x23f191b,0x1fa8c90,'Add synchronously dispatches collection signal'),
             (0x23f1b09,0x1fa6d00,'Remove invokes ModifierSystem.ObjectDestroyed'),
             (0x233156d,0x1ebbf80,'ObjectDestroyed clears modifiers'),
             (0x2331015,0x1dff3f0,'ClearModifiers clears each actual modifier ID')]
    for address, target, _ in calls:
        core.call(address, target)
    instructions = [(0x2c66680,'31c9','monitor passes boolean false'),
                    (0x2bf6118,'4531c0','GetYield passes NULL contribution pointer R8'),
                    (0x2a3ed98,'4d89c6','wrapper retains R8 contribution pointer'),
                    (0x2a3edd3,'4d89f1','wrapper forwards contribution pointer in R9'),
                    (0x2a3ee5e,'4c894c2418','GetPlayerYieldFrom saves R9 at stack+18'),
                    (0x2a3f08d,'48837c241800','test actual contribution pointer'),
                    (0x2a3f097,'7424','NULL skips modifier/Add/Remove branch to2a3f0bd'),
                    (0x23f179b,'c7430800000000','PlotYields constructor zeros counter'),
                    (0x23f189d,'8b730883c601897308','Add increments live uint32 counter'),
                    (0x2335950,'897718c3','object SetInstanceId writes actual ID'),
                    (0x2430a90,'488d87e0000000c3','OnDestroyed returns object signal+e0'),
                    (0x1fe2220,'488b8750020000c3','actual game map pointer+250'),
                    (0x2661d30,'488b8768020000c3','actual game GameEffects pointer+268'),
                    (0x23292c0,'488b87a8000000c3','actual GameEffects PlotYields pointer+a8')]
    for address, expected, name in instructions:
        if core.read(address,len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('original yield instruction contract differs: ' + name)
    slots = [(0x3ae2390,0x18,0x2bc8d80,'live plot final-kind predicate'),
             (0x3ae2390,0x30,0x2bc96c0,'live plot appeal'),
             (0x3ae2390,0x38,0x2bca780,'live plot city attributes'),
             (0x3ae2390,0x50,0x2bca7f0,'existing/empty property map'),
             (0x3ae2390,0x58,0x2bca810,'kind property lookup'),
             (0x3ad5350,0x78,0x2a48ba0,'live map GetPlot'),
             (0x3a37150,0x30,0x2335950,'temporary object SetInstanceId'),
             (0x3a37150,0xc8,0x2430a90,'temporary object destruction signal')]
    for table, slot, target, name in slots:
        if core.relocated_pointer(table+16+slot) != target:
            raise ValueError('original virtual binding differs: ' + name)
    for slot,target in [(0x60,0x395f3b0),(0x70,0x395f480)]:
        if int.from_bytes(host.read(0x53b3680+16+slot,8),'little') != target:
            raise ValueError('standard TypedVariantMap const lookup binding differs')
    result = dict(schema='c6-linux-reporting-yield-effects-v1',checks_passed=True,
        host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
        methods=[dict(image=i,address=hex(a),bytes=z,name=n,sha256=h) for i,a,z,n,h in METHODS],
        calls=[dict(address=hex(a),target=hex(t),contract=n) for a,t,n in calls],
        instruction_contracts=[dict(address=hex(a),bytes=b,contract=n) for a,b,n in instructions],
        virtual_bindings=[dict(table=hex(t),slot=hex(s),target=hex(a),contract=n) for t,s,a,n in slots],
        abi='GetYield int32 EAX; receiver RDI, const Plot* RSI, YieldTypes EDX, fallback PlayerTypes ECX, bool R8b; no hidden return',
        actual_monitor=dict(active_humans=1,plot_cache_entries=6996,trade_cache_entries=182,
            evidence='evidence/linux/reporting-provider-focused-2.json',
            argument_domain='yield index starts0 and increments to NUM_YIELD_TYPES; boolean false; original current local observer',
            direct_improvement_contribution_pointer=0,direct_temporary_plot_yields_path_excluded=True,
            per_plot_branch_outcomes_observed=False),
        read_groups=['definitions and existing scalar/array fields',
                     'Feature.Manager.Edit -> Globals.EditMap -> existing component lookup; no lazy component creation in inspected chain',
                     'Stats distinct terrain count mutates a local bool vector, not the Stats receiver',
                     'building enumeration clears/appends caller-owned temporary vector',
                     'owned appeal reads signed16 plot+4a; unowned appeal computes through captured local accumulator',
                     'live plot property accessor returns existing map or static empty map; standard native map lookup is const find/default, not insertion'],
        general_temporary_effect=dict(reachable_only_with_nonnull_contribution_list=True,
            system_bytes=88,counter_offset=8,counter_type='u32',counter_wrap='native 32-bit add',
            added_signal_offset=16,id_vector_offset=40,id_element_bytes=4,object_vector_offset=64,object_element_bytes=16,
            object_bytes=344,object_instance_id_offset=24,object_destroyed_signal_offset=224,
            sequence=['allocate original object','increment counter','assign instance ID','append ID/object vectors','synchronous collection signal','read calculated yield','erase ID/object entries','ClearModifiers and OnDestroyed dispatch'],
            remove_restores_counter=False,remove_all_resets_counter=False,
            all_counter_writers_proven=False,counter_serialization_or_sync_proven=False,
            serialization_classification='unproven; persistent live counter alone does not establish gameplay authority',
            surviving_id_consumers_after_remove='unproven; actual modifier/signal registrations and cleanup effects require evidence'),
        unresolved_actual_domains=[
            dict(domain='property submaps',boundary='GetYield0x2bf61de/0x2bf620a virtual+60; actual nested map type/vtable not captured',condition='valid improvement and relevant native property exists',minimal_observation='at existing monitor stop, bounded live plot/property-map pointer/vtable attestation; nested subtype remains unproven without provenance'),
            dict(domain='improvement adjacency/bonus',boundary='0x2a3f150 and tech/civic getters; exact rules values/remaining getter closure not implemented',condition='actual improvement definitions/references match requested yield',minimal_observation='current plot improvement/owner/definition and exact provider binding; no generic AI namespace exclusion'),
            dict(domain='unowned appeal',boundary='live GetAppeal0x2bc96c0 -> Rules.Appeal0x2bd35c0/local callback0x2bd3ab0',condition='appeal-sensitive yield and actual plot owner=-1',minimal_observation='existing monitor plot owner/definition fields or bounded branch attestation; no new run by this audit')],
        typed_boundary=dict(return_value='native i32 integer yield; preserve exact ordering/wrapping/definitions and native FixedPoint conversions',
            supported_read_environment='only identified actual plot/map/property bindings and modeled rules inputs',
            monitor_effects='outside GetYield: compare/update actual receiver-owned IndexLists and report changed plot/aggregate yields in native order',
            unsupported_environment='reject before mutation until relevant rules/provider dependencies are modeled',live_rust_action_admitted=False),
        rng_and_reporting=dict(no_direct_rng_or_reporting_call_in_getyield=True,
            complete_transitive_rng_or_callback_absence_proven=False,
            reason='general temporary object signals can invoke real modifiers; direct monitor route skips that chain, remaining recursive rule/property bindings not fully closed'),
        native_execution=False,private_evidence_local_only=True,
        full_transitive_effects_proven=False,gameplay_parity=False,
        raw_save_load_failure_bytes_unclassified=89198,
        limits='Static target-version evidence only. Do not infer exercised branches from nonempty monitor or unchanged Save bytes. No unknown bytes masked; no full-host, complete live state/RNG, Rust action or turn acceptance.')
    result['source_audit_sha256']=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods=len(METHODS),monitor_direct_add_remove_excluded=True,native_execution=False)))

if __name__ == '__main__':
    main()
