#ifndef C6_LINUX_CURRENT_HANDLER_INTERRUPTION_H
#define C6_LINUX_CURRENT_HANDLER_INTERRUPTION_H
/* Preparation-only standalone passive helper. No runner integration is implied.
 * Caller retains the genuine loaded module, initialized Context/Game and all
 * registry/Player/Unit/container owners under the native main-thread game lock
 * throughout this synchronous call. The callback must authenticate exactly that
 * context/Game association, idle work/queues and retained owner lifetime. Failure
 * is fatal before any further observation or native cleanup. This callback is a
 * caller admission service, never a game predicate or replacement game service.
 * Its implementation must use separately source-closed passive controller/worker
 * field reads, with NO original getter/predicate/script calls. No implementation
 * or integrated idle/zero-original-call closure is admitted by this header.
 * Ordinary enclosing original Load/Save/cleanup is a separate caller scope.
 * Full rules/DLC/mod/input/configuration/source identities belong in the pinned
 * profile identified by profile_sha256. No observed answers may be profile inputs.
 * Two full passes, six full unmasked Saves, known Game/Map RNG endpoint controls
 * and original terminal cleanup must be admitted separately by the sole runner.
 * Output is exclusive JSONL; partial/nonzero/fatal output is never accepted.
 * Neither helper nor its admission callback may call original getters, handler
 * methods, predicates, scripts, actions or turns. Callback source/profile closure
 * remains pending before this contract can be integrated.
 * Native root/containers are read using exact pinned layout and safe self reads;
 * unsupported storage, mapping, bindings and code reject without fabricated nulls.
 * Loader symlinks are canonicalized only to obtain a no-follow regular-file
 * fd. Original loader/canonical path bytes are recorded; fd hashes, stable
 * metadata and actual mapped-file/code identity remain mandatory. Path endpoint
 * checks do not prove absence of every transient external filesystem change.
 * Whole live-state/allocator/RNG reconstruction and native predicate parity are
 * not claimed. Existing original-image prediction profiles remain immutable. */
typedef struct {
    void *module;
    void *borrowed_context;
    const void *retained_game;
    const char *profile_sha256;
    const char *output_path;
    int (*quiescent)(void *module,void *context,const void *retained_game);
} C6CurrentHandlerInterruptionAdmission;
int c6_linux_current_handler_interruption_capture(const C6CurrentHandlerInterruptionAdmission *admission);
#endif
