#ifndef C6_LINUX_MOVEMENT_COST_H
#define C6_LINUX_MOVEMENT_COST_H
/* Mandatory runner-owned exported guard, resolved with dlsym(RTLD_DEFAULT).
 * int(void *game_module, void *borrowed_context): return 0 only after the exact
 * original Controller.IsExecuting=false and Context.HasPendingMessages=false
 * plus this caller profile's established idle predicates are observed.
 * Active/unknown worker state must log and _exit the isolated process immediately:
 * no post-state observation, Save, cleanup or Uninitialize may follow. The probe
 * also _exits on any nonzero return as a fail-closed guard-contract violation.
 * The runner gates the hook's preload/module/source/hash and dladdr identity;
 * mere presence/logged pointers do not establish native worker quiescence.
 * This own helper never replaces an original engine query. */
int c6_linux_movement_cost_quiescent(void *game_module, void *borrowed_context);
/* Read-only mechanics experiment on genuine loaded Unit/Formation/Plot owners.
 * Caller gates exact Linux XP2/host/dependencies/configuration/input identities,
 * proves the native worker quiescent, and retains the policy/dispatch guards.
 * No movement action, worker wake, turn processing or private-field patch occurs.
 * C6_MOVEMENT_COST_PROFILE_SHA256 identifies the runner's full rules/DLC/mod/
 * configuration manifest, validated by the runner before calling this probe.
 * C6_MOVEMENT_COST_OUTPUT and SNAPSHOT_{BEFORE,CONTROL,AFTER,REPEAT} are private
 * exclusive-create paths. C6_MOVEMENT_COST_EXECUTE=1 enables original cost calls;
 * absent/0 performs the same discovery/fact export without the cost calls.
 * Default maximum is 256 genuine formations, bounded by 4096; truncation is reported.
 * Output contains genuine query facts plus scalar phase/full-cost observations.
 * Two known RNG snapshots and raw Core saves do not establish complete live-state
 * or RNG coverage. Zero return means probe completion, not native parity.
 * No native execution is authorized until separate ABI/lifecycle review.
 */
int c6_linux_movement_cost_probe(void *game_module, void *borrowed_context);
#endif
