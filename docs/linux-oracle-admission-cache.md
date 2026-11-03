# Movement oracle admission binding cache

This source change removes repeated dynamic symbol lookup from the Linux movement
fixture's idle gate. It has **not received native acceptance or a measured speedup**.
It adds no simulator mechanic. Complete gameplay state, action execution and turns
remain unimplemented; the accepted bounded movement arithmetic corpus remains
separate evidence.

`native/linux/game_load.c` resolves the same twelve original function/data symbols
once per explicit movement probe, preserving their order, absolute host addresses,
Core RVAs and `dladdr` owner checks. Only immutable addresses and scope identities
are retained. Active controller values, bound Context values, predicates and Game
facades are never cached. Each gate still runs the original `dladdr`/Core filename
check and additionally compares the exact module handle, Context pointer, vptr and
module base with the admitted scope.

Each gate retains the original short-circuit evaluation and native call order:
read active controller; GetContext; compare active/bound identity; check controller
and Context initialized, controller not executing and no pending messages;
GetGame; query NeedsUpdate, NeedsTurnAdvance, IsAiProcessing and IsTurnProcessing;
release the acquired facade; GetContext again; repeat identity and idle predicates,
then reject any observed work. No facade survives the gate. Every preexisting
unknown/live failure still calls `_exit(184)` before subsequent state observations,
Save or teardown. Invalid/nested/out-of-scope admission also exits immediately.

The lifetime is synchronous and movement-only:

1. `mod_probe_main` in `native/linux/mod_activation.c` creates/configures the genuine
   framework Context, attaches the saved session, then calls `c6_linux_game_load`.
   Session leave and framework shutdown follow that call's return.
2. `c6_linux_game_load` takes a successful `dlopen(path, RTLD_NOW|RTLD_NOLOAD)`
   reference to the Context vptr's Core module. This reference remains held across
   StartupGame, Load, the initial Save and the whole movement lifecycle call.
   Ordinary successful `dlopen` reference counting also applies with `RTLD_NOLOAD`.
   The intercepted host main does not run during this synchronous probe; the
   executable's host bindings cannot be unloaded as a shared object.
3. `action_lifecycle.c` retains its original Initialize, InitializeCache and
   post-cache controller/context checks. It explicitly begins admission before
   the first idle gate, runs the existing movement probe and final idle gate,
   then retains its final controller/context checks.
4. Admission is cleared before native Uninitialize, including when the movement
   probe returns an ordinary error. The subsequent trace end, outer repeat Save,
   ShutdownGame and `dlclose` all occur after invalidation. Fatal paths terminate
   the process without teardown. Admission never spans an Uninitialize or unload.

The existing fixture is single-threaded at this caller boundary and requires
original worker quiescence; the cache is deliberately not a concurrent/reentrant
service or a synchronization replacement. The game module and genuine owner must
remain alive throughout the probe. This changes no other action profile.

Offline validation:

```sh
python3 tools/linux/oracle_admission_cache_audit.py
cc -std=gnu11 -Wall -Wextra -Werror -fsyntax-only native/linux/game_load.c native/linux/action_lifecycle.c
git diff --check
```

The audit compares the twelve binding identities and exact mutable guard body
(allowing struct qualification only) against committed baseline
`e7d4c394c04be70ff6526e6442890d03d3eff84f`. It also checks the unchanged remainder
of game loading, unchanged lifecycle outside the movement branch, explicit lifetime
ordering and cache clearing. These checks passed without loading original code.
They establish source preservation only; they do not prove a live provider binding,
native behavior equivalence, whole-state equality or performance.

Before accepting the change, the sole native runner must freeze/review the revised
source/build/profile independently, then run a separate control and the same full
1,536-edge corpus under the exact Linux XP2/host/dependency/configuration gates.
Compare every exported fact/result against the accepted corpus, all six whole
Save streams without masking unknown bytes, both observed complete RNG objects,
native binding observations and clean lifecycle boundaries. Keep the previously
accepted binaries and evidence immutable. A source change requires fresh profile
and executable identities. Timing must separate admission overhead from native
query latency; neither speed nor native query latency is measured here. The
recorded 89,198-byte Save→Load difference remains unresolved and unmasked.
