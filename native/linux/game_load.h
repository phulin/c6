#ifndef C6_LINUX_GAME_LOAD_H
#define C6_LINUX_GAME_LOAD_H
/* Diagnostic lower Core stream boundary. Caller owns the framework Context and
 * must retain genuine network/session/configuration/content providers. This is
 * not the host LoadGameViewState finalization or an action-ready game. */
int c6_linux_game_load(void *borrowed_context);
/* Movement fixture gate: exact original controller/context idle predicates and
 * acquired/released Game facade lock barrier. Returns zero only when proven;
 * unknown/active state terminates the isolated process before poststate/teardown.
 * This is not a general worker-completion or native service implementation. */
int c6_linux_movement_cost_quiescent(void *game_module,void *borrowed_context);
#endif
