# Linux serialized movement input integration

The production [adapter](../crates/c6-data/src/city_movement.rs) independently
reconstructs **5,287 owned CityAttributes masks and Wonder results**, all equal
to original Linux native-2 observations, with zero mismatches. All **6,996**
attribute-presence and coastal(min1) results also match. The
[strict comparison](../tools/linux/movement_domain_reconstruction_results.py)
and [report](../evidence/linux/movement-domain-reconstruction-native-2.json)
retain every plot and signed raw mask. The **1,709** plots without owned
attributes have explicit unavailable reserved-entry mask/Wonder results;
observed native zero/false values are preserved, never supplied as calculations.
Complete gameplay state/actions/turns and all-plot raw-mask parity remain false.

The frozen corrected candidate `e899e4d9…`, source `60507af`, takes only real
serialized Map/245 City/1,284 District records, actual native Terrain/District/
Building collections (17/28/109 entries), and all 65 PlayerManager records with
complete authoritative membership (43 resolved players). Expected coastal,
Wonder and mask observations never enter its input JSON. Owned masks comprise
4,781 values of 22, 477 of 19 and 29 of 23; 33 owned Wonder queries are true.
This does not establish native coverage of every Canal/Bridge rule branch.

The closed capture has actual profile `b8fcda05…`, native metadata `6e417c79…`
and runtime report `28d4956c…`. The checker verifies the complete prepared/runtime
configuration, pinned module/host, frozen candidate/archive/source, successful
fixture and both cleanup boundaries, all six raw Core controls, complete native
summary and both known RNG streams. All six Core streams are byte-identical
`ff31f4c5…`, permitting exact reuse of the original owner trace. Its gzip artifact
is `5dd8fb4d…`; the independently gated decompressed trace is `52872108…`.
The resulting logical candidate-input projection is `38099fff…`. Full per-plot
native and calculated outputs remain in content-addressed local diagnostics.

The earlier District-only fact control3 matched **3,072 / 3,072 endpoints**:
1,282 positive resolutions across 298 Districts and 1,790 invalid-ID misses.
That bounded result removed runtime type/completion inputs while retaining
the original domain-mask side inputs; the native-2 result above supersedes that
mask limitation for owned attributes.

`DistrictPrefix::decode_prefix` consumes the audited first 45 bytes of a District
v15 record whose boundary was supplied independently by original serialization.
It checks the version, length and completion bool; later bytes remain unconsumed.
`movement_record` takes the actual plot owner from typed Map CityAttributes and
remaps the saved definition hash through the genuine native District collection.
It never substitutes the requested/container owner, SQLite row IDs or native
expected type/completion. `city_record` projects the complete typed City header
into the existing core City identity record. No City/District allocation, creation,
removal or complete District codec is added.

The [checker](../tools/linux/movement_domain_inputs_check.py) reuses the immutable
control3 reproducer's complete native configuration, module/host, closed cleanup,
Core/RNG and original owner-trace identity gates. It extracts all 1,284 District
prefixes from the serializer-owned spans of the exact `ff31f4c5…` Core stream,
decodes the original Map with Rust, and passes only the actual 28-entry District
definition order/hashes to the new Rust driver. Native presence, ID, type,
completion and coordinate fields are expected answers only. The source and new
candidate hashes are in the [report](../evidence/linux/movement-domain-inputs.json).
The frozen world candidate `63d15a41…`, arithmetic and native helpers are unchanged.
The checked new candidate `450ba286…` was built offline from a clean `git archive`
of committed source `d852a50` in `/tmp/c6-movement-domain-d852a50`, independently of
the shared checkout. The exact isolated source passes `cargo test -p c6-data`
(14 passed, four explicitly fixture-dependent tests ignored) and
`cargo clippy -p c6-data --all-targets -- -D warnings`.

Player validity is still an independent input to general `city_district::Registry`
construction. Each positive native lookup in this comparison proves validity for
that particular queried player. Every missing District request is `UINT_MAX`,
whose generic invalidity precedes native player lookup. Consequently this fixture
comparison needs no fabricated complete player-validity map. Owner appearance in
serialized records does not prove player validity. This checker does not validate
general missing-slot or invalid-player behavior dynamically.

`initialization_domains` composes the existing exact two-pass core rebuild with
`map_is_water`, using actual Terrain definition order/properties and the original
15/16 fast path. The subsequent `MapDomainInputs` and `TypedDomainInputs` close
the directional-neighbor, IsCoastalLand(1) and Wonder-completion queries from
typed Map/City/Buildings. They retain the existing Canal, neighboring City,
signed-short Building remapping/Find, CanalWonder and independent Bridge
completion behavior. Required unavailable state remains a named error.

