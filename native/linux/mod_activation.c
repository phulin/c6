#define _GNU_SOURCE
#include "mod_activation.h"
#include <dlfcn.h>
#include <link.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <wchar.h>
#include <stdint.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <string.h>

/* Own OS main-entry witness; it is not a native Platform thread ID. */
static long observed_mod_main_tid;
long c6_linux_mod_main_thread_tid(void) { return observed_mod_main_tid; }

static void *constructed_client;
static void *constructed_framework;
static void *constructed_content_paths;
static bool started_art_manager;
static void *constructed_ui;

static bool events_registered;
int c6_linux_mod_register_events(void) {
    int (*common)(void)=dlsym(RTLD_DEFAULT,"c6_linux_register_host_events");
    if (common) return common();
    if (events_registered) return 0;
    void **app=dlsym(RTLD_DEFAULT,"g_pkApplication");
    bool (*register_events)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Civ6App14RegisterEventsEv");
    if (!app || !*app || !register_events) return 118;
    fprintf(stderr,"C6_LINUX_MOD register_events_enter\n");
    bool ready=register_events(*app);
    fprintf(stderr,"C6_LINUX_MOD register_events_return %d\n",ready);
    if (!ready) return 119;
    events_registered=true;
    return 0;
}

/* This exact-host fixture constructs the real provider and its native owner,
 * without entering AppFiraxisLiveClient::Startup or publishing app events. */
static int client_construct_only(void) {
    void *(*get)(void) = dlsym(RTLD_DEFAULT,"_ZN20AppFiraxisLiveClient11GetInstanceEv");
    void *(*allocate)(size_t) = dlsym(RTLD_DEFAULT,"_Znwm");
    void (*construct)(void *,const char *) = dlsym(RTLD_DEFAULT,"_ZN20AppFiraxisLiveClientC1EPKc");
    void (*own)(void *,void *,void *) = dlsym(RTLD_DEFAULT,
        "_ZNSt3__110shared_ptrI20AppFiraxisLiveClientEC2IS1_EEPT_NS_9enable_ifIXsr14is_convertibleIS5_PS1_EE5valueENS2_5__natEE4typeE");
    bool (*initialized)(const void *) = dlsym(RTLD_DEFAULT,"_ZNK17FiraxisLiveClient13IsInitializedEv");
    if (!get || !allocate || !construct || !own || !initialized ||
        (void *)get!=(void *)0x31998c0 || get()) return 120;
    /* Both addresses are from the gated non-PIE host's actual Startup caller.
     * Native shared_ptr construction writes its own valid ownership fields. */
    void **owner=(void **)0x6498e10;
    if (owner[0] || owner[1]) return 121;
    fprintf(stderr,"C6_LINUX_MOD client_construct_only_enter\n");
    void *client=allocate(0x218);
    if (!client) return 122;
    construct(client,(const char *)0x45fb5e1);
    own(owner,client,NULL);
    constructed_client=client;
    bool unstarted=get()==client && !initialized(client);
    fprintf(stderr,"C6_LINUX_MOD client_construct_only_return unstarted %d\n",unstarted);
    return unstarted?0:123;
}

int c6_linux_mod_client_fixture_shutdown(void) {
    void (*shutdown)(void) = dlsym(RTLD_DEFAULT,"_ZN20AppFiraxisLiveClient8ShutdownEv");
    void *(*get)(void) = dlsym(RTLD_DEFAULT,"_ZN20AppFiraxisLiveClient11GetInstanceEv");
    if (!shutdown || !get || !constructed_client || get()!=constructed_client) return 124;
    fprintf(stderr,"C6_LINUX_MOD client_shutdown_only_enter\n");
    shutdown();
    bool unpublished=get()==NULL;
    if (unpublished) constructed_client=NULL;
    fprintf(stderr,"C6_LINUX_MOD client_shutdown_only_return unpublished %d\n",unpublished);
    return unpublished?0:125;
}

static void diagnostic_json_string(const unsigned char *value) {
    if (!value) {fputs("null",stderr);return;}
    fputc('"',stderr);
    for (;*value;value++) {
        if (*value=='"' || *value=='\\') {fputc('\\',stderr);fputc(*value,stderr);}
        else if (*value<32) fprintf(stderr,"\\u%04x",*value);
        else fputc(*value,stderr);
    }
    fputc('"',stderr);
}

