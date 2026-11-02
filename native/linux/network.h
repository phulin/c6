#ifndef C6_LINUX_NETWORK_H
#define C6_LINUX_NETWORK_H
/* Exact hash gated Linux host fixture; no transport/session replacements. */
int c6_linux_network_initialize(void *application);
int c6_linux_network_host_offline(void);
int c6_linux_network_check_guards(void);
int c6_linux_network_observe_session(const char *phase);
int c6_linux_network_leave(void);
int c6_linux_network_shutdown(void);
#endif
