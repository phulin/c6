# Linux real-owner movement-cost probe contract

This is a concrete **unexecuted** native fixture for the
[reviewed scalar evaluator](linux-movement-cost.md), not a movement action or
gameplay parity result. The probe uses original loaded Unit, Formation, Plot,
CityAttributes, District and rules owners, and original constructors/getters.
No game object, flag, vtable, location, remaining movement or query return is
patched to force a branch. World-backed Rust resolution and full state/action/turn
equivalence remain unfinished.

Owned implementation files are [the native probe](../native/linux/movement_cost.c),
[its caller contract](../native/linux/movement_cost.h),
[the offline candidate](../crates/c6-differential/src/bin/c6-movement-cost-candidate.rs)
and [the static audit](../tools/linux/movement_probe_audit.py). The
[frozen build report](../evidence/linux/movement-cost-probe-contract.json) records
125 original call bindings and three caller/initialization witnesses plus source, helper and candidate hashes. Building and
static auditing these artifacts does not execute original game code. Discovery
reuses the pinned cru121 reference and actual-target evidence described in
[the scalar contract](linux-movement-cost.md); attribution remains cru121,
CC BY 4.0, revision `5af389445118bed2bdebf7a8a42b219c8fba5175`.

## Required lifecycle and integration review

The sole runner must integrate this helper at its established loaded-Core boundary
after proving original worker quiescence, before controller/cache/module teardown.
The caller must retain the existing native dispatch/policy guards. The probe does
not queue or execute messages, wake workers, call MoveTo, process a player policy
or advance turns. It does not itself synchronize an active worker. Calling it
while native mutation is in progress violates its contract.

The runner must export `int c6_linux_movement_cost_quiescent(void *module,
void *borrowed_context)`. Return zero only after exact original Controller
IsExecuting=false, Context HasPendingMessages=false and the reviewed fixture
idle predicates are established. The probe calls it before original world
queries, before and after each internal Save, after fact batches and after each
cost/phase call. Active or unknown worker state must log and immediately `_exit`
the isolated process, with no post-state/RNG observation, Save, cleanup or
Uninitialize. The probe also `_exit`s if the hook returns nonzero. No waiting or
state repair occurs. The runner must pin the hook's owning preload/module, source,
compiled hash and dladdr identity; a presence check or printed pointer alone does
not prove quiescence.

The caller must first enforce the exact XP2 and host SHA-256 identities from
[targets](targets.md), dependencies, actual rules database, enabled DLC/mods,
saved/runtime configuration and loaded fixture. `C6_MOVEMENT_COST_PROFILE_SHA256`
must identify the caller's complete checked configuration manifest; the helper
records that digest and does not pretend to infer configuration identity from a
DLL hash. Runtime transitive symbol/provider bindings still require the runner's
observations and independent review; the source's direct RVA table alone does not
prove the live PLT/GOT/provider context.

The discovery scan is explicitly scoped to players 0–63 through original
Globals::GetPlayer. Manager::GetPlayer indexes a pointer table without a bounds
check. The original Manager::Initialize (`0x2b51770`) allocates 65 null pointer
slots; matching DWARF places its ID-to-player vector at +0x20. Before the scan,
the helper requires the genuine Game::GetPlayerManager result, original
IsInitialized (`0x2b51830`) true and an actual aligned begin/end extent of at
least 64 slots. The observed extent is emitted. This is a bounded scan, not a
claim to enumerate every possible player slot or configuration.
Game::GetInstance, Globals::GetGame, Globals::GetMap, Map::GetInstance and
Game::GetMap must agree on genuine receivers. The real Map vptr/GetPlot virtual
target and every selected Plot/CityAttributes vptr plus its called attribute
virtual targets are checked. Receiver identities, relevant targets, hook identity
and each Formation archive subobject vptr are recorded for the bounded runtime
binding review. The helper checks non-null owners and owner/Formation registry identity on every
selected formation. It borrows those owners without reference release/destruction.

Required environment variables are:

- `C6_MOVEMENT_COST_OUTPUT`: exclusive-create private JSONL output.
- `C6_MOVEMENT_COST_PROFILE_SHA256`: checked manifest digest, 64 lowercase hex digits.
- `C6_MOVEMENT_COST_SNAPSHOT_BEFORE`, `..._CONTROL`, `..._AFTER`, `..._REPEAT`:
  distinct exclusive-create private original Core save artifacts.
