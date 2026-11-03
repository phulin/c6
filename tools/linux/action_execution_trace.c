#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Observation delegates to original Linux methods. A guarded unexpected policy
 * or game-update entry terminates the diagnostic process; it never substitutes
 * a policy result or allows that run to pass. No code/vtable bytes are patched. */
static void *trace_module;
static FILE *trace_log;
static atomic_bool armed;
static atomic_uint queued,received,executed,reported,phase;
void c6_linux_action_trace_mark(unsigned value) {
    atomic_store(&phase,value);
    if(atomic_load(&armed))fprintf(trace_log,"{\"event\":\"phase\",\"value\":%u}\n",value);
}
unsigned c6_linux_action_trace_unit_executed(void) {return atomic_load(&executed);}
static unsigned packet_number;
static const char *trace_directory;
static const char *report_profile="proven_skipturn_callers_v2";

static void report_return(unsigned sequence,int result,bool unchanged) {
    if(atomic_load(&armed))fprintf(trace_log,"{\"event\":\"report_forward_return\",\"sequence\":%u,\"result\":%d,\"input_unchanged\":%s}\n",sequence,result,unchanged?"true":"false");
}

static _Noreturn void fatal(const char *reason) {
    fprintf(stderr,"C6_LINUX_ACTION_TRACE failing_boundary %s action_acceptance 0\n",reason);
    if(trace_log){fprintf(trace_log,"{\"event\":\"failing_boundary\",\"reason\":\"%s\"}\n",reason);fflush(trace_log);}
    _exit(198);
}
static bool probe_owned(const void *pointer) {
    Dl_info probe={0},target={0};
    /* fatal is local to this translation unit and cannot be preempted by a
     * same-named executable export. Compare both live targets and hook values
     * independently to this actual loaded preload owner. */
    return dladdr((const void *)fatal,&probe)&&dladdr(pointer,&target)&&probe.dli_fbase==target.dli_fbase;
}
static void *original(const char *symbol,void *wrapper,void *caller) {
    void *value=NULL;
    if(atomic_load(&armed))value=dlsym(trace_module,symbol);
    else {
        Dl_info source,resolved;
        void *module=NULL;
        if(dladdr(caller,&source)&&source.dli_fname){
            const char *leaf=strrchr(source.dli_fname,'/');leaf=leaf?leaf+1:source.dli_fname;
            if(!strcmp(leaf,"libGameCore_Base.so")||!strcmp(leaf,"libGameCore_XP1.so")||!strcmp(leaf,"libGameCore_XP2.so")){
                module=dlopen(source.dli_fname,RTLD_NOW|RTLD_NOLOAD);
                if(module)value=dlsym(module,symbol);
                if(value&&(!dladdr(value,&resolved)||resolved.dli_fbase!=source.dli_fbase))value=NULL;
                if(value&&value!=wrapper)
                    fprintf(stderr,"C6_LINUX_ACTION_TRACE unarmed_entry symbol %s caller_module %s original_module %s original_rva %llx\n",symbol,source.dli_fname,resolved.dli_fname,(unsigned long long)((uintptr_t)value-(uintptr_t)resolved.dli_fbase));
                if(module)dlclose(module);
            }
        }
    }
    if(!value||value==wrapper){
        Dl_info source={0};(void)dladdr(caller,&source);
        fprintf(stderr,"C6_LINUX_ACTION_TRACE unresolved_symbol %s caller_module %s armed %d\n",symbol,source.dli_fname?source.dli_fname:"unknown",atomic_load(&armed));
        fatal("original_symbol_resolution");
    }
    return value;
}
static void packet(const char *kind,int from,const void *stream) {
    const void *(*bytes)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK13Serialization13MemoryIStream8GetBytesEv");
    size_t (*size)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK13Serialization13MemoryIStream7GetSizeEv");
    size_t (*offset)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK13Serialization13MemoryIStream9GetOffsetEv");
    if(!bytes||!size||!offset)fatal("packet_stream_provider");
    size_t count=size(stream),position=offset(stream);
    const void *data=bytes(stream);
    if(!data||!count||count>1048576||position>count)fatal("unsupported_packet_boundary");
    char path[4096];unsigned number=++packet_number;
    int length=snprintf(path,sizeof path,"%s/%03u-%s.bin",trace_directory,number,kind);
    if(length<0||(size_t)length>=sizeof path)fatal("packet_output_path");
    FILE *out=fopen(path,"wx");if(!out)fatal("packet_output_open");
    bool failed=fwrite(data,1,count,out)!=count;
    if(fclose(out)||failed)fatal("packet_output_write");
    fprintf(trace_log,"{\"event\":\"%s\",\"from\":%d,\"bytes\":%zu,\"offset\":%zu,\"packet\":%u}\n",kind,from,count,position,number);
}

