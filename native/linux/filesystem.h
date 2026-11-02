#ifndef C6_LINUX_FILESYSTEM_H
#define C6_LINUX_FILESYSTEM_H
/* Original exact-Linux ASL APIs; caller gates host/dependencies/launch inputs. */
int c6_linux_filesystem_initialize(void);
int c6_linux_filesystem_probe_paths(const char *phase);
int c6_linux_filesystem_shutdown(void);
#endif
