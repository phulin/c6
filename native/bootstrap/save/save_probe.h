#ifndef C6_BOOTSTRAP_SAVE_PROBE_H
#define C6_BOOTSTRAP_SAVE_PROBE_H
#include <windows.h>
#include <stdio.h>
/* Diagnostic only: requires caller's verified DLL and genuine StartupGame setup.
 * Nonzero means unavailable/refused; zero does not establish initialized gameplay. */
int bootstrap_save_probe(HMODULE image, void *context, const wchar_t *path);
/* Optional crash-filter hook; observes fields of the active original stream. */
void bootstrap_save_probe_report_fault(FILE *output);
#endif
