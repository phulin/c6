# Bounded initialized CityAttributes entry0 invariant

This is a source proof packet for the remaining 1,709 fallback queries in
[native-2](linux-movement-domain-inputs.md). It does not change the frozen
`60507af` / `e899e4d9…` candidate or promote its unavailable results to parity.
Original observed zero masks and false Wonder answers are expected outputs only.
No original game code runs in the [static audit](../tools/linux/city_attribute_default_audit.py).
The [packet](../evidence/linux/city-attribute-default-invariant.json) pins complete
method fingerprints, all target-internal relevant direct calls and every direct
reference to the entire primary pool object.

The proposed invariant is bounded to a fresh canonical single-Map read/restore
sequence with positive width and height (`P > 0`) and `2 * P <= 65535`. This is not a claim that entry0 is
unconditionally immutable. A concrete original overflow counterexample remains
in scope for future full-engine implementation.

## Reservation, initialization and relocation

The primary sparse pool is a **local** 32-byte BSS object at `0x3c05df0`.
It is distinct from the Cache/UI pool at `0x3c02de0`. All 13 direct references to
the complete primary object's address range belong to its global initialization,
CityPlots construction/destruction/initialization/uninitialization, and the four
CityPlots Get/Edit/Allocate/Deallocate entry points. No separate primary pool
write entry point appears in that storage-reference inventory.

`CityPlots.Initialize` clears the sparse vector and allocates its first element,
reserving index0, then resets each plot's local attribute index. The first Alloc
uses vector size zero; a sparse node constructs CityAttributes and calls
Initialize. Original Initialize stores domain i16 **0**, packed Wonder i16 **-1**,
owner **-1**, working/owning/District IDs **UINT_MAX**, and plot-yield-effect **0**.
These are direct original stores, not values inferred from the native-2 answers.
Node/vector construction and copy/move relocation preserve the payload; reserve
and growth do not substitute data from another node for entry0.

`DeallocateAttributes` explicitly refuses index0. For a live initialized pool,
all ordinary primary frees therefore leave it allocated and outside the free
chain. Alloc reuses free nodes or appends at vector size; Free does not shrink
that vector. Uninitialize/destruction clear storage and end this live invariant;
no default getter semantics are claimed during that interval.

## Mutable receivers

`Plot.GetCityAttributes` is the only direct caller of primary GetAttributes. It
reads the plot's u16 index and returns its node if valid, otherwise entry0.
Primary EditAttributes has exactly one direct caller: `Plot.EditCityAttributes`.
For a plot whose stored index is zero, that method first allocates, stores the
result in the plot's u16 field, then requests the mutable node.

The audit records 16 primary domain-mask setter sites. Every receiver comes
from primary Plot mutable virtual slot `+0x40`, including SetValidDomains' inline
mask clear and four subsequent setters, both construction-complete paths,
pillage/destruction/transfer paths, neighboring-city marking and the XP2 load
augmentation. The DomainTypes setter is one forwarding wrapper with no incoming
target-internal direct calls. The three Wonder setter sites all use CManager's
mutable Plot wrapper, which calls that same `+0x40` slot. Their saved-register
aliases remain the allocated attribute receiver.

CityAttributes.Read has one direct caller, Plot.Read: that resets the plot index
to zero, then uses mutable `+0x40` only when saved HasCityAttributes is true.
CityAttributes.Update has three direct callers, all Cache::Map Plot
initialization/update paths. Those mutate the separate Cache pool, not primary
entry0. Full method fingerprints retain these receiver and relocation paths;
the proof does not rely on the absence of a method name containing “AI”.

## The actual fresh-load allocation bound

Map.Read calls Initialize(params), which calls Reset and InitPlots. Reset
uninitializes an existing nonempty map's CityPlots. InitPlots establishes the
primary Plot vector, initializes its canonical coordinates, then calls
CityPlots.Initialize before component and Map-initialized callbacks.

Only two target-internal sites call the primary Plot constructor: a vector
resize fill temporary and the base construction of Cache::Map::Plot. The fill
temporary is inserted before attribute allocation; Cache immediately uses its
own vtable/pool. Primary Plot vector resize is called only by InitPlots and the
indexed Plot-vector reader. Primary Plot.Initialize is called only by its
constructor and InitPlots. The current Map codec requires the saved vector
length to equal width times height, so the reader's resize does not introduce
additional primary plots after initialization.

