# Linux isolated RNG variants

The component port covers XP2 `GameCore::ScopedRandom` and matching Linux host `Math::RandomPCG32` integer operations. These are isolated components, not a complete gameplay RNG inventory or stream lifecycle. Complete state, action execution and turns remain unimplemented; no AI-named state or unknown save bytes are excluded by this work.

`ScopedRandom` has exactly one u32 seed. Construction/reset replace it. Get advances with wrapping `seed * 0x41c64e6d + 0x3039`, then returns `((seed >> 16) * u16_bound) >> 16`; bound zero still advances. The library accepts the actual u16 argument. The differential adapters separately exercise original unsigned-short call conversion from full-width inputs.

Host PCG32 preserves both u64 words, state and increment. Its default constructor copies `(0x08f5dc87e5c07d8a, 3)`. Its seeded constructor and Seed set state to wrapping `u64(u32_seed) * 0x5851f42d4c957f2d + 0x08f5dc87e5c07d8a`, and increment to three. Rand advances state with that multiplier plus the existing increment and permutes the old state as `rotr32(u32(((old >> 18) ^ old) >> 27), old >> 59)`. Raw imported even/zero increments are preserved, not normalized. The algorithm was read from the actual pinned Linux bytes; a generic PCG implementation or host PRNG was not substituted.

| Method | Original module | Address |
| --- | --- | --- |
| Scoped constructor / Reset | XP2 | `0x2d4d030` / `0x2d4d040` |
| Scoped Get / GetSeed | XP2 | `0x2d4d050` / `0x2d4d070` |
| PCG default / seeded constructor | Civ6 host | `0x3c087b4` / `0x3c087c0` |
| PCG Seed / Rand | Civ6 host | `0x3c087ea` / `0x3c08816` |

Target identity and original method hashes are recorded in `evidence/linux/random-variants-contracts.json`. Private instruction/DWARF evidence is under `../c6-ghidra/evidence/linux-rng-inventory`; no proprietary dumps are published. The host is an ET_EXEC image: its listed addresses are absolute virtual addresses, whereas XP2 addresses are module offsets.

The accepted original-code run in `evidence/linux/random-variants-parity.json` compares 3,986 unique cases with zero value/state mismatches in both fresh Rust debug and release builds. Those builds produce identical observations; they do not count as additional cases. Native exit is zero. Rust core tests (45), clippy and strict C adapter compilation also pass. The complete configuration identity is `bc28bb83c37d3482857a6e50b8a120ea3dc44e75b6c4617d6a0374f362da31cd`.

The reproducible native runner is:

```sh
python3 tools/linux/random_variants_compare.py --output evidence/linux/random-variants-parity.json
```

Acquire the shared native execution lease before running it. It builds its own preload and fresh Rust example, hash-gates every runtime asset, records resolved dependencies/compiler/source/candidate identities, enters the original host through its CRT, and loads original XP2. Original constructors, reset/seed, Get/GetSeed and Rand execute without logging stubs, invented service globals or an initialized game. A mismatch, missing dependency, refused case, original-code failure or incomplete output fails the command. `--expected-configuration` additionally pins the complete fixture identity.

The 3,986-case corpus includes wrap boundaries, every PCG rotation, full-width raw increments and 1,032 retained interleaved operations across four ScopedRandom plus four PCG objects. Every returned value and all state words are compared. Result records have a unique prefix and sequential case ID; the runner enforces exact field/count/order checks and separates the known original host startup diagnostic. Complete native stdout/stderr and their hashes are preserved. The native adapter also checks every nonselected object and the other RNG family remain unchanged. Python's RNG selects fixture inputs only. Process time includes host startup and adapter I/O and is not a gameplay or pure RNG benchmark.

The first runner rejected 3,987 total stdout lines for 3,986 cases before comparing arithmetic, because it counted the original host startup diagnostic as a result. `random-variants-attempt-1.json` preserves that acceptance failure; raw output was not captured by that first runner. Attempt two captured complete output and matched all observations; its report and raw output remain under `random-variants-attempt-2.*`. The final accepted run adds unambiguous sequential framing. These parser changes did not change component arithmetic or state transitions. Its measured 0.296 seconds includes startup and I/O.

The bounded Lua GetRandNum wrapper, its zero-bound post-draw trap, real-time Lua seeding, fRand floating conversion, gameplay stream ownership/allocation and actual save records are outside this component port. A future Lua seed must enter as the real exogenous input; it must not be replaced with a constant. These coverage gaps do not exclude their gameplay effects from the full objective.