static void selection_inventory(void *framework,void *db,const char *phase) {
    void *(*getters[3])(const void *)={
        dlsym(RTLD_DEFAULT,"_ZNK19AppModdingFramework14GetEnabledModsEv"),
        dlsym(RTLD_DEFAULT,"_ZNK19AppModdingFramework13GetActiveModsEv"),
        dlsym(RTLD_DEFAULT,"_ZNK19AppModdingFramework22GetActiveModComponentsEv")};
    const char *labels[]={"enabled_mods","active_mods","active_components"};
    int (*prepare)(void *,const char *,int,void **,const char **)=dlsym(RTLD_DEFAULT,"sqlite3_prepare_v2");
    int (*step)(void *)=dlsym(RTLD_DEFAULT,"sqlite3_step");
    const unsigned char *(*text)(void *,int)=dlsym(RTLD_DEFAULT,"sqlite3_column_text");
    int (*finalize)(void *)=dlsym(RTLD_DEFAULT,"sqlite3_finalize");
    if (!prepare || !step || !text || !finalize) return;
    /* Exact native consumers establish vector<ModHandle>/ComponentHandle and
     * their four-byte elements. Read the getters' begin/end without modifying
     * containers. Observed order is not claimed to be component execution order. */
    for (unsigned list=0;list<3;list++) {
        if (!getters[list]) continue;
        uintptr_t range[2];
        memcpy(range,getters[list](framework),sizeof range);
        if (range[1]<range[0] || (range[1]-range[0])%4 || (range[1]-range[0])/4>10000) continue;
        size_t count=(range[1]-range[0])/4;
        fprintf(stderr,"C6_LINUX_MOD selection {\"phase\":\"%s\",\"list\":\"%s\",\"count\":%zu,\"entries\":[",phase,labels[list],count);
        for (size_t i=0;i<count;i++) {
            int32_t handle;
            memcpy(&handle,(const void *)(range[0]+4*i),sizeof handle);
            char query[384];
            if (list==2)
                snprintf(query,sizeof query,"SELECT m.ModId,c.ComponentId,c.ComponentType FROM Components c JOIN Mods m ON m.ModRowId=c.ModRowId WHERE c.ComponentRowId=%d",handle);
            else snprintf(query,sizeof query,"SELECT ModId FROM Mods WHERE ModRowId=%d",handle);
            void *statement=NULL;
            bool found=!prepare(db,query,-1,&statement,NULL) && step(statement)==100;
            fprintf(stderr,"%s{\"order\":%zu,\"handle\":%d,\"resolved\":%s,\"mod_id\":",i?",":"",i,handle,found?"true":"false");
            diagnostic_json_string(found?text(statement,0):NULL);
            if (list==2) {
                fputs(",\"component_id\":",stderr);diagnostic_json_string(found?text(statement,1):NULL);
                fputs(",\"component_type\":",stderr);diagnostic_json_string(found?text(statement,2):NULL);
            }
            fputc('}',stderr);
            if (statement) finalize(statement);
        }
        fputs("]}\n",stderr);
    }
}

static void mod_inventory(void *framework,const char *phase) {
    void *(*core)(void *)=dlsym(RTLD_DEFAULT,"_ZN19AppModdingFramework19GetModdingFrameworkEv");
    void *(*database)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Modding9Framework11GetDatabaseEv");
    void *(*sqlite)(void *)=dlsym(RTLD_DEFAULT,"_ZN8Database10Connection9GetSQLiteEv");
    int (*prepare)(void *,const char *,int,void **,const char **)=dlsym(RTLD_DEFAULT,"sqlite3_prepare_v2");
    int (*step)(void *)=dlsym(RTLD_DEFAULT,"sqlite3_step");
    long long (*number)(void *,int)=dlsym(RTLD_DEFAULT,"sqlite3_column_int64");
    int (*finalize)(void *)=dlsym(RTLD_DEFAULT,"sqlite3_finalize");
    if (!core || !database || !sqlite || !prepare || !step || !number || !finalize) return;
    void *connection=database(core(framework));
    if (!connection) {
        fprintf(stderr,"C6_LINUX_MOD discovered_mods %s unavailable_connection\n",phase);
        return;
    }
    const char *queries[]={"SELECT count(*) FROM Mods", "SELECT count(*) FROM Mods WHERE lower(ModId)='4873eb62-8ccc-4574-b784-dda455e74e68'"};
    long long counts[2]={-1,-1};
    for (unsigned i=0;i<2;i++) {
        void *statement=NULL;
        if (!prepare(sqlite(connection),queries[i],-1,&statement,NULL) && step(statement)==100)
            counts[i]=number(statement,0);
        if (statement) finalize(statement);
    }
    fprintf(stderr,"C6_LINUX_MOD discovered_mods %s total %lld xp2 %lld\n",phase,counts[0],counts[1]);
    selection_inventory(framework,sqlite(connection),phase);
}

