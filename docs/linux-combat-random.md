# Conditional Linux randomized damage

`combat_random` adds a bounded candidate for the original nonaverage damage
helper and its named `Game::Instance::m_kSyncRandom` transition. The
[accepted comparison](../evidence/linux/combat-random-parity.json) covers 80
continuous native transitions on an already-dirty sync wrapper, tested in debug
and release for 160 comparisons with zero mismatches. Complete combat
actors/modifiers, attacks, actions, turns, full live
native layout and complete gameplay RNG ownership remain unsupported. The
accepted [average arithmetic profile](linux-combat-kernel.md) is unchanged.

The original `CalculateDamage(DamageParameters const&)` resolves the global
maximum extra damage, truncates it to unsigned short, calls `Game::GetRandNum`,
then tests zero strengths. The Game draw edits the real Random wrapper before
calling Random::Get. If attached, edit inserts that wrapper into its owner's
unique changed-variable tree. One draw advances wrapping seed and call count;
resets, live lock count and the raw synchronous byte remain unchanged. Locks do
not suppress draws. Both zero-strength early returns still perform the edit and
draw. See the [ownership inventory](linux-rng-inventory.md); it is not exhaustive.
Internally Random::Get increments calls, computes the new seed/sample, invokes
logging, then stores the seed. The conditional component compares completed call
boundaries and requires normal non-reentrant logging/allocation completion. It
does not claim identical observations inside callbacks or atomic behavior for
failed logging or allocation. The oracle uses original loaded services.

The static audit fingerprints the actual target prefix and verifies the relevant
instruction order: save signed base, truncate bound, call draw, zero-extend its
u16 result, test zero strengths, wrapping i32 ADD, then one signed CVTSI2SS.
The Rust candidate supplies that identical wrapped base to unchanged accepted
average arithmetic with explicit zero extra. The following Q8/FP instructions
and early-return results are retained. This is a CFG-grounded reuse of the common
arithmetic; the native oracle always calls the real nonaverage helper with its
actual global rule. It does not substitute a global rule or a Game object.

The conditional API receives logical live RNG fields and caller-supplied archive
ordering keys. These keys must preserve the actual native pointer comparator;
they are neither pointers nor registration IDs. The fixture reads the original
registration vector and changed-variable tree, validates original node links,
membership and pointer order, and derives a pointer-to-rank mapping. It checks
the sync wrapper belongs to the actual Game archive and that the archive's class
instance is that Game. Rust compares the logical ordered set under this validated
mapping. Native allocation, RB-tree shape/colors and absolute pointer identity
are outside the Rust layout claim. Full original raw snapshots remain private.

Malformed key order or an unsupported math platform is a preflight rejection
without mutation. A zero fixed-point divisor is different: the original would
trap **after** archive editing and drawing. Rust returns
`NativeArithmeticFaultAfterDraw` with that prefix retained. This explicitly
models the prefix, not a returned C++ error or a successful numerical result.
Trapping inputs are excluded from the initial native corpus and never counted as
passing parity cases.

The small initial corpus has 80 calculations: early returns, caps, defense/bypass,
integer extremes and repeated equal-strength cases exposing the sample directly
in the result. It consumes one continuous genuinely loaded stream without seed
injection, Reset, synthetic fields, commands or policy choices. It records the
actual rule, complete 32-byte Random representation, wrapper metadata, logical
dirty membership/order and private archive/node snapshots for every calculation.
The accepted glibc compatibility `expf` binding, runtime module and actual FP
controls are re-observed because original startup preparation can change the
runtime environment.

Two original Save controls surround the operation: a no-draw baseline control,
and a post-draw Save followed by the ordinary repeated Save. The comparator
reports every raw baseline-to-postdraw byte difference privately, without masks.
Dirty membership and live locks remain separate from the 18-byte normal Random
record. Named-stream agreement does not establish complete state import,
branch restoration or semantic ownership of other save bytes. The earlier raw
89,198-byte Save→Load failure remains open.

Prepare the static contract and synthetic requests without original execution:

```sh
python3 tools/linux/combat_random_audit.py --output evidence/linux/combat-random-contracts.json
python3 tools/linux/combat_random_compare.py prepare \
  --input target/linux/combat-random/cases.txt \
  --output evidence/linux/combat-random-input.json
```

The separate native fixture requires a source/configuration-gated genuinely
loaded Core and original host services. Its new private outputs contain original
observations, baseline/postdraw Saves and raw byte differences. No unavailable
fixture, static audit or Rust-only test counts as native gameplay parity.

The accepted native configuration is
`664dae1b08dceb2f38fda01df8f35693ad0ae896c4b353e9600c9264b77e54d3`,
with original early ASL main-thread preparation and complete owner cleanup. The
actual global rule is `12`; samples span `0..11`. Actual `expf`/`floorf` offsets
are `95136`/`492080`, with the same pinned libm implementation, rounding mode zero,
MXCSR controls `0x1f80` and x87 control word `895`. The candidate uses the measured
`expf@GLIBC_2.2.5` binding. This remains runtime-specific parity.

The original Game archive has six registered variables. Its initial changed set
already contains the sync wrapper, so all 80 native edits exercise deduplication
and preserve dirty membership/order. **Absent-to-present insertion has static
instruction evidence and a Rust unit test, but no native acceptance fixture.**
Likewise, the explicit post-draw fault outcome has no passing native trap case.
Normal-return seed/call changes, resets, live locks, raw synchronous byte, opaque
Random bytes, wrapper metadata and logical ordering all pass the current fixture.

Both baseline/control Saves are byte-identical; both post-draw/repeat Saves are
byte-identical. Each is 80,657,462 bytes. The 80 draws change **five raw bytes**
between baseline and post-draw output. Every differing offset and byte is retained
privately, without masking or claiming full serialized ownership. The new
post-draw Save hash is
`4d07dea8e5ac29581ee9b92ad4745fbc298631901f355089ce22d5ce3ee5f253`.
Immutable original observations, runtime/source identities and byte differences
are local under `../c6-ghidra/evidence/linux-combat-random/native-1/`; they are
separate from earlier approved private upload manifests.

Workspace tests, clippy with warnings denied, the C ABI smoke check, static target
audit and native observer syntax checks pass. This checkpoint establishes the
named conditional transition above; it does not promote combat or full gameplay
RNG inventory to complete support.
