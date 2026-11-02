# Linux RequirementSystem v3 wire codec

`c6_core::requirements` models the complete serialized RequirementSystem v3
section observed in the two canonical native Linux Core Saves. It is a typed
building block for future state support. Full gameplay state import, requirement
evaluation, legal actions and turns remain unimplemented.

The pinned GameCore identity is
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
Original writer `0x23f54c0` and reader `0x23f7d20`, matching DWARF and
[the prior typed GameEffects audit](../evidence/linux/save-normalization-gameeffects.json)
establish this little-endian contract:

| Field | Representation |
| --- | --- |
| Section version | u8, supported value 3 |
| Record count | u32 |
| Each ordered record | u32 name byte count, exact name bytes, i32 context ID, i32 subject ID, u8 raw RequirementState |

The original serializer owner trace attributes bytes
`[73,741,108, 74,930,233)` in both canonical 80,657,462-byte streams to this
section. Its 1,189,125 bytes hold 28,304 records. No pattern scan identifies
these records. This wire contract contains no Unit variant payload, so reusing
that unrelated variant codec would be incorrect.

Names remain byte vectors; the codec does not impose UTF-8 or ASCII semantics,
append terminators, sort records, reject duplicates or assign object IDs. States
remain raw bytes without asserting enum validity or evaluation semantics. The
codec rejects other section versions. There is no record type tag or variant
branch to reject within version 3.

Decode first validates the complete structure and aggregate allocation budget
without allocating records or names. The caller controls encoded byte count,
record count and combined record-array/name-byte allocation limits. The limits
are a resource policy, not inferred native gameplay caps. Encode preflights the
same limits and u32 count/length representability. Allocation failures return an
error; truncated or trailing input cannot produce a successful partial state.

The original reader resolves context/subject IDs through ObjectPool::FindObject,
obtains requirements, sets their state and holds temporary references while
modifiers load. None of those operations is implemented here. In particular,
wire records do not supply live object ownership, allocator counters, subscriptions,
rule definitions, RequirementSet state or ModifierSystem state. The observed Team
ID changes and their 28 requirement references remain exact, including their
original order. This codec neither remaps them nor explains the native allocator
sequence that produces the +24 change.

Reproduce the actual-record check with:

```sh
cargo build --release -p c6-core --example requirements_codec
python3 tools/linux/requirements_codec_audit.py \
  --canonical ../c6-ghidra/evidence/linux-game-load/canonical \
  --codec target/release/examples/requirements_codec \
  --iterations 200 --output /tmp/requirements-codec.json
```

The audit verifies canonical manifest module identity and both complete native
Save hashes, extracts only the previously attributed owner section, then compares
Rust record starts, field starts, references and states against the independent
prior Python decoder. Rust encodes the whole typed sequence back to identical
bytes. The check covers all 56,608 actual record occurrences, without preserving
unknown section bytes as a substitute for field decoding. Actual section
truncations, an unsupported version, hostile record count and a record budget
failure are also checked. Unit tests cover every truncation of a constructed
record, duplicate order, signed extremes, embedded NUL/non-UTF8 name bytes,
all 256 raw state bytes, trailing data and aggregate resource limits. These
negative cases are Rust API checks, not a claim of native malformed-input parity.

The optimized example reports elapsed nanoseconds per complete section decode
and encode. Decode includes structural preflight, allocation/copy and destruction;
encode includes allocation, writing and output destruction. It excludes native
Load, name resolution, object lookup, gameplay evaluation and save decompression.
Timing is diagnostic and depends on the machine and concurrent work.

[The fixture summary](../evidence/linux/requirements-codec.json) retains hashes,
counts, exact comparisons and measured timings. The original raw Save→Load
89,198-byte difference remains reported; no semantic state equivalence, complete
GameEffects coverage or full Core import/advance is established by this codec.

The recorded run used 200 iterations per fixture: 1.57–1.62 ms to decode and
0.403–0.405 ms to encode one complete section. Both sections retain distinct
SHA-256 identities. `cargo test -p c6-core requirements` passes four tests and
`cargo clippy -p c6-core --all-targets -- -D warnings` passes. No new native
execution was needed; the oracle bytes and serializer provenance are reused from
the earlier hash-gated original-code captures.
