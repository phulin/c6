# Saved benchmark host configuration

This diagnostic exercises saved configuration maps and an experimental original
Windows singleton-reader path. Successful singleton restoration remains unproven. It
is separate from gameplay deserialization and from c6 Rust snapshot transport.
The Rust implementation still covers isolated primitives; neither configuration
copying nor native map parsing establishes a loaded game or turn equivalence.

Run `python3 tools/saves/configuration_evidence.py` to reproduce the guarded
configuration key initializer manifest. Run
`python3 tools/saves/configuration_probe.py` for the passing native map diagnostic. Add `--restore-configuration` to reproduce
the separately reported experimental singleton-reader attempt. Its launcher
fingerprints the host, GameCore, runtime providers, save, compiled probe and source
configuration. Native serialized map bytes remain private under
`../c6-ghidra/evidence/configuration/native-roundtrip/`. The public observations are
in `evidence/saves/configuration-native-maps.json`.

All host RVAs refer to SHA-256
`e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63`;
GameCore RVAs refer to XP2 FinalRelease SHA-256
`324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`.
The native host static allocator at `0x2347df0` must be constructed: empty
TypedVariant maps can hide this dependency, whereas owned string destruction
calls allocator virtual `+0x20`. Actual initializer `0xef40` constructs its vptr
`0x103c578` and registry roots, then registers its original cleanup through host
atexit. The probe first uses the job manager owner's once-only original CRT
onexit preparation. It never replaces the allocator vtable or map fields.

The target host configuration reader `0x169590` orchestrates configuration,
settings, session and context changes. The narrower native operations are:

| Original boundary | Target-version contract |
| --- | --- |
| GameConfig getter `0x100110` | Lazily allocates `0x1308`, calls complete constructor `0xfeec0`, publishes global `0x22d7a20` |
| GameConfig reader `0xff1f0` / `0x102bd0` | Reads a u32 version and original TypedVariantMap into the instance's root map, then rebuilds native views with `0x101e80` |
| Native view rebuild `0x101e80` | Rebuilds 64 player instances, GameConfig and MapConfig views and native metadata; may normalize the configuration using original code |
| UserConfig getter `0x118000` | Lazily allocates `0x1c8`, calls complete constructor `0x116ec0`, publishes global `0x22d7b90` |
| UserConfig reader `0x1190f0` | Accepts version <=1; reads map at `+0x90`, player settings count and each player ID/map, sets loaded flag at `+0x16a` |
| TypedVariantMap constructor `0x7a44a0` | Constructs original map and publishes vptr `0x103c918` |
| Original map read / write | Native map virtual `+0x1e8` -> `0x7ac2f0`; virtual `+0x1f0` -> `0x7a5b00` |

The original key globals also require initialization before native view rebuilding.
The manifest records 148 closed pure initializer functions, each guarded as
original-literal address, call to original CRC `0x7a0da0`, one original global
store and return. The special original initializer `0x5a90` builds the 64
per-player keys. No key hash or default server setting is fabricated. The native
configuration constructor and readers decide all resulting fields. Calling the
original CRC wrapper also depends on original C++ thread-local static initialization
and CRC table state. That startup dependency is not established by calling these
pure initializer bodies in the bounded EXE mapping; the manifest does not establish
correct published live key values.

The XP2 fixture's GameConfig version/map span is `8914–65937`. UserConfig version
and empty map span `65937–65945`. Bytes at `65945` contain player settings count
one, at `65949` player ID zero, and `65953–66257` the 15-entry player settings map.
The apparently equivalent "version one / empty map" framing of `65945–65953`
was a byte-level ambiguity resolved by original reader `0x1190f0`. The final
67-entry map at `66257–103901` belongs to a later host stage and must not be read
as part of UserConfig.

The probe uses the genuine borrowed original GameCore MemoryIStream and genuine
MemoryOStream allocator, preserving the shared Windows Serialization interface.
It compares all five genuine individual native map reads and typed serialized values with
the independent bounded Python parser. Native writer normalizes saved variant flags: an owned nested map may change `04000000` to
`05000000`, and a zero scalar carrying the invalid marker `20000000` loses that
marker. The main saved map exhibits three such changes, preserved explicitly in
the report. Comparison sorts map keys, preserves list order and checks all typed
payloads against these documented native canonicalization rules. Raw byte and
raw typed equality remain separate observations; neither is reported as passing
for the main map.

The passing diagnostic records five complete native map reads, serialization and
original cleanup. Four maps are byte-identical; the main configuration map differs
only by the three recorded original flag normalizations. Original native collection
count helper `0x25abc0` and nested field helper `0x25ac50`, plus original map lookup,
independently confirm one enabled and one required XP2 mod with GUID
`4873eb62-8ccc-4574-b784-dda455e74e68` and `RULESET_EXPANSION_2` in both saved
configuration copies. These helpers receive genuinely parsed maps and proven
saved key hashes, without a synthetic GameConfig object.

The restore helper refuses existing GameConfig/UserConfig singletons and gates the
entire original XP2 save by SHA-256 and size before mutation. It retains genuinely
constructed globals for subsequent original initialization. Passing its stream
boundary and view checks is not the full `0x169590` orchestration: session,
notifications, later host stages, full cleanup, database identity and gameplay
state still need separate observations. The current singleton-reader attempt constructs original GameConfig and UserConfig,
then faults in original read/view code at wide-string copy host `0x984210`
(return address `0x1c981b`, source argument `0x4b0`); its observed phase and arguments are retained in
`evidence/saves/configuration-native-restore-attempt.json`. Runtime integration
remains conditional on resolving the original host provider and startup dependencies and
validating the phase and complete executable diagnostic. No restore helper is
wired into the runtime.

Private decompilation evidence is under
`../c6-ghidra/evidence/configuration/`. Method naming leads use the attributed
cru121 reference described in [reference.md](reference.md); current target bytes
and native execution determine contracts.
