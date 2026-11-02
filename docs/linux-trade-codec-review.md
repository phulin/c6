# Independent Linux Trade codec review

Reviewed implementation commit `837851ce72e8a6489839b269edccfc66b24995c6`:
`crates/c6-core/src/trade.rs`, its example adapter, fixture checker and codec
documentation. No correctness blocker was found for the documented extracted
current-version wire codec. This is isolated serialization support, not complete
Civ VI state/action/turn support.

The review compared the Rust field order and signed widths with the independent
Linux serialization contracts and Python fixture decoder: Graph 1, Edge 3,
Route 4 and Vertex 1; signed 16-bit ComponentID owner/type and unsigned 32-bit ID;
raw signed 32-bit yield values; unsigned path/yield counts and signed graph-map
counts. The checked minimum sizes are 20 bytes per origin and 126 per empty-path,
empty-yield destination/Edge. Byte and aggregate item budgets are resource policy,
separate from native gameplay semantics.

Graph vectors retain empty origins and observed outer/inner order. Auxiliary
indexes never drive encoding; immutable graph access prevents invalidating them.
Keyed equality retains every payload field and path/yield order, while separate
comparisons detect wire-order changes. Duplicate keys/IDs/yield hashes, endpoint
disagreement, noncanonical flags/type words and unsupported versions fail rather
than being repaired. These are supported-format restrictions, not assertions
that native readers reject the same malformed inputs.

Independent checks, recorded with exact source and executable hashes in
[the review evidence](../evidence/linux/trade-codec-review-1.json), passed:

- `cargo test -p c6-core trade::`: all five tests, including independent Route
  wire words, signed boundaries, every truncated graph prefix, malformed versions,
  counts/flags/types, duplicate guards, empty membership, ordering and exact
  resource-budget boundaries. Duplicate-origin/destination and endpoint guards
  are exercised through construction; duplicate edge IDs also through decoding.
- `python3 tools/linux/trade_serialization_audit.py --output /tmp/c6-trade-review-static.json`:
  freshly verified the supplied ELF SHA-256 and 26 native method byte identities;
  output equals the committed static evidence.
- `python3 tools/linux/trade_codec_check.py --private-root ../c6-ghidra --iterations 10 --output /tmp/c6-trade-codec-independent-review.json`:
  both hash-gated 80,657,462-byte streams and serializer traces passed. Each
  469,638-byte graph roundtripped exactly, with 179 origins, 1,571 edges and all
  1,571 native Route boundaries matched. Keyed payloads compare equal; wire order
  remains different. Independent raw graph comparison retains 89,156 changed bytes.

No original game code was executed or rebuilt for this review. The fixture checker
pins complete streams/traces and records source/executable hashes; its module SHA
attributes existing fixture provenance rather than constituting a fresh native
execution gate. The separate static audit checks the supplied module itself.
This review accepts no newly initialized configuration, DLC/mod profile or native
fixture. Reported timing samples cover codec/index work only.

The five synthetic tests cover codec behavior beyond the observed fixture; they
do not supply native gameplay acceptance for arbitrary values. Native yield-hash
definition remapping is not implemented by retaining wire pairs. Native graph
ownership/load initialization, ID-map callback order and Route `m_bFinalizing`
remain outside this codec. No whole-engine coverage or future-turn equality follows
from keyed equality. The complete raw Save/reload/Save comparison remains an
89,198-byte failure, with the other 42 changed bytes still outside this contract.
