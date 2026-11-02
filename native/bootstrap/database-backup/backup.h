#ifndef C6_ORIGINAL_DATABASE_BACKUP_H
#define C6_ORIGINAL_DATABASE_BACKUP_H
#include <windows.h>
/* Genuine DatabaseDLL destination.CopyFrom(source), vtable +0xc0.
 * Rejects existing destination files. Returns zero only after native backup and
 * native source/destination per-table row-count comparison succeed.
 * Does not create or advance a game. */
int bootstrap_original_database_backup(HMODULE database, void *source,
    const wchar_t *new_path, const char *label);
#endif
