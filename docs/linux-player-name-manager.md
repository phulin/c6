# Linux Player.NameManager current wire

`player_name_manager::NameManager` transports the complete current v2
`GameCore::Player::NameManager` record from the pinned Linux XP2 GameCore
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
This is a bounded wire codec. Complete Player loading, initialized gameplay
state, action execution and turns remain unimplemented.

The original writer at `0x2b54eb0` (104 bytes) emits:

| Order | Wire value | Meaning |
| --- | --- | --- |
| 1 | u32 little endian, 2 | Current version at `0x2f65e6c` |
| 2 | u32 little endian, n | Native vector length |
| 3 | u32 little endian, n | `WriteArray(short)` length |
| 4 | n signed i16 little endian | Citizen name usage counts, in vector order |

Matching target DWARF identifies `m_aiCitizenNameUsageCount` as an
`FAutoVariable<Types::Vector<short>, Player::NameManager>` at object offset
160, with the vector value at 176. No names, localization records, bools,
definition hashes, enum values, maps or pointers occur in this wire record.
Repeated values and all signed short bit patterns are retained.

The array provider at `0x2d50920` independently writes its count and raw short
payload. Its concrete PLT relocation and the caller are checked by the audit;
the second count is not inferred from fixture bytes. The reader at
`0x2b54f20` reads a vector count and calls `ReadArray(short)` at `0x2d50680`.
The canonical codec requires both counts to agree and rejects counts above
`i32::MAX`, outside the nonnegative native reader count domain. It deliberately
does not reproduce malformed native capacity/array-count mismatch behavior.

The reader also contains historical v1 handling that consumes and discards an
additional short array. This codec rejects v1 and every version other than 2.
After reading, native `PadNameUsageCount` at `0x2b55040` consults the player's
configuration, civilization definition and major-player status and may append
zero counts to match the civilization's citizen-name collection. The wire
codec performs no such live fixup and asserts no constructor defaults.

`decode` rejects trailing data. `decode_prefix` returns the consumed size;
`decode_prefix_with_items` returns `(value, bytes, items)` and
`encode_with_items` returns `(bytes, items)`. One vector entry is one item.
Both `unit::Limits` budgets cover the entire record. Checked sizes, the byte
budget, item budget and input availability are verified before fallible vector
allocation. Encoding preflights the complete output before allocation.

The original Player writer at `0x2b4ee00` orders Districts, NameManager,
Culture, with calls at `0x2b4f04e`, `0x2b4f061`, `0x2b4f074`. In each of the
two canonical original Core streams, the first NameManager starts at
4,988,302 and consumes 92 bytes (40 counts), ending at 4,988,394, where the
next schema is Player.Culture. These are observations, never parser offsets.
Culture's writer is `0x2a6e330` (1,357 bytes) and reader `0x2a6e880`
(3,414 bytes); Culture remains unsupported by this codec.

Run the read-only audit with:

```sh
python3 tools/linux/player_name_manager_audit.py --output evidence/linux/player-name-manager.json
```

It gates both original ELF hashes, checks full method fingerprints and concrete
PLT JUMP_SLOT symbol bindings, verifies current-version data and matching DWARF,
then builds an isolated Rust harness. The original binaries are never executed.
The existing canonical Core and native owner traces are hash gated; each
Districts end and following Culture start delimit a validation-only NameManager
record. All 43 records in each original stream roundtrip byte exactly, including
empty and nonempty vectors. Source-only tests cover signed ordering and repeated
values, exact budgets, allocation preflight, unsupported versions, inconsistent
counts, truncation and trailing data. Artifact identities and record observations
are recorded in the public evidence JSON. The audit writes no private artifacts.

This does not establish native Read fixups, full Player/world import, gameplay
state or turn equivalence. Unknown save bytes remain unclassified and unmasked;
the recorded raw Save→Load failure remains 89,198 bytes.
