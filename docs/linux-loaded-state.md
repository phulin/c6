# Consuming loaded state

`loaded_state::LoadedState::decode` consumes immutable Core storage, invokes the
attributed contextual reader once, and moves its Map and every ordered Player
Unit/Formation roster into the existing `world::World`. It retains all other
decoded Core and Player components, including City/District/AI/manager data,
and owns the original bytes for the unparsed tail. It does not discard state
merely because the next diagnostic action only accesses a Unit.

`CoreImage::Unique(Vec<u8>)` transfers the existing buffer. `Shared(Arc<[u8]>)`
explicitly shares immutable bytes. Neither path clones a World or falls back
to cloning shared parsed records. `loaded_components` exhaustively splits the
source structs; Unit slots are converted with checked fallible reserve while
values move, and Formation collections move directly. World builds lookup
indices once and preserves native iteration, requested/resolved IDs, low16
lookup and allocator metadata. Its existing allocation policy remains separate
from the reader's byte/item accounting.

Saved flags remain in retained headers. Actual Manager mapping/nonnull-slot
facts, Configuration team/slot status/alive/turn-active state and pause/
automation controls enter through explicit `ControlInputs` bound to the
admitted stream identity. Missing facts yield named errors. Validation precedes
any control mutation. Control binding is startup-only; repeated binding rejects
without replacing live facts or discarding sidecars. Roster presence never
supplies a validity map or human status.

Unit and Formation runtime sidecars have independent one-time stages. Unit
preparation needs decoded owners/Formations and admitted immutable definitions,
not Manager controls. Formation preparation uses the proven publication of its
enclosing Player before that Player's Formation Read; it infers no unrelated
slots. The sidecars retain resolved identities, derived arrays and
logical archive attachment beside the actual mutable World records. Getter
paths allocate nothing; native dirty-pointer trees or desktop presentation
subscribers are not made arbitrary action gates. Sidecars currently cover only
their documented Read/application subset. Experience/RockBand/other enum
remaps, complete gameplay post-load effects, real callback continuation and
complete actions remain open. Unavailable native snapshot/delta behavior is
not synthesized.

The adapter exposes immutable World/retained-component access, direct mutable
Unit access and matched mutable Unit/Formation plus runtime access. Runtime
identity and exact dependency checks refuse stale attachments after type,
formation or source-array changes. Dependencies are captured once; getter
comparisons allocate nothing and never reset mutable arrays. Activity, timer
and queue changes remain usable. Consumers
must preserve authoritative state through synchronous reports/scripts; a
scratch Unit must never overwrite callback changes to the real World.

Source-only tests verify buffer sharing/ownership, atomic identity-bound
controls, missing-live-input refusal and absence versus unknown. The consuming
split tests separately verify retained buffers, provenance and allocator
metadata without cloning. A real fixture assembly/profile and its startup
measurement are separate validation, not action timing. Production Civ VI
save writing is not required; complete executable live state, RNG, actions and
turns are not asserted by this ownership adapter.

## First attributed fixture assembly

The unchanged `3b733db` candidate imported the actual ff31 Core image using the
closed Native3 Read framing. It moved the source buffer and assembled all
43 Players, 1,262 Units and 1,250 Formations; 245 Cities and 1,284 Districts
remain in the retained components. Independent review `99cb71d` compared every
retained field, full Map, ordered rosters/values and allocator links. Pure
field queries independently agree on zero busy Units and 705 Units with moves.

Decode plus assembly took **0.157825059 seconds** in this one fixture run. RSS
was 99,048 KiB before and 172,648 KiB after. The 963,456 KiB process peak includes
earlier capture admission and original module images; it is not assembly-only
memory. The measurement excludes actions and artifact admission. It is a
single-run startup observation, not a gameplay performance benchmark.

[The exact evidence](../evidence/linux/loaded-world-assembly.json) preserves
the source/executable/input identities. Missing live controls and runtime
initialization remain explicit. Review found reinitialization/rebinding and
stale-runtime hazards in the original API; those are being fixed in a separate
successor, without rewriting this frozen assembly result. Native3 Read
attribution is not merged with another capture's mutable snapshot facts.

The lifecycle successor tracks a conservative revision on mutable record
access. Actual observed SavedOutput snapshot controls can be admitted once,
only while pristine, with full image SHA/length and separate runtime/facts/
configuration provenance. The decoder's original Read binding remains
unchanged. The resource adapter validates original preimages and static
resource equivalence; this is supplied-fact consistency, not cryptographic
authentication. Consuming the snapshot claim prevents replay after simulation
advancement. Reinitialization cannot restore old arrays or overwrite live
Formation state. Five focused source tests cover these authority boundaries;
actual-record mutation regressions and independent review remain separate.

`install_game_relations` additionally accepts an explicitly attributed owner
export from the same immutable snapshot. It checks the SavedOutput source,
full image identity/length, bounded owner span and decoded prefix length,
then calls the current component reader and native-order definition
initializer once. The owning data adapter validates the original owner trace
and resource preimages. This is not an offset-based general Core loader:
the byte-zero consumed boundary stays unchanged and every unclassified suffix
remains in the original image. Later relationship queries borrow the owned
authority without a revision restriction. Reinstalling old captured values
after advancement is rejected.
