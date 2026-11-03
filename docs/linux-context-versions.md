# Linux Context v4/v5 prefix framing

The bounded Rust Core prefix now accepts Context versions 4 and 5. This is wire decoding only. Complete gameplay state, native initialization, action execution and turns remain unimplemented. Inner serializer versions remain independently strict; accepting Context4 does not enable older Game, Player or Map versions.

The pinned Linux module is `libGameCore_XP2.so`, SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. Its `Context::Instance::Read` at `0x290bca0` reads a signed version, rejects values above 5, constructs a local TypedVariantMap, and reads that map only for version 5. It then calls virtual `ReadMetaData` in both versions. Version 4 and version 5 both call `IndexSerializer::LoadVersion` before reading Game; neither reads the old version-1 configuration branch. Both later read Tutorial and Achievements. The bounded prefix stops after Map and does not implement those later callbacks or records.

| Outer version | Immediately after version | Rust representation |
| --- | --- | --- |
| 4 | IndexSerializer version i32 | `context_version = 4`, constructed empty `metadata` |
| 5 | TypedVariantMap, then IndexSerializer version i32 | `context_version = 5`, decoded `metadata` |

`context_version` is retained in both CorePrefix and OwnedCore. This distinguishes absent v4 metadata bytes from an encoded empty v5 map. No producer identity or metadata compatibility value is inferred from the empty map. Version 4 spends four bytes for its version and no metadata item/allocation credit. All downstream reads use the same aggregate budgets, strict booleans and borrowed tail policy as version 5. Other outer versions, including older branches visible in native Read, remain unsupported.

The matching host's TypedVariantMap constructor (`0x395f070`) builds an empty typed map through the typed rbtree constructor (`0x39626b0`) and reset (`0x3962700`). Reset installs the empty header links, null root and zero size. The audit fingerprints this chain. The Context constructor (`0x290c000`) initializes its runtime members and creates Achievements; the local metadata map in Read is separately constructed and does not inherit producer metadata from Context. No native Context object is reconstructed by the Rust prefix.

Native `ReadMetaData` (`0x290bf00`) reads unsigned metadata keys `fd1bbf76` and `4e33fe49`, clears a GUID and optionally converts key `f4e540d8`, then consults Context getters. The GUID absence path retains its explicitly cleared value. Missing numeric keys use the host's shared default TypedVariant through `GetVariant` (`0x395f480`). This audit fingerprints the conversion and constructors but does not close the shared sentinel's global initialization or claim a native compatibility result. Rust performs no compatibility callback, fabricated numeric defaults or initialization from this metadata. Existing runtime fields not covered by this prefix remain unknown; no zero-filled gameplay state is supplied.

The immutable original raw Read-input `a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa` (76,556,938 bytes) has Context4, Index1, Game19 at fixed byte-zero offsets 0, 4 and 8. Consequently this Context-only change advances the expected Rust refusal from Context5/4 to Game22/19, still **before Map**. The audit verifies those exact bytes after hashing the whole input; it does not scan for another version or transfer offsets from SavedOutput. The separately retained first-City diagnostic must establish any actual composed-parser outcome. No City flags or full raw load are claimed here. The recorded 89,198-byte raw Save→Load difference and all unknown bytes remain untouched and unmasked.

Reproduce the source-only evidence and focused framing tests:

```sh
python3 tools/linux/context_version_audit.py --output /tmp/context-versions.json
cargo test -p c6-core core_stream::tests --offline
```

The audit checks the exact game and host hashes, method bytes, matching Context DWARF output and original raw header. It never executes original code. Synthetic tests cover both version paths, every v4 truncation, unsupported outer/inner versions, exact aggregate budget caps and tail borrowing. A native differential oracle for this newly accepted framing remains unavailable; these are bounded source-backed checks, not native execution parity or whole-state acceptance.
