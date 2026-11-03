# Linux GameEffects reference reconstruction

The 42 changed GameEffects bytes remain exact object IDs and requirement
references. The [new target-byte audit](../evidence/linux/effect-reference-read.json)
extends the [closed typed classification](linux-save-normalization.md), without
changing either stream or its 89,198-byte raw Save→Load→Save discrepancy. This is
static source evidence and an observation protocol, not native parity or a Rust
game loader. Complete gameplay state, actions and turns remain unimplemented.

The audit pins XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and Linux host SHA-256
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Its 30 function ranges are hashed directly from executable ELF load segments;
all 16 previously closed function fingerprints must match. Host addresses, when
used by a future observer, are absolute ET_EXEC virtual addresses. Function
addresses below refer to module RVAs. Manual instruction interpretation is kept
separate from the mechanically checked identities and fixture decoding.

## What original Read actually does

The global pool at module `0x3c014d8` constructs permanent next ID 1 and temporary
next ID -1 (`0x2335030`). `ClearObjects` (`0x23289f0`) clears both index families
without resetting these counters. Instance v4 Read starts with `Trim`, not
`ClearObjects`. Consequently, existing live roots and earlier startup allocations
can affect subsequent reuse. A fresh process alone does not establish an empty
pool at this Read boundary.

For each serialized object, Instance Read assigns the permanent next counter to
its saved ID (`0x23f5fba`), then invokes the polymorphic payload reader
(`0x23f5fcd`). Team Read consumes its version and team key, checks that the key
is not -1 and occurs among current `WasEverAlive` players, then calls MakeObject.
It returns empty when that membership check fails. It does not directly assign
the returned object's saved ID.

`Share` (`0x23353a0`) chooses the permanent or temporary type map, visits its weak
vector in insertion order, and returns the first live equal object. Team is
permanent (`0x23efc40`); equality compares its type GUID and team key at object
`+0x110` (`0x23efa90`). Reuse preserves the existing object's ID and does not
increment a counter. Otherwise `Append` assigns the selected counter, appends a
weak entry to the ID index and type vector, and increments the permanent counter
or decrements the temporary counter. Both indices may be trimmed before insertion
when capacity is exhausted. The common update at `0x2335744` targets the temporary
counter on its temporary branch: `rbx` was advanced by `0x38` at `0x23356a8`.
Reading that common update without its predecessor would misidentify the counter.

`FindObject` (`0x2335770`) treats zero as empty, searches positive IDs in the
permanent index and negated negative IDs in the temporary index. It takes the
first result from `equal_range`, then locks that single weak entry. It does not
continue through duplicate keys if that first weak entry has expired. Therefore
index ordering and lifetime remain relevant even when an application expects IDs
to be unique. This audit does not establish a duplicate-key occurrence.

RequirementSystem v3 Read resolves context then subject using these exact IDs.
`GetRequirement` rejects a null context, null subject or empty definition ID, and
queries the subject's requirement cache with the context's instance ID
(`0x2336a40`). After a nonnull result, the reader invokes virtual predicate slot
`+0x78`; only a false result permits `SetState` at slot `+0x48` and insertion into
temporary roots (`0x23f7ef1–0x23f7f2d`). The predicate's semantic name is not assumed.

A cache key includes context ID, definition hash and complete definition string
(`CacheRequirement`, `0x2335d50`; lookup, `0x2335fb0`). Its custom hasher consumes
the numeric ID and definition hash (`0x2336110`). Factory success assigns another
runtime ID from RequirementSystem `+0`, increments that separate counter, caches
the result and appends a weak list entry (`0x2336bd8–0x2336c74`). Cache hits bypass
creation and that increment. Object-pool and requirement IDs must remain distinct.
These are concrete identity dependencies; they do not establish a future gameplay
or RNG divergence by themselves.

