#ifndef C6_BOOTSTRAP_LOCALIZATION_H
#define C6_BOOTSTRAP_LOCALIZATION_H
#include "../xml-import/xml.h"
/* Owns a genuine named-memory Localization DB for process lifetime.
 * Caller gates original EXE/DatabaseDLL; manifest and ICU bytes are SHA-gated.
 * Explicit bounded en_US fixture; no game initialization claim. */
int bootstrap_original_localization(HMODULE executable, const wchar_t *manifest,
                                    const wchar_t *icu_path);
#endif
