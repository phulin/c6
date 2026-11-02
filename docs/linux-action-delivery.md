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


## Follow-up: local transport and pump deferral

The complete normal control flow of core BroadcastMessage converges on PutInBuffer and EngineUtility.SendMessage after logging; no ordinary action-drop return intervenes. HandleSendTurnUnready can cause an additional broadcast. Reaching both core broadcasts therefore does not identify two UnitOperation messages.

The host NetGameCoreMessage constructor passes message type 3 and flags 3 into FNetMessage. For the ordinary host path without migration, FNetSession.BroadcastMessage walks actual players 0..63, resolves relay targets, then SendAllMessageChunks `0x39e9462` calls session+0x28's provider vtable offset `0x10`. Missing target players, suspended messaging (unless the original override argument permits it), and zero chunks can prevent this call. Host-migration or session+0xd8 with at least two players can instead defer normal messages into the outgoing queue. Deferred entries are reconstructed and broadcast with the fourth argument true; the original outgoing drain also waits for migration/session+0xd8 to clear when there are at least two players.

For original FNetAccessENet.Send `0x39ddfee`, connection zero uses an ENetPacket queue at provider+0x18. This is the local route. Nonnull session+0x28 alone does not establish a functioning receive path. Provider Update `0x39de160` drains its queue (count at provider+0x40), calls provider+8 callback slot `0x30` with `(callback, connection=0, bytes, length, true)`, then destroys the packet. A null callback causes destruction without delivery. FNetSession.ProcessNetwork `0x39f2f32` invokes provider slot `0x38`; trace that live binding and the provider Send slot before assuming a handshake is missing. Derived providers must be checked against their actual live bindings.

Network.Manager.Update `0x33bd8c0` invokes session slot `0xd8`. Original FNetSession.Update `0x39e99f4` calls FNetMessageSync.Update `0x3a23ce6`; false at `0x39e9a7f` skips the later incoming/outgoing queues. A present pending-connection queue whose slot `0x28` update returns false at `0x39e9ad2` also skips them. The eventual drain calls are `0x39ea227` incoming and `0x39ea234` outgoing. Sync.Update calls live session ProcessNetwork slot `0x238` before obtaining synchronized messages and invoking HandleMessage slot `0x248`. It can return false after processing synchronized messages, so that return does not imply no work occurred.

Once a packet reaches MessageReceived, `0x39f29da` tests sync-manager byte +0x29. When it is nonzero and message byte +8 is zero, the message goes through original Sync.Add; otherwise it calls HandleMessage slot `0x248`. HandleMessage `0x39f2d1a` routes application message types below 1000, including NetGameCoreMessage, through session+0x18 callback slot `0x128`; a null callback silently returns. System messages take original FNetMessage.Process. Thus callback identity and presence, synchronization mode and queue movement are the next observations after nonnull initial session checks.

The most useful forwarding observations preserve original EngineUtility slot 0x78 and session Broadcast slot 0xc0, then provider Send 0x10 and Update 0x38 with their original arguments and behavior. Record packet length, connection, original target-module identity, actual callback presence, and before/after queue counts. This is an observation plan, not authorization to replace original flags or dispatch directly into Execute.

Original StartGame `0x39ec190` checks host status and CheckPreGameStatus, then LaunchGame only for status 3. LaunchGame `0x39ec290` broadcasts NetLaunchGame. Its later live callbacks have not been proven policy-free by this bounded audit; no transitive safety or need for StartGame is claimed. The transport observations above can distinguish deferral from callback loss without manufacturing a started session.


The packet-queue size at provider+0x40 is **u64**, proven by `cmpq` at `0x39de186` (`49837f4000`). The callback pointer load at `0x39de1bb` is `498b7f08`; the sync byte comparison at `0x39f29e7` is `80782900`. JSON pins these instruction bytes and hashes. The queue begins at provider+0x18, map-begin is provider+0x20, and its u64 start index is provider+0x38. Session+0x28 is the provider pointer; provider+0x28 belongs to the deque map structure.

Original provider vtable bases (address point is +16): base ENet `0x53b5048`, EOS `0x53b4280`, Steam `0x53b71b0`. Send slot 0x10 binds respectively `0x39ddfee`, `0x39ada8c`, `0x3a06b58`; Update slot 0x38 binds `0x39de160`, `0x39ad1fe`, `0x39de160`. EOS Update first calls base ENet Update; EOS Send also enqueues local zero-connection packets in the same +0x18 queue. Snapshot actual live identity before interpreting provider offsets.


## Passive message and stream capture

