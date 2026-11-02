#ifndef C6_LINUX_TEMP_HEAP_H
#define C6_LINUX_TEMP_HEAP_H
/* Original Linux process-global TempHeap; caller gates exact host/configuration. */
int c6_linux_temp_heap_initialize(void);
int c6_linux_temp_heap_shutdown(void);
#endif
