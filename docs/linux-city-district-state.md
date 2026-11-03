# Linux City and District movement registry contract

This targeted contract supplies the missing City/District identity and movement facts for the world resolver and native movement probe. It is not a complete City or District simulation, a complete world importer, or movement parity. No native execution, Ghidra change or private upload occurred for this work.

The [read-only audit](../tools/linux/city_district_state_audit.py) and [checked report](../evidence/linux/city-district-state.json) gate original Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. They pin 76 complete original method fingerprints, selected branch/field bytes, concrete virtual providers, domain tables and matching DWARF. Existing [Map fields](linux-map-state.md), [dynamic Core Map extraction](linux-core-stream-layout.md), [definition identity](linux-definition-ids.md) and [movement phases](linux-movement-cost.md) were consulted first. The pinned cru121 reference revision `5af389445118bed2bdebf7a8a42b219c8fba5175` supplies layout/name discovery leads under CC BY 4.0; see [attribution and limits](reference.md). The supplied Linux methods and matching DWARF establish the contracts below; SQL rowids and upstream Windows layouts are not substituted.

## Separate instance IDs, definition indexes and hashes

| Value | Native meaning |
|---|---|
| ComponentID | Eight bytes: signed i16 owner, signed i16 component tag, u32 full instance ID; native alignment 8 |
| CityID / DistrictID | ComponentID with tag **2 / 3** respectively; constructors retain the supplied owner's low 16 bits and full u32 ID |
| Instance ID | Allocator identity with low 16-bit slot and upper counter bits; not a district definition |
| DistrictTypes | Runtime signed i32 **definition collection index**; `NO_DISTRICT=-1` |
| District definition hash | u32 saved type identity, mapped through the active native definition registry |
| SQL rowid | Separate database identity; not any of the above by assumption |

The Core prefix does **not** supply definition enumeration mappings. Context.MakeMetaData (`0x290bde0`) writes only numeric compatibility values under keys `fd1bbf76` and `4e33fe49`, plus the GUID string under `f4e540d8`; ReadMetaData (`0x290bf00`) validates those against the corresponding Context getters. IndexSerializer.SaveVersion/LoadVersion (`0x2d34280`/`0x2d34290`) transfer one i32 version, currently 1. The independently parsed prefix has those three metadata entries immediately followed by IndexVersion1 and Game22. Neither contains Terrain/Feature/Route/District ordinal tables. Reuse actual definition registries and their hash mapping; do not infer runtime indexes from SQL rowids.

CityAttributes stores one signed player byte shared by its working-city, owning-city and district references. Original getters sign-extend that byte and construct each original ID with the corresponding u32 field; they do not extract an owner from the ID bits. `GetDistrict()` at `0x21674b0`, `GetOwningCity()` at `0x1fe2460` and City/District `GetComponentID` return nontrivial eight-byte values through **hidden result storage in RDI, receiver in RSI**. Use eight-aligned storage. The native constructors perform ordinary scalar writes; these temporary IDs acquire no reference and their callers show no destructor work.

## Exact lookup, including the generation detail

Static `City::CManager::GetCity(ComponentID const&)` (`0x286dc00`) and GetDistrict (`0x286e0b0`) take the ID address in RDI and return a **borrowed** pointer in RAX. Generic ComponentID.IsValid (`0x208e9e0`) checks only owner bits !=`0xffff` and full ID !=`0xffffffff`; it does **not** check the component tag or player range. The manager then resolves an actually valid player and uses that player's Cities/Districts container. GetValidPlayer's range is **unsigned owner <65**, followed by the native player mapping !=`-1`; player 64 must not be rejected merely because another action message limits its encoded player to 0–63.

`Player::Cities::Find` (`0x2a597e0`) and Districts.Find (`0x2ae12e0`) reach their recycled containers at manager `+0xa0`. Both concrete `Find` bodies (`0x2a5cc90`, `0x2ae4290`) reject ID `UINT_MAX`, then use:

