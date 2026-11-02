#ifndef C6_LINUX_MOD_ACTIVATION_H
#define C6_LINUX_MOD_ACTIVATION_H
/* Original host owns this context; never pass it to DllDestroyGameContext.
 * Call only before creating any standalone context, with host services ready.
 * A successful bootstrap is not XP2 activation or initialized-game parity. */
int c6_linux_mod_register_events(void);
int c6_linux_mod_framework_initialize(void **borrowed_context);
int c6_linux_mod_framework_configure(void);
int c6_linux_mod_framework_shutdown(void);
int c6_linux_mod_client_fixture_shutdown(void);
#endif
