# Independent diplomacy pending-state review

Review of `ddd342d` against source contracts `4641dac`/`662efe5` found two
admission/lifecycle defects. No original code was executed and no current
native diplomacy snapshot was admitted. Historical execute-3 empty predicates
remain confined to that runtime; equal saved Core bytes do not transfer them.

1. `diplomacy_manager_mut` and the drain used wrapping revision increments.
   At `u64::MAX`, a mutation changed revision to zero, making advanced state
   appear pristine. The drain also removed/processed a statement at exhaustion.
   Revision refusal must occur before mutation, while an empty drain at MAX
   remains a read-only success. The author corrected these paths in `431eead`.
2. The original inventory check confused owner IDs with allocation slots.
   It iterated owners0..63 and restricted handles to those IDs. Native
   InitializeLists scans allocation slots0..63; the separate ID table has65
   entries. Conditional on an actual owner64 object in a scanned allocation
   slot, the drain reads its ID and uses the configuration fallback. In the
   source-only review carrier, a complete [64,1] list was rejected, while an
   incomplete [1] list was silently admitted despite supplied pointer presence
   and WasEverAlive for64. More generally an ever-alive owner in allocation
   slot64 must be excluded, even when its ID is below64. Correct admission needs
   the actual allocation inventory/order, not a sorted or complete set of IDs.
   No actual native owner64 initialization/capture is claimed by these cases.

The [initial reproduction](../evidence/linux/diplomacy-manager-independent-review-ddd342d.json)
retains six passing cases and one failing revision case. The
[expanded reproduction](../evidence/linux/diplomacy-manager-independent-review-ddd342d-expanded.json)
retains seven passing cases and four failing assertions across the two defects.
Both use an archived, unchanged `ddd342d` production tree plus a retained,
explicitly injected private test fixture. All source preimages, exact patched
files, test executable and logs remain in the private evidence directories
named in those reports. The first rejected review-carrier setup was retained
privately as debugging evidence and is excluded from these meaningful cases.

The independent cases confirm removal before synchronous processing, retry
from the live head, copied handle iteration across callbacks, source allocation
order rather than numeric ID order, and iteration of ever-alive AI players
independently of current alive/turn-active flags. A private fixture models
configuration changes: the current player's IsAI is read once, while the next
player's current configuration is read after earlier callback effects. This
checks timing only. Production exposes no configuration mutation setter;
Statement subtypes requiring such effects remain unsupported, as the author
confirmed. Complete subtype rules, routing/history, RNG/effects and external
player decisions remain required providers.

Exact runtime/facts/configuration/role witness changes reject even with equal
save hashes. Invalid membership rejects atomically and repeat snapshot
installation rejects. Pending actions retain their typed values and ordering;
this drain does not consume that separate vector. Empty processing is read-only.
External-decision and diagnostic-budget errors preserve the already-removed
statement and preceding effects, matching the existing diagnostic kernel.
That aftermath is not a successful action, atomic unsupported operation or
resumable external-policy continuation. It cannot count as whole-action parity.

`tools/linux/diplomacy_manager_independent_review.py` takes an explicit source
commit and a new private output directory. It archives the complete tree,
checks all32 target methods and10 selected instructions against the pinned ELF,
injects only review tests into the private source copy, builds/runs only the
review cases, verifies source contents after the build and retains all inputs.
For the corrected allocation API, it adds two cases for malformed/reordered
inventories and exclusion of an actual allocation-slot64 occupant. It never
patches the shared production files or launches the original program.

## Corrective committed review

The author corrected revision exhaustion in `431eead` and allocation admission
in `13a7a4780aac981d15f5e997f1e9547c3538cde7`. The latter requires the complete
actual65-slot allocation inventory, validates unique membership against the
separate ID table, and checks the exact ordered ever-alive result from only
slots0..63. Owner64 can appear in that scanned prefix. Any owner at allocation
slot64 is retained in the inventory and excluded from this derived list.

The [independent corrective report](../evidence/linux/diplomacy-manager-independent-review-13a7a47.json)
records **13 passing targeted cases**, zero failures. All32 source-contract
method bodies and10 selected instructions match the pinned original ELF.
The private `candidate-13a7a47` directory retains the complete committed source
archive (SHA `a781529f055ddf26a4691e55b326bbd4416fb91f0cd1050ecf8e7a6a91daf63b`),
all1,656 checked source preimages, exact review-only patch/driver and executable
(SHA `aa5044715785c7b85027a0011f0a32f65073c7b30b2289438b35f6281b9729e4`).
No shared production source was patched by this review, and the initial failing
reports remain unchanged. No remaining defect was found within this bounded
source/admission/drain review.

These checks do not supply a current original pending-state capture. A capture
adapter still must attribute all current facts to the same dynamic context and
boundary as its controls, rather than transplanting execute-3 observations or
accepting save-hash equality. Full Statement behavior and gameplay effects,
configuration mutation support and a genuine external-decision continuation
remain unavailable. Neither these source-only tests nor the successful empty
kernel path establish native full action, turn or RNG equivalence.
