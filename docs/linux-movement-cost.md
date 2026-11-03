# Linux adjacent movement-cost boundary

The Rust [movement module](../crates/c6-core/src/movement.rs) ports the coherent
cost accumulator, nine movement rule phases, formation ignore aggregation and ZOC
hard-minimum selection over explicit resolved query facts. This is a bounded
arithmetic implementation with original-target static evidence. **Native
differential acceptance is outstanding.** It does not yet resolve those facts
from a complete world, test movement legality, execute a movement action or
advance turns. Complete gameplay state/action/turn equivalence remains unfinished.

The target is Linux XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, with
host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Run `python3 tools/linux/movement_cost_audit.py --output target/linux/movement-cost.json`.
The [checked static report](../evidence/linux/movement-cost.json) fingerprints
37 original methods, verifies all nine phase calls/tail calls, the shore-phase
Plot virtual slots and the fixed-point float multiplier. Matching Linux DWARF
corroborates the layout below. Private original instructions and DWARF are in
`../c6-ghidra/evidence/linux-movement-cost/`. These are static checks, not original
execution or passing gameplay parity tests.

Discovery consulted existing [Map state](linux-map-state.md),
[Unit/Formation state](linux-unit-codec.md),
[geometry](linux-geometry.md), [rules identities](linux-definition-ids.md), and
cru121's pinned *Civ6-GameCore-reference* revision
`5af389445118bed2bdebf7a8a42b219c8fba5175`, especially the function index and
constant-stub findings. Attribution: cru121, CC BY 4.0; see
[reference provenance](reference.md). Reference names supplied leads; the actual
Linux target bytes determine the contracts. No Windows acceptance is transferred.

## Actual calculation boundary

| Original Linux method | RVA | Role |
| --- | --- | --- |
| `Formation::MovementCost::Values` default/parameter constructors | `0x2945cc0` / `0x2945d40` | Initialize accumulator and query inputs |
| `Values::GetFinalCost` | `0x2945de0` | Ordered final cap/floor/fallback |
| `Movement::MovementCostNoZOC` | `0x2948430` | Construct, run rules, return final |
| `Movement::MovementCost` with formation/plots | `0x29484e0` | Construct, run ZOC-aware overload, return final |
| `Movement::MovementCost` with Values | `0x2949170` | Select hard minimum, run rules, return final |
| `Movement::ApplyMovementRules` | `0x2949850` | Dispatch nine phases in fixed order |
| `Movement::IsSlowedByZOC` direct/cache forms | `0x2949740` / `0x2949060` | Separate spatial query |
| `Rules::Movement::IsInEnemyZOC` plot form | `0x2becf40` | Visibility/diplomacy/unit/district query |

The formation/plot overload's third plot parameter is passed as the ZOC query's
exception plot. It is not another terrain-cost input. The no-ZOC wrapper bypasses
the entire hard-minimum prelude, including the usual exit-moves minimum; invoking
the ZOC prelude with a false result is therefore not equivalent to that wrapper.
Its flags and extra plot do not affect the nine phases. The ordinary wrapper
constructs exit moves as zero; the Values overload permits nonzero exit moves.

`Values` is 64 bytes, including four trailing padding bytes:

| Offset | Type | Field |
| --- | --- | --- |
| 0 / 8 / 16 | pointer | Formation / source Plot / destination Plot |
| 24 | signed Q8 | Base moves, constructed from an integer |
| 28 / 32 / 36 | signed Q8 | Maximum / remaining / exit moves |
| 40 | i32 enum | ZOC selector: 0 query, 1 not slowed, 2 slowed |
| 44 / 48 | signed Q8 | Hard minimum / soft maximum |
| 52 / 56 | signed Q8 | Regular cost / destination exit addition |

The initial accumulator is `(hard minimum=0, soft maximum=0x7fffff00,
regular=0, exit addition=0)`. Hard-minimum updates take the signed maximum;
soft-maximum updates take the signed minimum. Regular and exit additions use
wrapping 32-bit Q8 addition. Final cost first caps regular by soft maximum,
then raises it to hard minimum; if the result is nonpositive, it becomes **one
whole movement point (raw 256)**. It is not unconditionally capped by remaining
moves. Exit addition is a separate output, excluded from GetFinalCost.

The ZOC prelude ignores ZOC when flags bit 11 is set. Otherwise selector 2 is
slowed, selector 0 calls IsSlowedByZOC, and selector 1 is not slowed. A slowed
move raises the hard minimum to remaining moves when positive, otherwise to
maximum moves. An unslowed move raises it to exit moves. The Rust enum represents
the three evidenced selectors; arbitrary native enum bit patterns are outside its
typed input contract.