The [static query audit](../tools/linux/movement_domain_inputs_audit.py) pins
45 complete original method fingerprints and both native direction tables.
Its [report](../evidence/linux/movement-domain-query-contracts.json) records
source contracts separately from native runtime comparisons. The coastal query
at `0x2bc9710` first rejects water at the source, then traverses directions 0–5.
A neighbor qualifies only if it is water, its serialized Impassable flag is
clear, and its actual **Landmass** pointer is nonnull with signed plot count at
least one. Area size and the Landmass water flag are not queried. Plot.Read
restores Landmass through `Container.Get(saved id)`; that lookup ignores upper
generation bits and resolves the low-sixteen slot. The adapter uses the saved
container slot, not the Landmass object's ID or an inferred connected component.
An empty complete Landmass table proves a null lookup; an inconsistent table
fails construction. Native PlotXToX/XToPlotX arithmetic, direction order, X/Y
wrapping and the negative-multiple CoordRange quirk are retained. This adapter
explicitly bounds canonical coordinates to the original signed-short range.

`CityBuildings::is_wonder_complete` follows `CityAttributes.IsWonderComplete`
at `0x2bc8100`: remap the saved Wonder hash, narrow to signed i16, reject packed
-1, resolve the owning City using the actual core Registry, then call the
equivalent of Buildings.HasBuilding. This tests **signed building location >=0**;
it does not check pillage or require the stored location to equal this plot.
The native Buildings reader calls Reset, which fills actual Building-count
locations with -1. It then applies the serialized hash/value pairs in wire order:
unknown hashes are ignored and repeated aliases overwrite. The production query
reproduces that mapping without materializing a full location vector. Missing
typed Buildings for a resolved City is an explicit MissingWonderCompletion error.
Player validity and complete City/District container membership are separate
requirements, not inferred from a typed City's owner.

Five focused source tests cover prefix validation, coastal passability/null and
nonpositive Landmass size, low-slot aliases independent of object IDs, direction
order/wrap quirks, Building hash aliases/defaults/negative queries, pillage
independence, Wonder guards and unavailable City state. These and the existing
core branch tests are source-contract tests, not native coastal/Wonder/mask
comparison. The new offline driver calculates coastal and ordered neighbors for
all original Map plots and independently compares water at native endpoints.
The [follow-up report](../evidence/linux/movement-domain-derived-queries.json)
records 6,996 calculated coastal/neighbor queries, including 243 true coastal
results, and 3,072 / 3,072 water endpoints equal to original control3. Coastal,
neighbor and Wonder results had no new native runtime comparison at that checkpoint. Its
candidate was built from a clean archive of `5d2dfc7` at
`/tmp/c6-movement-domain-5d2dfc7`; isolated data tests pass 18 with four explicit
fixture-dependent skips, and strict all-target data clippy passes.

The existing authoritative movement export schema and original control3 JSONL
provide Terrain/Feature/Improvement/Route/District registries only. District rows
carry FreeEmbark, not Canal/InternalOnly. The data registry importer contains no
Building collection loader/order proof. The newer complete City wire codec
preserves Building location/pillage hashes, but it does not prove the original
Building index order. That order is still required to use the newly closed
Wonder-completion calculation. Those original packets alone could not support
complete masks; the separate native-2 export supplies the required genuine
metadata. SQL table iteration is not an alternative native collection export.

The separate native-2 export supplies:

- Complete actual PlayerManager mapping for all 65 slots, independently of owners:
  slot, GetPlayerToIndex entry, IsValid result, GetValidPlayer nonnull result and
  returned owner when present. Also provide actual complete per-player City and
  District container ID membership (including empty containers), unless enclosing
  serialized owner boundaries independently supply it. Grouping Districts by
  their actual plot owner alone does not prove native container membership.
- Actual District collection order/hash plus original Canal and InternalOnly
  getter results, including explicit absence of optional XP2 definitions.
- Actual Building collection count/order/hash, CanalWonder and Bridge getter
  results, and the original Find mapping needed after signed-short narrowing.
- Existing original raw masks for comparison. Original coastal/Wonder/neighbor
  results may additionally validate the newly derived intermediates, but are
  expected answers only; they are no longer required calculation inputs.

The separate read-only export can enumerate actual initialized base collections
with GetCount and At, then resolve optional XP2 definitions by the base type hash
using the original XP2 Find. Represent XP2 absence explicitly, for example
`{"status":"absent"}` versus `{"status":"present","canal":false}`;
do not default a missing export to absent. The relevant audited entry points are:

| Input | Linux entry points |
| --- | --- |
| Building collection / count / At / Find / type hash | `2213730` / `1fe2260` / `1fe4ad0` / `1ffb960` / `20a1260` |
| District collection / count / At / type hash / InternalOnly | `2213a70` / `1fe2250` / `1fe2b00` / `21d56f0` / `1fe2500` |
| Building XP2 collection / Find / CanalWonder / Bridge | `22137a0` / `20a11f0` / `2a3e3f0` / `28696c0` |
| District XP2 collection / Find / Canal | `2213ae0` / `21d5680` / `21d5a40` |
| PlayerManager GetPlayerToIndex / IsValid / GetValidPlayer | `2004680` / `2004fc0` / `2003940` |

