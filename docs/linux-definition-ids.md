# Linux definition identity and lookup

The intended simulator still covers isolated primitives. The Rust
[`registry`](../crates/c6-data/src/registry.rs) module implements a bounded Linux
XP2 string-hash and collection-lookup contract. It accepts an explicit native
collection order; it does not infer runtime IDs from the rules export, initialize
definitions, implement yield gameplay or establish complete state/action/turn
equivalence. No `RulesRowId` to native DefinitionID conversion is supplied.

Discovery started with cru121's pinned
[hash reference](https://github.com/cru121/civ6-gamecore-reference/blob/5af389445118bed2bdebf7a8a42b219c8fba5175/findings/topics/events-hashes-and-data-symbols.md)
(CC BY 4.0). That reference identifies `~crc32(NAME)` but explicitly had not
separately disassembled `MakeHash`. The conclusions below use the independently
gated supplied Linux GameCore SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and matching host SHA-256
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
[Static evidence](../evidence/linux/definition-ids-static.json) fingerprints
complete original dynamic-symbol function extents. Raw target disassembly and
DWARF remain private under `../c6-ghidra/evidence/linux-definition-ids/`.

## Three separate identities

| Value | Exact observed role |
| --- | --- |
| database rowid | Native `Yield::m_DatabaseRowId`, signed 32-bit at offset 16; populated from the query's first column. Rust SQLite rowids remain signed 64-bit values. |
| collection index | Native unsigned 32-bit at offset 20; assigned the current collection size immediately before append. It is an ordinal, not the database rowid. |
| type hash | Native unsigned 32-bit at offset 24; computed from `YieldType` bytes and used in saved yield arrays. |

`YieldCollectionBuilder::GetBuilderSql` (`0x221f970`) returns exactly:

```sql
SELECT rowid, DefaultValue, IconString, Name, OccupiedCityChange, YieldType FROM Yields ORDER BY rowid
```

`BuildCollection<YieldCollectionBuilder>` (`0x2260c00`) calls `BuildDefinition`
for each query row, sets the collection index to `size()`, then appends.
Thus the importer provides concrete ordering evidence for this collection.
This does not turn arbitrary fingerprint ordering or SQLite rowids into runtime
IDs, establish the active rows in a particular initialized context, or prove
that rowid equals index plus one in every profile. `BuildDefinition`
(`0x221f850`) narrows the query rowid through its signed 32-bit getter.

The exact Linux DWARF gives `Yield` size 160 and its collection size 48.
The collection owns an append-order pointer vector at offset 0 and a separate
hash-sorted pointer vector at offset 24. `push_back` (`0x231a1a0`) appends to
the first vector and inserts at unsigned-hash `lower_bound` in the second.
Equal-hash insertion precedes previous entries; lookup selects the first equal
hash, so the newest inserted colliding definition wins. No type-string equality
check disambiguates collisions in this method. This is static target behavior;
no colliding mod fixture has been executed.

## Hashing and lookup boundaries

`Utilities::MakeHash(char const*)` (`0x2d577c0`) obtains the CRC object initialized
with polynomial `0xedb88320`, then calls host `Crypto::CRC::CalcString`
(`0x39508ee`) with initial accumulator `0xffffffff`. The host consumes unsigned
bytes until NUL and returns the accumulator without a final xor. There is no
case conversion: uppercase identifiers in the reference are input conventions.
The empty string hashes to `0xffffffff`. Embedded NUL terminates input; a null
pointer is not a valid `MakeHash` argument. Separately, `Yield::SetYieldType`
(`0x221f810`) handles a null pointer by clearing the name and storing hash zero.
Those are distinct contracts.

`Find(u32)` (`0x1ff7290`) first interprets any argument below the collection count
as a direct collection index. Otherwise it searches the hash-sorted vector using
unsigned comparison. A miss returns null. `FindIndex(u32)` (`0x2b35520`) returns
the selected definition's collection index, or signed `-1` on a miss.
`At(u32)` (`0x1fe47a0`) accepts only a numeric index and returns null at/beyond
count. The Rust API returns `Option` for these missing results; it keeps the
low-value numeric shortcut and collision precedence.

The yield array reader (`0x1ff7160`) resolves serialized hashes against the
current registry, writes known entries at their collection indexes and consumes
an unknown hash's fixed-point payload into a temporary. The wire codec preserves
hashes and values without pretending to know the active registry. It must not
substitute exported database rowids when gameplay begins using those values.

Units uses the same `DefinitionCollectionWithTypeIndex` mechanism. Its original
builder (`0x225eec0`) likewise sets index from current size before append, and
its builder SQL (`0x221c190`) explicitly orders by rowid. This is a bounded
mechanism check, not native validation of all Unit IDs or every definition kind.

## Reproduce and native verification gap

```sh
python3 tools/linux/definition_ids_audit.py \
  --output evidence/linux/definition-ids-static.json
cargo test -p c6-data registry::
cargo clippy -p c6-data --all-targets -- -D warnings
cc -std=c11 -Wall -Wextra -Werror -fPIC -fsyntax-only native/linux/definition_ids.c
```

The optional [`native observer`](../native/linux/definition_ids.c) is intended
for the existing hash/configuration-gated Linux runner after native definitions
were built, between two original canonical core Save captures. It calls original
getters, `MakeHash` and `FindIndex`, emits each yield's ordinal/rowid/name/hash,
and checks index zero, `At(count)`, an actually absent hash, empty input and case
sensitivity. It does not initialize cache managers or mutate registry entries.
Its caller must gate the exact binary and complete source/configuration identity.
The native hasher is lazy; importing named definitions already initializes it.

The audit supports `--runtime-report`, `--before-save` and `--after-save` to bind
the observer output to its runtime source identity and require byte-identical
canonical core captures. This establishes that observed serialized boundary,
not unchanged entire process memory or all unrecorded live state. A source
identity from a saved-context run must not be silently relabeled as the registered
request-None diagnostic profile.

At this checkpoint native registry-query execution and active-profile mapping
remain pending. The Rust tests are meaningful hash/input/lookup boundary fixtures
and do not count as original-native runtime parity. Current rules export coverage
remains 506 tables / 43,565 rows for the activated diagnostic profile; its row
fingerprints alone establish no initialized native registry. No lookup benchmark
is reported: any future timing must exclude game startup and turn/gameplay claims.
The complete raw Save→Load comparison still includes the recorded 89,198-byte
failure; this work does not classify, mask or normalize unknown save bytes.