At original EngineUtility.SendMessage entry RSI is the original MemoryOStream pointer. Read its data pointer as u64 at +0x08 and byte length as u64 at +0x10. Original GetBytes `0x3c54386` is `488b4708c3`; original GetOffset `0x3c5438c` is `488b4710c3`. Original MemoryOStream vtable `0x53cc788` address point +16 slot 0x18 binds GetOffset. NetGameCoreMessage constructor calls that slot to determine the byte count it copies. Do not substitute allocated capacity for this length.

At actual FNetSession.HandleMessage entry RSI is FNetMessage. Read its type as i32 at +0x1c, proven by the comparison at `0x39f2d1d` and constructor store at `0x39e0228`. A message of type 3 whose original vptr is `0x53a9ae0` (NetGameCoreMessage table `0x53a9ad0` +16) contains a byte vector at +0x40: data pointer u64 at message+0x40, size **u32** at message+0x50. ByteVector.size `0x2f636c0` is `8b4710c3`; const index `0x33afb40` calls ptr64 conversion `0x2f63d90` (`488b07c3`) and adds the requested index. This proves pointer storage directly without invoking methods in GDB. Check live module/table identity and readable nonnull pointers, bound the dump by the proven length, and retain truncation metadata when limiting an observation. Other application/system message types must not be decoded with this payload layout.

A HandleMessage event alone may therefore be a launch/handshake message rather than a gamecore packet. Capture original stream bytes and type-checked NetGameCoreMessage bytes at their respective real boundaries; these observations establish transport identity without fabricating a message or calling original functions from the debugger.


## Receive entry and original application-message registration

The **true entry** of FNetSession.MessageReceived is `0x39f2062`: receiver RDI, message RSI, boolean DL. Address `0x39f29da` is an interior synchronization branch and must not be used as its entry breakpoint. It first performs actual relay/player checks. For nonsystem messages, local-player lookup and raw player bytes +0x1e/+0x1f can also prevent forwarding: the path with a local player, byte +0x1e nonzero and +0x1f zero rejects ordinary message+8 zero unless message flags contain bit 4. These bytes remain unclassified here. Another live session callback at session+0x18 slot 0x130 controls incoming queuing; observing that original target is preferable to inventing readiness flags.

A missing application creator can drop packets **before MessageReceived**, even when ENet drained its packet queue. Original CreateAndReadMessage calls FNetMessageFactory.CreateMessage at `0x39e0dfa`; null at `0x39e0e05` follows the unknown-message path, optional logging and chunk cleanup without a message for dispatch. Factory CreateMessage `0x39e2046` returns null when a key is absent. A null singleton or zero bucket count is not a valid empty lookup and must not be dereferenced/divided by during inspection.

For passive lookup, read the singleton pointer at original host `0x653d530`. The original lookup uses a bucket-array pointer at factory+8 and u32 bucket count at factory+0x10. Type 3 hashes to `3 % bucket_count`; each node has i32 key at +0, creator pointer at +8, next pointer at +0x10. Bound chain traversal, reject unreadable pointers/cycles and retain unknown or truncated observations. Expected type-3 creator is original `0x33afde0`. This is passive evidence, not a replacement creator registry.

Original application registration occurs during loader initialization: `.init_array` entry `0x535e160` points to grouped initializer `0x2c87e80`, whose tail reaches wrapper `0x2c87e70` and registration constructor `0x33af980`. That constructor obtains or creates the original factory, then invokes original AddCreator with creator `0x33afde0` and type 3. Network.Manager.Initialize is not this registration boundary. Omitting the application initializer or later replacing the factory can therefore leave type 3 absent while system-message setup still works; the current audit establishes this failure mechanism, not its occurrence in a fixture. Do not replay the grouped initializer indiscriminately because it also initializes other application objects. Observe the actual factory key first and restore only an evidenced original lifecycle if it is absent.


## Proven snapshot requirement in the receive gate

Original exported FNetPlayer.GetNeedsSnapshot `0x33e0ff0` reads player+0x1e (`8a471ec3`), establishing that byte's name. SetNeedsSnapshot `0x39e231c` writes SIL to +0x1e; true also records SnapshotReason EDX at +0x2c, while false sets EverHadSnapshot +0x20 to one. Constructor `0x39e2290` sets NeedsSnapshot to one and initializes the adjacent +0x1f byte to zero. Earlier deliberately unclassified descriptions of +0x1e in this document are superseded by these original accessor bytes; +0x1f remains semantically unclassified.

Original FNetSession.IsIgnoreIncomingMessages `0x39f2cb6` returns true exactly when the actual local player exists, NeedsSnapshot is nonzero and adjacent +0x1f is zero. This directly names the receive-blocking state; a freshly constructed player begins in that state. SendSnapshotData writes +0x1f to one at `0x39eee02`, after original session SendMessage at `0x39eedf8`. No exported getter naming +0x1f was found in this bounded audit, so a lifecycle write does not establish a full field name.

