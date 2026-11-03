# Compatibility target and decision policy

The active behavior target is the supplied Linux XP2 build. Profile identity is
part of each operation's contract; support for a similar name, schema or platform
does not establish compatibility.

| Artifact | SHA-256 |
| --- | --- |
| `libGameCore_XP2.so` | `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` |
| Matching Linux `Civ6` host | `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4` |

These identities do not replace matching dependencies, rules, enabled DLC/mods,
configuration and state inputs. Public builds do not load these binaries or
require a game installation. The [registered rules profiles](rules-data.md)
retain their own complete descriptors. Historical Windows compatibility data
remains separate and supplies no Linux behavior guarantee.

## External decision policy

The intended simulator yields to external policies for **all player decisions**.
It must execute exact rules, mandatory callbacks and upkeep between decisions,
then yield at the next genuine choice. Native AI choices cannot silently fulfill
an action or end a turn. Diplomacy, deals, NPC state and gameplay RNG remain in
scope regardless of the owning class's name. A planning-only exclusion requires
proven semantics; unknown fields remain unresolved requirements.

Complete gameplay state, action execution and turns are still unfinished; see
[status](status.md). Deterministic arithmetic and supported wire data are bounded
building blocks, not complete executable gameplay.

## State and persistence

Simulator snapshot transport and component wire codecs are separate from Civ VI
save-container loading. Unknown bytes may be preserved but cannot be advanced
correctly without their semantics. Save writing and byte-identical save rewriting
are not completion requirements or action-acceptance gates. Exact affected
state, RNG, ordering and logical identity remain required. Serialization-only
differences do not justify hiding unresolved gameplay effects.

Rendering, desktop startup and network transport are optional integrations. Their
omission does not remove gameplay legality, initialization or synchronous rule
callbacks. Coverage and behavior remain specific to the declared target and
configuration. No cross-platform equivalence or build-age inference follows
from matching names or similar layouts.
