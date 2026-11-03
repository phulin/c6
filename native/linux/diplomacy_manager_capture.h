#ifndef C6_LINUX_DIPLOMACY_MANAGER_CAPTURE_H
#define C6_LINUX_DIPLOMACY_MANAGER_CAPTURE_H
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
/* Returns zero only while the exact module/configuration, retained actual Game,
 * main thread and native owner token remain externally admitted. No acquisition,
 * getter, policy, lock or release is performed by this helper. */
typedef int (*C6DiplomacyCaptureOwnerGuard)(void *,void *,const void *);
/* Returns zero after copying exactly n bytes; must not dereference unsafe input.
 * NULL selects process_vm_readv on this process. Synthetic readers are for tests,
 * never evidence of an actual loaded-game observation. */
typedef int (*C6DiplomacyCaptureRead)(void *,uintptr_t,void *,size_t);
typedef struct {
    void *module;
    void *context;
    const void *game;
    uintptr_t core_base;
    long main_tid;
    const void *player_manager;
    const char *profile_sha256;
    const char *core_sha256;
    /* Bounded caller label, [A-Za-z0-9_.-]{1,64}; not runtime evidence. */
    const char *stage;
    C6DiplomacyCaptureOwnerGuard retained_owner;
    C6DiplomacyCaptureRead read;
    void *read_context;
    /* Aggregate actual read bytes/items; positive, at most 8MiB/65536. */
    size_t max_bytes;
    size_t max_items;
    FILE *output;
} C6DiplomacyCaptureAdmission;
/* JSONL schema c6-linux-diplomacy-capture-v1. Zero means a bounded capture of
 * documented raw fields completed, never semantic completeness/gameplay parity.
 * Nonzero terminates the stream with complete:false when output is available.
 * No original module functions execute and no original memory is written.
 * Caller owns stream lifetime and externally proves target/input/config identity,
 * actual Game+0x248 PlayerManager linkage and retained native owner/thread scope.
 * Allocation/ID tables are distinct vectors; native rebuild scans only allocation
 * slots 0..63, while capture preserves the actual full vector (including slot64).
 * A caller-supplied guard/reader is a trust boundary, not independent admission. */
int c6_linux_diplomacy_manager_capture(const C6DiplomacyCaptureAdmission *);
#endif
