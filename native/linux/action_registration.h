#ifndef C6_LINUX_ACTION_REGISTRATION_H
#define C6_LINUX_ACTION_REGISTRATION_H
/* Pump the existing genuine session handshake while original CoreActive=false.
 * No message objects, peer identities, events or readiness flags are created. */
int c6_linux_action_registration(void *game_module, void *borrowed_context);
#endif
