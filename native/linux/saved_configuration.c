#define _GNU_SOURCE
#include "saved_configuration.h"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

/* Pinned Linux benchmark boundary, independently decoded by inspect_save.py.
 * Object size follows the actual Linux MemoryIStream constructor (0x28).
 * All configuration objects and mutations belong to the original host. */
static int inspect_restored(void);
int c6_linux_restore_saved_configuration(const char *path) {
    void *(*allocate)(size_t);
    void (*release)(void *);
    void (*construct)(void *,const void *,size_t);
    void (*destruct)(void *);
    void (*seek)(void *,size_t);
    size_t (*offset)(const void *);
    int (*restore)(void *);
    void *(*edit_config)(void);
    void (*set_mode)(void*,uint32_t);
#define RESOLVE(target,name) do {target=dlsym(RTLD_DEFAULT,name);if(!target){ \
    fprintf(stderr,"C6_LINUX_CONFIGURATION missing_symbol %s\n",name);return 80;}}while(0)
    RESOLVE(allocate,"_Znwm");
    RESOLVE(release,"_ZdlPv");
    RESOLVE(construct,"_ZN13Serialization13MemoryIStreamC1EPKvm");
    RESOLVE(destruct,"_ZN13Serialization13MemoryIStreamD1Ev");
    RESOLVE(seek,"_ZN13Serialization13MemoryIStream9SetOffsetEm");
    RESOLVE(offset,"_ZNK13Serialization13MemoryIStream9GetOffsetEv");
    RESOLVE(edit_config,"_ZN8GameCore3App13Configuration4Game17EditConfigurationEv");
    RESOLVE(set_mode,"_ZN8GameCore3App13Configuration4Game8Instance11SetGameModeENS_13Configuration13GameModeTypesE");
    RESOLVE(restore,"_ZN8GameCore10Serializer7Manager17LoadConfigurationERN13Serialization7IStreamE");
#undef RESOLVE
    FILE *source=fopen(path,"rb");
    if(!source)return 81;
    /* Bound the native stream to the independently observed configuration end.
     * No compressed game state is supplied to this restoration operation. */
    unsigned char *bytes=malloc(103901);
    if(!bytes){fclose(source);return 82;}
    size_t count=fread(bytes,1,103901,source);
    int failed=ferror(source);fclose(source);
    if(count!=103901||failed){free(bytes);return 83;}
    void *stream=allocate(0x28);
    if(!stream){free(bytes);return 84;}
    construct(stream,bytes,count);seek(stream,8842);
    fprintf(stderr,"C6_LINUX_CONFIGURATION restore_enter offset %zu\n",offset(stream));
    /* Matching Linux DWARF GameModeTypes DIE 0x2c8ce: SINGLEPLAYER is
     * -379035929 (uint32 bits 3915931367), exactly the saved benchmark mode.
     * LoadConfiguration intentionally preserves the caller-selected mode. */
    set_mode(edit_config(),UINT32_C(3915931367));
    fprintf(stderr,"C6_LINUX_CONFIGURATION selected_mode SINGLEPLAYER 3915931367\n");
    int result=restore(stream);
    size_t end=offset(stream);
    fprintf(stderr,"C6_LINUX_CONFIGURATION restore_return result %d offset %zu\n",result,end);
    destruct(stream);release(stream);free(bytes);
    if(result!=0||end!=103901)return 85;
    return inspect_restored();
}

