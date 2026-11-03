# Focused original Linux action reporting observer

The first conditional Core SkipTurn runs observed nine entries through three selected identity reporting overloads. That is an incomplete reporting surface. Original notification Activate additionally calls `Send(Type, Value const&, Value const&)`; native message draining calls `Send(Type)`, and a changed local end-turn blocker can call `Send(Type, int, int)`. The new `proven_skipturn_callers_v2` profile observes these six overloads. It does not establish a complete engine/subscriber event inventory or full gameplay parity.

All contracts target XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. [The read-only audit](../tools/linux/action_event_audit.py) and [checked contract](../evidence/linux/action-event-abi.json) pin original instructions, matching native layouts and the observer source. New proprietary excerpts remain local under `../c6-ghidra/evidence/linux-action-event/`; no upload is authorized for those new files. Reference discovery/attribution remains in [reference.md](reference.md).

## Native ABI and actual values

Matching target DWARF identifies every reporting Send overload's return as **int**. Event Type is eight bytes: event hash at +0 and subtype hash at +4. Linux SysV passes that integer-class aggregate in RDI; Value const references are pointers in RSI/RDX. `Send(Type, int, int)` uses ESI/EDX for the two integers. The original int returns in EAX.

Reporting Value is 32 bytes: key u32 at +0 and original 24-byte TypedVariant at +8. Its variant type/modifier u16 fields are at Value +8/+10; storage is at +16, element size u32 at +24 and native count/flags u32 at +28. Original ArrayVariant masks the low 24 bits for count and tests array flag `0x20000000`. Native bool construction creates variant type 1/modifiers 0 and scalar element-size/count fields zero. The observer reads only the actual scalar bool byte. For activation's actual ComponentID argument, the original Value constructor uses user type 1024, element size eight and count one; the original const GetAs<ComponentID> getter supplies the native array data pointer. No original constructor, setter or conversion is invoked by observation.

Original Notification Instance Activate(bool) `0x2d2dd80` first sets HasBeenActivated at +0x19e, then constructs event `0x025f6cfd`. Its first Value key is `0xeab69e5a`, containing the genuine notification ComponentID returned by GetID; its second key is `0x78a2f946`, containing the actual incoming bool. At `0x2d2dde4` the original code calls ValueValue Send through PLT `0x1e578a0`. Both temporary Values remain native-owned and live through the call.

Original ProcessMessages calls Type-only Send for `0x9ea4c222` after Receive/Execute and queue pop. Original SetEndTurnBlockingState `0x2b5be10` conditionally calls Type/int/int Send for `0x55f15a7b` at `0x2b5be54`, with old/new blocker values, when the local player's blocker changes. The old nine selected entries cannot establish whether that conditional call occurred.

## Forwarding and limited coverage

Every reporting wrapper now calls the original with the unchanged arguments, saves its actual int result, performs after-call logging and returns that saved result. A before-call sequence number pairs each entry with an original return record. Identity payload bytes and Value32 bytes are compared after the call; captured ComponentID userdata's eight bytes are compared too. Unexpected general Value types are reported as unclassified and prevent the new bounded checker from accepting typed event coverage; the original function is still forwarded without fabricated results or values.

The runtime verifies all sixteen original selected GOT/PLT bindings against the actual preload-owned wrappers, including the three new reporting overloads. The component checker requires the new profile and exact configuration/source contract, balanced entry/return identities, unchanged captured inputs, typed activation payload and queue-complete ordering after original UnitOperation return. Its repeat comparison checks selected call order and native return observations. These are **original Send entry/return observations**, not proof of complete host event delivery, cache/subscriber behavior or complete event ownership.

The six overloads cover the currently proven unit replacement/SkipTurn, notification activation, conditional blocker and queue-drain callers. Reporting triggered by other transitive subscribers, scripts, subtypes or overloads remains an explicit gap. The library still lacks complete Rust gameplay state, action execution and turns; gameplay-affecting events and state remain exact requirements.

## Historical return ABI gap

The first three reporting wrappers were declared void even though original Send returns int. In the archived execute-3 preload, a stack-canary epilogue overwrites RAX after the original call. This is a material instrumentation ABI defect; it must not be described as generally transparent reporting forwarding.

The nine recorded entries correspond to five known original caller sites. SetActivityType (`0x2da1897`, three entries) and PopFront (`0x2da1d5d`, two entries) ignore the result in their enclosing void methods. BaseHandler Deactivate (`0x2d9650b`, two entries) immediately zeros EAX for its own false return. InsertAtEnd (`0x2da263f`) calls GetSize before its later EAX comparison, and SkipTurn Start (`0x2dac91d`) calls manager Edit before later RAX use. Exact caller bodies and epilogues are pinned in the new audit. Historical traces did not record caller PCs, so this is a bounded static path audit, not a universal declaration that ignored returns are harmless.

Old raw runs, observed queue/action/state outcomes and conditional limitations are preserved. Their reporting scope remains incomplete and their ABI declarations remain defective. A corrected fresh control/execution/repeat must establish the new observer profile and compare its bounded outcomes; no corrected native result is claimed by this source/static checkpoint. Full-host admission, complete affected state/RNG and Rust action parity remain unproven.

Run the static checks without executing original code:

```sh
python3 tools/linux/action_event_audit.py --output target/linux/action-event-abi.json
python3 tools/linux/action_execution_audit.py --output target/linux/action-execution-abi.json
cc -std=c11 -Wall -Wextra -Werror -fPIC -fsyntax-only tools/linux/action_execution_trace.c
```