All addresses are module-relative hexadecimal addresses for the pinned target,
not portable ABI constants. The property getters read fields and the collection
queries read existing initialized tables; this export must not initialize rules,
call policies or change gameplay. Its proposed metadata arrays carry explicit
`query`, `index`, `hash`, actual property values and XP2 presence per definition.
This plan does not authorize an original execution by the offline adapter task.

The corrected candidate accepts the genuine `d090532` exporter schema through typed
`NativeDomainDefinitions`, `NativeTerrainDefinitions` and `WorldInstances`.
Definitions require complete Count/At order, actual collection index, original
Find(hash) and Find(query) results. The bounded importer verifies these against
the supported native index-or-hash and duplicate-precedence contract; a different
collection layout fails explicitly. Missing XP2 status/property fields, incomplete
tables, unknown fields and contradictory native lookups are errors. Terrain
includes actual IsWater and movement cost from the same original definition.

`WorldInstances` requires all 65 explicit player records and complete native
City/District membership, then joins every member to a real serialized City or
District record by actual owner and full ID. The **player query** chooses the
container; the member's actual owner remains separate. Stale requested upper-ID
bits still resolve by low-sixteen slot. Every serialized record must be consumed
exactly once, duplicate slots fail, and empty complete containers remain empty.
The current bounded importer rejects valid mappings with a null player pointer
as an unsupported context; it does not invent membership for them.

The offline driver receives only Map bytes, owner-bounded District/City bytes,
the three genuine definition packets and full player/member inputs. Its strict
input structs contain no `plot_expected` observations. Full reconstruction mode
sets `require_domain_metadata:true`; absent metadata is an error. It decodes all
Cities with the complete current City codec, constructs the production Registry,
derives coastal/Wonder queries and invokes the unchanged core two-pass rebuild.
Any missing prerequisite aborts without a partial output or skipped plot.
Absent CityAttributes produce an explicit absent mask entry; this is distinct
from an unsupported plot. The source fixture tests the full composition with
different container/member owners and stale requested generations. This does not
extend the stated native coverage to reserved-entry fallback state.
The follow-up candidate also represents auxiliary Wonder results as absent for
plots without owned attributes. A direct Wonder query there returns
MissingWonderCompletion: original GetCityAttributes may return reserved entry0,
whose later mutations have not been closed. Neither an absent mask nor an absent
Wonder result may be converted to a zero/false native fallback answer. A future
comparison reports those fallback queries as unavailable explicitly.

The [historical reconstruction manifest](../evidence/linux/movement-domain-reconstruction-candidate-65d71d4.json)
freezes source `65d71d4`, clean Git archive SHA-256 `399a64bc…`, and candidate
SHA-256 `eeb2e434…` (26,220,480 bytes). The source archive and independent build
remain under `/tmp/c6-domain-reconstruction-65d71d4`. All 60 recorded build-source
and embedded-input hashes match committed source. Isolated data validation passes
21 tests with four explicit private-fixture skips and strict all-target clippy.
The [preparation report](../evidence/linux/movement-domain-reconstruction-preparation.json)
repeats all 3,072 District and water comparisons and 6,996 source coastal/neighbor
calculations. The full genuine-metadata path has only source-fixture validation
until a fresh independently admitted original capture exists; native coastal,
Wonder and raw-mask parity remain unclaimed. This manifest is the runner's
candidate identity gate, not acceptance of a new native execution profile.
It remains preserved as history and is superseded for new comparison by the
corrected candidate's explicit unavailable fallback-Wonder output.
The [current manifest](../evidence/linux/movement-domain-reconstruction-candidate.json)
freezes corrected source `60507af`, archive `37772c67…`, and candidate `e899e4d9…`
(26,239,736 bytes) in `/tmp/c6-domain-reconstruction-60507af`. The
[corrected preparation report](../evidence/linux/movement-domain-reconstruction-preparation-corrected.json)
repeats the same bounded checks with 21 passing tests, four explicit skips and
strict clippy. Reserved-entry raw masks and auxiliary Wonder results remain
explicitly unavailable pending their own state/mutation proof; no native parity
is claimed for those missing results.

Such exports need the same pinned module/host/profile and original owned Core
association, closed execution, unchanged affected-state controls and RNG gates.
No original game code was executed for this task. Initialization rebuild also
does not implement later construction, pillage, destruction or transfer updates.

Reproduce with an independently built offline candidate:

```sh
cargo build -p c6-data --example city_movement_inputs
python3 tools/linux/movement_domain_inputs_check.py \
  --candidate target/debug/examples/city_movement_inputs \
  --source-commit COMMITTED_SOURCE_REVISION \
  --output target/linux/movement-domain-inputs.json
```

Complete Linux gameplay state and RNG, external policy decision boundaries,
actions and turns remain the objective. The recorded raw **89,198-byte Save→Load
difference remains unresolved and unmasked**.
