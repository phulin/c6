#ifndef C6_SAVED_CONFIGURATION_H
#define C6_SAVED_CONFIGURATION_H
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
/* Experimental exact XP2Benchmark host configuration reader attempt.
 * Current standalone execution faults before a verified restoration; not wired
 * into the runtime. This is not gameplay state loading.
 * Original host+Core module identities/providers must already be gated by caller.
 * Refuses existing native singletons; invokes native key initializers, getters,
 * GameConfig reader and UserConfig reader. Keeps original globals alive.
 * Returns 1 only after consumed boundaries and native view invariants match. */
/* Requires genuine host CRT onexit tables initialized. Runs original allocator
 * static initializer once; accepts only the expected already constructed vptr. */
int bootstrap_saved_configuration_prepare_allocator(HMODULE host);
int bootstrap_saved_configuration_restore(HMODULE host,HMODULE core,const wchar_t *save);
#endif
