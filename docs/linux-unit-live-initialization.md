# Linux Unit Read application

The library now applies a bounded, source-derived part of current Unit28 Read
through `unit_initialization::UnitRuntime::from_read`. It prepares a mutable
sidecar beside the actual owned wire Unit without cloning that Unit or changing
its hash fields. This is not complete Unit initialization, full SkipTurn or a
complete state/action/turn simulator. Acceptance is static source evidence and
Rust boundary tests; no new native execution or comparison is claimed.

The [static contract](../evidence/linux/unit-live-initialization-static.json)
gates the supplied Linux GameCore and host SHA-256 identities and fingerprints
complete original functions. Discovery reused the pinned cru121 CC BY 4.0
[reference](reference.md); actual target bytes determine the implementation.
Raw disassembly stays private. Reproduce the audit with:

```sh
python3 tools/linux/unit_initialization_audit.py \
  --output evidence/linux/unit-live-initialization-static.json \
  --private-output ../c6-ghidra/evidence/linux-unit-live-initialization
cargo test -p c6-core unit_initialization
```

`Serialization::operator>>(Unit&)` at `0x2c231f0` resets storage before reading.
Its UnitTypes reader at `0x2d4ec80` remaps the saved hash through the actual
Unit definition collection. The postlude sets archive identity, clears the
unsaved `m_aiPlanner` pointer and invokes virtual post-load initialization.
The real Linux vtable relocations bind that slot to `CInstance::InitializePostLoad`
at `0x2c12d00`. It calls `IInstance::InitializePostLoad` at `0x2c1c850`, which
calls `InitializeDefinition` at `0x2c1c6e0`, installing the actual definition
reference and runtime collection index. It then dispatches `InitializeDomain`
at `0x2c12bc0`. Five explicit domain hashes map to runtime domains 0–4;
other known domain hash values map to native `NO_DOMAIN` (-1).

The Rust input requires an admitted native-order definition table containing
actual domain hashes and the original collection lookup behavior. Unknown Unit
definitions error. The decoded `Unit.e_type_index` remains its wire hash;
`UnitRuntime.definition_index` stores the derived live index. The table inputs
must carry the same complete `InputIdentities` as the admitted imported state,
including explicit unavailable live database identity where applicable. Equality
checks prevent accidental cross-profile application; they do not authenticate
an arbitrary caller-created registry.

Formation references use the owner's complete original recycled lookup. Native
low16 lookup can resolve a different generation: the sidecar preserves requested
and resolved IDs. A known missing slot becomes a null reference. An absent owner
or unavailable Formation collection errors before any world mutation.

Reset at `0x2c23dc0` clears and sizes the six top-level definition arrays: ability
counts and both GreatPersonClass point arrays start at zero; invalid terrain,
ignored terrain and ignored feature arrays start false. The current reader then
overlays saved pairs by actual definition lookup. Unknown keys are consumed and
ignored; repeated known keys overwrite in order. The generic helper itself does
not clear destinations. The public `overlay` helper preserves unmentioned input
values and preflights resolved indexes before its first mutation. The current
UnitOperation IndexList reader separately resets false before its saved overlay,
so the sidecar contains mutable, definition-sized operation-disabled values.

This boundary does not initialize nested Experience/RockBand arrays or their
other remapped enum fields, register sync-manager subscribers, clear the complete
outer-load dirty baseline, or execute reporting callbacks. The sidecar attaches the 73 original Unit wrappers to a logical archive with
component type 1 and the original truncated owner/ID identity. Its outer-load
dirty baseline remains explicitly unavailable; dirty membership queries error
until that boundary is source-closed. This module does not set the older
replacement kernel's archive booleans. Its runtime
fields are actual derived storage rather than precomputed movement/busy/action
answers. Source-only tests cover signed-high domain hashes, ordered duplicate
and unknown-key overlays, retained destinations and no partial mutation on a
bad resolved index. Full native initialized-state comparison remains required.

No save production is implemented by this module. The recorded raw Save→Load
failure remains 89,198 unclassified, unmasked bytes; it is independent of this
bounded application work. External policies continue to supply every player
decision, with gameplay rules, upkeep and RNG remaining in scope.