bool trace_make_request(const void *,uint32_t,const void *,void *) __asm__("_ZNK8GameCore4Unit10Operations7Manager11MakeRequestENS1_5Types8TypeHashERKN4Data15TypedVariantMapEPNS1_7RequestE");
bool trace_make_request(const void *manager,uint32_t hash,const void *params,void *request) {
    bool (*call)(const void *,uint32_t,const void *,void *)=original("_ZNK8GameCore4Unit10Operations7Manager11MakeRequestENS1_5Types8TypeHashERKN4Data15TypedVariantMapEPNS1_7RequestE",trace_make_request,__builtin_return_address(0));
    if(atomic_load(&armed))fprintf(trace_log,"{\"event\":\"make_request_enter\",\"hash\":%u,\"phase\":%u}\n",hash,atomic_load(&phase));
    bool accepted=call(manager,hash,params,request);
    if(atomic_load(&armed)){
        uint32_t operation,flags;memcpy(&operation,request,4);memcpy(&flags,(unsigned char *)request+24,4);
        fprintf(trace_log,"{\"event\":\"make_request_return\",\"accepted\":%s,\"operation\":%u,\"flags\":%u,\"bAppend\":%u,\"bFromUser\":%u,\"phase\":%u}\n",accepted?"true":"false",operation,flags,((unsigned char *)request)[28],((unsigned char *)request)[29],atomic_load(&phase));
    }
    return accepted;
}
void trace_broadcast(void *) __asm__("_ZN8GameCore8Messages7Manager16BroadcastMessageERNS0_11BaseMessageE");
void trace_broadcast(void *message) {
    void (*call)(void *)=original("_ZN8GameCore8Messages7Manager16BroadcastMessageERNS0_11BaseMessageE",trace_broadcast,__builtin_return_address(0));
    if(atomic_load(&armed))fprintf(trace_log,"{\"event\":\"broadcast_enter\",\"phase\":%u}\n",atomic_load(&phase));
    call(message);
    if(atomic_load(&armed))fprintf(trace_log,"{\"event\":\"broadcast_return\",\"phase\":%u}\n",atomic_load(&phase));
}
void trace_queue(int,void *) __asm__("_ZN8GameCore8Messages7Manager12QueueMessageEiRN13Serialization13MemoryIStreamE");
void trace_queue(int from,void *stream) {
    void (*call)(int,void *)=original("_ZN8GameCore8Messages7Manager12QueueMessageEiRN13Serialization13MemoryIStreamE",trace_queue,__builtin_return_address(0));
    if(atomic_load(&armed)){atomic_fetch_add(&queued,1);packet("queue",from,stream);}
    call(from,stream);
}
void trace_receive(int,void *) __asm__("_ZN8GameCore8Messages7Manager14ReceiveMessageEiRN13Serialization13MemoryIStreamE");
void trace_receive(int from,void *stream) {
    void (*call)(int,void *)=original("_ZN8GameCore8Messages7Manager14ReceiveMessageEiRN13Serialization13MemoryIStreamE",trace_receive,__builtin_return_address(0));
    if(atomic_load(&armed)){atomic_fetch_add(&received,1);packet("receive",from,stream);}
    call(from,stream);
}
void trace_unit_execute(const void *) __asm__("_ZNK8GameCore8Messages13UnitOperation7ExecuteEv");
void trace_unit_execute(const void *message) {
    void (*call)(const void *)=original("_ZNK8GameCore8Messages13UnitOperation7ExecuteEv",trace_unit_execute,__builtin_return_address(0));
    if(atomic_load(&armed)){atomic_fetch_add(&executed,1);fprintf(trace_log,"{\"event\":\"unit_execute_enter\"}\n");}
    call(message);
    if(atomic_load(&armed))fprintf(trace_log,"{\"event\":\"unit_execute_return\"}\n");
}
int trace_event_id(uint64_t,const void *) __asm__("_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDE");
int trace_event_id(uint64_t event,const void *identity) {
    int (*call)(uint64_t,const void *)=original("_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDE",trace_event_id,__builtin_return_address(0));
    uint64_t bits=0;unsigned sequence=0;
    if(atomic_load(&armed)){memcpy(&bits,identity,8);sequence=atomic_fetch_add(&reported,1)+1;fprintf(trace_log,"{\"event\":\"report_identity\",\"sequence\":%u,\"type_bits\":\"%016llx\",\"component_bits\":\"%016llx\"}\n",sequence,(unsigned long long)event,(unsigned long long)bits);}
    int result=call(event,identity);
    uint64_t after=0;if(sequence)memcpy(&after,identity,8);
    report_return(sequence,result,!sequence||after==bits);
    return result;
}
int trace_event_id_int(uint64_t,const void *,int) __asm__("_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDEi");
int trace_event_id_int(uint64_t event,const void *identity,int value) {
    int (*call)(uint64_t,const void *,int)=original("_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDEi",trace_event_id_int,__builtin_return_address(0));
    uint64_t bits=0;unsigned sequence=0;
    if(atomic_load(&armed)){memcpy(&bits,identity,8);sequence=atomic_fetch_add(&reported,1)+1;fprintf(trace_log,"{\"event\":\"report_identity_int\",\"sequence\":%u,\"type_bits\":\"%016llx\",\"component_bits\":\"%016llx\",\"value\":%d}\n",sequence,(unsigned long long)event,(unsigned long long)bits,value);}
    int result=call(event,identity,value);
    uint64_t after=0;if(sequence)memcpy(&after,identity,8);
    report_return(sequence,result,!sequence||after==bits);
    return result;
}
int trace_event_id_two_int(uint64_t,const void *,int,int) __asm__("_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDEii");
int trace_event_id_two_int(uint64_t event,const void *identity,int first,int second) {
    int (*call)(uint64_t,const void *,int,int)=original("_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS_6Player11ComponentIDEii",trace_event_id_two_int,__builtin_return_address(0));
    uint64_t bits=0;unsigned sequence=0;
    if(atomic_load(&armed)){memcpy(&bits,identity,8);sequence=atomic_fetch_add(&reported,1)+1;fprintf(trace_log,"{\"event\":\"report_identity_two_int\",\"sequence\":%u,\"type_bits\":\"%016llx\",\"component_bits\":\"%016llx\",\"first\":%d,\"second\":%d}\n",sequence,(unsigned long long)event,(unsigned long long)bits,first,second);}
    int result=call(event,identity,first,second);
    uint64_t after=0;if(sequence)memcpy(&after,identity,8);
    report_return(sequence,result,!sequence||after==bits);
    return result;
}

