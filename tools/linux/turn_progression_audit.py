#!/usr/bin/env python3
"""Exact-target static top-level turn contract; never launches original game code."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA

# Frozen original identities are inserted below; names are original ELF symbols.
METHODS = [(46650608,
  3528,
  'GameCore::Exports::Context::RequestAction(GameCore::ActionTypes, GameCore::PlayerTypes, '
  'Data::TypedVariantVectorMap const&)',
  '8d470698084c8e3d2d2e21137f2c16c717f3a9e7f816306f47be3575aa085319'),
 (47348480,
  59,
  'GameCore::Messages::PlayerTurnComplete::PlayerTurnComplete(GameCore::PlayerTypes, '
  'GameCore::Events::TurnRequest::RequestSubTypes)',
  '0e698736920e9586a00af92ae746ffcca314229b18423747ea4f01faf000ce48'),
 (47349760,
  41,
  'GameCore::Messages::PlayerTurnComplete::PutInBufferBody(Serialization::OStream&) const',
  '66c07ba2214ee0e4363af0e501e28dfade3712a05729c56d2132047628db747c'),
 (47349808,
  41,
  'GameCore::Messages::PlayerTurnComplete::SetFromBufferBody(Serialization::IStream&)',
  'b86fc1fdd8106bc8e5568e7335f572ec3c092629fed329678acbc332094d878a'),
 (47348560,
  1190,
  'GameCore::Messages::PlayerTurnComplete::Execute() const',
  '695d8e7b98876bc8582a8778c2fe4372fd5b0cca6420ba40041994121d9b84fa'),
 (47350064,
  3,
  'GameCore::Messages::PlayerTurnComplete::GetUnreadiesTurn() const',
  '251447ee91a9067dcd6ab96703133f617565974cd6c4819021760c4688c91abf'),
 (47329968,
  83,
  'GameCore::Messages::GameTurnComplete::Execute() const',
  '422fc46b74202df82aae00f8ccc46310510ee22a64a01718fa8a559d2d5fdee2'),
 (45463616,
  130,
  'GameCore::Player::Processor::SetTurnComplete(GameCore::Player::Instance&, bool, '
  'GameCore::Events::TurnRequest::RequestSubTypes)',
  'bf6e10c8a7a8fb2f77f4ca269a1b80a28aef56140b0c42eae3f8db1bd59bf6fc'),
 (45750192,
  22,
  'GameCore::Player::TurnManager::DoSetTurnComplete(GameCore::Events::TurnRequest::RequestSubTypes)',
  'adfce5a688d5d22da5aeb9fe66d7e4e7e2b0f22692e796e1a399da8a0f9b827f'),
 (44414528,
  51,
  'GameCore::Player::Cities::DoSetTurnComplete(GameCore::Events::TurnRequest::RequestSubTypes)',
  'd6ee12cd3bba79040e5914ca82a066475c9435a602168376a04fc2eda01052e6'),
 (44414320,
  195,
  'GameCore::Player::Cities::RemoveUnassignedProductionOverflow()',
  '4846d8d97a3029fa5cb57f36b1ce74e27ac5a2cdfb53ddbeda44e1b0c68ee151'),
 (42274752,
  14,
  'GameCore::City::BuildQueue::ClearOverflow()',
  '1ff881b344b791b64935ad8275bf140397c280331ab8112c93d32d332484a7af'),
 (40246176,
  17,
  'GameCore::Game::Instance::Update()',
  'e9d50e09f9c4183d872ce51adfb4b35db4aaad7ed99d28f150eaa004fe74e6cc'),
 (44090416,
  33,
  'GameCore::Game::Processor::IsNewTurn()',
  '42f989ce40a0ef442c09059b84fa6cc9a13f4862f4841cad4b50da1c2a7472eb'),
 (44092272,
  2326,
  'GameCore::Game::Processor::Update()',
  '3d0e408c00c96e44fb5ea509baedf9a0f690347b4696e469c37b0ac7f2b12055'),
 (44090464,
  746,
  'GameCore::Game::Processor::BeginTurn()',
  'babd3496e211c2cc8a2fa5cb011cec3553665156ac97c02504a4000198566ca6'),
 (44091216,
  634,
  'GameCore::Game::Processor::EndTurn()',
  '16949465303cae46f7b95966ee04510626b99a4dda30b3aed6d09e6245c3b477'),
 (44089072,
  147,
  'GameCore::Game::Processor::ActivatePlayersAtTurnStart()',
  '55231d33b54161fd4b63f8b2db0ff0947b0ff2172210ae4130e7832e069f1699'),
 (44089232,
  295,
  'GameCore::Game::Processor::ActivateNextPlayer()',
  'd402bd681e0a98d5dfbb0db1967e291fd97bb3e458a4c673e92ab7621b354d30'),
 (44089536,
  487,
  'GameCore::Game::Processor::GetPlayersToProcess(Types::Vector<GameCore::PlayerTypes, '
  'Types::LinearAlloc::DefaultInline<GameCore::PlayerTypes, 64u, 51u, 0u> >&)',
  'a8892c6ac0fe4a386b36aee656d88cbfb3459a389522b1d66129672be4f4b8b1'),
 (44090032,
  377,
  'GameCore::Game::Processor::GetPlayersToDeactivate(Types::Vector<GameCore::PlayerTypes, '
  'Types::LinearAlloc::DefaultInline<GameCore::PlayerTypes, 64u, 51u, 0u> >&)',
  '1f599a1174d3258cb284cf134bcd89b0445fca94aa2f910517b3e0edfa51cdf6'),
 (44091872,
  164,
  'GameCore::Game::Processor::BeginPhase(int)',
  '01feb862550a3eaa0fc0058fe00dd152114fda6cf8de71c3bb676291595e7bac'),
 (44092048,
  42,
  'GameCore::Game::Processor::EndPhase()',
  'c388e4013488828e1e222d675963d3ccda8ff5d17a1ea8c0719ea0fd8f5c3635'),
 (44092128,
  135,
  'GameCore::Game::Processor::DoAutomation()',
  'fc056c16075642035c5e8ede11006de889c7578864be6f51bcf4086fb6544d21'),
 (44386048,
  181,
  'GameCore::Phase::Processor::InitializeTurnPhaseData()',
  '23621eded7d30d7320575912b08ea3affa64b3112114a84eb7a5a24b24de7147'),
 (44386432,
  1683,
  'GameCore::Phase::Processor::ActivateNextTurnPhase(bool&)',
  '1ee231112b4658b5c4101d58a4ea9a7684a8a3a09b25f8b8b2fb470b72aabfa5'),
 (44386400,
  11,
  'GameCore::Phase::Processor::BeginTurn()',
  '5afa3064e44ca66f93685648c985e8268f461fa441e4dbb78aabccf8f70d8087'),
 (44386416,
  11,
  'GameCore::Phase::Processor::EndTurn()',
  '9bb4ab636d6a03f7d8cea744e08c3c008c8468dd7789e6900d4e385cbd5aa989'),
 (45459648,
  313,
  'GameCore::Player::Processor::BeginTurn()',
  'a8b2b54a5f93781cf44516618f9729bf83bdbe83c5f5f96311beee5a59d459ee'),
 (45459968,
  269,
  'GameCore::Player::Processor::EndTurn()',
  'e795ea50bdb401861f2afd557cca1ffa72f9bda6f71b9f6ed220c528d4a311de'),
 (45460240,
  196,
  'GameCore::Player::Processor::BeginPhase()',
  '6c5209859b7a1ad0b2586523d23f81c05543e2a3bcbfd57344bed8d3e148506f'),
 (45460480,
  163,
  'GameCore::Player::Processor::DoTurnActivate(GameCore::Player::Instance&)',
  'e40e22d51b9a2648f6f0c0f661211ca4966f667f4eae88154fc18dc6582e5bb9'),
 (45460768,
  156,
  'GameCore::Player::Processor::DoTurnDeactivate(GameCore::Player::Instance&)',
  '7f47b1f7c487e241b0372495ed223b1b856bf0beb80033a8022914054db8e2ce'),
 (45460656,
  109,
  'GameCore::Player::Processor::DoTurn(GameCore::Player::Instance&)',
  '241f765996df381cdbbf8d381721652b85d662f1c4c6e7af2421dee849999516'),
 (45467424,
  356,
  'GameCore::Player::Processor::Update(GameCore::Player::Instance&)',
  '8464aedee5e1a5fd83c6ce257d67ded9cbf3ed578a6f0f2c4341aa881f3125c0'),
 (45461920,
  1571,
  'GameCore::Player::Processor::DoAutomation(GameCore::Player::Instance&)',
  'ec31b9b95c27f31b5718b921688112e7320314e56440bce010892d4214ca1019'),
 (45460928,
  984,
  'GameCore::Player::Processor::SetTurnActive(GameCore::Player::Instance&, bool)',
  '3555d19911dd2669bf7ca9cd24ac141ebb79a2b211c48c78cccc04c0aac833f0'),
 (45750096,
  18,
  'GameCore::Player::TurnManager::DoBeginTurn()',
  '91f6f1f4adb290f340477c57e365c9336e57076d620d2c225705b44d23983fda'),
 (45747040,
  1356,
  'GameCore::Player::TurnManager::DoTurnActivate(bool)',
  'f76c17c4a7ad1b7ff87dc464d1d4df414f4a15ead689448ff4682d3236a7b33e'),
 (45750128,
  53,
  'GameCore::Player::TurnManager::DoEndTurn()',
  'fae38035b87e6b8a589ab1f276ce1d5420f21ed76ccd5d1c07ff2dc40aa1c699'),
 (47677344,
  196,
  'GameCore::XP2::Game::Instance::DoBeginTurn()',
  '6d6ee1acf44f7edc05c9c406455d8f38b8ea3bba2ef230cff73f0d65a234aec6'),
 (44335328,
  73,
  'GameCore::Map::Instance::DoTurn()',
  'aec1c05f2ba1c71357182e65c7270515994b3212850e9994b33ff9a0788c2102')]
METHODS += [(47545792, 49, 'GameCore::Utilities::GetNumberOrHash(Data::TypedVariant const*)', '1b4df6bf627231aee32e7fe143745be52e569fe61787a24c4b2c19c9cf051b33'), (47349904, 19, 'Serialization::EnumToStream<RequestSubTypes>', 'e3c81196bd5a80bf03aed9f9294eba27522677a483262ee8c38ed280791907dc'), (33566912, 19, 'Serialization::EnumToStream<PlayerTypes>', '53e00befd8655db0c427919039335f3534f6be53858eb37a84c25490081ba08b')]

CALLS = [(46650635, 32200208, 62380528, 46650400, 'GameCore::Exports::Context::IsWorldBuilderActive()'),
 (46650748, 31640864, 62100856, 43039264, 'GameCore::Context::Globals::GetLocalPlayer()'),
 (46652331, 32285792, 62423320, 47545792, 'GameCore::Utilities::GetNumberOrHash(Data::TypedVariant const*)'),
 (46652580,
  32972400,
  62766624,
  47348480,
  'GameCore::Messages::PlayerTurnComplete::PlayerTurnComplete(GameCore::PlayerTypes, '
  'GameCore::Events::TurnRequest::RequestSubTypes)'),
 (46652588,
  33265560,
  62004416,
  47332848,
  'GameCore::Messages::Manager::BroadcastMessage(GameCore::Messages::BaseMessage&)'),
 (47348610, 32012288, 62286568, 45290800, 'GameCore::Player::Instance::IsHuman() const'),
 (47348626, 31529456, 62045152, 42055456, 'GameCore::Game::Instance::IsPaused() const'),
 (47348987,
  32327648,
  62444248,
  45464416,
  'GameCore::Player::Processor::IsTurnComplete(GameCore::Player::Instance const&)'),
 (47349378,
  31856528,
  62208688,
  45463616,
  'GameCore::Player::Processor::SetTurnComplete(GameCore::Player::Instance&, bool, '
  'GameCore::Events::TurnRequest::RequestSubTypes)'),
 (47349391, 33097392, 62829120, 40246176, 'GameCore::Game::Instance::Update()'),
 (47349728,
  32457872,
  62509360,
  47488704,
  'GameCore::Reporting::Event::Send(GameCore::Reporting::Event::Type, int)'),
 (45463685, 32870944, 62715896, 33574848, 'GameCore::Player::Manager::IsValid(GameCore::PlayerTypes) const'),
 (45463697, 33252176, 62906512, 33569968, 'GameCore::Player::Instance::IsAlive() const'),
 (45463720,
  31601152,
  62081000,
  45750192,
  'GameCore::Player::TurnManager::DoSetTurnComplete(GameCore::Events::TurnRequest::RequestSubTypes)'),
 (47329972, 33144688, 62852768, 44094656, 'GameCore::Game::Processor::NeedsTurnAdvance()'),
 (47330005,
  31735040,
  62147944,
  45464480,
  'GameCore::Player::Processor::SetGameTurnComplete(GameCore::Player::Instance&, bool)'),
 (40246177, 31507424, 62034136, 44092272, 'GameCore::Game::Processor::Update()'),
 (40246182, 32696192, 62628520, 43070736, 'GameCore::Diplomacy::Manager::ProcessPending()'),
 (44092371, 31609824, 62085336, 44090464, 'GameCore::Game::Processor::BeginTurn()'),
 (44092661, 32676256, 62618552, 45467424, 'GameCore::Player::Processor::Update(GameCore::Player::Instance&)'),
 (44092712,
  32210448,
  62385648,
  44090032,
  'GameCore::Game::Processor::GetPlayersToDeactivate(Types::Vector<GameCore::PlayerTypes, '
  'Types::LinearAlloc::DefaultInline<GameCore::PlayerTypes, 64u, 51u, 0u> >&)'),
 (44092805,
  31902672,
  62231760,
  45460768,
  'GameCore::Player::Processor::DoTurnDeactivate(GameCore::Player::Instance&)'),
 (44092828, 32976272, 62768560, 44089232, 'GameCore::Game::Processor::ActivateNextPlayer()'),
 (44094071, 32590592, 62575720, 44091216, 'GameCore::Game::Processor::EndTurn()'),
 (44094096, 32814160, 62687504, 34581040, 'GameCore::AI::GlobalAi::WaitForJobs()'),
 (44090521, 32867520, 62714184, 45459648, 'GameCore::Player::Processor::BeginTurn()'),
 (44090526, 31732896, 62146872, 44386400, 'GameCore::Phase::Processor::BeginTurn()'),
 (44091169, 32832208, 62696528, 44089072, 'GameCore::Game::Processor::ActivatePlayersAtTurnStart()'),
 (44091244, 32937872, 62749360, 45459968, 'GameCore::Player::Processor::EndTurn()'),
 (44091249, 32103872, 62332360, 44386416, 'GameCore::Phase::Processor::EndTurn()'),
 (44091830, 32499968, 62530408, 42092816, 'GameCore::Game::Autoplay::Manager::EndTurn()'),
 (45460191,
  33265560,
  62004416,
  47332848,
  'GameCore::Messages::Manager::BroadcastMessage(GameCore::Messages::BaseMessage&)'),
 (45459927, 31741152, 62151000, 45750096, 'GameCore::Player::TurnManager::DoBeginTurn()'),
 (45460050, 31817696, 62189272, 45750128, 'GameCore::Player::TurnManager::DoEndTurn()'),
 (45467501,
  32724496,
  62642672,
  45460480,
  'GameCore::Player::Processor::DoTurnActivate(GameCore::Player::Instance&)'),
 (45467509, 33113296, 62837072, 45460656, 'GameCore::Player::Processor::DoTurn(GameCore::Player::Instance&)'),
 (45467555,
  33110352,
  62835600,
  45464944,
  'GameCore::Player::Processor::UpdateEndTurnBlockingState(GameCore::Player::Instance&)'),
 (45467573, 32573424, 62567136, 33680240, 'GameCore::AI::Control::Manager::DoTurn()'),
 (45460592, 31892512, 62226680, 45747040, 'GameCore::Player::TurnManager::DoTurnActivate(bool)'),
 (45460778,
  33067008,
  62813928,
  45460928,
  'GameCore::Player::Processor::SetTurnActive(GameCore::Player::Instance&, bool)'),
 (45460786,
  32557648,
  62559248,
  45461920,
  'GameCore::Player::Processor::DoAutomation(GameCore::Player::Instance&)'),
 (45460878, 31626368, 62093608, 45750000, 'GameCore::Player::TurnManager::DoTurnDeactivate()'),
 (44386951, 32797072, 62678960, 44092128, 'GameCore::Game::Processor::DoAutomation()'),
 (44386994, 31910816, 62235832, 44092048, 'GameCore::Game::Processor::EndPhase()'),
 (44387957, 32468800, 62514824, 44388368, 'GameCore::Phase::Processor::CalcPhaseTimeLimit()'),
 (44387974, 33058448, 62809648, 44091872, 'GameCore::Game::Processor::BeginPhase(int)'),
 (45748343, 31724944, 62142896, 33679488, 'GameCore::AI::Control::Manager::DoTurnActivate(bool)')]


def defined_relocations(game):
    start = struct.unpack_from('<Q', game.raw, 40)[0]
    width, count = struct.unpack_from('<HH', game.raw, 58)
    sections = [struct.unpack_from('<IIQQQQIIQQ', game.raw, start + i * width) for i in range(count)]
    result = {}
    for section in sections:
        if section[1] != 4:
            continue
        symbols = sections[section[6]]
        for pos in range(section[4], section[4] + section[5], section[9]):
            offset, info, addend = struct.unpack_from('<QQq', game.raw, pos)
            if info & 0xffffffff not in (6, 7):
                continue
            symbol = struct.unpack_from('<IBBHQQ', game.raw, symbols[4] + (info >> 32) * symbols[9])
            if symbol[3]:
                result[offset] = symbol[4] + addend
    return result

BYTES = [
    (0x2c7d51b, '83fdff745c', 'player_minus_one_resolves_local'),
    (0x2c7db94, 'be75b0bf34', 'reason_parameter_key'),
    (0x2c7dc94, 'ba8412fab6', 'absent_reason_defaults_NONE'),
    (0x2d27b11, 'be8627c0d7', 'PlayerTurnComplete_message_key'),
    (0x2d28130, '31c0c3', 'completion_does_not_unready_turn'),
    (0x2d27b63, '8b7b1883ff3f0f8776040000', 'unsigned_player_range_rejection'),
    (0x2d27b87, '84c00f8469010000', 'nonhuman_skips_pause_rejection'),
    (0x2d27b97, '84c00f8459010000', 'unpaused_continues_to_complete_check'),
    (0x2d27d00, '84c00f846f010000', 'not_complete_advances'),
    (0x2d27fc7, 'be6152b8a7', 'paused_human_ignored_report'),
    (0x2b5b863, '881885db744c', 'write_complete_then_false_skips_hooks'),
    (0x2b5b88e, '4c89ff', 'alive_predicate_uses_same_requested_player'),
    (0x2b5b89a, '4c89ff', 'turn_manager_uses_same_requested_player'),
    (0x2b5b8ad, '83c50183fd4075cb', 'repeat_per_valid_slot_0_through_63'),
    (0x2a5b644, '81fe68640b72740881fec9ea0146751d', 'only_forced_or_timer_city_overflow_path'),
    (0x2a0c6d3, 'ff5130', 'game_begin_turn_virtual'),
    (0x2d77fac, 'ff9160010000', 'XP2_map_turn_virtual'),
    (0x2a48110, 'ff5128', 'map_components_turn_in_index_order'),
    (0x2ba0b71, '85f60f84ca040000', 'not_first_activation_skips_upkeep_region'),
    (0x2ba0f73, 'ff5160', 'unit_operations_activation_callback_requires_classification'),
    (0x2b5b387, 'ff5158', 'automation_calls_original_unit_operations'),
    (0x2a0d3ab, '0f84d4f8ffff', 'processor_can_repeat_within_single_update'),
]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    module = args.assets / 'libGameCore_XP2.so'
    game = Elf(module, GAME_SHA)
    for address, size, name, expected in METHODS:
        if hashlib.sha256(game.read(address, size)).hexdigest() != expected:
            raise ValueError('method mismatch: ' + name)
    relocations = defined_relocations(game)
    for address, target, got, native, name in CALLS:
        game.call(address, target)
        raw = game.read(target, 6)
        if raw[:2] != b'\xff\x25' or target + 6 + struct.unpack_from('<i', raw, 2)[0] != got:
            raise ValueError('PLT mismatch: ' + name)
        if relocations.get(got) != native:
            raise ValueError('defined original target mismatch: ' + name)
    for address, expected, name in BYTES:
        if game.read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('byte contract mismatch: ' + name)
    virtuals = [(0x3af95d0, 0x30, 0x2d77fa0, 'XP2 game DoBeginTurn'),
                (0x3afb150, 0x160, 0x2a480e0, 'XP2 map DoTurn')]
    for table, slot, target, name in virtuals:
        if game.relocated_pointer(table + 16 + slot) != target:
            raise ValueError('virtual binding differs: ' + name)
    # GDB reads matching DWARF only. No run/start/call commands are issued.
    dwarf = subprocess.check_output(['gdb', '-nx', '-batch', str(module),
        '-ex', "ptype /o 'GameCore::Messages::PlayerTurnComplete'",
        '-ex', "ptype 'GameCore::Events::TurnRequest::RequestSubTypes'",
        '-ex', "ptype 'GameCore::Player::Processor::SetTurnComplete'"], text=True)
    for token in ('/*     24      |       4 */', '/*     28      |       4 */',
                  'total size (bytes):   32', 'NONE = -1225125244',
                  'UserForced = 1913349224', 'TurnTimerExpired = 1174530761', 'type = bool ('):
        if token not in dwarf:
            raise ValueError('matching DWARF contract differs: ' + token)
    report = dict(schema='c6-linux-turn-progression-static-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA,
        discovery_reference=dict(author='cru121', revision='5af389445118bed2bdebf7a8a42b219c8fba5175',
                                 license='CC BY 4.0', target_addresses_rederived_from_linux=True),
        methods=[dict(address=hex(a), size=n, name=s, sha256=h) for a,n,s,h in METHODS],
        direct_calls=[dict(address=hex(a), plt=hex(p), got=hex(g), original_target=hex(t), name=s)
                      for a,p,g,t,s in CALLS],
        byte_contracts=[dict(address=hex(a), contract=s) for a,_,s in BYTES],
        virtuals=[dict(table=hex(t), slot=hex(s), target=hex(a), name=n) for t,s,a,n in virtuals],
        matching_dwarf=dict(player_turn_complete_bytes=32, player_offset=24, reason_offset=28,
            set_turn_complete_returns_bool=True, request_subtypes=dict(NONE='0xb6fa1284',
                UserForced='0x720b6468', TurnTimerExpired='0x4601eac9')),
        dispatch=dict(action='0x2cc9a6b5', player_minus_one='original local player',
            parameter_key='0x34bfb075', absent_reason='0xb6fa1284',
            message_key='0xd7c02786', completion_unreadies_turn=False,
            invalid_player='unsigned >63 returns without dispatch-side gameplay calls',
            paused_human='logs and sends 0xa7b85261(player); callbacks remain in scope',
            already_complete='logs only; no setter/update/ignored report in this body',
            accepted='SetTurnComplete(true, reason); Game.Update; if player remains active, Game.Update again',
            set_complete_true_hook='same requested player DoSetTurnComplete once per valid slot in [0,64)',
            forced_human_hook='clear production overflow only for empty city build queues, original city order'),
        next_bounded_transition=dict(name='player-completion dispatch prefix with required update continuation',
            default_NONE_changes='player global turn-complete true; no forced/timer overflow branch',
            continuation='RequiresGameUpdate, internal phase token; never expose as completed EndTurn',
            implementable_from_static_contract=True, native_differential_fixture_available=False),
        update_order=['Autoplay.Update', 'clear NeedsUpdate', 'conditional Game.BeginTurn',
            'ordered GetPlayersToProcess -> Player.Update',
            'ordered GetPlayersToDeactivate -> Player.DoTurnDeactivate including automation',
            'if no active players: ActivateNextPlayer/next phase',
            'conditional Game.EndTurn', 'GlobalAi.WaitForJobs',
            'single-player diplomacy/session/deal pending gates and internal repeat', 'Autoplay.Update',
            'Game.Instance.Update tail: pending diplomacy and cache profiling'],
        decision_contract='all player decisions supplied externally; retain exact rules/upkeep, NPC state, RNG and ordering',
        earliest_next_external_boundary='not proven universally; activation/offturn/automation/diplomacy can choose before human-ready return',
        dependencies=['full logical player/phase/global transient state', 'production queue semantics for forced/timer',
            'registered reporting and signal effects', 'game/map/component and player/city/unit upkeep',
            'external policy continuations before choice mutation or RNG', 'complete gameplay RNG inventory'],
        raw_archival_attempt=dict(automatic_review_rejected=True, retried=False,
            resolution='continued approved read-only inspection and public reproducible hashes; no Ghidra/private Git mutation'),
        native_execution=False, full_host_UI_admission_required=False,
        complete_affected_state_proven=False, complete_decision_coverage=False,
        full_action_acceptance=0, gameplay_parity=False, full_gameplay_state_action_turn_supported=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(METHODS), resolved_calls=len(CALLS),
                          byte_contracts=len(BYTES), virtuals=len(virtuals), native_execution=False)))


if __name__ == '__main__':
    main()
