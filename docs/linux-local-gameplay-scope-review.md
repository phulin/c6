# Independent local gameplay metadata scope review

The bounded exclusions in archive checkpoint `bad86da` and reporting checkpoint
`08998a5` are supported for the pinned Linux local-authority, stock/no-added-consumer
profile. Native Unit/Formation dirty-pointer history, sync registration acceptance
and the concrete desktop UnitFlag dirty-ID vectors are not required gameplay
state in that profile. Actual field values, action ordering, reports, gameplay
callbacks and RNG remain required. This is a source-scope decision, not native
SkipTurn parity or completion of the Rust state/action/turn library.

The [independent evidence](../evidence/linux/local-gameplay-scope-review.json)
records direct target checks, both rerun producer audits, source identities and
explicit limits. No original game code or new capture is used. Raw disassembly
remains in the existing private archive/reporting evidence directories.

## Positive source evidence

The archive conclusion does not follow merely from zero direct XP2 calls to
the selected archive query APIs. Activity assignment `0x2da4010` branches around
dirty-tree insertion when its archive owner is null and still copies the
actual value. Queue edit `0x2da3100` similarly returns its actual storage with
or without insertion. These leaves do not replace their higher-level setters
or reporting continuations. Self-assignment behavior and native equal-value
reports must still follow the actual caller contracts.

Host clear `0x3c45d18` performs an indirect call through each wrapper's actual
virtual slot `+0x38`; this is not ignored as an unknown direct-call edge.
The audit resolves all 49 exported Unit/Formation wrapper vtables at their ELF
relocation slots and verifies each complete matching clear target is one RET.
Thus this particular clear's pointer-tree membership/order cannot change the
wrapped gameplay values or execute gameplay callbacks. The result does not
cover a replacement vtable, another archive family or arbitrary allocator
overrides.

The independent byte checks positively retain the transport mutation boundary:

| Host call site | Actual target or dispatch | Scope consequence |
| --- | --- | --- |
| `0x39d553c`, `0x39d5559` | Full data hash `0x3c45e82`, dirty indices `0x3c45f34` | Native synchronization bookkeeping |
| `0x39dae1d` | Delta union output `0x3c45c2c` | Native snapshot production |
| `0x39d82f7` | Live delta application `0x3c46036` | Mutates selected wrapped values |
| `0x3c4615f` | Wrapper virtual reader at `+0x18` | Live input is not a metadata-only operation |
| `0x39d836d`, `0x39d83be` | Desync callback lookup `0x3c470fc`, then indirect invocation | Resynchronization callbacks remain gameplay obligations if this path is supported |
| `0x39d98f7` | Archive clear `0x3c45d18` | The verified typed clear boundary above; later session callbacks are separate |

The complete-disassembly direct census remains useful corroboration: ten
selected host archive API call sites, all in sync/archive code, and zero XP2
direct sites. It is not a whole-program proof against inlining, aliases,
custom controllers or indirect calls. The inspected setter bodies, actual
virtual clear targets and explicit resync input path supply the positive
evidence for the narrower local boundary.

## Stock desktop receiver boundary

The supplied stock UnitFlag playback drain iterates its dirty IDs, resolves
existing flag instances, calls UpdateFlagType/UpdateReadyState, and clears the
list. The inspected Lua methods write flag textures, local dim state and
control alpha. Host receiver creation makes a 32-byte vector, not a callback
object; MembersAux reads identities and advances its Lua iterator. The source
audit checks the concrete lifetime/accessor bodies, selected storage/API
references and all five supplied Lua files containing DirtyComponents.

Only activity-changed and operation-deactivated sends in the retained eleven
SkipTurn Send inventory intersect the literal UnitFlag dirty registrations.
CityBanner dirty registrations concern population/religion changes and do not
intersect that inventory. This does not exclude independent consumers of those
same Send events. Queue completion's YieldMonitor continuation, notifications,
script callbacks, translator/cache delegates and their ordering remain separate
gameplay obligations.

The optional no-UI receiver exclusion is not a claim that every native getter
it calls is universally pure. The actual drain calls IsHuman,
IsReadyToSelect, IsEmbarked and GetFortifyTurns. Existing readiness evidence
includes HasAdjacentMove and conditional HoldCycling paths. Implement those
semantics wherever authoritative gameplay/report code requires them; do not
derive a default readiness answer from omitting the flag renderer. This review
does not classify all CityBanner transitive getters, arbitrary UI callbacks
or dynamically loaded scripts.

## Conditions for loaded SkipTurn

The loaded action adapter must enforce or explicitly retain these conditions:

- Owned in-process gameplay state is authoritative. Native snapshot/delta
  resynchronization is unsupported; a caller cannot route a snapshot through
  this metadata exclusion and silently skip live writes or callbacks.
- The archive exclusion applies to the verified Unit/Formation family. Other
  archive classes or gameplay consumers of dirty state require their own
  implementation/evidence before admitting a wider profile.
- Omission of host dirty fanout is conditional on the stock source profile
  with no added dirty-list gameplay consumer. Supplied Lua hashes alone do
  not prove the active mod/configuration identity or historical registrations.
  Unknown registrations remain unknown, rather than empty.
- Preserve real setter mutations, identities, queue/member order, source-required
  reports and their synchronous gameplay effects, locks, projected values,
  notifications, readiness and RNG. Unknown gameplay subscribers cannot be
  replaced with a collected transcript or a permissive default.
- Do not require native dirty-tree baselines/controller acceptance, fabricate
  attachment flags, or reproduce desktop receiver capacity/ID history as an
  action gate. Diagnostics may still retain unavailable baselines and raw
  observations.

The reporting doc's former statement retaining all Core archive dirtiness was
an earlier boundary; `bad86da` supplies the narrower Unit/Formation local
exception. Reporting correction `d44e6ed` links that distinction. Neither
checkpoint proves an active initialized fixture or closes complete action
affected-state comparison. Unknown save bytes and the recorded 89,198-byte
Save→Load difference remain unmasked evidence; save production is not a
gameplay action requirement.

Reproduce the static review checks with:

```sh
python3 tools/linux/local_gameplay_scope_review.py \
  --output evidence/linux/local-gameplay-scope-review.json
```
