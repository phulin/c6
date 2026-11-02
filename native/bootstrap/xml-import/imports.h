#ifndef C6_BOOTSTRAP_ORIGINAL_IMPORTS_H
#define C6_BOOTSTRAP_ORIGINAL_IMPORTS_H
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
/* Requires fingerprinted EXE mapped at preferred base. Binds genuine CRT,
 * KERNEL32 and DatabaseDLL providers only; never invokes EXE startup.
 * Returns positive count, or zero on failure. Caller must gate providers. */
int bootstrap_bind_original_imports(HMODULE original);
#endif
