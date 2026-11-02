#ifndef C6_LINUX_ACTION_LIFECYCLE_H
#define C6_LINUX_ACTION_LIFECYCLE_H
#include <stdbool.h>
/* Original controller Initialize / InitializeCache, actual actor/cache queries,
 * then original Uninitialize while XP2 and all native owners remain alive. */
int c6_linux_action_prepare_observe(void *game_module, void *borrowed_context);
int c6_linux_action_prepare_dispatch(void *game_module, void *borrowed_context, bool execute);
int c6_linux_action_prepare_registered_dispatch(void *game_module, void *borrowed_context, bool execute);
#endif
