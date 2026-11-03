# Linux Unit initialization definition inputs

The bounded Unit28 runtime initializer now has a concrete source for its four
previously missing definition collections. `c6_data::unit_initialization_definitions::Definitions`
builds them from the admitted activated Linux rules snapshot and requires actual
resource equivalence with the existing loaded-action resources. This is live
array initialization support, not complete Unit loading, full SkipTurn or a
complete state/action/turn simulator.

The initializer requires six collections: Units, UnitAbilities, Terrains,
Features, GreatPersonClasses and UnitOperations. The existing closed placement
and Manager captures supply Units/domain hashes and UnitOperations. This builder
supplies the remaining four: 155 abilities, 17 terrains, 31 features and nine
GreatPersonClasses in the admitted profile. It does not supply the separate UnitPromotion,
RockBand yield/district/improvement collections or other enum bindings needed to
initialize every nested component; those remain explicit gaps.

The pinned Linux SQL getters select every row from their respective table and
use `ORDER BY rowid`. There is no WHERE clause, join, inactive-ability exclusion
or natural-wonder exclusion. Specialized collection builders construct one
object per query row, assign the current collection size as its index and append
it. SQL row identity remains separate from collection index, including zero,
negative and gapped rowids.

| Collection | SQL getter | Collection builder | Type column position |
| --- | --- | --- | --- |
| UnitAbility | `0x221c410` | `0x225f580` | 6 |
| Terrain | `0x2219e80` | `0x225e380` | 13 |
| Feature | `0x21f9150` | `0x22517e0` | 12 |
| GreatPersonClass | `0x21fc580` | `0x2253be0` | 5 |

The positions include selected rowid at position zero. Each original type setter
calls Linux `MakeHash` on the type bytes. The two-vector collection first handles
small numeric indexes, then searches unsigned hashes. Insertion uses lower_bound,
so the newest colliding definition wins. The existing Linux registry implements
these contracts; the builder does not replace them with SQL primary keys or
case-insensitive string lookup. Unsupported signed-rowid narrowing extremes and
embedded-NUL type names error explicitly.

[The static audit](../evidence/linux/unit-initialization-definition-builders.json)
fingerprints 69 complete original methods across SQL getters, row/collection
builders, setters, hash getters, lookup and insertion helpers. It gates the exact
supplied Linux GameCore and host SHA-256 identities. Discovery reused the pinned
cru121 CC BY 4.0 [reference](reference.md); actual target bytes are authoritative.
Proprietary assembly remains in
`../c6-ghidra/evidence/linux-unit-initialization-definitions/`.

`Definitions::from_rules` accepts only the registered
`LinuxXp2ActivatedDiagnostic` Rules object, which has already checked both real
SQLite files and all 506 tables. It pins the original final-1 rules report and
checks its configuration identity against that Rules source. The bound action
resource adapter then compares actual module assets, base/full content gates,
saved configuration, DLC descriptors and activated mod/component values. The
rules report and loaded observer profile hashes may differ. Matching process
lifetime, equal Core bytes or a bare GameCore hash supplies no replacement proof.

After that one-time setup, the caller combines the four registry getters with
`Bound::unit_definitions()` and `Bound::operation_lookup()` to construct
`InitializationInputs`. Its identities are borrowed from the admitted loaded
state. The core initializer's exact input-identity equality remains a consumer
binding; it is not the cross-observer resource equivalence test. No logs or full
Core hashes need reprocessing per Unit.

The [offline result](../evidence/linux/unit-initialization-definitions-check.json)
passes for all 1,262 Unit and 1,250 Formation sidecars. Its harness uses the independent review's public strict-capture helper,
`tools/linux/loaded_actions_review_load.rs`. It admits the actual native-3
artifacts and moves the decoded roster into World. It compares the complete
saved array hash order against all four rebuilt registries and the existing
operation registry, initializes every Unit runtime sidecar, and checks the live
array values through the public Unit-only initializer. Manager controls are not
invented for this preparation. Formation owner initialization uses the separately
source-proven enclosing Player Read identity; it does not construct a complete
Manager mapping from saved rosters. The check verifies every logical archive
identity, resolved Unit Formation identity, Formation evaluating-operation reset,
its local Read dirty diagnostics and rejection of repeated initialization. Comparing encoded Unit and Formation bytes before and after is a separate
nonmutation diagnostic, not an action-acceptance gate.

```sh
python3 tools/linux/unit_initialization_definitions_audit.py \
  --private-output ../c6-ghidra/evidence/linux-unit-initialization-definitions \
  --output evidence/linux/unit-initialization-definition-builders.json
cargo test -p c6-data unit_initialization_definitions
python3 tools/linux/unit_initialization_definitions_check.py \
  --output evidence/linux/unit-initialization-definitions-check.json
cargo clippy -p c6-data --all-targets -- -D warnings
```

The harness reuses original saved observations; it does not execute original game
code or compare unsaved native initialized Unit state. Complete Read dirty
history, actual gameplay callbacks, nested remaps and native gameplay/RNG
comparison remain separate requirements. The raw 89,198-byte Save→Load failure
remains unclassified and unmasked. Save production is not required. External
policies continue to supply every genuine player decision.