## Rule phases and exact inputs

The Rust evaluator supports non-null formation/source/destination objects.
Optional costs represent actual null definition lookups; they are not unknown
values, and must never be populated by invented defaults. Missing unresolved
query facts must prevent the caller from invoking the evaluator.

1. **City** (`0x2bed5a0`): destination IsCity lowers soft maximum to integer
   `CITY_MOVEMENT_COST`.
2. **Terrain** (`0x2bed5f0`): resolve destination TerrainType to its definition.
   If present, add integer MovementCost, or 1 when the formation's terrain-index
   ignore query returns true. There is no separate hills surcharge in this phase;
   terrain identity and its definition cost determine it.
3. **Feature** (`0x2bed680`): resolve destination FeatureType to its definition.
   Add integer MovementChange when present and the formation's feature-index
   ignore query returns false.
4. **Improvement** (`0x2bed700`): resolve destination ImprovementType and add its
   integer MovementChange when present. This phase has no pillage/ignore test.
5. **Route** (`0x2bed760`): both plots must IsRoute and neither may be route
   pillaged. A river crossing additionally requires IsBridgeBetweenPlots. Resolve
   both route definitions; if both exist, lower soft maximum to the larger endpoint
   float MovementCost converted to Q8 as described below.
6. **River** (`0x2bed890`): an actual IsRiverCrossing result adds integer
   `MOVEMENT_RIVER_COST` unless the formation ignores rivers. Bridge presence does
   not suppress this addition; the route soft maximum can cap the resulting cost.
7. **Portal** (`0x2bed8f0`): if actual wrapped-map IsAdjacentPlotCoord is false,
   query the source portal and its HasExitAt(destination). If connected, add integer
   `MOVEMENT_PORTAL_COST`. Adjacent movement never incurs this phase's addition.
8. **Embarking** (`0x2bed9f0`): requires formation CanEmbark, destination water
   requiring embark, and source raw IsWater false. Water requires embark unless
   HasCityAttributes and CityAttributes::IsDomainValid(formation domain).
9. **Disembarking** (`0x2bedbb0`): requires formation CanEmbark, source water
   requiring embark, and destination raw IsWater false.

Both shore phases compute a free-embark flag as follows, preserving the supplied
target's observed instructions rather than substituting a conventional rule:

```
free = false
if source district != NO_DISTRICT and source IsDistrictComplete:
    free = source district definition IsFreeEmbark
if destination district != NO_DISTRICT and source IsDistrictComplete:
    free = destination district definition IsFreeEmbark
if not free:
    free = formation GetIgnoreShores
if not free:
    regular += integer MOVEMENT_EMBARK_COST
if base moves < remaining moves:
    destination exit addition += remaining moves - base moves
```

The second completion query is on the **source**, and the destination flag
**replaces**, rather than ORs with, the source flag. The relevant source receiver
loads and assignments are `0x2bedafa..0x2bedb28` and
`0x2bedcc0..0x2bedcee`. A separate mechanics reviewer independently confirmed this branch against the
supplied instructions. Live original fixtures remain necessary before behavioral
acceptance. The Plot virtual calls
were independently checked through actual ELF relocations: slot `+0x38` is
GetCityAttributes (`0x2bca780`), slot `+0x48` is HasCityAttributes (`0x2bca7b0`).
CityAttributes domain-valid state is not serialized in the current Plot codec;
its exact reconstruction remains a resolver dependency.

Route selection uses UCOMISS/CMOVA: choose destination only when its float cost
is ordered and strictly greater than source; otherwise choose source. NaNs and
ties consequently select source. Do not use Rust `f32::max`. Conversion
(`FixedPointT<8>::operator=(float)`, `0x1fceb70`) calls floorf, converts the whole
part with CVTTSS2SI, shifts left eight, computes the f32 fractional difference
times 256, truncates with CVTTSS2SI, keeps its low byte and ORs that with the whole
part. Invalid x86 conversions produce `i32::MIN`, unlike Rust saturating casts.
The implementation preserves those output bits, including nonfinite inputs;
floating exception flags/errno are outside the scalar contract. The numerical scope is x86_64 Linux binary32, default round-to-nearest, masked
floating exceptions and DAZ/FTZ off. The scalar contract does not model MXCSR
status, NaN payload propagation, exceptions or arbitrary floating modes. A tiny
negative subnormal can quantize to raw -256: floor is -1, fractional subtraction
rounds to 1, the scaled fraction is 256 and its low byte is zero. This is not
mathematical floor(value*256). Native numerical fixtures remain outstanding.

