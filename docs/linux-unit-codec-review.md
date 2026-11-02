# Independent Linux Unit codec review

The current Unit v28, Formation v1 and nine-handler queue codecs are acceptable as **bounded wire transport components** after the corrections below. They remain isolated primitives: they do not reconstruct complete live gameplay state, execute actions or establish complete Civ VI state/action/turn equivalence. The [review artifact](../evidence/linux/unit-codec-review.json) records the exact reviewed source identities and checks.

Review covered implementation `508b01e`, variant implementation `da56d98`, resource fix `d83016b`, and documentation correction `6ba051b`. The target remains Linux XP2 module SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and matching host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. No engine code executed during this review.

## Findings and corrections

1. **Aggregate resource accounting — fixed in `d83016b`.** Variant key-name and Char8 bytes originally consumed the byte budget without consuming `max_items`, contradicting the Unit API's aggregate dynamic-element/string-byte policy. The decoder now charges before copying; the encoder tracks cumulative nested charges before writing. The independent literal fixture requires exactly six items: one entry, two name bytes and three Char8 bytes including NUL. Lower budgets reject both directions, and a failed prefix decode preserves the caller's prior budget. The reviewer inspected the fix and independently ran all 17 focused Unit tests successfully.
2. **Nested version prose — fixed in `6ba051b`.** The contracts document incorrectly said EspionageBonus, EspionageTimeReduction and InitiationYieldPopulation lacked version prefixes. Actual target writer constants are signed int32 7, 7 and 1. The machine contract and Rust implementation already agreed. Only Sight.PlotEntry lacks a prefix among those four records.

No unresolved implementation mismatch was found within the reviewed supported wire subset. This is a scoped review, not proof of every possible input or semantic branch.

## Schema and boundary checks

The reviewer regenerated `tools/linux/unit_wire_contracts_audit.py` into `target/linux/unit-wire-contracts-review.json`; the SHA-gated ELF/DWARF result exactly matched the committed 18-component contract, including 168 writer field calls. Separate comparisons checked all 73 Unit field names/order, all 18 components' member names/source offsets/order, and every component version gate against that contract. Scalar serializers were checked against the separate scalar report, avoiding an inference from equal-width round trips alone.

Activity and automation remain signed enum32 bits; the operation timer is int32; formation IDs and Unit IDs remain uint32. PlotCoord's invalid sentinel is signed -9999 on both axes; path coordinates retain signed int16 widths. Raw fixedpoint int32 and variant float/union bits survive without conversion. Definition enum hashes and ordered hash/value sequences remain separate from live definition indices. Promotion values remain uint8, with duplicate hashes and stream order retained. Counted raw strings have no UTF-8 conversion. City component IDs preserve signed int16 owner/type halves and uint32 identity.

Queue hashes, base node version 2, saved activity/automation order, null sentinel 0 and handler-specific parameter order match the independent operation contract. Runtime handler IDs are not substituted for wire hashes. Unknown handlers reject because a generic body length is absent. Formation path and membership order remain serialized sequences; the codec does not check or reconstruct cross-record pointers, reciprocal membership or live graph invariants.

Recursive variants retain unsigned tree order or vector order, header/name metadata and scalar bits within their documented subset. Unsupported types/modifiers/raw arrays and conflicting branches reject. Byte/item/depth limits bound parsing; failed decode returns no partially constructed public object, failed prefix decode preserves the shared budget, and encoding returns no partial byte vector. These rejection policies are Rust support boundaries, not claims that native readers reject the same malformed inputs.

## Independent fixture verification and provenance

After `d83016b`, the reviewer independently ran:

```sh
cargo test -p c6-core unit::
python3 tools/linux/unit_codec_check.py --iterations 1 --output target/linux/unit-codec-independent-review.json
python3 tools/linux/unit_wire_contracts_audit.py --output target/linux/unit-wire-contracts-review.json
```

All 17 focused tests passed. The read-only slice checker reproduced 2,524 complete Unit round trips, 2,500 Formation round trips, 2,524 queue round trips and 17,668 matching nested boundaries. Both streams have identical Unit/Formation payload hashes and attributed ranges: the counts represent two comparisons of 1,262 Unit records and 1,250 Formation records, not twice the branch diversity. There is no attributed nonempty SkipTurn queue. Rare nested/recursive variants rely on static contracts and constructed tests rather than independent populated native fixtures.

The checker gates exact uncompressed canonical Save hashes, decompressed native trace hashes and the canonical manifest's module identity. It extracts serializer-owned ranges and compares seven nested boundaries per Unit, without searching for ID byte patterns. It records current Rust source and adapter hashes. It does not rerun capture, validate the module currently installed, or independently gate every original capture configuration input. Historical capture reports retain configuration/source/dependency identities and their limitations; this review does not upgrade those captures to a fully initialized reversible game fixture. A checker field called `native_malformed_checks` counts Rust rejection of mutated native-derived records, not malformed-input comparison with native readers.

## Acceptance limits

Wire records omit Unit domain, Formation evaluating-operation, pointers, archive dirty history and other live data. Native readers resolve owners/definitions and perform fixups absent here. Definitions/configuration must be validated separately before hashes can become runtime IDs or wire values can initialize live state. Path cache fields are serialized gameplay state; their exact future mutations and dirty history remain required. Preserving those bytes does not implement ClearPathCache or SkipTurn.

All actions remain unsupported. Full legality, queue deactivation/continuation, delayed death, callbacks/subscribers, notification/diplomacy state, RNG and genuine external player-decision boundaries remain acceptance work. The raw Save→Load→Save failure remains **89,198 bytes**, unclassified and unmasked.

The one-iteration review run is a correctness check, not a new benchmark. Existing release timings measure record decode/encode allocation/drop and an adapter lookup with index construction excluded; they measure no original engine performance, initialization, rules, actions, turns or RNG.
