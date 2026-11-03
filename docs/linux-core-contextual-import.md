# Contextual all-Player saved-wire trial

The frozen Rust candidate from `4095ed2` imported all 43 Player records and
GlobalAi from byte zero in the closed `native-3` profile. Each role's complete
Player.Manager and GlobalAi re-encoding matched its own original bytes and
dynamic-item charge exactly. Independent acceptance is pending; this is a
saved-wire trial, not complete live state or gameplay equivalence.

The genuine profile supplied a fresh Read of Core `ff31f4c5…` and two separate
own Writes of Core `85d858ef…`. All three roles reached Player.Manager end
70,566,134 and GlobalAi end 70,934,595, consuming 10,665,369 dynamic items.
Each imported roster contains 43 Players, 1,262 Units, 245 Cities,
1,284 Districts and 1,250 Formations. The remaining 9,722,867 bytes are a
borrowed, unparsed tail beginning at Barbarian; this profile does not use later
codec availability to skip that boundary.

The executable is SHA-256
`a3bf423c5b71d9d1b13ef04cf0a1b89f60508c54648e1c256c2d7392bec2c57d`.
Its complete 1,421-file committed source archive, executable and manifests are
preserved under `target/frozen/core-contextual-import/4095ed2040f582884edf251d94048f8e8b92212b/`.
[The hash/coverage record](../evidence/linux/core-contextual-import.json)
preserves role-specific identities and digests. The
[candidate protocol](linux-core-contextual-candidate.md) admits bounded raw
capture artifacts once, retains actual initialization/vector provenance, and
binds each Military record to its absolute parser cursor. It never derives
framing from a convenient default or borrows a different stream's context.

Both initial attempts failed `Provenance` because a path adapter supplied
relative source names where the recorded gate used absolute names. Those
failures remain preserved. A versioned path-only correction kept the same
source, executable and original capture facts. The corrected `native-2` attempt
then failed `MilitaryWire`: its first Military14 record reaches nested
OperationDef5, whereas the current codec supports OperationDef7. That older
nested schema remains explicitly unsupported. No version was coerced and no
native-2 output role was substituted for its input Read.

No bytes are masked. The recorded 89,198-byte raw Save-to-Load difference remains
reported; differing tail digests are retained. These observations supply wire
framing while Rust initialization is incomplete. They do not reproduce native
Read fixups, construct an executable World, establish a live database snapshot,
prove RNG transitions, or implement actions or turns. No 85d input Read context
is claimed by this profile.
