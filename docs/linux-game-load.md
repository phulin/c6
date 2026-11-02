# Original Linux Core Load/Save boundary

The c6 Rust implementation still covers isolated primitives. The diagnostic in
[native/linux/game_load.c](../native/linux/game_load.c) prepares a lower native
XP2 Core stream boundary; it does not establish full state/action/turn parity or
an action-ready initialized gameplay fixture.

The genuine AppModdingFramework owns the active context. The diagnostic requires
original XP2 content activation, original restored benchmark configuration and a
genuine native session with an observed local peer before calling StartupGame.
The host loader's actual stage 9 calls StartupGame before Core Load; Game
serialization itself initializes the local player, so attaching after Load would
be too late. See [the saved-load contract](linux-save-lifecycle.md).

The adapter resolves the active context's actual module, requires its XP2 leaf
name, observes original IsInitialied, and uses the existing original
MemoryIStream/MemoryOStream constructors through
[context_stream.c](../native/linux/context_stream.c). The launcher must retain
its exact host, XP2, dependency, source-input and configuration identity gates.
The accepted extracted benchmark stream is 76,556,938 bytes, SHA-256
`a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa`.
Its parent complete benchmark save has SHA-256
`a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8`.
Neither proprietary input nor generated native save bytes belong in this public
repository.

A returning attempt reports the native signed Result and exact stream offset,
then, only after native Load returns OK, writes two consecutive original Core
Saves to exclusive new private files. It calls original ShutdownGame before the
caller leaves the native session and tears down the framework. A timeout, fault,
nonzero Load result, incomplete consumption or failed cleanup is retained as a
failed boundary, never a passing comparison.

[game_load_report.py](../tools/linux/game_load_report.py) extracts the native
results and hashes private outputs without copying their bytes into public
evidence. Consecutive Save equality would establish only that observed serialized
output's stability. Core Save excludes live queues, caches, host/session state,
event/reporting subscribers and asynchronous work. It is neither a complete Civ
VI save container nor proof that a native branch can be fully restored.

The current UpdateContent fixture uses a genuine constructor-state
LoadGameViewState callback with request None and censorship false. It does not
run saved RequestLoad/Begin. Host cache initialization, PostInitialize, visual
state restoration, pending user settings and original active-user selection
remain separate prerequisites. No action or turn may count as accepted gameplay
parity from this lower serialization diagnostic alone.

Two independent original-native attempts now pass this lower boundary:
[attempt 1](../evidence/linux/game-load-summary-attempt-1.json) and
[attempt 2](../evidence/linux/game-load-summary-attempt-2.json). Both use exact
configuration identity
`446b6e6afc2542d244f4335ef513392e3e75f4fafefdb0d0e724afcaac8f202c`.
Original Load returns OK and consumes all 76,556,938 bytes. All four original
Save outputs are 80,657,462 bytes, SHA-256
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`.
Original ShutdownGame, native session LeaveGame and full framework/provider
cleanup return successfully in each process. These results establish reproducible
lower Core serialization under the identified diagnostic profile. The source
stream differs from the reserialized output; no byte-preserving container or
full host-load equivalence is inferred.

The exact captured-output input now has a separate
[manifest gate](../tools/linux/game_load_captured_gate.json), tied to the first
native report, configuration and original output hash. The existing fixed
benchmark Load API is unchanged. A new sized entry checks actual file length
before constructing the same original MemoryIStream; its caller must gate exact
SHA-256, length and profile.

Both fresh-process captured-input runs return OK, consume all 80,657,462 bytes,
write two identical consecutive Saves and complete cleanup:
[roundtrip 1](../evidence/linux/game-load-summary-roundtrip-1.json) and
[roundtrip 2](../evidence/linux/game-load-summary-roundtrip-2.json). Their separate
configuration identity is
`b115764c2ed13c0821dc3620e04bbe083873146dd55723a3f131a37d773dabfa`.
All four resulting Saves have SHA-256
`85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8`.

**Byte-stable Save→Load→Save fails in this diagnostic profile.** The output has
the same length but differs from the captured input in 89,198 bytes.
[The minimized difference report](../evidence/linux/game-load-roundtrip-differences-1.json)
records ranges; actual bytes and complete spans remain private. The repeated
change is reproducible, but its field semantics and cause remain unproven. Do not
normalize or ignore these differences, or treat successful native Results as
proof of a branch-restorable serialized state.
