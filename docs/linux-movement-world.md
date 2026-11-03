# Linux movement world resolver

[`movement_world`](../crates/c6-core/src/movement_world.rs) resolves a bounded
movement query from typed current Map/Plot, Unit and Formation wire state and
active native-order definitions. It complements the
[arithmetic boundary](linux-movement-cost.md). This is production resolver code
with **static target evidence, not native differential acceptance**. It does not
load a Civ VI save, reconstruct a complete live world, validate movement legality,
execute actions, update RNG or advance turns. Complete state/action/turn behavior
remains unfinished. The recorded raw Save→Load difference remains **89,198 bytes**;
no unknown bytes are masked or classified as planning state.

## Definition and Unit import

`DefinitionLookup` adapts the existing
[`LinuxDefinitionRegistry`](../crates/c6-data/src/registry.rs) native-order lookup
contract without introducing a data/core dependency. It must provide actual
native collection count/order/hash identities. SQLite row IDs and row order
cannot supply those indices. `DefinitionTable` borrows properties in that same
order, verifies their count, and never clones the world. Properties must be
original getter values from the active configuration; route costs are original
binary32 bits, not inferred conversions from SQL binary64 values. The caller
must gate module, rules and configuration identities. These adapters do not
authenticate arbitrary caller-provided collections.

Target `HashToIndex<Terrain>` (`0x21951d0`) and feature equivalent (`0x1fe8220`)
call collection Find then GetCollectionIndex, or return -1. This includes the
collection's direct-index interpretation of small arguments and last-inserted
precedence for equal hashes. Plot stores resolved terrain, feature and
improvement indexes in signed 16-bit fields, distinct from wire uint32 hashes.
Route type is a serialized signed byte already containing an index: it uses At,
without hash remapping.

Current Unit read (`0x2c231f0`) calls Reset (`0x2c23dc0`) before reading fields.
Reset clears invalid-terrain, ignore-terrain and ignore-feature vectors and
resizes them to the active collection count with explicit **false** values
(`0x2c23e14..0x2c23edd`). Read resizes to those same counts
(`0x2c238fb..0x2c23985`), then overlays wire pairs. Terrain bool-array read
(`0x2c2a8e0`) and feature read (`0x285ef60`) consume each pair in order: known
indexes write the slot, unknown hashes consume/discard the value, and the last
duplicate write wins. Sparse current records leave false slots; wire pair count
is not native vector length. `remap_unit_bools` implements this reset-and-read
contract, not an arbitrary overlay onto an existing live Unit.

`UnitMovement::from_wire` imports those arrays once and uses serialized
MovementModifiers river, shore and escort bytes. It does not use the similarly
named global `b_ignore_terrain_cost`. Native terrain getter (`0x2c27cc0`) bounds
against **invalid-terrain length**, then reads **ignore-terrain**. Feature getter
(`0x2c27d00`) bounds and reads its feature array. Both reject -1. Other negative
indexes would access invalid native storage; Rust reports an error. Normal
imports have equal terrain lengths; the separate getter API preserves the
different-array bound and detects inconsistent storage.

`resolve_formation` sends each serialized member ID and owning Player to the
provided stable Units lookup. Player::Units::Find (`0x2ba5580`) calls the Unit
recycled container (`0x2baa650`), which rejects UINT_MAX and uses the low sixteen
slot bits, ignoring upper generation bits. `find_unit_slot` preserves that lookup
and retains requested and resolved IDs. Full-ID equality is not native lookup.
Verified missing members are skipped; unavailable
Player/Units state is an error. Each ignore predicate independently follows the
native all-member/escort rule and early exit. Empty resolved membership returns
true. CanEmbark comes from Formation's own serialized field. No native policy
choice supplies missing gameplay data.

## Plot and edge queries

`resolve_plot` derives costs, route presence/pillage/bridge flags and terrain
water from typed fields and native properties. IsWater (`0x2bc97d0`) returns true
for runtime terrain indexes **15 and 16**, otherwise tests the actual terrain
definition's IsWater getter, with false on null lookup. An ocean name/hash cannot
replace this native index contract. Route presence (`0x20e3570`) means route
byte != -1. Improvement cost ignores pillage because the original phase does.

CityAttributes' serialized district is a **district instance ID**, not a
collection index. RuntimePlotInputs separates verified absent districts from
unavailable registry state. Its provider must implement native owning-player
instance lookup; full-ID equality cannot replace recycled-slot lookup.
`resolve_plot` computes IsCity by testing resolved district definition hash
against immediate `0x8a32007d` (CityAttributes::IsCity, `0x2bc8170`) and obtains
FreeEmbark from that definition. Nonserialized CityAttributes validDomains is a
required provider input on water plots with city attributes. The native reader
rebuilds and augments it using city/coastal, domain, district, canal and wonder
state; raw Plot bytes alone are insufficient. `NoRuntimePlotInputs` returns typed
errors rather than inventing completion, type or domain values.

`plot_at` borrows actual y*width+x slots in constant time. Geometry accepts
positive dimensions and actual in-map coordinates. `adjacent` ports the native
six-neighbor loop (`0x26b1030`), both wrap flags and negative remainder behavior.
`direction_to` separately ports GetDirectionToPlot (`0x26b0ee0`), X-only wrapping
and directional sectors even for nonadjacent destinations. River crossing uses
source crossing-mask bits and that direction (`0x26b0f80`), after rejecting
water/city endpoints as Rules::IsRiverCrossing (`0x2bed3b0`) does. This API has a
present typed Map; it does not infer direction from distance.

Adjacent moves bypass native portal queries. For nonadjacent moves a present
empty Route component proves no portal exit. Nonempty portals currently return
PortalLookup: runtime sparse source-table reconstruction from ReadPortals and
AreaInstance::Read remains unimplemented. Serialized entrance and source lists
must not be conflated. This is an explicit implementation gap.

## Reproduction and acceptance

```sh
python3 tools/linux/movement_world_audit.py --output target/linux/movement-world.json
cargo test -p c6-core movement_world
```

The [static report](../evidence/linux/movement-world.json) pins Linux XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, fingerprints
29 original methods, checks reset instruction operands/default bytes and native
direction constants, and never executes game code. Raw original disassembly is
local at `/tmp/c6-linux-movement-world-evidence/`, outside the public repository
and private snapshot approval set. Discovery follows the pinned cru121 CC BY 4.0
reference and actual Linux evidence described in [movement-cost](linux-movement-cost.md)
and [reference provenance](reference.md). No Windows acceptance is transferred.

The native probe owner exports independent original ordered registries,
properties, full live Unit vectors, direct getters, plot facts and phase results.
These must be compared against the resolver, alongside all affected state/RNG,
after original execution is separately authorized. No native resolver corpus has
been accepted. Runtime player/district/domain inputs, nonempty portals, configured
parameter extraction, budgets, ZOC, legality, actions and complete turns remain
explicit dependencies or separate boundaries.
