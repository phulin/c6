#ifndef C6_BOOTSTRAP_SAVE_PROBE_H
#define C6_BOOTSTRAP_SAVE_PROBE_H
#include <windows.h>
/* Diagnostic only: requires caller's verified DLL and genuine StartupGame setup.
 * Nonzero means unavailable/refused; zero does not establish initialized gameplay. */
int bootstrap_save_probe(HMODULE image, void *context, const wchar_t *path);
#endif
