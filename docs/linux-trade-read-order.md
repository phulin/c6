# Linux Trade fresh Read order reconstruction

`trade_read_order::fresh_read_wire_order` reconstructs the serialized origin and
destination traversal of a **fresh**, default-container Linux XP2 Graph after its
current-version Read. It preserves every Edge payload and ID and reproduces the
complete 469,638-byte second canonical Graph from the first canonical Graph.
This is source-derived container reconstruction, not native runtime gameplay
acceptance, a complete save importer, action execution, or turn support.

The target is GameCore SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and host SHA-256
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Host addresses below are absolute ET_EXEC VAs. The
[audit report](../evidence/linux/trade-read-order.json) records exact complete
symbol extents and byte hashes, source hashes and both full canonical identities.
It executes Rust over existing native outputs and never executes original code.

## Original construction and traversal

Graph constructor `0x2bf9740` constructs the origin and reverse maps with initial
bucket request 51. Concrete EASTL construction at `0x2c038a0` uses load factor
1.0 and prime policy construction `0x1fced20` sets growth factor 2.0. Host
`GetNextBucketCount` at `0x3974b36` selects the first adequate entry in the
256-word prime table at `0x46433c0`: initial 53 buckets. Host
`GetRehashRequired` at `0x3974c2a` tests the prospective element count against
its resize threshold, selects a prime at least twice the old capacity, and
updates the threshold. The Rust resource policy limits origins and destinations
to one million; the report records all growth steps needed within that bound.

Graph Read `0x2bf8210` reads origin keys in stream order, inserts/reuses each outer
entry, then reads its destination/Edge pairs. It allocates and reads each Edge,
attaches its archive, populates its ID entry, then sets the forward and reverse
entries. Empty origins are inserted too. The reader does not clear the graph.
Consequently fresh defaults are an explicit precondition of this component;
pre-existing bucket counts, contents and load history cannot be inferred from
Graph wire bytes. Graph Clear `0x2bf99c0` is a distinct lifecycle operation.

The outer `DoInsertKey` at `0x2bfe010` finds an existing equal key before allocation
and growth. A new entry is prepended to its hash bucket. `DoRehash` at
`0x2bfe210` visits old buckets in ascending order, pops their chains from the
head, and prepends each node into its new bucket. Iteration (`0x2bfdef0`,
`0x2bfdeb0`, begin `0x2bfca40`) visits ascending buckets and follows each chain.
It does not sort keys. This chain reversal makes another reload's traversal
potentially different again; idempotence is not assumed.

Each inner map is the concrete `Types::HashMap<Vertex, Edge*, VertexToIndex,
51, 0>`, not EASTL. Default construction `0x2bfe380` / `0x2bfe390` starts with
zero slots and native single-precision load factor 0.7. `CanSet` at `0x2bfe9a0`
tests the current load and requires two free slots. `Set` at `0x2bfcc90` grows
to max(4, twice capacity) before insertion when needed. Resize `0x2bfe9c0`
clears new slots and copies occupied old slots in ascending order using
`CopyFrom` `0x2bfed50` and its distinct no-key-selector insertion `0x2bfee00`.

`Set_Base` at `0x2bfeb20` uses Robin Hood linear probing with wraparound. A
resident is displaced only when its probe distance is **strictly smaller**
than the incoming distance; equal distances retain resident precedence.
Matching hash plus matching Vertex replaces the value at the existing slot.
`MapToRange32` `0x28e91c0` computes the upper 32 bits of capacity times the
wrapping 32-bit product `hash * 0x7f4a7c15`. `ProbeLength` `0x28e91f0` measures
wrapped distance. Forward iteration `0x2bfdfd0` scans ascending slots and skips
the `0xffffffff` empty-hash sentinel. No keys are sorted.

Vertex GetHash `0x2bf8540` ORs `(u16(type) << 48)`, `(u32(data1) << 32)` and
**sign-extended** data2 into a u64. It does not mask data2 to 32 bits. The
MSVC2012 bitwise wrapper `0x2bfcd30` runs FNV-1a `0x20ea2f0` over eight
little-endian bytes with wrapping 32-bit arithmetic, then zero-extends the
result. EASTL uses that hash modulo bucket count; inner VertexToIndex
`0x2bfed20` XOR-folds its high/low halves (`0x2bfefb0`), which leaves this
zero-extended hash unchanged. Packing collisions do not replace Vertex equality.
The Rust component accepts the codec's unique, consistent valid keys; it does
not extend native malformed duplicate handling into an importer claim. A
destination whose native hash is `0xffffffff` returns an explicit unsupported
sentinel-hash error before reconstruction; the native inner insertion reserves
that hash and does not insert it. This includes the boundary Vertex
`(type=0, data1=3, data2=1621429197)`.

## Complete owner comparison and remaining boundaries

Run:

```sh
python3 tools/linux/trade_read_order_audit.py \
  --output evidence/linux/trade-read-order.json
```

The audit gates both 80,657,462-byte streams by their full hashes, checks each
complete extracted Graph hash against the earlier trace-validated
[codec report](../evidence/linux/trade-rust-codec-1.json), decodes every typed
origin/Edge, and runs the Rust component over each complete owner. The first
input's reconstructed encoding must equal every byte of the observed second
owner: 179 origins, 1,571 Edges, unchanged keyed payloads and all paths/yields.
The second input's reconstruction is also compared to both actual owners;
its prospective next-load encoding is a prediction without a third native
output and is not accepted as native parity.

The complete raw streams still differ by **89,198 bytes**, including the
**89,156 Trade ordering bytes**. Nothing is sorted, masked, or excluded from
state. The remaining 42 GameEffects/Team reference bytes remain a separate
lifecycle task. Reproducing this native non-idempotent read behavior is distinct
from removing that raw discrepancy.

The serialized-order component does not instantiate the native block allocator,
archive attachments, reverse destination map or ID map. `IterateRoutes`
`0x2bfa860` uses the ID map; outgoing callbacks `0x2bfa910` use inner map order.
Full callback state, retained capacities across mutations, deletion/reinsertion,
archive behavior and omitted Route `m_bFinalizing` still need their own state
and native differential acceptance. This component cannot establish future
RNG consumption, gameplay tie breaks or decision/turn equivalence.

For a native runtime acceptance test of this bounded component, the minimal
passive observation is the graph's pre-Read origin/reverse/ID container sizes,
bucket/slot counts and load-policy fields, followed by post-Read ordered keys,
Edge IDs and callback-map order, plus the original complete Graph Write bytes.
That would confirm the fresh-default lifecycle precondition and independently
accept live maps rather than inferring them from output order. No missing
allocator address is substituted or guessed here. The fixture's broader saved
host RequestLoad/Begin/finalization acceptance remains incomplete.