Original PlayerAppRegistered `0x39f4294` sets AppRegistered +0x1d. At `0x39f42f5` it calls original SetNeedsSnapshot(false, -1) when either the live session callback slot +0xd0 returns false or the registered player matches the original session host ID (slot +0x158). This is an actual original lifecycle route that can clear the requirement for a local host. It also invokes subsequent callbacks and synchronization handling; static evidence does not authorize calling it with invented arguments or replacing the snapshot lifecycle with a field write. Observe original getters and live identities before selecting any bounded original lifecycle experiment.


## Bounded app-registration ABI and passive rejection observation

The original session slot 0x188 binds PlayerAppRegistered `0x39f4294`. Use `void(session* RDI, int playerID ESI, bool EDX)`; it borrows the actual session and player identity, with no ownership transfer and no meaningful success return. NetAppRegister.Execute `0x39fb1a2` forwards message+0x3c i32 player ID and message+0x40 bool to that slot. In original OnMultiplayerPlayerConnected `0x33d8f80`, the constructor at `0x33d8ff4` receives the actual event player ID and false, then broadcasts. SendIntroduction uses true only for already app-registered players at `0x39e7eee` and `0x39e80aa`. These original booleans distinguish native call contexts; a desired snapshot flag is not evidence for choosing one.

For passive observation, break before `0x39f237e`: RAX is the actual local FNetPlayer pointer, R12 the session pointer, R14 the FNetMessage pointer and R15 preserves the entry boolean. Observe player bytes +0x1d AppRegistered, +0x1e NeedsSnapshot, raw +0x1f, +0x20 EverHadSnapshot and i32 +0x2c SnapshotReason; compare message i32 type +0x1c, raw byte +8 and flags i32 +0x0c. Falling through to `0x39f23a8` establishes the exact drop combination before optional logging and return. The factory hypothesis need not explain a fixture once true MessageReceived has already received type 3 with the original vptr/payload; this static audit does not overwrite runtime observations.

Registration's early original bookkeeping sets AppRegistered and may call SetNeedsSnapshot(false,-1). The live callback +0xd0 is IsGameInProgress `0x33bb6a0`, which reads original application game configuration and compares GameState to `0xcf36f2ee`. Actual host-ID equality clears NeedsSnapshot even when that callback returns true. Local registration may also touch a live voice interface when transport is nonzero and session+0x58 exists. Nonhosts may change matchmaking state. The final live callback +0x78 binds Network.NetSessionCallbacks.PlayerAppRegistered `0x33b9930`: it publishes event 82 with player ID, then event 54 for the ordinary local path or event 70 for a registered remote host. These events may have subscribers; their complete transitive behavior has not been established as policy-free. The bounded functions have no direct native AI entry, which is insufficient to certify every subscriber. Fatal policy guards remain necessary for any separately authorized original lifecycle experiment. No platform/game Begin or session StartGame is required by this function's direct control flow; that does not establish a complete application lifecycle equivalent to its original caller.


## Type 1208 is protocol registration, not app registration

Original type **1208** (`0x4b8`), vptr **0x53b6490**, is NetRegister: constructor `0x39fac9c`, Execute `0x39facec`. NetAppRegister instead uses type **1209** (`0x4b9`), vptr **0x53b64d0**, constructor `0x39fb05e`, Execute `0x39fb1a2`. An observed HandleMessage of type 1208 therefore does not establish that PlayerAppRegistered has run.

NetRegister.Execute sets protocol registration byte player+0x1c and invokes live callback slot +0x68, original Network.NetSessionCallbacks.PlayerRegistered `0x33b9820`. That callback has a significant native branch: with GameCore active and automation inactive, a host requests original Context action hash `0x23cabda3` for the actual player at `0x33b98ef` through Context slot +0x1f8. It does not publish the ordinary connected event on that path. With the other original condition it may change original application configuration and publish event 55 at `0x33b98ad`; the original OnMultiplayerPlayerConnected handler then broadcasts NetAppRegister(false).

Consequently a protocol registration packet processed after the core is already active can produce a second gamecore action stream without completing ordinary app registration. This is a concrete lifecycle ordering lead, not proof of a fixture's branch without passive observations. Do not replay PlayerRegistered as a passive bookkeeping method: its original active-host path requests a gameplay action. A direct bounded original PlayerAppRegistered experiment has separate callback/event risks described above and still requires evidence of the actual local snapshot gate.