## Formation aggregation and remaining resolver work

Formation GetIgnoreCost(terrain) (`0x293d4d0`), GetIgnoreCost(feature)
(`0x293d590`), GetIgnoreShores (`0x293d6d0`) and GetIgnoreRivers (`0x293d830`)
iterate formation member IDs through the owning Player's Units::Find. Missing
IDs are skipped. Begin with true; a resolved nonignoring member clears the
aggregate, while an ignoring member with HasEscortMobility returns true
immediately. Otherwise return whether all resolved members ignored. Thus the
empty resolved set returns true. The generic Rust helper consumes the actual
per-unit getter result plus escort mobility; it does not equate terrain, feature,
river and shore ignores or infer any from one unit.

Unit GetIgnoreTerrain (`0x2c27cc0`) bounds-checks using the invalid-terrain vector
size at `+0x378` but reads the ignore-terrain vector at `+0x3a0`. GetIgnoreFeature
(`0x2c27d00`) checks/reads the feature vector at `+0x3c8`. Both reject enum -1.
Do not replace these with the similarly named global ignore-terrain-cost byte.
The wire codec stores definition hashes; native getters consume runtime collection
indices. Correct hash-to-index remapping, vector lengths and default slots must
be established before a typed Unit resolver can claim equivalence.

Compact resolver inputs still needed for a world-backed evaluator are:

- Formation member IDs, owner Units lookup, per-unit type-ignore predicates and
  escort/river/shore modifiers; formation CanEmbark, domain and ZOC cache fields.
- Destination terrain/feature/improvement definitions; both route definitions and
  original float bits; four configured integer global parameters.
- Actual IsCity/IsWater, route presence/pillage/bridge flags, district type and
  source completion, district FreeEmbark and reconstructed city domain validity.
- Map dimensions/wrapping and adjacency, directional river crossing and portal
  source/exit relation. IsRiverCrossing (`0x2bed3b0`) rejects null/water/city endpoints
  and null map, then uses source IsRiverCrossing(GetDirectionToPlot). Bridge
  (`0x2bed460`) requires a river crossing and both routes present, bridged, unpillaged.
- Base/max/remaining/exit move budgets and the ZOC selector/flags. The spatial ZOC
  query depends on location layer, unit stacking limit, friendly units of type,
  district defensibility, diplomacy, visibility, enemy domain/religion, unit death
  flags, ExertsZOC and directional rivers. Its neighbor callback and transitive
  queries remain unported; supply a real resolved result, never a proximity guess.

There is no direct RNG call or gameplay-state store in the inspected scalar
accumulator/phase instructions. They mutate Values only and call world/definition
queries. This does not prove transitive query purity or eliminate cache/provider
side effects. Native validation must compare affected state and all gameplay RNG
streams, rather than assuming a query is harmless from its name.

## Native validation plan and current checks

The sole native runner must first gate host, XP2, dependencies, actual rules/DLC/mod
configuration and loaded fixture identities. Use an actual loaded formation and
its actual adjacent plots, not a synthetic object for gameplay acceptance. Observe
all resolver facts above, invoke the phases in original order, record every Values
field before/after each phase, and compare Rust final cost and exit addition.

`ApplyMovementRules(Values*)` at `0x2949850` takes RDI=Values. Each rule method
takes RDI=Rules::Movement::Instance and RSI=Values. GetFinalCost at `0x2945de0`
returns the nontrivial FixedPoint through RDI=output-i32 pointer, RSI=const Values;
it is not an integer-return C ABI. The Values constructor at `0x2945d40` takes
RDI=this, RSI=formation, RDX=from, RCX=to, R8D=integer base,
R9=pointer to max Q8; stack arguments are pointers to remaining/exit Q8 and the
ZOC enum. A runner may initialize the verified POD words while preserving all
padding separately, but should cross-check constructor output first.

Representative live cases must cover plain/hill/feature terrain and type ignores,
different/pillaged/bridged routes, river ignore and bridge interaction, city cap,
embark/disembark with both district combinations, moves below/equal/above base,
ZOC remaining/max/exit selection and bit-11 bypass. Include signed overflow,
negative/nonfinite/fraction-boundary route conversion in a separately labeled
scalar native corpus. Unsupported fixture coverage remains a gap. Before/after
Core serialization and per-stream RNG state must accompany original results;
preserve raw Save→Load differences, including the separately recorded
89,198-byte failure, without masking or classifying them as planning state.

Six Rust unit tests currently check cap/floor order, wrapping, float conversion,
escort aggregation, route/river/ZOC interaction and the exact shore branch. They
are arithmetic regression tests, not native parity or full movement acceptance.
