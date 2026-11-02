# Proposed Linux core command oracle contract

This is a static boundary review, not an executable oracle or permission to reroute a rejected request. The original Linux Core has a canonical **queued command ingress**, `IContext1::QueueMessage`, reached by the original `NetGameCoreMessage::Execute`. It is a legitimate interface at which to define a separately identified headless transport provider. Its existence does **not** prove that the current fixture can use that provider while retaining all original acceptance semantics. The current original host receive deferral remains a failed precondition. Direct `ReceiveMessage` is unsuitable for the required queue/worker contract.

The Rust library implements bounded primitives, transport and rules/registry components; complete gameplay state, action execution and turns remain unimplemented. This proposal enables investigation of an original-Core transition oracle in parallel with the [full-host delivery work](linux-action-delivery.md). It replaces neither that work nor the complete exact state/action/turn objective.

## Evidence and exact identities

All addresses are pre-rebase ELF virtual addresses for XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. Matching dependencies, rules DB, DLC/mod order, save and configuration remain mandatory execution gates. No native execution, Ghidra project opening or existing-runner change was performed for this review.

The discovery reference is cru121's *Civ6-GameCore-reference*, revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, especially `findings/topics/exe-gamecore-architecture.md` and its engine/network interface material; attribution and CC BY 4.0 terms are in [reference.md](reference.md). The reference distinguishes Core commands, engine services, gameplay scripting, cache/reporting and controller scheduling. Its ownership shorthand and cross-platform slot observations are leads; conclusions here use actual target bytes. In particular, an interface's presence is not evidence for replacing its provider's semantics.

Reproduce the new static audit:

```sh
python3 tools/linux/core_oracle_boundary_audit.py \
  --output target/linux/core-oracle-boundary-static.json
```

The [checked static report](../evidence/linux/core-oracle-boundary-static.json) pins six Core method bodies, two relocated Context slots, seven Core call edges, two forwarding tail jumps and original host sender/queue forwarding. Existing `action_boundary_audit.py`, `action_delivery_audit.py` and `action_execution_audit.py` cover adjoining boundaries. Private raw evidence is under `../c6-ghidra/evidence/linux-core-oracle-boundary/`; existing full-host captures remain in their original evidence directories.

## What each ingress actually does

| Boundary | Actual target behavior | Decision |
|---|---|---|
| Context ReceiveMessage `0x2c7d310`, slot `+0x228` | Immediately forwards to static Manager ReceiveMessage `0x2d243c0` | Reject for this oracle: executes immediately without native queue ordering, worker pass, queue pop or post-message reporting |
| Context QueueMessage `0x2c7d320`, slot `+0x230` | Forwards to Manager QueueMessage `0x2d246a0`; original list `push_back`, sender storage, vector resize and byte copy; original recording behavior | Canonical Core provider interface, conditional on proven external admission and scheduling obligations |
| NetGameCoreMessage Execute `0x33af7f0` | Checks actual Core active/nonempty payload/Context; creates native input stream at offset zero; gets real envelope fromID; invokes Context QueueMessage | Preferred original adapter if a replacement provider is eventually proven; calling it below failed host gates does not preserve those gates |
| FNetMessage Process `0x39e0262` | Calls Execute unless processed byte already set, then marks processed | Retain with original message lifecycle to preserve processed-once bookkeeping; direct Execute alone omits it |
| UnitOperation Execute `0x2d28a00` or manager Add/setters | Omits request/transport/queue stages when invoked directly | Never an end-to-end action oracle |

Manager ReceiveMessage reads the first u32 message key, resets stream offset to zero, finds the registered message, calls original `BaseMessage::SetFromBuffer`, performs original debug/log work, sets the original executing-message state and invokes its Execute virtual. Unknown key follows the original no-dispatch branch. No general malformed-stream safety or source authorization is established by this body.

Crucially, its `fromID` is used for debug/log formatting in this exact body. It is not passed to `SetFromBuffer` or message Execute. UnitOperation Execute checks the **encoded component owner** with the player manager, finds the encoded actual unit, invokes the original operations manager, processes pending diplomacy and updates end-turn blocking. Those checks do not establish that the network sender was entitled to command that owner. Native request creation, transport sender identity, session eligibility and original manager legality must all remain represented; preserving an integer called fromID is insufficient.

Original ProcessMessages `0x2d248a0` repeatedly constructs a native input stream from the front entry, calls ReceiveMessage, pops that front, sends reporting event `0x9ea4c222` and continues until empty, including messages added during execution. Thus immediate ReceiveMessage is observably different even for one otherwise valid packet. Queue-empty establishes neither passive-world completion nor an external decision boundary.

