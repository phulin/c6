/* Original QueryInterface ABI bridge. Requires original gated XP2 image already mapped. */
#include <stdint.h>
uintptr_t c6_context_query_interface(uintptr_t self,const uint8_t guid[16]){
 return ((uintptr_t(__attribute__((ms_abi))*)(uintptr_t,const uint8_t*))(uintptr_t)0x1805907a0ULL)(self,guid);
}
