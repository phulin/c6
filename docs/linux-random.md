# Linux isolated Random parity

The supplied Linux XP2 `GameCore::Random` now has an original-code oracle and fresh Rust comparisons. This covers isolated RNG primitives and the normal complete Random stream record. It does not establish initialized-game RNG ownership, save loading, legal actions or turn equivalence.

Run the concrete Linux differential entrypoint from the repository root:

```sh
python3 tools/linux/random_compare.py --output evidence/linux/random-parity.json
```

The runner builds `native/linux/random_oracle.c` as its own preload, starts the supplied `Civ6` executable through genuine CRT initialization, intercepts application main and loads the whole original GameCore. It gates every supplied runtime asset against `tools/linux/runtime-gate.json`, records resolved dependency hashes, compiler, adapter/source/configuration hashes, and builds then executes the existing `c6-rng-candidate` and `c6-serialization-candidate` process adapters. Their own executable/configuration identities are recorded. This new Linux evidence is independent of the adapters' historical Windows corpora. Missing assets, dependencies, original symbols, native crashes, refused supported fixtures, or mismatches fail the command; an unavailable oracle never counts as a pass. `--expected-configuration SHA256` additionally pins the complete native fixture configuration.

The recorded corpus contains 3,072 comparisons: 2,048 independent arithmetic/state boundaries, 512 operations interleaved across four retained original Random objects, and 512 original Read/Write comparisons using genuine host MemoryIStream/MemoryOStream objects. Boundaries include zero/one/maximum unsigned-short bounds, explicit u32-to-u16 adapter truncation, wrapping seeds and counters, lock overflow, unlock at zero, synchronous true/false, ignored version words, trailing input bytes, and raw noncanonical synchronous bytes in the codec. Every output and named state field is compared; native vptr/padding and every nonselected 32-byte Random object must remain unchanged. Constructor fields and original destructor/manager/context cleanup execute successfully. No Rust mechanic changes were needed.

| Target Linux method | Address |
| --- | --- |
| Write | `0x2d48a20` |
| Read | `0x2d48ab0` |
| Constructor | `0x2d48c10` |
| Reset | `0x2d48c30` |
| Initialize | `0x2d48cf0` |
| Get | `0x2d48d60` |
| Lock / Unlock | `0x2d48ea0` / `0x2d48eb0` |

The pinned [cru121 reference](reference.md) supplied the discovery lead. Actual target DWARF confirms object size `0x20`, seed/calls/resets/lock offsets `8/12/16/20` and synchronous byte `24`; original target instructions and native execution independently confirm the accessed fields. Private evidence is `../c6-ghidra/evidence/linux/xp2-random.txt` and `random-target-instructions.txt`; no proprietary dumps are committed here. The public corpus records only synthetic inputs, observations and hashes.

Get uses wrapping `seed * 0x41c64e6d + 0x3039`, increments calls even for a zero bound, and scales the high seed half by the unsigned-short bound. Reset increments only resets; Initialize clears calls/resets/locks while preserving synchronous. Unlock at zero leaves zero. Write emits an 18-byte record: version u32 `1`, seed/calls/resets u32, raw synchronous byte, zero legacy-vector flag. Read ignores version, consumes 18 bytes, preserves locks and retains the raw synchronous byte. Rust's logical bool API rejects noncanonical bytes atomically; its raw record codec preserves them. Native legacy-vector and failing/short stream behavior remain outside this supported comparison.

The fixture constructs original Random objects, then supplies synthetic field inputs for boundary testing. Four retained objects additionally test continuity without reinjecting fields. These object IDs are caller labels; they do not identify Civ VI gameplay streams or establish their allocation/names/lifecycle. The factory-only first Get probe failed with SIGSEGV because its logging global was absent. The working fixture explicitly binds a genuine host LoggingManager, constructed by original code at `0x323d970`, with file output disabled. Initialize may register logging metadata; logging callbacks, file behavior, game-turn logging context and its complete service state are excluded. Logger storage is oversized `0x4700`; original constructor instructions access through `0x46d0`. No logging stub or initialized-game stand-in is used.

`native_process_seconds` includes host startup and adapter I/O as well as RNG/codec execution. It is not a turn benchmark or a pure RNG throughput measurement. The full simulator objective remains open.
