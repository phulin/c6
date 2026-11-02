/* Bounded host ABI diagnostic for the fingerprinted Windows XP2 target.
 * Contracts and limits: docs/bootstrap-services.md, evidence/bootstrap/services.json.
 * All unimplemented operations terminate; this is not a gameplay engine host.
 */
#include "service_fixture.h"
#include "../xml-import/imports.h"
#include <stdint.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

struct registration { uint32_t event, parameter, usage; };
struct fixture {
    uintptr_t *vtable;
    struct registration registrations[64];
    size_t registration_count;
};
static uintptr_t slots[46];
static struct fixture engine = {slots, {{0, 0, 0}}, 0};
static HMODULE original_host_image;
static uint64_t original_archive_storage[6];
static int original_archive_constructed;
static uintptr_t archive_slots[3];
static uintptr_t *archive_object = archive_slots;
static HMODULE original_gamecore;
static size_t removed_absent_archive_types;
static uintptr_t logging_slots[12];
static uintptr_t logfile_slots[4];
static uintptr_t *logging_object = logging_slots;
static uintptr_t *logfile_object = logfile_slots;
static size_t registered_logs;
static size_t log_messages;
static void *database_connections[2];
static const char configuration_database_uri[] = "file:configuration-0?mode=memory&cache=shared";
static const char gameplay_database_uri[] = "file:gameplay-0?mode=memory&cache=shared";
static const unsigned char engine_guid[16] = {
    0xc2,0x99,0xce,0x27,0xa8,0x78,0xd5,0x41,
    0x91,0xa2,0x56,0x7c,0x08,0x3e,0x47,0xaf
};
static void trace(const char *event, unsigned offset) {
    fprintf(stderr, "{\"bootstrap_service\":\"%s\",\"slot_offset\":%u}\n", event, offset);
    fflush(stderr);
}
static _Noreturn void unavailable(unsigned offset) {
    trace("unsupported_required_service", offset);
    /* A dedicated process status preserves the failed boundary, rather than letting
     * an arbitrary return value advance original code as fabricated success. */
    ExitProcess(86);
}
#define UNSUPPORTED(N) static uintptr_t unsupported_##N(void *self) { (void)self; fprintf(stderr, "{\"unsupported_caller_rva\":\"0x%llx\"}\n", (unsigned long long)((uintptr_t)__builtin_return_address(0) - (uintptr_t)original_gamecore)); unavailable((N)*8); }
UNSUPPORTED(0)
UNSUPPORTED(1)
UNSUPPORTED(2)
UNSUPPORTED(3)
UNSUPPORTED(4)
UNSUPPORTED(5)
UNSUPPORTED(6)
UNSUPPORTED(7)
UNSUPPORTED(8)
UNSUPPORTED(9)
UNSUPPORTED(10)
UNSUPPORTED(11)
UNSUPPORTED(12)
UNSUPPORTED(13)
UNSUPPORTED(14)
UNSUPPORTED(15)
UNSUPPORTED(16)
UNSUPPORTED(17)
UNSUPPORTED(18)
UNSUPPORTED(19)
UNSUPPORTED(20)
UNSUPPORTED(21)
UNSUPPORTED(22)
UNSUPPORTED(23)
UNSUPPORTED(24)
UNSUPPORTED(25)
UNSUPPORTED(26)
UNSUPPORTED(27)
UNSUPPORTED(28)
UNSUPPORTED(29)
UNSUPPORTED(30)
UNSUPPORTED(31)
UNSUPPORTED(32)
UNSUPPORTED(33)
UNSUPPORTED(34)
UNSUPPORTED(35)
UNSUPPORTED(36)
UNSUPPORTED(37)
UNSUPPORTED(38)
UNSUPPORTED(39)
UNSUPPORTED(40)
UNSUPPORTED(41)
UNSUPPORTED(42)
UNSUPPORTED(43)
UNSUPPORTED(44)
UNSUPPORTED(45)
static uintptr_t unsupported_slots[46] = {
    (uintptr_t)unsupported_0,
    (uintptr_t)unsupported_1,
    (uintptr_t)unsupported_2,
    (uintptr_t)unsupported_3,
    (uintptr_t)unsupported_4,
    (uintptr_t)unsupported_5,
    (uintptr_t)unsupported_6,
    (uintptr_t)unsupported_7,
    (uintptr_t)unsupported_8,
    (uintptr_t)unsupported_9,
    (uintptr_t)unsupported_10,
    (uintptr_t)unsupported_11,
    (uintptr_t)unsupported_12,
    (uintptr_t)unsupported_13,
    (uintptr_t)unsupported_14,
    (uintptr_t)unsupported_15,
    (uintptr_t)unsupported_16,
    (uintptr_t)unsupported_17,
    (uintptr_t)unsupported_18,
    (uintptr_t)unsupported_19,
    (uintptr_t)unsupported_20,
    (uintptr_t)unsupported_21,
    (uintptr_t)unsupported_22,
    (uintptr_t)unsupported_23,
    (uintptr_t)unsupported_24,
    (uintptr_t)unsupported_25,
    (uintptr_t)unsupported_26,
    (uintptr_t)unsupported_27,
    (uintptr_t)unsupported_28,
    (uintptr_t)unsupported_29,
    (uintptr_t)unsupported_30,
    (uintptr_t)unsupported_31,
    (uintptr_t)unsupported_32,
    (uintptr_t)unsupported_33,
    (uintptr_t)unsupported_34,
    (uintptr_t)unsupported_35,
    (uintptr_t)unsupported_36,
    (uintptr_t)unsupported_37,
    (uintptr_t)unsupported_38,
    (uintptr_t)unsupported_39,
    (uintptr_t)unsupported_40,
    (uintptr_t)unsupported_41,
    (uintptr_t)unsupported_42,
    (uintptr_t)unsupported_43,
    (uintptr_t)unsupported_44,
    (uintptr_t)unsupported_45,
};
static void *query(void *self, const void *guid) {
    trace("QueryInterface", 0);
    return guid && memcmp(guid, engine_guid, sizeof engine_guid) == 0 ? self : NULL;
}
static void destroy(void *self) {
    (void)self;
    /* Static host-owned storage stays valid through DLL teardown. */
    trace("Destroy_borrowed_fixture", 8);
}
static void *script(void *self) {
    (void)self;
    trace("script_system_unavailable_null", 0x10);
    return NULL;
}
static void *logging(void *self) {
    (void)self;
    trace("GetLoggingManager_stderr_sink", 0x18);
    return &logging_object;
}
static void *archive_factory(void *self) {
    (void)self;
    if (original_archive_constructed) {
        trace("GetAutoArchiveFactory_original_registry", 0x110);
        return original_archive_storage;
    }
    trace("GetAutoArchiveFactory_empty_registry", 0x110);
    return &archive_object;
}
static void archive_unregister(void *self, uintptr_t *owned_name) {
    (void)self;
    /* Original factory +8 removes a key if found, then destroys its by-value
     * AutoVariableString argument. Register (+0) is unsupported here, so every
     * removal is of an absent key in this permanently empty registry.
     * The original three-pointer string constructor is GameCore RVA14930;
     * allocation is 994970 -> 990900. Use its paired original aligned free.
     * No host-CRT allocator or guessed C++ destructor is substituted. */
    if (!owned_name || owned_name[2] < owned_name[0]) unavailable(0x110);
    if (owned_name[2] - owned_name[0] > 1 && owned_name[0])
        ((void (*)(void *))((uintptr_t)original_gamecore + 0x9908f0))((void *)owned_name[0]);
    ++removed_absent_archive_types;
    trace("UnregisterArchiveType_absent_in_empty_registry", 8);
}
static void log_register(void *self, uint32_t id, const wchar_t *filename, uint32_t flags, void *reserved) {
    (void)self; (void)filename; (void)flags; (void)reserved;
    ++registered_logs;
    fprintf(stderr, "{\"bootstrap_service\":\"RegisterLog_stderr_sink\",\"id\":%u}\n", id);
}
static void log_enable(void *self, uint32_t id, unsigned char enabled) {
    (void)self;
    fprintf(stderr, "{\"bootstrap_service\":\"EnableLog_stderr_sink\",\"id\":%u,\"enabled\":%u}\n", id, enabled);
}
static void *get_logfile(void *self, uint32_t id) {
    (void)self; (void)id;
    return &logfile_object;
}
static void logfile_write(void *self, const char *format, ...) {
    (void)self;
    ++log_messages;
    va_list arguments;
    va_start(arguments, format);
    fputs("[original GameCore log] ", stderr);
    vfprintf(stderr, format, arguments);
    fputc('\n', stderr);
    va_end(arguments);
    fflush(stderr);
}
static void register_parameter(void *self, uint32_t event, uint32_t parameter, uint32_t usage) {
    struct fixture *fixture = self;
    if (fixture->registration_count == 64) unavailable(0x98);
    fixture->registrations[fixture->registration_count++] = (struct registration){event, parameter, usage};
    fprintf(stderr, "{\"bootstrap_service\":\"RegisterEventParameter\",\"event\":%u,\"parameter\":%u,\"usage\":%u}\n", event, parameter, usage);
    fflush(stderr);
}
static const char *configuration_uri(void *self) {
    (void)self;
    if (!database_connections[0]) {
        trace("missing_configuration_database_fixture", 0x140);
        unavailable(0x140);
    }
    trace("GetConfigurationDatabaseUri_empty_fixture", 0x140);
    return configuration_database_uri;
}
static const char *gameplay_uri(void *self) {
    (void)self;
    if (!database_connections[1]) {
        trace("missing_gameplay_database_fixture", 0x138);
        unavailable(0x138);
    }
    trace("GetGameplayDatabaseUri_empty_fixture", 0x138);
    return gameplay_database_uri;
}
void *bootstrap_service_fixture_create(HMODULE gamecore) {
    /* The external launcher gates the whole module SHA-256. Also check the exact
     * target GUID bytes before this module-specific interface can be installed. */
    if (!gamecore || memcmp((unsigned char *)gamecore + 0x9bfba8, engine_guid, 16)) {
        trace("target_guid_mismatch", 0);
        return NULL;
    }
    original_gamecore = gamecore;
    memcpy(archive_slots, unsupported_slots, sizeof archive_slots);
    archive_slots[1] = (uintptr_t)archive_unregister;
    memcpy(slots, unsupported_slots, sizeof slots);
    memcpy(logging_slots, unsupported_slots, sizeof logging_slots);
    memcpy(logfile_slots, unsupported_slots, sizeof logfile_slots);
    logging_slots[5] = (uintptr_t)log_enable;
    logging_slots[8] = (uintptr_t)log_register;
    logging_slots[10] = (uintptr_t)get_logfile;
    logfile_slots[2] = (uintptr_t)logfile_write;
    slots[0] = (uintptr_t)query;
    slots[1] = (uintptr_t)destroy;
    slots[2] = (uintptr_t)script;
    slots[3] = (uintptr_t)logging;
    slots[19] = (uintptr_t)register_parameter;
    slots[34] = (uintptr_t)archive_factory;
    slots[39] = (uintptr_t)gameplay_uri;
    slots[40] = (uintptr_t)configuration_uri;
    engine.registration_count = 0;
    return &engine;
}
static void *game_sync_manager(void *self) {
    typedef void *(*getter_fn)(void *);
    void *manager = ((getter_fn)((uintptr_t)original_host_image + 0x1c9710))(self);
    trace(manager ? "GetGameSyncManager_original_nonnull" : "GetGameSyncManager_original_null", 0x108);
    return manager;
}
int bootstrap_service_fixture_prepare_original_host_services(HMODULE original_host) {
    typedef void *(*constructor_fn)(void *);
    if ((uintptr_t)original_host != 0x140000000ULL || original_archive_constructed) return 0;
    if (!bootstrap_bind_original_imports(original_host)) return 0;
    original_host_image = original_host;
    /* Original FAutoArchiveFactory ctor is closed, writes the original vptr and
     * empty-bucket state. Subsequent original methods retain their real typed
     * registry and argument ownership. No EXE global startup is invoked. */
    memset(original_archive_storage, 0, sizeof original_archive_storage);
    if (((constructor_fn)((uintptr_t)original_host + 0xdba4f0))(original_archive_storage) != original_archive_storage)
        return 0;
    original_archive_constructed = 1;
    slots[33] = (uintptr_t)game_sync_manager;
    trace("original_archive_factory_constructed", 0x110);
    return 1;
}
int bootstrap_service_fixture_prepare_empty_databases(HMODULE database) {
    typedef unsigned char (*initialize_fn)(void);
    typedef void *(*create_fn)(const char *, int, void *);
    union { FARPROC address; initialize_fn function; } initialize;
    union { FARPROC address; create_fn function; } create;
    if (!database || database_connections[0] || database_connections[1]) return 0;
    initialize.address = GetProcAddress(database, "?Initialize@Database@@YA_NXZ");
    create.address = GetProcAddress(database, "?CreateConnection@Database@@YAPEAVIDatabaseConnection1@1@PEBDHPEAVIDatabaseLogger@1@@Z");
    if (!initialize.address || !create.address || !initialize.function()) return 0;
    database_connections[0] = create.function(configuration_database_uri, 0x46, NULL);
    database_connections[1] = create.function(gameplay_database_uri, 0x46, NULL);
    if (!database_connections[0] || !database_connections[1]) return 0;
    trace("original_database_empty_keepalives_created", 0);
    return 1;
}
void *bootstrap_service_fixture_database_connection(int gameplay) {
    return database_connections[gameplay ? 1 : 0];
}
int bootstrap_service_fixture_execute_sql(int gameplay, const char *sql, int length) {
    typedef unsigned char (*execute_fn)(void *, const char *, int);
    void *connection = database_connections[gameplay ? 1 : 0];
    if (!connection || !sql) return 0;
    return ((execute_fn)(*(uintptr_t **)connection)[5])(connection, sql, length) != 0;
}
void bootstrap_service_fixture_report(FILE *output) {
    fprintf(output, "{\"service_fixture\":\"bounded_diagnostic\",\"registered_parameters\":%zu,\"script_vm\":false,\"logging_manager\":\"stderr_sink\",\"empty_database_keepalives\":%s,\"gameplay_initialized\":false}\n", engine.registration_count, database_connections[0] && database_connections[1] ? "true" : "false");
}
