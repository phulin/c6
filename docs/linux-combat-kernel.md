# Linux XP2 average damage arithmetic

`c6_core::combat::calculate_average_damage` implements the bounded arithmetic
branch of the supplied Linux XP2 damage helper. Inputs already contain resolved
Q8 strengths, damage bounds, defense information and power scaling. Complete
combat, actor modifiers, legal attacks, damage application, randomized damage,
actions and turns remain unsupported. The simulator's combat coverage entry
remains `Unsupported`.

Discovery consulted cru121's pinned CC BY 4.0
[reference](reference.md), revision
`5af389445118bed2bdebf7a8a42b219c8fba5175`, especially the
`Rules::Combat::Instance::DamageParameters` layout and damage function index.
Original supplied Linux XP2 bytes and matching DWARF independently establish
this contract. Raw instructions and DWARF remain private under
`../c6-ghidra/evidence/linux-combat-kernel/`.

Run the static audit and reproduce the synthetic input corpus:

```sh
python3 tools/linux/combat_kernel_audit.py --output evidence/linux/combat-kernel-contracts.json
python3 tools/linux/combat_kernel_compare.py prepare \
  --input target/linux/combat-kernel/cases.txt \
  --output evidence/linux/combat-kernel-input.json
```

The audit gates the complete XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
checks sixteen original method boundaries, fingerprints their bytes and
validates the parameter layout, defense enum and synchronous RNG location in
matching target DWARF. It does not execute original gameplay code.
The native probe verifies all thirteen transitive arithmetic/constructor PLT
dependencies, both global storage GOT bindings and five directly selected
entrypoints. The measured dependency profile contains nine XP2 bindings and six
genuine host weak-template bindings. The host's integer assignment, default
constructor, copy assignment and comparison have complete instruction bytes
identical to XP2; its float conversion differs only in a verified read-only
constant displacement. Its integer constructor directly tail-calls the audited
host assignment. Both complete original module hashes are gated. The probe
requires the exact measured host addresses/module ownership and exact XP2
offsets, and never overwrites bindings. Passive
[binding discovery](../evidence/linux/combat-kernel-binding-discovery.json)
records actual ownership separately from arithmetic acceptance.

The native helper is `CalculateDamage(DamageParameters const&)` at `0x2bd8980`.
Its receiver is unused. The real 48-byte parameter record is initialized by its
original constructor at `0x2be20e0`. It contains no Unit or Map receiver. Even
average mode reads the original global `COMBAT_MAX_EXTRA_DAMAGE`; a context
factory alone does not establish populated rule globals. The native fixture
therefore requires the existing genuine loaded XP2 context and actual activated
rules databases. It does not synthesize gameplay objects or substitute globals.

Average mode uses signed division of the global maximum extra damage by two,
truncating toward zero. The record's `iMaxExtraDamage` and `iPowerDampener` fields
are unused by this target helper. Defender strength zero returns its maximum HP;
otherwise attacker strength zero returns minimum damage. The original function
reads the global rule before those early returns.

For other inputs, the strength difference wraps as signed 32-bit Q8. Scaling
multiplies signed 64-bit raw values, shifts eight bits and retains the low signed
32-bit result. Float conversion separately converts the arithmetic integer part
and unsigned low-byte fractional part, then adds them in single precision. The
helper applies original `expf`, multiplies the single-precision base damage plus
average extra damage, adds `0.5f`, calls `floorf` and converts with x86
`CVTTSS2SI`. Overflow and NaN convert to `INT_MIN`, preserving the instruction's
behavior instead of Rust's saturating cast. Floating equivalence is specific to
the tested Linux x86-64/glibc profile. The Rust component explicitly calls the
measured `expf@GLIBC_2.2.5` compatibility symbol; other target platforms return
`UnsupportedMathPlatform`. The oracle records actual `fegetround`,
MXCSR and x87 control words before and after the corpus, without changing FP
controls. It reads the actual original `expf` and `floorf` GOT bindings, whose
PLT jumps and ELF relocations the static audit verifies, and requires both to
belong to the same runtime module. The comparison hashes that module and the
candidate's resolved libm, requires identical selected `expf` implementation
offsets within that module, and requires matching FP control bits. This avoids
assuming that compatibility/default symbol versions or IFUNC selections within
the same libm file implement identical arithmetic. MXCSR exception
status bits are recorded separately from controls and may change during math.

