#ifndef C6_LINUX_FULL_MANAGER_PROFILE_GUARD_H
#define C6_LINUX_FULL_MANAGER_PROFILE_GUARD_H
#include <stdint.h>
/* New caller scope only. Field reads prove retained owners/main/native lock;
 * they do not assert a policy boundary, empty queues, pure queries or Lua safety.
 * Original facade acquisition/release and the predicate belong to the separate
 * reviewed caller. External debugger frame/VM admission remains mandatory. */
int c6_linux_full_manager_profile_admission_begin(void *module,void *context);
int c6_linux_full_manager_profile_caller_idle(void *module,void *context);
int c6_linux_full_manager_profile_admission_end(void *module,void *context);
int c6_linux_full_manager_pre_acquisition(void *module,void *context,const void *game);
int c6_linux_full_manager_locked_begin(void *module,void *context,const void *game,uintptr_t core_base,long main_entry_tid);
int c6_linux_full_manager_retained_owner(void *module,void *context,const void *game);
int c6_linux_full_manager_locked_end(void *module,void *context,const void *game);
int c6_linux_full_manager_released_owner(void *module,void *context,const void *game);
int c6_linux_full_manager_profile_probe(void *module,void *context);
#endif
