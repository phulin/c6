# Linux Plot collection versions 5 and 6

The Map reader now accepts Plot collection versions 5 and 6. This removes one typed decoding boundary in the original retained `a66736f0…` stream; it does not establish complete gameplay state, actions, or turns. Map remains version 4, and every other nested schema keeps its existing version gate.

The target is Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. [`plot_versions_audit.py`](../tools/linux/plot_versions_audit.py) checks its complete collection reader (`0x2a472c0`), Plot reader (`0x2bc8820`), CityAttributes reader (`0x2bc8090`), version getter, constructors/initializers, and matching target DWARF. [`plot-versions.json`](../evidence/linux/plot-versions.json) records those identities. Discovery uses cru121's pinned `5af389445118bed2bdebf7a8a42b219c8fba5175` findings, CC BY 4.0; target instructions govern the implementation.

Both versions have the field order and minimum 55-byte record described in [the Map contract](linux-map-state.md). The collection reads one signed version, one unsigned count, resizes, then passes the same version to every Plot. There are no per-record version words. FeatureLayout (version ≥2), appeal (version ≥4), 24-bit flags (version ≥5), and the obsolete bool (only versions 3–4) have identical paths for 5 and 6. CityAttributes Read does not branch on the supplied version.

There is one Read application difference: after consuming flags, the target compares the version with 5 at `0x2bc8bf2`. For version 5 it explicitly clears native FeatureVolatile (object offset `0x4d`, bit 6), regardless of serialized flag bit 16. Version 6 keeps flag bit 16. The Rust decoder applies this same clear. Tests retain a raw bit16=1 witness and verify both results against that source-derived contract; they are synthetic structural tests, not a new native differential oracle. Bits 17–23 remain explicitly unsupported for both versions under the existing bounded codec policy.

`Map::decode_prefix_with_version_and_usage` returns the Plot collection version alongside the typed Map, consumed bytes, item charge, and allocation charge. Existing APIs keep their signatures. The caller must retain that provenance and the immutable original input when comparing bytes. Encoding still emits current version 6; no version 5 writer or byte-identical v5 rewrite is asserted. The explicit Read clear must not be hidden by masking raw comparisons.

Native Plot construction nulls links and calls Initialize with coordinates −9999/−9999; Map initialization supplies actual coordinates. Initialize clears the city allocation index and relevant flags, but preserves native found-location bit4 of byte `0x4d`; its complete live provenance is not established here. CityAttributes Initialize sets IDs/player/wonder to their invalid sentinels and sets validDomains and plotYieldEffect to zero. Read overwrites the serialized IDs/player/wonder but does not write those last two live fields. This source evidence does not reconstruct native allocation, definition resolution, property-pointer lifecycle, or subsequent live cache updates. Omitted state remains a gameplay reconstruction gap.

Reproduce the static gate with:

```sh
python3 tools/linux/plot_versions_audit.py --output /tmp/plot-versions.json
cargo test -p c6-core map:: --lib --offline
```

The focused checks cover v5/v6 flag behavior, optional CityAttributes, signed extrema, every truncated record prefix, byte budgets, unknown versions/spare bits, collection version provenance, and existing Map budget/tail behavior. Raw prefix progression must be measured from an immutable committed-source candidate; no offset transfer from another stream is used.

The retained raw `a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa` (76,556,938 bytes) was probed using source revision `dd19808b9941da4ad0d75a0142721f165378ad34`, frozen in a complete committed-source archive before compilation. The report records archive, generated probe/manifest/lock and executable identities. The Rust-only candidate advances past Plot collection v5 and stops at `Map(UnsupportedVersion { expected: 7, actual: 6 })`. A GDB breakpoint on this archived Rust candidate's `Reader::version(expected=7)` identifies `FeatureComponent::read`, at the start of its framed child reader (`pos=0`). The next true boundary is therefore **Map.Component.Feature v6**. No Feature bytes were skipped and no first-Player or whole-Map completion is claimed. The optional `--raw-input` audit argument reproduces this committed-source prefix probe with a fresh retained candidate; it does not execute original game code.