The separately named original EngineUtility SendMessageToLocal at `0x32b4d60` is also a transport adapter, not immediate Core execution. It constructs a genuine NetGameCoreMessage, obtains actual Network GetLocalPlayerID, calls original Network SendMessage `0x33bdbb0` with that destination, destroys the temporary and returns zero. It supplies no delivery acknowledgment and does not establish an alternate permission to bypass incoming gates. The existing unit request uses Broadcast; changing it to the local variant would require its own semantic evidence.

## ABI and lifetimes

Linux x86-64 SysV uses RDI/RSI/RDX for `(Context*, int fromID, MemoryIStream&)`; Manager Queue/Receive are static `(int EDI, MemoryIStream* RSI)`. The Context methods return void. NetGameCoreMessage Execute is `void(message* RDI, FNetSessionIFace* RSI) const`; its ordinary target body does not consume the session argument, but callers must retain the real live session. Original `FNetMessage::GetFromID` at `0x33afab0` returns i32 message `+0x28`.

Execute uses the original message vector, original `GameCore::MemoryIStream` constructor and original SetOffset(0), then destroys the stream after synchronous QueueMessage returns. QueueMessage makes an original-owned copy, so temporary stream/backing bytes must survive that call; no native vector/list/message object may be fabricated. Preserve the original Process ownership path, destruction and reentrancy rules. Concurrent delivery while the worker drains is unsupported until original lock/order discipline is proven.

Host `GameCore::GetContext` `0x31dd790` uses hidden return storage. It ordinarily copies the actual cached Context pointer; no AddRef is shown. NetGameCoreMessage Execute nevertheless runs original ICvUnknown cleanup for that returned value. Preserve this original caller behavior; do not invent reference counting, delete the framework's underlying Context, or confuse it with controller GetContext's borrowed return. Acquired GetGame/GetPlayer/GetUnitOperations facades retain their established original cleanup convention. Keep framework, Core, host services, maps, registries, session/controller, scripting and stream providers alive through execution and synchronized observation.

## Concrete input provenance

The existing actual RequestUnitOperation, using a freshly discovered real owner-0 unit 15794187 and genuine native empty parameter map, produced a 28-byte command with SHA-256 `5c262073a0b62b252c10ca562ca765faff535bc0e6bdf78c4f41f331498f288a`. Private files `linux-action-execution/transport-hardware-3/packets/001-engine_utility_enter.bin` and `007-session_message_received_enter.bin` are byte-identical at actual original EngineUtility send and session receive. Their `events.jsonl` and `replay.json` retain source/configuration provenance. This establishes command-byte transport identity, not receipt into the Core queue. **The actual envelope fromID was not captured in that profile**; owner 0 is not a substitute for that observation.

A future provider must receive the live original request/broadcast output, retain the native message or an independently validated original serialization/deserialization reconstruction, observe its original sender, and preserve ordering and exactly-once behavior. Historical captured bytes are diagnostic inputs only until their complete matching-state and sender preconditions are established. Do not manually encode a packet, choose operation 36 from a prior run, reuse actor IDs as constants, inject queue nodes or execute a fabricated message object. Native CanStart/MakeRequest acceptance must be observed afresh; the public parameterless branch's append=false/fromUser=false remains part of this profile.

## Provider obligations and unresolved classification

The host adapter is outside the Core DSO by placement. That alone excludes no behavior. A distinct headless provider can avoid window/render/UI bootstrap only after each omitted service is proven presentation-only or replaced by a behaviorally equivalent provider. The required split is:

| Obligation | Evidence/classification now | Required provider behavior |
|---|---|---|
| UI/window/audio presentation | Not a prerequisite imposed by the Core QueueMessage ABI; some services share dispatch with gameplay | Do not require presentation bootstrap solely to manufacture nonnull state; audit transitive callbacks before omitting them |
| Actual request legality, cached owner/active turn, original MakeRequest and registered handler | Gameplay acceptance | Retain original checks and byte production; stale-state rejection must match |
| Sender mapping, peer/snapshot/app registration, host migration and sync sequencing | Host transport with potentially gameplay-affecting admission/order | Preserve or prove equivalent, including rejection/defer outcomes and generated actions; no blanket exclusion |
| Load/save deferral | Lifecycle isolation from serialization and state replacement | Prove genuine Core readiness and equivalent exclusion/order, not a renamed bypass of native false CanReceive |
| Controller executing and Game.IsAiProcessing deferral | Gameplay concurrency/order | Retain synchronized exclusion; never force flags or call below a busy gate |
| Controller event subscribers, player activation/local-player change, cache updates, barrier/processing-complete handshake | Potentially gameplay-affecting | Audit and retain transitive effects and exact event order; not all host events are UI |
| Reporting, cache dispatch and gameplay Lua | Gameplay-affecting until proven otherwise | Preserve native callbacks, outputs and ordering; IScriptSystem is an engine service used by gameplay too |
| Passive rules, diplomacy, gameplay RNG, NPC state | Gameplay | Preserve all effects even under AI namespaces |

