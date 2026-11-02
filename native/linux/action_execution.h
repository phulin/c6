#ifndef C6_LINUX_ACTION_EXECUTION_H
#define C6_LINUX_ACTION_EXECUTION_H
/* Phase one observes real cached/live player dispatch preconditions only.
 * Caller owns loaded XP2 Context and original services and Save guards. */
int c6_linux_action_execution(void *game_module, void *borrowed_context);
#endif
