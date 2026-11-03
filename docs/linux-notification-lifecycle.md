# Linux notification lifecycle contract

The original Linux notification aftermath is ordered gameplay work. A supported Rust component can model base-instance selection, activation effects and cleanup from typed state plus explicit live inputs; the wire codec alone is insufficient. Complete gameplay state, action execution and turns remain unsupported. No original code was executed for this research checkpoint.

The [reproducible static audit](../tools/linux/notification_lifecycle_audit.py) gates XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. Its [machine-readable contract](../evidence/linux/notification-lifecycle.json) checks all 39 notification virtual tables, every deleting-destructor wrapper, concrete branch/call sites, method hashes and target DWARF. It inherits the general [command-aftermath selection contract](../evidence/linux/skip-turn-state-aftermath.json). Original instructions and DWARF remain local-only under `../c6-ghidra/evidence/linux-notification-lifecycle`; these new raw artifacts are outside the previously proposed upload batch and have not been committed or uploaded. Attribution and reference limitations remain in [reference.md](reference.md).

The lifecycle sequence is SendAutoNotify, cloud-gated ProcessAutoActivate, DismissExpired, then Cleanup. Running only ProcessAutoActivate and Cleanup is a bounded phase, not the entire notification Update. SendAutoNotify can construct instances, assign identities, run subtype behavior and send events; DismissExpired invokes actual subtype expiration predicates and Dismiss. Both require exact support or an explicit unsupported boundary.

## Live state and corrected names

Exact target DWARF identifies these fields; compiled getters corroborate message, summary, dismissed and lock locations:

| Offset | Logical field | Serialized by Instance v5 |
| --- | --- | --- |
| +8 | NotificationID: i16 owner, i16 type, u32 ID | Yes |
| +18 | Group | No |
| +20 | Message | Yes, first locale |
| +a0 | Summary | Yes, second locale |
| +130 | DefaultEndTurnBlocking | No |
| +138 | Parent pool link | No |
| +188 / +18c | AddedOnTurn / ExpireOnTurnDelta | Yes |
| +190 / +194 | Count / signed LockCount | Count only |
| +198 | Dismissed | Yes |
| +199 | CanUserDismiss | Yes |
| +19a | BlocksTurnAdvancement | Yes |
| +19b | IsAutoNotify | Yes |
| +19c | IsExpired | Yes |
| +19d | IsAutoActivate | Yes |
| +19e | HasBeenActivated | Yes |

Earlier derived codec/adapter metadata reversed the two text labels. The first locale is **message**, and the second is **summary**: GetMessageA `0x2cd0310` returns +20 and GetSummary `0x2cd0320` returns +a0; setters `0x2cd1450`/`0x2cd1460` agree. Wire roundtrips establish bytes, not field meaning. A separate scoped correction updates those labels while retaining original serialized bytes and raw historical captures.

The prior classification of three removed records as **already dismissed remains correct**. Cleanup calls IsDismissed (`0x2722dd0`), which reads +198. It does not use +19c IsExpired. The base IsExpired implementation itself uses added turn and expiry delta, not +19c; subtype implementations can use the latter. These predicates must remain distinct.

PlayerNotifications is 864 bytes: owner +0, next ID +4, list sentinel at +8. List nodes have next +0, previous +8, payload pointer +16; size is computed by traversal. Manager is 576 bytes, with the player unique-pointer vector at +24, begin/end pointers +0/+8 relative to that vector and eight-byte elements. PoolType is a 32-byte vector with the same pointer stride. The typed Rust model needs logical identities/order and ownership, not these native addresses or allocator capacities. Passive observation must bound traversal and validate links/counts before associating objects with wire records.

## Selection and blocking

ProcessAutoActivate requires the real manager to accept notifications and the owner to be turn-active. AcceptsNotifications requires initialized player storage, a game that is not uninitializing, and human configuration. The cloud gate applies to autoactivation, while the remaining Update passes still run. Network multiplayer chooses signed severity threshold 3; otherwise threshold 0.

FindAutoActivate scans native list order. A record qualifies when actual IsAutoActivate is true and signed severity meets the threshold, or variant key `c362976f` converts to true. An eligible locked record returns immediately, preempting any earlier unlocked candidate. Otherwise retain the first eligible unlocked record with HasBeenActivated=false, continue scanning for a locked blocker, and return that retained record at the end. The method has no dismissed filter. Locking is signed +194 > 0. IsAutoActivate returns true if already activated; otherwise non-network plus a found definition with ShowIconSinglePlayer=false returns true; remaining cases use +19d. ProcessAutoActivate stops on null or locked, otherwise calls actual virtual Activate(false) and rescans after its effects.

The original vector-map lookup uses the first matching key in vector order. An absent key returns the default typed variant. For a conservative supported bool profile, require nonarray type 1 and canonical low payload byte 0/1; operator bool tests that low byte. Unsupported variant forms can reject before mutation. Duplicate keys need either the proven first-match behavior or explicit rejection, never a sorted-map substitution.

