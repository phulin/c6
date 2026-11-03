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
any control mutation; omitted replacement controls clear prior installed
values rather than silently retaining them. Roster presence never supplies a
validity map or human status.

Unit and Formation runtime sidecars are built once by the source-backed
initialization modules. They retain resolved identities, derived arrays and
logical archive attachment beside the actual mutable World records. Getter
paths allocate nothing; native dirty-pointer trees or desktop presentation
subscribers are not made arbitrary action gates. Sidecars currently cover only
their documented Read/application subset. Experience/RockBand/other enum
remaps, complete gameplay post-load effects, real callback continuation and
complete actions remain open. Unavailable native snapshot/delta behavior is
not synthesized.

The adapter exposes immutable World/retained-component access, direct mutable
Unit access and matched mutable Unit/Formation plus runtime access. Runtime
identity checks refuse stale attachments after identity changes. Consumers
must preserve authoritative state through synchronous reports/scripts; a
scratch Unit must never overwrite callback changes to the real World.

Source-only tests verify buffer sharing/ownership, atomic identity-bound
controls, missing-live-input refusal and absence versus unknown. The consuming
split tests separately verify retained buffers, provenance and allocator
metadata without cloning. A real fixture assembly/profile and its startup
measurement are separate validation, not action timing. Production Civ VI
save writing is not required; complete executable live state, RNG, actions and
turns are not asserted by this ownership adapter.
