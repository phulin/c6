# Original benchmark save loading boundary

The supplied benchmark save is a genuine `CIV6` container. It is separate from
c6's Rust snapshot transport. The work below recovers metadata and a pinned
compressed GameCore stream candidate; it does **not** initialize a game or prove
full state, action, or turn equivalence. The existing implementation still covers
isolated primitives.

## Reproduce

Run from the repository root, with the original private assets in the sibling
asset tree:

```sh
python3 tools/saves/recover_contract.py
python3 tools/saves/inspect_save.py \
  ../c6-assets/Base/Assets/Benchmark/Saves/XP2Benchmark.Civ6Save \
  --output ../c6-ghidra/evidence/saves/xp2-container.json \
  --extract-xp2-stream ../c6-ghidra/evidence/saves/xp2-gamecore-stream.bin
```

The contract probe gates both original modules by SHA-256, verifies the actual
Windows export vtable Load pointer, and checks objdump's first instruction
address. Verbose disassembly, decoded metadata, and extracted bytes belong in the
private research tree. Do not commit save contents or decompressed gameplay data
to this repository. The compact public result is
[`xp2-benchmark.json`](../evidence/saves/xp2-benchmark.json).

The inspector rejects unknown save hashes, truncation, unsupported payload flags,
excessive nesting/counts, changed configuration block counts, and changed chunk
boundaries. This is deliberately a bounded benchmark probe, not a general save
format implementation. Metadata inspection also succeeds on the two other
fingerprinted supplied benchmark saves; only XP2 stream extraction is established.

## Original Windows call contract

All RVAs refer to XP2 FinalRelease SHA-256
`324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`.
Host RVAs refer to CivilizationVI.exe SHA-256
`e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63`.

| Boundary | Verified contract |
| --- | --- |
| Exported context vtable `0xa101e8 + 0x170` | Pointer to Load `0x5906e0` |
| `Context::Load` `0x5906e0` | RCX export object, RDX `Serialization::IStream*`; preserves integer result; disables/restores a flag via `0x7bbf0`; calls underlying context at `export_this+8`, virtual slot `+0x70` |
| `Context::Read` `0x463b0` | Reads signed i32 serialization version; versions >5 return 2. Sets/clears loading flag at `this+0x1a70`. Version 5 reads a metadata map before gameplay; version 4 skips this metadata map |
| Stream raw read `+0x30` | RCX stream; RDX pointer to five qwords `[-1, size, 16, 0, 0]`; R8 destination; R9 size; followed by completion slot `+0x40` |
| Typed-map read `0x99e750` | Delegates to map virtual slot `+0x1e8`, with stream passed as RDX and zero R8; the map's actual vtable `0xabf080` resolves that slot to `0x9a9a40` |
| `Context::ReadMetaData` `0x46530` | Fetches hashed keys `fd1bbf76`, `4e33fe49`, `f4e540d8`; compares saved serialization/DLL identifiers with virtual results and returns 2 on incompatibility. Do not fabricate these values |
| Context gameplay read | Calls `0x751b60` with RCX stream and RDX game pointer from `this+0x60`; then `0x3ace0` for context `+0x1a18`; version >=3 also calls `0x16e590` for `this+0x48` |
| After successful state reads | Calls context virtual `+0x30` (`InitializeExistingGame` `0x45f20`) and `0x5e4cd0`; `InitializeExistingGame` loads gameplay scripts, initializes achievements if present, and calls through game `+0x268` |
| Host LoadGameCoreState `0x169a90` | Constructs compressed IStream through `0x991c20`, calls context virtual `+0x170`, destroys compressed stream via `0x991ca0` |

These are stream callbacks and live initialized object dependencies, not permission
to call Load on the newly allocated factory context. Database, definition,
configuration, scripting, game allocation, archive/index serialization, and engine
services must be genuinely initialized first. `InitializeExistingGame` happens
inside a successful Read; calling it first does not deserialize a save.

The host's `UnserializeGameState` `0x16e340` reads magic (helper `0x169b00`),
container version, typed metadata, and host configuration (`0x169590`) before
session/core/audio/UI/recorder stages. It must not pass the container's offset zero
to GameCore Load. Host configuration version >16 returns 2. Its configuration
metadata and application session/configuration objects are additional inputs to a
standalone loader; decoding their byte maps does not recreate those objects.

## Observed XP2 fixture framing

The save SHA-256 is
`a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8`,
size 7,348,717 bytes. Offsets are absolute byte offsets; ends are exclusive.

