#ifndef C6_LINUX_HOST_SERVICES_H
#define C6_LINUX_HOST_SERVICES_H
/* Diagnostic fixture for the exact hash gated Linux host; no stand-in objects. */
void *c6_linux_create_host_engine_utility(void);
int c6_linux_destroy_host_services(void);
#endif
