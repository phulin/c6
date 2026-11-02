# GameCore reference provenance and working index

Research starting point: [cru121's GameCore reference](https://cru121.github.io/civ6-gamecore-reference/) and its [source repository](https://github.com/cru121/civ6-gamecore-reference), acquired 2026-11-02 at commit `5af389445118bed2bdebf7a8a42b219c8fba5175`. The Pages URL could not be opened through the web tool; GitHub metadata, README, full source and generated site were available. Run `python3 tools/reference/acquire.py` to obtain the pinned, ignored mirror at `evidence/reference/upstream`. `tools/reference/manifest.json` records SHA-256 hashes for key artifacts. No game binaries are downloaded or committed by that script.

This is an index and research lead, not proof of simulator fidelity. Upstream explicitly says much of its prose is AI-assisted, most entries are untested, and **verified means read from decompilation**, unless the page specifically reports a running-game observation. Preserve its evidence and mapping categories when using a row.

## Version and license boundary

| Upstream identity | Purpose and limitation |
|---|---|
| Steam depot 947510, older symbol-bearing XP2 build | Source of C++ names and old RVAs; exact depot manifest is not supplied by `builds.json`. |
| Steam build 15038592, July 2024 | Target of current GameCore RVAs. Upstream says August 2024 build 15296837 has the identical GameCore DLL. This claim does not establish the identity of a supplied DLL. |
| Steam Linux depot 533502, described by upstream as before New Frontier Pass | Upstream DWARF member names, sizes, offsets and parameters. This attributed label does not date the supplied Linux binary; layouts are not a cross-platform ABI guarantee. |

Documentation and data are [CC BY 4.0](https://github.com/cru121/civ6-gamecore-reference/blob/5af389445118bed2bdebf7a8a42b219c8fba5175/LICENSE), tools are [MIT](https://github.com/cru121/civ6-gamecore-reference/blob/5af389445118bed2bdebf7a8a42b219c8fba5175/LICENSE-CODE). Attribution: cru121, *Civ6-GameCore-reference*, cited repository and pinned revision; this document reorganizes a small selection of its data. Upstream credits Sukrit Tan, ChimpanG, WildW, and Community Extension contributors. Its repository publishes names, addresses, notes and short snippets, not the game implementation or binaries.

The old/current RVAs below describe upstream Windows mappings and preserved historical research. Active execution and acceptance use the supplied Linux profile; see [targets](targets.md), [release identity](version-audit.md) and [reference reuse](reference-reuse.md). Cross-platform layout differences do not establish relative age.

Before applying any address, record the actual module SHA-256, native image identity and ruleset. An expansion DLL or platform mismatch invalidates direct use of these RVAs. A symbol-build/current-build match is a claim to verify against bytes and runtime, especially for changed functions.

## Local data worth using

All paths below are relative to `evidence/reference/upstream/`.

| Artifact | Use |
|---|---|
| `data/old_to_new_offsets.tsv`, `data/README.md` | 43,803 old function rows, 40,404 with candidate current addresses. TSV columns: old RVA, new RVA, category, old size, qualified C++ name. Overloads and folded implementations can share names or addresses. |
| `reference/data/function_index.json` | Compact function rows and category list; consult `reference/tools/extract_index.py` for row schema. |
| `reference/data/builds.json` | Three build identities above. |
| `reference/data/layouts.json` | 1,915 Linux DWARF class layouts with named members, sizes, bases and evidence. Upstream coverage says only 25 members were checked against Windows code. |
| `reference/data/globals.json`, `vtables.json` | Globals with per-build addresses, mapping evidence; per-class virtual slots. |
| `reference/data/lua_methods.json`, `lua_signatures.json` | 1,920 Lua methods, wrapper addresses, registration contexts, callees and recovered argument types. UI cache and gameplay objects are distinct. |
| `reference/data/operations.json`, `op_params.json` | 154 operation/command entities and 167 parameter entities, hashes, handlers, confidence and parameter meanings. |
| `findings/tables/operation_handlers.tsv`, `player_operation_cases.tsv` | Handler lifecycle functions and inline player-operation branches. |
| `reference/data/effects.json`, `modifier_uses.json` | Data-driven effects and places the game uses them; not a complete gameplay database. |
| `findings/topics/build-differences.md` | Crucial old/current divergences, including turn-ending message changes and shifted unit combat flags. |
| `tools/tsv_to_symbols.py` | Upstream converter to name→RVA arrays, filtering ambiguous and weaker categories. |
| `tools/frida-live/` | Existing Windows live-game dispatcher, Lua bridge, crash reporting and named native calls. |

Mapping categories are materially different. `unique`, `unique-reordered` and `resolved` are upstream's strongest byte/call-neighbor evidence, not certification. `callgraph-*` and `vtable-derived` require the stated inference. `ambiguous` contains candidate lists, potentially truncated; do not parse it as one RVA. `datashift-probable` and `fuzzy-best` need inspection. `unmatchable` and `none` have no counterpart. The scripts that originally made the map are **not supplied**, so acquisition does not make the matching independently reproducible.

## RNG: what the reference establishes

`GameCore::Random` has Linux DWARF size 32 bytes: vptr at 0, `m_ulRandomSeed` (`SeedType`) at 8, `m_ulCallCount` u32 at 12, `m_ulResetCount` u32 at 16, `m_uiLockCount` uint at 20, `m_bSynchronous` bool at 24. `ScopedRandom` is a separate four-byte seed object. `Game::Instance::m_kSyncRandom` is an `FAutoVariable<Random, Game::Instance>` at Linux offset 232; wrapper offset is not necessarily the inner object's offset.

| Function | Symbol RVA | Current RVA | Map / old size |
|---|---|---|---|
| `Random::Random` | `0x5f9390` | `0x5f9ee0` | unique / 31 |
| `Random::Get` | `0x5f94f0` | `0x5fa040` | unique / 214 |
| `Random::Initialize` | `0x5f95d0` | `0x5fa120` | unique / 82 |
| `Random::Read` | `0x5f9640` | `0x5fa190` | unique / 470 |
| `Random::Reset` | `0x5f9820` | `0x5fa370` | callgraph-callee / 7 |
| `Random::Write` | `0x5f9840` | `0x5fa390` | unique / 156 |

`Game.GetRandNum` gameplay Lua wrapper current RVA `0x6d7710` calls `Random::Get`, returns `IntRange`, and accepts an optional number and optional log string according to recovered wrapper signatures. `Game.GetRandomSeed` is at `0x6d77f0`; `Game.SetRandomSeed` at `0x6d7fc0`, taking `SeedType` and calling `Game::Instance::SetRandomSeed`. `TerrainBuilder.GetRandomNumber` wrapper at `0x22920` also calls `Random::Get`, with required `IntRange` and log string. The map generator owns separate global `Map::Generator::ms_kRandom` at current RVA `0xb8f460`.

**Do not substitute PCG32 based on the UI name:** `Cache::Lua::ms_Random` is separately typed `Math::RandomPCG32`, size 16, current RVA `0xb8ffa0`. This establishes multiple RNG contexts. No verified gameplay recurrence, exact native parameter ABI, range reduction or zero-range behavior was found in upstream notes/data. Those must be derived from the actual binary and differential tests, including seed/count/reset/lock behavior, bounds 0/1/max, and interleaved streams. Algorithm names alone are insufficient.

## Actions and turn transition targets

The tracked `tools/reference/entrypoints.json` preserves original rows for Random, Game/Player processors, completion messages, sync-seed initialization and player operation processing. Addresses are RVAs, not absolute pointers, and old sizes are not target instruction boundaries.

For the current XP2 build, `Game::Processor` exposes `BeginTurn` `0x49390`, `EndTurn` `0x498f0`, `Update` `0x4afc0`; `Player::Processor` exposes `BeginTurn` `0x5c2d0`, `DoTurn` `0x5ca10`, `EndTurn` `0x5cbc0`, `Update` `0x5e2d0`, `SetTurnComplete` `0x5e230`. `Messages::PlayerTurnComplete::Execute` is `0x39920` and `Messages::GameTurnComplete::Execute` `0x35f70`. **These names are hook and analysis leads, not proof that invoking one directly is an entire legal end turn.** Upstream flags both completion-message bodies as changed between builds.

Unit/city actions have `CanStart`, `Start`, `MakeParameters`, `SetParameters`, with spy operations also `ResolveOperation`. Upstream reports 108 handler hashes recovered directly and 16 inferred from names. Player operations largely dispatch through a hash decision tree in `Player::Operations::Manager::Process` (current `0x30d330`); six are separate classes. Recovered parameters and wrapper contexts should determine the action schema instead of inventing a generic argument list. The supplied upstream hash function is bitwise NOT of standard CRC32 of an uppercase identifier, stored as 32 bits. Uppercase normalization and signed JSON presentation must be explicit.

Upstream Windows build-difference observation: `Unit::CombatModifiers` gained byte `NoBarbXPLimit` at offset `0x81`, shifting following flags. Upstream identifies Windows `unit+0x821` and links it to Caesar's effect. The supplied Linux layout demonstrably differs at this boundary; this prevents transferring layouts, but does not establish that the supplied Linux build is older or noncurrent.

## Reusing the original-game harness

Read upstream `tools/frida-live/README.md`, `LESSONS.md`, `RECIPES.md` before running it. `builds/15038592/run.bat` supplies tick `0x42780` (`Game::Autoplay::Manager::Update`) and turn hook `0x5c2d0`. It uses a persistent Frida daemon, authenticated local client, game-thread work queue, native RVA calls and runtime Lua bridge. Helpers support module rebasing because loading a save can reload the DLL.

These tools require a running Windows game with a loaded single-player save; they are not a standalone DLL loader or automatic snapshot restorer. Upstream observed detach crashes and stale Lua-state heap corruption. A repeatable harness should save/load each fixture, verify module identity and ruleset, run actions in their actual execution context, wait for a defined completion boundary, and record both observable state and RNG state/counts. Check all legality results, events, ordering, delayed removals and completion messages. Treat reference claims and fresh observations as separate evidence.

For isolated native execution, first prove the function's imported dependencies, calling convention, object layout and global references from the exact supplied PE. A function's short size and plausible name do not establish that it can execute outside the initialized game. Comprehensive exact end turns also depend on gameplay database, enabled expansions/mods, Lua callbacks and deterministic iteration; those are outside this public reference's contents.

## Importing the verified target into Ghidra

A later focused check found upstream's exact module fingerprint in `tools/frida-live/builds/15296837/README.md`: XP2 FinalRelease size **12,940,384 bytes**, SHA-256 **324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c**. The supplied local FinalRelease DLL matches that fingerprint. This establishes binary identity for the current RVAs; it does not strengthen any individual mapping category. The separately supplied XP2 Release DLL has a different identity and must not receive these target RVAs.

`tools/ghidra/import_reference.py` checks the loaded program hash, supplied module bytes, pinned TSV checksum and upstream identity statement before opening an annotation transaction. It creates labels in `Ref_civ6_<mapping-category>` namespaces, records full original names, addresses, old sizes and mapping evidence in comments, and stores source revision, license, module fingerprint and counts in the program's `Reference provenance` options. Recovered native signatures are attached as descriptive comments with an explicit unverified ABI warning; they are not applied as prototypes. Only auto-generated DEFAULT function names are renamed, using strong categories (`unique`, `unique-reordered`, `resolved`, `datashift-verified`). Existing imported or user function names remain intact. Derived categories receive separate labels; ambiguous/unmapped/fuzzy rows and non-executable addresses are rejected. Multiple folded names remain visible as labels/comments.

The import intentionally does not create Windows structures from Linux layouts or infer native prototypes from Lua wrapper types. Native signatures and data types require independent target-specific evidence. Stop the MCP server before running the standalone script against its project; alternatively the project owner can call `annotate(program, binary=...)`, save, and keep the same owning process. The script emits a JSON report including all applied rows; keep the large report in the sibling research project rather than committing it.

Actual project run on 2026-11-02 succeeded through the project owner's PyGhidra session after analysis and save, against the hash-matched FinalRelease program. It processed 43,803 rows, added **36,808 reference labels and comments**, attached **367 descriptive signature comments**, and rejected **6,995** rows by mapping category (3,595 ambiguous, 3,387 unmatchable, 12 none, one fuzzy-best). Of the applied rows, 28,927 were at existing function entrypoints and 7,881 were labels without existing functions; the script does not guess function bodies. The full report is `../c6-ghidra/evidence/reference-import-report.json`. Confidence namespaces contain: unique 20,894; unique-reordered 1,776; resolved 5,336; datashift-verified 15; callgraph-caller 3,013; callgraph-callee 2,224; vtable-derived 3,538; datashift-probable 12. Provenance and policy were stored inside the saved program. Ghidra may promote an imported label at an auto-generated function entry to its displayed name; meaningful pre-existing names are protected.
