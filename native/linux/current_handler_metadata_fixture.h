#ifndef C6_LINUX_CURRENT_HANDLER_METADATA_FIXTURE_H
#define C6_LINUX_CURRENT_HANDLER_METADATA_FIXTURE_H
#include <stdint.h>
/* Caller preparation only. The enclosing, independently admitted caller must
 * witness original main entry and acquire a genuine Game facade before begin.
 * main_tid is the caller's own main-entry SYS_gettid observation, never an
 * invented conversion of the native lock owner. No native execution is admitted
 * by this interface. Module/source identities belong in the future profile. */
int c6_linux_current_handler_metadata_fields_begin(void *module, void *context,
    const void *game, uintptr_t core_base, long main_tid);
int c6_linux_current_handler_metadata_quiescent(void *module, void *context,
    const void *game);
int c6_linux_current_handler_metadata_fields_end(void *module, void *context,
    const void *game);
/* Separate unlocked caller/lifecycle boundaries; not the helper callback. */
int c6_linux_current_handler_metadata_admission_begin(void *module,void *context);
int c6_linux_current_handler_metadata_caller_idle(void *module,void *context);
int c6_linux_current_handler_metadata_admission_end(void *module,void *context);
int c6_linux_current_handler_metadata_probe(void *module,void *context);
#endif
