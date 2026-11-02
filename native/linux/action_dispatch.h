#ifndef C6_LINUX_ACTION_DISPATCH_H
#define C6_LINUX_ACTION_DISPATCH_H
#include <stdbool.h>
/* Caller owns initialized native controller/cache, armed trace and all providers.
 * Selects an actual legal empty-queue SkipTurn actor in original iterator order.
 * Control mode uses identical queries/snapshots but sends no request. */
int c6_linux_action_dispatch(void *module,void *borrowed_context,bool execute);
#endif
