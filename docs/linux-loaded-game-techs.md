LoadedState now owns the separately attributed Game.Techs Read/Validate and
nonempty InitializePostLoad projection. The byte-zero Core parser's boundary
and untouched tail remain unchanged. Complete gameplay state, action execution,
RNG and turns remain unimplemented and unvalidated.

`install_game_techs(witness, definitions, limits)` accepts a saved-source owner
once while state is pristine. It checks the source role, complete image identity
and size, nonzero trace witness, bounded nonempty owner range and exact decoded
length. The producer remains responsible for authenticating the original trace,
owner class and admitted native-order registry. The existing ChooseTech adapter
performs the concrete ff31/trace/owner attribution; the generic core installer
contains no fixture offsets.

The installer decodes the actual retained slice, remaps with that registry,
performs stable ValidateTechs filtering and enters the exact nonempty post-load
branch. Empty nodes or iteration require BuildTechs and reject before binding.
Unknown fallback prerequisites remain unknown. Version, bool, budget, mapping
and witness errors leave the graph absent. Installation does not advance the
world revision. Duplicate and late installation reject; unrelated Unit writes
preserve the imported immutable graph and its witness.

`game_techs()` returns the full typed owner, including raw costs and independent
iteration order. An absent owner is an explicit missing-input error. The getter
identifies a source projection, not final native runtime authority across other
load callbacks. A future gameplay operation that mutates this owner must update
it explicitly; this installer cannot be replayed to reset gameplay state.

Four focused LoadedState tests cover exact source admission, eight malformed
witnesses, byte/item bounds, trailing data, the BuildTechs gap, revision policy,
repeat/late rejection and persistence across unrelated formation mutation.
They are ordinary synthetic unit checks; no native fixture or frozen native
comparison candidate is claimed by this bridge. Source is retained in the scoped
commit, and existing graph/ChooseTech frozen candidate archives remain unchanged.
