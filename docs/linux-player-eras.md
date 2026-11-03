# Linux Player.Eras wire boundary

The pinned Linux XP2 writer `0x2aee400` (66 bytes) writes current version 2
from `0x2f5ded8`, then `m_eEra`, then `m_eGameEra`. Matching DWARF puts the
FAutoVariable wrappers at object offsets 160 and 184, with their values at
176 and 200. Each EraTypes value uses definition-index-to-hash serialization:
the wire is exactly three little-endian u32 words, **not** two raw enum indexes.
`player_eras::Eras` preserves both definition hashes, including unknown and
sentinel patterns. It does not consult rules definitions or invent choices.

The concrete reader is `0x2aee450` (97 bytes). Its first era is unconditional;
its second era is versioned since v2 (the historical default is enum index -1).
Only current v2 is supported. FAutoVariable load/save virtual slots resolve to
`0x2aef1b0` and `0x2aef1f0`; EraTypes overloads resolve to `0x2d4edd0` and
`0x2d4eda0`. The native reader remaps hashes through Era definitions and writes
the wrapper values directly. Current Read does not call Reset or dispatch
assignment callbacks. These native operations and archive ownership are separate
from the wire codec. `Eras::Reset` exists independently at `0x2aee750`.

The Player writer calls GetEras at `0x2b4f101`, then the Eras serializer at
`0x2b4f10c` through PLT `0x1e75d00`, whose concrete target is `0x2aee400`.
The next component is Trade (serializer call `0x2b4f11f`, PLT `0x1e397e0`).
No PLT address is treated as a serializer body.

Strict decoding rejects trailing input; prefix decoding consumes 12 bytes.
The aggregate wire charge is 12 bytes. Fixed scalar fields have no collection
item charge, so the exact item cap is zero and a one-short item cap does not
exist. There are no dynamic typed allocations. Encoding checks the wire cap
and uses fallible reservation for its output. This is not an aggregate heap cap.
There are no boolean, variable collection, localized-string or nested-version
fields in this record.

`tools/linux/player_eras_audit.py` gates both Linux module and host hashes,
method bytes, concrete PLT bindings, virtual slots, version and DWARF. It builds
one committed source archive, including the actual module and audit driver;
no working-tree recovery supplies candidate inputs. The pinned canonical streams
and owner traces supply validation cuts only. Eras has no direct owner hook;
matching-parent Units ends and Trade starts independently bracket its 12 bytes,
consistent with the verified Player call order. Every complete Eras record in both
streams must decode and re-encode identically, succeed at exact byte/zero-item
caps, reject every truncation and reject the one-short byte cap. Source tests
also cover nonempty unequal hashes, sentinel patterns, duplicates, every short
byte cap, strict/prefix behavior and unsupported versions.

Run with a committed candidate identity:

```sh
python3 tools/linux/player_eras_audit.py --source-commit COMMIT \
  --output evidence/linux/player-eras.json
```

This is complete current Eras **wire transport**, not complete Player loading,
world import, gameplay state, action execution or turns. All player decisions
remain external. Unknown save bytes remain unclassified and unmasked; the
recorded raw Save→Load difference is still 89,198 bytes. No original game code
is executed and no private evidence or proprietary disassembly is written.

Validated candidate source `9f9891a` passes 43 records (516 bytes) in each
canonical stream, 86 records total. Every record roundtrips byte-for-byte; all
truncations and the 11-byte cap fail. Both isolated source tests pass. The
complete source commit, archive, source manifest, binary and harness identities
are recorded in `evidence/linux/player-eras.json`. These offline source records
are not a live initialized-game or native-Read acceptance test.
