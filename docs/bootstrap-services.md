# Windows XP2 host service boundary

This is a bounded original-code lifecycle experiment, not an initialized Civ VI game or an exact action/turn library. The service fixture supplies a real interface receiver, records event parameter declarations, and optionally opens real empty named-memory databases through the supplied DatabaseDLL. It provides a bounded stderr logging sink and an empty-only archive registry. It does not implement a script VM, event delivery, rules loading, networking or game semantics. Unsupported virtual calls terminate the process with status 86 and identify the slot.

The target GameCore SHA-256 is `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`. The supplied `CivilizationVI.exe` is `e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63`, and `DatabaseDLL_FinalRelease.dll` is `e6577e873f946224df65c02f6c4bc3b385607e2a82d957b0fd613e01cbe78b0c`. The external native runner must gate every executed module; the fixture's comparison of the target GUID constant is only an additional sanity check.

`python3 tools/analysis/bootstrap_services_evidence.py` checks all three complete module hashes before extracting targeted instructions and constants to private `../c6-ghidra/evidence/bootstrap/services/`. Fresh Ghidra decompilation of SetEngineUtility, Events::Register and Scripting::Manager Initialize/Uninitialize is separately saved under `../c6-ghidra/evidence/bootstrap/services-*.json`. This public document summarizes those observations; it contains no proprietary implementation listing. Names originate from cru121's [Civ6-GameCore-reference](https://github.com/cru121/civ6-gamecore-reference/tree/5af389445118bed2bdebf7a8a42b219c8fba5175), CC BY 4.0, and were checked against these supplied module instructions.

## Exact observed calls

Offsets are vtable byte offsets, not slot indexes. All addresses are RVAs in the named Windows x64 module. These signatures describe observed register use; they are not Linux layouts.

| Interface | Offset | Observed contract |
| --- | --- | --- |
| IContext1 | `+0` | QueryInterface(self, pointer to 16 GUID bytes) returns interface pointer or null; GameCore RVA `0x5907a0`. |
| IContext1 | `+0x28` | Initialize(self), boolean in AL; wrapper `0x5905f0` dispatches underlying object `self+8` vtable `+8`. |
| IContext1 | `+0x30` | Uninitialize(self), reference slot label; target wrapper and full teardown require separate checks before lifecycle use. |
| IContext1 | `+0x38` | IsInitialized(self), reference slot label. |
| IContext1 | `+0x60` | SetEngineInterface(self, engine); wrapper `0x591750` forwards engine directly to `0x45050`. |
| IContext1 | `+0x68` | GetScriptSystemUtility(self); wrapper `0x590440`. This is distinct from engine GetScriptSystem. |
| Engine utility | `+0` | QueryInterface(self, GUID pointer). Required raw GUID `c299ce27a878d54191a2567c083e47af`, constant at GameCore `0x9bfba8`. |
| Engine utility | `+8` | Destroy(self), invoked when replacing an installed interface. Fixture has process-lifetime borrowed storage. |
| Engine utility | `+0x10` | GetScriptSystem(self) returns pointer; global `0xb8aa88`. Fixture explicitly reports null/unavailable. |
| Engine utility | `+0x18` | GetLoggingManager(self) returns pointer; global `0xb8aa90`. Fixture returns the bounded stderr logging manager. |
| Engine utility | `+0x98` | RegisterEventParameter(self, u32 event, u32 parameter, u32 usage). Sixteen declarations are made by `0x1a1400` during interface installation. Fixture retains all three fields in order. |
| Engine utility | `+0x140` | GetConfigurationDatabaseUri(self) -> narrow char pointer, first call during Initialize. |
| Engine utility | `+0x138` | GetGameplayDatabaseUri(self) -> narrow char pointer, second call during Initialize. |
| Database connection | `+8` | Destroy(self), called when original context replaces/releases connection. |
| Database connection | `+0x20` | Execute(self, const char *sql, int byte_length) -> boolean AL; native RVA `0x1ae0`. |
| Database connection | `+0x28` | ExecuteMultiple(self, const char *sql, int byte_length) -> boolean AL; native RVA `0x1bf0`. Negative length computes strlen. |

