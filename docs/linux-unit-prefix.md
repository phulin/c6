# Linux Unit prefix boundary

`Unit::decode_prefix_with_items(bytes, limits)` decodes one supported version-28
wire record at a caller-supplied structural cursor and returns `(unit, consumed,
items)`. `decode_prefix` omits the item charge. The parser never receives a native
trace endpoint. A large untouched Core tail does not consume the byte budget:
input is clipped to that budget, and clipped truncation, including nested variant
truncation, reports `ResourceLimit`. Other semantic errors retain their identity.
Unclipped truncation retains the existing strict parser error.

`encode_with_items` returns `(bytes, items)`. Both directions report the exact
aggregate charge already enforced by the shared parser/writer, including nested
containers, string bytes and operation queue nodes. An enclosing roster must
subtract each charge from its remaining item budget. Existing variant allocation
and depth policy remains in force; no new allocation policy is claimed.

Strict `decode` and `decode_with_ranges` still gate the full supplied byte length
and require exhaustion. All entry points share the same field grammar, version,
handler and variant gates. Fixed per-record field-range collection remains.

Reproduce the read-only audit from a committed source identity:

```sh
python3 tools/linux/unit_prefix_audit.py --source-commit COMMIT --output target/linux/unit-prefix.json
```

The checker builds only an isolated archive of that commit and rejects a checker
script that differs from it. It gates the two immutable Linux Core streams and
native serializer traces used by [the strict codec checker](linux-unit-codec.md).
Every native Unit is decoded from its Core tail, compared with independently
traced boundaries, reencoded byte-for-byte, checked at exact and insufficient byte
and item budgets, and checked for strict trailing rejection. All short/clipped
prefixes of the first native record in each stream are rejected. Concatenated
records check cumulative exact and one-short item budgets. The original strict
Unit/Formation/queue roundtrips and nested-boundary checks run at one iteration.
No original game code executes.

This is a wire transport boundary, not complete live Unit state or gameplay.
Unsupported operations/variants/versions still reject. Native domain, definition
fixups, live pointers and archive dirty history remain unsupported; no actions or
turns are implemented. Target module SHA-256 remains
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
The raw Save→Load→Save failure remains **89,198 bytes**, unclassified and unmasked.

The [recorded audit](../evidence/linux/unit-prefix.json) freezes source commit
`cf0fe89a94657ba2fc16f2d5858ae41014d132cc`, with hashes for all 74 recorded
source/manifests and the generated adapter. Both streams pass all **2,524 Unit
prefixes**; each contains 6,137,501 Unit bytes and charges 780,570 aggregate items.
Each first record supplies 4,870 clipped byte limits and 4,870 truncated inputs.
Both cumulative exact and one-short budgets pass their expected outcomes. The
archived strict checker passes 2,524 Unit, 2,524 queue and 2,500 Formation
roundtrips, including 17,668 nested Unit boundary comparisons. All 18 focused
Unit/component/variant Rust tests pass. The wrapper also passes c6-core library
clippy with warnings denied. No complete gameplay equivalence follows from these
wire checks.