Original registration events 55/82/54 now complete before controller activation in the separately owned full-host fixture. Replaying registration after activation can itself produce a gameplay action; it is not a harmless readiness fix. Native CanReceive still calls Serializer.IsLoadInProgress; the observed null-current/null-load comparison returns loading. It is reasonable to design a provider that has no UI game-state machine, but the semantic replacement for that lifecycle predicate is **not proven** merely because both pointers are null. Current original incoming deferral cannot be discarded or double-delivered. Original SetupGameStates enters InitApp.Begin and ForgeUI/scripts; this document does not authorize that rejected/bootstrap path, split its initialization or mutate its state.

## Proposed progression and continuation contract

Before implementation authorization, prove a provider admission record containing actual request/sender/actor identity, complete source configuration, original legality/conversion result, original packet identity, session/order eligibility and genuine serialization/controller quiescence. Trace every omitted full-host gate to its gameplay dependencies; preserve uncertain gates in scope. Establish a legitimate independent provider lifecycle rather than accepting a packet the active original provider has deferred. A conditional transition experiment may explicitly assume such eligibility, but then its claim is **Core transition under supplied admission preconditions**, not original full-host accepted Request parity.

Only after this proof, the candidate sequence is original Request → original serialized native message → validated provider admission → original Process/Execute → original Context QueueMessage → original threaded ExecuteGameCoreUpdate → original Synchronize → complete affected-state observation. The ordinary full-host route remains the reference sequence when available. Never send through both providers in one fixture; exact original outgoing/incoming/sync queues must either remain authoritative or have a proven mutually exclusive provider handoff. Do not consume, erase or mark the original deferred message to create that handoff synthetically.

The worker must remain the original threaded ProcessMessages path. Inline controller Game.Update is a different contract. Preserve mandatory original reporting/cache/event progression and any subsequent rule work without inventing an arbitrary fixed pump count. Return `InProgress` with captured unfinished work for bounded diagnostic timeouts. A guard termination after mutation is a failed isolated process, not `Unsupported`, `DecisionRequired` or a resumable yield.

There is no proven universal native continuation API that yields before **every** player policy decision. Human configuration only establishes one branch. Off-turn jobs, diplomacy responses, minors/barbarians, automation and script callbacks remain in scope. Actual UnitOperation invokes ProcessPending, so even an action with no direct Game.Update can reach policy. All genuine decisions must go to the external policy; no AI stub, native AI choice, forced return, patched function/vtable/flag or direct setter may fulfill them. A future implementation needs proven continuations or fixtures whose entire transitive path reaches no policy decision. The latter permits only a bounded action oracle, not general turns.

## Required fixtures and comparison surface

Before any passing action claim, run a no-request control with identical providers and observations, an original fresh legal request, repeat from a verified matching initial branch, and preserve minimized failures. Compare original request conversion/rejection, ordered bytes/senders, delivery/defer/duplicate outcomes, queue/Receive/Execute/report sequence, all affected gameplay state and RNG. Include stale/wrong-owner actor, inactive/completed player, invalid hash/parameters, illegal operation, preexisting operation replacement/append, unknown message key, pending recording/playback, load/save/busy worker, pending diplomacy and transitive policy entry. Malformed streams require separate stream/parser safety evidence before native execution; they are not authorized by this static design.

Capture complete Core streams and raw differences, live unit/formation/operation nodes and parameter ownership, cache/dirty order, notifications/end-turn blockers, diplomacy/NPC/session state, reporting/subscriber order, gameplay script effects, pending jobs/messages and complete gameplay RNG inventory. Two known RNG objects are not proof of complete RNG. Do not compare only a selected actor or normalize unknown bytes. The recorded raw Save→Load **89,198-byte failure** remains unmasked; full branch restoration is unresolved. Independent fresh process reloads must still prove matching complete relevant initial state before comparing runs. Queue drain, a HOLD state or matching Core Save alone is insufficient.

The static result is therefore a concrete canonical **interface and conditional contract**, with unresolved admission/provider equivalence, affected-state completeness, branch restoration and universal policy continuations. Until those are supplied, neither direct ReceiveMessage nor direct NetGameCoreMessage Execute is a faithful drop-in repair for the deferred full-host request.
