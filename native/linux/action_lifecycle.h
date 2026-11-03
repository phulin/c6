#ifndef C6_LINUX_ACTION_LIFECYCLE_H
#define C6_LINUX_ACTION_LIFECYCLE_H
#include <stdbool.h>
/* Original controller Initialize / InitializeCache, actual actor/cache queries,
 * then original Uninitialize while XP2 and all native owners remain alive. */
int c6_linux_action_prepare_observe(void *game_module, void *borrowed_context);
int c6_linux_action_prepare_dispatch(void *game_module, void *borrowed_context, bool execute);
int c6_linux_action_prepare_registered_dispatch(void *game_module, void *borrowed_context, bool execute);
int c6_linux_action_prepare_core_component(void *game_module, void *borrowed_context, bool execute);
/* Separate read-only movement-cost callback under original controller/cache
 * ownership and existing trace guards; no command dispatch or worker wake. */
int c6_linux_action_prepare_movement_cost(void *game_module, void *borrowed_context);
#endif
