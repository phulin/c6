# Contextual Core saved-wire candidate

`c6-core-contextual-candidate` is an offline adapter for the production
`core_player::initialization::capture::admit_capture` API. It never executes the
original game. It checks supplied capture consistency, then imports each
distinct admitted stream role from byte zero through Player.Manager, GlobalAi,
Barbarian, FreeCities and the serialized sync Random record. Native execution
attribution remains a separately reviewed claim. The immutable v1 profile
stops after GlobalAi; v2's later prefix has a separate source/binary identity.

Build from a single committed archive, preserving the executable and all source
identities before supplying genuine capture artifacts:

```sh
cargo build --release -p c6-differential --bin c6-core-contextual-candidate
c6-core-contextual-candidate artifact-paths.json > result.json
```

The path-only JSON manifest requires these string fields:

- `observations_jsonl`, `runtime_plan_json`, `static_plan_json`
- `prepared_report_json`, `exit_json`
- `target_module`, `target_host`, `input_save`, `input_core`
- `outputs`: object mapping actual original output IDs to Core artifact paths
- `source_files`: object mapping the exact source names in the frozen runtime
  gate to their exact frozen source artifact paths

Paths are resolved relative to the manifest. The Save container and incoming
Core are separate inputs. Output names must match the original output binding;
pointer equality or matching hashes do not merge stream roles or lifetimes.
The source map includes the original runner source. Source names are identities,
not path aliases: a runtime gate using absolute names requires those same names,
even when the artifact copies are stored under repository-relative paths. A separate finalized
artifact manifest can retain size/hash metadata and be translated to this path
view without changing any closed capture artifact.

The manifest is limited to 1 MiB; artifact count, individual size and aggregate
size are bounded before loading. The admission API applies its independent
limits and validates source/module identities, raw marker bytes, event/frame
ordering, vector and query semantics, stream lifetime and finalization. The
byte-zero wrapper verifies each full immutable Core image once, then passes
structurally derived absolute Military cursors and remaining byte/item credits
to the contextual decoder. It does not rerun initialization or native Read.

Each role's complete Player.Manager and GlobalAi wire re-encoding must match
its own source bytes and dynamic-item charges exactly. Output preserves the
role, occurrence, actual Core digest, consumed offsets, borrowed-tail digest,
ordered player identities and imported roster slots/Unit IDs. No unknown tail
is skipped to manufacture a complete import; the next owner in v2 is Trade.Manager.
Version 2 additionally reports the decoded NPC instance counts and raw sync
Random fields. Its retained roundtrip diagnostic is explicitly scoped to
Player.Manager/GlobalAi; it is not a production save writer or a gameplay gate.
No bytes are masked or normalized. Existing raw Save-to-Load differences remain
unresolved unless a separate source-derived transition explains them.

This profile reports `native_acceptance=false`,
`initialization_reconstructed=false`, `native_read_effects_reproduced=false`,
`complete_world_import=false` and `gameplay_parity=false`. A successful result
establishes bounded saved-wire composition conditioned on the supplied,
consistent framing facts. It does not establish complete live World state,
gameplay RNG, action or turn behavior. Low-level Military framing remains an
interim explicit input until faithful Rust initialization supplies it.
