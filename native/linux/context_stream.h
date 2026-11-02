#ifndef C6_LINUX_CONTEXT_STREAM_H
#define C6_LINUX_CONTEXT_STREAM_H
#include <stddef.h>
int c6_linux_stream_check(void);
int c6_linux_context_load(void *game, void *context, const char *path);
/* Caller must gate exact file SHA-256, size and configuration before execution.
 * Same original stream/Load boundary; no fabricated stream fields. */
int c6_linux_context_load_sized(void *game, void *context, const char *path, size_t expected_size);
int c6_linux_context_save(void *game, void *context, const char *path);
#endif
