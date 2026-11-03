# Independent full Manager fixture source review

Caller checkpoint `25646c0`, with contract clarification `9099b51`, passes this
bounded ABI, ownership and synthetic lifecycle review. No blocking caller defect
was found. This does not admit an original execution profile, establish live
script callbacks, compare full Manager behavior, or accept gameplay actions or
turns. Complete Rust gameplay state/actions/RNG/turns remain unimplemented.

The [independent evidence](../evidence/linux/full-manager-fixture-review.json)
records exact tested source hashes, mock executable identity, original ABI
checks and eighteen independent lifecycle/refusal cases. The review executes
only synthetic object storage and mock functions. It reads original ELF/DWARF
for source evidence without loading the original modules for execution.

## ABI and owned lifetime

The fixture calls genuine Context.GetGame (`0x2c7cae0`), rather than constructing
a lookalike facade or manipulating lock fields. Original facade construction
(`0x2c85ce0`) stores its native Game pointer, initializes refcount one and calls
the current utility's GetGameCoreLock slot `+0x40`. Original destruction
(`0x2c85d80`) refetches the utility and calls ReleaseGameCoreLock at `+0x38`.
ICvUnknown operator delete (`0x290bba0`) invokes facade virtual slot `+8`;
the verified relocation binds it to original Destroy (`0x2c86370`). Only the
facade created by this fixture is released. Context, native Game, Player,
Manager, Unit and Formation remain borrowed from independently retained owners.

The independent checks confirm these actual dispatch directions and original
Operation DWARF layout. Its constructor (`0x213d4a0`) writes runtime ID at 0,
directive -1 at 4, null shared-pointer words at 8/16 and flags zero at 24.
Complete storage is 32 bytes with eight-byte natural alignment; the fixture
uses that storage and original constructor/destructor rather than a short
synthetic argument. The Operation remains live through ENTER/RETURN observations
and is destroyed exactly once on supported normal/error cleanup paths.

The complete predicate signature has six SysV arguments: Manager, Unit,
Operation reference, Plot pointer, visible-only bool and result-map pointer.
The fixture supplies NULL Plot/results and retains the native bool return.
NULL Plot selects the original Manager's post-script fallback; it is not a
precomputed Plot or a direct SkipTurn leaf call. Both visibility values and
both predicate returns are exercised by mocks. A synthetic nonzero runtime ID
is used independently of the operation hash, preventing an accidental fixed
zero/hash argument from satisfying these tests.

The producer audit's fourteen exported name/address bindings and pinned body
gates also pass independently. Export/fingerprint checks establish source
identity; they do not establish actual loaded mappings, live receiver ownership
or genuine runtime guard behavior.

## Save effects and actor validity

Optional Save is not assumed pure. The caller resolves Manager, runtime ID,
handler and actual actor inside `invoke`, after both before/control Save calls.
Actor resolution uses the real Player/Units lookup chain and checks current
pointer, concrete vptr, owner, full ID and nonnull Formation. A native low16
lookup resolving another generation is conservatively refused as an unknown
selected actor, not converted into a Manager false result.

Independent Save stubs change actor payload before the query, remove/protect
the actor page, change its full ID, and replace the Manager. The query sees the
updated payload/replacement Manager; removed or changed-identity actors exit
240 before Operation construction or predicate execution. The first Save's
nonzero diagnostic result remains recorded and does not itself suppress a
safe query. Save outputs and known-RNG samples remain diagnostics, without
byte-equality or complete-RNG acceptance claims.

The strongest post-query challenge removes the actor and marks its entire
page PROT_NONE before the mock predicate returns. Return observation,
Operation destruction and owned-facade release still complete successfully.
No stale Unit dereference or membership re-query is performed. The corrected
header makes the boundary explicit: Unit is live at ENTER under retained-owner
proof; at RETURN its address is provenance only, and later phases clear it.
Operation and facade have their separate explicit lifetimes. Future observers
must not infer post-query actor liveness from an unchanged address.
The retained Manager address is likewise not a generic proof that every member
survived callbacks; any additional post-state graph access needs its own current
ownership/membership witness. The caller itself does not dereference that
Manager after the predicate.

## Refusals and observation failure

The eighteen independent cases extend the producer's four mocked branches and
three refusals. They cover full-run observation failure at facade acquisition,
predicate entry and predicate return; pre-acquisition/locked-begin/retained-owner
failure; changed utility/refcount refusal before release; and public TID,
profile and Save-path validation. Counters verify at most one predicate call,
matched local constructor/destructor lifetimes on safe cleanup, and release
only when ownership remains proven.

An observation failure before entry suppresses the predicate but is not a
legality result. Entry failure destroys the live local Operation before safe
facade release; return failure retains the actual predicate-return discriminants.
Optional diagnostic work remains subject to independent owner checks, rather
than treating observer failure as proof of lost ownership. Conversely, lost
ownership, changed release utility or unknown facade state exits the isolated
process without inventing cleanup success or restoring legitimate effects.
The independent lost-owner case verifies no Operation destruction or facade
release occurs after that failure.

These are C caller control-flow checks, not exception-safe native execution.
An original abort/exception/unwind may bypass ordinary C cleanup or relocking;
the runner must preserve terminal failure evidence and must not report a
completed observation or teardown from that prefix.

## Remaining admission requirements

Concrete field-only guard/observer implementations and their exact runner
profile are still separate requirements. Function-pointer contracts do not
prove supplied callbacks avoid getters, locks, VM lookup, policy or turn calls.
The retained-owner guard must preserve legitimate callback/queued effects while
excluding competing mutation; it cannot require the pre-query state, clear work
or advance native AI to manufacture a safe endpoint.

The real main-entry OS TID witness, native compact core-lock owner/depth and Lua
ASL ThreadHANDLE remain distinct authorities. Endpoint samples do not prove
uninterrupted ownership while the original Manager releases/reacquires a core
lock for scripting. Inactive-turn or other early false results can be useful
predicate evidence without proving a gameplay decision boundary or absence of
callbacks. Natural script windows still need the separate hardware observer's
runtime and lifecycle admission.

This review does not test the real C++ facade allocation/refcount operations,
real original Save/query callbacks, native lock release/reacquisition, VM/GC,
all live virtual dispatch or competing-thread behavior. Those remain original
runtime evidence requirements. All affected gameplay state, RNG, identities,
ordering and callback effects still require comparison. Unknown save bytes and
the raw 89,198-byte Save→Load difference remain unmasked; producing a Save is
not an action acceptance requirement.

Reproduce the source-only review with:

```sh
python3 tools/linux/full_manager_fixture_review.py \
  --output evidence/linux/full-manager-fixture-review.json
```
