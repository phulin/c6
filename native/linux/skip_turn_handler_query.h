#ifndef C6_LINUX_SKIP_TURN_HANDLER_QUERY_H
#define C6_LINUX_SKIP_TURN_HANDLER_QUERY_H
#include <stdbool.h>
#include <stdint.h>
typedef struct {
    bool unit_present;
    bool allowed;
    int32_t operation_id;
    uint32_t operation_hash;
} C6LinuxSkipTurnHandlerResult;
/* Separate handler-only observer. Runner must gate the pinned module, host,
 * dependencies, configuration, sources and static audit, retain the genuine
 * loaded Context/Game admission owners, and verify Unit container membership.
 * unit and optional plot are borrowed actual objects. NULL unit is recorded
 * without invoking the handler. NULL plot is a genuine nullable argument.
 * Run for every actual Unit, including unplaced Units; bracket with complete
 * Saves, known RNG checks and independent repeat. No full Manager legality,
 * action, request, scripts, reporting or turn support is implied.
 * Unknown work exits immediately, without cleanup or post-state reads. */
int c6_linux_skip_turn_handler_query(void *module, void *context,
    const void *unit, const void *plot, bool visible_only,
    C6LinuxSkipTurnHandlerResult *result);
#endif