int trace_event_type(uint64_t) __asm__("_ZN8GameCore9Reporting5Event4SendENS1_4TypeE");
int trace_event_type(uint64_t type) {
    int (*call)(uint64_t)=original("_ZN8GameCore9Reporting5Event4SendENS1_4TypeE",trace_event_type,__builtin_return_address(0));
    unsigned sequence=0;
    if(atomic_load(&armed)){sequence=atomic_fetch_add(&reported,1)+1;fprintf(trace_log,"{\"event\":\"report_type\",\"sequence\":%u,\"type_bits\":\"%016llx\"}\n",sequence,(unsigned long long)type);}
    int result=call(type);report_return(sequence,result,true);return result;
}

int trace_event_two_int(uint64_t,int,int) __asm__("_ZN8GameCore9Reporting5Event4SendENS1_4TypeEii");
int trace_event_two_int(uint64_t type,int first,int second) {
    int (*call)(uint64_t,int,int)=original("_ZN8GameCore9Reporting5Event4SendENS1_4TypeEii",trace_event_two_int,__builtin_return_address(0));
    unsigned sequence=0;
    if(atomic_load(&armed)){sequence=atomic_fetch_add(&reported,1)+1;fprintf(trace_log,"{\"event\":\"report_two_int\",\"sequence\":%u,\"type_bits\":\"%016llx\",\"first\":%d,\"second\":%d}\n",sequence,(unsigned long long)type,first,second);}
    int result=call(type,first,second);report_return(sequence,result,true);return result;
}

