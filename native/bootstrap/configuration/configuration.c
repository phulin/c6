#include "configuration.h"
#include <bcrypt.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "key_initializers.h"
static int save_gate(const unsigned char *data,size_t size) {
    static const char expected[]="a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8";
    BCRYPT_ALG_HANDLE alg=NULL; BCRYPT_HASH_HANDLE hash=NULL;
    unsigned char sum[32];char hex[65];int ok=0;
    if(size!=7348717 || BCryptOpenAlgorithmProvider(&alg,BCRYPT_SHA256_ALGORITHM,NULL,0)<0 ||
       BCryptCreateHash(alg,&hash,NULL,0,NULL,0,0)<0 ||
       BCryptHashData(hash,(unsigned char *)data,(ULONG)size,0)<0 ||
       BCryptFinishHash(hash,sum,sizeof sum,0)<0) goto done;
    for(unsigned i=0;i<32;++i) sprintf(hex+i*2,"%02x",sum[i]);
    ok=!strcmp(hex,expected);
done:
    if(hash) BCryptDestroyHash(hash);
    if(alg) BCryptCloseAlgorithmProvider(alg,0);
    return ok;
}
int bootstrap_saved_configuration_prepare_allocator(HMODULE host) {
    uintptr_t base=(uintptr_t)host;
    if(base!=UINT64_C(0x140000000)) return 0;
    uintptr_t *allocator=(uintptr_t *)(base+0x2347df0);
    if(!*allocator) ((void (*)(void))(base+0xef40))();
    return *allocator==base+0x103c578;
}
int bootstrap_saved_configuration_restore(HMODULE host,HMODULE core,const wchar_t *save) {
    uintptr_t base=(uintptr_t)host,cb=(uintptr_t)core;
    if(base!=UINT64_C(0x140000000) || !core ||
       *(uintptr_t *)(base+0x2347df0)!=base+0x103c578 ||
       *(void **)(base+0x22d7a20) || *(void **)(base+0x22d7b90)) return 0;
    FILE *input=_wfopen(save,L"rb");if(!input) return 0;
    if(fseek(input,0,SEEK_END)) {fclose(input);return 0;}
    long size=ftell(input);rewind(input);
    if(size!=7348717) {fclose(input);return 0;}
    unsigned char *bytes=malloc((size_t)size);
    int read_ok=bytes && fread(bytes,1,(size_t)size,input)==(size_t)size;
    fclose(input);
    if(!read_ok || !save_gate(bytes,(size_t)size)) {free(bytes);return 0;}
    /* Each guarded pure initializer hashes its original literal and publishes
     * only its original key global. The special native loop builds the original per-player keys. */
    for(size_t i=0;i<sizeof configuration_key_initializers/sizeof configuration_key_initializers[0];++i)
        ((void (*)(void))(base+configuration_key_initializers[i]))();
    puts("{\"phase\":\"configuration_pure_keys_initialized\"}"); fflush(stdout);
    ((void (*)(void))(base+0x5a90))();
    puts("{\"phase\":\"configuration_player_keys_initialized\"}"); fflush(stdout);
    void *game=((void *(*)(void))(base+0x100110))();
    puts("{\"phase\":\"original_game_configuration_constructed\"}"); fflush(stdout);
    void *user=((void *(*)(void))(base+0x118000))();
    puts("{\"phase\":\"original_user_configuration_constructed\"}"); fflush(stdout);
    if(!game || !user) {free(bytes);return 0;}
    union {uint64_t align;unsigned char data[0x28];} stream;
    ((void *(*)(void *,void *,uint64_t))(cb+0x9a9ec0))(&stream,bytes+8914,66257-8914);
    ((void (*)(void *,void *))(base+0xff1f0))(&stream,game);
    uint64_t game_consumed=*(uint64_t *)(stream.data+0x10);
    uint64_t result=((uint64_t (*)(void *,void *))(base+0x1190f0))(user,&stream);
    uint64_t consumed=*(uint64_t *)(stream.data+0x10);
    ((void (*)(void *))(cb+0x9a9f80))(&stream);
    uintptr_t *views=(uintptr_t *)((unsigned char *)game+0x2e0);
    size_t count=(views[1]-views[0])/0x40;
    typedef const unsigned char *(*find_fn)(void *,uint32_t);
    void *root=*(void **)game;
    find_fn find=(find_fn)(*(uintptr_t **)root)[0x58/8];
    int player_maps_equal=count==64;
    for(size_t i=0;player_maps_equal && i<count;++i) {
        uint32_t key=*(uint32_t *)(base+0x22d6880+i*0x44);
        const unsigned char *variant=find(root,key);
        uintptr_t view=views[0]+i*0x40;
        player_maps_equal=variant && *(uint16_t *)variant==10 &&
            *(void **)(variant+8)==*(void **)(view+0x18) && *(uint32_t *)view==i;
    }
    const unsigned char *game_variant=find(root,*(uint32_t *)(base+0x22d6708));
    const unsigned char *map_variant=find(root,*(uint32_t *)(base+0x22d6838));
    int root_views_equal=game_variant && map_variant &&
        *(uint16_t *)game_variant==10 && *(uint16_t *)map_variant==10 &&
        *(void **)(game_variant+8)==*(void **)((unsigned char *)game+0x88) &&
        *(void **)(map_variant+8)==*(void **)((unsigned char *)game+0xb0);
    uintptr_t head=(uintptr_t)user+0x78,node=*(uintptr_t *)head;
    size_t user_players=0;uint32_t first_player=UINT32_MAX;
    while(node!=head && user_players<65) {
        if(!user_players) first_player=*(uint32_t *)(node+0x10);
        ++user_players;node=*(uintptr_t *)node;
    }
    int ok=game_consumed==65937-8914 && result==0 && consumed==66257-8914 &&
        player_maps_equal && root_views_equal && user_players==1 && first_player==0;
    printf("{\"phase\":\"original_configuration_view_identity\",\"player_maps_equal\":%s,\"game_map_views_equal\":%s,\"user_player_count\":%zu,\"first_user_player\":%u}\n",player_maps_equal?"true":"false",root_views_equal?"true":"false",user_players,first_player);
    printf("{\"phase\":\"original_saved_configuration_restored\",\"game_bytes\":%llu,\"total_bytes\":%llu,\"user_read_result\":%llu,\"player_views\":%zu,\"native_game_singleton\":true,\"native_user_singleton\":true,\"gameplay_state_loaded\":false,\"invariants_equal\":%s}\n",(unsigned long long)game_consumed,(unsigned long long)consumed,(unsigned long long)result,count,ok?"true":"false");
    free(bytes);return ok;
}
