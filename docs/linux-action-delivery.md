# Linux action delivery ABI

This is a static audit of the pinned original Linux XP2 module and matching Civ6 host. It establishes isolated request conversion and transport contracts, not action execution or complete state/action/turn parity. Reproduce with:

```sh
python3 tools/linux/action_delivery_audit.py --output evidence/linux/action-delivery-abi.json
```

The JSON records exact module identities, method byte hashes, original call sites and original virtual bindings. Private raw disassembly is in `../c6-ghidra/evidence/linux-action-delivery/`. No native fixture is executed by this audit.

## Conversion boundary

`Exports::Context::RequestUnitOperation` at `0x2c7e2c0` is a member function with four ABI arguments: Context receiver RDI, ComponentID RSI, operation hash EDX, TypedVariantMap RCX. Its 13 original bytes move RSI→RDI, EDX→ESI, RCX→RDX and tailcall the static cache manager via PLT `0x1fb22d0`. The exported member must be called with its Context receiver. The JSON names this contract `exports_context_request_unit_operation_abi`.

`Cache::Unit::Manager::RequestOperation` at `0x28061a0` is static: RDI is ComponentID, ESI the operation hash, RDX the TypedVariantMap. The JSON names this separate three-argument contract `cache_request_operation_abi`. It rejects owner values outside 0..63 and inactive cached turns, then calls the live Unit Operations Manager vtable offset `0x100` at `0x2806203`.

That slot binds `Manager::MakeRequest` at `0x2da2af0`: `bool(receiver RDI, hash ESI, map RDX, Request* RCX)`, with result in AL. It maps the hash through manager slot `0xa0`, stores the resulting type at Request+0, rejects type -1, obtains a handler through slot `0x88`, rejects null, and tailcalls handler slot `0x78`. The actual SkipTurn table binds that slot to `BaseHandler::MakeRequest` at `0x2d96720`, which returns true even for an empty map. A null parameter shared pointer is not itself rejection. RequestOperation tests AL and only constructs/broadcasts UnitOperation when true. This return is conversion acceptance, not execution legality or delivery acknowledgement.

For an observational conversion hook, capture and forward the original live manager slot, preserve all arguments and return AL unchanged, and log the resulting type and acceptance. An exported MakeRequest symbol hook alone does not intercept the vtable call.

## Transport boundary

The request calls BroadcastMessage through PLT `0x1fb9798`, GOT `0x3b21cc0`, at `0x280622e`. BroadcastMessage `0x2d23df0` serializes the original message then invokes live EngineUtility vtable offset `0x78` at `0x2d24055`. Its ABI is `(receiver RDI, MemoryOStream* RSI)`.

Core getter `_ZN8GameCore7Context7Globals16GetEngineUtilityEv` at `0x2103fb0` returns the live utility with no arguments. The original host EngineUtility table `_ZTVN8GameCore13EngineUtilityE` at `0x53a6c80` has address point +16; slot `0x78` binds `_ZN8GameCore13EngineUtility11SendMessageERN13Serialization13MemoryOStreamE` at `0x32b4d00`. Capture the live original pointer before any forwarding hook and verify its module identity. This distinguishes conversion from reaching the transport bridge without replacing transport behavior.

That original bridge constructs NetGameCoreMessage and directly calls host `Network::Manager::BroadcastMessage` at `0x33bdbd0`. It does not call Manager::SendMessage at `0x33bdbb0`. The bridge always returns EAX=0 after broadcast; that value does not acknowledge delivery. Host direct calls also do not pass through the core broadcast GOT.

The manager silently returns when its session pointer at +8 is null. Otherwise it tailcalls session slot `0xc0` with `(session RDI, FNetMessage* RSI, bool EDX=1, bool ECX=0)`. The original FNetSession binding is `0x39e8a28`. It first requires nonnull live GetSyncManager slot `0x280`, then requires a nonnull pointer at session+`0x28`. This audit leaves the latter pointer semantically unclassified. Subsequent routing depends on actual players, host/migration status and message flags; broadcast entry alone does not establish local execution.

Read-only host accessors, all returning pointers in RAX:

| Original symbol | Address | Receiver and result |
| --- | --- | --- |
| `_ZN7Network7Manager11GetInstanceEv` | `0x2fa09f0` | no receiver; manager |
| `_ZN7Network7Manager10GetSessionEv` | `0x3044300` | RDI manager; manager+8 |
| `_ZNK11FNetSession14GetSyncManagerEv` | `0x39d2194` | RDI session; session+0x38; live slot 0x280 |
| `_ZNK11FNetSession16GetPlayerManagerEv` | `0x39d218e` | RDI session; session+0x40; live slot 0x278 |

Use the real session's live virtual bindings for derived implementations. Record nulls and return naturally; do not fabricate session fields, bypass returns, or manufacture flags.

## Receive boundary and remaining gap

`NetGameCoreMessage::Execute` at `0x33af7f0` requires original Network::IsGameCoreActive, a nonempty payload and a nonnull original Context. It forwards `(context RDI, fromID ESI, MemoryIStream* RDX)` through context slot `0x230`. Queue processing and UnitOperation execution remain later boundaries.

The reported loaded fixture reached RequestOperation with an active cached player but produced no queue/receive/execute events and unchanged full save bytes. This audit identifies observations needed to locate that runtime failure; it does not establish which gate rejected it. Full affected-state comparisons, native action execution and the complete external-policy decision-boundary goal remain open. Existing raw Save→Load differences, including the recorded 89,198-byte failure, are not reclassified or masked by this work.
