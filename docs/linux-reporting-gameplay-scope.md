# Host dirty-component gameplay scope

The stock UnitFlag dirty receiver does not add a gameplay transition to
SkipTurn. Its dirty IDs drive flag textures and dimming after event playback.
A headless gameplay adapter should omit that concrete desktop receiver and its
ID history. Reconstructing its registrations, native allocation/capacity or
before/after ID vectors is not a gameplay acceptance prerequisite.

This corrects the blanket remaining capture requirement in
[the earlier reporting closure](linux-skip-turn-reporting-closure.md). That
requirement remains useful for reproducing full host reporting state, but the
user does not require desktop presentation fidelity. The conclusion is scoped
to the supplied stock sources and the relevant event registrations, not to an
arbitrary registered callback. Complete gameplay state, actions, turns and RNG
remain unimplemented; no native parity is newly claimed here.

## Target evidence and lifetime

The [offline audit](../tools/linux/reporting_gameplay_scope_audit.py) and
[result](../evidence/linux/reporting-gameplay-scope.json) gate host SHA-256
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`
and XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
They check 40 complete original host method bodies, 27 selected direct
instruction references and all supplied Lua files containing DirtyComponents.
No original executable, debugger or engine is run. Raw method disassembly is
kept privately in
`../c6-ghidra/evidence/linux-reporting-gameplay-scope/host-dirty-components.disassembly.txt`.
The public evidence contains identities, addresses and conclusions only.

The actual host object is a 32-byte vector container with no callback/vtable.
Lua `lCreate` calls Create at `32b7824`; Create (`32b6280`) allocates and
constructs the vector and adds its pointer to the manager's active requests.
AddEvent (`32b5930`) forwards the hash and receiver pointer to RegisterForEvent;
SetDirty/Add append deduplicated eight-byte identities as already established
by the earlier closure. Clear (`32b5a10`) clears IDs without unregistering.
Destroy (`32b6310`) unregisters the receiver from every event, removes its
active pointer, destroys the vector and deletes the allocation.

The selected host direct-reference census finds four GetArray call sites
(`32b7232`, `32b7246`, `32b725a`, `32b7270`), all in Lua MembersAux
(`32b71c0`). It reads an identity, advances a Lua iterator cursor, and returns
owner/ID; it does not dispatch a gameplay callback. GetCount is addressed by
its Lua method wrapper (`32b75f0`), Clear by its wrapper (`32b7600`). Create is
called by Lua lCreate and the constructor by Create. The active-request and
event-map references are initialization and their own registration, lookup and
lifetime methods. No separate gameplay consumer appears in this selected
reference census. This is a bounded reference fact, not proof against every
possible alias, indirect call or dynamically loaded script.

## Concrete stock consumers

The complete supplied `.lua` text-reference inventory contains five files:

| Source relative to supplied assets | Registration | Drain |
| --- | --- | --- |
| `Base/assets/ui/unitflagmanager.lua` | Unit operation deactivated, activity changed, movement points changed, embark changed | GameCoreEventPlaybackComplete: update flag type/readiness, Clear |
| `DLC/polandscenario/ui/unitflagmanager.lua` | Same four Unit events | Same flag type/readiness drain, Clear |
| `Base/assets/ui/worldview/citybannermanager.lua` | City population changed, religion changed | Refresh city banners, Clear |
| `DLC/expansion1/ui/citybanners/citybannermanager.lua` | Same two City events | Refresh city banners, Clear |
| `DLC/expansion2/ui/citybanners/citybannermanager.lua` | Same two City events | Refresh city banners, Clear |

The audit records hashes of all five files, so changing a consumer changes the
accepted evidence. It also records literal AddEvent registrations.

For the Base UnitFlag source, OnEventPlaybackComplete at line 1877 iterates the
list and invokes only UpdateFlagType and UpdateReadyState for existing flag
instances, then clears it. UpdateFlagType (593) reads embarkation/fortification
and local style, and sets four control textures. UpdateReadyState (377) reads
human/readiness and calls SetDim (530). That changes the flag instance's local
`m_IsDimmed` and UpdateDimmedState (546) sets animation endpoint/control alpha.
GetUnit (216) resolves the flag's player/Unit ID through the Unit collection.
None of these Lua bodies submits an action, ends a turn, writes a Unit,
consumes gameplay RNG or calls a gameplay script event. The Poland variant has
the same drain and visual effects. Registration is created by
RegisterDirtyEvents (Base 1968), playback handlers are added/removed (2036/1992)
and OnShutdown destroys the receiver (2098).

Of the eleven captured SkipTurn sends, only UnitActivityChanged and
UnitOperationDeactivated intersect these stock dirty registrations. None
intersects a CityBanner registration. NotificationActivated, operations
cleared/added/started and queue complete have no matching literal stock
DirtyComponents registration in this source inventory. This closes the
relevant stock dirty-list consumer obligation without claiming city numerical
getter purity or needing to reproduce full city banner refreshes.

| Captured send hash | Calls | Stock dirty-list consumer |
| --- | ---: | --- |
| UnitActivityChanged `e23b112d` | 3 | UnitFlag visual refresh |
| UnitOperationDeactivated `a60328c5` | 2 | UnitFlag visual refresh |
| UnitOperationsCleared `383669f1` | 2 | No literal registration |
| UnitOperationAdded `15c35863` | 1 | No literal registration |
| UnitOperationStarted `3d4006f2` | 1 | No literal registration |
| NotificationActivated `025f6cfd` | 1 | No literal registration |
| Queue complete `9ea4c222` | 1 | No literal registration |

These classifications concern DirtyComponents registrations only. The same
sends may still invoke independent gameplay-affecting delegates or consumers.
The call inventory is retained from the earlier reporting closure; this
static audit adds no native calls or successful full-action fixtures.

## Admission consequences and limits

For a verified stock rules/script profile with no additional dirty-list
consumer, the headless adapter can omit this host dirty fanout. Preserve exact
Core setters, logical identity, ordering and gameplay RNG. Keep actual Unit
readiness semantics in scope when needed by Manager/gameplay; drawing a dimmed
flag is not a reason to implement an additional desktop observer. The existing
bounded Rust dirty-vector helper can remain a host-reporting utility; wiring it
into the gameplay state is not required by this evidence.

The historical replay did not capture actual registration pointers or vector
contents. They remain unknown, not empty. The supplied source hashes are not
an active mod/configuration identity. A new mod/script that uses this Lua API
and then mutates gameplay must be admitted and implemented as a gameplay
consumer; the name DirtyComponents is no exclusion rule. Unknown gameplay
subscribers must not be silently discarded. This audit does not classify all
city banner transitive getters, every native getter side effect, other cache
delegates, translators, queued events, lock semantics or scripting callbacks.
Those remain separate source/capture boundaries rather than a requirement to
copy desktop receiver storage.

Core tracked archive dirty-index history is distinct from these host vectors.
The newer [archive gameplay scope](linux-load-archive-gameplay-scope.md) excludes
the pinned Unit/Formation native dirty-tree history and sync registration
acceptance from local-authority action requirements, under its verified wrapper
and no-resynchronization conditions. Actual field mutations, ordering and
required report/subscriber effects remain exact; native snapshot resynchronization
is gameplay-mutating and unsupported. The raw 89,198-byte Save→Load difference
remains unclassified and unmasked; save production is not a gameplay
action-acceptance gate.

Reproduce with:

```sh
python3 tools/linux/reporting_gameplay_scope_audit.py --output target/linux/reporting-gameplay-scope.json
```