For `DISTRICT_DEFENSE_GARRISON` (`0x5e97d629`) without defense bypass, damage is
reduced using the Q8 fraction of remaining outer defenses. Integer-to-Q8
conversions, subtraction, multiplication and conversion back to integer retain
their native wrapping and rounding. Fixed-point division truncates signed
64-bit division toward zero. Some synthetic positive outer-defense maxima wrap
to a zero Q8 divisor; Rust explicitly returns `ZeroFixedPointDivisor` where the
original would divide by zero. These cases are excluded from the combined native
run and counted in its input manifest. Minimum damage is tested before the
maximum cap, preserving unusual contradictory-bound inputs.

`native/linux/combat_kernel.c` forces average mode for every case, snapshots the
entire actual 32-byte synchronous Random object at game offset `248`, and checks
that record, each complete parameter record and global pointers/rule values
remain unchanged. The loaded fixture must separately compare original Core Save
outputs before and after the probe. Average arithmetic consumes no RNG. The
nonaverage original branch instead calls `Game::GetRandNum` before either
zero-strength return; that branch edits the actual synchronous Random through
its archive wrapper and remains unimplemented in Rust.

The corpus contains 32,364 parameter records covering dense fractional-strength
differences, rounding transitions, zero strengths, min/max caps, garrison and
other defense types, bypass, outer-defense extremes, unused-field variations,
signed integer extrema and deterministic full-width arithmetic. Python's random
generator only selects synthetic test inputs.

After the separately gated loaded-context fixture produces a private native
observation file and a successful runtime profile, compare both Rust build modes:

```sh
python3 tools/linux/combat_kernel_compare.py compare \
  --input target/linux/combat-kernel/cases.txt \
  --manifest evidence/linux/combat-kernel-input.json \
  --native-output PRIVATE_NATIVE_OBSERVATIONS \
  --native-profile GATED_NATIVE_PROFILE \
  --output evidence/linux/combat-kernel-parity.json
```

The comparison rejects source/input changes, incomplete native output and
nonzero native exit status. It also requires recorded RNG/parameter preservation,
unchanged original Core Saves, matching configuration identity and linked output
digests. It preserves mismatches with complete synthetic
inputs. Debug and release candidate executable identities are recorded. A
release benchmark repeats only the already parsed arithmetic calls, excludes
startup, parsing and I/O, and checks its aggregate result against original
observations. This measures the implemented average-damage component; it says
nothing about full combat or turn performance.

The [accepted comparison](../evidence/linux/combat-kernel-parity.json) covers
**32,364 unique original parameter cases**, tested in debug and release for
64,728 comparisons with zero mismatches. The loaded global extra-damage rule is
`12`. This is the measured runtime with libm SHA-256
`fce00b6f25f459cf4ae0b7fae4257909a1a8a86f9149e7c029a5d617baf1ccd0`,
actual `expf`/`floorf` offsets `95136`/`492080`, rounding mode zero, MXCSR controls
`0x1f80` and x87 control word `895`. MXCSR exception status changes are recorded;
controls remain unchanged. Both original 80,657,462-byte Core Saves and the entire
actual synchronous Random record remain unchanged across all queries. Other
global rule values and math runtimes have no native parity claim here.

The initial default Rust exponential resolved to a different implementation in
the same libm and was rejected by the dependency gate. The versioned binding
corrects that candidate runtime dependency without changing original bindings or
FP state. The [original native input manifest](../evidence/linux/combat-kernel-native-input.json)
is retained separately from the current candidate manifest; immutable native
sources, configuration and observations remain private at
`../c6-ghidra/evidence/linux-combat-kernel/native-3/`. The earlier unavailable
oracle attempts are preserved as [attempt one](../evidence/linux/combat-kernel-attempt-1.json)
and [attempt two](../evidence/linux/combat-kernel-attempt-2.json), each with zero
original arithmetic cases.

The accepted optimized executable processed 3,236,400 arithmetic calls in
0.0890 seconds, approximately **27.5 ns per call** on this corpus. That timed
loop uses the same executable and cases as release parity, excludes startup,
parsing and I/O, and verifies its aggregate result against original observations.
It does not measure full combat or simulator throughput. Workspace tests,
clippy with warnings denied, and the C ABI smoke check pass. See the
[independent review](linux-combat-kernel-review.md).
