# Linux Unit archive and serialized state contract

The implementation still covers isolated primitives. This is a statically verified state-schema contract for the exact Linux XP2 module `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, with matching DWARF and the pinned original Linux host. It does **not** implement a Rust Unit codec, execute SKIP_TURN, decode a complete Core Save, or establish complete affected-state parity. Unknown gameplay data remains unknown and cannot be advanced by retaining opaque bytes.

Run:

```sh
python3 tools/linux/unit_archive_audit.py --output target/linux/unit-archive-schema.json
```

The [schema artifact](../evidence/linux/unit-archive-schema.json) contains every registered Unit and Formation variable's original name, C++ value type, wrapper/value offsets and sizes, constructor call, archive index, and complete top-level write order. The audit verifies original ELF identities, all typed wrapper constructors' individual base-registration calls, actual argument provenance, matching target DWARF, method fingerprints, and SkipTurn's inherited serializer bindings. Raw disassembly and DWARF remain private in `../c6-ghidra/evidence/linux-unit-archive/`. Research consulted cru121's CC BY 4.0 reference at revision `5af389445118bed2bdebf7a8a42b219c8fba5175`; see [attribution and limitations](reference.md). The conclusions here are independently grounded in the supplied Linux target, not Windows addresses or inferred cross-platform equality.

## Registration identity and dirty state

`Unit::IInstance::IInstance` (`0x2c1c180`) constructs the actual archive at Unit `+8`, then registers ten variables. `CInstance` (`0x2c128b0`) registers six more, and `Instance` (`0x2c24f10`) registers another 57. All 73 pass the same actual archive receiver. Each typed constructor calls `FAutoVariableBase` once; the host base constructor calls `FAutoArchive::add` (`0x3c459e8`), which appends its variable pointer. There are no name-derived or hash-derived field IDs here: the index is the pointer's position in that vector.

| Unit field | Archive index | Wrapper | Inner value | Native value type |
|---|---:|---:|---:|---|
| `m_eAutomation` | 10 | `0x268` | `0x278` | enum32 |
| `m_eActivity` | 11 | `0x280` | `0x290` | enum32 |
| `m_iOperationTimer` | 32 | `0x518` | `0x528` | int32 |
| `m_formationID` | 70 | `0xba8` | `0xbb8` | uint32 |
| `m_kOperationsDisabled` | 71 | `0xbd0` | `0xbe0` | original definition-indexed bool list |
| `m_kOperations` | 72 | `0xc00` | `0xc10` | original 200-byte OperationQueue |

`m_kHero` at `+0xcd8` is different: its default wrapper constructor (`0x2c2b510`) sets the owner pointer to null and does not register with this archive. It has **no archive index 73**, although the explicit Unit serializer writes it. Absence from archive deltas therefore does not prove a field is absent from gameplay/save state.

Host `remove` (`0x3c45a10`) compacts the variable vector. Constructor indexes describe the intact original registration; a live observer should cross-check the original archive vector before treating observed indexes as field identities. No live native index-to-pointer cross-check is claimed here. Do not generalize these numbers to another class, version, or dynamically modified archive.

Activity's tracked assignment (`0x2da4010`) compares source and destination **addresses**, not values. With an archive owner, it inserts the wrapper pointer into the native unique dirty set, then copies the value. Manager `SetActivityType` (`0x2da1840`) separately checks value equality and returns before assignment/reporting when already equal. Timer `SetTimer` (`0x2da2a20`) has no such value check: writing zero from its stack marks index 32 dirty even when the timer was zero. The identical int assignment body is at `0x1fe7180`.

Queue `edit` (`0x2da3100`) marks index 72 dirty before returning the original queue. A queue that starts and finishes empty can therefore have a real dirty delta after insertion/removal. Dirtiness is historical tracked mutation, not a comparison of final serialized values. Repeated edits deduplicate the same wrapper pointer. Preserve pre-existing dirty state and the original iterator order.

`getDeltaIndexes` (`0x3c45f34`) does not clear dirty state: it traverses the native dirty-pointer set and maps each pointer to its current original vector index, skipping pointers absent from that vector. It returns an owned `Types::Vector<unsigned int>` through 24 bytes of hidden output storage in RDI, with archive in RSI. Original `size`/const-index methods and destructor are identified in [the SkipTurn contract](linux-skip-turn.md); no manual vector representation is authorized. A getter result of no indexes is weaker than proving no dirty pointers when registration integrity is unknown. For these intact classes, registered wrapper offsets increase with registration order, but generic archive outputs must retain native order rather than be sorted by the observer.

## Actual Core Unit record

The explicit `Serialization::operator<<(OStream&, Unit::Instance const&)` is at `0x2c22d00`; the reader is at `0x2c231f0`. This writer is distinct from host `FAutoArchive::save`, which writes an archive header then virtual saves in registration order. A Core Unit record must not be decoded as an archive payload.

The current Unit writer emits version **28**, followed by 73 field writes. Its full 74-entry order is machine-readable under `unit_write_order` in the schema artifact. It writes 72 of the 73 registered fields, omitting `m_eDomain`, and additionally writes unregistered `m_kHero`. Unit domain derivation/read compatibility remains a separate contract; an omitted value is not permission to discard it from live exact state. Properties are the final write: original inner TypedVariantMap at `+0x1f8`, virtual `+0x1f8`, flag 1.

The affected field positions are **ordinal positions**, not fixed byte offsets:

| Write ordinal (version is 0) | Field |
|---:|---|
| 11 | formation ID |
| 13 | automation |
| 14 | activity |
| 20 | operation timer |
| 70 | operation queue |
| 71 | disabled operations |
| 72 | Hero |
| 73 | properties |

Dynamic/nested fields and definition-indexed enum/array serialization prevent inferring byte boundaries from object offsets or ordinal positions. Six arrays use original `WriteIndexHashArray` with the actual UnitAbility, Terrain, Feature or GreatPersonClass definitions; two promotion-class maps use original hash-table serialization. Full nested contracts, index serializer state, exact Linux definitions/configuration, and real fixture slices remain necessary before accepting a complete Unit codec. Do not search for a unit ID byte pattern and declare it a record boundary.

For future native fixture slicing, the exact object boundary is entry to Unit writer `0x2c22d00`, where RSI is the genuine Unit receiver and RDI is its genuine OStream. Match the original identity getter and retain the same original stream and nesting. Capture original stream position before and after that exact call, under the existing synchronized Save owner, to obtain an attributed record. The corresponding queue writer (`0x2da66b0`) receives the actual queue value in RSI. This describes provenance requirements, not an implemented capture hook or verified runtime slice.

## Queue and node wire format

The queue writer emits signed int32 version **1**, unsigned uint32 native queue size, then visits original `GetAt(i)` in ascending queue index. It resolves each node's live numeric `eOperation` through the real operations manager. For each resolved handler it writes that handler's unsigned **operation hash**, then invokes handler virtual Write. The reader resolves the hash, converts it back to the registered numeric operation ID, constructs the real node and invokes its handler Read before appending. Wire hashes and live numeric IDs must remain distinct. An unknown handler must be explicitly unsupported by a future Rust codec; no opaque payload length is established here. The original writer's missing-handler branch skips that entry's body even though it already wrote the queue count; this is not a recoverable opaque-entry format.

SkipTurn's actual virtual Write/Read slots resolve to BaseHandler `0x2d965a0` / `0x2d96630`. Its base node body is:

1. signed int32 node version **2**;
2. directive enum32, signed int32 flags, signed int32 push turn;
3. saved pre-operation activity enum32, then automation enum32;
4. actual shared-parameters virtual Write when nonnull, otherwise signed int32 **0** (`EmptyParameterVersion`).

The base reader fills pre-operation state with two `-1` values for node versions below 2; version 2 reads both fields. It obtains the handler's real parameter instance before reading either that instance or the empty sentinel. No C++ pointers, allocator state, reference counts, or raw 40-byte Node memory belong in the wire representation. Activity, automation and directive writers use the actual 32-bit enum bits through original OStream integer writing. The host integer primitive passes four source bytes to the original stream virtual Write; on this pinned little-endian Linux target those scalar bytes are little-endian. This does not supply formats for all other operation handlers or parameter subclasses.

## Formation effects are serialized gameplay state

The actual Unit stores formation ID at `+0xbb8` and a separate formation pointer; neither should be fabricated from the other. `Formation::Instance` registers 17 variables with its own archive. `ClearPathCache` (`0x293d8e0`) always performs two tracked edits:

| Formation field | Archive index | Wrapper | Inner value | Effect |
|---|---:|---:|---:|---|
| `m_kLastPath` | 14 | `0x1f0` | `0x200` | clear the actual ResultNode vector |
| `m_kLastPathCacheLocation` | 15 | `0x218` | `0x228` | original PlotCoord SetInvalid |

`PlotCoord::SetInvalid` (`0x1fe5510`) writes the eight-byte value `0xffffd8f1ffffd8f1`: both signed int coordinates become **-9999**. Clearing an already empty path or invalid coordinate still marks both wrappers dirty. This clears logical contents; it does not require freeing the vector allocation or zeroing its unused capacity bytes.

Formation writer `0x293b440` includes both path fields, so they cannot be excluded as nonserialized transient caches. It omits `m_bEvaluatingOperation` while preserving the other 16 registered values; its full order is in `formation_write_order`. The current Formation wire format is now statically recovered through the nested helpers: version 1; uint32 ID; PlotCoord (int32 x, int32 y); owner enum32; fixed-point moves as raw int32; domain enum32; eight one-byte bools in the reported order; path array; last cache PlotCoord; UnitID array. The bools are the registered fields 5 through 12. The path array is uint32 count followed by each node in original order. Node writer `0x2a53b90` emits int32 version **1**, raw fixed-point int32 data1, int32 data2, int32 flags, signed int16 x, signed int16 y: **20 wire bytes**, although DWARF identifies a 16-byte object. UnitIDs are uint32 count followed by uint32 IDs. Current canonical record size is `52 + 20 * path_count + 4 * unit_id_count` bytes. Player/domain enums preserve four original bits bytes; no index-serializer conversion occurs for these two Formation fields. The report fingerprints the original coordinate, fixed-point, bool/short, path sequence and UnitID sequence writers/readers.

Formation reader `0x293b560` consumes the same current field sequence, then sets omitted evaluating-operation=false, resolves the real owner player pointer (NO_PLAYER yields null), and sets its archive identity from its actual ComponentID. Path sequence reader `0x2a53d40` and UnitID sequence reader `0x2944450` clear, read uint32 count, reserve, append and read each original element. Node reader `0x2a53bf0` reads version then the same fields. These native readers do not branch on the Formation/node version values; a bounded Rust codec should explicitly reject other versions until evidenced. A wire round trip is distinct from executing native load side effects or restoring complete live gameplay state. Native attributed record comparison is still pending.

Under an accepted immediate SkipTurn with an initially empty queue, no unexpected subscriber effects, and normal deletion, static tracked-state expectations are existing Unit deltas union `{11,32,72}` (when activity changes), and existing Formation deltas union `{14,15}` when a formation exists. These are predictions to compare with native observations, not a native transcript. Automation is unchanged on the observed public empty-map request because `bFromUser=false`; other request flags need separate contracts. Final equal Save bytes would not erase these tracked mutations.

## Remaining acceptance work

The next faithful Rust component is a typed, versioned **Unit/Formation wire-state model** with explicit supported nested codecs and separate archive mutation history. Validate it against attributed original record slices and round trips before using it for action parity. Reject unmodeled handlers, parameter types, versions and gameplay fields without mutation. A standalone activity/queue implementation cannot claim exact SKIP_TURN or complete Unit state.

The transitive gaps in [SkipTurn](linux-skip-turn.md) remain: complete legality inputs/scripts, existing queue handler deactivation, delayed death and continuation, actual formation membership/path state, reporting/cache subscribers, notification expiry/activation and end-turn blocking, diplomacy statement processing and policy decisions, and complete gameplay RNG. Original request/network delivery and synchronized host completion remain separate from this schema. All player decisions must yield externally; AI-namespaced gameplay state and RNG stay in scope. Raw Save→Load differences, including the recorded **89,198-byte failure**, remain reported and unmasked. Native observer purity, complete fixture restoration, and native affected-state comparisons are still required.
