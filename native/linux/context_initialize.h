#ifndef C6_LINUX_CONTEXT_INITIALIZE_H
#define C6_LINUX_CONTEXT_INITIALIZE_H
/* Host preparation creates no GameCore context and may precede a framework
 * that owns its contexts. Independent-factory initialization stays separate. */
int c6_linux_prepare_host(void);
int c6_linux_context_initialize(void *game, void *context);
int c6_linux_context_finish(void);
#endif