```
slot = requested_id & 0xffff
block = slot >> 3
entry = slot & 7
if block >= actual_block_count: return null
return block_entries[block][entry].object
```

**They do not compare the full ID or upper counter bits with the stored object's ID.** A stale upper half can therefore alias a present slot. Native Add increments the container's wrapping u32 counter by `0x10000`, then ORs the allocated slot into the returned ID. This is a container-wide creation counter, not a separately checked per-slot generation. Preserve full object IDs, allocator state and order for future creation/removal support, while reproducing this particular lookup's slot behavior. A Rust full-ID dictionary that returns missing for such an alias changes the original query contract. Conversely, accepting arbitrary stale IDs as externally authorized actions is not implied by this internal lookup.

The targeted registry needs valid-player availability and `(owner, slot)` presence, with each record retaining its **actual full ID**. Requested and actual IDs must remain separately observable. City GetID/GetOwner read native `+0xb0/+0xe0`. District GetID reads `+0xa8`, while its owner is derived through actual District.GetPlot → Plot.GetOwner, not a separate serialized District-owner field. District.GetCity reads that plot owner and looks up its stored parent City ID (`+0xe0`) in the corresponding player's Cities container. City and District pointers are borrowed; no facade release or synthetic owner is required for these internal getters.

## IsCity, district type and completion

CityAttributes' concrete vtable (`0x3ae2350`) binds GetCity at slot 0 to `0x2bc8310` and GetDistrict at slot 8 to `0x2bc8320`; those forward to the Core CManager lookups. GetDistrictType (`0x2bc81b0`) constructs the genuine DistrictID, invokes that original virtual lookup, and returns District.GetType (`+0xf8`) or **-1 on a missing instance**. IsDistrictComplete (`0x2bc8200`) follows the same lookup and returns District.IsComplete (`+0x400`) or **false on a miss**.

CityAttributes.IsCity (`0x2bc8170`) obtains that runtime type. For -1 it returns false. Otherwise it indexes actual District definitions and compares the definition TypeHash with **`0x8a32007d` (`DISTRICT_CITY_CENTER`)**. It has **no completion check and no City-instance lookup**. An incomplete resolved City Center still satisfies this predicate. District.Instance.IsCityCenter is a different method: it resolves the owning City and compares locations. Do not substitute that method or “owning City ID present” for the movement IsCity predicate.

Original IsCity does not guard a null definition after a nonnegative type index. A missing required definition is an unsupported/invalid context for the resolver, not an evidenced native false result. District.GetTypeHash has its own null→0 behavior, but substituting that safer getter would change this specific call chain. Definition collection order, active rules/DLC/mod configuration and hash remapping remain required.

## Serialized projection and omitted live fields

Current City wire version is **51**. Its initial fields are version, full instance ID, PlotCoord `(i32 x,i32 y)`, owner; the audit reads this 20-byte prefix only. Current District version is **15**. Its initial fields are version, full instance ID, PlotCoord, parent City ID, **District definition hash**, five i32 values, then completion bool at prefix byte 44. The audit reads this 45-byte prefix only. Native FAutoVariable wrappers route DistrictTypes through original WriteIndexToHash/HashToIndex, rather than saving the live ordinal directly. The completion bool is serialized. Many later City/District fields remain outside this projection; these prefixes are not full codecs.

Map CityAttributes has live size 32 and the following fields. The 17-byte wire record saves working/owning/district u32 IDs, signed player byte and Wonder Building definition hash in that order.

| Live offset | Field | In this wire record? |
|---|---|---|
| +8 / +12 / +16 | Working / owning City ID; District ID | Yes |
| +20 | signed 16-bit `validDomains` | **No** |
| +24 | Plot yield-effect ID | **No** |
| +28 | signed player byte | Yes |
| +30 | packed signed BuildingTypes index | Saved through definition hash; original Read narrows the resolved index back to 16 bits |

