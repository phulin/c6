#ifndef C6_LINUX_ACTION_OBSERVER_H
#define C6_LINUX_ACTION_OBSERVER_H
/* Read-only actor/SkipTurn legality query in a genuinely loaded, host-owned
 * XP2 context. Caller serializes this call between original Core Saves and
 * retains all original services/session. No request, drain or Update occurs. */
int c6_linux_action_observe(void *game_module, void *borrowed_context);
#endif