| Offset | Observed framing |
| --- | --- |
| 0 | `CIV6` then little-endian u32 container version 1 |
| 8–8842 | Typed metadata map with 28 entries |
| 8842 | Host configuration version 3, metadata version 1, then map with 3 entries ending 8914 |
| 8914–65937 | Version 1 and map with 67 entries |
| 65937–65945 | Version 1 and empty map |
| 65945–65953 | Version 1 and empty map |
| 65953–66257 | Map with 15 entries |
| 66257–103901 | Map with 67 entries |
| 103901–7319671 | 111 length-prefixed compressed chunks: 110 of 65,536 bytes and final 6,366 bytes |

The original compressed input method `host:0x991f20` reads a signed i32 length,
rejects values outside 0..65536, feeds that many bytes into one persistent inflate
state, and refills on input exhaustion. Thus concatenating payload bytes while
removing each u32 prefix is required; ordinary zlib decompression starting at a
raw save offset fails when it encounters the next prefix.

The final pinned chunk ends in `00 00 ff ff`, consistent with `Z_SYNC_FLUSH`;
original output flush `host:0x991df0` passes flush argument 2. There is no
`Z_STREAM_END` or Adler trailer, so finding zlib EOF cannot locate the section end.
The original gameplay consumer's reads determine when to stop; the probe pins
this observed short-chunk boundary and explicitly records `zlib_eof=false`.
It produces 76,556,938 bytes, SHA-256
`a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa`,
starting with context serialization version 4. This is an extracted stream
candidate with opaque gameplay contents, not a decoded simulator state.

The metadata contains app version `1.0.0.13396 (372737)`,
`RULESET_EXPANSION_2`, Dido/Phoenicia, Future Era, standard speed, huge map,
Prince difficulty, and localized Inland Sea text. A nested four-field mod record
contains GUID `4873eb62-8ccc-4574-b784-dda455e74e68` and version string `1`.
The hashed outer key `8427ae5c` is the required-mod metadata list. Saved host
configuration additionally contains explicit `ENABLED_MODS` (`1b8cd1c8`) and
`REQUIRED_MODS` (`fed47f44`) collections, each containing exactly that XP2 GUID,
repeated in both configuration copies. The enabled collections start at offsets
37228 and 84912; required collections at 39778 and 87422. Native host key
initializers, CRC implementation, accessors, and metadata population establish
these names; see [the saved-mod identity proof](../evidence/assets/xp2-saved-mod-identity.json).
This establishes the saved selected-mod collections, not successful current host
content activation, a rules DB hash, or a reconstructed live configuration. The saved app build is older than
the supplied target DLL; accepting version 4 statically does not prove backward
compatibility for every subsystem.

## Prepared native adapter

`native/bootstrap/save/save_probe.c` exposes `bootstrap_save_probe(image, context,
path)` for the runtime owner. It gates the exact private extracted payload size and
SHA-256 and verifies the original export vtable/Load RVA. The synthetic stream
implements only proven raw-read/completion slots, rejects non-sequential,
asynchronous, misaligned, or short requests before copying, and aborts explicitly
on any other slot. It logs initial reads, periodic counts, original return code,
and total consumption. A successful diagnostic return is still not a parity
claim. The adapter compiles under strict MinGW warnings; executing its real Load
call awaits successful original StartupGame with appropriate definitions.

Original StartupGame `0x46be0` is mandatory: it initializes the definition
repository using the context database at `+0x58`, starts the index serializer at
`0x5e4c40`, creates the game through virtual `+0x98` and stores it at context
`+0x60`, calls game virtual `+0x18/+0x20`, and creates worldbuilder/tutorial/cache
objects. These are absent from the narrower Initialize-only fixture.

## Remaining executable gate

Feed the private extracted candidate to the actual loaded context only after the
runtime can complete the genuine existing-game setup with this save's rules and
configuration. Record the complete stream read trace, return code, consumed byte
count, state/RNG observations, and original lifecycle outcomes. Confirm the final
compressed section boundary against that trace. Until then, native load,
configuration semantics, initialized benchmark fixture, and gameplay parity remain
unvalidated.

Method/name leads use the pinned cru121 *civ6-gamecore-reference* revision
`5af389445118bed2bdebf7a8a42b219c8fba5175`, CC BY 4.0; see
[`reference.md`](reference.md). Current Windows bytes decide contracts and slots;
Linux/old-address names are attribution and leads rather than ABI proof.
