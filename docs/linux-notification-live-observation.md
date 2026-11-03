# Passive notification inputs for the accepted Linux fixture

`tools/linux/notification_live_observe.py` prepares and sources a bounded, read-only GDB observer for the already accepted conditional SkipTurn fixture. It does not launch a process. No native observation is claimed by this tooling checkpoint. Complete gameplay state, action execution and turns remain unimplemented.

The first capture uses one run and four hardware execution breakpoint lanes, with each lane rotated only after its Python `stop()` callback returns. The custom `c6-notification-drain` command resumes the existing inferior between observations. No inferior function call, memory/register/vtable write, software game breakpoint, forced return or game-code patch is performed. GDB's ordinary loader breakpoint remains separate instrumentation; debug timing differs from an unobserved run.

## Exact first-run scope

| Lane | Original boundaries | Actual inputs |
|---|---|---|
| Update | `2d2f360` entry, cloud getter result `2d2f37e`, conditional local-player result `2d2f390`, original return `2b5bdd9` | Owner's complete ordered list before/after; manager vector initialization, game-pointer presence, direct live local player/uninitializing, actual cloud result |
| Selection | `2d30ec0` entry from the proven `2d2f618` caller, return `2d2f618`, conditional virtual Activate call `2d2f608` | Every rescan's ordered records and signed lock counts; actual threshold/network mode; actual selected logical object; live Activate binding and false argument |
| Eligibility | `2d2e1f0` entry, conditional network/definition/icon results `2d2e20f`/`2d2e223`/`2d2e230`, exit `2d2e23f`, conditional severity branch `2d30f6c` | Actual short-circuit path, definition found/missing, type agreement, direct definition icon bit and getter result, resulting IsAutoActivate; native effective severity override and comparison inputs |
| Cleanup | virtual Release call `2d31252`, original return `2d31255` | Real pool binding/order before Release; actual release/destructor targets; pool order and cleared parent link after a pooled Release |

The initial snapshot is at original Update entry, before SendAutoNotify. Selection snapshots are later and repeated after synchronous activation/reporting hooks. Initial lock counts or definitions cannot replace those branch-time observations if callbacks modify state. Each snapshot identifies the live ComponentID separately from its logical object handle: duplicate serialized IDs do not merge objects. Pool prefix objects are recorded as ordered handles without dereferencing their unobserved contents. Physical pointers are used privately for consistent handle assignment, never as cross-run equality requirements.

The Rust ProcessAutoActivate→Cleanup boundary must use the **first Find snapshot after SendAutoNotify**, including its pool prefixes. The adapter must reject if that snapshot's needed serialized inputs differ from the immutable Save-before records, or obtain separately attributed starting records. Initial Update pools cannot substitute for the later phase inputs merely because the serialized owner list stayed equal.

Actual entry to Find from `2d2f618` proves ProcessAutoActivate passed its AcceptsNotifications and owner-turn-active branches (`2d2f5c2`, `2d2f5dd`). The pinned manager/Accepts contract then establishes initialized=true, game-uninitializing=false and human-configuration=true at those checks. These are explicitly labeled branch-derived observations, not fabricated direct reads. Initial manager initialization is also directly read. Local player and uninitializing are read through the exact pure getter layouts: Game `+248` → PlayerManager `+1c68`, and Game bool `+2ae`. Getter bodies are fingerprinted offline and checked live.

An already-activated or network short circuit can omit definition lookup. Its definition/icon value remains **unobserved**, not false or a missing definition. The Rust adapter's `DefinitionIcon::Unobserved` must retain that distinction and reject it when the supported branch needs the value. When IsAutoActivate is true, the same hardware lane moves to `2d30f6c`, verifying this exact Find/candidate/threshold before observing BL: the original first-matching `FindVariant(c362976f)` and native bool conversion result. This is the **effective native severity override**, not a parameter-map inventory: missing and present-false both yield false. The adapter must compare this effective value with its actual wire map or reject. When IsAutoActivate is false the override is unobserved and irrelevant to that branch, never filled with false. This profile does not capture unexecuted branches or claim a native boundary matrix.

