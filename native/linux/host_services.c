#define _GNU_SOURCE
#include "host_services.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <wchar.h>

/* Linux host SHA256 106d4986...: WinMain 0x2fb9250 allocates 0x6800,
 * calls Civ6App C1, then publishes g_pkApplication. The fixture repeats that
 * binding, never invoking RunApp (which opens a window). All object fields
 * otherwise belong to the genuine constructor and original methods. */
static void *application;
static void *utility;
static void *localization_database;
static void **application_global;
static void (*delete_object)(void *);
static void (*app_destruct)(void *);
static void (*utility_destruct)(void *);
static bool database_started;
static bool archive_factory_created;
static bool lua_system_created;
static bool network_started;
static bool host_events_registered;
static bool host_arguments_bound;

static void *symbol(const char *name) {
    void *address=dlsym(RTLD_DEFAULT,name);
    if(!address) fprintf(stderr,"C6_LINUX_HOST missing_symbol %s\n",name);
    return address;
}

void *c6_linux_get_host_localization_database(void) {
    if(!application||!application_global||*application_global!=application||
       !localization_database)return NULL;
    void *(*get_database)(void *)=symbol("_ZN7Civ6App23GetLocalizationDatabaseEv");
    if(!get_database||get_database(application)!=localization_database)return NULL;
    return localization_database;
}

int c6_linux_bind_host_arguments(void *candidate) {
    if(!application||candidate!=application||!application_global||
       *application_global!=application)return 1;
    int (*arguments)(int *,char ***)=symbol("c6_linux_main_arguments");
    int argc=0;
    char **argv=NULL;
    if(!arguments||arguments(&argc,&argv)||argc!=1||!argv||!argv[0]||argv[1])return 1;
    /* Exact original RunApp stores at 0x42b9aa3/0x42b9aa7. This is actual CRT
     * input, not a fabricated command line or a reconstructed App object. */
    if(host_arguments_bound)return *(int *)((char *)application+0x30)==argc&&
        *(char ***)((char *)application+0x38)==argv?0:1;
    *(int *)((char *)application+0x30)=argc;
    *(char ***)((char *)application+0x38)=argv;
    host_arguments_bound=true;
    fprintf(stderr,"C6_LINUX_HOST arguments_bound argc %d genuine_crt 1\n",argc);
    return 0;
}

int c6_linux_register_host_events(void) {
    if(!application||!application_global||*application_global!=application)return 1;
    if(host_events_registered)return 0;
    bool (*register_events)(void *)=symbol("_ZN7Civ6App14RegisterEventsEv");
    if(!register_events)return 1;
    fprintf(stderr,"C6_LINUX_HOST register_events_enter\n");
    bool result=register_events(application);
    fprintf(stderr,"C6_LINUX_HOST register_events_return %d\n",result);
    if(result)host_events_registered=true;
    return result?0:1;
}

/* Actual AppLuaScriptSystem::Create performs allocation, Havok VM setup and
 * singleton publication. No synthetic Lua state or reconstructed VM fields. */
static int create_lua_system_and_probe(void) {
    void *(*get_system)(void)=symbol("_ZN9LuaSystem15LuaScriptSystem3GetEv");
    void (*create_system)(void)=symbol("_ZN18AppLuaScriptSystem6CreateEv");
    void *(*get_main_state)(void *)=symbol("_ZN9LuaSystem15LuaScriptSystem8GetStateEv");
    void *(*create_thread)(void *,int)=symbol(
        "_ZN18AppLuaScriptSystem12CreateThreadEN9LuaSystem9LuaThread10ThreadTypeE");
    void *(*thread_state)(const void *)=symbol("_ZNK9LuaSystem9LuaThreadcvP9lua_StateEv");
    void (*release_thread)(void *)=symbol("_ZN12AppLuaThreadD0Ev");
    int (*gettop)(void *)=symbol("_Z10lua_gettopP9lua_State");
    void (*settop)(void *,int)=symbol("_Z10lua_settopP9lua_Statei");
    void (*pushnumber)(void *,double)=symbol("_Z14lua_pushnumberP9lua_Stated");
    double (*tonumber)(void *,int)=symbol("_Z12lua_tonumberP9lua_Statei");
    void (*lock)(void)=symbol("_ZN3Lua7Details10LockAccessEv");
    void (*unlock)(void)=symbol("_ZN3Lua7Details12UnlockAccessEv");
    if(!get_system||!create_system||!get_main_state||!create_thread||
       !thread_state||!release_thread||!gettop||!settop||!pushnumber||
       !tonumber||!lock||!unlock||get_system()) return 1;
    fprintf(stderr,"C6_LINUX_HOST lua_system_create_enter\n");
    create_system();
    void *system=get_system();
    if(!system) return 1;
    lua_system_created=true;
    if(!get_main_state(system)) return 1;
    fprintf(stderr,"C6_LINUX_HOST lua_system_create_return main_state_nonnull\n");
    fprintf(stderr,"C6_LINUX_HOST lua_thread_create_enter\n");
    /* ThreadType 1 matches actual GameCore::ScriptSystem::CreateLuaThread. */
    void *thread=create_thread(system,1);
    if(!thread) return 1;
    void *state=thread_state(thread);
    if(!state) {release_thread(thread);return 1;}
    fprintf(stderr,"C6_LINUX_HOST lua_thread_create_return state_nonnull\n");
    lock();
    int before=gettop(state);
    pushnumber(state,16777215.0);
    bool roundtrip=gettop(state)==before+1&&tonumber(state,-1)==16777215.0;
    settop(state,before);
    roundtrip=roundtrip&&gettop(state)==before;
    unlock();
    fprintf(stderr,"C6_LINUX_HOST lua_stack_roundtrip_return %d\n",roundtrip);
    release_thread(thread);
    fprintf(stderr,"C6_LINUX_HOST lua_thread_release_return\n");
    return roundtrip?0:1;
}