/* Query and serialize genuine singleton state; never fabricate typed fields. */
static int inspect_restored(void) {
    void *(*get)(void)=dlsym(RTLD_DEFAULT,"_ZN8GameCore3App13Configuration4Game16GetConfigurationEv");
    void *(*values)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance13GetGameValuesEv");
    void *(*variant)(const void*,uint32_t)=dlsym(RTLD_DEFAULT,"_ZNK4Data15TypedVariantMap10GetVariantEj");
    const char *(*text)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK4Data12TypedVariantcvPKcEv");
    unsigned (*enabled)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance18GetEnabledModCountEv");
    unsigned (*required)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance19GetRequiredModCountEv");
    if(!get||!values||!variant||!text||!enabled||!required)return 86;
    void *config=get();
    const char *ruleset=text(variant(values(config),0x34a1e258));
    if(!ruleset||strcmp(ruleset,"RULESET_EXPANSION_2"))return 87;
    fprintf(stderr,"C6_LINUX_CONFIGURATION ruleset %s enabled_mods %u required_mods %u\n",ruleset,enabled(config),required(config));
    const char *(*enabled_id)(const void*,unsigned)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance15GetEnabledModIdEj");
    const char *(*required_id)(const void*,unsigned)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance16GetRequiredModIdEj");
    void *(*player)(const void*,unsigned)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance22GetPlayerConfigurationEj");
    const char *(*civilization)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration6Player8Instance23GetCivilizationTypeNameEv");
    const char *(*leader)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration6Player8Instance17GetLeaderTypeNameEv");
    int (*team)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration6Player8Instance7GetTeamEv");
    if(!enabled_id||!required_id||!player||!civilization||!leader||!team)return 91;
    for(unsigned i=0;i<enabled(config);++i)
        fprintf(stderr,"C6_LINUX_CONFIGURATION enabled_mod %u %s\n",i,enabled_id(config,i));
    for(unsigned i=0;i<required(config);++i)
        fprintf(stderr,"C6_LINUX_CONFIGURATION required_mod %u %s\n",i,required_id(config,i));
    /* ID 0 is observed in the pinned save's player-settings prefix, not invented. */
    void *p=player(config,0);if(!p)return 92;
    fprintf(stderr,"C6_LINUX_CONFIGURATION player 0 civilization %s leader %s team %d\n",civilization(p),leader(p),team(p));
    const char *key_names[]={"SyncRandomSeed","GameMode","CloudTurnPlayer","SavedGame","MatchMaking","GamePaused","GameReady"};
    for(unsigned i=0;i<sizeof key_names/sizeof key_names[0];++i){
        char symbol[160];
        snprintf(symbol,sizeof symbol,"_ZN8GameCore3App13Configuration4Game%zuKey_%sE",strlen(key_names[i])+4,key_names[i]);
        const uint32_t *key=dlsym(RTLD_DEFAULT,symbol);
        if(!key)return 93;
        fprintf(stderr,"C6_LINUX_CONFIGURATION key %s %08x\n",key_names[i],*key);
    }
    const char *path=getenv("C6_SAVED_CONFIGURATION_OUTPUT");
    if(path){
        void *(*allocate)(size_t)=dlsym(RTLD_DEFAULT,"_Znwm");
        void (*construct)(void*,void*,size_t)=dlsym(RTLD_DEFAULT,"_ZN13Serialization13MemoryOStreamC1EPvm");
        void (*destruct)(void*)=dlsym(RTLD_DEFAULT,"_ZN13Serialization13MemoryOStreamD0Ev");
        size_t (*offset)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK13Serialization13MemoryOStream9GetOffsetEv");
        void (*write_config)(const void*,void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance5WriteERN13Serialization7OStreamE");
        if(!allocate||!construct||!destruct||!offset||!write_config)return 88;
        unsigned char *bytes=malloc(1048576);if(!bytes)return 89;
        void *stream=allocate(0x28);construct(stream,bytes,1048576);
        write_config(config,stream);size_t count=offset(stream);
        FILE *out=fopen(path,"wbx");
        int valid=out&&fwrite(bytes,1,count,out)==count;
        if(out&&fclose(out))valid=0;
        destruct(stream);free(bytes);
        if(!valid)return 90;
        fprintf(stderr,"C6_LINUX_CONFIGURATION canonical_game_written bytes %zu\n",count);
    }
    return 0;
}
