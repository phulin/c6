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
[trade effects](linux-reporting-trade-effects.md). It checks 146 supplemental
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

The [owned data adapter](../crates/c6-data/src/loaded_actions/yield_monitor.rs)
admits the genuine Trade.Graph serializer frame once: save1, depth2,
call162111, parent162110, byte range `[71027800,71497438)`. It checks complete
Core image, owner trace, frame and binding identities before passing the
witness to LoadedState's immutable graph owner. Its `warm_route_domain`
queries each active Route's actual city fields through that owner and validates
every returned plot index. It returns counts and current revision, not computed
yields, cache freshness or permission to skip the observer. A missing/empty
forward path remains unsupported before any pooled finder executes.

Native cache initialization has a specific lifecycle origin: Game.Read
(`2d7aaa0`, `9ec` bytes) calls Monitor.Initialize at `2d7b264`, following city
domain reconstruction. This occurs **before** Tech/Culture InitializePostLoad,
Heroes/Analysis/Player/Rules/Barbarian/GlobalAI postludes, conditional City
destruction, and Processor.InitializePostLoad. Tech/Culture postload can build
their collections if either required vector is empty. Consequently this call
alone does not certify fresh derived values at action entry. The post-init
history remains an explicit missing authority; an abstract initialization at
the action boundary would silently replace potentially stale native state.

The [postload history audit](../tools/linux/yield_monitor_postload_history_audit.py)
and [source result](../evidence/linux/yield-monitor-postload-history.json)
pin 30 focused bodies and nine exact calls. Player postload invokes concrete
BehaviorTree/Control initialization, guarded Unit sight population, and City
Buildings/Citizens initialization. BehaviorTree handle initialization rebinds
saved owner/contract identities. The Control manager subsequently invokes
existing Contract virtual `+0x10` at `201e38b`; these transitive effects remain
unclosed. Its namespace is not a reason to omit a gameplay write or callback.

City Citizens postload (`286eb60`) has a direct intersection with monitor
inputs: a true `force_update_plot_appeal` clears the flag and calls
`Rules.Appeal.Update` for each nonnull city plot. The resulting cached appeal
can affect plot yields. City Buildings postload can clear a GreatWorkSlot ID
whose indexed GreatWork has missing type. The separate Hero discovery-order
rebuild contains a real conditional gameplay RNG call at `269c815`; its
eligibility/profile branch remains with its dedicated owner task. GlobalAI's
postload body is a single return, established from bytes.

The [saved-guard result](../evidence/linux/yield-monitor-postload-domain.json)
from committed `a2a8d1a` contains 245 Cities with zero saved appeal-refresh
flags and 1,262 Units, 21 with empty sight vectors. Complete retained sources
and candidate `43494c1bccdb8cbcaf9d406820f5652c372d6507aff41f1af1d5cdce9d729c9c`
remain under
`target/linux/yield-monitor-domain-candidates/aabbd3e14b6f4f6cbe58323b036cc367/`.
These fields belong to **postload SavedOutputCore `ff31`**. They do not prove
the corresponding flags in original raw Read input `a667` were false: native
postload may already have cleared them. Nor do 21 empty vectors prove 21
reachable repairs; original IsDead, location and sight-range guards matter.
Original Read framing/guard inputs and actual relevant callback effects are
the next bounded history requirements. No baseline certificate is minted.

The separate retained-source domain checker uses the real 43-player,
1,262-Unit LoadedState. It tests missing-owner and corrupt-trace rejection,
one-time admission, actual warm lookups, and persistence of graph authority
across a conservative Unit-borrow revision increment. It does not extend the
historical reporting action prefix or authorize arbitrary Unit writes.

The [owned-domain result](../evidence/linux/yield-monitor-owned-domain.json)
passed from committed tree `195d3a8`, with 182 actual Route-city forward hits
and 6,193 valid path nodes. The complete source archive, extracted preimages,
generated Cargo manifest/lock, candidate and result remain under
`target/linux/yield-monitor-domain-candidates/db76ce425a9b472eb8266ff24df6ebe6/`.
Candidate SHA-256 is
`b16df326ed2fdae2154b5346943ed9c177c208d9a430b696eaa508c6a944a614`.
The earlier failed archive is retained separately with its missing embedded
manifest compile diagnostics; it produced no accepted candidate. Concurrent
`choose_tech.rs` edits were excluded from the accepted committed-tree archive.

Reproduce this static/input check without native execution:

```sh
python3 tools/linux/yield_monitor_headless_scope_audit.py \
  --output target/linux/yield-monitor-headless-scope.json
python3 tools/linux/yield_monitor_domain_check.py \
  --output target/linux/yield-monitor-owned-domain.json
```

Full affected-state/RNG, complete actions and turns remain unimplemented.
Historical report fixtures, unknown save bytes and the separate 89,198-byte
Save-to-Load difference retain their existing scope.