/* Exact Value32/TypedVariant24 layout and original native accessor bodies are
 * pinned by action_event_audit.py. No constructor, conversion or mutation is
 * used here. Unknown types remain explicitly unsupported observations. */
static bool report_value(const void *value,const void **pointee) {
    uint32_t key,element_size,count_flags;uint16_t type,modifiers;
    const unsigned char *p=value;
    memcpy(&key,p,4);memcpy(&type,p+8,2);memcpy(&modifiers,p+10,2);
    memcpy(&element_size,p+24,4);memcpy(&count_flags,p+28,4);
    fprintf(trace_log,"{\"key\":%u,\"variant_type\":%u,\"modifiers\":%u,\"element_size\":%u,\"element_count\":%u,\"count_flags_raw\":%u",key,type,modifiers,element_size,count_flags&0xffffffu,count_flags);
    bool supported=false;
    if(type==1&&modifiers==0&&element_size==0&&count_flags==0&&p[16]<=1){
        fprintf(trace_log,",\"kind\":\"bool\",\"value\":%s",p[16]?"true":"false");supported=true;
    }else if(key==UINT32_C(0xeab69e5a)&&type==1024&&element_size==8&&(count_flags&0x20000000u)&&(count_flags&0xffffffu)==1){
        const void *(*get_as)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK4Data12TypedVariant5GetAsIN8GameCore6Player11ComponentIDEEEPKT_v");
        const void *data=get_as?get_as(p+8):NULL;
        if(data){uint64_t bits;memcpy(&bits,data,8);*pointee=data;fprintf(trace_log,",\"kind\":\"component_id\",\"component_bits\":\"%016llx\"",(unsigned long long)bits);supported=true;}
    }
    if(!supported)fprintf(trace_log,",\"kind\":\"unclassified\"");
    fprintf(trace_log,",\"typed_capture_supported\":%s}",supported?"true":"false");return supported;
}

int trace_event_values(uint64_t,const void *,const void *) __asm__("_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS0_5ValueES5_");
int trace_event_values(uint64_t type,const void *first,const void *second) {
    int (*call)(uint64_t,const void *,const void *)=original("_ZN8GameCore9Reporting5Event4SendENS1_4TypeERKNS0_5ValueES5_",trace_event_values,__builtin_return_address(0));
    unsigned sequence=0;unsigned char a[32],b[32];
    const void *first_pointee=NULL,*second_pointee=NULL;uint64_t first_bits=0,second_bits=0;
    if(atomic_load(&armed)){
        memcpy(a,first,32);memcpy(b,second,32);sequence=atomic_fetch_add(&reported,1)+1;
        flockfile(trace_log);
        fprintf(trace_log,"{\"event\":\"report_value_value\",\"sequence\":%u,\"type_bits\":\"%016llx\",\"values\":[",sequence,(unsigned long long)type);
        (void)report_value(first,&first_pointee);fputc(',',trace_log);(void)report_value(second,&second_pointee);fprintf(trace_log,"]}\n");
        funlockfile(trace_log);
        if(first_pointee)memcpy(&first_bits,first_pointee,8);
        if(second_pointee)memcpy(&second_bits,second_pointee,8);
    }
    int result=call(type,first,second);
    uint64_t first_after=first_bits,second_after=second_bits;
    if(first_pointee)memcpy(&first_after,first_pointee,8);
    if(second_pointee)memcpy(&second_after,second_pointee,8);
    report_return(sequence,result,!sequence||(!memcmp(a,first,32)&&!memcmp(b,second,32)&&first_bits==first_after&&second_bits==second_after));
    return result;
}
#define STOP_VOID(NAME,SYMBOL,REASON) \
void NAME(void *) __asm__(SYMBOL); \
void NAME(void *receiver){ \
    if(atomic_load(&armed))fatal(REASON); \
    (void)receiver; \
    fprintf(stderr,"C6_LINUX_ACTION_TRACE unarmed_guarded_entry %s\n",SYMBOL); \
    fatal("unarmed_" REASON); \
}
STOP_VOID(stop_game_update,"_ZN8GameCore4Game8Instance6UpdateEv","unexpected_Game_Update")
STOP_VOID(stop_processor_update,"_ZN8GameCore4Game9Processor6UpdateEv","unexpected_GameProcessor_Update")
STOP_VOID(stop_ai_control,"_ZN8GameCore2AI7Control7Manager6DoTurnEv","unresolved_AI_Control_DoTurn")
STOP_VOID(stop_ai_process,"_ZN8GameCore2AI8GlobalAi9ProcessAiEv","unresolved_GlobalAi_ProcessAi")
#undef STOP_VOID
int stop_diplomatic_action(void *,const void *,int) __asm__("_ZN8GameCore2AI10Diplomatic13ProcessActionERKNS_9Diplomacy6Action8InstanceENS_11PlayerTypesE");
int stop_diplomatic_action(void *receiver,const void *action,int player) {
    if(atomic_load(&armed))fatal("unresolved_Diplomatic_ProcessAction_callback");
    (void)receiver;(void)action;(void)player;
    fprintf(stderr,"C6_LINUX_ACTION_TRACE unarmed_guarded_entry Diplomatic_ProcessAction\n");
    fatal("unarmed_unresolved_Diplomatic_ProcessAction_callback");
}
int stop_diplomatic_deal(void *,int,const void *,bool) __asm__("_ZN8GameCore2AI10Diplomatic11ProcessDealENS_9Diplomacy4Deal14ProposalActionERKNS3_8InstanceEb");
int stop_diplomatic_deal(void *receiver,int proposal,const void *deal,bool flag) {
    if(atomic_load(&armed))fatal("unresolved_Diplomatic_ProcessDeal_callback");
    (void)receiver;(void)proposal;(void)deal;(void)flag;
    fprintf(stderr,"C6_LINUX_ACTION_TRACE unarmed_guarded_entry Diplomatic_ProcessDeal\n");
    fatal("unarmed_unresolved_Diplomatic_ProcessDeal_callback");
}

