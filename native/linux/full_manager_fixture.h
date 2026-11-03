#ifndef C6_LINUX_FULL_MANAGER_FIXTURE_H
#define C6_LINUX_FULL_MANAGER_FIXTURE_H
#include <stdbool.h>
#include <stdint.h>
/* Separate original-call fixture, never the passive metadata helper. The sole
 * runner must gate exact mapped target/host/configuration/rules/DLC/mod/source
 * identities before entry. It retains module/context/native Game owners and
 * witnesses main_entry_tid at its intercepted original main entry. No OS TID is
 * converted to either the native core u32 owner token or Lua ASL ThreadHANDLE.
 * Callbacks below are runner-owned, field-only admission/observation services;
 * they must not call original getters, locks, Lua, callbacks, policies or turns.
 * Nonzero admission/ownership results are fatal before further original calls.
 */
typedef int (*C6FullManagerOwnerGuard)(void *module,void *context,const void *game);
typedef int (*C6FullManagerLockedBegin)(void *module,void *context,const void *game,
    uintptr_t core_base,long main_entry_tid);
typedef enum {
    C6_FULL_MANAGER_FACADE_ACQUIRED=1,
    C6_FULL_MANAGER_BEFORE=2,
    C6_FULL_MANAGER_CONTROL=3,
    C6_FULL_MANAGER_PREDICATE_ENTER=4,
    C6_FULL_MANAGER_PREDICATE_RETURN=5,
    C6_FULL_MANAGER_AFTER=6,
    C6_FULL_MANAGER_REPEAT=7,
    C6_FULL_MANAGER_BEFORE_RELEASE=8,
    C6_FULL_MANAGER_RELEASED=9
} C6FullManagerPhase;
typedef struct {
    C6FullManagerPhase phase;
    void *module,*context;
    const void *game,*facade,*manager,*unit,*operation;
    uintptr_t core_base;
    int32_t actor_owner,operation_id;
    uint32_t actor_full_id;
    bool visible_only,predicate_returned,allowed;
} C6FullManagerObservation;
typedef int (*C6FullManagerObserve)(const C6FullManagerObservation *observation);
typedef struct {
    void *module,*context;
    const void *expected_game,*expected_unit;
    int32_t actor_owner;
    uint32_t actor_full_id;
    bool visible_only;
    long main_entry_tid;
    const char *profile_sha256,*output_path;
    /* Optional immutable Save diagnostic paths, before/control/after/repeat.
     * Either all four are present or all absent. Save results are recorded;
     * no byte/RNG equality requirement controls this predicate call. */
    const char *diagnostic_save_paths[4];
    /* pre_acquisition checks real initialized root linkage, zero native lock,
     * no worker and a genuine external-policy decision boundary. */
    C6FullManagerOwnerGuard pre_acquisition;
    C6FullManagerLockedBegin locked_begin;
    /* retained_owner verifies same live owners, main thread, no competing
     * mutator and originally acquired core token/depth at original-call
     * endpoints. It MUST retain legitimate queued/script/gameplay effects;
     * pending work alone is not evidence of ownership failure. It must not
     * clear queues, advance decisions, or require the pre-query state. */
    C6FullManagerOwnerGuard retained_owner,locked_end,released_owner;
    /* Passive affected-state/lifecycle evidence. game remains borrowed;
     * facade is owned by this fixture only, until BEFORE_RELEASE. Unit is
     * borrowed only before PREDICATE_ENTER; it may be removed by callbacks.
     * operation is live only at ENTER/RETURN. Do not dereference unit after
     * ENTER merely because its address is retained. RELEASED has facade NULL.
     * A nonzero observation result is recorded; safe cleanup still needs the
     * independent ownership guards. No observer result is a legality answer. */
    C6FullManagerObserve observe;
} C6FullManagerFixtureAdmission;
typedef struct {
    bool predicate_invoked,predicate_returned,allowed,facade_released;
    int32_t operation_id;
    uint32_t operation_hash;
    int save_status[4],observation_status;
} C6FullManagerFixtureResult;
/* Stable debugger marker, no original calls or VM/lock access. An external
 * all-stop hardware observer owns its natural CallTestAll checkpoints. */
void c6_linux_full_manager_fixture_marker(const C6FullManagerObservation *observation);
int c6_linux_full_manager_fixture_run(const C6FullManagerFixtureAdmission *admission,
    C6FullManagerFixtureResult *result);
#endif
