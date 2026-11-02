# Linux Unit nested variant wire contracts

This is static framing evidence for the supplied Linux profile, not native
differential validation, complete Unit support, or a state/action/turn simulator.
The [machine manifest](../evidence/linux/unit-wire-contracts-variants.json)
records method byte fingerprints, recursive framing, Linux DWARF enums, and
reader normalization. It gates host `Civ6` SHA-256
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`
and GameCore SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.

Reproduce without starting an inferior:

```sh
python3 tools/linux/unit_wire_contracts_variants.py \
  --private-output ../c6-ghidra/evidence/linux-unit-wire-contracts/variants
```

`objdump` instructions and `gdb ptype` output stay in that private evidence
directory. The public manifest contains contracts and hashes, not game code.
The pinned cru121 reference commit
`5af389445118bed2bdebf7a8a42b219c8fba5175`,
`findings/topics/game-properties-storage.md`, supplied ownership/discovery
leads only. Its Windows offsets and observations do not validate this profile.
The contracts below were read independently from the pinned Linux bytes and
Linux GameCore DWARF.

All integer wire primitives are little endian. The host primitive writers
forward exactly 2, 4, or 8 bytes from the source object without endian conversion.

`Data::VariantMap` (`0x3967ec0`) writes an `i32` scalar count, ascending unsigned
key entries of `u32 key, u32 value_bits`, then an `i32` array count and ascending
unsigned key entries of `u32 key, ArrayVariant`. Each section uses its own tree.
The value bits have no per-entry type tag; the manager's type/name registry is
not written here. Nested ArrayVariants use options zero through the wrapper at
`0x3968040`. Its reader (`0x3968470`) treats negative section counts as zero
iterations. Numeric key bits survive the signed key read.

`Data::TypedVariantMap` (`0x3961e50`) and `TypedVariantVectorMap`
(`0x3966c30`) share the keyed header/member methods (`0x3971510`, `0x3971580`).
Both vtables resolve those methods, and the shared reader `0x3971330`, in the
pinned host. The ordinary map writes ascending unsigned keys; the vector map
writes stored vector order. The latter must not be sorted by a codec.

The header is `u32 count`. Only when count is nonzero, option mask 1 is set, and
the container has a manager, it instead writes `u32 0x80000001, u32 count`.
Members write `u32 key`, optionally `u16 name_length, raw name bytes`, then a
TypedVariant. Name length is `strlen` truncated to 16 bits; exactly that many
bytes are written without a terminator. A missing manager key name writes zero
length. The reader accepts any first word with bit 31 set as a prefix, uses its
low bit to select member names, and reads the next word as count. It appends an
in-memory NUL and registers a nonempty name, but ignores the registration return
value and retains the numeric saved key. No manager dictionary is serialized by
these methods. Option 2 has no effect here. Options propagate recursively;
each nested keyed container independently checks its manager.

`Data::TypedVariant` (`0x395ef70`, reader `0x395eff0`) writes two 16-bit bit
patterns, value type and modifiers, then its ArrayVariant. Neither serializer
switches on the declared type. Linux DWARF gives a 24-byte live object with
type/modifiers at offsets 0/2 and ArrayVariant at offset 8. The full enum identity
is in the manifest. `TypedVariantVector` (`0x3963330`) writes `u32 count` and
TypedVariants in stored vector order, propagating options unchanged.

`Data::ArrayVariant` (`0x3967da0`, reader `0x3968060`) always writes raw
`u32 countFlags, u32 elementSize` first. Count is the low 24 bits. The first
matching branch selects its payload, in this exact precedence:

| Flag | Payload |
| --- | --- |
| `0x02000000` | VariantMap, options zero |
| `0x08000000` | No payload; reader creates an empty VariantVector |
| `0x04000000` | TypedVariantMap, options propagated |
| `0x10000000` | TypedVariantVector, options propagated |
| `0x40000000` | TypedVariantVectorMap, options propagated |
| `0x01000000` | Raw buffer, byte length `(count * elementSize) mod 2^32` |
| `0x80000000` | Inline 64-bit union bits |
| Otherwise | Inline 32-bit union bits |

`0x20000000` (`ValueIsArray`) does not select a writer branch. In the reader's
inline32 branch it causes the just-read union and countFlags to be zeroed;
elementSize survives. Child readers create an owning object and call its setter,
which normalizes metadata and adds OwnsMemory. These behaviors mean raw
wire-preserving transport and native load/resave normalization are separate
contracts. Invalid flag combinations should be rejected unless normalization
is explicitly implemented and checked.

Bool/int32/uint32/float setters clear countFlags and elementSize and use inline32;
int64/uint64 setters use `countFlags=0x80000000, elementSize=0` and inline64.
The bool setter writes only the union's low byte while its writer emits all
four bytes, so upper union bytes must not be discarded. Float bits, including
NaNs, must be preserved. Default None has zero type, modifiers, metadata and
inline bits. Nonnull child setters clear elementSize/count, set the pointer and
OR the child flag with caller high-byte flags; native readers pass OwnsMemory.
Null child setters leave zero metadata and write inline32 zero, with no child
payload. Fresh nonempty char8 construction uses an owning buffer,
elementSize 1 and count `strlen+1`, including the NUL in the raw payload. Its
generic raw setter adds both OwnsMemory and ValueIsArray (`0x21000000` flags).
The raw reader also adds both flags on reconstruction. The generic setter stores
count without masking before ORing high-byte flags, so oversized counts can
collide with flags; safe codecs must reject these counts. The untyped VariantMap
char16 setter uses elementSize 2 and count `char16_strlen+1`, including its
16-bit terminator. A null or empty typed char8 setter leaves a fresh default
ArrayVariant unpopulated; a codec
must not invent a separate string length prefix. Canonical construction and
semantic element widths for other enum members are not established by this audit.

A bounded framing parser can classify scalar bit patterns, raw buffer bytes,
and each recursive container without interpreting unknown payloads as gameplay
state. Reject unsupported type/modifier semantics, excessive recursion/counts,
and multiplication overflow before allocating. The native wrapping multiplication
is evidence, not a requirement to perform dangerous allocations. This audit
does not supply native boundary fixtures, validate complete affected state, or
resolve the recorded 89,198-byte Save→Load difference; unknown save bytes remain
unclassified and unmasked.
