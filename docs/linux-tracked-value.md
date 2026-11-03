# Linux TrackedValue wire boundary

`tracked_value::TrackedValue` transports the current version 1 Linux XP2 record.
This is a typed wire codec, not native Read application, a constructor, a value
update operation, gameplay RNG, an action, or a turn. Complete gameplay state,
action execution and turns remain unimplemented. Its history has not been
classified as planning-only or excluded from exact gameplay state.

The pinned Core SHA-256 is
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`;
the matching host SHA-256 is
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The original ELF's DWARF identifies a 48-byte live TrackedValue with optional
minimum at 0, optional maximum at 8, raw Q8 value at 16, include flag at 20 and
steps at 24. Those are discovery/validation evidence, never production offsets.
The current version word is at `0x2f88e98`. `save` is `0x2d50a60` (412 bytes),
`load` is `0x2d50c00` (1060 bytes).

The little-endian grammar is version u32, raw signed Q8 i32, include-steps bool,
minimum-present bool and optional raw i32, maximum-present bool and optional raw
i32, signed i32 step count, then ordered steps. Each step is priority i32, ID u32,
current Localization::String, and a recursive versioned TrackedValue. Steps are
serialized regardless of the include flag. Duplicate IDs, original order,
negative priorities, optional values and all raw fixed-point bits are preserved.
No clamp, sort, recomputation, definition mapping or policy choice occurs.
Booleans must be canonical 0/1; unsupported versions, negative counts,
truncation and trailing bytes fail explicitly.

Localization uses the existing `map::LocalizedText` codec: byte-preserving base
and names, current typed arguments including recursive text and raw float bits.
The original concrete Core stream operators are at `0x2d4e3c0` and `0x2d4e6e0`;
the audit fingerprints their full bytes and the tracked call sites into those
operators through the original PLT. This verifies the wire provider boundary;
it does not execute or reconstruct the live localization service.

The caller's `unit::Limits` bytes and items aggregate over the entire recursive
record and all localization fields. Each tracked node, including the root,
charges one item; each step charges one item; localization contributes its
existing dynamic element and text-byte charges. Fixed scalar fields do not
charge items. This differs from codecs that charge only dynamic collections.
Prefix decoding reports exact consumed bytes/items and leaves trailing caller
bytes uncharged, so enclosing Player.Stats retains its shared budget.

Depth is a codec resource policy, not a native limit: root depth zero, maximum
64. Over-depth decoding fails before constructing another node, and error
cleanup/drop therefore remains bounded. Each localization value separately
retains the existing localization depth limit of 64. These limits bound the
combined nesting; they do not infer native maximum history depth. The public
Rust types permit caller construction of deeper trees; encoding rejects such
trees, and callers remain responsible for their own constructed value's drop.

Allocation policy is distinct from the serialized byte limit. Before reserving
steps, the decoder checks count against the aggregate item budget, the minimum
remaining wire size (34 bytes per step), addressable typed storage, and fallible
`try_reserve_exact`. A typed Step can exceed its serialized record size; an exact
wire-byte budget must still accept it. Writers reserve output fallibly. Each
localization call receives remaining bytes/items and an independently derived
checked logical-storage cap of 256 bytes per remaining item, sufficient for its
charged vectors and text boxes, preserving localization's allocation checks.
Across fields that storage remains bounded by the shared aggregate item count;
allocator overhead is outside this logical policy. This API does not promise a
caller-specified aggregate heap-byte cap.

Run the static/offline audit with an immutable committed source ref:

```sh
python3 tools/linux/tracked_value_audit.py --source-ref <commit> \
  --output evidence/linux/tracked-value.json
```

The audit hash-gates both original ELF files, fingerprints concrete methods,
checks current version and step call ordering, and compares target DWARF identity.
It builds and tests the candidate from `git archive`, records exact source commit,
source hashes, candidate/harness identity, and never recovers missing modules
from the mutable checkout. There are no direct TrackedValue owner cuts in the
selected records: validation uses hash-gated enclosing Stats v27 cuts, with the
tracked prefix following the version and 22 scalar words. That offset exists
only in the offline validation script. The evidence reports observed record
shapes and source-only branch gaps; saved empty histories cannot validate
nonempty recursive history behavior by themselves. No original code executes,
no private artifacts are modified, and no proprietary data is committed.

Four source tests exercise nested nonempty histories with include=false,
duplicate IDs, both optional values and raw extremes, every localization argument
tag, every-prefix truncation, malformed framing, exact and one-short shared
budgets, typed storage larger than wire size, and depth acceptance/rejection with
safe bounded cleanup. These are source tests, not native branch comparisons.
The pinned reference revision `5af389445118bed2bdebf7a8a42b219c8fba5175` was a
discovery lead; original target bytes and DWARF are the acceptance identity.
Unknown save bytes remain unclassified and unmasked, including the recorded
89,198-byte raw Save→Load difference. This wire boundary does not resolve it.
