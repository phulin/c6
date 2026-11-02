#ifndef C6_LINUX_ACTION_STATE_H
#define C6_LINUX_ACTION_STATE_H
/* Caller must gate the exact XP2/host hashes and configuration, synchronize
 * the native worker, and supply a borrowed genuine core Unit::Instance.
 * NULL unit captures the two identified RNG streams only. Output is stderr.
 * These observations are not a complete affected-state/action parity proof. */
int c6_linux_action_state_capture(void *module, const void *genuine_unit_or_null,
                                 const char *stage);
int c6_linux_action_unit_queue_size(void *module, const void *genuine_unit,
                                   unsigned *out);
#endif