/* Hash gating is the caller's responsibility: tools/linux/runtime-gate.json.
 * Exact original host entrypoints and ownership: docs/linux-mod-activation.md.
 * No C++ fields, importer rows, std::function objects or content are synthesized. */
int c6_linux_mod_framework_initialize(void **borrowed_context) {
    if (!borrowed_context) return 110;
    *borrowed_context = NULL;
    bool (*initialize)(void) = dlsym(RTLD_DEFAULT,
        "_ZN19AppModdingFramework12InitInstanceEv");
    void *(*get)(void) = dlsym(RTLD_DEFAULT,
        "_ZN19AppModdingFramework11GetInstanceEv");
    bool (*available)(void) = dlsym(RTLD_DEFAULT,
        "_ZN8GameCore18IsContextAvailableEv");
    void **active = dlsym(RTLD_DEFAULT,"_ZN8GameCore16s_pActiveContextE");
    void **app = dlsym(RTLD_DEFAULT,"g_pkApplication");
    if (!initialize || !get || !available || !active || !app) return 111;
    /* This detects host-owned state; the caller must also refuse privately
     * factory-created contexts, which do not publish this host global. */
    if (!*app || get() || available()) {
        fprintf(stderr,"C6_LINUX_MOD framework_initial_state_refused\n");
        return 112;
    }
    if (c6_linux_mod_register_events()) return 118;
    void *(*options)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Civ6App23GetApplicationOptionSetEv");
    void *(*environment)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Civ6App17GetAppEnvironmentEv");
    void (*register_options)(void *,void *,const void *)=dlsym(RTLD_DEFAULT,
        "_ZN7Civ6App26RegisterApplicationOptionsERN7AppHost9OptionSetEPKNS0_14AppEnvironmentE");
    if (!options || !environment || !register_options) return 128;
    fprintf(stderr,"C6_LINUX_MOD register_application_options_enter\n");
    register_options(*app,options(*app),environment(*app));
    const char *(*option_string)(const void *,const char *,const char *)=dlsym(RTLD_DEFAULT,
        "_ZNK7AppHost9OptionSet15GetOptionStringEPKcS2_");
    if (!option_string) return 129;
    bool options_ready=option_string(options(*app),"Debug","EndGameState")!=NULL &&
        option_string(options(*app),"Debug","TutorialState")!=NULL;
    fprintf(stderr,"C6_LINUX_MOD register_application_options_return %d\n",options_ready);
    if (!options_ready) return 129;
    if (getenv("C6_MOD_CONTENT_PATHS")) {
        void *(*paths)(void)=dlsym(RTLD_DEFAULT,"_ZN3App11ContentPath7Manager3GetEv");
        bool (*setup)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Civ6App17SetupContentPathsEv");
        if (!paths || !setup || paths()) return 139;
        fprintf(stderr,"C6_LINUX_MOD content_paths_setup_enter\n");
        bool ready=setup(*app);
        constructed_content_paths=paths();
        fprintf(stderr,"C6_LINUX_MOD content_paths_setup_return result %d manager %d\n",ready,paths()!=NULL);
        if (!ready || !constructed_content_paths) return 140;
    }
    if (getenv("C6_MOD_ART_MANAGER")) {
        bool (*setup)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Civ6App15SetupArtManagerEv");
        unsigned char *allocate=dlsym(RTLD_DEFAULT,"_ZN10ArtLoading19ArtLoadingAllocator13allocFunctionE");
        unsigned char *release=dlsym(RTLD_DEFAULT,"_ZN10ArtLoading19ArtLoadingAllocator12freeFunctionE");
        if (!setup || !allocate || !release || *(void **)(allocate+0x20) || *(void **)(release+0x20)) return 143;
        fprintf(stderr,"C6_LINUX_MOD art_manager_setup_enter\n");
        bool ready=setup(*app);
        started_art_manager=ready;
        bool callbacks=*(void **)(allocate+0x20) && *(void **)(release+0x20);
        fprintf(stderr,"C6_LINUX_MOD art_manager_setup_return result %d native_allocator_callbacks %d\n",ready,callbacks);
        if (!ready || !callbacks) return 144;
    }
    if (getenv("C6_MOD_UI_CONSTRUCTOR")) {
        void **published=dlsym(RTLD_DEFAULT,"_ZN7ForgeUI9UIManagerE");
        void *(*allocate)(size_t)=dlsym(RTLD_DEFAULT,"_Znwm");
        void (*construct)(void *)=dlsym(RTLD_DEFAULT,"_ZN12AppUIManagerC1Ev");
        if (!published || *published || !allocate || !construct) return 150;
        fprintf(stderr,"C6_LINUX_MOD ui_construct_only_enter\n");
        void *ui=allocate(0x24e00);
        if (!ui) return 151;
        construct(ui);
        /* Original SetupUI 0x2f95a93..aaa: actual C1, then this publication.
         * Full renderer/StartupDesc/Initialize is intentionally not invoked. */
        *published=ui;
        constructed_ui=ui;
        fprintf(stderr,"C6_LINUX_MOD ui_construct_only_return native_published 1 renderer_uninitialized 1\n");
    }
    if (getenv("C6_MOD_CLIENT_CONSTRUCTOR")) {
        int client_result=client_construct_only();
        if (client_result) return client_result;
    }
    fprintf(stderr,"C6_LINUX_MOD init_instance_enter\n");
    bool result = initialize();
    if (result) constructed_framework=get();
    fprintf(stderr,"C6_LINUX_MOD init_instance_return %d singleton %d context %d\n",
        result, get()!=NULL, *active!=NULL);
    if (get()) {
        bool (*path)(void *,wchar_t *,unsigned)=dlsym(RTLD_DEFAULT,
            "_ZN19AppModdingFramework10GetDLLPathEPwj");
        if (path) {
            wchar_t selected[260]={0};
            bool found=path(get(),selected,260);
            fprintf(stderr,"C6_LINUX_MOD selected_module_path result %d path %ls\n",found,selected);
        }
    }
    if (get()) mod_inventory(get(),"after_initialize");
    if (!result || !get() || !*active) return 113;
    *borrowed_context = *active;
    Dl_info module;
    if (dladdr(*(void **)*active,&module))
        fprintf(stderr,"C6_LINUX_MOD active_context_module %s\n",module.dli_fname);
    return 0;
}