void *c6_linux_create_host_engine_utility(void) {
    if(application || utility) return NULL;
    int (*initialize_temp_heap)(void)=symbol("c6_linux_temp_heap_initialize");
    if(!initialize_temp_heap||initialize_temp_heap())return NULL;
    void *(*allocate)(size_t)=symbol("_Znwm");
    void (*app_construct)(void *)=symbol("_ZN7Civ6AppC1Ev");
    void (*utility_construct)(void *)=symbol("_ZN8GameCore13EngineUtilityC1Ev");
    wchar_t *(*copy_path)(wchar_t *,const wchar_t *)=symbol("_ZN8Platform6wcscpyEPwPKw");
    application_global=symbol("g_pkApplication");
    delete_object=symbol("_ZdlPv");
    app_destruct=symbol("_ZN7Civ6AppD1Ev");
    utility_destruct=symbol("_ZN8GameCore13EngineUtilityD1Ev");
    if(!allocate||!app_construct||!utility_construct||!application_global||
       !copy_path||!delete_object||!app_destruct||!utility_destruct) return NULL;
    if(*application_global) {
        fprintf(stderr,"C6_LINUX_HOST application_already_published\n");
        return NULL;
    }
    const char *userdata=getenv("C6_HOST_USERDATA");
    wchar_t docs[260];
    if(!userdata||userdata[0]!='/') return NULL;
    size_t length=mbstowcs(docs,userdata,260);
    if(length==(size_t)-1||length>=259) return NULL;
    fprintf(stderr,"C6_LINUX_HOST app_construct_enter\n");
    application=allocate(0x6800);
    if(!application) return NULL;
    app_construct(application);
    /* RunApp 0x42b9d79/0x42b9d98 initializes this exact 260-wchar
     * AppEnvironment documents buffer via Platform::wcscpy. Explicit fixture
     * input substitutes only the directory, not App behavior. */
    copy_path((wchar_t *)((char *)application+0x464),docs);
    *application_global=application;
    if(c6_linux_bind_host_arguments(application))return NULL;
    fprintf(stderr,"C6_LINUX_HOST app_construct_return published\n");
    fprintf(stderr,"C6_LINUX_HOST utility_construct_enter\n");
    /* Allocation size from AppModdingFramework::GetEngineUtility native
     * allocation site; the genuine ctor writes vptr/refcount/achievement ptr. */
    utility=allocate(0x18);
    if(!utility) return NULL;
    utility_construct(utility);
    fprintf(stderr,"C6_LINUX_HOST utility_construct_return nonnull\n");
    const char *assets=getenv("C6_HOST_ASSETS");
    if(!assets) {
        fprintf(stderr,"C6_LINUX_HOST database_assets_missing\n");
        return NULL;
    }
    bool (*database_initialize)(void)=symbol("_ZN8Database10InitializeEv");
    void (*config_construct)(void *)=symbol("_ZN11Application4Data21ConfigurationDatabaseC1Ev");
    void (*game_construct)(void *)=symbol("_ZN11Application4Data16GameplayDatabaseC1Ev");
    bool (*config_initialize)(void *,const wchar_t *,const wchar_t *)=
        symbol("_ZN11Application4Data21ConfigurationDatabase10InitializeEPKwS3_");
    bool (*game_initialize)(void *,const wchar_t *,const wchar_t *)=
        symbol("_ZN11Application4Data16GameplayDatabase10InitializeEPKwS3_");
    if(!database_initialize||!config_construct||!game_construct||
       !config_initialize||!game_initialize) return NULL;
    fprintf(stderr,"C6_LINUX_HOST database_initialize_enter\n");
    if(!database_initialize()) return NULL;
    database_started=true;
    /* These 0x38 allocations and owning-pointer stores are taken from original
     * Civ6App::SetupDatabase, 0x2f91e6d..0x2f91ec9. Cleanup is original App dtor.
     * Parameterized original methods select fixture inputs without rewriting
     * the binary's legacy mixed-case relative default paths. */
    void *config=allocate(0x38), *game_database=allocate(0x38);
    if(!config||!game_database) return NULL;
    config_construct(config);
    game_construct(game_database);
    *(void **)((char *)application+0x6320)=config;
    *(void **)((char *)application+0x6328)=game_database;
    /* Optional fixture view changes only path spelling; runtime_probe gates
     * every source byte and mapping against original assets before execution. */
    const char *source_root=getenv("C6_RULES_SOURCE_ROOT");
    if(!source_root) source_root=assets;
    char source[1024];
    wchar_t config_source[1024],game_source[1024];
    int count=snprintf(source,sizeof source,"%s/Base/assets/configuration/data/",source_root);
    if(count<0||(size_t)count>=sizeof source||
       mbstowcs(config_source,source,1024)==(size_t)-1) return NULL;
    count=snprintf(source,sizeof source,"%s/Base/assets/gameplay/data/",source_root);
    if(count<0||(size_t)count>=sizeof source||
       mbstowcs(game_source,source,1024)==(size_t)-1) return NULL;
    fprintf(stderr,"C6_LINUX_HOST configuration_database_enter\n");
    bool config_ok=config_initialize(config,L"c6-no-configuration-cache.sqlite",config_source);
    fprintf(stderr,"C6_LINUX_HOST configuration_database_return %d\n",config_ok);
    if(!config_ok) return NULL;
    fprintf(stderr,"C6_LINUX_HOST gameplay_database_enter\n");
    bool game_ok=game_initialize(game_database,L"c6-no-gameplay-cache.sqlite",game_source);
    fprintf(stderr,"C6_LINUX_HOST gameplay_database_return %d\n",game_ok);
    if(!game_ok) return NULL;
    void (*localization_construct)(void *)=
        symbol("_ZN11Application4Data20LocalizationDatabaseC1Ev");
    void *(*get_localization)(void *)=symbol("_ZN7Civ6App23GetLocalizationDatabaseEv");
    if(!localization_construct||!get_localization||get_localization(application))return NULL;
    /* Original Civ6App::SetupDatabase 0x2f91ec9..0x2f91ef7 allocates 0x18,
     * invokes C1 and publishes this owning pointer. Its genuine Initialize is
     * performed by the separately gated localization bridge; App D1 owns free. */
    localization_database=allocate(0x18);
    if(!localization_database)return NULL;
    localization_construct(localization_database);
    *(void **)((char *)application+0x6330)=localization_database;
    bool published=c6_linux_get_host_localization_database()==localization_database;
    fprintf(stderr,"C6_LINUX_HOST localization_database_construct_return app_owned %d\n",published);
    if(!published)return NULL;
    if(getenv("C6_HOST_NETWORK")) {
        int (*initialize_network)(void *)=symbol("c6_linux_network_initialize");
        if(!initialize_network||initialize_network(application))return NULL;
        network_started=true;
    } else {
        void *(*archive_get)(void)=symbol("_ZN19FAutoArchiveFactory11GetInstanceEv");
        void *(*archive_create)(void)=symbol("_ZN19FAutoArchiveFactory14CreateInstanceEv");
        if(!archive_get||!archive_create||archive_get()) return NULL;
        fprintf(stderr,"C6_LINUX_HOST archive_factory_create_enter\n");
        if(!archive_create()) return NULL;
        archive_factory_created=true;
        fprintf(stderr,"C6_LINUX_HOST archive_factory_create_return nonnull\n");
    }
    if(getenv("C6_HOST_LUA")&&create_lua_system_and_probe()) return NULL;
    return utility;
}

