# Linux application-state construction boundary

The pinned original host's `Civ6App::SetupGameStates` is **not isolated state bookkeeping**. It creates all six owned state objects and immediately enters `InitAppState::Begin`, which requires ForgeUI and starts UI work. A narrower original API that publishes the application's load-state owner has not been established. This static audit therefore does not establish a safe native experiment that resolves the current receive gate.

Reproduce the read-only audit:

```sh
python3 tools/linux/action_app_state_audit.py --output evidence/linux/action-app-state-abi.json
```

Both original Linux ELF SHA-256 identities are mandatory. The audit checks 21 complete function byte hashes, original call sites, relevant instruction bytes and virtual bindings; its JSON also identifies the audit source. Raw original disassembly and the operand search remain private in `../c6-ghidra/evidence/linux-action-app-state/`. No original code executes during the audit. This adds no simulator mechanic, action support or full gameplay state/turn parity.

## Why the receive gate reports loading

`Serializer::Manager::IsLoadInProgress` (`0x32490e0`) returns true when a serializer request is active and its type equals 2. Otherwise it obtains `Civ6App::GetCurrentGameState` (`0x2fbff10`, App+`0x6250`) and compares it to the load-state owner at App+`0x2310`. There is no null guard on this equality. A fixture with both null therefore reports loading even though no load state exists.

The separate `Civ6App::IsLoadGameStateActive` (`0x2fa0010`) explicitly rejects a null load owner before testing equality. These predicates differ and cannot be substituted. The evidence identifies the original lifecycle gap; it does not justify replacing either predicate, changing its return or publishing a synthetic state pointer.

## Ownership and original setup ABI

Setup at `0x2f96240` has SysV ABI `bool(Civ6App* RDI)`, AL true on normal return; allocation/constructor exceptions may unwind. Its six allocations use original platform allocation and immediately publish constructed objects into the actual application:

| State | Allocation bytes | App owner offset | Constructor |
| --- | --- | --- | --- |
| InitAppState | `0x448` | `0x2300` | `0x2fcca20` |
| IntroState | `0x488` | `0x22f8` | `0x2fcd2e0` |
| MainMenuState | `0x448` | `0x2308` | `0x2fd81a0` |
| LoadGameViewState | `0x4f0` | `0x2310` | `0x2fd0dd0` |
| GameViewState | `0x1330` | `0x2318` | `0x2fb96f0` |
| OwnershipState | `0x438` | `0x2320` | `0x2fda630` |

Setup has no existing-owner rejection or cleanup: calling it twice overwrites owners and leaks existing objects/subscriptions. It needs the actual application, original allocation services, six empty owner slots and constructor dependencies. Intro constructs a movie blitter; GameView constructs render/frame/camera/job objects. Those dependencies have not been proven initialized in the loaded headless fixture.

At `0x2f96283`, setup forces InitApp+`0x430` bit 2. After constructing all six states it sets current to the original InitApp owner, clears next at App+`0x6248`, and invokes current virtual slot `0x10` at `0x2f963d2`. Original InitApp table `0x53a2140`, address point +16, binds that slot to Begin `0x2fcca90`.

Begin calls original ExternalLighting.SetFrontEndColor, tests the forced bit, and therefore calls `LoadUI` (`0x2fccad0`). LoadUI obtains global `ForgeUI::UIManager` at `0x6560808` and calls AllocateControl, sets the context base filename, calls the context virtual slot `0x30`, adds a root context and changes primary-target clearing. A null or incomplete original UI manager is not a supported precondition. Begin then tailcalls `Civ6GameState::RunScript` (`0x2fb8dc0`), whose nonempty-path branch reads a file and publishes UI debug-console-input events. RunScript returns early for an empty path; this audit does not claim a script necessarily runs on a fresh InitApp. UI/context transitive closure remains unproven.

Setup does not directly call `LoadGameViewState::Begin`. It nevertheless already performs broader application/UI work through InitApp.Begin, so it cannot be selected as a policy-free allocation helper from these findings.

## Standalone load constructor is insufficient

`LoadGameViewState::C1` (`0x2fd0dd0`) has only the concrete receiver in RDI. It initializes its base, embedded AsyncJob and timers, clears its own fields, then subscribes to Sequencer AppInitComplete event 1 and PostLoadBegin event 9. It neither receives a Civ6App argument nor writes App+`0x2310`. Constructing it alone would not resolve the application owner's null equality.

Its D1/D2 at `0x2fd0f10` unsubscribes both callbacks, releases receiver+`0x4c0` if nonnull through the original interface deallocator, destroys the embedded AsyncJob and base. D0 at `0x2fd0fc0` invokes destruction and native storage deletion. Use the original allocation/deletion ownership; a constructed receiver is not permission to publish an application pointer manually. Constructor subscription failure/exception behavior and complete job dependencies remain outside this bounded proof.

A whole-host `objdump -d` search for explicit memory operands at offset `0x2310` found Setup's store at `0x2f96337` as the only Civ6App writer. Other matching stores belong to unrelated device-context constructors. This is a syntactic search, not a proof excluding aliased access, bulk initialization or indirect writes. No narrower genuine owner-publishing method was found.

## Teardown is also application-wide

`ShutdownGameStates` (`0x2f98270`) has ABI `void(Civ6App* RDI)`. When GameView owner exists, it calls original `GameViewState::ShutdownGame` and `Shutdown` before invoking each of the six owners' deleting destructor virtual slot `0x8`. It finally zeros next/current together. It **does not clear the six owner slots**, so it is not safely repeatable on the same application without the rest of its original lifetime handling. It also does not call InitApp.End to mirror a UI Begin explicitly.

ShutdownGame first tests GameView byte +`0x11f7`. The fresh GameView constructor writes a qword at +`0x11f0` which makes this byte zero. That fresh path skips the broader network/core teardown and calls the active controller's original `SetInitializingGameState(false)` if a controller exists. For a nonzero byte, the broader path contains Network.LeaveGame, Controller.Uninitialize, GameCore.ShutdownGame and receiving-event-queue clearing, subject to its further original branches. These are conditional paths, not a claim that a fresh object necessarily destroys the loaded core.

The subsequent `GameViewState::Shutdown` (`0x2fbc150`) independently calls global ColorKey, LeaderScreen, Lighting, WorldView and other rendering/simulation shutdown functions. This prevents treating Setup/Shutdown as an isolated allocation/free pair around an already live loaded core without additional original dependency/ownership proof.

The next missing result is an evidenced original lifecycle that creates and publishes genuine App state with ready dependencies and a compatible teardown. The current static result leaves that blocked; no Begin/request/platform bypass, fake objects, field writes or event insertion are proposed. Native receive/execute, affected-state comparison and the full external-policy decision-boundary goal remain open. Unknown save bytes stay unclassified and unmasked; the recorded raw 89,198-byte Save→Load failure remains a failure.
