# Linux startup provider contracts

This is bounded static evidence for supplied Linux host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. It explains original provider ordering and the current native fixtures; it does not establish initialized state/action/turn equivalence. The host has dynamic symbols but no host DWARF claim. Persistent targeted decompiles and annotations live privately in `../c6-ghidra/evidence/linux/ghidra-*-startup-*.json`, `ghidra-startup-*.json` and `ghidra-content-local-helpers.json`.

Run `python3 tools/linux/startup_contracts_audit.py --output evidence/linux/startup-contracts.json` to verify the pinned ELF, ten function entries and function-byte hashes, nineteen original direct CALLs, and ten relevant literals. The audit uses ELF load-segment virtual addresses, independently of a Ghidra image base. These are static checks, not passing native startup tests.

## Original provider ordering

`AppHost::RunApp(int,char**,Application*)` at `0x42b99f0` initializes the real environment and options, invokes the application override, calls `SetupFXSPlatform` at `0x42b4ad0`, then dispatches application `InitialInit` through virtual slot `+0x18`. Later it creates/manages a window and enters GUI initialization and the message loop. It also performs display, accessibility and OS compatibility calls before the platform setup. It is not an established offline/headless startup entrypoint.

The environment is embedded at application `+0x8`; options are embedded at `+0x880`. Original instructions `0x42b9aa3`/`0x42b9aa7` bind actual CRT `argc`/`argv` at application `+0x30`/`+0x38`, corresponding to environment `+0x28`/`+0x30`. The environment constructor does not initialize these fields. The native fixture must bind the real process inputs before original methods consume them; zeroed or forged arguments are not evidence of correct original startup.

`Civ6App::OverrideAppOptions` begins at `0x2f8c990`; `0x2f8cd60` is an interior assignment, not a callable function entry. RunApp initializes the temporary heap option to `0x1000000` bytes, and the override also stores this value at application `+0x890` (options `+0x10`). This proves the original 16 MiB default; it does not recover the complete AppOptions layout or size.

`SetupFXSPlatform` reads real option buffers and environment version data, hashes host/time information, configures logging and conditional bug collection, then performs:

| Original call | Condition and inputs |
| --- | --- |
| `PerfEvents::Initialize`, call `0x42b4d7a` | Unconditional in this function. |
| `GetTempHeap` / `TempHeap::Startup`, calls `0x42b4d86` / `0x42b4d94` | Options `+0x10` is nonzero; uses that byte count and null optional storage. |
| `JobManager::Initialize`, call `0x42b4dcf` | Both options `+0x4` and `+0xc` are nonzero; descriptor uses the former worker count and flag 1. |

The final helper `0x42b4fa0` gathers CPU, physical-memory and laptop/tablet information and publishes a telemetry event. The original configuration permits skipping JobManager through its condition. Current isolated native fixture success without PerfEvents does not prove PerfEvents optional for general startup or gameplay. `TeardownFXSPlatform` at `0x42b5260` shuts down PerfEvents, destroys JobManager, then releases/resets the original process-global temporary heap. See [the temporary heap contract](../evidence/linux/mod-activation-tempheap-contract.json) and the native owner implementation; do not free that arena before its dependent owners and temporary allocations are released.

`Civ6App::InitialInit` at `0x2f8d120` crosses EOS setup and graphics-device selection before the useful provider stages. Its original path orders `RegisterEvents` (`0x2f8dff7`), `SetupNetwork` (`0x2f8f288`), `SetupDatabase` (`0x2f8f4a2`), `SetupLocalization` (`0x2f8f8a0`), `SetupContentPaths` (`0x2f8fcf9`), then `SetupOwnership` (`0x2f8ff57`). Later it starts the external Firaxis Live client at `0x2f9081d`. These are call-site ordering facts along the original gated path; they do not mean every provider is required for every narrower fixture. Whole InitialInit is not an established offline/headless wrapper.

## Genuine content paths

`Civ6App::SetupContentPaths` at `0x2f92a20` reads application option section `Language`, key `AudioLanguage`, and calls whole `App::ContentPath::Local::Startup` at `0x2f4f920` with the real environment, real options and resulting language string. It returns true without inspecting the delegated result. The original Local startup owns this sequence:

1. Allocate 0x40 bytes, run the genuine Manager constructor and publish it through `Manager::Set`.
2. Run original helper `0x2f4fa10`, which uses the real base File catalog to scan `../../../Base/Assets` excluding `*/Scenarios/*`, then `../../../Base/ArtDefs` and `../../../Base/Platforms/Windows/ARX`.
3. Run helper `0x2f4fa90`, which builds real content definitions for the supported platform/format pairs and language-specific audio paths.
4. Set default platform/format `(2,2)` and process actual command-line overrides. The original switches are `-PlatformContent` and `-EnableDevFeatures`.
5. Add the real package `Base` rooted at `../../../Base` with package type 0, then call `Manager::ApplyChanges`.

The `Windows` directory names above are literals in the supplied Linux ELF. Their spelling is not transferred Windows evidence. A Linux source/launch view must supply the original catalog paths, case and relative working directory. Publishing a handmade Manager or registering a substitute package list does not reproduce whole Local startup.

The prior native crash at `SetupCommandLineOverrides+0x43` (`0x2f4f493`) came from dereferencing the environment's unset `argv` while its unset `argc` was positive. That is a missing RunApp input binding, not proof of a broken content catalog. The preceding native temporary-allocation failure was independently traced to the missing genuine 16 MiB TempHeap startup; see the separate contract. These are fixture prerequisites, not simulated game behavior.

`ShutdownContentPaths` at `0x2f986d0` calls `Manager::Set(NULL)`. Actual `Set` at `0x2f4e610` destroys and deletes the previous manager before publishing its new pointer. Calling it with the current manager pointer would therefore publish a freed object. The manager and its providers must outlive mod/framework operations that consume them; shut them down once after dependent framework/controller teardown, then release the platform arena after all owners are done. The fixture adapter's current cleanup order is separately validated by its native lifecycle evidence; this static document does not substitute for that check.

## Remaining acceptance gaps

The reproducible audit verifies target bytes and the bounded graph. Successful original provider startup, native package discovery, exact configuration/DLC selection, gameplay database exports and full initialization still require their separate fixture identities and observed state comparisons. None of these static contracts establishes complete save loading or advancing a Civ VI turn.