The separate end-turn blocking reduction (`0x2d31060`) first requires actual virtual IsNeeded and nonzero actual virtual GetEndTurnBlocking. It retains the first candidate and replaces it only for strictly greater **unsigned** severity, preserving first ties. It then calls the selected blocking getter again. This unsigned comparison differs from the signed autoactivation threshold. Base IsNeeded is `!(BlocksTurnAdvancement && actual IsExpired)`. Base blocking is zero when dismissed or expired, otherwise live +130. Base expiry requires positive signed delta and `current_turn >= wrapping_i32(added_turn + delta)`, using signed comparison. Nine notification tables use base IsExpired; thirty override it. Do not apply base expiry to an unverified subtype.

The player setter reads/writes old blocking +7a8. A changed value sends `55f15a7b(old,new)` only for the local owner. Initial/final equal serialized Player bytes do not establish missing live reads or intermediate reports.

## Release, destruction and pool abstraction

All 39 notification tables bind Release +20 to `0x2d2de30`. Cleanup visits list order, calls Release on every dismissed record, then erases its node. It retains surviving order and does not decrement or reassign next ID.

Release reads parent pool +138. With a pool, it appends the same logical object to that pool, clears its parent link and returns; its fields remain until reuse or pool destruction. Without a pool, it dispatches the actual deleting destructor at virtual +8. Thirty-eight deleting wrappers destroy base payloads then delete storage. PromoteUnit additionally destroys its ComponentID vector at +1a0. Base destruction releases parameters, message and summary. Owned arbitrary variant payloads can reach virtual destruction through VariantManager, so support must be bounded to proven payload branches.

Pool reuse takes the last stored object, pops it, calls its actual virtual Initialize and restores its pool link. Base Initialize resets logical lock/count, default blocking, flags, turns, type/severity/group, target/location, parameters and text before applying definitions/parameters. Addresses and allocation capacity therefore need no native replica. A Rust phase can retain logical ownership/order explicitly. The mere existence of the pool pointer is not proof of gameplay effects or an indefinite blocker; blanket elimination of every subtype's retained state also is not established. Creation, ID assignment, subtype reset and callback/lifetime dependencies must be proven before dropping that logical ownership history.

The inspected release/base expiration/selection bodies have no direct gameplay RNG call. This does not establish complete transitive RNG equivalence: subtype factories, arbitrary payload destruction, diplomacy activation and reporting callbacks remain part of the dependency graph.

## Activation and subscriber effects

Thirty-seven tables use base Activate `0x2d2dd80`: unconditionally set HasBeenActivated=true, then send event `025f6cfd`, with NotificationID under key `eab69e5a` and the actual incoming bool under `78a2f946`. PlayerDefeated and DiplomacySession override Activate. The former has an IsExpired one-shot gate, can request a diplomacy session, and then calls Dismiss. The latter checks expiration, target alive/turn state and human/paused state before ActivateStatement. These branches are gameplay and must reject before mutation until implemented; their class names do not justify an exclusion.

The base two-Value sender checks reporting enabled/not suspended, constructs the typed payload, invokes synchronous Cache.Dispatcher handlers, then EngineUtility. The host NotificationActivated translator `0x3212d00` requires EventData type `6447c33d` and ID variant type `400`, converts the bool, sets KeepOrder and publishes event 257. Publish invokes an optional synchronous pre-publish delegate before queuing/unlocking the host event. Returning a typed effect without delivering required synchronous behavior must remain diagnostic projection, with full-action acceptance false.

Existing [passive provider evidence](../evidence/linux/reporting-provider-passive-1.json) observes the actual NotificationActivated translator once and a null pre-publish subscriber for that invocation. All 17 observed pre-publish getters were null. The [focused provider evidence](../evidence/linux/reporting-provider-focused-2.json) separately resolves actual Unit and YieldMonitor callbacks. Those are bounded actual observations from separately identified accepted component replays; they do not prove universal subscriber absence or complete transitive gameplay/RNG effects. This static lifecycle task performed no new native run.

Initial lock/pool/default-blocking/configuration/definition observations alone cannot stand in for values at later rescans after synchronous callbacks. Passive capture must retain branch-time selection inputs, actual virtual methods, release ownership and before/after blocking reads. The capture owner is preparing that observation separately. Until it closes, a comparison using supplied scenario inputs must say those inputs were not observed natively.

## Reproduce

```sh
python3 tools/linux/notification_lifecycle_audit.py \
  --output target/linux/notification-lifecycle.json
```

The audit checks static contracts only. No Rust gameplay transition is implemented by this checkpoint. The raw Save→Load **89,198-byte failure** remains unclassified and unmasked; complete gameplay state/actions/turns, branch restoration and universal external player-policy boundaries remain unresolved.
