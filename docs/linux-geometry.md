# Linux stateless coordinate parity

The supplied Linux XP2 target now has fresh original-code comparisons for coordinate sentinel validity and horizontal-wrap hex distance arithmetic. The implementation still covers isolated primitives; these checks do not establish map initialization, map bounds, movement, legal actions, pathfinding or turn behavior.

Run the concrete native/Rust differential entrypoint:

```sh
python3 tools/linux/geometry_compare.py --output evidence/linux/geometry-parity.json
```

The command gates the supplied host, GameCore and runtime libraries against `tools/linux/runtime-gate.json`, builds its separate preload, enters genuine host CRT initialization, intercepts application main and loads the whole original GameCore. It builds and calls the existing `c6-rng-candidate` process adapter for `plot_valid` and `plot_distance`, recording that executable/configuration identity independently of historical Windows corpora. Native module, resolved dependency, source, helper, compiler, preload and complete fixture configuration identities are recorded. `--expected-configuration SHA256` rejects a configuration mismatch before original execution. Missing dependencies, crashes, unavailable symbols, refused supported cases or mismatches fail the comparison.

The recorded result is **7,985 comparisons, zero mismatches**: 289 validity cases and 7,696 distance cases. The compact synthetic corpus is `evidence/linux/geometry-parity.jsonl`; its digest is stored in the summary JSON. It includes sentinel boundaries, both signed integer extremes, negatives, negative/even/odd rows, equal coordinates, zero/negative/extreme widths, both wrap settings, ties between wrapped alternatives, and deterministic full-width integer cases. Invalid map dimensions exercise the native function's arithmetic only. No Rust changes were needed.

| Actual Linux boundary | Address |
| --- | --- |
| PlotCoord constructor `(int, int)` | `0x1fe6b30` |
| PlotCoord::IsValid | `0x1fe2340` |
| Map::GetPlotDistance, coordinates | `0x2c48240` |
| Map::GetPlotDistance, six integers/two bools | `0x2c48300` |
| Hexspace::PlotXToX `(int, int)` | `0x1fe6c30` |
| Hexspace::Distance `(int, int)` | `0x2004ef0` |

Both stateless native distance overloads execute for every distance case and must agree. The fixture constructs both eight-byte PlotCoord objects using the original constructor and verifies every coordinate byte is unchanged after calls. GetPlotDistance uses scalar/const-coordinate inputs and the original arithmetic helpers; no Map receiver or Map fields are synthesized. Target instructions show that height and vertical-wrap parameters do not affect this helper; the fixture supplies height zero and vertical-wrap false. Actual Map::IInstance::GetPlotDistance at `0x2004e60` obtains width/wrap through original instance accessors and uses the same arithmetic helpers, but that initialized receiver path is outside this comparison.

Actual target DWARF confirms eight-byte PlotCoord with its TPlotCoord2<int> base at offset zero. Constructor and validity instructions corroborate two signed integers at offsets zero/four. IsValid tests `x > -9999 && y > -9999`; this is a sentinel check, not bounds validation. Distance preserves wrapping 32-bit intermediate arithmetic and signed choices, including unusual extreme-input results. The Rust implementation's negative-row conversion matches the actual target helper. None of these observations establish direction APIs or gameplay topology.

The pinned [cru121 reference](reference.md) supplied discovery/type leads; Linux symbols/DWARF, the actual target instruction bytes and original execution supplied this profile's evidence. Private raw evidence is `../c6-ghidra/evidence/linux/geometry-plotcoord-dwarf.txt` and `geometry-target-instructions.txt`. Public artifacts contain synthetic inputs/results and identities, not proprietary dumps. The completed RNG source/configuration was kept unchanged.
