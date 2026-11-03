#ifndef C6_LINUX_UNIT_MOVEMENT_QUERY_FIXTURE_H
#define C6_LINUX_UNIT_MOVEMENT_QUERY_FIXTURE_H
#include <stdint.h>
/* Future runner owns genuine Context.GetGame/Release and exact module, host,
 * dependency, configuration, rules/input and original-lock admission. This new
 * three-argument field-only guard must retain the actual Game owner/native token
 * on the witnessed main thread with no competing worker. It must _exit on any
 * unknown ownership before further original calls. It is NOT an old two-arg
 * quiescence alias, empty-queue predicate or an authorization to execute. */
typedef int (*C6UnitMovementQueryOwnerGuard)(void *,void *,const void *);
typedef struct {
    void *module;
    void *borrowed_context;
    const void *retained_game;
    C6UnitMovementQueryOwnerGuard retained_owner;
    const char *profile_sha256;
    /* Candidate reference image identity, not a claim about a produced Save. */
    const char *candidate_core_sha256;
    const char *inputs_path;
    const char *answers_path;
    uint32_t expected_units;
} C6UnitMovementQueryAdmission;
/* Complete stored Units, without filtering movement, placement or delayed death.
 * Pass0 exports original input fields; pass1 calls Unit.GetMaxMoves/HasMoved;
 * pass2 repeats inputs. Answers are physically separate from candidate inputs.
 * No action, AI policy, turn, save/load, flush or queue-drain call occurs here.
 * Zero is a complete capture, never native/Rust parity or full purity. */
int c6_linux_unit_movement_query_capture(const C6UnitMovementQueryAdmission *);
#endif