The context's first logging-manager call is conditional on a non-null manager, but logging is required later: the original database logger constructor (`0x5e34e0`) unconditionally calls manager `+0x40` RegisterLog and `+0x28` EnableLog. A real empty-database probe with null manager faulted at `0x5e3524`; the fixture now supplies a real stderr sink. Manager `+0x50` GetLogFile returns that sink, whose `+0x10` consumes the original narrow variadic format string. Log-file registration uses a wide filename, id, flags and fifth null argument. Logging output is diagnostic and does not provide game semantics. `Scripting::Manager::Initialize` (`0x5fda50`) calls Uninitialize and returns true. Uninitialize dereferences engine script only while iterating existing script instances. An empty newly constructed manager can therefore pass this narrow path with an unavailable script system; creating/running scripts remains unsupported. A lifecycle Initialize result alone does not establish usable gameplay scripting.

## Real database fixture

The original host's configuration URI generator (`0xbcb60`, literal `0xfb30c8`) uses `file:configuration-%i?mode=memory&cache=shared`; gameplay (`0xbf980`, literal `0xfb35b0`) uses `file:gameplay-%i?mode=memory&cache=shared`. Each increments its own byte generation counter. The original URI getters return pointers into host string objects; they do not supply on-disk paths. Existing host database objects and a live keepalive connection are separate requirements.

The supplied DatabaseDLL exports `Database::Initialize()` at `0x1120` and `Database::CreateConnection(const char*, int, IDatabaseLogger*)` at `0x1020`. Original host configuration CreateConnection (`0xbc880`) passes flags `0x46` to create its writable connection. Original Context Initialize (`0x45d90`) passes `0x41` when creating each context-owned connection. The DatabaseDLL passes those flags unchanged to its internal sqlite3_open_v2 implementation (`0x13e30`). These are respectively READWRITE|CREATE|URI and READONLY|URI.

`bootstrap_service_fixture_prepare_empty_databases` calls the real DatabaseDLL initializer and creates two real writable connections using generation-zero names. It retains them until process exit so context read connections share their databases. Its return is checked; failures cannot become fabricated connection pointers. No rules or configuration rows are invented. This helper must be invoked explicitly; without it a required URI getter terminates with a missing-fixture trace. The fixture deliberately provides no unproven cross-cycle database shutdown/reinitialization contract.

Original host writable configuration setup also registers `Make_Hash` with a native scalar callback (constructor `0xbd050`, registration virtual offset `+0xf0`) and executes `PRAGMA foreign_keys = ON` and `PRAGMA recursive_triggers = ON`. The bounded empty database helper does not reproduce that schema/data bootstrap by itself. `bootstrap_service_fixture_execute_sql` exposes original ExecuteMultiple for further source-gated SQL experiments; successful SQL does not establish complete XML import, enabled DLC/mod ordering or initialized game configuration. See the asset inventory and schema evidence for remaining data gates.

## Validation boundary

The service C translation unit compiles with MinGW x64 `-std=c11 -Wall -Wextra -Werror`. Static evidence extraction verifies all input hashes and observed GUID/URI constants. Runtime loader evidence is maintained separately by the native bootstrap runner. The runner now records actual original Initialize returning true and the original IsInitialized accessor transitioning false to true, with both empty and original SQL schema fixtures. A null-logging failure and a raw SQL BOM rejection are retained as negative evidence. Native runtime evidence remains the authority for current lifecycle completion; successful context initialization remains separate from StartupGame, InitializeDatabase, full rules population, legal actions and complete turns.


## Empty-only archive cleanup

Context Uninitialize (`0x470b0`) invokes archive type removal (`0x46df0`), which obtains engine `+0x110` GetAutoArchiveFactory and calls factory `+8` with an owned three-pointer AutoVariableString. The supplied host factory vtable (`0x10f0888`) maps registration to `0xdbaaf0`, removal to `0xdbae40` and destruction to `0xdba660`; constructor `0xdba4f0` creates the empty registry. The original removal searches for the key, erases only if present, and then destroys its by-value string argument. Calling this implementation from an uninitialized mapped EXE is unsafe because string deallocation reaches an unresolved EXE import.

The fixture therefore implements only removal from a permanently empty registry. Registration remains unsupported and terminates, preserving the empty invariant. Removal has no registry mutation and releases the consumed name with the actual GameCore aligned deallocator (`0x9908f0`), paired with the original temporary constructor (`0x14930` -> `0x994970` -> allocator `0x990900`). This is a bounded service implementation, not the original host archive subsystem or save support. StartupGame will need real archive registration and additional semantics; these remain explicit unsupported operations.
