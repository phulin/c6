# Linux serialized movement input integration

The production [adapter](../crates/c6-data/src/city_movement.rs) derives District
identity, location, parent City, runtime type and completion from real serialized
state. Its offline comparison matches **3,072 / 3,072 endpoints** in the accepted
Linux fact control3: 1,282 positive resolutions across 298 distinct Districts and
1,790 invalid-ID misses. This removes runtime type/completion as calculation
inputs for this bounded comparison. It does not reconstruct the 257 required
runtime domain-mask observations or establish complete gameplay state/actions/turns.

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

Player validity is still an independent input to general `city_district::Registry`
construction. Each positive native lookup in this comparison proves validity for
that particular queried player. Every missing District request is `UINT_MAX`,
whose generic invalidity precedes native player lookup. Consequently this fixture
comparison needs no fabricated complete player-validity map. Owner appearance in
serialized records does not prove player validity. This checker does not validate
general missing-slot or invalid-player behavior dynamically.

`initialization_domains` composes the existing exact two-pass core rebuild with
`map_is_water`, using actual Terrain definition order/properties and the original
15/16 fast path. The provider must still supply exact IsCoastalLand(1), Wonder
completion and original directional neighbors; required unavailable values remain
named errors. It retains the existing Canal, neighboring City, signed-short
Building remapping/Find, CanalWonder and independent Bridge completion behavior.
It does not replace the coastal query with adjacent-water intuition or infer
building presence from unremapped sparse City building rows. Source-only branch
tests in core remain distinct from this native District comparison.

The existing authoritative movement export schema and original control3 JSONL
provide Terrain/Feature/Improvement/Route/District registries only. District rows
carry FreeEmbark, not Canal/InternalOnly. The data registry importer contains no
Building collection loader/order proof. The newer complete City wire codec
preserves Building location/pillage hashes, but it does not prove the original
IsWonderComplete call chain or Building index order. Therefore zero masks are
claimed calculated or compared here. SQL table iteration is not an alternative
native collection export.

The minimal next export for the sole original-code runner is:

- Complete actual valid-player mapping for all 65 slots, independently of owners.
- Actual District collection order/hash plus original Canal and InternalOnly
  getter results, including explicit absence of optional XP2 definitions.
- Actual Building collection count/order/hash, CanalWonder and Bridge getter
  results, and the original Find mapping needed after signed-short narrowing.
- Per attributed plot original IsCoastalLand(1), original six ordered neighbor
  indices, and original IsWonderComplete results, alongside the existing raw mask
  for comparison. These are named intermediate inputs until their typed-state
  contracts are closed; raw masks remain expected outputs, never calculations.

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