- `C6_MOVEMENT_COST_EXECUTE`: `0` or absent for discovery/fact-only control;
  `1` for original cost calls after review. No original execution has occurred.
- Optional `C6_MOVEMENT_COST_MAX_FORMATIONS`: 1–4096, default 256. The summary
  reports truncation; a truncated scan cannot claim complete fixture availability.

Compile/link the native source with the existing `context_stream` save helper;
it is not a new replacement serializer. The initially frozen standalone `.so`
intentionally leaves `c6_linux_context_save` unresolved for the established helper
to provide. Any integrated `.so` rebuild needs a fresh hash in the audit report.
Shared runtime/build/dispatch files remain owned by the sole runner.

## Genuine owner and query provenance

For each original Player's Units container, the original default vector constructor
creates a 24-byte ID-vector output, Units::GetUnitIDs fills it, and original
size/index getters enumerate it. The original destructor releases it. Unit::Find
resolves each ID. Unit::GetFormation returns its actual borrowed formation;
Formation::GetOwner/GetID and that Player's Formations::Find must agree exactly.
The probe deduplicates `(owner, native formation ID)` and rejects conflicting pointers.

Formation::GetLocation returns the genuine current coordinate reference.
Map::Instance::GetPlot resolves the current source and original GetAdjacentPlot
enumerates directions 0–5. No formation is relocated to a convenient test plot.
Map::IsAdjacentPlotCoord with actual dimensions/wrapping independently provides
the edge fact; a nonadjacent result triggers the original portal/exit query rather
than an assumed false portal result.

The JSONL stream starts with identity/environment metadata and a bounded
receiver/vptr attestation record, then five complete
live registry records (`terrain`, `feature`, `improvement`, `route`, `district`).
Each registry entry records original query/index/hash and the original cost,
float bits or FreeEmbark value. Live native definition indices are not SQL rowids.
Each case includes:

- Actual player/unit/formation identity, formation domain/layer, direction and
  source/destination coordinates.
- Native base/max/remaining budgets. The destination IsWater result supplies
  domain argument 0 or -1 to GetBaseMoves, following the inspected MoveTo caller;
  original GetMaxMoves returns an integer converted by the actual Q8 constructor.
  Original GetMovesRemaining returns Q8 through hidden output storage. Exit moves
  are constructed as zero, as in the original full-cost wrapper.
- Four original global parameters and formation terrain/feature/river/shore ignore
  getter results plus CanEmbark. Type-specific ignore queries record the actual
  resolved definition collection index, or -1 when lookup is absent. Such absent
  definitions do not contribute terrain/feature cost in the native phases.
- Every genuine member ID and lookup presence, original Unit ignore getter results,
  escort/river/shore modifier queries and all native invalid-terrain/ignore-terrain/
  ignore-feature bits. Actual Unit vector subobjects at +0x378/+0x3a0/+0x3c8 are
  read through original vector size/index getters; no vector internals are constructed.
- Actual Plot IsCity/IsWater/IsHills, route presence/pillage/bridge state and original
  definition query/index/hash/cost facts. Improvement movement cost is observed even
  when a separate pillage concept would seem intuitive; no new gameplay rule is added.
- HasCityAttributes separately from the actual IsDomainValid result and omitted live
  signed 16-bit validDomains mask. The shared original empty CityAttributes result is
  not mislabeled as an allocated plot attribute record.
- The original CityAttributes::GetDistrict output ID (owner/type/raw instance ID),
  actual borrowed CManager lookup and actual resolved District::GetID, original
  GetDistrictType and IsDistrictComplete results. Requested generation bits must not
  be assumed equal to the resolved object's full ID. District definition index/hash
  and FreeEmbark are separately observed.
- Original river/bridge query results and original ordered rule-phase observations.

The native getters and field facts are backed by supplied Linux symbols/DWARF and
instructions; no upstream Windows layout is used. The separate City/District and
typed resolver owners review the corresponding registry/mask/vector contracts.

## Exact original ABI and invocation

`Formation::Movement::MovementCost` at `0x29484e0` is a static method returning a
nontrivial four-byte FixedPoint through hidden output storage. Its C-callable
register/stack projection is:

