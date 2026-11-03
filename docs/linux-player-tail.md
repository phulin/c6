# Linux Player v47 final inline tail

This is saved wire transport for the pinned Linux XP2 module (`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`), not initialized gameplay state or native Read behavior. Complete Player/world import, actions and turns remain unsupported. The recorded raw Save→Load difference of 89,198 bytes remains unmasked.

`player_tail::Tail` takes an explicit `Context { player_version: 47, was_ever_alive }`. The context comes from the enclosing Player header and adds no wire bytes. Other parent versions fail closed; encoding rejects disagreement between the flag and `alive: Option<AliveTail>`. Decode, prefix decode, prefix decode with item usage, encode and encode with item usage accept `unit::Limits`.

The writer is `Serialization::operator<<(OStream&, Player::Instance const&)` at `0x2b4ee00` (1679 bytes); the reader starts `0x2b4f490`. When `m_bWasEverAlive` at object value offset `0x278` is true, the final inline portion begins immediately after BehaviorTree.Manager, at writer `0x2b4f315` / reader `0x2b4fe20`. When false, the earlier branch skips three boolean hash arrays, fifteen AI records **and the conditional inline fields**, directly to properties at writer `0x2b4f413` / reader `0x2b5007f`.

Each vector/list has a little-endian u32 count, then the following entries in stored order. No extra scalar follows the agenda list. All hash fields preserve raw u32 identities, sentinel bits, duplicates and order.

| Condition | Field | Entry wire |
|---|---|---|
| ever alive | agendas | agenda hash u32 |
| ever alive | agenda visibilities | signed i32 enum value |
| ever alive | foreign continent city unit classes | promotion-class hash u32 |
| ever alive | district naval units | district hash u32 |
| ever alive | improvements | improvement hash u32 |
| ever alive | specially allowed projects | project hash u32, signed i32 |
| ever alive | chop behaviors | feature raw i32, canonical bool u8, yield signed i32, signed amount i32 |
| ever alive | additional pillaging | plunder signed i32, improvement and district hashes u32, signed amount i32 |
| ever alive | CO2 reductions | signed i32 |
| always | properties | existing TypedVariantMap grammar, without an added outer vector |
| always | goody hut replacements | improvement key hash u32, improvement value hash u32 |
| always | random events avoided | random-event hash u32 |
| always | terrain work impassable | signed i32 |
| always | random event no unit damage | signed i32 |
| always | random event modified opposing-player damage | signed i32 |

No nested versions occur in these records. Player version gates in native Read select legacy fields/defaults; this module supports only current v47. Properties use the host Data::TypedVariantMap vtable: symbol `0x53b3680`, object address point +16, slots +`0x1f8` Serialize (`0x3961e50`) and +`0x1f0` keyed reader (`0x3971330`). Existing typed-variant unsupported types/modifiers/branches fail closed.

The chop writer (`0x2b4ecc0`) writes FeatureTypes directly as an integer. Its reader (`0x2b4ed10`) invokes the hash-remapping FeatureTypes reader instead. `feature_wire` deliberately preserves the actual saved integer; no remapping or repaired resave is claimed. Native Read also initializes agendas, edits archives, resets project values, clears/defaults vectors, remaps hashes and calls InitializeListeners after consuming the tail. Those effects, constructor defaults, decisions and RNG changes are not executed.

Tail-owned vectors use checked aggregate item/storage arithmetic and minimum remaining wire checks before fallible exact reserves; decode validates the whole prefix before collecting them. Encoding preflights then reserves output fallibly. Nested properties share the remaining wire/item credit and depth limit 64, but the inherited bounded variant codec currently uses infallible Vec push/to_vec/collect internally. This is an explicit transitive allocator limitation, not a claim that every allocation is fallible.

`tools/linux/player_tail_audit.py` pins original module/host identities, parent and nested machine bytes, DWARF layouts and properties virtual bindings. It builds only an immutable committed Git archive containing the actual driver and module, records all Rust source hashes and the executable identity, and compares independent Python typed projections plus exact encoded bytes from the actual BehaviorTree.Manager end through Player end. Trace boundaries are validation-only. Production code never searches for signatures or consults traces. Both canonical streams contain 43 ever-alive tails; the false branch is tested from source grammar only. Actual properties are empty in these fixtures; nested nonempty variants retain the separately documented variant boundary and validation limits.
