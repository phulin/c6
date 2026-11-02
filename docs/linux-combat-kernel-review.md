# Independent review: Linux XP2 average damage

Accepted for the bounded average-damage arithmetic profile recorded in
[the comparison](../evidence/linux/combat-kernel-parity.json). No remaining
concrete correctness defect was found in this boundary. This acceptance does
not cover complete combat, randomized damage, actor/modifier resolution,
`Engine.apply`, actions or turns; those gameplay operations remain unsupported.

The review compared the Rust implementation with existing private disassembly
of XP2 `CalculateDamage` at `0x2bd8980` and its Q8 helpers. It checked wrapping
subtraction and conversion order, signed division, fractional float conversion,
single-precision evaluation, `CVTTSS2SI` overflow/NaN results, zero-strength
return priority, garrison reduction and minimum-before-maximum caps. The native
multiply's logical 64-bit shift and Rust's arithmetic shift retain the same low
32 bits. The guarded zero-divisor exclusion matches the original instruction
path; its two omitted synthetic cases remain explicit unsupported arithmetic.

The initial candidate's default `expf` selected a different glibc implementation
and was correctly rejected by the binding gate. The reviewed candidate imports
`expf@GLIBC_2.2.5`; its adapter observes that same versioned symbol. Both tested
builds selected offset `95136` in libm SHA-256
`fce00b6f25f459cf4ae0b7fae4257909a1a8a86f9149e7c029a5d617baf1ccd0`.

Acceptance is specific to supplied Linux XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
host SHA-256
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`,
global maximum extra damage `12`, round-to-nearest, MXCSR control bits `0x1f80`
and x87 control word `895`. The original selected `floorf` offset was `492080`.
The runtime verified nine XP2 bindings, six independently audited host Q8
bindings and five direct XP2 entries without overwriting them.

The reported native run completed all 32,364 synthetic records with unchanged
parameter records and the entire observed synchronous RNG object. Its original
Core Saves before and after were identical: 80,657,462 bytes, SHA-256
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`.
Debug and release produced 64,728 total comparisons with zero mismatches.

The review inspected the immutable input/source/configuration/output gates and
independently checked that the comparison's final source hashes and native
profile digest matched the files reviewed. It did not run another original-code
fixture or rebuild the candidate. Native parity and timing results are the
implementation owner's recorded runs; ordinary tests and static inspection
alone do not establish native parity. The corpus and observed profile do not
establish equivalence for every possible parameter or other libm profiles.
