/* Run after cargo build -p c6-ffi --release:
 * cc -Iinclude crates/c6-ffi/tests/load.c -ldl -o /tmp/c6-load
 * /tmp/c6-load target/release/libc6_ffi.so
 */
#include "c6.h"
#include <assert.h>
#include <dlfcn.h>
#include <stdio.h>
int main(int argc, char **argv) {
    assert(argc == 2);
    void *library=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);
    if (!library) { fprintf(stderr,"%s\n",dlerror()); return 1; }
    uint32_t (*version)(void)=dlsym(library,"c6_abi_version");
    int32_t (*create)(uint32_t,C6Handle**)=dlsym(library,"c6_create_v1");
    int32_t (*apply)(C6Handle*,const C6Action*,uint32_t*)=dlsym(library,"c6_apply_v1");
    void (*destroy)(C6Handle*)=dlsym(library,"c6_destroy_v1");
    assert(version && create && apply && destroy && version()==1);
    C6Handle *h=NULL; assert(create(1,&h)==C6_OK);
    uint32_t out=0; C6Action draw={C6_RANDOM_DRAW,65535};
    assert(apply(h,&draw,&out)==C6_OK && out==16837);
    C6Action end_turn={C6_END_TURN,0};assert(apply(h,&end_turn,&out)==C6_UNSUPPORTED);
    destroy(h); assert(dlclose(library)==0);
    puts("C ABI dynamic-load smoke passed");
}
