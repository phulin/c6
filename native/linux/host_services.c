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
static void **application_global;
static void (*delete_object)(void *);
static void (*app_destruct)(void *);
static void (*utility_destruct)(void *);
static bool database_started;
static bool archive_factory_created;

static void *symbol(const char *name) {
    void *address=dlsym(RTLD_DEFAULT,name);
    if(!address) fprintf(stderr,"C6_LINUX_HOST missing_symbol %s\n",name);
    return address;
}

void *c6_linux_create_host_engine_utility(void) {
    if(application || utility) return NULL;
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
    char source[1024];
    wchar_t config_source[1024],game_source[1024];
    int count=snprintf(source,sizeof source,"%s/Base/assets/configuration/data/",assets);
    if(count<0||(size_t)count>=sizeof source||
       mbstowcs(config_source,source,1024)==(size_t)-1) return NULL;
    count=snprintf(source,sizeof source,"%s/Base/assets/gameplay/data/",assets);
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
    void *(*archive_get)(void)=symbol("_ZN19FAutoArchiveFactory11GetInstanceEv");
    void *(*archive_create)(void)=symbol("_ZN19FAutoArchiveFactory14CreateInstanceEv");
    if(!archive_get||!archive_create||archive_get()) return NULL;
    fprintf(stderr,"C6_LINUX_HOST archive_factory_create_enter\n");
    if(!archive_create()) return NULL;
    archive_factory_created=true;
    fprintf(stderr,"C6_LINUX_HOST archive_factory_create_return nonnull\n");
    return utility;
}

int c6_linux_destroy_host_services(void) {
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
    if(application) {
        if(*application_global!=application) return 1;
        app_destruct(application);
        delete_object(application);
        application=NULL;
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
