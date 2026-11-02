#ifndef C6_LINUX_RULES_SNAPSHOT_H
#define C6_LINUX_RULES_SNAPSHOT_H
/* Export real App-owned native DB connections after original initialization.
 * C6_RULES_SNAPSHOT_DIR must name a private existing absolute directory.
 * Every destination must be new. Zero means both native backups/counts passed. */
int c6_linux_rules_snapshot(void *gamecore, void *context);
#endif
