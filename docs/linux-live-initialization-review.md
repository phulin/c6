# Independent Unit and Formation initialization review

Formation source `5809ff8` and Unit source `10d404c`, corrected by `3760ee7`, provide bounded,
source-backed sidecars beside owned decoded records. The reviewed initialized
values and local Formation edits agree with the inspected Linux target bodies.
This accepts these scoped contracts, not complete native Read, an initialized
World, a complete action, or a turn. The final outer-load dirty baseline remains
outside this acceptance; the separate [archive lifecycle audit](linux-load-archive-postlude.md)
preserves its unresolved loaded-idle boundary.

The [review evidence](../evidence/linux/live-initialization-review.json) records
source hashes, both hash-gated static audits, and independent public-API checks.
The GameCore target remains
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
No original game code executes in this review. Raw target disassembly remains
private under `../c6-ghidra/evidence/linux-unit-live-initialization/`.

## Reviewed contracts

Unit definition initialization at `0x2c1c6e0` resolves the actual definition and
writes its collection index; post-load `0x2c1c850` invokes it, and concrete
post-load `0x2c12d00` dispatches domain initialization. The domain body at
`0x2c12bc0` confirms all five hash branches and unknown-domain -1, including
the two signed-high hashes. Rust retains the wire Unit type hash and stores
the runtime index/domain separately. Unknown Unit definitions reject rather
than manufacturing a definition. Complete input identity equality is checked;
it does not authenticate caller-created tables or establish a live database.

The actual Unit Reset body at `0x2c23dc0` clears and sizes the six admitted
top-level arrays with zero/false defaults. Inspected overlay bodies at
`0x2c2a7a0` and `0x2b13a80` consume unknown keys without writes and write known
keys in stream order, with no destination-wide clear. The current reader's
versioned call sites use these admitted current schemas. The sidecar separately
restores operation-disabled storage through its false-reset/ordered-overlay
contract. Other nested arrays and enum remaps remain outside this module.

The independent harness moves one Unit and one Formation into an owned Player,
then calls `UnitRuntime::from_read` through public APIs. A stale requested
Formation generation resolves the actual low16 slot and retains both requested
and resolved IDs. Unknown collection errors remain distinct from known-empty,
sentinel and missing-slot null results. Owner/profile mismatches and bad
definition/index inputs reject without altering the owned Unit. All six arrays
and disabled operations are checked for defaults, ignored unknown keys and
ordered duplicate overwrites. These are deliberately modified source fixtures,
not initialized native observations.

Formation Read `0x293b560` calls domain edit before decoding it, unconditionally
assigns false to evaluating-operation, skips Globals player lookup only for
owner -1, and then installs its ComponentID. The constructor-derived schema
contains 17 Formation and 73 Unit registrations, preserving inline-address
order and the actual field identities. Formation path/cache edits mark indices
14 then 15 even for empty/invalid values. Independent checks preserve member
order and refuse a different resolved identity before changing state or trace.
This identity check follows World resolution; it must not reject a valid native
low16 Find merely because the original request used an older generation.

The Formation dirty set `[4, 13]` is specifically the fresh local Read boundary.
Unit dirty membership remains unavailable. Neither sidecar supplies a later
global baseline, sync-manager acceptance, subscribers or gameplay callbacks.
Logical inline attachment is evidenced; the older SkipTurn kernel's attachment
booleans are not set by these constructors.

Runtime construction borrows the actual records and creates only its own
derived arrays/archive metadata. It does not clone the World or substitute
precomputed legality answers. Public sidecars still need a consuming adapter
with genuine registries, Manager inputs and all omitted live controls. At this
checkpoint no gameplay consumer uses them, so omitted Experience/RockBand and
other remaps do not silently enter an accepted action path. Future adapters
must retain that limit rather than interpret successful sidecar construction
as complete Unit Read or loaded-action acceptance.

## Public API defect corrected

Review identified that `overlay` resolved every key during preflight and again
while writing. An `&self` provider can change through interior mutability, so
the second answer could bypass the first bounds check and panic after earlier
writes. An immutable native registry does not justify that behavior in this
public helper's error-before-mutation contract.

Producer correction `3760ee7` fallibly reserves index storage, resolves each pair
once, validates every result, and writes only the cached indices. The separate
public-API harness supplies a changing provider that returns valid indices for
the first two calls and out-of-range indices thereafter. It now verifies exact
output and exactly two calls. The producer also added its own regression.

## Checks and limits

`cargo test -p c6-core initialization --lib` passes 20 targeted tests. The
separate integration harness adds actual `UnitRuntime::from_read` coverage;
the producer's four Unit tests exercised its helper contracts only. Reproduce
the independent checks using an existing private Unit record solely as a
decode seed:

```sh
python3 tools/linux/live_initialization_review.py \
  --unit-seed ../c6-ghidra/evidence/linux-placement-handler-metadata/native-1/differential-1/units/07-000076.bin \
  --output evidence/linux/live-initialization-review.json
```

The runner builds a separate Cargo harness offline and reruns both static
target audits. It does not create Saves, perform roundtrip acceptance, capture
new native state, or execute the original engine. Native initialized-object
comparison, complete RNG and affected-state parity, archive post-load cleanup,
full Manager/SkipTurn execution and external-decision turns remain open.
Raw Save→Load differences remain unmasked evidence; binary-identical save
production is not an action gate.
