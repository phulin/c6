# Linux Trade graph wire codec

`c6_core::trade` implements the extracted current Linux XP2 Graph wire contract
(Graph 1, Edge 3, Route 4, Vertex 1) documented in
[the target serialization contracts](linux-trade-serialization-contracts.md).
This is one state-serialization component of the intended exact headless simulator.
It does not implement trade gameplay, initialize native containers, load complete
Civ VI saves, execute player decisions or advance turns.

The typed graph retains outer origins (including empty ones), inner destinations,
native edge/route/component IDs, ordered full paths, ordered yield definition
hashes, and signed raw fixed-point values. Encoding follows those vectors exactly.
Auxiliary Rust hash indexes provide endpoint and ID lookup and never determine
serialization or simulation iteration. No edge IDs are renumbered.

`logical_eq` compares the total-created counter, origin membership and every keyed
serialized Edge payload, retaining path/yield order within payloads.
`iteration_order_eq` compares outer/inner vertex order independently of payloads.
`wire_state_eq` compares both. Logical equality establishes neither live-state
equality nor future callback order, RNG consumption or gameplay equivalence.
Native `IterateRoutes` traverses an ID map whose order is not directly serialized
by Graph Write. The codec preserves observed wire order but does not reconstruct
that missing live order. Route `m_bFinalizing` is also omitted from the wire.

`Graph::decode` and `Route::decode` require the entire input to be exactly one
record. Unsupported versions, truncated or trailing bytes, duplicate graph keys,
duplicate edge IDs, duplicate yield hashes, noncanonical bool/type encodings and
inconsistent indexed endpoints return errors. Endpoint consistency corresponds
to the target `Graph::Validate` contract. These are restrictions of the supported
valid wire representation, not claims that native Read rejects each malformed
input. The codec preserves any signed 16-bit vertex kind without inventing enum
semantics. It does not infer limits on IDs, counters, traversal indices, path
values or yield quantities from the observed fixture.

`Limits` separately bounds input/output bytes and the aggregate number of origins,
destinations, path entries and yield pairs. Defaults are 64 MiB and 1,000,000
items; callers may supply other policies. Each decoded count is also checked
against the remaining minimum wire size before vector allocation. These are
resource policies, not engine gameplay limits. No unsupported operation mutates
an existing graph; graphs expose immutable views and are rebuilt through validation.

Run synthetic integer-boundary, malformed-input, ordering, empty-origin,
duplicate, lookup and resource-boundary checks with:

```sh
cargo test -p c6-core trade::
```

The private-fixture check performs no original-code execution and needs no native
preload rebuild. It gates both complete canonical streams and their original
serializer traces by SHA-256 before extracting graph spans in a temporary
directory. It verifies exact Rust roundtrips, compares keyed/order equality,
checks all Route boundaries against the native traces, and measures release
decode/encode and successful indexed lookups over the actual first graph:

```sh
python3 tools/linux/trade_codec_check.py \
  --private-root ../c6-ghidra --iterations 1000 \
  --output evidence/linux/trade-rust-codec-1.json
```

[The recorded result](../evidence/linux/trade-rust-codec-1.json) verifies both
469,638-byte Graphs, each with 179 origins and 1,571 edges, including all 1,571
Route boundaries per stream. Both original graph byte sequences roundtrip exactly;
keyed payloads compare equal while observed graph iteration order differs. Timings
measure this codec and its indexes only, including allocation/validation/drop for
decode and allocation/drop for encode. They exclude original engine execution,
game loading, rules, actions, turns and gameplay RNG.

The raw complete Save/reload/Save comparison remains an 89,198-byte failure.
The independently established 89,156-byte trade ordering contribution does not
authorize masking bytes or classifying unknown state. The remaining 42 bytes
and full state/action/turn acceptance remain separate work.
