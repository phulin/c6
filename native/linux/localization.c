#define _GNU_SOURCE
#include "localization.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wchar.h>

static void *database;
static bool initialized;
static void *symbol(const char *name) {
    void *address=dlsym(RTLD_DEFAULT,name);
    if(!address) fprintf(stderr,"C6_LINUX_LOCALIZATION missing_symbol %s\n",name);
    return address;
}
int c6_linux_initialize_localization(void) {
    if(database||initialized) return 1;
    void *(*allocate)(size_t)=symbol("_Znwm");
    void (*construct)(void *)=symbol("_ZN11Application4Data20LocalizationDatabaseC1Ev");
    bool (*db_initialize)(void *,const wchar_t *)=symbol("_ZN11Application4Data20LocalizationDatabase10InitializeEPKw");
    void *(*connection)(void *)=symbol("_ZN11Application4Data20LocalizationDatabase21GetDatabaseConnectionEv");
    bool (*initialize)(const wchar_t *,void *,const char *,bool,void *)=
        symbol("_ZN12Localization10InitializeEPKwPN8Database20IDatabaseConnection1EPKcbPNS_19ILocalizationLoggerE");
    bool (*dictionary)(void *,const char *,bool)=symbol("_ZN12Localization20InitializeDictionaryEPN8Database20IDatabaseConnection1EPKcb");
    void **global_dictionary=symbol("_ZN12Localization14m_pkDictionaryE");
    const char *cwd=getenv("C6_LOCALIZATION_BOOTSTRAP_CWD");
    const char *icu=getenv("C6_LOCALIZATION_ICU");
    wchar_t wide_icu[1024];
    if(!allocate||!construct||!db_initialize||!connection||!initialize||!dictionary||
       !global_dictionary||!cwd||!icu||strlen(icu)>=1024||
       mbstowcs(wide_icu,icu,1024)==(size_t)-1) return 2;
    if(*global_dictionary) return 3;
    bool null_icu_rejected=!initialize(NULL,NULL,"en_US",false,NULL);
    bool null_db_rejected=!dictionary(NULL,"en_US",false);
    fprintf(stderr,"C6_LINUX_LOCALIZATION negative_null_inputs %d %d dictionary_null %d\n",
            null_icu_rejected,null_db_rejected,*global_dictionary==NULL);
    if(!null_icu_rejected||!null_db_rejected||*global_dictionary) return 4;
    /* Original Civ6App::SetupDatabase allocation site uses 0x18 bytes. */
    database=allocate(0x18);
    if(!database) return 5;
    construct(database);
    char *previous=getcwd(NULL,0);
    if(!previous||chdir(cwd)) { free(previous); return 6; }
    fprintf(stderr,"C6_LINUX_LOCALIZATION database_initialize_enter\n");
    bool db_ok=db_initialize(database,L"c6-no-localization-cache.sqlite");
    int restored=chdir(previous);
    free(previous);
    fprintf(stderr,"C6_LINUX_LOCALIZATION database_initialize_return %d cwd_restored %d\n",db_ok,!restored);
    if(!db_ok||restored) return 7;
    void *db=connection(database);
    if(!db) return 8;
    fprintf(stderr,"C6_LINUX_LOCALIZATION initialize_enter\n");
    initialized=initialize(wide_icu,db,"en_US",false,NULL);
    fprintf(stderr,"C6_LINUX_LOCALIZATION initialize_return %d dictionary_nonnull %d\n",initialized,*global_dictionary!=NULL);
    if(!initialized||!*global_dictionary) return 9;
    void *(*lookup)(void *,const char *)=symbol("_ZN12Localization6LookupEPKc");
    const char *(*utf8)(void *,unsigned *,int)=symbol("_ZN12Localization6String6toUTF8ERji");
    void (*destroy)(void *)=symbol("_ZN12Localization6StringD1Ev");
    bool (*set_language)(const char *)=symbol("_ZN12Localization18SetCurrentLanguageEPKc");
    void *(*get_language)(void)=symbol("_ZN12Localization18GetCurrentLanguageEv");
    const char *(*language_type)(void *)=symbol("_ZNK12Localization8Language7GetTypeEv");
    if(!lookup||!utf8||!destroy||!set_language||!get_language||!language_type) return 10;
    /* Bounded storage belongs entirely to original String construction. The
     * hidden result pointer is proven by Linux Lookup instructions. */
    union { max_align_t align; unsigned char bytes[512]; } result;
    unsigned length=0;
    lookup(result.bytes,"LOC_TECH_POTTERY_NAME");
    const char *text=utf8(result.bytes,&length,1);
    bool match=text&&length==7&&!memcmp(text,"Pottery",7);
    destroy(result.bytes);
    fprintf(stderr,"C6_LINUX_LOCALIZATION lookup_pottery_match %d length %u\n",match,length);
    bool rejected=!set_language("c6_invalid_locale");
    bool retained=!strcmp(language_type(get_language()),"en_US");
    fprintf(stderr,"C6_LINUX_LOCALIZATION invalid_locale_rejected %d en_US_retained %d\n",rejected,retained);
    return match&&rejected&&retained ? 0 : 11;
}
int c6_linux_shutdown_localization(void) {
    void (*shutdown)(void)=symbol("_ZN12Localization8ShutdownEv");
    void (*destruct)(void *)=symbol("_ZN11Application4Data20LocalizationDatabaseD1Ev");
    void (*release)(void *)=symbol("_ZdlPv");
    void **dictionary=symbol("_ZN12Localization14m_pkDictionaryE");
    if(!shutdown||!destruct||!release||!dictionary) return 1;
    if(initialized) { shutdown(); initialized=false; }
    if(database) { destruct(database); release(database); database=NULL; }
    fprintf(stderr,"C6_LINUX_LOCALIZATION shutdown_return dictionary_null %d\n",*dictionary==NULL);
    return *dictionary ? 2 : 0;
}
