#ifndef C6_BOOTSTRAP_SCRIPT_SYSTEM_H
#define C6_BOOTSTRAP_SCRIPT_SYSTEM_H
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
/* Caller must fingerprint EXE and every supplied DLL/runtime, map EXE at
 * 0x140000000 and bind its genuine CRT/kernel imports before calling.
 * Constructs original critical section, Lua manager and ScriptSystem only.
 * No replacement VM, host entry point or broad global constructors. */
void *bootstrap_original_script_system(HMODULE executable);
#endif
