#ifndef C6_LINUX_DOMAIN_METADATA_H
#define C6_LINUX_DOMAIN_METADATA_H
/* Runner-owned, independently admitted hook, resolved from RTLD_DEFAULT.
 * Return zero only after original Controller.IsExecuting=false,
 * Context.HasPendingMessages=false and the loaded-facade profile's idle checks.
 * Unknown/active worker state MUST log and _exit: no later reads/save/cleanup.
 * Runner pins hook dladdr identity, module/host/dependencies/configuration/input
 * SHA-256 identities, and retains genuine loaded owners until this probe returns.
 * This exporter does not construct game objects or initialize/advance the game. */
int c6_linux_domain_metadata_quiescent(void *game_module, void *borrowed_context);
/* Separate read-only export. No ABI/lifecycle execution approval is implied.
 * Required C6_DOMAIN_METADATA_OUTPUT, PROFILE_SHA256, SNAPSHOT_BEFORE,
 * SNAPSHOT_CONTROL, SNAPSHOT_AFTER, SNAPSHOT_REPEAT (all C6_DOMAIN_METADATA_).
 * Paths are private exclusive-create files; profile is the runner-verified full
 * rules/DLC/mod/config manifest digest. Four raw saves and two known RNG streams
 * are controls, never complete state/RNG coverage. Expected plot getter outputs
 * are comparison targets only, never simulator inputs. A partial output or
 * nonzero status is unusable. No complete gameplay state/action/turn claim. */
int c6_linux_domain_metadata_probe(void *game_module, void *borrowed_context);
#endif
