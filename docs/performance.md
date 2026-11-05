# Performance scope

The library executes bounded action and turn sequences, including the
[three-round retained Session case](linux-found-city.md), but cannot execute
complete games or arbitrary turns. There is no full-game throughput or
native-game speedup claim. Numerical loops, codecs, rules loading, state import
and gameplay execution have different costs and must be measured separately.

A single optimized Linux comparison measured the retained sequence on an AMD
EPYC 7642 host with rustc 1.98.1, using public code `eedbad4` and private comparison
code `215f4f8c` (Cargo release, thin LTO, one codegen unit, incremental disabled):

| Measured scope | Elapsed |
| --- | ---: |
| FoundCity request and its synchronous callbacks | 12.706 ms |
| Three ordinary rounds, including inter-round external orders/DEFER responses and rule upkeep | 86.965 ms |
| Whole candidate invocation, including import, setup, comparison and serialization | 2.927 s |

The round timer excludes import, immutable service setup, initial founding,
research/production/Fortify requests, native AFTER comparison and JSON output.
The comparison matched its affected gameplay fields and represented RNG; complete
RNG/runtime coverage remains open. This is one measurement of the bounded
[initialized fixture sequence](linux-found-city.md), not a latency distribution,
complete-game throughput estimate or native-game speedup measurement.

The public RNG microbenchmark runs the implemented primitive directly:

```sh
C6_BENCH_DRAWS=200000000 cargo bench -p c6-core --bench rng
```

It starts at seed `0x12345678`, draws with bound `65535` and reports elapsed
nanoseconds per draw, checksum, final seed and call count. The timed loop includes
its checksum, loop and optimization-barrier overhead. It excludes C ABI calls,
stream routing, snapshot transport, rules loading, World queries and gameplay.
A primitive result is not an estimate of complete simulator throughput.

For comparable measurements, record the exact source/lockfile, compiler,
release settings, CPU, draw count and final state; run repeated processes under
consistent load. Report the distribution and what the timing includes. The
workspace release profile uses thin LTO and one codegen unit. Do not reuse an
older executable's measurements as a result for changed code.

The [C ABI smoke check](../tools/check/c_abi_smoke.sh) verifies interface behavior,
not performance. Public [build checks](../README.md#build-and-check) and bounded
component tests should pass before timing a changed implementation. Complete
state import and bounded gameplay sequences require their own measurements;
existing primitive benchmarks exclude them.
