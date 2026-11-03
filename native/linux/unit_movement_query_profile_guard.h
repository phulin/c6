#ifndef C6_LINUX_UNIT_MOVEMENT_QUERY_PROFILE_GUARD_H
#define C6_LINUX_UNIT_MOVEMENT_QUERY_PROFILE_GUARD_H
#include <stdint.h>
/* New caller scope only. Field reads prove retained owners/main/native lock;
 * they do not assert a policy boundary, empty queues, pure queries or Lua safety.
 * Original facade acquisition/release and movement queries belong to the separate
 * reviewed movement-only caller. Debugger use is a separate supervisor choice;
 * this getter profile introduces no VM/frame gate. */
int c6_linux_unit_movement_query_profile_admission_begin(void *module,void *context);
int c6_linux_unit_movement_query_profile_caller_idle(void *module,void *context);
int c6_linux_unit_movement_query_profile_admission_end(void *module,void *context);
int c6_linux_unit_movement_query_pre_acquisition(void *module,void *context,const void *game);
int c6_linux_unit_movement_query_locked_begin(void *module,void *context,const void *game,uintptr_t core_base,long main_entry_tid);
int c6_linux_unit_movement_query_retained_owner(void *module,void *context,const void *game);
int c6_linux_unit_movement_query_locked_end(void *module,void *context,const void *game);
int c6_linux_unit_movement_query_released_owner(void *module,void *context,const void *game);
int c6_linux_unit_movement_query_profile_identity(void *module,void *context,uintptr_t *base,const void **game,long *main_tid);
int c6_linux_unit_movement_query_profile_probe(void *module,void *context);
#endif
