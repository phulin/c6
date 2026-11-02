# Native bootstrap host exports

The bootstrap currently exercises the original XP2 DLL's loader and context
factory boundary. These host-export findings do not establish initialized game
behavior or state/action/turn equivalence.

`tools/analysis/host_exports.py` inventories host `EXP_*` exports and DLL string
occurrences from locally supplied PE files. It gates the source executable by
SHA-256 `e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63`.
Reproduce the public metadata with:

```sh
python3 tools/analysis/host_exports.py ../c6-assets --output evidence/bootstrap/exports.json
```

The output contains symbol names, identities and addresses, without proprietary
image contents. Raw disassembly is private in
`../c6-ghidra/evidence/bootstrap/exports/{exe,xp2}-disassembly.txt`.
String occurrences alone are leads; the XP2 request wrappers listed in the
metadata were also checked in disassembly for the exact DLL hash
`324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`.

## Supported original getter

The Windows x64 export `void *EXP_GetEmptyBasicString(void)` at executable RVA
`0x9aee10` returns the address of executable RVA `0x2d227d8`. Its complete body
computes that address and returns; there are no calls, imports, TLS accesses,
initialization guards or data loads. This closed body can be delegated through
`GetProcAddress` on the hash-gated executable mapped with
`DONT_RESOLVE_DLL_REFERENCES`. The bootstrap executable must occupy a different
image base. This is a narrow exception to the general prohibition on calling
code in an uninitialized executable image.

The result is the shared empty character buffer, not a string object or an
owned allocation. Its 32-bit capacity and length headers are at offsets `-8`
and `-4`. Both headers and the terminator lie within the PE loader's zero-filled
part of `.data`, so they do not need the original executable's CRT startup.
The inventory command verifies both the closed getter shape and the zero-fill
range. Original executable string destructor RVA `0x9adf70` compares pointer
identity with this sentinel and skips freeing it. Non-sentinel strings free
the allocation at character pointer minus eight. Assignment code at RVA
`0x9ae3c0` uses the same identity and header convention. This evidence explains
why a separately allocated empty string or a null return would be wrong.

Keep the executable image mapped until the DLL and all objects holding its
sentinel have been destroyed. Do not free, modify, copy as an owned allocation,
or substitute another address for the sentinel. No separate `EXP_*` wide-string
getter or allocator export appears in this source executable.

## Remaining XP2 requests

The exact XP2 FinalRelease DLL dynamically looks up these additional exports:

| Export | XP2 requesting wrapper RVA | Audited behavior |
| --- | --- | --- |
| `EXP_GetLoggingOptions` | `0x991880` | On missing export, native code writes 64-bit value 1 to each of its two output pointers. |
| `EXP_GetLogEventCallback` | `0x9918e0` | Getter result is called as a callback with the wrapper's three arguments; missing export is unchecked. |
| `EXP_JobManager_CreateJobList` | `0x992160` | Unchecked indirect tail call. |
| `EXP_JobManager_DestroyJobSet` | `0x992190` | Null job set is a native no-op; otherwise unchecked indirect call. |
| `EXP_JobManager_FreeJobList` | `0x9921d0` | Unchecked indirect tail call. |
| `EXP_JobManager_PushJobOntoList` | `0x992210` | Four-argument forwarding, including a copied 16-byte fourth argument. |
| `EXP_JobManager_SpawnList` | `0x992280` | Unchecked indirect tail call. |
| `EXP_JobManager_Wait` | `0x9922c0` | Two-argument forwarding, including a copied 16-byte second argument. |
| `EXP_GetTempHeap` | `0x9934b0` | Calls getter and caches object pointer without checking export presence. |
| `EXP_GetTelemetrySessionHash` | `0x9947a0` | Checks missing export and returns cached value, initially zero. |
| `EXP_GetTelemetryEventSystem` | `0x9947e0` | Checks export presence but subsequently accesses the event object; absence does not make the complete operation safe. |

The job, logging, heap and telemetry services remain unsupported. Returning a
zero-filled original object does not initialize its methods, synchronization,
ownership or service state. In particular, the original temp-heap and telemetry
getters also compute static addresses, but that alone does not establish usable
objects. Do not forward them into an uninitialized executable or invent dummy
objects to continue initialization. The verified native missing-export branches
above may execute; reaching an unsupported unchecked call remains a bootstrap
failure, not a parity success.

The executable exports 26 `EXP_*` names in total. Other names belong to UI,
event systems, logging setters and jobs and are not requests found in this XP2
FinalRelease image. The release rather than FinalRelease DLLs also contain an
`EXP_GetAssertState` name absent from this executable's exports, reinforcing
that these inventories must retain their individual binary identities.
