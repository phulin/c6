# Independent movement admission-cache review

The movement-only binding cache in `21b5993` passes this static and lifetime review. The closed-profile comparison below now also establishes bounded observed query/helper invariance. It supplies no engine performance or complete gameplay acceptance. The original helper `d13acf6c...` and Rust candidate `d4dddd41...` remain unchanged.

The [metadata](../evidence/linux/oracle-admission-cache-review.json) pins the reviewed sources and [offline audit](../tools/linux/oracle_admission_cache_audit.py). Baseline `e7d4c394...` was independently checked against the actual accepted cost runtime's archived Game loader and lifecycle source hashes, rather than assumed to represent that run.

## Handle and owner lifetime

`mod_probe_main` calls `c6_linux_game_load` synchronously while retaining the genuine Context; session leave and framework shutdown follow its return. The loader successfully obtains `dlopen(path, RTLD_NOW|RTLD_NOLOAD)` before movement preparation. Its sole matching final `dlclose` follows the callback, outer repeat Save and ShutdownGame. The early close for missing loader symbols occurs before admission can begin.

`RTLD_NOLOAD` returns a resident object's handle rather than loading a new object, and successful opens participate in reference counting. Thus this explicit held reference protects the Core symbols for the whole admission scope. [Linux dlopen manual](https://man7.org/linux/man-pages/man3/dlopen.3.html). Host bindings belong to the process's main ELF executable, which remains present through the intercepted synchronous main. The reviewed caller has no unloading or rebinding path inside this scope.

The cache neither owns nor extends native object lifetimes. It records the module handle, borrowed Context pointer, initial vptr and module base as scope identities. Every idle gate repeats the original Context owner/name check and compares all four identities. The actual loader/framework call chain provides their lifetime; this is deliberately not a concurrent, reentrant or cross-unload service.

## Immutable addresses and mutable observations

The twelve original bindings are eleven function addresses and the address of the active-controller global. The global's current value is not cached. `admission_begin` resolves the same symbols in the same order, retaining exact host addresses, Core RVAs and `dladdr` owner checks. It publishes the completed local binding structure only after every binding passes. Missing or nested admission remains fatal.

The native guard body is source-identical to the accepted baseline after removing only `b->` qualification. Each invocation rereads active controller, bound Context, controller/Context initialization, executing and pending state with the original short-circuit order. It obtains an original Game facade, calls the same work predicates, releases that newly acquired wrapper and repeats the identity/idle checks. Neither controller values, predicate results nor facades survive a call. The native facade lock acquisition/release barrier remains intact.

`admission_end` clears every cached field after the final idle and identity checks and before Uninitialize. An ordinary probe error still follows those checks and clearing before returning. Trace end, outer repeat Save, ShutdownGame and `dlclose` occur later. All unknown/live or invalid-scope paths still terminate the isolated process with `_exit(184)`; none returns into poststate or teardown. No returning path with an active cache was found.

The audit also confirms unchanged loader code beyond the cache and unchanged lifecycle code outside the movement branch. Only the movement lifecycle begins/ends admission; the immutable movement helper's API is unchanged. Other action profiles do not enter this scope.

## Validation and limits

The source-equivalence audit and strict C syntax check pass. Independent temporary-copy negative tests demonstrate rejection of a removed pending check, a changed binding and missing cache invalidation. Both frozen helper/candidate hashes were independently verified without rebuilding or running them.

The source preflight required a fresh original fact control and full cost corpus, comparison of observed facts/results, unmasked Core streams, both observed Random objects and actual cleanup/binding observations. The closed-profile review below checks that boundary. Timing must distinguish admission overhead from native query latency. No original game execution, performance measurement, saved-Begin work, Windows work, Ghidra change or private push was performed for this review. Complete gameplay state/actions/turns and gameplay RNG ownership remain outside the cache review; the separate 89,198-byte Save-to-Load failure remains unresolved and unmasked.

## Closed cached-harness comparison

Independent reproduction of `e0a45e6` produced the exact [cache result](../evidence/linux/movement-cost-cache-results.json) SHA-256 `3d2b5681...`. The [new review metadata](../evidence/linux/movement-cost-cache-review.json) records this acceptance separately from the immutable source preflight and original movement report. Only the frozen Rust candidate ran during review; no original game process or build was launched.

The cache control/cost profiles closed normally with native controller, framework and client cleanup. Their canonical whole configurations match the expected accepted baseline plus only the exact reviewed cache sources/audit, runner, integrated preload and cache metadata. Cache control and cost differ only in movement mode. All 60 archived source files per new profile and the runners/preloads verify; both older 59-file source sets and accepted reports remain unchanged. Candidate, source manifest and native helper identities remain pinned.

All 1,536 rows retain native order and identity. Facts, all five registries, scalar outputs and all nine phase snapshots match the accepted baseline; an independent phase replay and frozen Rust comparison report zero mismatches across 13,824 phase states. Every candidate result row and equality flag was additionally checked, beyond its summary. All six negative-layer rows remain present. The scan remains truncated at 256 formations.

Each of six whole Core streams from all four profiles was compared with the accepted pre-query stream: 24 overlapping comparisons, zero differing bytes and zero masked bytes. The complete two 32-byte RNG endpoints match within every process. Across processes only proven seed/calls/resets/locks fields and the raw sync byte are compared; vptr and padding remain retained in raw endpoint data and are not declared equivalent or excluded gameplay state. Negative checks reject an unrelated configuration change, a changed reviewed source hash and a changed unclassified RNG padding byte.

This accepts observed query/helper behavior invariance under the reviewed cache change. It does not prove intermediate purity, complete affected state/RNG, world-query reconstruction, legal movement/actions, turns or engine speed. It adds no simulator mechanic and leaves the earlier corpus coverage gaps and 89,198-byte Save-to-Load failure unresolved. Raw data remain local; no private push or frozen-binary refresh occurred.
