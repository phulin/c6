# Conditional original Linux Core command component

This separately identified experiment constructs a fresh original Core command and tests its queued Core transition **conditioned on supplied admission preconditions**. It does not send a public Request, reroute a deferred network packet, repair original host admission or establish full action parity. The complete Rust gameplay state/action/turn API remains unsupported; current Rust coverage is bounded primitives, transport and rules/registry components.

Root authorized this bounded component investigation after original full-host evidence identified a genuine sender-0 command deferred by original CanReceive reason 1: the minimal host's null current/load-state pointers compare equal in Serializer.IsLoadInProgress. The actual message remains in its original incoming deque. This helper neither consumes that message nor changes the predicate, native flags, pointers, vtables or original code. It is a fresh isolated profile with **no original Request/broadcast/network update call**. The [full-host investigation](linux-action-delivery.md) and [conditional ingress design](linux-core-oracle-boundary.md) retain their distinct acceptance requirements.

## Original sequence and ownership

The caller must gate XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`, matching dependencies and complete input/configuration identities. `c6_linux_core_component(module, borrowed_context, execute)` borrows the genuine framework Context and already initialized native threaded controller/cache. The existing selected policy/update trace must be armed and remain unchanged. Loading this C entry point by itself supplies none of those gates.

The component reuses owner-approved original actor discovery and bounded observation APIs from `action_dispatch`/`action_state`, without editing those owners' sources. It selects in original iterator order, checks actual owner/local-player identities, original public CanStart(SKIP_TURN, genuine empty TypedVariantMap, false, null), native live/cached human active control and activation completion, and observes the existing operation queue. At most one original queue node is allowed by this experimental profile. No node is cleared synthetically. Append=false lets the original manager perform replacement and old-handler effects.

The native call sequence reproduces the **construction/conversion portion** of original Cache RequestOperation `0x28061a0`, stopping before its broadcast:

1. Original Request C2 `0x213cad0` initializes 32-byte aligned storage. Original manager Get `0x2da1260`, then its actual virtual `+0x100`, converts the native hash/real parameter map into that Request. It records the native result and booleans rather than assigning operation fields.
2. Original UnitOperation C1 `0x2d28970` initializes 72-byte aligned storage. Original Init `0x2d289d0` copies the genuine ComponentID and converted Request with `InsertMode::USE_REQUEST = 0`, matching the target caller.
3. Original BaseMessage PutInBuffer `0x2d20ec0` serializes the complete message into an original fixed-buffer MemoryOStream. The bounded empty-map profile requires the **actual** output offset to be 28, writes those actual bytes to a private exclusive-create artifact and preserves them through QueueMessage's copy. It does not write wire fields itself.
4. In execute mode only, an original MemoryIStream over those bytes feeds original Context QueueMessage `0x2c7d320`; actual Network GetLocalPlayerID supplies sender, which must equal Context local player and the ComponentID owner. This is supplied admission for a component test, not proof of sender authorization through the full host.
5. Original threaded ExecuteGameCoreUpdate and Synchronize drain the native queue. Exactly one observed UnitOperation execution, idle worker, no remaining messages and strict final turn/phase/local/update/advance/AI/processing/diplomacy checks are required for zero helper result. There is no fixed multi-pass pump, network update or inline Game.Update substitute.

Original factory CreateMessage allocates exactly `0x48` bytes at `0x2d28da2`, proving complete message storage independently of the caller stack frame. Matching prior target DWARF establishes Request size 32, with bools at +28/+29 reusing base tail padding. The helper uses exported C2/D2 variants: original UnitOperation D2 `0x28079d0`, then original Operation D2 `0x213cac0` for Request, matching the original stack caller's cleanup. Original message and Request survive serialization and synchronous queue copying. Genuine shared parameter ownership is acquired/released by native assignment/destructors.

MemoryOStream uses native allocation `_Znwm(0x28)`, original fixed-buffer C1 and deleting D0, as in the established `context_stream` adapter. Its backing buffer remains alive through destruction. MemoryIStream uses aligned 40-byte storage, original C1 and D1, with its backing bytes alive until original QueueMessage returns. No original object fields are fabricated. Acquired facades/maps use established original cleanup; genuine core unit/container pointers remain borrowed. Framework, controller, session, maps, engine/script services and module providers must outlive the helper.

## Separate controls and result interpretation

Set these private exclusive-create paths in both control and execute profiles:

```text
C6_CORE_COMPONENT_SNAPSHOT_BEFORE
C6_CORE_COMPONENT_SNAPSHOT_AFTER
C6_CORE_COMPONENT_SNAPSHOT_REPEAT
C6_CORE_COMPONENT_COMMAND
```

Control mode performs the same actor discovery, legality, MakeRequest, constructor/serialization, command capture, observation and teardown. It does not queue or run the worker. Execute mode starts a separate isolated process from the same identified original input. Repeat execution requires another isolated process with matching complete relevant initial state; unresolved save branch restoration is not silently replaced by a matching selected actor.

Logs use `C6_LINUX_CORE_COMPONENT`; source is `fresh_native_make_request_component`. Every component outcome retains `host_admission_proven=false`, `full_action_acceptance=0` and `gameplay_parity=false`. The original full-host dispatch checker must continue rejecting missing Request/Broadcast evidence; it must never accept this profile. A separate result schema must compare genuine conversion, command capture and queue/Receive byte/sender identity, balanced original execution, strict helper/cleanup outcomes and raw snapshots. Trace phase 1 covers component conversion/queue, phase 2 its return, phases 5/6 worker/synchronization and phase 7 observations; a phase number alone does not mean a full Request occurred.

The no-action control must show no queued/received/executed message, unchanged live observations/RNG objects and identical full Core snapshots. The execute run must show the actual queued message and synchronized execution, raw before/after Core differences, repeated post-state stability and changed bounded live state consistent with original mechanics. Preserve all nonzero and fatal-guard outcomes. A native policy/update guard exits the isolated process after possible mutation; that is a failed diagnostic, never an external-policy continuation or nonmutating Unsupported result.

This observes only the existing selected-unit/queue/dirty-index surface and **two identified RNG objects** plus complete Core stream bytes. Formation/cache/reporting subscribers, notifications, diplomacy, complete gameplay RNG, jobs and live state not serialized into Core streams still require coverage. Native UnitOperation always processes pending diplomacy and end-turn blocking after a found actor; their effects remain gameplay. Selected trace coverage does not prove universal policy coverage. All actual player decisions must remain external, including NPC/diplomacy/off-turn/automation decisions; no native AI choice may count as external input.

The recorded raw Save→Load **89,198-byte failure** remains unclassified and unmasked. Neither a successful original SkipTurn component nor matching streams establishes exact complete gameplay state, branch restoration, all-player decision continuations, end turns or Rust action acceptance.

## Validation checkpoint

The [static report](../evidence/linux/core-component-static.json) pins thirteen original constructors/destructors/serializer/sender methods, original manager/Queue virtual bindings, original factory allocation and the helper source hashes. Run:

```sh
python3 tools/linux/core_component_audit.py --output target/linux/core-component-static.json
cc -std=c11 -Wall -Wextra -Werror -fPIC -fsyntax-only native/linux/core_component.c
```

These checks do not execute original code. Native lifecycle/profile integration and its separately reported control/action/repeat results belong to the runtime owner. No native result is claimed by this source/static checkpoint. Private original instructions and matching prior DWARF remain research evidence; cru121's pinned reference attribution and limitations are retained in [reference.md](reference.md).
