# Linux Player.Techs current wire

`player_techs::Techs` transports the complete current Linux XP2 version 6 wire
record. It does not initialize gameplay state, remap definition hashes, apply
native Read fixups, run AI, execute actions, or advance turns. Full Player/world
and exact gameplay/RNG remain unimplemented. Unknown save bytes remain unmasked;
the recorded raw Save→Load difference remains 89,198 bytes.

The target is Linux `libGameCore_XP2.so` SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
with host `Civ6` SHA-256
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The actual writer is `0x2b89f90`, reader `0x2b8a090`, and current-version
word `0x2f685e4`. The matched target's DWARF identifies the owning fields;
Linux instructions establish wire order independently of member order.

Every integer/count/hash is little-endian 32 bits. The wire grammar is:

| Field | Representation |
| --- | --- |
| version | u32, exactly 6 |
| research queue | u32 count, then technology definition hashes |
| research overflow | signed Q8 raw i32 |
| science rate change | signed Q8 raw i32 |
| modified boost | i32 |
| acquired technologies | u32 count, ordered (u32 hash, canonical bool byte) pairs |
| boost triggered | same ordered bool hash-array grammar |
| research progress | u32 count, ordered (u32 hash, signed Q8 raw i32) pairs |
| random tech boosts on new era | i32 |
| number of technologies repeated | i32 |
| free tech boost wonder era | i32 |

Queue writers/readers `0x2b8f680`/`0x2b8f610` write a count and iterate the
whole queue. Template parameter 4 is not a serialized count or a four-entry cap.
Technology enum operators `0x2d4eef0`/`0x2d4ef20` use definition hashes;
queue entries must not be treated as raw enum ordinals. Bool hash-array methods
are `0x218fe50`/`0x218fd60`; Q8 hash-array methods are
`0x2b8dd50`/`0x2b8de60`. Duplicate/unknown hashes and queue order are retained.
No SQL row ordinal mapping or guessed default is supplied.

The native reader calls Reset before reading fields and supports historical
version branches, including an old TreeManagement record for versions ≤4.
This codec accepts only current v6; it does not guess those historical grammars.
The acquired field reuses `player_prerequisites::IndexedBools`. Its separately
proven `acquired_after_owner_read` operation models Reset(false) and the actual
definition overlay; wire decoding never invokes that runtime initialization.

`decode`, `decode_prefix`, `decode_prefix_with_items`, `encode`, and
`encode_with_items` expose strict/prefix transport and exact bytes/items.
Dynamic item cost is the sum of queue entries and each hash-array pair count.
Wire bytes and items are aggregate budgets under `unit::Limits`. Each vector's
typed allocation is independently checked for multiplication overflow and
addressability, then fallibly reserved. Typed allocation bytes are not charged
against the remaining wire-byte budget. For `IndexedBools`, the adapter checks
count/items, full wire length and available input before delegating, then derives
a separate allowance large enough for its checked typed storage. This explicitly
relaxes the underlying helper's per-vector `max_bytes` storage cap; it preserves
the aggregate wire cap and introduces no unchecked allocation. The nonempty
200-pair regression has typed storage larger than the complete wire record.

Reproduce the read-only audit with:

```sh
python3 tools/linux/player_techs_audit.py --output evidence/linux/player-techs.json
```

The audit builds from an isolated archive of committed source `5057bac`, records
its full commit/source hashes, and never recovers source from shared build output.
It pins 27 complete method ranges, DWARF identity and five neighboring writer
calls. Both canonical hash-gated Core streams contain 43 complete Techs records
(61,662 bytes each stream). All 86 decode/re-encode byte exactly, reject every
truncated prefix, accept exact byte/item caps, and reject one-short caps. Owner
trace cuts are offline validation inputs only; production codecs accept no trace
or save offsets. Five source tests additionally cover nonempty queues exceeding
four entries, duplicate hashes, signed/Q8 boundaries, bool/version/trailing
rejection and hostile counts. This is saved-wire transport evidence, not native
Read state parity or original execution.

Player writer calls `0x2b4f0ad`, `0x2b4f0c0`, `0x2b4f0d3`,
`0x2b4f0e6`, `0x2b4f0f9` establish Resources → Stats → Techs → Treasury →
Units. The exact next component after Techs is Treasury.