bool stop_pending_statement(void *) __asm__("_ZN8GameCore9Diplomacy7Manager16ProcessStatementEPNS0_9Statement8InstanceE");
bool stop_pending_statement(void *statement) {
    (void)statement;
    fprintf(stderr,"C6_LINUX_ACTION_TRACE guarded_entry ProcessStatement armed %d\n",atomic_load(&armed));
    fatal("unresolved_pending_statement_processing");
}

int c6_linux_action_trace_begin(void *module) {
    if(!module||atomic_load(&armed)||trace_log)return 202;
    trace_directory=getenv("C6_ACTION_TRACE_DIRECTORY");if(!trace_directory)return 203;
    char path[4096];int n=snprintf(path,sizeof path,"%s/events.jsonl",trace_directory);
    if(n<0||(size_t)n>=sizeof path)return 204;
    trace_log=fopen(path,"wx");if(!trace_log)return 205;
    trace_module=module;packet_number=0;atomic_store(&phase,0);
    atomic_store(&queued,0);atomic_store(&received,0);atomic_store(&executed,0);atomic_store(&reported,0);
    void **table=dlsym(module,"_ZTVN8GameCore8Messages13UnitOperationE");
    bool unit_hook_owned=probe_owned((void *)trace_unit_execute);
    bool unit_target_owned=table&&probe_owned(table[2+0x20/8]);
    bool virtual_bound=table&&unit_hook_owned&&unit_target_owned&&table[2+0x20/8]==(void *)trace_unit_execute;
    void *(*get_manager)(void)=dlsym(module,"_ZN8GameCore4Unit10Operations7Manager3GetEv");
    void *manager=get_manager?get_manager():NULL;
    bool make_hook_owned=probe_owned((void *)trace_make_request);
    bool make_target_owned=manager&&probe_owned((*(void ***)manager)[0x100/8]);
    bool make_bound=manager&&make_hook_owned&&make_target_owned&&(*(void ***)manager)[0x100/8]==(void *)trace_make_request;
    fprintf(trace_log,"{\"event\":\"begin\",\"reporting_observer_profile\":\"%s\",\"UnitOperation_Execute_virtual_interposed\":%s,\"hook_owner_preload\":%s,\"target_owner_preload\":%s}\n",report_profile,virtual_bound?"true":"false",unit_hook_owned?"true":"false",unit_target_owned?"true":"false");
    fprintf(trace_log,"{\"event\":\"original_virtual_binding\",\"name\":\"UnitManager_MakeRequest\",\"interposed\":%s,\"hook_owner_preload\":%s,\"target_owner_preload\":%s}\n",make_bound?"true":"false",make_hook_owned?"true":"false",make_target_owned?"true":"false");
    Dl_info info;
    void *native_update=dlsym(module,"_ZN8GameCore4Game8Instance6UpdateEv");
    bool slots_bound=native_update&&dladdr(native_update,&info);
    const char *leaf=slots_bound?strrchr(info.dli_fname,'/'):NULL;
    if(slots_bound)slots_bound=!strcmp(leaf?leaf+1:info.dli_fname,"libGameCore_XP2.so");
    struct binding {uintptr_t offset;void *wrapper;const char *name;};
    const struct binding bindings[]={
        {0x3b21cc0,(void *)trace_broadcast,"BroadcastMessage"},
        {0x3b290d8,(void *)stop_processor_update,"GameProcessor_Update"},
        {0x3b3a468,(void *)stop_ai_process,"GlobalAi_ProcessAi"},
        {0x3b44778,(void *)trace_receive,"ReceiveMessage"},
        {0x3b7a830,(void *)trace_event_id,"EventSend_identity"},
        {0x3b88880,(void *)trace_queue,"QueueMessage"},
        {0x3b97fa8,(void *)trace_event_id_two_int,"EventSend_identity_two_int"},
        {0x3b9b0e8,(void *)stop_diplomatic_action,"Diplomatic_ProcessAction"},
        {0x3bab2e0,(void *)stop_ai_control,"AIControl_DoTurn"},
        {0x3baf188,(void *)stop_diplomatic_deal,"Diplomatic_ProcessDeal"},
        {0x3bc1560,(void *)stop_pending_statement,"Diplomacy_ProcessStatement"},
        {0x3beb240,(void *)stop_game_update,"Game_Update"},
        {0x3bfdb70,(void *)trace_event_id_int,"EventSend_identity_int"},
        {0x3b4eb38,(void *)trace_event_values,"EventSend_value_value"},
        {0x3b9bbd0,(void *)trace_event_type,"EventSend_type"},
        {0x3b69c98,(void *)trace_event_two_int,"EventSend_two_int"}
    };
    for(size_t i=0;slots_bound&&i<sizeof bindings/sizeof bindings[0];i++){
        void *bound=*(void **)((uintptr_t)info.dli_fbase+bindings[i].offset);
        bool hook_owner=probe_owned(bindings[i].wrapper),target_owner=probe_owned(bound);
        bool match=hook_owner&&target_owner&&bound==bindings[i].wrapper;
        fprintf(trace_log,"{\"event\":\"original_PLT_binding\",\"name\":\"%s\",\"interposed\":%s,\"hook_owner_preload\":%s,\"target_owner_preload\":%s}\n",bindings[i].name,match?"true":"false",hook_owner?"true":"false",target_owner?"true":"false");
        if(!match)slots_bound=false;
    }
    if(!virtual_bound||!make_bound||!slots_bound){fclose(trace_log);trace_log=NULL;return 206;}
    atomic_store(&armed,true);
    fprintf(stderr,"C6_LINUX_ACTION_TRACE armed original_virtual_binding_proven 1\n");
    return 0;
}
int c6_linux_action_trace_end(unsigned expected_messages) {
    if(!atomic_load(&armed)||!trace_log)return 207;
    atomic_store(&armed,false);
    unsigned q=atomic_load(&queued),r=atomic_load(&received),e=atomic_load(&executed),p=atomic_load(&reported);
    fprintf(trace_log,"{\"event\":\"end\",\"queued\":%u,\"received\":%u,\"unit_executed\":%u,\"reports\":%u}\n",q,r,e,p);
    bool failed=fflush(trace_log)||ferror(trace_log);if(fclose(trace_log))failed=true;
    trace_log=NULL;trace_module=NULL;
    fprintf(stderr,"C6_LINUX_ACTION_TRACE complete queued %u received %u unit_executed %u reports %u\n",q,r,e,p);
    return failed?208:(q==r&&e==expected_messages&&q>=e&&(expected_messages||q==0))?0:209;
}
