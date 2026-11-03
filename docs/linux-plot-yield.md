# Linux numerical plot yields

`c6_core::plot_yield::get_yield` implements the numerical control flow of original Linux XP2 `Rules::Yields::Instance::GetYield` at `0x2bf5ad0` over a concrete owned `World`, native ordered definition inputs, the existing typed City/District registry, and explicit live query providers. It is a source implementation with source arithmetic tests. Initialized native differential acceptance, complete gameplay state, action execution and turns remain unimplemented.

The [read-only audit](../tools/linux/plot_yield_audit.py) and [result](../evidence/linux/plot-yield-source.json) gate the exact original XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and matching Linux host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`, twelve source bodies, seventeen instruction contracts and six calls. The audit never executes original code. Full-host admission and native numerical comparison remain false. The separate 89,198-byte Save→Load failure remains unclassified and unmasked.

The original function body is retained privately at `../c6-ghidra/evidence/linux-reporting-provider/focused-contracts/rules-plot-yield.asm`. Existing getter evidence is under `../c6-ghidra/evidence/linux-reporting-yield-effects-local`. The earlier [query closure](linux-reporting-yield-query-closure.md) describes the accepted callback profile and property construction provenance; this implementation does not generalize that profile's effects classification to other rules or property providers.

## Implemented arithmetic

The public entry point accepts a real Map plot index, requested yield index, fallback player and the original boolean argument. `YieldMonitor` supplies a concrete native Plot and false. Hypothetical Plot subclasses, nonnull improvement contribution lists and hypothetical map changes are outside this API. Rules, enabled content, saved configuration and module identities must be gated before importing its inputs.

The function preserves these source branches:

- A resolved wonder returns zero before terrain queries. The native wonder index is narrowed to signed 16 bits. Impassable plots return zero unless their political owner can work that terrain.
- A completed non-city district returns the sum of all matching district citizen rows and all matching rows of buildings at that plot. Null citizen references are skipped. The boolean argument suppresses this branch before completion and building queries. Terrain, adjacent features and climate are not evaluated after its return.
- Terrain uses the first matching hash/yield row. All six adjacent directions are visited only for that matching row. Any double-adjacent-terrain feature sets the multiplier to two; additional such neighbors do not multiply it again.
- A natural wonder feature removes the terrain contribution. Feature and visible-resource base changes each use the first matching row, while stored Feature-manager extra yield is added independently.
- A present unpillaged improvement contributes only after a matching declared base-yield guard. Its numerical helper uses the same imported base rows and registry, avoiding an independent guard table. The [improvement helper](linux-improvement-yield.md) composes its first base row, every adjacency reference, technology and civic bonuses with a null contribution list. It uses the political owner, including the original unowned zero path. The builder owner is not substituted.
- Appeal contribution performs a wrapping 32-bit multiply before signed division by 100, then the original integer→FixedPoint8→AsInt conversion. The latter conversion can wrap even after division. Owned appeal comes from the actual signed-short Plot cache; unowned recomputation requires a provider.
- Improvement properties, city minimum food/production, and CityPlots `PlotYieldData` are added in source order. City minimums precede neighboring and later player/city bonuses.
- All six neighboring features accumulate every matching adjacent-yield row. Neighboring district appeal rows retain the owning-city, same-owner, unimproved, building-location, building-pillage and inclusive appeal-range guards. Missing appeal rows are demanded only after owning-city and owner guards survive.
- Player great-person/resource bonuses, city flood counts, city terrain-adjacent-improvement bonuses and terrain-class city counts retain the native guards and integer/fixed-point conversions. Adjacent improvement bonuses do not gain an invented pillage check.
- Food drought loss still applies with the boolean argument true. It subtracts with 32-bit wrap and then clamps a negative signed result to zero. City drought protection follows original owner/city lookup guards.

All integer additions and multiplications wrap at native width. FixedPoint8 conversion shifts and wraps at 32 bits; AsInt uses an arithmetic right shift, including for negative values. `YieldTypes::from_name` implements the original six-hash switch over supplied actual `YIELD_*` global values. It does not accept arbitrary registered yield hashes or small integer indices as names.

## Concrete state composition and remaining providers

`Context` borrows the actual `World`, the existing concrete City/District registry and native direction-ordered adjacent plot indices. `Definitions` aligns properties with the supplied native definition registry. Direct At, small-index/hash Find, last duplicate hash selection and signed-short Plot index narrowing remain distinct. SQL row IDs and guessed hash orders are not substitutes. Definition references and rule rows must retain original order and duplicates. `None` reference slices mean uncaptured input; an empty slice means known empty.

`FeatureYieldStorage::from_world` reconstructs the actual Feature component's stored sparse IndexLists from `World::map().components`. This is gameplay component storage, not monitor cache replay or a table of final plot yields. It imports the sparse default as well as every explicit value. Original `IndexList.Read` (`0x2194fb0`) resets the registry-sized vector to zero, then `ReadIndexHashArray` (`0x2002e80`) overlays ordered hash/value pairs through native `HashToIndex` (`0x1ff7260`). Unknown hashes consume their values without writing; duplicate known hashes overwrite. Const sparse lookup returns the imported sparse default for missing, negative or out-of-range plot indices. Const IndexList lookup returns its reset zero default for invalid yield indices. Missing Feature components, duplicate components, inconsistent indices and definition bounds are explicit errors.

The improvement property path directly handles an absent Plot map, a missing improvement key and a native typed improvement map missing key `877b95f0`. That includes the accepted input's empty nested improvement maps. Other conversion kinds, duplicate improvement keys and present per-yield property maps require the explicit property provider; they are not treated as empty. Foreign property virtual classes remain outside the established typed-map provenance domain.

Remaining live inputs have separate read-only methods on `Runtime`. Each default returns `Error::Provider`; there is no default zero or invented availability. The methods cover:

| Input | Required meaning |
| --- | --- |
| Terrain work permission | Original political-owner terrain-work array/query |
| Visible resource | Original resource reveal query for the effective player; native player `-1` and `1000` bypass it |
| Present improvement properties | Exact native keyed-map and typed-variant conversion semantics |
| Unowned appeal | Original appeal/ring/governor calculation |
| Buildings and CityPlots effects | Original building-location order, pillage state and real `PlotYieldData` |
| Great-person resource changes | Original valid-player lookup and acquired great-person class/resource calculation |
| Flood changes | Actual city per-flood field and existing named-river `GetFloods(-1)` count |
| City terrain modifiers | Original presence query, FixedPoint8 getter, improvement and terrain-class behavior |
| Distinct city terrain count | Original player-city statistic |
| Climate | Actual drought type at plot, city prevention and supplied global loss parameter |

The concrete World prerequisite implementation supplies the improvement technology/civic helpers. Real current player mapping, acquired masks and global node availability must already be installed; missing inputs propagate errors. Unavailable unrelated providers are not demanded by skipped branches. The query takes immutable references and never writes World, RNG, monitor caches or reports. It cannot establish that an arbitrary external provider is pure; the accepted provider/profile evidence remains required.

There is no new native oracle run or runtime capture in this checkpoint. Existing callback acceptance still lacks all live provider imports and native numerical comparison across affected state. Monitor cache updates/reporting, trade queries, SkipTurn legality/aftermath, decision boundaries and complete turns remain separate work.

## Validation

```sh
cargo test -p c6-core plot_yield
cargo test -p c6-core improvement_yield
cargo clippy -p c6-core --all-targets -- -D warnings
python3 tools/linux/plot_yield_audit.py --output target/linux/plot-yield-source.json
```

The twelve focused plot tests cover first-match and all-row behavior, doubling capped at two, natural wonder replacement, null references, concrete sparse Feature reconstruction and duplicate/unknown hashes, political-owner improvement composition, negative appeal fixed-point arithmetic, district early returns, missing-input demand order, immutable World on failure, city-minimum-before-effects ordering, extreme terrain multiplication, empty-versus-present property demand, wrapping drought subtraction, inclusive adjacent-appeal bounds/building guards/owner-demand ordering, and late great-person/flood/terrain-class overflow with repeated-query order and negative fractional fixed-point floor. Explicit missing great-person, flood, unowned appeal and present-property providers are tested. They are source fixtures, not comparisons against executing original code.

Independent source review identified and corrected the standalone improvement composition boundary: original `0x2a3ee93` calls direct-index `At`, so out-of-range hash-valued arguments return zero before providers. Its technology/civic callees retain their separately evidenced hash-capable `Find`. The focused audit pins that distinction; the outer plot caller supplies an actual improvement index.
