# Exact engine slices and full-engine gates

Evidence basis: supplied XP2 FinalRelease DLL SHA-256 `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`, image base `0x180000000`; cru121 *Civ6-GameCore-reference*, revision `5af389445118bed2bdebf7a8a42b219c8fba5175` ([CC BY 4.0](https://github.com/cru121/civ6-gamecore-reference/blob/5af389445118bed2bdebf7a8a42b219c8fba5175/LICENSE)). Local reference is an index; original instructions and fresh native observations take priority. This document adapts reference names and separates those names from proven behavior.

## Completed bounded discovery

`PlotCoord::IsValid`, RVA `0x315b80`, reads two signed 32-bit coordinates and returns `x > -9999 && y > -9999`. It does not check map dimensions. It has no call/global dependencies.

`Map::IInstance::GetPlotDistance`, RVA `0x15ec0` through return at `0x16001`, has no calls, imports, or global references. It reads only signed width at object `+0x28` and wrap-X byte at `+0x40`. Its actual native argument registers are map pointer, x1, y1, x2, followed by y2 on the stack. The upstream two-coordinate-reference signature is not its native ABI.

The distance converts x to `x - half(y)` and compares axial deltas. Same-sign deltas use the sum of absolute values; different-sign deltas use their signed maximum. With wrap enabled, choose the signed minimum for axial x deltas `dq`, `dq-width`, `dq+width`. Every add/subtract/absolute-value operation uses wrapping i32 semantics. Negative half uses the actual instruction sequence `a=y-1; (a-(a>>31))>>1`; at `INT_MIN` it yields `1073741823`, so a generic Euclidean division implementation is observably different.

`tools/analysis/check_geometry.py` independently models these instructions and invokes the unchanged mapped original PE through native C wrappers. `evidence/engine/geometry-validation.json` records **699,781 distance cases and 169 validity cases, zero mismatches**, including signed extremes, unusual widths, wrap byte 255, and random i32 inputs. This verifies isolated functions with synthetic map fields, not initialized map objects or gameplay. Native/Rust integration and their differential protocol are owned by their respective implementations.

## Action pipeline established from bytes

All addresses below are RVAs. Exact instructions are in `evidence/engine/bootstrap-dispatch.asm`; original reference rows and call sites are in `entrypoints.json`. Referenced callee names can be aliases after identical-code folding and remain hypotheses until inspected. A direct call edge is proven; its inferred C++ name is a separate claim.

1. Export `RequestUnitOperation` `0x5915d0` rearranges its arguments and tail-calls `0xd1e90`, labeled `Cache::Unit::Manager::RequestOperation`.
2. `0xd1e90` consults the cached player and operations manager, performs an indirect call at vtable byte slot `+0xf8`, constructs and initializes `Messages::UnitOperation` (`0x3a640`, `0x3ab60`), then calls `Messages::Manager::BroadcastMessage` (`0x366b0`). Consequently accepting an operation means resolving a typed request and message transport, not directly calling the handler's `Start`.
3. `Messages::Manager::ProcessMessages` `0x372f0` constructs memory streams, calls `ReceiveMessage` `0x37b50`, frees buffers, and reports an event. Message factories, serialization, queues, and execution ordering must exist before an action can be reproduced through this route.
4. `Messages::UnitOperation::Execute` `0x3aad0` resolves game/player objects, gets operations manager `0x408f0`, calls its virtual byte slot `+0x60`, then processes pending diplomacy (`0x186e60`) and updates end-turn blocking (`0x5e410`). Offset/type labels for object lookup are not established here.
5. `Unit::Operations::Manager::Add` `0x3f9c0` has many virtual calls and reporting dependencies. It is not an isolated next native target. Identify each receiver's vtable before bypassing any call.

Next bounded action milestone: reconstruct one message type and its typed parameter conversion from `0xd1e90` and the message constructor/init/buffer readers; validate serialized request bytes in a running-game fixture, then trace a legal SkipTurn action before attempting movement/pathfinding. Record both accepted/rejected requests and operation queue changes. `CanStart`, `Start`, and game legality are not yet validated by the geometry work.

## End turn is a state machine

Completion message `PlayerTurnComplete::Execute` `0x39920` calls `SetTurnComplete` `0x5e230` and triggers `Game::Instance::Update` `0x63e2e0`; `GameTurnComplete::Execute` `0x35f70` consults `NeedsTurnAdvance`, sets game-turn-complete flags, and triggers the same update. The reference explicitly warns that these completion bodies changed between builds. Current bytes must determine reasons/flags and ordering.

`Game::Instance::Update` invokes `Game::Processor::Update` `0x4afc0`, pending diplomacy, then a cache dispatch/profile tail call. The processor handles BeginTurn, players to process/deactivate, player updates, activation, active counts, automation, EndTurn, AI job waits, and pending diplomacy/session/deal checks. `Game::Processor::EndTurn` `0x498f0` calls player EndTurn, phase BeginTurn, reporting, signals, and autoplay EndTurn. `Player::Processor::DoTurn` `0x5ca10` reaches `Player::TurnManager::DoTurn` `0x344a00`, which can enter `AI::Control::Manager::DoTurn` `0x401050`.

A misleading shortcut: export `Exports::Context::Update` `0x591900` only calls autoplay Update when its bool argument is false and returns zero. It is not a gameplay tick loop.

Next bounded end-turn milestone: trace one loaded, single-player, no-mod fixture through completion message -> game update until a clearly recorded next-player/turn boundary. Capture ordered messages/events, player active/complete/blocking state, phase/segment, queued operations, RNGs/counts, and pending diplomacy. Do not equate setting a completion flag or calling EndTurn once with a legal complete transition. Idle/nonhuman automation and victory checks still require evidence.

## Bootstrap service blockers

Original `Context::Globals::SetEngineUtility` `0x45050` uses a GUID query via the input interface's first virtual slot, stores engine utility at global `0xb8aa80`, fetches script system through byte slot `+0x10` into `0xb8aa88`, and logging manager through `+0x18` into `0xb8aa90`, then registers events. The slots are proven instruction offsets; semantic interface labels come from reference global types.

`Context::Instance::Initialize` `0x45d90` reads initialized flag at current Windows `this+0x40`, sets context global `0xb8aa60`, and **unconditionally dereferences engine utility** to call byte slots `+0x140` and `+0x138`. It creates logger objects and invokes imported `DATABASEDLL_FINALRELEASE.DLL::Database::CreateConnection` twice, stores connections at `this+0x50/+0x58`, and calls `Scripting::Manager::Initialize` `0x5fda50`. The Linux layout has initialized flag at +8; copying it would be wrong.

`Context::Instance::StartupGame` `0x46be0` allocates definitions/capabilities/tags, initializes the definition repository and index serializer, creates rules, initializes worldbuilder, registers archive types, and calls additional virtual hooks. `InitializeNewGame` `0x45f80` copies configuration and reaches `Game::Initializer::InitializeNewGame` `0x227d90`. `InitializeDatabase` `0x45ed0` reaches effects database writer logic and database virtual calls.

The current isolated mapper deliberately does not run DllMain or bind imports and supplies only a logging stub sufficient for RNG. It cannot initialize the game context. No safe exact bootstrap follows merely from finding a context constructor.

Next bounded bootstrap milestone: enumerate the engine/script/logging vtable contracts and GUIDs, required import providers (database, HavokScript, platform/allocator), static initializers/TLS, and game-core exports against the supplied matching dependency DLLs. Implement an explicit service inventory with required/optional slot classification before any context call. Prefer a running-game capture fixture for the full-engine oracle while standalone initialization is unresolved. Database file contents alone do not provide a live `IDatabaseConnection1` object or engine script/event services.

## Evidence limits and acceptance gates

- Disassembly extraction initially uses old reference sizes. Current `.pdata` entries are unwind regions and may split one logical function; they must not be treated as complete function ranges. Never execute a copied slice sized by an upstream row without proving branch closure.
- The geometry instructions are closed and freshly tested; action/bootstrap observations are static call-edge evidence, not successful initialized execution.
- A future snapshot must include actual ruleset/database/mod identity and deterministic state/iteration order. Neither a hand-written state struct nor Linux DWARF layout establishes complete Windows state.
- The next simulator claim should remain bounded by actual differential coverage. Full state, actions, and end-turn are outstanding until original-game fixtures execute the same legal requests and compare defined state/event/RNG observations.

## Incoming Linux installation

Pinned upstream `findings/topics/linux-debug-symbols.md` identifies **`libGameCore_XP2.so`** in Steam Linux depot **533502**, with full DWARF (clang 6, roughly 1,100 source files, 140k types and 109k functions with parameter names). This is a pre-New Frontier Pass build, not evidence that a newly supplied library has the same symbols/version. `findings/topics/exe-gamecore-architecture.md` separately identifies host executable **`Civ6`**; the simulation is a separate shared library, not embedded entirely in that executable.

The architecture notes report about 2,041 plain GameCore imports resolved against the host executable's exports, including Data/TypedVariant, String, Lua, Platform, Localization and Types. Therefore preserve the matching `Civ6` host, all libraries (including library symlinks), debug/symbol files, rules/data and installation/build metadata. A copy of the full installation is more useful than only the GameCore `.so`. DT_NEEDED alone cannot establish that `dlopen` works; loader relocation, host-exported symbols, constructors, and service callbacks remain separate gates.

Run `python3 tools/analysis/inventory_elf.py INSTALL_ROOT --output INVENTORY.json` after files arrive. It hashes ELF files and uses `readelf` to report architecture/type, build IDs, shared-library dependencies/paths, debug info/symtab/debuglink presence, dynamic unresolved symbols and context exports. It does not run the game, call `ldd`, or load a library. It does not automatically hash game databases or establish that external debug files match; use ELF build IDs/debuglink evidence for that next step.

Upstream describes host bootstrap as factory -> construct EngineUtility -> IContext1 SetEngineInterface (byte slot +0x60) -> Initialize (+0x28) -> GetScriptSystemUtility (+0x68) -> profile parameters (+0x1e8) -> controller SetContext. These are reference observations, not a successful local bootstrap. The actual supplied Linux symbols and vtable layout must decide its ABI. Linux ELF VAs, calling convention and object layouts cannot reuse Windows RVAs/ABI.

## Next executable context slice

On the fingerprinted Windows PE, `Exports::Context::QueryInterface`, RVA `0x5907a0`, compares the 16 bytes pointed to by RDX with read-only constants at RVAs `0xa101c8` and `0xa101d8`, returns RCX unchanged on a match, otherwise null. It has no calls, object dereferences, writes, allocator or engine-service requirements. This is independently executable with a synthetic `this` pointer. GUID raw bytes are respectively `ae1400d760d96846bd9788a9c5b36b9a` and `e8e4f51860ef8d4cbb1b3834cc571031`; assigning interface names to each requires additional type evidence. The input is a pointer to 16 bytes in the actual Windows ABI, even though the C++ source signature uses a value GUID.

The factory `Exports::Context::Create` at `0x58fc60` is more involved: it uses imported heap creation/information/allocation through IAT slots `0x9b7240/0x9b7260/0x9b7250`, constructor `0x58fb40`, and a tracked vector at `0xb8f0c0`. The constructor installs vptr `0xa101e8` in a 16-byte export object, allocates an underlying `0x1a80`-byte XP2 context via allocator `0x990900`, then calls XP2 constructor `0x7503e0` -> base `0x454d0`. Factory allocation and QueryInterface success would establish only object/interface creation, not an initialized gameplay engine. No invented service stub should turn that narrow result into a gameplay parity claim.
