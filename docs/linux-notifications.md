# Linux notification wire component

`c6_core::notifications` implements the pinned Linux XP2 `PlayerNotifications` v1 and `Notification::Instance` v5 writer formats. This is bounded typed transport. Complete gameplay state, action execution and turns remain unsupported. The target is `libGameCore_XP2.so` SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`; historical Windows work provides no Linux acceptance.

The container writer at `0x2d305c0` emits i32 version, i32 owner, u32 next ID, i32 record count, then ordered factory type hashes and each instance's virtual writer. All 39 audited notification vtables inherit `Instance::Write` at `0x2d2e0b0`. The codec preserves sequence, duplicate IDs, independent factory/instance type hashes and every serialized scalar. The instance writer emits:

| Field | Native offset / framing |
| --- | --- |
| Version | i32, 5 |
| Notification ID | +8; i16 owner, i16 component type, u32 ID |
| Type, severity | +10/+14; u32/i32 |
| Message, summary | +20/+a0; localized string |
| Location, component | +120/+128; two i32 / ComponentID |
| Scalars | +188/+18c; i32 each |
| Flags | +198/+199/+19a; bool each |
| Scalar | +190; i32 |
| Flags | +19b/+19c; bool each |
| Parameters | +140; actual TypedVariantVectorMap Serialize(options=1) |
| Flags | +19d/+19e; bool each |

The text labels were corrected from target DWARF and compiled GetMessageA/GetSummary accessors: +20 is message, +a0 is summary. Earlier derived metadata reversed these names; serialized bytes and original raw captures are unchanged. This correction does not change the already-dismissed cleanup classification, which uses +198 rather than +19c. See the [lifecycle contract](linux-notification-lifecycle.md) for the newly classified live fields and flags.

Only +198 (`IsDismissed`, getter `0x2722dd0`) and +19e (`HasBeenActivated`, getter `0x2d313c0`) receive semantic names. Unknown fields retain offset names. `IsLocked` at `0x2ab70e0` reads signed live +194 > 0; this field is absent from the writer. A decoded record cannot establish its lock state or reconstruct complete live notification state.

The localized string writer at `0x2d4e3c0` emits u32 `ffffffff`, u8 version 1, u32 byte length, base bytes, u8 positional argument count, arguments, u8 named argument count, then named bindings. All 42 strings in the 21 captured records have both argument counts zero. This component rejects nonzero counts explicitly. It retains arbitrary base bytes without UTF-8 conversion or terminators. Native argument handling includes int/long/float/double/nested-string and indexed name branches; these remain unsupported rather than skipped or stored as opaque bytes.

Parameters reuse the production typed variant vector-map codec with a shared aggregate budget. Caller limits bound input/output bytes, record count, aggregate strings/records/variant items and nesting. Negative/oversized counts, invalid bool bytes, unsupported versions/branches, truncation and trailing bytes fail explicitly. No mutation is performed while decoding. Encoding validates aggregate resources and supported branches before returning output; it does not assign IDs or normalize fields.

`Cleanup` at `0x2d311e0` iterates in order, calls `IsDismissed`, invokes virtual +20 on dismissed instances, and erases their nodes. `Activate(bool)` at `0x2d2dd80` unconditionally sets +19e and sends general event `025f6cfd` with ComponentID value key `eab69e5a` and incoming bool key `78a2f946`. Neither is exposed as complete gameplay behavior: destruction and event subscriber effects remain separate research requirements. Selection must be proven from general end-turn blocking rules, never fixture IDs or a presumed presentation-only exclusion.

The frozen conditional original SkipTurn fixture has 12 records before and 9 afterward. Three already-dismissed records are removed (1,064 bytes), and one survivor changes HasBeenActivated. These are command aftermath effects. The codec compares every original record and entire owner section byte-for-byte and rejects every truncated prefix plus version, count, bool, trailing-byte and resource corruptions. See [audit report](../evidence/linux/notifications-codec.json). Codec timing excludes original execution, native Load, subscribers and all gameplay.

Reproduce without executing original code:

```sh
python3 tools/linux/notifications_codec_audit.py \
  --module /private/path/libGameCore_XP2.so \
  --before ../c6-ghidra/evidence/linux-core-component/native-execute-3/before.bin \
  --after ../c6-ghidra/evidence/linux-core-component/native-execute-3/after.bin \
  --output target/linux/notifications-codec.json
cargo test -p c6-core notifications
```

Private frozen writer/getter/cleanup/activation disassemblies remain under `../c6-ghidra/evidence/linux-skip-turn-state/execute-3-contracts`; no proprietary instructions or snapshot bytes are included publicly. Attribution and reference limitations remain in [reference.md](reference.md). The raw Save→Load 89,198-byte failure remains unclassified and unmasked. Successful wire roundtrips do not establish branch restoration, full-host admission, complete gameplay RNG, complete notification state or Rust SkipTurn acceptance.