int c6_linux_mod_framework_configure(void) {
    void *(*get)(void) = dlsym(RTLD_DEFAULT,
        "_ZN19AppModdingFramework11GetInstanceEv");
    uint64_t (*configure)(void *) = dlsym(RTLD_DEFAULT,
        "_ZN19AppModdingFramework16ConfigureContentEv");
    if (!get || !configure) return 114;
    void *framework = get();
    if (!framework) return 115;
    fprintf(stderr,"C6_LINUX_MOD configure_content_enter\n");
    uint64_t result = configure(framework);
    fprintf(stderr,"C6_LINUX_MOD configure_content_return flags %u code %u\n",
        (unsigned)(result&0x1f),(unsigned)(result>>32));
    bool (*path)(void *,wchar_t *,unsigned)=dlsym(RTLD_DEFAULT,
        "_ZN19AppModdingFramework10GetDLLPathEPwj");
    if (path) {
        wchar_t selected[260]={0};
        bool found=path(framework,selected,260);
        fprintf(stderr,"C6_LINUX_MOD configured_module_path result %d path %ls\n",found,selected);
    }
    mod_inventory(framework,"after_configure");
    void **active=dlsym(RTLD_DEFAULT,"_ZN8GameCore16s_pActiveContextE");
    Dl_info module;
    if (active && *active && dladdr(*(void **)*active,&module))
        fprintf(stderr,"C6_LINUX_MOD configured_active_context_module %s\n",module.dli_fname);
    /* ConfigureContent alone selects/configures; it does not execute the
     * subsequent UpdateContent gameplay ApplyComponents phase. */
    return (result&1) ? 0 : 116;
}

