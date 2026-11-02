# Original localization bootstrap fixture

This boundary initializes the supplied `Localization_FinalRelease.dll` (SHA-256
`0becd5c425a3d6edc88f5315c01af3dd30dd7cb158271d56f4bba7ff2bb4e056`)
using original content and code. It does not establish initialized game, turn,
or localization subsystem equivalence.

Reproduce with `python3 tools/analysis/localization_probe.py`. Use
`--expected-configuration SHA256` to refuse a changed execution configuration.
The compact result is `evidence/bootstrap/localization.json`; the pinned source
inventory is `tools/analysis/localization_sources.json`. No proprietary text or
binary is included in this repository.

The explicit fixture selects `en_US`; this is a fixture choice, not a recovered
user preference. It executes the original localization schema and language data,
122 sorted `Base/Assets/Text/en_US/*.xml` files, then original postprocessing SQL.
Each original source SHA-256 is checked before execution and again by the native
adapter. UTF-8 BOM removal is the only input transformation. These files all
import successfully through the original executable's XML importer, with its
real constructors and original DatabaseDLL connection. The source sequence is a
bounded Base English fixture, not the original package, DLC, or mod resolver.

The genuine database connection uses
`file:Localization?mode=memory&cache=shared`, original flags `0x46`, and stays
alive for process lifetime. Callers must first initialize DatabaseDLL, load the
localization DLL normally, map the fingerprinted original executable at its
preferred base, and bind the genuine restricted imports required by its parser.
`bootstrap_original_localization(executable, manifest, icu_path)` returns zero on
success. Its manifest format is `sql|xml TAB SHA256 TAB absolute-Windows-path`.
The caller gates the source sequence/configuration; the helper gates each file,
the loaded localization DLL, and the exact ICU data file.

## Verified ABI and order

The Windows x64 export at RVA `0x192f0` is
`bool Initialize(const char16_t *icu_file, IDatabaseConnection1 *connection,
const char *language, bool option, ILocalizationLogger *logger)`.
The fixture passes original `Base/Assets/Text/icudt56l.dat`, `en_US`, false,
and a null logger. Disassembly shows the first argument reaches the UTF-16 file
stat/open/map path; mapped size/data globals are RVAs `0x18c6b8`/`0x18c6b0`.
It registers ICU data, initializes ICU, and calls `InitializeDictionary` at
`0x19530`. That function constructs the dictionary, stores its pointer at
`0x18c6c0`, then calls `SetCurrentLanguage` at `0x196f0`.

The original global `Lookup` export (`0x196d0`) returns a String through the
Windows x64 hidden result pointer. Its `GetBaseString` preserves the key; resolved
output comes from `toUTF8(String*, unsigned& length, int form)` at `0xecb0`.
Forms are one-based. Form 1 of `LOC_TECH_POTTERY_NAME` returns the original
seven-byte text `Pottery`; the original String destructor releases the result.
The probe reserves bounded storage for the original String constructor, not a
synthetic replacement object.

## Checks and limits

The standalone probe builds with MinGW `-Wall -Wextra -Werror` and passes native
checks under Wine: null ICU initialization and null connection dictionary
initialization return false with no dictionary; normal initialization succeeds;
the known key resolves; invalid `c6_invalid_locale` returns false and retains
`en_US`. A configuration mismatch is rejected before native execution.
The test uses no localization logger, does not cover all formatting/languages,
and does not prove completeness of expansion-specific tags. A failed helper
terminates the diagnostic; failure cleanup is not a reusable recovery API.

Raw proprietary disassembly remains in the private sibling repository at
`../c6-ghidra/evidence/localization/exports.asm`.
