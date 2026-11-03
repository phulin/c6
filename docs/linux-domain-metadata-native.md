# Separate Linux domain metadata exporter

`native/linux/domain_metadata.c` is a read-only exporter candidate for the pinned
Linux XP2 module `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
It is separate from `movement_cost.c`; existing helper revisions and accepted
corpora are unchanged. Static compilation and byte fingerprints do not admit
native execution or establish parity. Independent ABI/lifetime review and a new
capture by the designated native runner remain required.

The intended use is to supply actual definition order and ownership to Rust so
Rust derives domain masks from typed state. Exported getter answers are expected
outputs only. This exporter does not implement complete gameplay state, actions,
or turns. The existing raw Save→Load 89,198-byte failure remains a failure;
unknown bytes remain unclassified and unmasked.

The hook is `c6_linux_domain_metadata_probe(module, borrowed_context)`. The
runner must retain the actual module, GameLoad-created context, initialized
controller/cache, original facade, dispatch/policy guards and borrowed owners
through its return. It must gate the exact module/host/dependencies, input save,
rules database, enabled DLC/mods, configuration, helper build/source and complete
profile-manifest identity before entry. A profile digest string alone is not an
identity check. This probe never calls Begin, executes an action, wakes a worker,
or constructs a native gameplay object. No Windows execution is involved.

The runner exports `c6_linux_domain_metadata_quiescent(module, context)` and pins
its `dladdr`/helper identity separately for this new query profile. Zero requires
original Controller.IsExecuting=false, Context.HasPendingMessages=false and the
profile's other established idle predicates. Active/unknown state must log and
`_exit` immediately, without post-state reads, Save, cleanup or Uninitialize.
The exporter also `_exit(240)` on any nonzero hook return. The original movement
corpus's controls do not automatically cover these new queries.

Required environment variables all start with `C6_DOMAIN_METADATA_`:

- `OUTPUT`: private exclusive-create JSONL path.
- `PROFILE_SHA256`: lowercase 64-character digest of the runner-validated manifest.
- `SNAPSHOT_BEFORE`, `SNAPSHOT_CONTROL`, `SNAPSHOT_AFTER`, `SNAPSHOT_REPEAT`:
  distinct private exclusive-create raw Context save paths.

The helper captures the known 32-byte sync and map RNG streams before the first
save, takes before/control saves before discovery, exports registries, players
and plot observations in that order, then takes after/repeat saves. Quiescence
checks surround phases, saves, each player and each plot. It compares the same
known RNG bytes at the end; a change fails status 228. Runner comparison must
retain raw save lengths, hashes and byte differences for all control pairs,
including normalization differences. These two known streams and raw saves do
not cover all live state or all gameplay RNG. All new query call paths still
need controlled original execution to establish this read-only profile.

The JSONL schema is `c6-linux-domain-metadata-native-v1`:

- `header` carries module/host/profile identity, 65 player slots and map dimensions.
- Three `registry` records (`district`, `building`, `terrain`) preserve actual GetCount/At
  enumeration order. Entries contain `query`, actual original `index` (GetCollectionIndex) and
  original type `hash`. `find_index` observes original base Find(hash) followed
  by GetCollectionIndex; `direct_index` observes original base Find(query)
  followed by GetCollectionIndex. A null lookup yields JSON null, preserving
  duplicate-hash/direct-index selection without assuming At positions equal
  native indices. Terrain entries include original `is_water` from the same
  loaded registry context, closing provenance without borrowing an older
  movement packet. Terrain has no XP2 object. District
  entries also contain `internal_only`. Each district/building entry contains an `xp2` object.
  Its `status` is `present` only after original XP2.Find(base hash) returns a
  pointer, followed by original district `is_canal` or building `canal_wonder`
  and `bridge` getters. An actual Find null yields `status:"absent"` with no
  Boolean answers. Missing base/XP2 collection or a null in-range At is an
  unsupported boundary (223), never an absent definition or a false answer.
- Exactly 65 `player` records preserve query order 0..64. `mapped_index` is the
  actual GetPlayerToIndex entry, `is_valid` comes from original IsValid, and
  `resolved` records original GetValidPlayer nonnull. The mapping is not used
  as a player pointer-table index. A resolved owner has original `actual_owner`
  and `cities`/`districts` membership objects. Each preserves original traversal
  `ordinal`, full `id`, its low16 `slot`, and original member `actual_owner`.
  Membership count must match original GetCount; Find(id) must resolve exactly
  the same member pointer. No guessed contiguous IDs or map-location ownership
  inference is involved. No membership is invented for unresolved owners.
- Every in-bounds real plot has a `plot_expected` record with `x`, `y`, original
  `coastal_min_one`, original `wonder_complete`, raw signed16 `valid_domains_mask`
  and three `valid_domains` answers
  for domain values 0,1,2. Original `has_city_attributes` is reported separately
  from GetCityAttributes: the getter may return original slot-zero fallback
  attributes for absent/invalid IDs. The signed mask uses memcpy from that
  actual returned receiver at +0x14, as proven by original
  IsDomainValid(DomainMask)'s sign-extending two-byte load. This preserves Hover,
  Immobile and all upper bits and never turns fallback attributes into owned
  plot attributes. Call order is HasCityAttributes(plot), GetCityAttributes(plot),
  raw mask observation, coastal(plot,1), wonder(attributes),
  domain(attributes,0), domain(attributes,1), domain(attributes,2). These records
  must never populate the simulator's input masks or prerequisite state.
- The final `summary` must have status zero, complete expected plot count and
  `known_rng_equal:true`. A missing summary, nonzero return/status, write error,
  incomplete record or count mismatch invalidates the entire artifact. Status
  221 is binding admission, 222 environment, 223 unsupported registry, 224
  ownership/membership, 225 plot, 226 owner admission, 227 output and 228 known
  RNG mismatch. Original save failures propagate their own status codes.

Ownership uses original Game/GetPlayerManager and actual initialized player
containers. The mapping vector has exactly 65 native entries and the player
pointer-table live extent must be exactly 65 pointers before GetValidPlayer.
Original const Begin/End return one-word iterators; original operator-> and ++
traverse their linked list. Original constructors establish the iterator ABI;
no synthetic BlockEntry fields are dereferenced by the helper. The original
per-player Find uses the low16 slot. Original member GetID/owner and pointer
identity are recorded rather than inferred from serialized City locations.

Map singleton/current-map/Game-map identity, Map vptr/GetPlot slot, Plot
vptr/GetCityAttributes slot, CityAttributes vptr and City primary vptr are
checked. District is owned by the original container and its original Find;
no District class vptr is fabricated (its leading archive is a distinct type).
The City/District constructors are fingerprinted as evidence and never called.

Reproduce static checks without executing original code:

```sh
python3 tools/linux/domain_metadata_audit.py --output /tmp/domain-metadata-query-contracts.json
cmp /tmp/domain-metadata-query-contracts.json evidence/linux/domain-metadata-query-contracts.json
cc -std=c11 -Wall -Wextra -Werror -fsyntax-only native/linux/domain_metadata.c
```

The audit verifies 86 complete original method fingerprints, the precise source
binding sequence, original 65-slot initialization, iterator constructors and
City constructor GOT/vtable reference. Original HasCityAttributes, raw mask
accessor and CityPlots.GetAttributes/Initialize/CityAttributes constructor bodies
are included to preserve presence versus original fallback semantics. It complements
`evidence/linux/movement-domain-query-contracts.json`; neither packet is an
execution approval. The future runner must capture every new operation with
fresh source/build/input identities and full-save/RNG/cleanup controls before
using the output for independent Rust comparisons of all 6,996 plot masks in
the current fixture. No such comparison has been performed by this change.
