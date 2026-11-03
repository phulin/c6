# Performance scope

The library cannot yet execute complete games or turns. There is no full-game
throughput, action latency or native-game speedup claim. Numerical loops,
codecs, rules loading and owned queries have different costs and must be
measured separately.

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
state import, gameplay callbacks and turns require their own measurements once
implemented; existing primitive benchmarks exclude them.
