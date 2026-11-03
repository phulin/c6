# Linux headless yield-monitor boundary

`MessagesProcessed` (`9ea4c222`) reaches `YieldMonitor.OnMessagesProcessed`
at `2c66430`. With the accepted one active human it runs plot checking, then
trade checking. This is a synchronous effect boundary even though the monitor's
own two vectors contain derived output. The current checkpoint does **not**
authorize blanket omission of this observer or claim completed SkipTurn.

The [focused audit](../tools/linux/yield_monitor_headless_scope_audit.py) and
[result](../evidence/linux/yield-monitor-headless-scope.json) reuse the existing
[plot effects](linux-reporting-yield-effects.md),
[conditional plot-query closure](linux-reporting-yield-query-closure.md), and
[trade effects](linux-reporting-trade-effects.md). It checks 142 supplemental
body identities against the pinned Linux XP2 bytes. No original code executes.
Hash checks retain the reviewed boundary; they do not automatically establish
purity of every transitive callee.

`InitializePlotYields` (`2c660f0`) fills the derived cache using the same
`Plot.GetYield(false)` operation as checking. `InitializeTradeYields`
(`2c661f0`) visits actual active routes and inserts their computed origin and
destination yields under the original sorted CityID pair. The monitor global
at `3c06b20` is 48 bytes; its Get/Edit accessors return that existing object.
The checking bodies compare each computed scalar and update only that scalar
when unequal. Plot changes synchronously send `1f943311`, followed by aggregate
`8fef0d8e`; trade changes send `14deda5a` for the endpoint identity pair.

The passive-4 monitor preimage has 6,996 plot rows with six yields each and 182
trade rows. Packing CityID as native unsigned `ComponentID.GetBits` gives 182
strictly sorted, unique trade keys. The active route endpoints are unique and
bijectively match those keys: actual `lower_bound` returns 182 distinct exact
slots. This matters because the native callback checks only against end, not
key equality. All 182 active forward paths are nonempty, containing 6,193 nodes.
This is input-domain evidence, not observation of native graph lookup hits.
The authoritative graph owner must perform the actual Route-city lookup;
outer edge keys must not be silently substituted for Route endpoint fields.

The supplemental getter inspection distinguishes output writes from source
mutation. Seven `InitializeOnUse<int>` counts live on the call stack.
District/resource count operations traverse existing city/plot/player storage
and accumulate local integers. The City ParentPtr constructor writes the
caller-local pointer, and its destructor is a single return; these calls do
not mutate gameplay ownership. Progress counts traverse existing civic/tech
storage, with the actual false argument excluding repeatable progress.
`GetBestRoutePossibleInGame` scans definition placement values and does not
initialize a persistent route cache. City terrain/improvement modifier scans,
religious follower counts, and river flood counts read existing vectors and
produce caller-local results. The actual `includeTempRoute=false` branches
and null plot contribution argument retain their earlier source-proven
exclusions. Cold pathfinding remains outside this proof.

An exact-literal scan of executable LOAD segments finds only the two plot
producers in GameCore and their host translator registrations. Supplied Lua
consumers refresh plot lens/icon state (and a workshop UI analysis dirty flag).
This scan is bounded evidence about these literal event IDs, not a census of
arbitrary computed aliases or additional registrations.

Trade-change reporting has a concrete additional native consumer:
`Cache.City.EventStream.Manager.RegisterEvents` contains `14deda5a` at
`27162b4`. `OnTradeRouteActivityChanged` (`27170f0`) publishes snapshots for
both endpoint cities. Snapshot construction includes city yields, growth,
religion, cultural identity, power and AI CityBuild output. The latter invokes
real `CanTrain`, `CanConstruct`, `CanAdvance`, and `CanZone` legality queries.
Their complete transitive effects are not closed here. An AI or cache name
does not justify omitting those calls. Changed-city refresh therefore remains
unsupported unless its effects are independently established.

A narrower omission can avoid that unexecuted branch: establish a **fresh
derived cache from the current authoritative inputs**, prove query purity
within the admitted property/appeal/warm-path domain, and preserve those exact
inputs through the supported action. Equal pure function inputs then entail
equal derived values, so no generated refresh event occurs. This does not
require storing or exposing numerical yield answers. The unchanged passive-4
endpoints corroborate the observed interval; they cannot transfer unsaved
cache freshness to a different runtime merely because saved bytes match.

Any implementation of this elimination must own initialization/history and
invalidation authority. It must identify actual Unit/Formation dependencies
and admit only the proved setters, rather than treating all Unit writes as
irrelevant. Diplomacy, city/player/plot/trade mutation or any broader world
mutation invalidates the unchanged-input argument. Missing history, foreign
property providers, different appeal definitions, cold paths, changed-output
consumers and added registrations must remain explicit unsupported cases.
External policies retain genuine yield-query boundaries; omitted presentation
work must not provide guessed numbers or replace missing runtime providers
with zero. Existing numerical `plot_yield::get_yield` retains its provider
contract separately.

Reproduce this static/input check without native execution:

```sh
python3 tools/linux/yield_monitor_headless_scope_audit.py \
  --output target/linux/yield-monitor-headless-scope.json
```

Full affected-state/RNG, complete actions and turns remain unimplemented.
Historical report fixtures, unknown save bytes and the separate 89,198-byte
Save-to-Load difference retain their existing scope.