int c6_linux_mod_framework_update_constructor_callback(void **borrowed_context) {
    void *(*get)(void)=dlsym(RTLD_DEFAULT,"_ZN19AppModdingFramework11GetInstanceEv");
    uint64_t (*update)(void *,void *)=dlsym(RTLD_DEFAULT,"_ZN19AppModdingFramework13UpdateContentENSt3__18functionIFvPN8Database20IDatabaseConnection1EbEEE");
    void *(*allocate)(size_t)=dlsym(RTLD_DEFAULT,"_Znwm");
    void (*release)(void *)=dlsym(RTLD_DEFAULT,"_ZdlPv");
    void (*owner_construct)(void *)=dlsym(RTLD_DEFAULT,"_ZN17LoadGameViewStateC1Ev");
    void (*owner_delete)(void *)=dlsym(RTLD_DEFAULT,"_ZN17LoadGameViewStateD0Ev");
    void (*callback_destruct)(void *)=dlsym(RTLD_DEFAULT,"_ZNSt3__18functionIFvPN8Database20IDatabaseConnection1EbEED2Ev");
    unsigned char *anchor=dlsym(RTLD_DEFAULT,"_ZN17LoadGameViewState22InitializeErrorResultsEv");
    int (*request)(void)=dlsym(RTLD_DEFAULT,"_ZN8GameCore10Serializer7Manager20GetRequestInProgressEv");
    void **active=dlsym(RTLD_DEFAULT,"_ZN8GameCore16s_pActiveContextE");
    if (!borrowed_context || !get || !update || !allocate || !release || !owner_construct ||
        !owner_delete || !callback_destruct || !anchor || !request || !active || !get() || !*active) return 146;
    if (request()!=0) return 147;
    /* Exact host callback constructor at 0x2fd62d0, adjacent to the exported
     * 0x2fd6230 anchor. It constructs its real inline libc++ callable, capturing
     * the real owner. Owner allocation size is original SetupGameStates 0x4f0.
     * C1's false censor flag remains untouched; Begin/Request is not simulated. */
    void (*callback_construct)(void *,void *)=(void (*)(void *,void *))(anchor+0xa0);
    void *owner=allocate(0x4f0),*callback=allocate(0x30);
    if (!owner || !callback) {
        if (owner) release(owner);
        if (callback) release(callback);
        return 148;
    }
    fprintf(stderr,"C6_LINUX_MOD update_constructor_callback_owner_enter request_none 1\n");
    owner_construct(owner);
    callback_construct(callback,owner);
    fprintf(stderr,"C6_LINUX_MOD update_content_enter callback_native_constructor_state 1\n");
    uint64_t result=update(get(),callback);
    fprintf(stderr,"C6_LINUX_MOD update_content_return flags %u code %u\n",(unsigned)(result&7),(unsigned)(result>>32));
    callback_destruct(callback);
    release(callback);
    owner_delete(owner);
    fprintf(stderr,"C6_LINUX_MOD update_constructor_callback_owner_cleanup_return\n");
    *borrowed_context=*active;
    Dl_info module;
    if (*active && dladdr(*(void **)*active,&module))
        fprintf(stderr,"C6_LINUX_MOD updated_active_context_module %s\n",module.dli_fname);
    mod_inventory(get(),"after_update");
    return (result&1)&&*active ? 0 : 149;
}

