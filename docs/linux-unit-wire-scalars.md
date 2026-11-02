# Linux Unit scalar and container wire contracts

This is static current-writer evidence for the pinned Linux game module
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and
matching host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
It does not establish native record parity, historical reader compatibility,
complete save loading, or complete state/action/turn support. The implementation
still covers isolated primitives.

[Machine evidence](../evidence/linux/unit-wire-contracts-scalars.json) records
bounded function fingerprints and symbolic calls. Reproduce it with:

```sh
python3 tools/linux/unit_wire_contracts_scalars.py \
  --gamecore /path/to/libGameCore_XP2.so --host /path/to/Civ6 \
  --output /tmp/unit-wire-contracts-scalars.json
```

`--symbols` accepts cached `nm -C` output. SHA gates apply to both actual binaries
before analysis. `--private-disassembly` can preserve raw instructions outside the
public checkout. Research used pinned target bytes and matching symbol names;
reference material is a discovery lead rather than cross-platform proof.

All primitive writes pass raw little-endian bytes to the stream: bool and unsigned
char use one byte, short two, int and unsigned int four. The bool writer itself
does not normalize a source byte. Unit enum `FAutoVariable::save` methods dispatch
the payload at wrapper offset `+0x10` to the corresponding scalar writer without
a header. The special `EnumToStream<FAutoVariable<PlayerTypes,...>>` at `0x2c29f30`
likewise writes the payload as i32.

| Enum family | Wire representation |
| --- | --- |
| DirectionTypes, GreatWorkListIndex, HiddenVisibilityTypes, MilitaryFormationTypes, PlayerTypes, RelicSource, UnitAITypes, UnitActivityTypes, UnitAutomateTypes, YieldTypes | i32 ordinal |
| BuildingTypes, DistrictTypes, GreatPersonClassTypes, GreatPersonIndividualTypes, HeroClassTypes, ReligionTypes, TechTypes, UnitOperationTypes, UnitTypes, ImprovementTypes, ResourceTypes, RouteTypes, WMDTypes | u32 definition hash |

`YieldTypes` at `0x2d4fb70` is an explicit exception to any naming-based assumption
that a definition enum must serialize a hash. Its nested
`InitiationYieldPopulation` field uses i32; a YieldIndexList uses hash-array framing.

For definition enums, `WriteIndexToHash` writes the result of `IndexToHash` as u32.
The Unit specialization at `0x2005d70` first compares its u32 input to definition
count. An in-range input becomes `At(index).GetTypeHash()`. An out-of-range input
is preserved if `Find(input)` recognizes it as an existing hash; otherwise the
result is `0xffffffff`. Unit `HashToIndex` at `0x1ffc120` returns the found
collection index or `-1`. These are runtime rules-collection identities, not
stable cross-configuration ordinals.

| Helper | Current wire contract |
| --- | --- |
| Sight::PlotEntry vector `0x2c38680` | u32 count, then each PlotEntry in begin/end order; each entry is i32 sight-count then u32 plot index |
| EspionageBonus vector `0x2c167a0` | u32 count, then each component record in begin/end order |
| EspionageTimeReduction vector `0x2c16800` | u32 count, then each component record in begin/end order |
| InitiationYieldPopulation vector `0x2c442d0` | u32 count, then each component record in begin/end order |
| Types::Vector<int> `0x1feff10` | u32 count, then i32 elements in begin/end order |
| YieldIndexList<int> `0x2194800` | Yield definition hash array of i32 |
| OperationIndexList<bool> `0x2c3fc20` | UnitOperation definition hash array of one-byte bool |
| CityID `0x2c9ad10`, UnitID `0x2c9adc0` | i16 owner, i16 component type, u32 ID |
| PlotCoord `0x2c9af20` | i32 x, i32 y |
| FixedPoint8 `0x2c9afe0` | i32 raw fixed-point bits |
| String Encoding1 `0x264bed0` | u32 GetLength(), then exactly that many raw buffer bytes; no terminator |
| StaticArray<int,5> `0x2c48940` | five i32 values, no count |

The vector count constructor truncates `size()` to its low 32 bits before writing.
There is no element-length prefix; nested component versions and fields come from
[the component writer evidence](../evidence/linux/unit-wire-contracts.json).
The string contract does not establish UTF-16 encoding.

Definition hash arrays emit u32 definition count, then every `At(i)` in increasing
index order as `(u32 GetTypeHash(), typed value)`. They include zero/false values.
Ability and GreatPersonClass values are i32; Terrain, Feature and UnitOperation
values are one-byte bool. Yield values here are i32. Rules count and ordering must
come from the exact Linux rules input, not a synthetic count inferred from state.

The two Unit promotion tables instead call `0x2c2a190`, a
`Types::HashTable<unsigned char>` writer: u32 table size followed by `(u32 key,
u8 value)` for each native forward-iterator entry. Values are unsigned bytes,
not normalized bools, and the count is table size rather than definition count.
The writer preserves native iteration order; sorting keys is not justified.
