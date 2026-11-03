# Separate fresh ff31 Military initialization capture

The closed `native-3` capture supplies fresh original Read/Initialize witnesses for
the `ff31f4c5…` Core input. Its two actual Save outputs are `85d858ef…`; each differs
from that input by exactly **89,198 unmasked bytes**. No `85d` Read initialization
was observed. Complete gameplay state/RNG, actions and turns remain unsupported.

This packet is separate from the [earlier native-2 classification](linux-military-initialization-capture.md)
in `ecbef25`. Its initialization context comes from its own actual incoming Read,
not a prior writer snapshot. The earlier audit, report and documentation remain
unchanged.

The [new offline driver](../tools/linux/military_initialization_ff31_capture_audit.py)
reuses the frozen structural/artifact validator and its 13 corruption refusal
checks. It verifies the captured-input gate and historical origin reference,
derives the correct fresh-input role limits, and compares the complete actual
input/output bytes without masking unknown bytes. Its
[separate report](../evidence/linux/military-initialization-ff31-capture.json)
retains the actual per-owner ranges and context hashes. It performs no original
execution, candidate comparison or typed Rust snapshot admission.

## Closed artifacts and scope

The private closed directory is
`../c6-ghidra/evidence/linux-military-initialization/native-3`.

| Artifact | SHA-256 |
| --- | --- |
| Actual runtime report | `01479e18272065e11ab0a50dc2284b6fdf4fce77a166b07ae98a19bba5dfa8b3` |
| Raw 3,057-row JSONL | `f9035540d3862cb6fb16a08db47cd3895cfec398561b818a59bf9449f7ef0603` |
| Frozen ordinary Load/Save profile | `a65921cab58cc10e8943fd4655d9de9eacef7150341f2e718c53d0eb40e2f414` |
| Archive of 35 captured source preimages | `099518aef61bf3e53674909473594b699417feebf542199102abd8a886c32ea2` |
| Historical captured-input origin reference | `8c03bcdad2e44e9e0f9c7d5b08d011d0265b42a93582b5eaff2c3759d1380bd5` |

The original Linux XP2 module and host identities, frozen observer/runner/preload
source, and hardware-only game breakpoint protocol match the reviewed boundary.
The additional captured source preimage is `tools/linux/game_load_captured_gate.json`.
Its metadata and origin reference identify the accepted captured input; they do
not supply native initialization or prove a complete saved-game fixture.

The classifier rehashes the actual input, both complete output files, module/host,
closed artifacts and archived/unpacked source preimages. It verifies the prepared
configuration against the actual report and raw plan. Original process/debugger
exit statuses are zero; the terminal supervisor records no remaining session
processes. Startup, Shutdown, framework cleanup and client cleanup returned.
Controller Initialize/Uninitialize were not invoked. All 917 paused stops retain
the null-worker/controller-not-initialized guard. The final source-guard witness
checks 34 original method fingerprints.

## Fresh stream contexts

| Stream occurrence | Actual role | Core SHA-256 prefix | Bytes | Military records |
| --- | --- | --- | ---: | ---: |
| 1 | Fresh incoming original Read/Initialize | `ff31f4c5…` | 80,657,462 | 43 |
| 2 | First own original Save output | `85d858ef…` | 80,657,462 | 43 |
| 3 | Second own original Save output | `85d858ef…` | 80,657,462 | 43 |

The full incoming Core SHA-256 is
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`.
Each own output SHA-256 is
`85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8`.
The whole Save supplied to the saved-configuration loader remains separately
identified as `a328d643…`, 7,348,717 bytes. Its extraction relationship to the actual
MemoryIStream payload is explicitly unproven.

All 43 fresh Reads have matched prior vectors, installed owner, nested initializer,
actual personality/leader/query/count observations, initialized vectors and normal
Read return. Actual personality is true, the allowed-operation query is nonnull,
its observed handler default is zero, and the active definition count and
initialized allowed-vector size are seven in every observed case. These are fresh
call and vector observations; serialized counts do not supply their context.

The actual complete three-vector snapshots remain distinct across stages. Twenty
current vectors change during original Read; observed allowed and per-war values
remain unchanged between initialization and Read return for this fixture. The
separate per-war query/preparation/rows remain unobserved; its complete normal
initializer-return vector is preserved without claiming SQL replay.

All 86 own Writes have entry/guard/return witnesses, 258 ordered helper calls and
86 file-closed output bindings. All 129 Military ranges are checked against the
actual stream offsets, and all three physical stream lifetimes are destroyed.
Observed owner IDs are `0..40, 62, 63`; no outer Manager ordinal/requested-ID mapping
is inferred. Selected-frame owner lifetimes do not establish continuity across
streams. The ordered active definition identity is
`e895435ea6ff98783727bd6334736a39d94ca0413ac00e68f9675fe90c0f49c2`.

The raw `capture_complete=false` footer remains unchanged pending independent
attribution/admission. Rules/content/config/DLC/mod input gates remain separate
from the unavailable in-memory gameplay database byte snapshot. There are no
runtime policy-call counters. One post-Load raw64 known-RNG observation is retained;
Save endpoint equality is a frozen source-guard witness, with no independently
emitted endpoint dumps. Neither scope establishes complete gameplay equivalence.

Reproduce this separate offline classification:

```sh
python3 tools/linux/military_initialization_ff31_capture_audit.py \
  --directory ../c6-ghidra/evidence/linux-military-initialization/native-3 \
  --output evidence/linux/military-initialization-ff31-capture.json
```
