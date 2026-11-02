# Independent Linux native ABI and lifecycle review

The bounded critical calling contracts and adapter refusal checks pass this
review. Complete exact Civ VI state/action/turn behavior remains incomplete.
Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
remains the acceptance target; Base lifetime observations stay separate.

The reviewed host SHA-256 is
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Bounded DWARF searches found no definitions for these host methods. Proof comes
from exact bytes, symbols and original callers, with decompiler output used as
a lead. Private bounded disassembly is under
`../c6-ghidra/evidence/linux-native-abi-review/`; public hashes, findings and
source identities are in [native-abi-review.json](../evidence/linux/native-abi-review.json).

| Boundary | Exact observed contract |
| --- | --- |
| ConfigureContent | Receiver in RDI; eight-byte RAX result; status at byte4; success bit0; five defined flags, mask `0x1f`. |
| UpdateContent | Eight-byte RAX result, three defined flags; original nontrivial callback object required. Not called by this adapter. |
| Civ6App::SetupNetwork | `_ZN7Civ6App12SetupNetworkEv`, App receiver only, bool AL. |
| Network::Manager::HostGame | Integer EAX; success enum1. Concrete session/local host/player postconditions remain required. |
| GameCore::GetContext | Caller supplies an output object in RDI. Cannot be called as `void *getter(void)`. |
| Network GetSession / FNetSession GetPlayer | Raw pointer RAX; receiver and player ID remain explicit. |

The ConfigureContent and UpdateContent epilogues load eight bytes into RAX at
`0x3380908` / `0x3381f30`. The original UpdateContent caller tests Configure's
AL bit0 at `0x33818b7`. Configure's constructor at `0x3386170` clears five bits;
Update's at `0x3386200` clears three. Remaining flag-byte bits and other padding
are not normalized. Nonzero failure flags cannot be interpreted as boolean
success. Review found diagnostic mask7 dropping defined bits3/4; the owner
corrected it to31. Flags8 and16 now remain visible and their failures are refused.

Original Startup allocates `0x218` bytes for AppFiraxisLiveClient, calls its real
constructor, and constructs the native shared_ptr with receiver/client/zero
arguments. The fixture matches that allocation, constructor and initially empty
owner at `0x6498e10`, using original Shutdown. Both inspected native allocation
overloads have equivalent rounding/alignment behavior. App database object sizes
`0x38` and owner offsets `0x6320` / `0x6328` match original SetupDatabase. No
allocation ABI defect explaining the separate constructor heap abort was found.

Network initialization now refuses an App pointer differing from the actual
published App before writing argument fields. Host/leave refuse a Manager
replaced since fixture initialization. Three synthetic guards verify these
refusals before any sentinel session dereference. Original SetupNetwork reads
App argc/argv fields and takes no additional bool argument.

InitInstance alone is insufficient: original bytes publish the singleton and
return AL1 after construction. A preserved run returned1 with NULL context and
was correctly refused113. The framework branch additionally checks the concrete
Context vtable, canonical module path/hash against gated Base, and actual original
App configuration/gameplay connection presence. The new
[runtime observation](../evidence/linux/runtime-framework-base-observed-lifecycle.json)
passes these checks and native teardown. Nonnull connections do not establish
row population or native cache semantics.

The accepted Base baseline verifies borrowed context lifetime and original
framework DestroyInstance, Controller Shutdown, client Shutdown and host-services
cleanup. Borrowed contexts never cross into DllDestroyGameContext. Its Modding DB
is unavailable, so it cannot establish discovery/activation readiness. With the
physical Modding.sql path available, the minimal constructor fixture times out;
a localization-enabled private probe hit an unresolved original heap abort.
ConfigureContent's preserved status1/bit0-clear result remains failure. Full
UpdateContent/XP2 activation and gameplay remain unvalidated.

Run the focused verifier with:

```sh
python3 tools/linux/native_abi_review.py --output /tmp/c6-native-abi-review-check.json
```

Twelve exact-host byte anchors, seven synthetic aggregate vectors and three
receiver refusals pass. Strict shared compilation of mod/framework/network/host
adapters passes with `-Wall -Wextra -Werror`. This verifier executes no original
game code. Synthetic tests, static contracts and separately preserved native
observations do not establish full state/action/turn parity.
