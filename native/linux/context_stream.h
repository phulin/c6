#ifndef C6_LINUX_CONTEXT_STREAM_H
#define C6_LINUX_CONTEXT_STREAM_H
int c6_linux_stream_check(void);
int c6_linux_context_load(void *game, void *context, const char *path);
int c6_linux_context_save(void *game, void *context, const char *path);
#endif
