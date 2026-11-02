# Original Linux localization fixture

This fixture uses original localization and database code exported by the supplied
Linux host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
It is a separate execution profile from the Windows localization DLL. It does not
establish complete localization, initialized gameplay, actions or turns.

The pinned reference function index was consulted first; it names the Linux
`Application::Data::LocalizationDatabase` lifecycle and `Localization` methods.
Fresh instructions in the supplied host establish the invoked signatures,
allocation and source paths. There is no separate Linux localization shared
library in this fixture: the actual methods and dictionary global resolve from
the genuine `Civ6` executable after its original static constructors.

`tools/linux/localization-sources.json` pins 115 supplied Linux Base English XML
files, three original SQL files and `icudt56l.dat`. This is an explicit `en_US`
fixture, not recovered user preferences or DLC/mod selection. Source counts and
identities are Linux-specific. The source gate checks hashes and exact English
inventory before original execution; runtime configuration also records the host,
GameCore, dependencies, adapter, gate and prepared path identities.

The original database initializer takes a cache path but its Bootstrap method
hardcodes relative mixed-case source paths. `localization_gate.prepare` creates
an exact read-only symlink fixture beneath `target/linux-localization`, preserving
those original path spellings while pointing at supplied lowercase assets. The
native adapter temporarily changes to the fixture launch directory for the
original database initializer, then restores its working directory. Original
assets are neither patched nor written. Original schema, data, XML importer,
postprocessing and localization initialization execute without reconstructed SQL
or a substitute dictionary. Optional UI/user text trees and the cache must remain absent; the gate rejects
these paths to prevent original Bootstrap consuming ungated inputs.

The adapter requires the genuine App and original Database service already
initialized. It allocates the original localization database using the `0x18`
allocation size from `Civ6App::SetupDatabase`, invokes its constructor and
parameterized initializer, and keeps its real connection alive. Original
`Localization::Initialize` receives the original ICU path, connection, `en_US`,
false and a null logger. Lookup uses the Linux hidden result pointer ABI shown
by original `Localization::Lookup` instructions, calls original `String::toUTF8`
and destroys the original result. Bounded aligned storage reserves room for the
original object rather than defining a replacement String layout.

Cleanup calls original `Localization::Shutdown`, verifies the dictionary global
is null, invokes the original database destructor and deallocates it before the
host App and Database service teardown. The database fixture has private ownership
and does not populate App's localization database pointer; consumers requiring
that getter remain a separate boundary.

Reproduce through the native runtime:

```sh
python3 tools/linux/runtime_probe.py --localization \
  --output evidence/linux/localization.json
```

The original native checks include rejection of null ICU and null dictionary
connection inputs without a dictionary, known key `LOC_TECH_POTTERY_NAME` resolving
to seven UTF-8 bytes matching `Pottery`, and rejection of an invalid locale while
retaining `en_US`. The [native result](../evidence/linux/localization.json) passes with exit zero,
three original context creation/destruction cycles, original localization shutdown
with a null dictionary, host services cleanup and GameCore handle close.
[ABI anchors](../evidence/linux/localization-abi.json) record native addresses and
private evidence hashes. The original context and localization lifecycle must
complete before the diagnostic passes. StartupGame and save loading remain separately
reported runtime boundaries.

Private raw instructions and debug-query results remain under
`../c6-ghidra/evidence/linux-localization/`. Host debug queries returned no matching
type DIEs; the implementation does not infer host object layouts from GameCore
DWARF or Windows RVAs. [Reference reuse](reference-reuse.md) records attribution
and the limits of name/address overlap.

Run `python3 tools/linux/localization_boundaries.py --output evidence/linux/localization-boundaries.json`
to reproduce [pre-execution refusals](../evidence/linux/localization-boundaries.json)
for altered ICU/English bytes, an extra English source, optional UI text, an
existing cache and a configuration identity mismatch. The checker also confirms
unchanged source acceptance. These refusal checks execute no original code.