int c6_linux_destroy_host_services(void) {
    if(network_started) {
        int (*shutdown_network)(void)=symbol("c6_linux_network_shutdown");
        if(!shutdown_network||shutdown_network())return 1;
        network_started=false;
    }
    if(archive_factory_created) {
        void (*archive_free)(void)=symbol("_ZN19FAutoArchiveFactory12FreeInstanceEv");
        if(!archive_free) return 1;
        archive_free();
        archive_factory_created=false;
        fprintf(stderr,"C6_LINUX_HOST archive_factory_destroy_return\n");
    }
    if(utility) {
        utility_destruct(utility);
        delete_object(utility);
        utility=NULL;
        fprintf(stderr,"C6_LINUX_HOST utility_destroy_return\n");
    }
    if(lua_system_created) {
        void (*destroy_system)(void)=symbol("_ZN9LuaSystem15LuaScriptSystem7DestroyEv");
        void *(*get_system)(void)=symbol("_ZN9LuaSystem15LuaScriptSystem3GetEv");
        if(!destroy_system||!get_system) return 1;
        destroy_system();
        if(get_system()) return 1;
        lua_system_created=false;
        fprintf(stderr,"C6_LINUX_HOST lua_system_destroy_return unpublished\n");
    }
    if(application) {
        if(*application_global!=application) return 1;
        app_destruct(application);
        delete_object(application);
        application=NULL;
        localization_database=NULL;
        host_events_registered=false;
        host_arguments_bound=false;
        if(database_started) {
            void (*shutdown)(void)=symbol("_ZN8Database8ShutdownEv");
            if(!shutdown) return 1;
            shutdown();
            database_started=false;
        }
        *application_global=NULL;
        fprintf(stderr,"C6_LINUX_HOST app_destroy_return unpublished\n");
    }
    return 0;
}
