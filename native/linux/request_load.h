#ifndef C6_LINUX_REQUEST_LOAD_H
#define C6_LINUX_REQUEST_LOAD_H
/* Diagnostic original RequestLoad boundary, not Begin or a complete host load.
 * Caller gates the complete Linux benchmark SHA/7348717 bytes and host identity.
 * Requires genuine App current/game-view states both NULL; original Request then
 * defers CheckRequest. Retained stream belongs to original serializer. */
int c6_linux_request_load_prepare(const char *full_save_path);
/* Original Reset + ClearLastStream; call before framework/service destruction. */
int c6_linux_request_load_clear(void);
#endif
