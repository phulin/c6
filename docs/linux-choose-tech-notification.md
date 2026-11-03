The Linux ChooseTech module implements a bounded auto-notify constructor and
read-only research predicates over typed PlayerTechs and admitted activated
rules. It advances the notification aftermath dependency; complete gameplay
state, actions, RNG and turns remain unimplemented and unvalidated. The
[source audit](../evidence/linux/choose-tech-notification-source.json) gates the
supplied XP2 module SHA and records complete method identities. It executes no
original game code and makes no native factory/predicate parity claim.

`choose_tech::create_auto` covers the actual SendAutoNotify call: auto=true,
NULL parameters and type hash `0x592df4c8`. ChooseTech.Create also discards an
incoming parameter container before forwarding to Instance.Create. The complete
XP2 registration chain is Manager.Initialize's virtual slot +0x58, the XP2
RegisterNotifications implementation at `0x2d7bc80`, ChooseTech.Register at
`0x2d2a3d0`, and the inherited Manager.Register at `0x2d2f7d0`. The latter inserts
into the creator map; the source is not an observation of that map at an arbitrary
later invocation. Manager.Create at `0x2d2f700` uses base creation when the entry
is absent/null or the registered creator returns null. This fallback remains
explicitly unsupported by this subtype module.

The constructor applies base Initialize's semantic resets and the subtype's
final writes: expiry delta zero, flags +0x199=false/+0x19a=true and signed default
blocking `0xeef11bf9`. Locks reset to zero. The unsaved group hash is retained
separately from the wire Notification record. Auto mode skips InitializeText;
message/summary remain empty until that later phase. The actual activated
ChooseTech definition has no end-of-turn expiry flag, so the intermediate
PreviouslyActiveThisTurn branch is skipped. Other expiry settings reject until
that dependency is supplied. HIGH resolves to native severity enum 2.

The caller supplies logical fresh allocation or a typed reused object and owns
any LIFO pool removal. Physical addresses and free-list capacity are not
required. Reuse preserves the old own NotificationID and associated component
type where native Initialize does, resets other modeled fields, and does not
mutate the supplied prior record. Missing allocation/creator authority and
allocation failure reject explicitly. Nonempty reused parameters also reject
until their destruction effects are closed. The helper does not invent a live
pool, choose a null creator, append to the notification owner, or advance the
owner's next ID. The separately implemented redundancy, InitializeText and Add
phases remain the caller's responsibility.

`Definitions::from_rules` reuses the actual static-resource/configuration gate
and both original technology SQL orders. Research queue hashes are resolved
through the native collection lookup; they are not cast into native indices or
SQL row IDs. `Research::is_needed` preserves original order: base flag +0x19a,
GetResearchingTech, city count, actual human configuration, MoreToResearch,
then the -1/current CanResearch branch. Missing later inputs are not demanded
when an earlier native branch returns. PlayerTechs acquired/boost values come
from their typed hash arrays with final overwrite precedence; missing current
values reject rather than becoming false.

CanResearch follows the target's active global node, XP2 hidden/reveal, owned
repeatable technology and all-prerequisites checks. Hidden reveal uses any
completed prerequisite; CanResearch requires every prerequisite. MoreToResearch
uses its own native ordered type vector and repeatable/owned logic. These are
separate rules. Mutable validated GameTechs nodes and prerequisites are explicit
inputs, not reconstructed from initial SQL edges. The input constructor checks
shape/session binding but cannot authenticate an invented live export. Current
retained state has no such graph owner yet.

The actual diagnostic benchmark Player0 has 11 cities, human configuration,
queue hash `0x9ce01238` and resolved native research index 75 among 77 technology
definitions. It reaches MoreToResearch before the current-tech/CanResearch test,
so it correctly stops at the missing validated mutable GameTechs graph. This is
a named action-completion gap, not a false IsNeeded answer or a passing native
comparison. The exact next owner comprises GameTechs nodes at +0xc0 and its
separate ordered TechTypes vector at +0xf0. Node type is +0, active is +4, and the
current prerequisite vector is +0x10. Native Read/Validate provenance remains
necessary before using these values as live state.

The offline checker uses actual admitted rules and typed retained PlayerTechs,
plus explicitly synthetic graph and logical-creation boundary cases. It retains
complete source preimages, an immutable source archive, differences from the
recorded commit, lock/toolchain/dependency identities and the tested executable.
The project source comes from a fixed committed tree. Exact new checker/audit
preimages are retained as explicit differences from that commit; unrelated
concurrent WIP is excluded. Earlier mixed and failed artifacts remain unchanged
with their original limited provenance, without being relabeled as coherent builds.
Reproduce with:

```sh
python3 tools/linux/choose_tech_notification_audit.py \
  --output target/linux/choose-tech-notification-source.json
```

Save production and byte-identical rewriting are not acceptance requirements.
The inherited 89,198-byte raw Save→Load failure remains unmasked; complete
runtime/gameplay effects and RNG still require native comparison.
