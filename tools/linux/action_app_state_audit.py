#!/usr/bin/env python3
"""Read-only identity-gated Linux application-state ownership/lifecycle audit."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA

METHODS = [
  [
    50123488,
    164,
    "_ZN10IntroStateC1Ev",
    "d6f4c1fa75957a4dd1308a9abf5cc9da696f4b9f37d1b775eb5d5061830aee41"
  ],
  [
    50121360,
    52,
    "_ZN12InitAppState5BeginEv",
    "40e2a736ce52a1bbead118e008b4ac5415333664434a2b1322a084b479e4a3bf"
  ],
  [
    50121424,
    123,
    "_ZN12InitAppState6LoadUIEv",
    "36fc5e02e89c2723bb1cd66026b2831da565d21d892262f1dc62fa980745c8cf"
  ],
  [
    50121248,
    53,
    "_ZN12InitAppStateC1Ev",
    "479a451bc47aef3c7cd27aaf0bd4c4e49a338535c418bbdc6801c8eecb562dcb"
  ],
  [
    50040112,
    53,
    "_ZN13Civ6GameStateC2Ev",
    "b4b131bacd7812501344f6bbbc3701ece54be5093ffd6e3e3d81137a6478c030"
  ],
  [
    50041136,
    9,
    "_ZN13Civ6GameStateD2Ev",
    "52cb7d8d30e9d34eb54b80a932e39f0cdf679a306b02005199a2cd69ce7e73c1"
  ],
  [
    50052288,
    1120,
    "_ZN13GameViewState12ShutdownGameEv",
    "b436c36a5ea527140d68e756d76d18983c544d87896a74fe32e3789c7a48675b"
  ],
  [
    50053456,
    290,
    "_ZN13GameViewState8ShutdownEv",
    "63ef2d286852b7e47bfb784072ca50a1b0fed3fe1a49c640c1015957786fef47"
  ],
  [
    50042608,
    356,
    "_ZN13GameViewStateC1Ev",
    "131118fb133d03f4657a244946c94995aeeb7e627237a83fc9107bb2bfb7d8ee"
  ],
  [
    50168224,
    45,
    "_ZN13MainMenuStateC1Ev",
    "8dd8bac7ffbf5967a9bbe2eae4734bf5a6a3c31d4453662c6896488a6ce92274"
  ],
  [
    50177584,
    25,
    "_ZN14OwnershipStateC1Ev",
    "6be52b0da10f0ef99a65535afc02bbb84d5f67bc719afb370e49b68de99036c1"
  ],
  [
    50138448,
    100,
    "_ZN17LoadGameViewState12InitAsyncJobC1Ev",
    "37a855d3e298df6b84ee4db488afc9d316e5a9ba68796b6c9b5047427d5e5936"
  ],
  [
    50138576,
    248,
    "_ZN17LoadGameViewStateC1Ev",
    "49e728b6ba33cd4fea8ee3995d36fc3280252f39c3ab58006f68aaa86e038f43"
  ],
  [
    50138896,
    166,
    "_ZN17LoadGameViewStateD1Ev",
    "a13a811ced391c764bc38474de8a0b7ebf27af0215a44d6dbdbe2edcc2271ec7"
  ],
  [
    49898048,
    540,
    "_ZN7Civ6App15SetupGameStatesEv",
    "f0108a3c8b70c0488d90aa82891a1faba4e499aaabf548716ae8c9a4ed877b3a"
  ],
  [
    49906288,
    153,
    "_ZN7Civ6App18ShutdownGameStatesEv",
    "49e1613d1a268fa1cdc04444f657fadfe3ce391602739790c0c406d482400412"
  ],
  [
    50069264,
    8,
    "_ZN7Civ6App19GetCurrentGameStateEv",
    "cd3b088a46074ffa9b4c4790869e67ab64a703fe19f5c00dd630b68fe62361e6"
  ],
  [
    49905456,
    118,
    "_ZN7Civ6App19HandleGameStateSwapEv",
    "72c721f84cf61e11c3c4ea8ebad39261c90f1dd485594e9aa3fc1a2c094614e2"
  ],
  [
    49938448,
    26,
    "_ZN7Civ6App21IsLoadGameStateActiveEv",
    "e0b1ee82e09dce6e3b6de27c9d6f6638dbf61f04cd66ccbbc1d0485951f8067a"
  ],
  [
    52728032,
    58,
    "_ZN8GameCore10Serializer7Manager16IsLoadInProgressEv",
    "a29bd37f58433182f9e9d4e1e942a008d3f8f93f621f2919d69dfa543a1caea3"
  ],
  [
    50040256,
    242,
    "_ZNK13Civ6GameState9RunScriptEv",
    "71dd7f9a6b42dc25a7a514c90057ddf6d2a3ca4a3dada6fcaeb5887006de5b95"
  ]
]

CALLS = [
    (0x2f96277, 0x2fcca20, 'Setup constructs InitAppState'),
    (0x2f962ba, 0x2fcd2e0, 'Setup constructs IntroState'),
    (0x2f962f6, 0x2fd81a0, 'Setup constructs MainMenuState'),
    (0x2f96332, 0x2fd0dd0, 'Setup constructs LoadGameViewState'),
    (0x2f9636e, 0x2fb96f0, 'Setup constructs GameViewState'),
    (0x2f963aa, 0x2fda630, 'Setup constructs OwnershipState'),
    (0x2fcca94, 0x2fb9660, 'Init Begin changes external lighting'),
    (0x2fccaa5, 0x2fccad0, 'Init Begin loads UI with setup flag'),
    (0x2fd0e70, 0x2fd67d0, 'Load ctor subscribes AppInitComplete'),
    (0x2fd0e81, 0x2fd6810, 'Load ctor subscribes PostLoadBegin'),
    (0x2fd0f2e, 0x2fd6850, 'Load dtor unsubscribes AppInitComplete'),
    (0x2fd0f3f, 0x2fd6890, 'Load dtor unsubscribes PostLoadBegin'),
    (0x2f98280, 0x2fbbcc0, 'Shutdown calls GameView ShutdownGame'),
    (0x2f9828c, 0x2fbc150, 'Shutdown calls GameView Shutdown'),
    (0x2fbbf23, 0x33bfdc0, 'GameView shutdown path leaves network game'),
    (0x2fbc0bf, 0x31d1400, 'GameView shutdown path uninitializes controller'),
    (0x2fbc0c4, 0x31dd810, 'GameView shutdown path shuts down GameCore'),
    (0x3249106, 0x2fbff10, 'Serializer obtains actual current state'),
]
INSTRUCTIONS = [
    (0x2f96283, '808d3004000002', 'Setup sets InitApp LoadUI flag bit 2'),
    (0x2f96337, '4889ab10230000', 'Setup publishes load state at App+0x2310'),
    (0x2f963bd, '4889bb50620000', 'Setup publishes current InitApp at App+0x6250'),
    (0x2f963d2, 'ff5010', 'Setup invokes current Begin virtual slot 0x10'),
    (0x2fcca99, 'f6833004000002', 'Init Begin tests LoadUI flag bit 2'),
    (0x2fbff10, '488b8750620000c3', 'Current getter loads App+0x6250'),
    (0x324910e, '483b8110230000', 'Serializer compares load owner against current without null guard'),
    (0x2fb97d2, '48b80100000100010000', 'Fresh GameView constructor qword implies byte +0x11f7 zero'),
    (0x2fbbcd4, '80bbf711000000', 'ShutdownGame selects broad shutdown when byte +0x11f7 nonzero'),
    (0x2fa0017, '4885c0', 'Separate IsLoadGameStateActive has null guard'),
    (0x2f98300, '0f118348620000', 'Shutdown clears next/current but not six owner slots'),
]
SLOTS = [
    (0x53a2140, 0x10, 0x2fcca90, 'InitAppState.Begin'),
    (0x53a22e8, 0x08, 0x2fd0fc0, 'LoadGameViewState deleting destructor'),
    (0x53a22e8, 0x10, 0x2fd14c0, 'LoadGameViewState.Begin; not called by Setup'),
]
STATES = [
    ('InitAppState', 0x448, 0x2300, 0x2fcca20),
    ('IntroState', 0x488, 0x22f8, 0x2fcd2e0),
    ('MainMenuState', 0x448, 0x2308, 0x2fd81a0),
    ('LoadGameViewState', 0x4f0, 0x2310, 0x2fd0dd0),
    ('GameViewState', 0x1330, 0x2318, 0x2fb96f0),
    ('OwnershipState', 0x438, 0x2320, 0x2fda630),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for address, size, name, digest in METHODS:
        if hashlib.sha256(host.read(address, size)).hexdigest() != digest:
            raise ValueError('method identity differs: ' + name)
    for address, target, _ in CALLS:
        host.call(address, target)
    for address, raw, name in INSTRUCTIONS:
        if host.read(address, len(bytes.fromhex(raw))) != bytes.fromhex(raw):
            raise ValueError('instruction differs: ' + name)
    for table, offset, target, name in SLOTS:
        if struct.unpack('<Q', host.read(table + 16 + offset, 8))[0] != target:
            raise ValueError('virtual binding differs: ' + name)
    result = dict(
        schema='c6-linux-action-app-state-abi-v1', checks_passed=True,
        host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA,
        audit_source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        method='Read-only pinned original Linux function bytes, call sites, and virtual bindings',
        methods=[dict(address=hex(a), size=s, symbol=n, sha256=h) for a,s,n,h in METHODS],
        direct_calls=[dict(address=hex(a), target=hex(t), meaning=n) for a,t,n in CALLS],
        instructions=[dict(address=hex(a), bytes=b, meaning=n) for a,b,n in INSTRUCTIONS],
        virtual_bindings=[dict(table=hex(t), offset=hex(o), target=hex(a), meaning=n) for t,o,a,n in SLOTS],
        states=[dict(type=n, allocation_size=hex(s), app_owner_offset=hex(o), constructor=hex(c)) for n,s,o,c in STATES],
        abi=dict(setup='bool(Civ6App* RDI), AL=true on normal return; exceptions can unwind',
                 shutdown='void(Civ6App* RDI)',
                 constructor='void(allocated concrete state* RDI); no App receiver or ownership publication',
                 deleting_destructor='void(state* RDI), original virtual slot +8; use actual native allocation ownership'),
        setup_contract=dict(idempotent=False, allocates_all_six=True,
            publishes_all_six_before_begin=True, enters='InitAppState.Begin; never LoadGameViewState.Begin directly',
            begin_loads_forge_ui=True, narrow_state_bookkeeping=False,
            preconditions=['actual live Civ6App, original allocation/services',
                           'no existing six owned state objects; Setup overwrites owner slots without cleanup',
                           'valid original ForgeUI.UIManager and its context dependencies for InitApp.Begin',
                           'all six constructor dependencies including movie/render/job and Sequencer subscription services'],
            native_experiment_safe=False),
        load_constructor_contract=dict(app_publication=False, subscribes_sequencer_events=[1,9],
            owns_async_job=True, directly_begins_load=False,
            cleanup='D1 unsubscribes both Sequencer callbacks, releases +0x4c0 if nonnull, destroys AsyncJob and base; D0 additionally deletes storage',
            does_not_solve_null_owner_gate=True),
        loading_predicate=dict(serializer='0x32490e0',
            rule='true if request-in-progress and request-type ==2, else current App+0x6250 == load owner App+0x2310',
            null_current_and_load_owner_result=True,
            separate_IsLoadGameStateActive='0x2fa0010 returns false for null load owner; not the serializer predicate'),
        shutdown_contract=dict(deletes_six_via_virtual_slot='0x8', clears_next_current=True,
            clears_six_owner_slots=False, calls_GameView_ShutdownGame_and_Shutdown=True,
            broad_shutdown_paths=['Network.LeaveGame', 'Controller.Uninitialize', 'GameCore.ShutdownGame', 'event queue clears', 'render/UI/system shutdown'],
            fresh_GameView_shutdown='constructor sets byte+0x11f7=0; ShutdownGame takes early branch which calls active Controller.SetInitializingGameState(false), skipping LeaveGame/Uninitialize/CoreShutdown; subsequent Shutdown still invokes global subsystem shutdown',
            isolated_allocate_free_pair=False, repeated_shutdown_safe=False),
        narrow_publisher_search='Whole-host objdump explicit 0x2310 memory operand scan found Setup as only Civ6App writer; aliases, bulk writes, indirect flows not excluded',
        remaining_gaps=['No genuine narrower original owner-publishing API established',
                        'Setup transitive UI/movie/render/job dependency readiness and policy closure unproven',
                        'Safe live-core ownership interaction of global subsystem teardown unproven',
                        'No native fixture, action execution or affected-state comparison in this static audit'],
        native_execution=False, action_executed=False, gameplay_parity=False,
        scope='Bounded Linux App-state lifecycle evidence; complete exact gameplay state/actions/turns remain unimplemented; raw Save->Load differences including 89198 bytes remain unmasked')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods_checked=len(METHODS), native_execution=False)))


if __name__ == '__main__':
    main()
