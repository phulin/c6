# Unit/Formation archive bookkeeping and local gameplay

The headless library's authoritative local gameplay path does **not** require
native Unit/Formation archive dirty-pointer history, pointer-tree order or sync
registration acceptance. Preserve the actual field mutations, their gameplay
ordering and required reporting/subscriber effects. The native archive baseline
is a diagnostic surface for this boundary, rather than an action-acceptance gate.
This narrows the practical dependency stated in the earlier
[load lifecycle audit](linux-load-archive-postlude.md); its native observations
remain valid. Complete gameplay state, action execution and turns remain open.

This exclusion applies to the pinned Linux XP2 Unit/Formation wrapper family and
local authority. It does not assert that all network, cache, archive classes or
reporting paths are pure. Native snapshot resynchronization can mutate gameplay
and is outside this local library boundary.

Reproduce the static check, without original-code execution:

```sh
python3 tools/linux/load_archive_gameplay_scope_audit.py --output target/linux/load-archive-gameplay-scope.json
```

The [metadata](../evidence/linux/load-archive-gameplay-scope.json) gates the exact
XP2 and host SHA-256 identities, fingerprints full methods, verifies 49 typed
clear callbacks through their actual ELF vtable relocations, and scans direct
archive API consumers in both complete disassemblies. Raw source evidence stays
in `../c6-ghidra/evidence/linux-load-archive-gameplay-scope/`.

## Concrete consumers and mutation boundaries

| Surface | Source-backed behavior | Local gameplay consequence |
| --- | --- | --- |
| Unit Activity tracked assignment `0x2da4010` | Optional dirty-tree insertion precedes the actual value copy. With a null archive owner it still copies the value. No report, gameplay callback or RNG dispatch appears in this leaf. | Preserve assignment semantics; archive attachment is not the authority for a gameplay write. The higher-level setter's separate reports remain required. |
| Unit queue edit `0x2da3100` | Optional dirty-tree insertion, then return the actual queue value. | Preserve queue operations and order; native pointer-tree insertion is optional diagnostics. |
| `getDeltaIndexes` `0x3c45f34` / `hasDeltas` `0x3c46fca` | Read dirty pointers/size; return metadata without clearing. | Diagnostics, with no found local gameplay consumer. |
| `SentGameSyncCheck` `0x39d54c2` | Computes full archive data hash at `0x39d553c` and copies dirty indices into SyncCheckData at `0x39d5559`. | Native synchronization packet/state bookkeeping. Full data hashing visits all registered values, independently of dirty membership. |
| `SetSnapshotDeltaArchive` `0x39dadbc` | `saveDeltaUnion` at `0x39dae1d` serializes current dirty fields union supplied indices into a snapshot stream. | Native resynchronization output; its exact wire order is outside local gameplay authority. |
| `TriggerGameSyncComplete` `0x39d96de` | With a real session, clears actual registered archives; later invokes session callbacks. | Dirty clearing itself has no Unit/Formation value effect, as verified below. Session callbacks are not blanket-excluded or emulated by clearing. |
| `HandleNetGameSyncSnapshot` `0x39d5aa2` | Calls `loadDelta` at `0x39d82f7`, then gets selected registered desync callbacks at `0x39d836d` and invokes them at `0x39d83be`. | **Gameplay-mutating transport input.** Unsupported for this local boundary; must not be treated as a pure cache refresh. |

`FAutoArchive::loadDelta` (`0x3c46036`) clears the prior dirty tree, reads field
indices, resolves actual wrappers and invokes their live delta readers. It can
overwrite Core values and then invoke registered callbacks through the enclosing
snapshot handler. Supporting native multiplayer resynchronization would require
that state application and its callbacks; this exclusion establishes no such
support. Local actions derive their result directly from owned authoritative
state and action inputs.

The direct-call scan finds ten call sites to the inspected archive query/hash/
delta/clear APIs: all are in FGameSyncManager or FAutoArchive itself, with zero
in XP2. It also records full-data-hash consumers in ProcessNetGameSyncCheck.
No direct callers of hasDeltas or calculateDeltaHash appear. Exported APIs remain
callable by diagnostics. This scan is not an exhaustive proof about arbitrary
external callers, custom controllers, inlining or indirect calls. The positive
setter and typed-clear body checks provide the bounded local inclusion/exclusion;
the absence of direct XP2 calls alone would not suffice.

## Clear callbacks close the earlier uncertainty

Every one of the **49 exported typed Unit/Formation FAutoVariable vtables** has
its virtual `clearDelta` slot `+0x38` bound to the matching method whose complete
body is exactly one-byte `RET`. This includes the registered field type family,
the nested Unit Religion bool specialization and unregistered Hero wrapper.
The audit verifies relocations at vtable symbol+`0x48` (16-byte ABI prefix plus
56-byte slot), matching method names, extent and exact bytes.

Host `clearDelta` does visit dirty wrappers in pointer-tree order before emptying
the tree. For this verified family those visits cannot update a gameplay field,
consume RNG, emit notifications or change legality: every target is empty.
Consequently neither preexisting tree membership nor its iteration order is
needed to reproduce this clear's gameplay effects. This does not generalize to
uninspected archive classes, replacement vtables or network snapshot readers.
Native allocator/capacity failure behavior is outside the headless state model;
this is not a claim about arbitrary allocator overrides.

## Gameplay reports and projections remain required

Core FAutoArchive dirty pointers are distinct from the host
[DirtyComponents ID vectors](linux-skip-turn-reporting-closure.md), Event::Send
subscribers, Unit snapshots and YieldMonitor continuations. The inspected
tracked-assignment/edit leaves perform no cache/report delivery. Those effects
are called separately by gameplay setters/operation managers and must still run
at their source-required boundaries. Eliminating a dirty-pointer insert is not
permission to eliminate the actual setter, suppress an equal-value report where
native code sends one, or defer a synchronous subscriber until after legality
has been evaluated.

No inspected consumer requires reconstructing the native archive tree to update
these gameplay projections. Preserve their actual report inputs, registration
and enabled-state semantics, mutation order, and gameplay-visible projected
values. A direct typed projection update is suitable only after that consumer's
behavior is established; preserving reports alone does not close unknown
subscribers. Any newly identified gameplay consumer of archive dirtiness must be
included before admitting that wider profile.

## Application to the existing import and action work

Keep decoded/currently derived Unit/Formation values and genuine logical
identity. Implement definition/domain resolution, nested reset/remapping,
Formation evaluating-operation initialization, sight/post-load gameplay and
subsequent setter/report effects according to their source contracts. These
remain gameplay dependencies regardless of archive bookkeeping.

The existing [31 explicit Unit28 / 32 observed idle-index evidence](linux-load-archive-postlude.md)
remains a native diagnostic. Its extra index44 and missing Formation idle witness
are not local action blockers. LogicalArchive may retain optional diagnostic
traces or an unavailable diagnostic baseline. Local gameplay readiness must not
require the native dirty set, archive controller acceptance or fabricated
`*_archive_bound` flags. No further archive-only native capture is required to
advance the local action path.

Native snapshot/delta transport remains explicitly unsupported. Real gameplay
mutations, reports, notifications, callbacks, legality, gameplay RNG and every
external player decision remain exact requirements. The raw 89,198-byte
Save→Load difference remains retained and unmasked; this scope decision neither
classifies it nor makes binary-identical save rewriting an acceptance gate.
