# Original Linux save-load lifecycle

The original Linux host has an encompassing staged loader: `LoadGameViewState::LaunchNextStage(unsigned)` at `0x2fd2180`. Calling Context::Load directly executes a lower serialization boundary and omits this orchestration. The contracts below are verified against pinned original Linux ELF bytes, not copied from historical platform layouts. They do not establish that a complete game has loaded.

Run the reproducible, read-only audit:

```sh
python3 tools/linux/save_lifecycle_audit.py --output target/linux/save-lifecycle-contracts.json
```

The tool checks the exact original host and XP2 SHA-256 gates, 22 native direct calls, the 12-entry original stage jump table, original immediate arguments, and relocated target Context virtual slots. [save-lifecycle-contracts.json](../evidence/linux/save-lifecycle-contracts.json) records this static checkpoint. Raw disassembly is private under `../c6-ghidra/evidence/linux-save-lifecycle`.

The native loader stores its current stage at its own object offset `0x440`. The argument to LaunchNextStage is not the stage selector. These are original observed fields, not permission to fabricate a view-state object or write its stage manually. Original constructor, Begin, synchronous/asynchronous progression, Tick and End own the object and its surrounding UI, art, event, tuner and application dependencies.

| Original stage | Observed operation |
|---|---|
| 0 | Initialize error results and advance to 1. |
| 1 | Request type 2 selects stage 4; other requests select stage 2. |
| 4 | `Serializer::Manager::LoadCurrentRequest`, then stage 5 when result is OK or PENDING. |
| 5 | Wait for request completion; on success `ApplyModdingState(nullptr)` uses the retained original stream, then stage 2. |
| 2 | Pre-content automation, configuration ownership checks, ConfigureContent and UpdateContent; successful path selects stage 3. |
| 3 | Apply original gameplay options; select stage 6 without a retained stream or stage 9 with one. |
| 9 | Original Context StartupGame, serializer Unserialize, controller cache and postinitialization work; successful path advances to 10. |
| 10 | Apply pending user settings, choose active user from the original local-player cache, reload tuner states, then advance toward 11. |
| 11 | Remaining game-view/UI startup and completion flow. Full execution is still required to validate its prerequisites and completion. |

Stages 6–8 belong to the alternate new-game path. The supplied target has different stage numbering from the upstream reference's documented loader, so upstream stage IDs cannot be reused.

Stage 9 calls the Context virtual at offset `0x40` before unserialization. The target's actual ELF relocation identifies it as original `Exports::Context::StartupGame` at `0x2c7d2f0`. It sets the controller's initializing flag and disables event-stream cache updates before loading. After an OK result it calls SetContext and InitializeCache, original automation PostGameInitialization, and Context virtual `0x160`, proven by target relocation to `PostInitialize` at `0x2c7d0c0`, with argument true. It later updates cache components, restores visual state, reenables cache updates and publishes the native GameCore initialized event before stage 10. Those hooks are not interchangeable with merely returning from Load.

Stage 10 contains the only direct original host call to User::Instance::ApplyPending found by scanning executable instructions. Its argument is **true**, verified at caller `0x2fd2e7f`. Unless WorldBuilder editor mode is active, the original sequence then calls Cache::GetLocalPlayer and passes that result to User::SetActive. It does not select player zero because a save contains player-zero user settings. Cache::GetLocalPlayer returns `-1` when the original game cache is absent; otherwise it delegates to that cache's original virtual provider. Pending settings must therefore remain pending until the original postload lifecycle reaches this boundary with a valid cache.

UpdateContent takes the original libc++ `std::function<void(Database::IDatabaseConnection1*,bool)>` by value through its C++ ABI; its Linux call-site argument points to the native function object. The loader constructs a genuine callback capturing its owning LoadGameViewState. Its invocation ultimately calls original `App::Censorship::ApplyContentCensoring(Database*,bool,bool)`, with the second bool read from that native owner. UpdateContent checks the function object's presence before calling it; an absent callback is supported by that function's bytes. This does not prove that omitting the loader's supplied censorship callback preserves the benchmark configuration. An invented libc++ representation or a replacement callback is not a verified fixture.

`LoadCurrentRequest` opens the source using the serializer's real request filename/location. Its callback retains the original stream and SaveType, signals stream retention to OpenFromSource, and preserves ownership until ClearLastStream invokes the original deleting destructor. ApplyModdingState reads LoadHeader then calls stream SetOffset(0); the native host MemoryIStream vtable resolves that seek slot to its actual original method. `Unserialize(default,nullptr)` subsequently uses the retained original stream and type. It expects the **complete save container**, since UnserializeGameState starts with LoadHeader and then AttachToSession, LoadGameCoreState through a genuine ZLibIStream, LoadAudioState, LoadUIState and LoadRecorderState. The extracted GameCore payload used by the lower Context::Load probe is not a substitute for this input. RequestLoad's optional IStream and SaveLocation contracts must be established before substituting an explicitly supplied native stream for normal source opening.

Matching target DWARF defines serializer Result as a four-byte signed enum: OK=0, PENDING=1, FAILED_TO_READ=5, UNEXPECTED_END_OF_FILE=9 and ERROR_RESTORING_STATE=10. The optional native `c6_linux_save_lifecycle_check` checks only the missing-retained-stream boundary: `Unserialize(0,nullptr)` returns FAILED_TO_READ, leaves the request result and retained pointer unchanged, and reports the original local-player query. It never applies pending settings or changes player selection. [save-lifecycle-native-boundary.json](../evidence/linux/save-lifecycle-native-boundary.json) records native result 5, unchanged request state, absent retained stream, original local player -1 and complete fixture teardown with exit zero. Its successful execution is a negative serializer fixture, not a complete load.

A faithful next integration should let the genuine AppModdingFramework own its bootstrap and replacement contexts, then use the original loader's lifecycle. Manual startup of an independent Context cannot be combined with framework bootstrap without establishing ownership. Complete native Load/Save roundtrip, finalization, populated target rules/DLC identity, valid local-player cache, legal actions and turns remain unverified. The [saved-configuration comparison](linux-saved-configuration.md) retains its observed normalization differences separately.