```
void *full_cost(i32 *output, Formation const *formation,
                Plot const *source, Plot const *destination,
                Plot const *exception_plot, i32 integer_base,
                i32 const *max_q8, i32 const *remaining_q8, u32 options);
```

The first six arguments occupy RDI, RSI, RDX, RCX, R8 and R9 respectively;
max/remaining pointers and options follow on the stack. The probe passes the
genuine requested destination as exception plot, matching the inspected MoveTo caller
at `0x2947a4a`. Options `0` is an explicitly selected query argument, not a write
to any private unit/game flags. Actual MoveTo caller-derived options are currently
unavailable; no claim is made that the tested query is a command that a player
issued or a legal movement action.

The Values constructor (`0x2945d40`) takes:

```
void construct(Values *out, Formation const *, Plot const *, Plot const *,
               i32 integer_base, i32 const *max_q8,
               i32 const *remaining_q8, i32 const *exit_q8, i32 zoc_selector);
```

The original constructor supplies all fields; the C layout has static assertions
for size 64 and hard-minimum/exit-add offsets 44/56. The nontrivial eight-byte
ComponentID return storage is explicitly aligned to eight bytes. GetFinalCost (`0x2945de0`)
takes output storage RDI and Values RSI. Individual rule phases take Rules receiver
RDI and Values RSI. The Values overload (`0x2949170`) takes hidden result RDI,
Values RSI, exception Plot RDX and options ECX. No C++ scalar is incorrectly treated
as a plain integer return. FixedPoint and Values have no explicit destructor here;
native constructors/copy behavior supplies their storage values.

For each real edge in execute mode, the probe observes original IsSlowedByZOC,
calls original full and no-ZOC wrappers, constructs a no-ZOC Values and records
the four accumulator words initially and after each of the nine original phases,
then invokes GetFinalCost. A separately native-constructed Values runs the actual
ZOC-aware overload. The output retains both agreement and disagreement; repeated
original queries may reveal cache/provider effects and are not assumed pure.

The offline candidate consumes actual exported facts and compares no-ZOC final
cost, final four accumulator words, ZOC-aware final cost/accumulator, agreement of
native wrapper paths and the four formation ignore aggregates. Intermediate native
phase snapshots are retained for diagnosis; this candidate currently compares the
last phase accumulator, not every intermediate prefix. It neither loads game state
nor substitutes a simplified movement action.

The numerical precondition is x86_64 Linux binary32, FE_TONEAREST, masked floating
exceptions, DAZ/FTZ off. The helper reads FE rounding and MXCSR, rejects unsupported
settings and never changes them. MXCSR status bits are preserved and permitted;
the scalar evaluator's result contract excludes floating-status side effects.

## Controls, affected state and representative cases

Use distinct fresh, identically gated sessions for fact-only control and cost-call
mode. Each session writes two original Core saves before discovery and two after
the batch. Preserve all four byte streams and every raw difference; no unknown
range is masked or assigned a planning-only role. The recorded 89,198-byte
Save→Load failure remains a separate unresolved failure, not a permitted tolerance.
Before discovery and after the batch, retain the complete 32-byte original Game
sync and Map generator Random objects. This covers two known streams and does not
establish complete live RNG, PCG, archive/provider state or full affected state.
The whole Core serialization/control differences and runtime/provider observations
must accompany scalar comparison results.

The deterministic batch uses actual native ID enumeration and real neighbors. Its
report must count available/selected plain/hill/feature/city destinations, route
pair/pillaged/different-route/bridge/river cases, type-ignore/escort combinations,
shore transitions, district combinations, remaining-vs-base and slowed-ZOC cases.
Increase the explicit formation limit or use another genuine loaded fixture when
required classes are unavailable. Do not manufacture units, plots, flags, domain
masks or cost values to obtain missing gameplay branches. Scalar synthetic overflow/
float-boundary fixtures remain separately labeled and cannot replace real-owner
mechanics acceptance.

Current checks: strict GCC compile/link, Rust release candidate build, exact-module
static fingerprints and frozen build hashes. Native profile integration, runtime
bindings, reviewed runner-owned quiescence-hook integration, live execution,
branch availability and
original/Rust/state/RNG comparison remain outstanding. No native run is authorized
by this source checkpoint itself.