CityAttributes.Initialize sets player/IDs/Wonder to their sentinels and clears validDomains/yield-effect. Read does not restore the omitted fields. Plot.HasCityAttributes tests its local attribute index !=0; GetAttributes checks whether that sparse index exists and otherwise returns reserved default entry 0. That allocator index is not an imported object identity. Preserve HasCityAttributes separately from values read through a fallback/default pointer; do not invent a real per-plot attribute object merely because GetCityAttributes returned nonnull.

## Runtime domain validity is reconstructible with real dependencies

GetDomainMask (`0x2bc8370`) maps DomainTypes SEA=0, AIR=1, LAND=2, IMMOBILE=3, HOVER=4 to **1,2,4,8,16**. Every value outside 0–4, including -1 or a domain hash supplied as an ordinal, maps to **8**. IsDomainValid(DomainMask) sign-extends the live i16 mask to i32 and tests `(mask & query)!=0`. Keep the raw 16 bits; do not replace this getter with an inferred mask from terrain alone.

Original Game.Read calls CityPlots.RestoreValidDomainData (`0x2a41a10`) after loading world/player state. It traverses actual plot index order and, for plots with attributes, calls SetValidDomains (`0x2a41a80`). That clears the mask, then tests domains in order SEA, LAND, HOVER, AIR and sets their bits. Its IsValidForDomain table (`0x2a41b40`) is:

| Domain | Base predicate |
|---|---|
| Sea | Water, or IsCity and original IsCoastalLand(1) |
| Air | true |
| Land | not Water |
| Immobile | false |
| Hover | true |

XP2 Game.Read then visits plot indices again. After resolving a District type other than -1 and requiring DistrictComplete, it augments the mask:

- Canal district definitions, with actual attributes and a repeated completion check, add Sea and invoke original MarkAdjacentCitiesNavigable.
- An InternalOnly district with a Wonder whose Building_XP2 definition is CanalWonder adds Sea and marks adjacent City plots navigable. **This branch does not call IsWonderComplete**; completion of the enclosing District was tested earlier.
- An InternalOnly district whose actual Wonder is complete and whose Building_XP2 definition is Bridge adds Land when attributes exist.

MarkAdjacentCitiesNavigable visits actual directions 0–5 and adds Sea to adjacent IsCity plots when not already set. Incremental construction, district/building pillage/destruction and city transfer also update domains; the audit identifies these paths but does not claim an exhaustive indirect-write inventory. CityAttributes.Update also copies its reflection values, including the live mask.

Thus the mask is not irrecoverable opaque state, but complete reconstruction needs the real district registry, district/XP2 definitions, building/Wonder completion, water/coastal predicates, adjacency and original update order. The raw Map record by itself is insufficient. A targeted native component export can supply the current real mask while those dependencies are implemented; unknown masks must remain unsupported. This is gameplay state because shore movement reads it, not a presentation-only exclusion.

## Targeted registry and probe handoff

The resolver owns production types. The agreed conceptual input is a read-only registry with valid-player availability, City/District `(owner,slot)` records, full actual IDs, location/parent City, actual District runtime type index plus definition hash, completion, and per-plot HasCityAttributes/raw validDomains. It can remain narrower than complete City state without claiming complete City simulation. Do not require parsing every preceding Player subsystem before admitting this faithful component export.

The movement probe owner received these exact ABIs and is integrating original requested-ID construction, borrowed lookup, actual ID and raw-mask observations. Required per-endpoint provenance is: plot coordinate/index; HasCityAttributes; requested original DistrictID owner/tag/ID; null/present resolution; actual full District ID; runtime type ordinal and definition hash/FreeEmbark; completion; raw mask and queried formation domain. Working/owning City IDs are already available in typed Map fields and can be exported for direct wire/live consistency. Probe source/module/configuration identities, concrete original bindings, synchronized before/after state and normal cleanup remain its separate acceptance gate. This task ran no probe and asserts no dynamic mask or runtime ordinal observation.

