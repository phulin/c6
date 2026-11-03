# Owned Linux typed world assembly

`c6_core::world::World` owns a typed `Map`, native-ordered Player records and their optional complete Unit/Formation recycled collections. It builds lookup indices once, borrows actual records on queries and permits in-place Unit/Formation and Plot field mutation. This is reusable state ownership and lookup infrastructure for numerical rules and future actions. Complete gameplay state, full world import, action execution and turns remain unimplemented; this module does not change `Engine::apply`.

The importer must gate supplied Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, actual Linux rules/configuration identity and typed input provenance. `World::new` does not run original code or claim to validate these identities. There is no synthetic world snapshot, action fixture selector, per-query codec or gameplay HashMap iteration.

## Input ownership and lookup

`Player::new(owner, units, formations)` takes `Option<map::Recycled<Unit>>` and `Option<map::Recycled<Formation>>`. The shared recycled type preserves explicit slot indices, generated IDs, entry order, capacity and allocator metadata. A complete present empty collection proves absence; an unavailable collection is `None` and Find returns `MissingInput`. Native records must already have their actual owning player ID. Assembly rejects mismatching owners, duplicate/out-of-range slots and capacities beyond the low-sixteen-bit lookup domain. It does not derive slots from generated object IDs or normalize allocator metadata.

`World::players`, `Player::units` and `Player::formations` retain supplied entry order. Separate owner/layer indices are sorted only for lookup; record vectors remain unchanged. Units and Formations use dense slot-to-record indices. `UINT_MAX` returns absent on a present collection. Every other ID uses only its low sixteen bits. A found record exposes both requested and resolved IDs, including generation mismatches. Pointer identity is replaced by a stable borrowed owned record, not by an invented wire ID.

`Player::unit_formation` reads the Unit's actual `formation_id` and queries that owner's Formation collection. `Player::members` follows the supplied Formation's member IDs in their exact order, preserving duplicates and verified absent members. Missing collections remain errors. No reverse-membership repair or extra consistency assertion is inferred: native Find does not require generation equality, and this infrastructure does not manufacture Unit/Formation associations.

`World::map` borrows the original owned typed Map. Plots keep their dense `y*width+x` identity, as established in [Map state](linux-map-state.md). The original UnitContainer layer vector, sparse cell order and each cell's UnitReference candidate order remain untouched. Assembly builds a dense plot-to-cell index per layer and a separate layer-ID lookup index. `plot_units` borrows the original candidate slice. A verified absent layer/cell returns the native empty query result; an out-of-map plot fails. Candidate references resolve through their explicit owner and the actual owner's Unit slot index. Neither Plot unit-count bytes nor Unit locations replace the original occupancy container.

Mutable access returns the owned record directly. Changing ownership, identity, position or formation membership does **not** automatically maintain every other container. A future action must implement those coupled native effects before claiming support. A changed record owner or replacement Player with a different owner is rejected by subsequent owning-container lookup. Map dimensions and occupancy containers are kept private to prevent silently stale assembly indices; Plot fields themselves can be edited in place.

## Prerequisite and omitted live state

The `yields::PrerequisiteState` implementation reads actual owned native-order acquired technology/civic masks and global availability. These are `Option<Vec<bool>>` inputs because wire ownership alone does not reconstruct their complete live providers. Global false means a source-verified absent/inactive node, never uncaptured data. Out-of-range global queries fail explicitly. The numerical helper preserves native player-mask bounds and lazy query order; see [yield prerequisites](linux-yield-prerequisites.md).

Original Manager validity is independent of presence in an imported Player record vector. `World::player_mapping` is an optional complete `[i32;65]` native player-ID-to-container-index mapping. `IsValidRange` returns false for unsigned IDs >=65 (including all negative IDs). In-range validity requires captured mapping and is exactly `mapping[player] != -1`. Missing mapping returns `PlayerRegistry`. A valid mapped player whose typed record or acquired mask is absent returns `PlayerMask`; it never silently becomes an invalid player or a zero bonus. Mapping values other than -1 are preserved, not normalized to their own player ID.

The movement `RuntimePlotInputs` adapter explicitly rejects district instance resolution and nonserialized CityAttributes validDomains. Neither typed CityAttributes district ID nor owning-city ID is a complete live district lookup. It returns the existing typed `DistrictLookup`/`CityDomainValidity` errors rather than absent-district or permissive-domain defaults. Native legality, callback flags, archive attachments and reporting consumers are not supplied by this state assembly and no executable action accessor fabricates them.

## Linux source evidence and reproducibility

The Map/UnitContainer contracts and native iteration source are in [Map state](linux-map-state.md), [movement world](linux-movement-world.md), [Unit codec](linux-unit-codec.md), [Core stream indexing](linux-core-stream-layout.md) and their pinned original Linux evidence. In particular Player.Units.Find `0x2ba5580` delegates to recycled Find `0x2baa650`, whose complete static contract is already fingerprinted in `evidence/linux/movement-world.json`. Those methods reject UINT_MAX, select low16 slot bits, check the block index and read existing slot storage without generation validation.

Fresh read-only inspection of the SHA-gated supplied module additionally establishes the corresponding Formation path and Manager validity. The following are SHA-256 of exact original file-backed instruction ranges, not behavioral native fixtures:

| Linux VA | Bytes | Contract | SHA-256 |
|---|---:|---|---|
| `0x2b04f10` | 9 | Player.Formations.Find adds 8 to collection pointer and jumps to recycled Find PLT `0x1edad80` | `2a9655373cce8020799bada490208702c9696c5c50273cb3cf2c950af02804f5` |
| `0x2b05830` | 84 | Formation recycled Find: UINT_MAX guard, low16, block>>3, slot pointer; no generation comparison | `eba120c60410744f5c69f7277505827d2e4c0eff1548a17cda46a318c5815557` |
| `0x2004fc0` | 51 | Manager.IsValid: range guard, mapping at +0x1a08, compare entry against -1 | `31c2b843f442ca71b8fd27edeccb2888940a331a9f50686c2a1e078cd25be038` |
| `0x2005000` | 7 | Manager.IsValidRange: unsigned player <65 | `1bda6fddb3180619888eb42cbb75a05d0877b5498c502ebb72ec289b4e0ba7a1` |

The original dynamic symbol `_ZNK19RecycledIDContainerIN8GameCore9Formation8InstanceELj8ELj16EE4FindEj` resolves to `0x2b05830`. For independent static fingerprint reproduction from the repository root:

```sh
python3 - <<'PY'
import hashlib, sys
from pathlib import Path
sys.path.insert(0, 'tools/linux')
from save_lifecycle_audit import Elf, GAME_SHA
elf = Elf(Path('../c6-assets-linux/libGameCore_XP2.so'), GAME_SHA)
for address, size in [(0x2b04f10,9), (0x2b05830,84), (0x2004fc0,51), (0x2005000,7)]:
    print(hex(address), size, hashlib.sha256(elf.read(address,size)).hexdigest())
PY
cargo test -p c6-core world::tests
cargo clippy -p c6-core --all-targets -- -D warnings
```

The six new module-local synthetic checks exercise explicit-slot/generation semantics, native order, borrowed candidate pointer identity, absence versus unavailable state, owned mutation, malformed assembly and prerequisite provider errors. They are Rust contract tests, not new original-code execution or initialized-world differential acceptance. Complete raw Core roster import is still blocked by the intervening unframed player component schemas described in the Core index contract; callers can assemble faithful typed exports without making that importer claim. The successful original Map import is not a complete live-world reconstruction. All unknown save bytes remain unclassified and unmasked, including the independently recorded **89,198-byte Save→Load failure**.
