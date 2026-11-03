# Linux Hero discovery order

The bounded source implementation updates owned typed Player.Heroes discovery
order and the caller's actual Game RNG after fallible inputs have all succeeded.
It supports at most 28 undiscovered records. It has no admitted production
configuration/registry/eligibility provider or LoadedState bridge, and no new
original-code rebuild oracle. Complete gameplay state, actions and turns remain
unimplemented and unaccepted.

Reproduce the offline source, synthetic-kernel and saved-fixture checks from a
commit containing the driver and checker:

```sh
python3 tools/linux/hero_discovery_order_audit.py --source-commit COMMIT --output target/linux/hero-discovery-order-fresh.json
```

The driver gates the pinned Linux XP2 and host hashes, compiles the checker from
a complete extracted committed source archive, and retains archive, binaries,
compiler and standard-library identities in immutable content-named directories.
It refuses to overwrite reports. Unrelated working changes are excluded. The
standalone checker has a documented two-field Limits transport shim and copies
the exact committed RNG serializer preimage to satisfy Rust standalone module
lookup; this is recorded explicitly. Proprietary game bytes are not archived
with candidate source or committed publicly.

`HeroesManager.InitializePostLoad` at `0x2655ed0` rebuilds valid classes and then
calls `ValidatePlayerHeroDiscoveryOrders(false)` at `0x2655fc0`. Validation walks
the manager's alive-major IDs in their existing order, resolves each player and
rebuilds only empty orders. Saved player IDs or booleans alone do not establish
that native manager membership or successful resolution.

The rebuild at `0x269c6f0` follows this sequence:

1. Edit and clear the discovery-order wrapper, even when it is already empty.
2. Require a major player, Heroes allowance and a resolved current-era definition.
3. Iterate the HeroClass definition collection in native order. Skip discovered
   classes; each remaining class is appended, including ineligible zero weights.
4. A valid unclaimed class requires its creation project and unit reference.
   District and unit-domain support can be witnessed by different cities.
   Domain 2 starts with domain support true; no prerequisite district starts
   with district support true. Empty cities can therefore still satisfy both
   when both initial conditions hold. Missing references fail eligibility.
5. An eligible class whose optional minimum-era chronology is no greater than
   the current chronology gets `100 + Game.GetRandNum(20) - 10`. Other appended
   classes have weight zero. Draw order is definition order, before sorting.
6. Sort descending by signed weight, then emit the rebuild log event.

`IsAllowedToUse` at `0x26560b0` first requires capability `0x82ef2304`.
Multiplayer nonhuman players are allowed; human multiplayer allowance values
1 or 2 pass. In single player a local player of -1 passes; otherwise only
allowance 1 passes. Unknown capability/configuration inputs are fallible query
errors, never guessed allowances. Synthetic query providers test these source
branches and confer no captured eligibility authority.

The target uses insertion sort for up to 28 native eight-byte records; its
strict signed-greater comparator at `0x269e450` preserves ties in collection
order in that bounded branch. Larger lists enter native introsort, whose tie
permutations are not ported. The implementation rejects those lists before
committing state. Definition indices and hashes remain separate identities;
duplicate indices/hashes are rejected.

`rebuild` prepares effects and a cloned owner, then commits both Heroes order
and actual borrowed owner atomically. The RandomState adapter reuses the
independently [Linux-validated primitive](linux-random.md); it preserves resets,
locks and synchronous storage while updating seed and calls. Callers must prove
that the borrowed value is their actual Game stream. A saved Random record is
not a complete live owner and has no adapter here. Missing queries, unsupported
sort size and invalid identities leave both borrowed values unchanged.

Effects retain discovery archive edit/clear, Game draw, append, sort and logging
order for diagnostics. This does not assert complete native logging, archive
attachment, allocation or multiplayer resynchronization behavior. The existing
[local archive exclusion](linux-load-archive-gameplay-scope.md) proves a bounded
Unit/Formation family; it must not be silently generalized to Heroes or Game.
Native pointer-tree baseline is not a prerequisite for local owned-field
mutation, while source-scope/native observer gaps remain explicit.

The native HeroClass builder has count/row SQL at `0x21febd0/0x21febe0`, with
rows ordered by rowid. `BuildCollection` at `0x2254960` appends only successful
row iterations and assigns the previous collection size as the native index.
The fresh collection constructor at `0x2246d30` zeros its full 0x30-byte pointer
storage. Both retained activated databases hash to
`c880adaedcfd6da7808302b3b76bb9cad7d3188b35ae78e780cff7b56e04477e` and the
actual pinned builder query returns zero rows. Thus a fresh collection populated
solely from either exact database has zero classes, and every discovery rebuild
on that collection consumes zero draws regardless of earlier gates. This is
a conditional zero-RNG proof, not evidence that native InitializePostLoad used
that exact collection/database instance or that archive-clear effects vanish.

The raw incoming Read payload `a66736…c5aa` is 76,556,938 bytes. The retained
`ff31f4…d884` payload is post-load SavedOutput with writer-only owner cuts. The
audit walks its source-backed Quests/WMD/Improvements/Governors/CulturalIdentity
grammar into all 43 Heroes records and compares each typed re-encoding. Raw
Read has no retained Heroes owner span in this audit, so neither those offsets
nor post-load flags are borrowed to claim which original rebuild gates ran.
The next fixture contract is exact raw-Read player/Heroes framing, actual native
manager roster/order and configuration/capability/definition collection identity,
plus matching initialized native before/after observations of order and Game
RNG when exercising a nonempty eligible class branch.

Native Save bytes remain decoding diagnostics. Unknown bytes stay unclassified
and unmasked, including the recorded 89,198-byte Save→Load failure. This work
does not implement a Save writer or make save roundtrip equality an action gate.