The existing controlled native serializer traces allow a useful separate cross-check without a general Player parser: both original normalization streams contain **245 City and 1,284 District records**. The bounded prefixes join all **5,287 working and 5,287 owning City references** and **1,284 District references** from typed Map, with no missing lookup or requested/actual full-ID mismatch in these fixtures. Districts comprise 1,262 complete and 22 incomplete; 245 saved definition hashes are City Center. Both projection digests are `cdf49f535dacd6e01392d075aec848c283ec8900fe2a2f97cd4af1c75ed14d45`. These are serialized-prefix observations, not native runtime IsCity/mask query results or full City/District state equality.

Reproduce with `python3 tools/linux/city_district_state_audit.py --output target/linux/city-district-state.json`. Static/DWARF gates and original-prefix checks pass. Source-only registry checks cover stale high-counter aliases, player 64, absent slots/players/IDs and domain fallback. No native movement parity, complete world import or general action/turn support follows. The 89,198-byte raw Save→Load difference remains preserved and unmasked; complete logical gameplay state/RNG and all external player decisions remain the objective.

## Production registry and initialization rebuild

`c6_core::city_district` implements this movement-relevant projection. `SlotTable::new` validates borrowed records in strictly increasing low-16 slot order, rejects duplicate slots and UINT_MAX records, and bounds each container to 65,536 records. Lookup is a binary search without allocations. `Registry` owns only a fixed 65-entry player-status array; records remain borrowed. The importer must explicitly distinguish an unavailable player mapping, a proven invalid player, and a valid player whose complete City or District table is supplied. A missing table returns an error; an empty complete table proves a genuine miss. No constructor infers completeness from the handful of records referenced by a path.

`ComponentId` preserves the native eight-byte size/alignment and owner truncation. `InstanceMatch` retains the original requested ID and the record's actual full ID and owner. District record owners must come from the actual plot owner; the requested/container owner does not substitute for that observation. `DistrictRecord::from_saved_type` remaps through the supplied native-order `DefinitionLookup`. `Registry::is_city` uses that registry's actual definition hash and has the original lack of a completion requirement. The `MovementRuntime` adapter implements movement's existing `RuntimePlotInputs` directly, using these District records and borrowed per-plot actual/rebuilt masks. Unavailable masks remain errors.

`rebuild_valid_domains` performs the verified initialization base pass followed by XP2 augmentation, each in plot-index order. It allocates one bounded mask vector with fallible reservation, and neither clones nor mutates Map or registry records. Complete Canal Districts add Sea and mark adjacent City plots in directions 0–5. Internal-only Canal Wonders add Sea without requiring Wonder completion; the independent Bridge branch requires actual Wonder completion before adding Land. Adjacent incomplete City Centers still satisfy IsCity. A missing dependency aborts the result rather than exposing partially rebuilt masks.

This makes the rule branches executable but does not infer unavailable inputs. `map_is_water` closes IsWater from typed Map and actual native Terrain indexes/properties, including the built-in 15/16 checks. The `DomainRebuildInputs` interface accepts this exact query and names the remaining actual dependencies: IsCoastalLand(1), IsWonderComplete and directional neighbor lookup. District/Building properties must use the genuine definition mappings and original getter values, including proven absence of optional XP2 definitions; false is not an unknown-property default. Current Map plus the bounded City/District prefixes do not close coastal and Wonder-completion dependencies. A native component export can supply those values or the original masks. Initialization is distinct from subsequent construction, pillage, destruction and transfer updates, which remain unsupported here. No City/District allocator mutation, complete instance codec, complete city simulation or world-import support is added.

Focused Rust tests cover native ID alignment/truncation, stale-counter aliasing, tag-insensitive lookup, player64, unavailable versus genuinely absent inputs, definition remapping, incomplete City classification, domain fallback, initialization branches and explicit failure/resource limits. These tests verify the source-derived contract; native movement comparisons remain pending the separately owned probe and immutable candidate profile.

Production validation passed `cargo test --workspace` (86 core tests, including five focused registry/rebuild tests), `cargo clippy --workspace --all-targets -- -D warnings`, and `bash tools/check/c_abi_smoke.sh`. The coordinated export was added only after the runner archived the movement candidate immutably; this work did not refresh that candidate's original native comparison identity.
