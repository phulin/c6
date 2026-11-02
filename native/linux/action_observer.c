#define _GNU_SOURCE
#include "action_observer.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

/* Exact Linux symbols/ABI, original map factory branch and UnitID hidden
 * return are audited by tools/linux/action_observer_audit.py. Storage for the
 * 8-byte nontrivial UnitID return is filled exclusively by original code. */
int c6_linux_action_observe(void *module, void *context) {
    void *(*get_game)(void *)=dlsym(module,"_ZN8GameCore7Exports7Context7GetGameEv");
    /* Use individual declarations to retain ordinary C function pointer ABI. */
    void (*release)(void *)=dlsym(module,"_ZN8GameCore10Interfaces10ICvUnknowndlEPv");
    int (*local)(const void *)=dlsym(module,"_ZNK8GameCore7Exports7Context14GetLocalPlayerEv");
    void *(*get_player)(void *,int)=dlsym(module,"_ZN8GameCore7Exports7Context9GetPlayerENS_11PlayerTypesE");
    bool (*human)(const void *)=dlsym(module,"_ZNK8GameCore7Exports6Player7IsHumanEv");
    bool (*active)(const void *)=dlsym(module,"_ZNK8GameCore7Exports6Player12IsTurnActiveEv");
    void *(*units)(const void *)=dlsym(module,"_ZNK8GameCore7Exports6Player8GetUnitsEv");
    unsigned (*size)(const void *)=dlsym(module,"_ZNK8GameCore7Exports11PlayerUnits7GetSizeEv");
    void *(*begin)(const void *)=dlsym(module,"_ZNK8GameCore7Exports11PlayerUnits5BeginEv");
    void *(*end)(const void *)=dlsym(module,"_ZNK8GameCore7Exports11PlayerUnits3EndEv");
    void *(*next)(const void *,void *)=dlsym(module,"_ZNK8GameCore7Exports11PlayerUnits4NextEPv");
    void *(*get)(const void *,void *)=dlsym(module,"_ZNK8GameCore7Exports11PlayerUnits3GetEPv");
    unsigned (*id)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Unit5GetIDEv");
    int (*owner)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Unit8GetOwnerEv");
    void *(*component_id)(void *,const void *)=dlsym(module,"_ZNK8GameCore7Exports4Unit14GetComponentIDEv");
    bool (*dead)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Unit6IsDeadEv");
    bool (*delayed)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Unit14IsDelayedDeathEv");
    bool (*selectable)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Unit15IsReadyToSelectEv");
    void *(*operations)(void *)=dlsym(module,"_ZN8GameCore7Exports7Context17GetUnitOperationsEv");
    bool (*can_start)(const void *,const void *,uint32_t,const void *,bool,void *)=dlsym(module,"_ZNK8GameCore7Exports14UnitOperations8CanStartERKNS_6Player11ComponentIDENS_4Unit10Operations5Types8TypeHashERKN4Data15TypedVariantMapEbPSB_");
    void *(*default_manager)(void)=dlsym(RTLD_DEFAULT,"_ZN4Data19TypedVariantManager10GetDefaultEv");
    void *(*create_map)(void *,int)=dlsym(RTLD_DEFAULT,"_ZN4Data19TypedVariantManager32CreateKeyedTypedVariantContainerENS_21VariantContainerTypesE");
    void (*delete_map)(void *)=dlsym(RTLD_DEFAULT,"_ZN4Data15TypedVariantMapD0Ev");
    int (*turn)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game14GetCurrentTurnEv");
    int (*phase)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game15GetCurrentPhaseEv");
    bool (*pending)(void *)=dlsym(module,"_ZN8GameCore7Exports7Context18HasPendingMessagesEv");
    bool (*needs_update)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game11NeedsUpdateEv");
    bool (*needs_advance)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game16NeedsTurnAdvanceEv");
    bool (*ai)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game14IsAiProcessingEv");
    bool (*processing)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game16IsTurnProcessingEv");
    if(!module||!context||!get_game||!release||!local||!get_player||!human||!active||!units||!size||!begin||!end||!next||!get||!id||!owner||!component_id||!dead||!delayed||!selectable||!operations||!can_start||!default_manager||!create_map||!delete_map||!turn||!phase||!pending||!needs_update||!needs_advance||!ai||!processing)return 180;
    void *game=get_game(context);
    if(!game)return 181;
    int result=0,player_id=local(context);
    int before_turn=turn(game),before_phase=phase(game);
    bool before_pending=pending(context),before_update=needs_update(game),before_advance=needs_advance(game),before_ai=ai(game),before_processing=processing(game);
    fprintf(stderr,"C6_LINUX_ACTION_OBSERVER {\"stage\":\"before\",\"local_player\":%d,\"current_turn\":%d,\"current_phase\":%d,\"pending_messages\":%d,\"needs_update\":%d,\"needs_turn_advance\":%d,\"ai_processing\":%d,\"turn_processing\":%d}\n",player_id,before_turn,before_phase,before_pending,before_update,before_advance,before_ai,before_processing);
    if(player_id<0||player_id>=64||before_pending||before_update||before_advance||before_ai||before_processing){release(game);return 182;}
    void *player=get_player(context,player_id);
    if(!player){release(game);return 183;}
    bool is_human=human(player),is_active=active(player);
    void *list=units(player),*ops=operations(context),*params=create_map(default_manager(),2);
    if(!list||!ops||!params){if(params)delete_map(params);if(ops)release(ops);if(list)release(list);release(player);release(game);return 184;}
    unsigned count=size(list),seen=0,legal=0;
    void *stop=end(list),*iterator=begin(list);
    fprintf(stderr,"C6_LINUX_ACTION_OBSERVER {\"stage\":\"player\",\"local_player\":%d,\"human\":%d,\"turn_active\":%d,\"unit_count\":%u,\"parameters\":\"original_factory_empty_map\",\"test_visible\":false,\"results_requested\":false}\n",player_id,is_human,is_active,count);
    while(iterator!=stop){
        if(seen>=count){result=185;break;}
        void *unit=get(list,iterator);
        if(!unit){result=186;break;}
        uint64_t native_identity;
        component_id(&native_identity,unit);
        unsigned unit_id=id(unit);int unit_owner=owner(unit);
        bool is_dead=dead(unit),is_delayed=delayed(unit),is_selectable=selectable(unit);
        bool allowed=can_start(ops,&native_identity,UINT32_C(0x2c681908),params,false,NULL);
        legal+=allowed;
        fprintf(stderr,"C6_LINUX_ACTION_OBSERVER {\"stage\":\"unit\",\"index\":%u,\"owner\":%d,\"id\":%u,\"component_bits\":\"%016llx\",\"dead\":%d,\"delayed_death\":%d,\"ready_to_select\":%d,\"operation\":\"SKIP_TURN\",\"hash\":\"2c681908\",\"can_start\":%d}\n",seen,unit_owner,unit_id,(unsigned long long)native_identity,is_dead,is_delayed,is_selectable,allowed);
        release(unit);seen++;
        void *following=next(list,iterator);
        if(following==iterator){result=187;break;}
        iterator=following;
    }
    if(!result&&seen!=count)result=188;
    delete_map(params);release(ops);release(list);release(player);
    bool after_pending=pending(context),after_update=needs_update(game),after_advance=needs_advance(game),after_ai=ai(game),after_processing=processing(game);
    int after_turn=turn(game),after_phase=phase(game),after_local=local(context);
    release(game);
    if(after_turn!=before_turn||after_phase!=before_phase||after_local!=player_id)result=189;
    if(after_pending!=before_pending||after_update!=before_update||after_advance!=before_advance||after_ai!=before_ai||after_processing!=before_processing)result=189;
    fprintf(stderr,"C6_LINUX_ACTION_OBSERVER {\"stage\":\"after\",\"units_observed\":%u,\"legal_skip_turn\":%u,\"current_turn\":%d,\"current_phase\":%d,\"local_player\":%d,\"pending_messages\":%d,\"needs_update\":%d,\"needs_turn_advance\":%d,\"ai_processing\":%d,\"turn_processing\":%d,\"result\":%d,\"action_executed\":false,\"full_state_equivalence\":false}\n",seen,legal,after_turn,after_phase,after_local,after_pending,after_update,after_advance,after_ai,after_processing,result);
    return result;
}
