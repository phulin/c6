# Linux GameModifierAnalysis current wire contract

This is a static, read-only contract for the supplied Linux target
`libGameCore_XP2.so`, SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
It supplies the complete current nested record grammar to the BehaviorTree codec
owner. It adds no production codec, native execution or initialized-game fixture.

The exact writer is `Serialization::operator<<(OStream&,
GameCore::AI::GameModifierAnalysis const&)`, RVA `0x20e3660`, size `0x3e5`.
The matching reader is `Serialization::operator>>(IStream&,
GameCore::AI::GameModifierAnalysis&)`, RVA `0x20e3a50`, size `0x5f3`.
The writer emits signed version **17** at `0x20e3670`. Matching target DWARF
reports a 224-byte native object. Native object offsets below are discovery
identities, not offsets in a saved record.

Run:

```sh
python3 tools/linux/game_modifier_analysis_contract_audit.py \
  --output evidence/linux/game-modifier-analysis-contract.json
```

The audit checks the entire module SHA, 130 method hashes, 204 direct-call or
tail-call edges with exact PLT relocation slots, the version instruction and the
matching ELF's DWARF layout hash. The report contains named methods and addresses,
not archived disassembly. No owner-hook counts, production save offsets or native
fixture coverage are inferred from these static checks.

## Current version 17 grammar

All multibyte values are little-endian. There are **22** definition-hash words,
ordered as follows after the `i32` version. Each is a raw `u32` word on wire;
`WriteIndexToHash` converts the corresponding native index through its collection,
and `ReadHashToIndex` resolves it back. A wire-preserving codec must retain the
hash bits and must not interpret them as a runtime definition index.

| Field | Definition collection | Native offset |
| --- | --- | --- |
| Belief | Belief | 0 |
| Building | Building | 4 |
| Civic | Civic | 24 |
| District | District | 28 |
| Feature | Feature | 32 |
| Handicap | Difficulty | 52 |
| Improvement | Improvement | 56 |
| MinorCivType | MinorCivBonus | 60 |
| Technology | Technology | 80 |
| Unit1 | Unit | 92 |
| Unit2 | Unit | 96 |
| Yield | Yield | 100 |
| Resource | Resource | 76 |
| GreatWorkObject | GreatWorkObjectType | 48 |
| Policy | Policy | 68 |
| Governor | Governor | 36 |
| Government | Government | 40 |
| GovernmentSlot | GovernmentSlot | 44 |
| Project | Project | 104 |
| GreatPersonClass | GreatPersonClass | 108 |
| UnitOperation | UnitOperation | 84 |
| UnitPromotionClass | UnitPromotionClass | 88 |

Continue in this exact order:

| Field | Wire representation | Native offset |
| --- | --- | --- |
| Opponent | i32 PlayerTypes | 64 |
| Religion | i32 ReligionTypes | 72 |
| Count | i32 | 8 |
| CityId | component ID, 8 bytes | 16 |
| StartTurn | i32 | 180 |
| TimeHorizon | i32 | 184 |
| DiscountRate | raw f32 bits, 4 bytes | 188 |
| ExpectedBonusValue | i32 | 192 |
| ExpectedTurnsToMeet | i32 | 196 |
| AnalysisDepth | i32 | 200 |
| UnitList | u32 count, followed by that many ordered u32 items | 152 |
| BestImprovement | raw bool byte | 176 |
| CollectionIndex.FoundItem | raw bool byte | 204 |
| CollectionIndex.IteratorIndex | i32 | 208 |
| City | presence byte, then component ID if nonzero | 112 |
| DistrictInstance | presence byte, then component ID if nonzero | 120 |
| Player | presence byte, then i32 PlayerTypes if nonzero | 128 |
| Plot | presence byte, then i32 plot index if nonzero | 136 |
| Unit | presence byte, then component ID if nonzero | 144 |

