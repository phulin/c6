# Linux movement definitions and typed-wire comparison adapter

The [data adapter](../crates/c6-data/src/movement.rs) bridges observed native
movement definition collections into the independent
[core resolver](linux-movement-world.md). The new
[world candidate](../crates/c6-differential/src/bin/c6-movement-world-candidate.rs)
and [checker](../tools/linux/movement_world_check.py) are separate from the frozen
first-run scalar candidate. They never execute game code. Complete gameplay state,
actions and turns remain unimplemented; no native world-resolver acceptance is
claimed. The raw **89,198-byte** Save→Load difference remains unmasked.

## Reusable data API

`LinuxDefinitionHashRegistry::from_native_order` retains original observed type
hashes without inventing names or database identities. It shares the exact lookup
implementation with `LinuxDefinitionRegistry`: direct indexes below collection
count, otherwise hash lookup, last inserted definition wins equal hashes. SQL row
IDs and SQL row order never enter the adapter. Both registry types implement
core's `DefinitionLookup`; core has no dependency on c6-data.

`NativeMovementExports::from_records(&[RegistryExport])` requires exactly one
terrain, feature, improvement, route and district collection. It validates count
and actual `At(query)`/`GetCollectionIndex` order; a noncanonical observed mapping
is unsupported rather than sorted into an invented order. Hash-only exports
already support Unit bool-vector import and wire-to-runtime identity comparison.
`complete()` additionally requires original terrain integer cost and IsWater,
feature/improvement integer costs, original route binary32 bits and district
FreeEmbark. Missing values produce typed errors naming kind, index and property.
`NativeMovementDefinitions::as_core()` borrows native-order property slices; it
builds no world copy and performs no SQL conversion. The caller still gates the
original module, actual rules/DLC/mod profile and input identities.

**The cb6113b first-run export omits Terrain::IsWater.** Its native Plot IsWater
answers cannot replace that missing definition property. The adapter does not
fill it from terrain names, water hashes, SQL values or case answers. A later
original property observation/export needs its own reviewed source/build identity;
the first-run source, helper, candidate and corpus must remain immutable. Complete
property comparison requires the separately reviewed additive header revision
`xp2-map-and-terrain-water-2`; mere insertion of a water field into an old corpus
is not accepted. The
existing optional `is_water` field is an adapter capability, not proof of a native
export that has not occurred.

## Independent candidate and checker

Prepare/freeze the separate candidate and run four explicitly source-only test
groups:

```sh
python3 tools/linux/movement_world_check.py --output target/linux/movement-world-adapter.json
```

The [prepared report](../evidence/linux/movement-world-adapter.json) records all
compiled core/data boundary sources, the separate candidate hash and immutable
path and exact immutable source/embedded-input bytes. An isolated build records
the expected first-run scalar hash without reading or rebuilding that artifact. Constructed source-only
checks exercise native order, duplicate hashes, direct-index/hash distinction,
raw route NaN bits, missing IsWater and independent definition identity comparison.
They are not original-code comparisons or gameplay acceptance.

For a reviewable freeze, run the checker from an isolated archive/worktree of the
committed source, set a separate `CARGO_TARGET_DIR`, and pass `--source-commit`.
This excludes other agents' pending source edits. When a reviewed original
trace/snapshot/corpus association exists, pass `--binding`.
The binding is a JSON object with these fields:

```json
{
  "schema": "c6-linux-movement-world-binding-v1",
  "game_sha256": "<pinned Linux XP2 hash>",
  "host_sha256": "<pinned Linux host hash>",
  "profile_sha256": "<reviewed complete runtime configuration manifest hash>",
  "evidence_level": "native_serializer_owned",
  "native": {"path": "native.jsonl", "sha256": "<raw JSONL hash>"},
  "core": {"path": "before.bin.gz", "sha256": "<decompressed Core hash>"},
  "trace": {"path": "owners.jsonl.gz", "sha256": "<decompressed trace hash>"},
  "trace_core_sha256": "<exact Core associated with original owner trace>",
  "native_source_snapshot_sha256": "<exact Core associated with native cases>",
  "trace_save_ordinal": 1,
  "owner_counts": {"map": 1, "units": 1262, "formations": 1250}
}
```

The counts above illustrate the format; use actual complete observed counts for
the matched execution. Hashing unrelated files together does not establish a
reviewed association. A previous canonical save/trace must not be substituted
because it has similar IDs or object counts. The checker validates the supplied
identities and extracts exact original serializer-owned ranges, never ID byte
patterns or assumed offsets. It writes temporary typed fixtures with per-record
hashes, then invokes the already frozen world candidate.

The candidate imports typed Map/Unit/Formation records once and compares:

- Full native Unit vectors, member order/presence, actual resolved Unit identity,
  ignore getters/modifiers and formation aggregates, owner/domain/location and
  serialized remaining moves.
- Native definition query/index/hash identities from saved terrain/feature/
  improvement hashes and route bytes, even when water properties are unavailable.
- Plot costs/water/city/route/district facts and directional river/adjacency/bridge
  results only when complete properties and runtime inputs are available.

The selected Unit's saved Formation ID and Unit member requests use original
low-sixteen-slot semantics; requested generation bits are not incorrectly equated
to the resolved object's full ID. The Formation lookup is independently witnessed
by `RecycledIDContainer<Formation,8,16>::Find` at `0x2b05830`; Unit::GetFormation
(`0x20214e0`) itself reads its live pointer. Duplicate imported native slots reject.

District type/completion and domain masks are named runtime inputs from the
actual targeted probe observations. IsCity is derived from the active district
definition hash; domain validity is computed from the actual raw i16 mask. The
candidate does not pretend those fields were serialized or rebuild masks from
Plot bytes. Player/district registry reconstruction and complete domain-mask
rebuild remain separate dependencies. Nonempty portal reconstruction remains an
explicit error. Unused domain-valid placeholders on land/without attributes are
not compared as though the native queried branch consumed them.

Exit status 0 means all declared groups compared equal, 1 means mismatches, and
2 means unsupported/missing input or malformed provenance. A current-export run
can compare supported groups and still exit 2 for missing water properties.
Reports retain mismatch and unsupported counts separately and always leave
native differential acceptance, complete affected state/RNG and full movement
parity false. Original arithmetic results remain in the unmodified native corpus
and frozen scalar candidate comparison, independently of world-resolution gaps.
