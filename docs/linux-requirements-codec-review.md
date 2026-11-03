# Independent review: RequirementSystem v3 codec

Accepted for the bounded Linux XP2 version-3 wire section. No concrete defect
was found in the library's field preservation, length/count handling or resource
preflight. This is a codec acceptance; live requirement evaluation, object
resolution, complete GameEffects state and state/action/turn execution remain
unsupported.

The review inspected `requirements.rs`, its example and audit, the recorded
native writer at `0x23f54c0`, reader at `0x23f7d20`, matching DWARF and the
original serializer trace. Writer/reader/DWARF evidence hashes and the owner-map
and trace hashes match their recorded manifests. The trace attributes exactly
`[73,741,108, 74,930,233)` to RequirementSystem in both saves. This provenance
establishes the boundary independently of the Rust roundtrip.

The native writer and version-3 reader agree with the codec's ordered fields:
u8 version, u32 count, then length-prefixed name bytes, i32 context, i32 subject
and u8 state for each record. Rust preserves names without text conversion,
duplicate records, order, signed references and every raw state byte. Unsupported
versions fail explicitly. Accepting arbitrary state bytes is transport behavior;
the observed native states are only `0`, `1` and `4`, without inferred semantics.

Decode validates the entire structure and trailing boundary before allocation.
Checked arithmetic covers accumulated name lengths, record-array allocation and
record endpoints. Its second pass uses endpoints already proven in bounds.
Encode checks the same logical record/name allocation budget, encoded-byte
budget and u32 count/name representability before producing output. Empty
sections are handled by the library. Review identified an empty-section panic
in the validation example; commit `5a63463` fixes its truncation assertions and
uses checked subtraction for the reduced-record-budget check.

[The actual-section audit](../evidence/linux/requirements-codec.json) reports
28,304 records and 1,189,125 bytes in each of two distinct canonical sections:
56,608 record occurrences total. It compares record and field boundaries plus
signed references and states against the prior independent Python decoder, and
requires byte-identical full-section encoding. These checks provide more evidence
than a constructed Rust encode/decode roundtrip. That report records the earlier
executable. After the adapter fix, the owner verified exact empty-section output
and empty boundaries and reran both complete canonical sections successfully.
The reviewer inspected the fix and its rerun summary and verified the new
executable SHA-256:
`e146e95867eeee92f84c4de475bfc181a5989c1b8fdc2df514432a0883f1ce51`.

The reviewer performed read-only source/evidence and identity checks, without
another engine run or broad test repeat. The implementation owner's recorded
tests and full-section checks are distinct from native malformed-input parity,
which is not claimed. The raw Save-to-Load 89,198-byte failure remains unmasked
and unresolved by this codec.