/* Genuine GUIShutdown calls DestroyInstance before Controller::Shutdown. */
int c6_linux_mod_framework_shutdown(void) {
    void *(*get)(void)=dlsym(RTLD_DEFAULT,"_ZN19AppModdingFramework11GetInstanceEv");
    void (*destroy)(void)=dlsym(RTLD_DEFAULT,"_ZN19AppModdingFramework15DestroyInstanceEv");
    bool (*unload)(void *)=dlsym(RTLD_DEFAULT,"_ZN19AppModdingFramework14UnloadGameCoreEv");
    void (*controller_shutdown)(void)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController8ShutdownEv");
    void **active=dlsym(RTLD_DEFAULT,"_ZN8GameCore16s_pActiveContextE");
    void **controller=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController21ms_pkActiveControllerE");
    if (!get || !destroy || !unload || !controller_shutdown || !active || !controller ||
        !constructed_framework || get()!=constructed_framework) return 131;
    /* Core configuration maps can retain the actual host VariantManager.
     * Release the native Core while that owner is alive, before D1's original
     * ShutdownConfiguration. Exact UpdateContent uses this same native unload
     * boundary. D1 still executes its own later idempotent unload. */
    fprintf(stderr,"C6_LINUX_MOD framework_unload_before_configuration_destroy_enter\n");
    bool unloaded=unload(get());
    fprintf(stderr,"C6_LINUX_MOD framework_unload_before_configuration_destroy_return result %d context_null %d\n",unloaded,*active==NULL);
    if (!unloaded || *active) return 154;
    fprintf(stderr,"C6_LINUX_MOD framework_destroy_enter\n");
    destroy();
    fprintf(stderr,"C6_LINUX_MOD framework_destroy_return singleton %d context %d\n",get()!=NULL,*active!=NULL);
    if (get() || *active) return 132;
    constructed_framework=NULL;
    controller_shutdown();
    fprintf(stderr,"C6_LINUX_MOD controller_shutdown_return active %d\n",*controller!=NULL);
    if (!*controller && constructed_ui) {
        void **published=dlsym(RTLD_DEFAULT,"_ZN7ForgeUI9UIManagerE");
        void (*destroy)(void *)=dlsym(RTLD_DEFAULT,"_ZN12AppUIManagerD0Ev");
        if (!published || *published!=constructed_ui || !destroy) return 152;
        fprintf(stderr,"C6_LINUX_MOD ui_construct_only_destroy_enter\n");
        destroy(constructed_ui);
        *published=NULL;
        constructed_ui=NULL;
        fprintf(stderr,"C6_LINUX_MOD ui_construct_only_destroy_return unpublished 1\n");
    }
    if (!*controller && started_art_manager) {
        void (*shutdown)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Civ6App18ShutdownArtManagerEv");
        void **app=dlsym(RTLD_DEFAULT,"g_pkApplication");
        if (!shutdown || !app || !*app) return 145;
        fprintf(stderr,"C6_LINUX_MOD art_manager_shutdown_enter\n");
        shutdown(*app);
        started_art_manager=false;
        fprintf(stderr,"C6_LINUX_MOD art_manager_shutdown_return\n");
    }
    if (!*controller && constructed_content_paths) {
        void *(*paths)(void)=dlsym(RTLD_DEFAULT,"_ZN3App11ContentPath7Manager3GetEv");
        bool (*shutdown)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Civ6App20ShutdownContentPathsEv");
        void **app=dlsym(RTLD_DEFAULT,"g_pkApplication");
        if (!paths || !shutdown || !app || !*app || paths()!=constructed_content_paths) return 141;
        bool stopped=shutdown(*app);
        fprintf(stderr,"C6_LINUX_MOD content_paths_shutdown_return result %d manager_null %d\n",stopped,paths()==NULL);
        if (!stopped || paths()) return 142;
        constructed_content_paths=NULL;
    }
    return *controller?133:0;
}

