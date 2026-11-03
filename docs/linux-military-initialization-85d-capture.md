# Separate fresh 85d Military initialization capture

The closed `native-4` capture supplies fresh original Read/Initialize witnesses for
the `85d858ef…` Core input. Its two own actual Save outputs are `27ca0057…`, each
with **65,552 unmasked byte differences** from that input. There is no observed
`27ca` Read initialization. The historical 89,198-byte Save→Load failure remains
recorded and unmasked; complete gameplay state/RNG, actions and turns remain
unsupported.

This is a third separate packet. The [native-2 packet](linux-military-initialization-capture.md)
in `ecbef25` and [native-3 packet](linux-military-initialization-ff31-capture.md)
in `71cd09e` remain unchanged. The incoming initialization context comes from this
capture's own matched Read/Initialize calls, never a prior writer snapshot.

The [new offline driver](../tools/linux/military_initialization_85d_capture_audit.py)
reuses the frozen artifact/event validator and 13 corruption refusal checks. It
also verifies the closed-output selector, origin runtime/summary references and
actual compiled preload. The [new report](../evidence/linux/military-initialization-85d-capture.json)
records role-local owners/ranges/context hashes and complete input/output byte
differences. No original execution, typed Rust admission, candidate comparison or
prediction-artifact read is performed by this classifier.

## Closed identities

The private closed directory is
`../c6-ghidra/evidence/linux-military-initialization/native-4`.

| Artifact | SHA-256 |
| --- | --- |
| Actual runtime report | `f4ddea674a5ddc262a393e17efe049a15a1c145d3bf046a348ce1a411dd05c03` |
| Raw 3,057-row observations | `07925f3a4bf4c9d53a5c3d34a869b04d61dadb0275132ed3f4da3769a1095654` |
| Frozen ordinary Load/Save profile | `450387b547ff8b61eaeaf24a2b52d0576b0543698aa9f5891357274296bb598c` |
| Archive of 37 captured source preimages | `029b3a2c4f759a3203bcf866e3bc8c6901bf4ecdcab4603af3b28f96c796b6b5` |
| Actual compiled preload | `e96e49a9aca79692d2b6e8d9840ad49633967c3d2d9592bd8dc6934acceb43fe` |

The source-reviewed closed-output selector pins origin runtime
`01479e18272065e11ab0a50dc2284b6fdf4fce77a166b07ae98a19bba5dfa8b3`
and origin summary
`d77e86bfd9dce8b5857e20c6ff0213cd52510dfc0cac78e19d7a87ca3913ae3e`.
Their bytes and the archived selector JSON match the actual capture configuration.
Those references establish input provenance; they do not supply initialization,
predict the next output or turn raw Core bytes into a complete save container.

The pinned Linux XP2 module and host, actual input/output files, all declared
closed artifacts and archived/unpacked source preimages are rehashed. Prepared,
actual and raw-plan configuration identities agree. Native process/debugger exit
statuses are zero; the terminal supervisor records no remaining session processes.
Startup, Shutdown, framework and client cleanup returned. Controller
Initialize/Uninitialize were not invoked. All 917 paused stops retain the actual
null-worker/controller-not-initialized guard. The final recorded source guard
checks 34 original method fingerprints.

## Actual stream roles and contexts

| Stream occurrence | Actual role | Bytes | Military records |
| --- | --- | ---: | ---: |
| 1 | Fresh incoming `85d` Read/Initialize | 80,657,462 | 43 |
| 2 | First own `27ca` Save output | 80,657,462 | 43 |
| 3 | Second own `27ca` Save output | 80,657,462 | 43 |

The complete actual incoming Core SHA-256 is
`85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8`.
Both own output SHA-256 values are
`27ca0057dc37b1e5ca8b726373d31b10bfe5eb81e493137719c8848a5fd5365c`.
The separate whole Save supplied to the configuration loader remains `a328d643…`,
7,348,717 bytes; its extraction relationship to the actual MemoryIStream input
remains explicitly unproven.

All 43 input records have actual prior vectors, installed-owner witnesses, nested
Initialize entry, personality/leader/query/count observations, normal initializer
return, the paused initialized snapshot and normal Read return. Each actual
personality result is true, allowed-query pointer is nonnull, handler default is
zero, active definition count is seven and initialized allowed-vector size is
seven. Serialized counts never supply those facts.

All three typed vectors are retained at their genuine stages. Twenty current
vectors change during original Read. Observed allowed and per-war values remain
unchanged between initialization and Read return for this fixture. The separate
per-war query/preparation/rows remain unobserved; its complete normal initializer-
return snapshot is preserved without claiming SQL replay.

All 86 own Writes have actual entry/guard/return witnesses, 258 ordered helper
calls and 86 file-closed output bindings. All 129 original Military ranges are
checked against the actual stream offsets, with three physical destroyed stream
lifetimes. Actual saved owner IDs are `0..40, 62, 63`, without inferred outer
Manager ordinal/requested-ID mappings. Selected-frame owner lifetimes do not prove
cross-stream continuity. The observed portable active-definition ordering hash is
`e895435ea6ff98783727bd6334736a39d94ca0413ac00e68f9675fe90c0f49c2`.

The raw `capture_complete=false` footer remains unchanged pending independent
attribution/admission. Actual rules/config/content/DLC/mod input gates remain
separate from the unavailable in-memory gameplay database byte snapshot. Runtime
policy-call counters are unavailable. One post-Load raw64 known-RNG observation
is retained; Save endpoint equality remains a frozen source-guard witness, with
no independently emitted endpoint dumps. Complete gameplay equivalence is not
established.

Reproduce this third separate offline classification:

```sh
python3 tools/linux/military_initialization_85d_capture_audit.py \
  --directory ../c6-ghidra/evidence/linux-military-initialization/native-4 \
  --output evidence/linux/military-initialization-85d-capture.json
```
