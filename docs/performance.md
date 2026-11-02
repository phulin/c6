# Integration and primitive performance

The current library executes isolated, native-verified RNG and coordinate primitives. It cannot execute complete games or turns. These measurements establish a reproducible primitive baseline; they do not measure game throughput, native game speedup, search quality, or a complete rules engine.

## Reproduce

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./tools/check/c_abi_smoke.sh
C6_BENCH_DRAWS=200000000 cargo bench -p c6-core --bench rng
```

The C smoke script builds `c6-ffi` in release mode, compiles a C11 consumer against `include/c6.h`, links the actual shared library, and runs it. It checks ABI structure sizes, null/error behavior, unchanged state and outputs on failed actions, the native-verified draw sequence, exact snapshot roundtrip, retained nonzero identity bytes and opaque future sections, independent state after import, numeric stream routing and missing/duplicate stream errors, counter wrapping, and legacy snapshot v1 upgrades to v2. This catches header/export/linkage discrepancies that Rust-only tests cannot.

The script currently targets a Linux linker and `.so` loading. Rust tests cover implementation behavior; a real C consumer still needs running on each intended ABI/platform. Callers must satisfy the header's pointer lifetime, alignment, ownership and synchronization contracts. The C smoke deliberately does not pass dangling or misaligned pointers: these violate that contract rather than represent recoverable input errors.

## Recorded baseline

Measured on 2026-11-02, Linux x86-64, shared KVM VM reporting AMD EPYC 7642, rustc 1.98.1 (LLVM 22.1.8), workspace release profile with thin LTO and one codegen unit. No CPU isolation or frequency control was applied. Repeated process runs, not a statistical confidence interval:

| Draws per run | ns/draw, run 1 | run 2 | run 3 |
|---|---:|---:|---:|
| 20,000,000 | 4.852 | 4.860 | 3.438 |
| 200,000,000 | 3.632 | 3.491 | 3.582 |

All three 200-million-draw runs produced checksum `6553447264242`, final seed `2242362488`, and call count `200000000`, starting at seed `0x12345678` and using bound `65535`. The shorter runs produced checksum `655546658011`, final seed `399945080` and call count `20000000`. The shorter series overlapped other workspace compilation and shows why an isolated minimum should not become a throughput promise.

After the numeric stream API and snapshot v2 extension, the same 200-million-draw command measured 3.543 ns/draw with the same checksum and final state. This is a regression observation for the primitive loop; named-stream lookup and action dispatch are outside its timing. The earlier repeated measurements remain the historical baseline.

The timed loop calls the Rust RNG primitive, uses `black_box` for the bound and result, and accumulates a checksum. It includes loop, optimization barrier and checksum overhead. It excludes library loading, C ABI calls, snapshot cloning/encoding, allocation, JSON differential transport, original DLL execution, logging callbacks, and all game-state mechanics. The dependency chain between draws is intentional: each draw consumes the preceding seed. Geometry is covered by differential checks but has no recorded benchmark here.

## Review boundaries

The RNG multiplication/addition and counters use explicit wrapping arithmetic; the high-word product fits in `u32`. Zero bounds consume state. The Rust primitive accepts a `u16`; the differential adapter records raw `u32` and effective `u16` bounds, while the public C action API rejects oversized draw arguments. This intentional API distinction prevents an accidental caller truncation from being hidden.

Snapshot bytes describe a simulator envelope, not a native C++ object layout or complete Civ VI save. They preserve unknown sections and identities without pretending to simulate them. The zero-identity constructor denotes a primitive sandbox. A future game-state integration must require expected binary/rules identities before evaluation.

Native oracle execution is restricted to the pinned DLL and isolated functions with explicitly described synthetic object fields and callback stubs. Its declared configuration must bind the adapter and native host source/build contract as well as RVAs and layouts; otherwise a stale host can silently execute under the same identity. Local source/build checks now fail closed on that mismatch. These are trusted local developer artifacts, not a signed execution attestation; compiler provenance and the DLL hash/reopen race remain outside that guarantee.

Future performance claims should benchmark an actually implemented workload after its state and RNG traces match native execution, record CPU/compiler/configuration, preserve checksums and final state, and report repeated timings. Complete-turn or game throughput requires complete-turn or game coverage first.
