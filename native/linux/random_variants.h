#ifndef C6_LINUX_RANDOM_VARIANTS_H
#define C6_LINUX_RANDOM_VARIANTS_H
/* Isolated fixture only: no gameplay context or stream lifecycle. */
#include <stdint.h>
typedef struct { uint32_t seed; } C6ScopedRandom;
typedef struct { uint64_t state, increment; } C6RandomPCG32;
_Static_assert(sizeof(C6ScopedRandom) == 4, "ScopedRandom ABI");
_Static_assert(sizeof(C6RandomPCG32) == 16, "RandomPCG32 ABI");
#endif
