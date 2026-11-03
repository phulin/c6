# Linux AI.Control.Manager current wire

The bounded [Rust codec](../crates/c6-core/src/player_ai_control.rs) closes the current Control.Manager saved grammar, including all five polymorphic contract types. It preserves saved identities, hashes, signed values, raw fixed point values, allocated-instance order, recycled free-list order and null contract slots. This is wire support only. Full Player import, gameplay state/RNG, external-policy decision boundaries, actions and turns remain incomplete. No AI-owned block is excluded or masked; the recorded **89,198-byte raw Save→Load difference** remains a failure.

The [read-only audit](../tools/linux/player_ai_control_audit.py) gates the full Linux XP2 ELF against SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, pins complete method bodies, checks the factory switch table and resolves each concrete contract's virtual Read/Write slots. The [report](../evidence/linux/player-ai-control.json) compares two existing canonical Core streams and their independently SHA-gated owner traces. Trace boundaries are comparison inputs only; production decoding walks the grammar and returns exact prefix consumption. Discovery attribution is the [cru121 reference](reference.md), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`; supplied Linux symbols, DWARF and target bytes establish the actual schema.

All words below are little endian. Priorities, Player identities, status and timing fields retain raw signed i32 values. Definition references retain u32 hashes, including all-ones sentinels. Coordinate pairs are two signed i32 words; no narrowing to i16 occurs.

1. Manager version u32(2), Player i32, current priority i32.
2. Recycled container version u32(0x10001), block size u32(16), capacity u32 and active count u32. Each active entry is index i32, Instance version u32(2), ID u32, run priority i32 and recruit priority i32. Then current ID bits u32, free count u32, free head u32, free tail u32 and exactly free-count u32 indices in saved order.
3. `m_hasOutOfTurnProcessing` bool byte and `m_bRunningAsHuman` bool byte.
4. Contract count u32 and that many entries. Each starts with signed i32 type key. Key -1 is a null slot and has no body. Other unknown keys fail explicitly: these records are unframed.

Manager Write is `0x201cf80` (279 bytes), Read `0x201d0c0` (401 bytes); recycled writer/reader are `0x2021970`/`0x2021720`. Instance writer/reader are `0x201ce00`/`0x201ce70`. Manager's minimal current extent is 50 bytes; each allocated instance adds 20 bytes and each free index adds four.

Every non-null contract body begins with base version u32(3), status i32, Player i32, priority i32, time limit i32, time estimate i32, ID u32, owner u32 and extra-present-value raw FixedPointT<8> i32. The base body is 36 bytes. Its current writer is `0x2018cc0`; it does not serialize live pointers, archive metadata or `m_numHandles`.

| Key | Concrete type | Derived version and fields after base | Total including key | Writer / reader |
|---|---|---|---:|---|
| 0 | UnitRequestContract | v1; Unit, Building, District u32 hashes; location i32 pair; required City u32; keep-control bool byte | 69 | `0x200c260` / `0x200c1a0` |
| 1 | TechResearchContract | v2; goal/next Technology u32 hashes; best-controller u32 | 56 | `0x200c950` / `0x200c860` |
| 2 | CivicResearchContract | v1; goal/next Civic u32 hashes | 52 | `0x200cd10` / `0x200cc90` |
| 3 | PurchasePlotContract | v1; location i32 pair | 52 | `0x200cf00` / `0x200cec0` |
| 4 | DiplomaticActionContract | v2; target Player i32; DiplomaticAction u32 hash | 52 | `0x200d330` / `0x200d2a0` |

Factory `CreateRawContract` at `0x201ca10` accepts keys 0–4; the switch table at `0x2dfef74` reaches those concrete constructors. Contract stream operators at `0x2018210`/`0x2018230` dispatch through virtual offsets 0xc0/0xb8. The audit resolves the five vtables to each concrete writer and reader, rather than treating the base body as the entire contract.

The two original saved streams each contain 43 Player v47 records, all with saved header flag index 1 (`m_bWasEverAlive`, byte offset 77 within the current header) true. Each has exactly one Control.Manager record: 86 total. Each stream contains 1,045 active instances, 915 free-list entries and 795 contracts: 776 UnitRequest, seven TechResearch, one CivicResearch, five PurchasePlot and six DiplomaticAction. Thus all five current non-null types are exercised by actual Save output. Null slots and an empty manager are covered by source tests only. The false Player branch is established by the existing Heroes/Player writer pins, not exercised by these two streams; it skips the bool hash arrays and all fifteen AI records and reaches the inline tail. Control is followed by InfluenceMap v2. No owner cut becomes a production offset.

Decode limits separately bound consumed bytes and shared dynamic-item credit. Every active instance, free index and contract slot consumes one item; fixed fields consume none. Counts are checked against remaining byte credit, actual input and aggregate items before fallible owned-vector allocation; storage-size arithmetic is checked. Encode checks counts, exact total length and shared items before fallible exact output allocation. Invalid versions, wrong block size, noncanonical bools, unknown contract keys, truncation and strict trailing bytes fail explicitly. The codec does not validate live recycled-ID invariants or remap hashes into a rules database. It preserves raw saved fields and ordering.

Native Read is a broader operation: Manager resolves `Globals::GetPlayer`, calls Initialize and InitArchive, constructs contracts through the global AI factory, and edits wrapped fields. It does **not** append null sentinel slots to the live contract vector, whereas this wire codec preserves them so Save bytes can round trip. Read sets sync-turns from the actual network configuration and initialized true; those are not saved fields. Instance constructor/Read leave callback reconstruction and destroy-state lifecycle outside this codec. Contract readers edit archive fields and remap definition hashes to indices; factory creation increments global factory ID state before the saved ID is loaded. No claim of transitive RNG purity, live Read equivalence or planning-only exclusion follows from this structural audit.

Reproduce structural comparison and committed candidate validation:

```sh
python3 tools/linux/player_ai_control_audit.py --source-commit <committed-codec-and-driver> --output target/linux/player-ai-control.json
```

The candidate comes from a Git archive and the driver verifies its own archived bytes. If the parent library export has not landed, the driver adds that export only inside the isolated archive and records this fact. It runs strict Clippy with the exported module, two focused source tests and decode/re-encode of all 86 actual records with exact byte/item caps, one-short caps, every truncation, strict trailing refusal and exact prefix consumption. This uses existing original Save outputs; no original game Read is executed and no proprietary data is written to the public repository.
