# Linux Unit definition identities

This work covers an isolated Linux XP2 definition identity boundary. It does not
implement live Unit loading, definition remapping, actions, turns or complete
state equivalence. One genuine loaded native benchmark context supplies the
observed Unit collection order; coverage remains specific to that context.

The exact supplied GameCore SHA-256 is
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`;
the matching Linux host SHA-256 is
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Discovery used the independently verified Yield mechanism and the pinned
[cru121 reference](https://github.com/cru121/civ6-gamecore-reference/tree/5af389445118bed2bdebf7a8a42b219c8fba5175)
(CC BY 4.0) as leads. Unit conclusions below were checked against the target's
own symbols, bytes and DWARF. Full disassembly, layouts and inventory remain
private under `../c6-ghidra/evidence/linux-unit-definition-ids/`; public evidence
contains bounded fingerprints, counts and gate results.

The target Unit builder at `0x225eec0` calls `BuildDefinition`, obtains the current
collection size, assigns that value as collection index, and appends the new
object. Its SQL getter `0x221c190` independently names `Units ORDER BY rowid`.
The public [static contract](../evidence/linux/unit-definition-ids-static.json)
fingerprints the complete SQL bytes and 23 complete original symbol extents.
This ordering does not imply `rowid = index + 1`. Native zero and gapped rowids
retain their identity; they never become collection indices implicitly.
The signed native rowid getter stores 32 bits while SQLite rowids can be 64 bits;
narrowing extremes remain unexecuted and unsupported by the profile audit.

| Unit definition property | Native field / method |
| --- | --- |
| Database rowid | signed 32-bit at offset 16, getter `0x2114a70` |
| Collection index | unsigned 32-bit at offset 20, getter `0x1fcccb0` |
| Unit type hash | unsigned 32-bit at offset 36, getter `0x1fe5a70` |
| Unit type name | native String at offset 176, getter `0x1fe4b00` |
| Collection size | native 64-bit size, `0x1fe82a0` |
| Index-only lookup | `At(u32)` at `0x1fcd120` |
| Numeric/hash lookup | `Find(u32)` at `0x1ffc150`, `FindIndex(u32)` at `0x290b650` |

The exact DWARF gives Unit size 760 and collection size 48. Its two pointer
vectors begin at offsets 0 and 24. Original Unit-specific push, lower/upper-bound
and unsigned comparators were inspected separately. Numeric lookup below count
selects append-order entries; otherwise lookup searches hashes. Equal-hash
insertion precedes earlier equal entries, so the newest colliding entry wins.
A miss yields null / signed -1. These are static contracts; no collision/mod
fixture has been executed. `SetUnitType(nullptr)` clears name/hash to zero;
an empty non-null string hashes to `0xffffffff`. The read-only observer accepts
existing non-null names and never invokes the setter.

The [wire inventory](../evidence/linux/unit-definition-ids-wire-inventory.json)
uses the complete typed Rust Unit decoder on native serializer-owned slices in
two independently pinned canonical captures. Each contains 1,262 Unit records
and 40 distinct `Unit.e_type_index` hashes. Both inventories agree, and every
record roundtrips exactly. The report-driven adapter constructs the Rust
explicit-order registry from all 116 observed native definitions, compares every
native hash and lookup result, and resolves all 40 Unit hashes in each capture
against that observed benchmark collection. All comparisons pass. The secondary
capture has not had its own active registry queried; identical Unit bytes do not
prove that its entire rules/profile identity equals the benchmark context. The adapter
records its executable, lockfile and unchanged source identities. This field names a Unit definition hash on the wire,
not an already resolved collection index. Other hashes in Unit state belong to
other definition collections and must not be resolved against Units.

The [genuine native observation](../evidence/linux/unit-definition-ids-native-1.json)
contains 116 Unit definitions, indices 0–115 and signed database rowids 1–116.
Every stored hash agrees with original `MakeHash(name)` and `FindIndex(hash)`
returns the observed collection index. The boundary probes pass. This profile
contains no zero/gapped rowid or colliding-name fixture; synthetic gate tests
verify preservation of zero/gaps without claiming native execution coverage.
Its separately identified configuration is
`8d4cbe871ebad270c80b6edcbb0b2a57790f58036b214f085199d8d4e4384c12`.
The framework and client both completed measured cleanup. Canonical Save before
and after the query are byte-identical: 80,657,462 bytes, SHA-256
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`.
This proves the measured serialized boundary, not every live process field.
No action, update, turn or cache initialization is executed by this observer.

The native observer obtains the initialized collection using original
`GetUnitDefinitions` at `0x22147d0`. It calls original Unit getters, `MakeHash`
and `FindIndex` for every definition, and probes numeric index zero, `At(count)`,
an actually absent hash, empty input and lowercase input. It does not build or
edit definitions. Its full native result belongs only in private reports.

The separate runner option `--unit-definition-ids` records its own complete
configuration identity and ABI gate. Acceptance requires named host/GameCore
identities, exact observer source identities, the supported mod-activation
schema, integer returncode zero, configured cleanup, both cleanup booleans true,
the initialized XP2 marker and byte-identical independently identified original
Save captures around the query. Alternate success envelopes are rejected. Frozen runtime source digests remain
in the retained original configuration; the audit separately records its current
postprocessor source digest without relabeling the execution.
The inventory comparison re-audits the original runtime and both captures before
accepting the private report. A loaded benchmark observation remains distinct
from the request-None rules export until input identity is independently linked.

```sh
python3 tools/linux/unit_definition_ids_audit.py \
  --output evidence/linux/unit-definition-ids-static.json
python3 tools/linux/unit_definition_ids_audit_test.py
cc -std=c11 -Wall -Wextra -Werror -fPIC -fsyntax-only native/linux/unit_definition_ids.c
python3 tools/linux/unit_definition_ids_inventory.py \
  --private-output ../c6-ghidra/evidence/linux-unit-definition-ids/wire-inventory.json \
  --output evidence/linux/unit-definition-ids-wire-inventory.json
```

For a genuine registry observation, add `--runtime-report`, `--before-save`,
`--after-save` and `--private-output` to the audit. Then pass that private report
with `--registry-report`, its raw runtime with `--registry-runtime-report`, and
the captures with `--registry-before-save` / `--registry-after-save` to the wire
inventory. Missing original evidence is a validation gap. Gate tests use clearly
synthetic metadata and are not native coverage. No performance claim is made.
The recorded raw Save→Load failure remains 89,198 unclassified, unmasked bytes.