#ifdef C6_MOD_ACTIVATION_PROBE_MAIN
#include "host_services.h"
#include <stdlib.h>
#include <unistd.h>
typedef int (*c6_main_fn)(int,char **,char **);
static int actual_main_argc;
static char **actual_main_argv;
int c6_linux_main_arguments(int *argc,char ***argv) {
    if (!argc || !argv || !actual_main_argv) return 1;
    *argc=actual_main_argc;
    *argv=actual_main_argv;
    return 0;
}
static bool main_thread_prepared;
static int core_module_present(struct dl_phdr_info *module,size_t size,void *opaque) {
    (void)size;
    bool *present=opaque;
    const char *name=module->dlpi_name?strrchr(module->dlpi_name,'/'):NULL;
    name=name?name+1:module->dlpi_name;
    if (name && (!strcmp(name,"libGameCore_Base.so") || !strcmp(name,"libGameCore_XP1.so") || !strcmp(name,"libGameCore_XP2.so"))) *present=true;
    return 0;
}
static int prepare_original_main_thread(void) {
    void (*prepare)(void)=dlsym(RTLD_DEFAULT,"_ZN3ASL17PrepareMainThreadEv");
    volatile unsigned char *flags=(volatile unsigned char *)(uintptr_t)0x544cc70;
    void **application=dlsym(RTLD_DEFAULT,"g_pkApplication");
    bool core_present=false;
    dl_iterate_phdr(core_module_present,&core_present);
    fprintf(stderr,"C6_LINUX_MAIN_THREAD prepare_enter flags %u app_present %d core_present %d already_called %d\n",*flags,application&&*application,core_present,main_thread_prepared);
    if ((uintptr_t)prepare!=0x2cdf8e0 || !application || *application || core_present || main_thread_prepared || (*flags&1)) return 164;
    /* Original EmuPreMain invokes this before the game entry, App and TempHeap.
     * It performs real TLS/DLL thread notifications; never repeat after loading Core. */
    prepare();
    main_thread_prepared=true;
    core_present=false;
    dl_iterate_phdr(core_module_present,&core_present);
    fprintf(stderr,"C6_LINUX_MAIN_THREAD prepare_return flags %u app_present %d core_present %d process_scoped 1 paired_reset_not_proven 1\n",*flags,*application!=NULL,core_present);
    if (*application || core_present) return 166;
    return (*flags&1)?0:165;
}
static int mod_probe_main(int argc,char **argv,char **envp) {
    if (getenv("C6_CURRENT_HANDLER_METADATA")) observed_mod_main_tid=syscall(SYS_gettid);
    actual_main_argc=argc;
    actual_main_argv=argv;
    (void)envp;
    fprintf(stderr,"C6_LINUX_MOD host_main_intercepted\n");
    if (getenv("C6_MOD_PREPARE_MAIN_THREAD")) {
        int prepared=prepare_original_main_thread();
        if (prepared) _exit(prepared);
    }
    if (getenv("C6_MOD_FILESYSTEM")) {
        int (*filesystem)(void)=dlsym(RTLD_DEFAULT,"c6_linux_filesystem_initialize");
        bool (*directory)(const char *)=dlsym(RTLD_DEFAULT,"_ZN3ASL17ASL_SetCurrentDirEPKc");
        if (!filesystem || !directory || filesystem()) _exit(126);
        bool changed=directory("C:\\Emu\\AppAssets\\Base\\Binaries\\Linux");
        char cwd[4096];
        long cwd_result=syscall(SYS_getcwd,cwd,sizeof cwd);
        fprintf(stderr,"C6_LINUX_MOD original_asset_working_directory result %d kernel_path %s\n",changed,cwd_result>0?cwd:"unavailable");
        if (!changed || cwd_result<=0) _exit(127);
        char *(*normalize)(char *,size_t,const char *)=dlsym(RTLD_DEFAULT,"ASL_EmuToNativePath");
        bool (*exists)(const wchar_t *)=dlsym(RTLD_DEFAULT,"_ZN8Platform10PathExistsEPKw");
        char relative[4096];
        if (!normalize || !exists || normalize(relative,sizeof relative,"../../../Base/Assets/Database/Modding.sql")!=relative) _exit(137);
        fprintf(stderr,"C6_LINUX_MOD original_relative_modding_sql exists %d path %s\n",
            exists(L"../../../Base/Assets/Database/Modding.sql"),relative);
    }
    if (!c6_linux_create_host_engine_utility()) _exit(117);
    if (getenv("C6_MOD_CLIENT_ONLY")) {
        if (c6_linux_mod_register_events()) _exit(118);
        int result=client_construct_only();
        if (!result) result=c6_linux_mod_client_fixture_shutdown();
        fprintf(stderr,"C6_LINUX_MOD client_only_boundary_return %d\n",result);
        fflush(stderr);_exit(result);
    }
    if (getenv("C6_LOCALIZATION")) {
        int (*localize)(void)=dlsym(RTLD_DEFAULT,"c6_linux_initialize_localization");
        if (!localize || localize()) _exit(134);
    }
    void *context = NULL;
    int result = c6_linux_mod_framework_initialize(&context);
    bool request_prepared=false;
    const char *request_save=getenv("C6_REQUEST_LOAD_SAVE");
    if (!result && request_save) {
        int (*prepare)(const char *)=dlsym(RTLD_DEFAULT,"c6_linux_request_load_prepare");
        result=prepare?prepare(request_save):157;
        request_prepared=result==0;
        fprintf(stderr,"C6_LINUX_MOD request_prepare_boundary_return %d\n",result);
    }
    const char *saved=getenv("C6_SAVED_CONFIGURATION");
    if (!result && saved) {
        int (*restore)(const char *)=dlsym(RTLD_DEFAULT,"c6_linux_restore_saved_configuration");
        if (!restore) result=130;
        else result=restore(saved);
    }
    if (!result && getenv("C6_MOD_CONFIGURE"))
        result = c6_linux_mod_framework_configure();
    if (!result && getenv("C6_MOD_UPDATE_CONSTRUCTOR_CALLBACK"))
        result = c6_linux_mod_framework_update_constructor_callback(&context);
    if (!result && getenv("C6_RULES_SNAPSHOT_DIR")) {
        int (*snapshot)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_rules_snapshot");
        result=snapshot?snapshot(NULL,context):153;
        fprintf(stderr,"C6_LINUX_MOD rules_snapshot_return %d\n",result);
    }
    if (!result && getenv("C6_MOD_ATTACH_SAVED_SESSION")) {
        int (*attach)(void)=dlsym(RTLD_DEFAULT,"c6_linux_save_lifecycle_attach_session");
        int (*guards)(void)=dlsym(RTLD_DEFAULT,"c6_linux_network_check_guards");
        int (*leave)(void)=dlsym(RTLD_DEFAULT,"c6_linux_network_leave");
        if (!attach || !guards || !leave) result=155;
        else {
            result=attach();
            if (!result) result=guards();
            if (!result && getenv("C6_GAME_LOAD_STREAM")) {
                int (*load)(void *)=dlsym(RTLD_DEFAULT,"c6_linux_game_load");
                result=load?load(context):156;
            }
            int left=leave();
            fprintf(stderr,"C6_LINUX_MOD saved_session_control_return result %d leave %d\n",result,left);
            if (!result) result=left;
        }
    }
    if (getenv("C6_MOD_CLEANUP") && context) {
        int cleanup=0;
        if (request_prepared) {
            int (*clear)(void)=dlsym(RTLD_DEFAULT,"c6_linux_request_load_clear");
            cleanup=clear?clear():158;
            fprintf(stderr,"C6_LINUX_MOD request_clear_boundary_return %d\n",cleanup);
        }
        if (!cleanup) cleanup=c6_linux_mod_framework_shutdown();
        if (!cleanup && constructed_client) cleanup=c6_linux_mod_client_fixture_shutdown();
        if (!cleanup && getenv("C6_LOCALIZATION")) {
            int (*shutdown)(void)=dlsym(RTLD_DEFAULT,"c6_linux_shutdown_localization");
            cleanup=shutdown?shutdown():135;
        }
        if (!cleanup) cleanup=c6_linux_destroy_host_services();
        if (!cleanup && getenv("C6_MOD_FILESYSTEM")) {
            int (*shutdown)(void)=dlsym(RTLD_DEFAULT,"c6_linux_filesystem_shutdown");
            cleanup=shutdown?shutdown():136;
        }
        if (!cleanup) {
            int (*shutdown)(void)=dlsym(RTLD_DEFAULT,"c6_linux_temp_heap_shutdown");
            cleanup=shutdown?shutdown():138;
        }
        fprintf(stderr,"C6_LINUX_MOD cleanup_return %d\n",cleanup);
        if (!result) result=cleanup;
        context=NULL;
    }
    fprintf(stderr,"C6_LINUX_MOD boundary_return result %d borrowed_context %d\n",result,context!=NULL);
    /* This isolated boundary probe has not established full framework/controller
     * shutdown. Never run the unrelated original host atexit handlers. */
    fflush(stderr);
    _exit(result);
}
int __libc_start_main(c6_main_fn main,int argc,char **argv,void (*init)(void),void (*fini)(void),void (*rtld_fini)(void),void *stack_end) {
    (void)main;
    typedef int (*start_fn)(c6_main_fn,int,char **,void(*)(void),void(*)(void),void(*)(void),void*);
    start_fn original = dlsym(RTLD_NEXT,"__libc_start_main");
    return original(mod_probe_main,argc,argv,init,fini,rtld_fini,stack_end);
}
#endif
