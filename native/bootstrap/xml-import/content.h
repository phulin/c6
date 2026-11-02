#ifndef C6_BOOTSTRAP_ORIGINAL_CONTENT_H
#define C6_BOOTSTRAP_ORIGINAL_CONTENT_H
#include "xml.h"
/* Bounded manifest: database TAB format TAB phase TAB SHA256 TAB path
 * followed by optional TAB action_id (required for mod action transactions).
 * Connections must be genuine, empty, caller-owned original DB objects.
 * Registers native Make_Hash, executes original source import and transactions.
 * Returns zero only if every file and phase completes. Caller keeps DB objects. */
int bootstrap_original_content_manifest(HMODULE executable, void *configuration,
                                        void *gameplay, const wchar_t *manifest);
/* Gameplay mod content requires the actual original context to insert its
 * embedded database data. Method +0x50 must match target GameCore RVA590600. */
int bootstrap_original_content_manifest_with_context(HMODULE executable,
    void *configuration, void *gameplay, const wchar_t *manifest,
    void *gamecore_context);
#endif
