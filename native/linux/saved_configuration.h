#ifndef C6_LINUX_SAVED_CONFIGURATION_H
#define C6_LINUX_SAVED_CONFIGURATION_H
/* Caller must gate the complete Linux fixture and original host hashes. */
int c6_linux_restore_saved_configuration(const char *path);
#endif