A component ID is `owner:i16, type:i16, id:u32`, eight bytes total. The mandatory
`CityId` and optional `City` are distinct fields. City and Unit use typed ID writer
overloads with exactly the same widths as ComponentID. DistrictInstance uses the
ComponentID overload. The optional record has no additional nested version.

With `n = UnitList_count`, the complete record size is:

```text
151 + 4*n + 8*(City_present + DistrictInstance_present + Unit_present)
          + 4*(Player_present + Plot_present)
```

Here each `_present` is one when its presence byte is nonzero. The list count is
at wire offset 136, list items begin at 140, and the first optional presence is
at `146 + 4*n`. CollectionIndex.IteratorIndex2 and IteratorIndex3, native pointer
addresses, padding and allocation state are absent from the record.

The bool primitive copies one byte without normalization; preserve raw bool bytes
when transporting records. Optional presence is generated as 0/1 by the native
writer's pointer comparisons, while the reader branches on nonzero. DiscountRate
copies four float bytes, preserving bit patterns; there is no Q8 field or fixed
point conversion. UnitList is a u32 count plus u32 items, without sorting or
hash conversion. Its native reader clears the vector, reserves and appends in
source order.

## Source pins and load effects

The machine-readable audit names and pins all direct root call bindings and these
nested implementations:

| Boundary | Writer | Reader |
| --- | --- | --- |
| ComponentID | `0x2c9ac60`, size `0x5f` | `0x2c9acc0`, size `0x4c` |
| CityID writer overload | `0x2c9ad10`, size `0x5f` | root uses ComponentID reader |
| UnitID writer overload | `0x2c9adc0`, size `0x5f` | root uses ComponentID reader |
| UnitList | `0x1ff62f0`, size `0x5b` | `0x1ff61f0`, size `0xe`, then `0x1ff6200`, size `0x7d` |
| UnitList count writer | `0x1ff63c0`, size `0x2e` | u32 count read in sequence reader |
| bool raw primitive | `0x1fe7900`, size `0xe` | `0x1fe81a0`, size `0x4b` |
| float raw primitive | `0x20e9e30`, size `0xe` | `0x2009400`, size `0x4b` |

Native loading performs lookups, beyond consuming wire bytes:

- Every absent optional reference leaves the existing destination pointer
  unchanged. The reader does not reset or construct the destination object.
- For present City, it reads ComponentID, checks that its owner is valid and the
  owner player is initialized, then assigns `City::CManager::GetCity`'s result.
  If either guard fails, the existing destination pointer remains unchanged.
- Present DistrictInstance assigns `City::CManager::GetDistrict(ComponentID)`.
- Present Player assigns `Context::Globals::GetPlayer(PlayerTypes)`.
- Present Plot obtains the global map and calls virtual slot `+0x48` with the
  signed plot index, then assigns the returned pointer.
- Present Unit assigns `Unit::CManager::GetUnit(ComponentID)`.

The separate constructor at `0x20e7cc0`, size `0x9c`, initializes the five
pointers to null. Construction can therefore affect a subsequent save after an
absent reference; it must not be silently conflated with reader behavior.
Definition-hash lookup can also depend on the active rules collections. This
research audit does not reproduce these effects or establish Save→Load parity.

The reader contains historical version gates; their thresholds are recorded in
`HASH_FIELDS`. It notably skips Policy completely when the version is exactly 9.
CityId has threshold 2, BestImprovement threshold 4, and AnalysisDepth threshold
5. These are source observations, not a supported historical grammar: consumers
of this contract should explicitly restrict admission to v17 until older versions
have separate fixtures and acceptance checks.

This work establishes a nested current wire schema only. Complete Player/world
state, action execution and turns remain unsupported. It proves no planning-only
exclusion for this AI-namespaced object and masks no unknown save bytes; the
recorded **89,198-byte raw Save→Load difference remains a failure**. All player
decisions must still yield to external policies. Attribution: independent
analysis of supplied pinned Linux ELF symbols, bytes and matching DWARF; no
external reference material or proprietary byte dumps are reproduced.
