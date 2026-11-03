#!/usr/bin/env python3
"""Bound post-Monitor lifecycle effects; never run original code or infer input from output."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
GUARD_REPORT_SHA = 'df8906139b14fab957211ddb9e92478e31d5d07b2489b3451df71b3f974fcdf4'
METHODS = [('GameCore::AI::GlobalAi::InitializePostLoad()',
  '0x20fafb0',
  1,
  'ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e'),
 ('GameCore::Game::HeroesManager::InitializePostLoad()',
  '0x2655ed0',
  14,
  '616dd12ba4d7e62d6963ffdcadb7411a89a444ea080296e99285551e8d2c318d'),
 ('GameCore::Rules::Manager::InitializePostLoad()',
  '0x2bec370',
  63,
  '23059d8c3b020752e9f2e030aff9910b3e545eb1377d94fa2ed8ac452b8ebf2e'),
 ('GameCore::Analysis::Manager::InitializePostLoad()',
  '0x21e1a90',
  22,
  'e696775f8fa0b3ee7d84314fbcdefd0b9f60a8718fd4e4d72de0df74fd42f5d7'),
 ('GameCore::Barbarian::Manager::InitializePostLoad()',
  '0x282e920',
  37,
  '8378d7192b858ece49193008a07501d7a6e51e7a06060d312140acc39813fac9'),
 ('GameCore::Game::HeroesManager::RebuildValidHeroClasses()',
  '0x2655ee0',
  219,
  '60b2d8f1ecb94b80636b2d2ffd8d4929ab1535b8b99f496d8219d15a0c3c9342'),
 ('GameCore::Game::HeroesManager::ValidatePlayerHeroDiscoveryOrders(bool)',
  '0x2655fc0',
  202,
  '08d610194b9cea3f7852ce3f3655e6fd1c9439685f7c8a86f35d89324f4c09f1'),
 ('GameCore::Analysis::Technology::Initialize()',
  '0x21e5250',
  5,
  'b19a3410ba5885939ca53bc1420acc8b1d5d61a54c1b950bce5ff4e6c62cd034'),
 ('GameCore::Analysis::Civic::Initialize()',
  '0x21df280',
  5,
  'ad0e5fb785f9746f75b589e13e20f7adc656f9c27699772e6b13e18f339dbced'),
 ('GameCore::AI::Diplomatic::InitializePostLoad()',
  '0x2063c40',
  73,
  'e661768b129724ba76241d8cfd6c1aac300d5f037f811f9ee7add6f467a8e0d6'),
 ('GameCore::AI::BehaviorTree::Manager::InitializePostLoad()',
  '0x26e4910',
  127,
  'c520887cbe3262665e235ae5ba18c34f5921f7f87ab9f1ecd15289e46f664192'),
 ('GameCore::AI::GrandStrategic::InitializePostLoad()',
  '0x21214c0',
  167,
  '1b9db43baf60b4730ebcd565ca2fd3cac81354ba01c346e7fbe7d4faefbfa903'),
 ('GameCore::AI::Control::Manager::InitializePostLoad()',
  '0x201dde0',
  1578,
  '8ae49aadbc55dae247f96abca6fbad9ea71ff935e892bc0770d7f8b8a88b8a35'),
 ('GameCore::AI::Targets::InitializePostLoad()',
  '0x21a9150',
  1,
  'ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e'),
 ('GameCore::AI::Economic::InitializePostLoad()',
  '0x20a4280',
  73,
  '3cb5535f249a9b5b42568ab86dfd4964cb78ab7460e7ddbcb8792bfad17aef9d'),
 ('GameCore::AI::Research::InitializePostLoad()',
  '0x21871b0',
  73,
  'c4114843a187be48614c592b81aa5d87aa3a2406ce8c2473bed97617ca0f21b3'),
 ('GameCore::AI::CultureAi::InitializePostLoad()',
  '0x20310b0',
  73,
  '6ecf913c07e561c5f0302cf0c969d460af388df2fb2d185fbfc434c18718bb50'),
 ('GameCore::AI::Religious::InitializePostLoad()',
  '0x2173190',
  73,
  'c3f63a1779872e287ac376fdb4eb8c915623995cec227f3c179012eaa0bd45c7'),
 ('GameCore::City::Citizens::InitializePostLoad()',
  '0x286eb60',
  91,
  '288fd49f8126cb108ca6b3fdce801a66b619746f07f04f474912e1f8567abb85'),
 ('GameCore::City::Buildings::InitializePostLoad()',
  '0x2863db0',
  161,
  'b00c65c6465714acddc7009a2937be4851975be693fb42764b4e93a1f7921ac8'),
 ('GameCore::AI::BehaviorTree::BehaviorTree::InitializePostLoad()',
  '0x26b63a0',
  73,
  '23d1d6aff2b974b14eacd94149a040d1888c062f911c1fc76138405a7b784da3'),
 ('GameCore::AI::Control::Handle::InitializePostLoad()',
  '0x20184a0',
  18,
  '2d33d1903931f8d962b8da4018fe566dde47a88bff968c40f0e38136a4e1e325'),
 ('GameCore::AI::Control::Handle::Subscribe(GameCore::PlayerTypes, unsigned int)',
  '0x20184c0',
  80,
  '410a4d0f70fa4310afc040a5b189ca0947e1601da6308988c56d0e6945593c30'),
 ('GameCore::AI::Control::Handle::Release()',
  '0x2018310',
  170,
  '6ca961fe8277279e04b1f09aa55b930bd4e82d85450d2e5facc0205d1c290405'),
 ('GameCore::AI::Control::Handle::Subscribe(GameCore::AI::Control::Contract*)',
  '0x2018430',
  63,
  '66fb46371c0309415bf1b8acc12a4df842ddee13f438bf1281308aa3f3c3d24b'),
 ('GameCore::AI::Control::Manager::FindContract(unsigned int)',
  '0x201f1e0',
  110,
  'd60ca0dc5373a2a53f313865788d3092246ae0f1d9492c94226dbfa80627de09'),
 ('Player.InitializePostLoad',
  '0x2b32be0',
  510,
  'a167cb0aa1be6d40fbf4966833bfa893ad5ce35b2c949cac83be78ddac9c3dbe'),
 ('Rules.Placement.InitializePostLoad',
  '0x2bef1b0',
  375,
  'c8e6b94b2ac1f32e6c43da21ce51c336f19af63199379edd5921cd5d40d6014b'),
 ('Unit.IsDead', '0x20a8dc0', 33, '2f2339e4ee263607695aa20d36ce5edb7a10caafe1329e8ac667c86fc107b007'),
 ('Hero.DiscoveryOrder.Rebuild',
  '0x269c6f0',
  475,
  '418fa74f4d229ed5d6940b0b823ebb188a5228b1a6ce11b1e252d33565f8d0ab')]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit(args):
    image = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for name, address, size, expected in METHODS:
        if hashlib.sha256(image.read(int(address, 16), size)).hexdigest() != expected:
            raise ValueError('postload body differs: ' + name)
    calls = [
        (0x2d7b264, 0x1f83730, 'Game.Read initializes YieldMonitor'),
        (0x2d7b2d3, 0x1e80730, 'later Player.InitializePostLoad'),
        (0x2b32c06, 0x1ec9f80, 'BehaviorTree.Manager.InitializePostLoad'),
        (0x2b32c76, 0x1e493b0, 'Control.Manager.InitializePostLoad'),
        (0x2b32d1d, 0x1f37c90, 'conditional missing Unit sight population'),
        (0x2b32da6, 0x1e97840, 'City.Buildings.InitializePostLoad'),
        (0x2b32db6, 0x1f0f520, 'City.Citizens.InitializePostLoad'),
        (0x286eba5, 0x1fa4df0, 'conditional Rules.Appeal.Update on each city plot'),
        (0x269c815, 0x1fb94d0, 'conditional gameplay RNG in Hero discovery rebuild'),
    ]
    for address, target, _ in calls:
        image.call(address, target)
    for binding in CONTRACT_BINDINGS:
        target = image.relocated_pointer(int(binding['table'], 16) + 16 + int(binding['slot'], 16))
        if target != int(binding['target'], 16):
            raise ValueError('stock contract postload vtable differs: ' + binding['name'])
    if image.read(0x286eb67, 5).hex() != '41807e7000':
        raise ValueError('Citizens force_update_plot_appeal guard differs')
    if image.read(0x201e38b, 3).hex() != 'ff5010':
        raise ValueError('actual Control contract virtual postload differs')
    if sha(args.guard_report) != GUARD_REPORT_SHA:
        raise ValueError('requires the retained a2a8d1a actual saved-guard fixture')
    guards = json.loads(args.guard_report.read_text())
    if guards['source_revision'] != 'a2a8d1a18b677cd8ee905462543b7c8bad94be25':
        raise ValueError('saved-guard fixture source revision differs')
    return dict(schema='c6-linux-yield-monitor-postload-history-v2', checks_passed=True,
                original_code_executed=False, gamecore_sha256=GAME_SHA,
                audit_sha256=sha(Path(__file__)),
                methods=[dict(name=name, rva=address, bytes=size, sha256=digest)
                         for name, address, size, digest in METHODS],
                calls=[dict(rva=hex(address), target=hex(target), contract=name)
                       for address, target, name in calls],
                stock_contract_bindings=CONTRACT_BINDINGS,
                saved_output_guards=dict(report_sha256=GUARD_REPORT_SHA,
                    source_revision=guards['source_revision'],
                    candidate_sha256=guards['candidate_sha256'],
                    retained_source=guards['retained_source'],
                    values=guards['postload_guard_inputs'],
                    original_raw_input_branch_outcomes_proven=False),
                effects=dict(
                    global_ai_postload='Single RET; not a namespace exclusion.',
                    behavior_tree='Visits existing trees and rebinds their Control handles; concrete handle initialization skips when pointer already bound.',
                    handle='Unbound postload subscription resolves saved owner/id, releases a NULL pointer and attaches to existing contract. Wider contract lifecycle is separate.',
                    control_manager='Registers priority callback functions, then invokes existing Contract virtual+0x10 at201e38b. Six stock tables bind five classes to RET and UnitRequestContract to the concrete creation-signal subscription path.',
                    unit_request_contract='Guarded postload creates building/unit/district-created function objects and subscribes them if the corresponding stored handle is empty. Concrete Subscribe/EnsureRing/AddBefore allocate/link callback nodes and copy function objects; they do not dispatch the registered creation callback. Callback registrations remain required for future genuine events.',
                    city_buildings='Clears a GreatWorkSlot ID only when nonmissing slot indexes a Game.Culture GreatWork whose type is -1; retained slot identity can change.',
                    city_citizens='If force_update_plot_appeal true, clear it and invoke Rules.Appeal.Update on every nonnull city plot. Cached Plot appeal is a direct query input.',
                    unit_sight='Dead/invalid-location/nonempty-sight guards skip population.21 empty saved-output vectors do not prove21 reachable repairs.',
                    heroes='False force rebuild still rebuilds empty player discovery orders; actual allowed/eligible-class gates precede Game.GetRandNum. Separate owner task retains this gameplay RNG obligation.'),
                remaining=[
                    'Original a667 Read-input City.Citizens flags and guarded Unit sight fields; ff31 saved output cannot establish pre-clear values.',
                    'Actual runtime contract/callback admission must match the six source-checked stock vtables; arbitrary additional subclasses remain unsupported.',
                    'Known current definition bindings and source guard outcomes for Tech/Culture/Hero postloads; nonempty saved vectors alone are not the Read projection.',
                    'Any later relevant Rules/Barbarian/City destruction/Processor/idle continuation writes or callbacks.'],
                cache_freshness_history_closed=False,
                monitor_omission_authorized=False,
                full_affected_state_rng_or_action_parity=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--guard-report', type=Path, default=ROOT / 'evidence/linux/yield-monitor-postload-domain.json')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS),
                         saved_output_guards=result['saved_output_guards']['values'],
                         monitor_omission_authorized=False)))


METHODS += [('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::BuildingTypes, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::Subscribe(std::__1::function<void '
  '(GameCore::BuildingTypes, unsigned int)>)',
  '0x200d9d0',
  61,
  '431e1064435a32d0934a9a010fbd904dd2ac738414a5bdd42d09531b8672f747'),
 ('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::DistrictTypes, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::Subscribe(std::__1::function<void '
  '(GameCore::DistrictTypes, unsigned int)>)',
  '0x200db30',
  61,
  'bd130c1889a6b9be5c40be35cf676c57fff542525ef5ff70996c1b90040248d9'),
 ('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::Unit::Instance*, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::Subscribe(std::__1::function<void '
  '(GameCore::Unit::Instance*, unsigned int)>)',
  '0x200da80',
  61,
  'b84c9f823bbc0f6eda30a4b65b04b8c400e712db439fdf52c9ececfffff28504'),
 ('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::BuildingTypes, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::EnsureRing()',
  '0x20148c0',
  179,
  '4288aeaf20c760e98593b97e15c0827a96fdd93a76ab63c92f3c04d10358d83a'),
 ('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::BuildingTypes, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::SignalLink::AddBefore(std::__1::function<void '
  '(GameCore::BuildingTypes, unsigned int)> const&)',
  '0x2014980',
  149,
  'a7946a8b8beeac15ef1eba4e3d369264989ab4280b4ef5b07d6a5c255b7b4068'),
 ('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::DistrictTypes, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::EnsureRing()',
  '0x2014f20',
  179,
  'c9692c0f822917463927353abc8e7ba3495c05f1c9bdd88caf2827d4358f74a1'),
 ('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::DistrictTypes, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::SignalLink::AddBefore(std::__1::function<void '
  '(GameCore::DistrictTypes, unsigned int)> const&)',
  '0x2014fe0',
  149,
  '7974ca1ea4fbfa79f6081d98cfe2a7178450bc857ebf15198d5dbdc10417df38'),
 ('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::Unit::Instance*, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::EnsureRing()',
  '0x2014bf0',
  179,
  'c5750fec472ec5f65c976a72875536c984f45ac0700a0fd6cbdd596fe5539227'),
 ('GameCore::Message::SignalBase::ProtoSignal<void (GameCore::Unit::Instance*, unsigned int), '
  'GameCore::Message::SignalBase::CollectorDefault<void> >::SignalLink::AddBefore(std::__1::function<void '
  '(GameCore::Unit::Instance*, unsigned int)> const&)',
  '0x2014cb0',
  149,
  '8bc0d4cca77d6525a2c98fce0133789c269dd18b4644a83578d2ec6bd7f27e60'),
 ('Stock.UnitRequestContract.InitializePostLoad',
  '0x2009d10',
  595,
  'a5a1153252ce4d50ce1c144ee2f0ec947f93ad1a4ece8497d34e719ececf4450'),
 ('Stock.Contract.InitializePostLoad',
  '0x200dbd0',
  1,
  'ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e')]
CONTRACT_BINDINGS = [{'name': 'vtable for GameCore::AI::Control::UnitRequestContract',
  'slot': '0x10',
  'table': '0x3a04e40',
  'target': '0x2009d10'},
 {'name': 'vtable for GameCore::AI::Control::PurchasePlotContract',
  'slot': '0x10',
  'table': '0x3a050c8',
  'target': '0x200dbd0'},
 {'name': 'vtable for GameCore::AI::Control::TechResearchContract',
  'slot': '0x10',
  'table': '0x3a04f18',
  'target': '0x200dbd0'},
 {'name': 'vtable for GameCore::AI::Control::CivicResearchContract',
  'slot': '0x10',
  'table': '0x3a04ff0',
  'target': '0x200dbd0'},
 {'name': 'vtable for GameCore::AI::Control::DiplomaticActionContract',
  'slot': '0x10',
  'table': '0x3a051a0',
  'target': '0x200dbd0'},
 {'name': 'vtable for GameCore::AI::Control::Contract',
  'slot': '0x10',
  'table': '0x3a05ee0',
  'target': '0x200dbd0'}]

if __name__ == '__main__':
    main()
