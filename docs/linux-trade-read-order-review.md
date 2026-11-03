# Linux Trade fresh-Read ordering review

The source-order model at `8ba793851c4d2eb261ee86277cdfc30a098958ad`, built through the actual production export `0fbf8cd`, passes this independent bounded review. It reproduces all 469,638 bytes of the known Graph A → B output (179 origins, 1,571 edges). This accepts an ordering model for fresh default containers with valid typed unique keys. It does not establish a general native save loader, existing-container behavior, or gameplay equivalence.

The review driver is [trade-read-order-review.py](../evidence/linux/trade-read-order-review.py); its checked report is [trade-read-order-review.json](../evidence/linux/trade-read-order-review.json). It verifies the immutable source archive rather than injecting exports into the shared checkout. Two original module tests and eight independent discriminating case groups passed. No original game code ran during this review.

## Source contracts checked

The pinned Linux GameCore and original host bytes gate complete method extents. Independent inspection covered Graph construction, Read and Write; outer insertion, rehash and iteration; inner insertion, copying and iteration; vertex hashing/equality and load-factor calculation. The report fingerprints the author's method inventory plus the directly relevant `CalculateLoadFactor` and `Vertex` equality methods. Static import/export checks resolve the module's two EASTL prime-policy PLT entries to the original host's exact exported names, absolute ET_EXEC addresses and method sizes. These checks establish source-image bindings. Live GOT contents, interposition and transitive host math bindings were not observed or admitted.

Vertex hashing preserves the native signed packing: the zero-extended 16-bit kind, zero-extended 32-bit data1 shifted by 32, and sign-extended data2 are ORed, then hashed as eight little-endian bytes with wrapping FNV-1a. Equality compares the three fields rather than their hash. Independent signed-value goldens and distinct equal-hash vertices discriminate these operations.

Outer containers start at the host-derived prime 53 for the constructor's request 51. Inserts prepend; rehash visits old buckets and chains in native order and prepends into the new buckets. Prime growth follows the original table and policy, with no fitted per-fixture permutation. Tests at 53, 54 and 110 colliding empty origins distinguish prepend order and successive rehash reversals. Empty origins and the saved total-edge counter survive unchanged.

Inner containers begin empty, grow to four slots, and test the current load before insertion. Native float load-factor and the separate free-slot condition determine growth. Home selection preserves wrapping multiplication and high-half scaling. Robin Hood displacement uses a strict distance comparison, keeps the resident on ties, wraps probes, and reinserts old occupied slots in ascending slot order on growth. Tests distinguish tie behavior, displacement, wraparound, the third insertion at capacity four, resize copy order, and equal-hash distinct keys. The reserved destination hash is explicitly unsupported without changing the input; that hash remains valid for an empty outer origin.

The typed Graph boundary rejects duplicate or inconsistent keys before this model runs. Native duplicate lookup/reuse and overwrite behavior therefore remain outside this API's accepted domain. The model preserves complete edge payloads, IDs, paths and yields and changes only serialized traversal order. Auxiliary Rust lookup maps do not select the emitted order. The comparison uses the entire typed owner without sorting, masking, or dropping fields.

## Prediction frozen before a new answer

The immutable local evidence directory is `../c6-ghidra/evidence/linux-trade-read-order-review/frozen-before-native-c-xcznm22o/`. Its manifest was frozen at `2026-11-03T14:22:39Z`, before any native B → C answer was available:

| Artifact | SHA-256 |
| --- | --- |
| `manifest.json` | `2957dce152ff0f4c284ead3f04b930c8fd298cd0ef10c20a7c86d16740d3ddf5` |
| `candidate` | `2bca7e60eb81ebfaa8d585b9f2c87c4a320a4e0cfba0d4536e8a8b8367455a45` |
| `b-predicted-c.graph` | `38f3e34a0004bdca3249ec82b1f96cb4f528e78b151a3fa1c2f179921012bff2` |

The manifest also records the full source archive, candidate source, actual input Core and Graph identities and all output identities. The candidate accepts only the input Graph and output path; it receives no expected answer. Independent reruns reproduced both frozen outputs. The B → C prediction differs from B in 65,510 Graph bytes and remains unaccepted pending separate native comparison. It predicts the Graph owner only, not a complete Core C stream. Later answers must not rewrite this candidate or its pre-answer evidence.

## Remaining limits

Fresh pre-Read native capacities and container state were not captured. The model's fresh-default precondition is source-derived, not a claimed observation of the known fixture. Native allocator state, archive attachment, ID/reverse maps, Route finalization, and live gameplay effects are not modeled by this ordering boundary. Resource limits are implementation refusal bounds, not native gameplay limits.

The historical raw Save → Load failure remains 89,198 Core bytes: 89,156 Graph bytes and 42 bytes in other owners. No unknown bytes were classified away or masked. Complete state, action execution, gameplay RNG and turns remain unsupported.