The observer records separate native `Dismissed +198` and `IsExpired +19c` flags. They are not interchangeable. Base expiration uses current turn/added turn/delta; it does not read `+19c`. Expiration predicates, blocking reduction, SendAutoNotify factory effects and complete reporting subscribers remain outside this first input capture.

## Identity and read gates

Preparation requires corrected execution configuration `b6ca58d53073abc2afa9176a4ceba350fafe3f91ba12dbf84fd08e25a3be9a5b`, original XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, and host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. It rechecks canonical configuration hashing, archived source files and preload hash. On that frozen preload, `nm` proves `armed=27500` and `phase=27514`; the observer requires armed=true and actual original worker phase 5 and records both on every hit. The sole runner confirmed these symbols on preload SHA-256 `907b006e5225ac9816231eceab9124fedb2810f483ba5135232c380d4f89e9fa`.

All instruction sites, pure getter bodies and the notification virtual slots are checked against exact original ELF bytes. Live reads require readable mappings, canonical bools, aligned notification pointers, one of the 39 verified notification vtables and exact selected slot targets. Component type must be Notification (38); list members must have the actual owner. List traversal validates next/previous links and cycles; vector reads validate ordered begin/end/capacity and pointer stride. Counts/capacity are bounded at 4,096 and total hits at 10,000. Layouts come from matching target DWARF and compiled list/vector accessors in [the lifecycle contract](../evidence/linux/notification-lifecycle.json).

All-stop GDB mode is required. Other notification threads, unexpected return callers, unreadable objects, changed virtual targets, unsupported counts and other observation errors stop the inferior and prevent capture acceptance. An unpooled notification is never dereferenced after its deleting Release returns. A pooled notification's retained object and pool remain readable at the original return boundary. The tool leaves unexpected stops for the sole runner; it does not manufacture normal exit or cleanup. Reentrant/transitive behavior and timing equivalence are not certified by this bounded observer.

## Prepare, integrate and check

Preparation is offline and executes no original code:

```sh
python3 tools/linux/notification_live_observe.py \
  --runtime-report /tmp/c6-core-component-event-execute-1.json \
  --module ../c6-assets-linux/libGameCore_XP2.so \
  --host ../c6-assets-linux/Civ6 --profile accepted-skip-turn \
  --armed-rva 0x27500 --phase-rva 0x27514 --phase 5 \
  --observations /private/local/capture/observations.jsonl \
  --output /private/local/capture/manifest.json
```

Only the coordinated sole runner may schedule the native capture. In its already source-gated GDB driver, after `starti` and before ordinary continuation, source the prepared manifest using:

```text
python
import runpy
runpy.run_path('/home/phulin/c6/tools/linux/notification_live_observe.py', init_globals={'C6_NOTIFICATION_MANIFEST': '/private/local/capture/manifest.json'})
end
c6-notification-drain
```

Do not add concurrent game breakpoints: this profile consumes all four hardware slots. The native runner retains responsibility for original startup/environment/input gates, completion/Join/parking, exact command and complete Core stream hashes, exit and cleanup. This tool does not write or duplicate Core Saves; existing runner artifacts may be hashed and handled under that runner's retention rules. A successful observer boundary check alone does not establish any of those conditions.

After a coordinated completed run:

```sh
python3 tools/linux/notification_live_observe.py check \
  /private/local/capture/manifest.json /tmp/notification-live-hashes.json
```

This emits small hash/count metadata only. Private JSONL observations and the manifest remain local outside the pending 271-file upload approval. No upload is authorized for new raw evidence. The checker requires chronological lane transitions, consistent armed worker thread/phase, exact PCs, Find/eligibility nesting and receivers, and balanced original Update, Find, eligibility and Release boundaries. It rejects observation errors; it explicitly leaves original cleanup/Core identity validation and complete gameplay/live-state parity false.

Offline validation passed Python compilation, exact-source/configuration preparation and every original instruction/getter/vtable gate. A synthetic GDB/memory simulation exercised all four lanes, definition/icon lookup, already-activated short circuit, activation, pooled Release, Update return and incomplete-capture rejection, with a maximum of four hardware breakpoints. That simulation executes no original code and is not native validation. The historical 89,198-byte raw Save→Load failure remains unresolved and unmasked.