It is **unsafe to use the map count alone** as the number of allocated nodes.
Component/Map-initialized callbacks may already have edited canonical plots
before the saved Plot pass. Up to `P` plots can own one slot each at that point.
The single Plot.Read pass overwrites each local index and can orphan those
pre-read nodes, then allocates up to `P` saved owned nodes. Subsequent ordinary
canonical edit/free operations preserve one current owned slot per plot. Free
nodes are reused before vector growth; the possible pre-read orphan set does
not grow again in this single-read sequence. The conservative high-water bound
is therefore **`2*P + 1` constructed logical nodes**, including reserved0, and maximum allocated index
**`2*P`**. For the actual 6,996-plot fixture this is at most 13,993 nodes/index
13,992. Geometric storage capacity may be larger; this bounds constructed vector size, not reserved allocation bytes. The proof requires canonical primary Plot ownership and this fresh
single-read lifetime; it does not infer allocator history for arbitrary imported
live memory or multiple out-of-band reads/edits.

## Original overflow is a real limit

Plot.EditCityAttributes stores Alloc's u32 result in a **u16** field. At result
65,536, that stored index becomes **0**. EditAttributes(0) sees the reserved node
as valid and can return it for mutation. Thus constructor defaults plus a zero
free guard do not prove an unconditional default invariant. The prospective
bounded model must reject its allocation-bound failure explicitly. Larger-map
or arbitrary-history support still requires the original overflow/alias behavior
and adequate allocator state; it is not permanently excluded from the complete
state/action/turn objective.

No new Rust default model or all-plot comparison is claimed by this packet.
After source review, a new bounded candidate may model the explicit reserved
state and compare it against all retained native-2 rows. The historical raw
89,198-byte Save→Load failure remains unresolved and unmasked.

## Inline stores and bulk copies

The [extended writer inventory](../evidence/linux/city-attribute-writer-inventory.json)
pins all 92 exact BYTE/WORD destination stores at offsets `0x14`, `0x15`, `0x1e`
and `0x1f`, including read-modify-write and setcc instructions, with 81 complete
containing and related copy-method hashes. Nine are WORD stores: six are the
known attribute initialization/read/setter/inline-clear paths. Three have distinct
receivers: a freshly allocated BuildingProductionData effect object, a Combat
Member, and a freshly allocated PetTheDog command. Of the 83 BYTE stores, 29
write stack locals; the other 54 trace to distinct typed formal objects, fresh
effect allocations, a TargetDamageInfo vector element, Reflection output DTOs,
command parameters, or Lua MapInitData userdata. AI-namespaced objects are
classified by these receiver origins, without excluding their gameplay effects.
This narrow-store inventory is supplemented by the typed bulk paths; it is not
an assertion that arbitrary wider writes or adjusted pointers cannot exist.

CityAttributes' three copy-constructor call sites write stack destinations
(`RSP+0x10`, `RSP+0xb0`, `RSP+0x10`). Its other default-constructor caller writes
the separate global empty object at `0x3c05d68`; its move constructor is used by
sparse-node relocation. Plot copy construction and assignment preserve the
8-byte word at `+0x48`, including the local attribute index, and are confined to
vector helper paths. The canonical read uses the incoming dimensions before
Reset/InitPlots; Game.Read invokes the 20-byte Map stream wrapper once, which
calls Map.Read once, before yield/domain restore and XP2 augmentation.

CityAttributes Reflection Fill reads the const source payload and writes a
caller-provided 24-byte Reflection output. Its export forwards that output
pointer unchanged. The only direct internal Fill caller is EventStream.Manager.Add, whose output is a stack DTO at `RSP+0x38`; it then builds a separate event node. The cache Initialize/Update bulk assignments copy from
primary const attributes or event payloads into cache mutable attributes.
These directions do not expose a primary mutable receiver. The bounded proof
assumes well-typed internal ownership and the normal Reflection output contract;
it does not authorize arbitrary external callers to alias an output buffer to
the private primary pool.

The source packet still awaits independent final receiver-path review before a
new Rust default model is added. No new native execution or parity claim is made.
