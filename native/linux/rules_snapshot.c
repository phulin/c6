#define _GNU_SOURCE
#include "rules_snapshot.h"
#include <dlfcn.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef int (*Prepare)(void*,const char*,int,void**,const char**);
typedef int (*Step)(void*);
typedef const unsigned char *(*Text)(void*,int);
typedef int64_t (*Integer)(void*,int);
static Prepare prepare;
static Step step;
static Text text;
static Integer integer;
static int (*finalize)(void*);
static void *(*sqlite)(void*);
static int scalar(void *db,const char *sql,int64_t *value) {
    void *statement=NULL;
    if(prepare(sqlite(db),sql,-1,&statement,NULL))return 1;
    int result=step(statement);
    if(result==100)*value=integer(statement,0);
    int ended=finalize(statement);
    return result!=100||ended;
}
static int counts(void *source,void *destination,unsigned *tables,int64_t *rows) {
    int64_t a,b;
    if(scalar(source,"SELECT count(*) FROM sqlite_master WHERE type='table'",&a)||
       scalar(destination,"SELECT count(*) FROM sqlite_master WHERE type='table'",&b)||a!=b)return 1;
    void *statement=NULL;
    if(prepare(sqlite(source),"SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",-1,&statement,NULL))return 1;
    *tables=0;*rows=0;int status;
    while((status=step(statement))==100){
        const char *name=(const char*)text(statement,0);
        if(!name||strlen(name)>1024){finalize(statement);return 1;}
        char sql[2100]="SELECT count(*) FROM \"";size_t at=strlen(sql);
        for(size_t i=0;name[i];++i){sql[at++]=name[i];if(name[i]=='"')sql[at++]='"';}
        sql[at++]='"';sql[at]=0;
        int64_t x,y;
        if(scalar(source,sql,&x)||scalar(destination,sql,&y)||x!=y||x<0||x>INT64_MAX-*rows){finalize(statement);return 1;}
        ++*tables;*rows+=x;
    }
    int ended=finalize(statement);
    return status!=101||ended||*tables!=(uint64_t)a;
}
int c6_linux_rules_snapshot(void *gamecore,void *context) {
    (void)gamecore;(void)context;
    const char *directory=getenv("C6_RULES_SNAPSHOT_DIR");
    if(!directory||directory[0]!='/')return 70;
    void **app=dlsym(RTLD_DEFAULT,"g_pkApplication");
    void *(*getters[2])(void*)={
        dlsym(RTLD_DEFAULT,"_ZN7Civ6App34GetConfigurationDatabaseConnectionEv"),
        dlsym(RTLD_DEFAULT,"_ZN7Civ6App29GetGameplayDatabaseConnectionEv")};
    void *(*create)(const char*,int,void*)=dlsym(RTLD_DEFAULT,"_ZN8Database16CreateConnectionEPKciPNS_15IDatabaseLoggerE");
    bool (*backup)(const void*,void*)=dlsym(RTLD_DEFAULT,"_ZNK8Database10Connection8BackupToEPNS_20IDatabaseConnection1E");
    void (*destroy)(void*)=dlsym(RTLD_DEFAULT,"_ZN8Database10Connection7DestroyEv");
    sqlite=dlsym(RTLD_DEFAULT,"_ZN8Database10Connection9GetSQLiteEv");
    prepare=dlsym(RTLD_DEFAULT,"sqlite3_prepare_v2");step=dlsym(RTLD_DEFAULT,"sqlite3_step");
    text=dlsym(RTLD_DEFAULT,"sqlite3_column_text");integer=dlsym(RTLD_DEFAULT,"sqlite3_column_int64");
    finalize=dlsym(RTLD_DEFAULT,"sqlite3_finalize");
    if(!app||!*app||!getters[0]||!getters[1]||!create||!backup||!destroy||
       !sqlite||!prepare||!step||!text||!integer||!finalize)return 71;
    for(unsigned i=0;i<2;++i){
        const char *label=i?"gameplay":"configuration";char path[4096];
        int length=snprintf(path,sizeof path,"%s/%s.sqlite",directory,label);
        if(length<0||(size_t)length>=sizeof path)return 72;
        int fd=open(path,O_CREAT|O_EXCL|O_WRONLY|O_NOFOLLOW,0600);
        if(fd<0)return 73;
        close(fd);
        void *source=getters[i](*app),*destination=create(path,6,NULL);
        bool copied=source&&destination&&backup(source,destination);
        unsigned tables=0;int64_t rows=0;
        bool matched=copied&&!counts(source,destination,&tables,&rows);
        if(destination)destroy(destination);
        fprintf(stderr,"C6_LINUX_RULES {\"phase\":\"original_database_backup\",\"database\":\"%s\",\"native_success\":%s,\"row_counts_match\":%s,\"tables\":%u,\"rows\":%lld,\"result\":%d,\"native_symbol\":\"Database::Connection::BackupTo\"}\n",label,copied?"true":"false",matched?"true":"false",tables,(long long)rows,matched?0:74);
        if(!matched){unlink(path);return 74;}
    }
    return 0;
}