Instance then loads modifiers, releases temporary requirement roots in reverse
vector order, releases temporary object roots in reverse vector order and trims
expired weak entries. Other strong owners may keep objects alive. Trim preserves
the relative order of survivors. The final serialized pool cannot reveal every
transient object, expired entry, requirement factory result or remaining root.

## Reconstruction extends beyond GameEffects Read

Game Read loads effects at `0x2d7af5b`, then deals, notifications, summary and
VictoryManager. Victory Read clears its old victory requirement ownership.
Before global defeats and victory player reconstruction, Game Read also invokes
local player initialization; map yield/domain restoration and district-dependent
repairs; yield monitor, tech, culture, heroes and analysis postload; each
`WasEverAlive` player's postload; rules postload; and barbarian postload. Nested
creation or callback behavior in these stages remains a source dependency.

Global defeats runs at `0x2d7b307`. Victory initialization then follows the actual
`Alive` vector (`0x2d7b317–0x2d7b35d`), not the serialized Team suffix order.
`InitializePlayer` additionally depends on WorldBuilder activity, player
validity/type/major classification, existing victory entries and defeat/victory
definition order. It shares player/Team objects, creates requirements/sets,
subscribes callbacks and retains successful entries. The creation of a Team is
not unconditional for every player. Definitions, registered factories, providers
and startup context therefore belong to an exact reconstruction input.

The old suffix ends at Team ID 87,494; the new suffix begins at 87,505. If the last
restored Team appended once, its counter became 87,495. If no later setter/reset
occurred and the first recreated Team appended at 87,505, ten intervening
permanent appends would be required. These are conditional deductions, not an
observed ten-allocation explanation. The +24 cannot safely be implemented as an
ID translation or used to infer particular missing objects.

## Minimal next observation

The report specifies a passive, globally sequenced capture from startup through
complete Core Load and two Saves for each canonical input. Observe pool
constructor/clear/set/share/append/trim, Team lifetime, requirement cache/factory
outcomes and the listed stages. Capture every object type, both counters, exact
IDs, stream offsets, per-type and ID-index order, actual selected objects,
control-block/allocation generations, root releases, runtime requirement IDs,
state guard outcomes, player lists and definition/configuration identities.
Pool stage snapshots locate the allocations; object destruction and selected
control-block identities distinguish reuse, expiry and address recycling.

The observer must not call `weak_ptr::lock`, MakeObject, FindObject or native
GameEffects logging for inspection: extra strong references or reentrant logging
could change the lifetime being measured. Use bounded preallocated storage,
forward original calls unchanged and reject dropped/overflowed events. Preserve
both known original output fingerprints and full cleanup before treating a trace
as evidence. RNG observations must identify complete streams and actual accesses;
no RNG equivalence follows merely from the matching serialized synchronous record.

No Rust reconstruction component is added yet. Source closure requires the
actual allocation/reuse/destruction sequence, transitive requirement/set factory
and callback semantics, complete affected-state fixtures and startup identity.
A generic provider interface would leave these same missing mechanics outside
the implementation. A later exact component must reproduce the original
non-idempotent import and retain its resulting identities/order/counters for
subsequent operations; it cannot normalize the canonical fixture into a passing
comparison.

Reproduce the audit without executing original code:

```sh
python3 tools/linux/effect_reference_read_audit.py \
  --module ../c6-assets-linux/libGameCore_XP2.so \
  --host ../c6-assets-linux/Civ6 \
  --initial ../c6-ghidra/evidence/linux-game-load/canonical/ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884.bin.gz \
  --reloaded ../c6-ghidra/evidence/linux-game-load/canonical/85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8.bin.gz \
  --output /tmp/effect-reference-read.json
cmp /tmp/effect-reference-read.json evidence/linux/effect-reference-read.json
```

The optional fixture pair reruns the complete existing typed audit and requires
its report to reproduce exactly. Omitting both paths performs only the static
identity audit and does not claim a fixture recheck. No private game binary,
save or disassembly is included in these public artifacts.
