# Original ScriptSystem and Lua lifecycle boundary

This is an executed original-code lifecycle experiment, not an initialized Civ VI game. `evidence/bootstrap/script-system.json` records both a successful genuine VM/thread lifecycle and the failed full ScriptSystem state request. No replacement VM or synthetic application fields are supplied.

The EXE SHA-256 is `e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63`; HavokScript is `9fb4e34afa6666aafb519d6edb9c1e4e9add9cce2cecaa3b1e5bb8f08b168532`. The standalone runner verifies the complete existing bootstrap asset allowlist before execution. The helper requires the original EXE mapped at preferred base `0x140000000`, genuine CRT/kernel imports already bound, and exact provider gates enforced by its caller. It binds only the 71 genuine HavokScript imports in addition to the shared import binder. It never calls EXE entry, TLS, or broad global initializers.

## Observed original lifecycle

All addresses below are EXE RVAs in that fingerprinted Windows x64 image. `bootstrap_original_script_system` executes these unchanged functions:

| Phase | Native contract |
| --- | --- |
| Shared Lua lock | Constructor `0xf607c0` receives native lock storage at `0x3552fa0`. It delegates to the actual Win32 critical-section provider. |
| Lua manager | Creator `0x4d4c70` allocates `0xf8` bytes, calls constructor `0x971a40` with original allocator `0x971cf0`, parameters `0x6e` and `5`, installs vtable `0xfe9cf8`, and stores the real manager in global `0x2d08f88`. Manager `+0x10` is an actual nonnull Havok state. |
| ScriptSystem | Lazy creator `0x1c9890` allocates `0x20` bytes and calls constructor `0x1615e0`. The constructor installs vtable `0xfc0618`, builds its native list sentinel, and constructs the native TypedVariantManager. Getter `0x1c9880` returns the exact resulting pointer from `0x22d8140`. |
| Raw Lua thread | Manager virtual `+8` targets `0x971e60` and creates a genuine thread state distinct from its main state. Virtual `+0x18`, `0x971ef0`, releases that state. |
| Cleanup | The successful child calls ScriptSystem deleting destructor `0x161810(self,1)`, manager deleting destructor `0x4d4c30(self,1)`, then lock destructor `0xf607e0`. Every call returns. |

The lifecycle child checks actual exported Havok `lua_gettop`, `lua_pushinteger`, `lua_tointeger`, and `lua_settop`: initial stack zero, push and recover integer 17431 with stack one, then restore stack zero. It releases the original thread and completes native cleanup with exit zero. These checks establish that the provider creates a usable native Lua state; they do not validate game libraries or script semantics.

Cleanup is process-lifetime only. The native deleting destructors do not clear their EXE singleton globals; the child terminates immediately after cleanup and does not dereference or reuse those stale singleton pointers. Reinitialization and host-wide teardown remain unverified. The helper rejects pre-existing manager or ScriptSystem singletons.

## Failed full state creation

ScriptSystem virtual `+0` is `0x161a90(self, const char *name, bool)`. The full-state child requests the observed narrow `GameCore_Tuner` name with boolean one. This executes the original game-library registration path and fails before returning a state. The native exception is `0xc0000005` at EXE `0x64660`, with RCX zero; its return address is `0x2a3509`.

The exact caller is the original GameInfo registration callback `0x2a3440`. At `0x2a34fd` it loads the actual application singleton `0x12c2c88`, then calls `0x64660` at `0x2a3504`. That accessor dereferences application `+0x54d8` and delegates to the gameplay database wrapper accessor `0xf02860`. The actual application singleton is absent in this bounded process. Existing standalone DatabaseDLL connections do not establish a genuine Civ6App or its native database-wrapper lifecycle. Installing synthetic partial App fields would not resolve that evidence gap.

Consequently original StartupGame's final Tuner initialization remains unsupported. No full ScriptSystem state creation, Tuner exposure, initialized game, save loading, action, or turn success is claimed by this checkpoint. The separate GameCore script utility bridge also requires its own actual provider/context validation.

## Reproduction

Run `python3 tools/analysis/bootstrap_script_system.py` after coordinating exclusive use of the shared Wine prefix. It builds the owned child with MinGW `-Wall -Wextra -Werror`, fingerprints inputs and executable, and records sequential lifecycle and failing full-state cases. `python3 tools/analysis/bootstrap_script_system_evidence.py` verifies the original EXE hash and writes targeted instruction listings and vtable observations only to private `../c6-ghidra/evidence/bootstrap/script-system/`. No proprietary binary bytes or disassembly are stored in this public repository.
