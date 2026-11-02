#ifndef C6_LINUX_CORE_COMPONENT_H
#define C6_LINUX_CORE_COMPONENT_H
#include <stdbool.h>
/* Separate conditional Core experiment, never accepted full-host Request parity.
 * Caller gates exact module/config/input identities, initializes genuine native
 * controller/cache and arms the unchanged action trace/policy guards. No live
 * Request is sent. Fresh original MakeRequest/UnitOperation serialization feeds
 * native QueueMessage only in execute mode, followed by original worker+sync.
 * Original executing/suspended predicates and original worker Join establish
 * completion before observation. A deadline failure terminates the isolated
 * process with code240 before post-state/Save/Uninitialize (no cleanup claim).
 * Required C6_CORE_COMPONENT_SNAPSHOT_{BEFORE,AFTER,REPEAT} and
 * C6_CORE_COMPONENT_COMMAND are private exclusive-create artifact paths.
 * Return zero means bounded helper completion, not complete affected-state or
 * RNG coverage, branch restoration, next-decision readiness or gameplay parity.
 */
int c6_linux_core_component(void *module,void *borrowed_context,bool execute);
#endif
