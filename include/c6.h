#ifndef C6_H
#define C6_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* ABI v1: platform native C calling convention, pointer width and size_t.
 * All calls on a handle require exclusive external synchronization. Handles
 * belong to the creating library instance; destroy them before unloading it.
 * Invalid nonnull pointers, double frees and aliasing violate the contract.
 * Outputs remain unchanged on failure except export's required byte count.
 */
typedef struct C6Handle C6Handle;
enum { C6_OK=0, C6_INVALID=1, C6_UNSUPPORTED=2, C6_BUFFER_TOO_SMALL=3 };
enum { C6_RANDOM_DRAW=1, C6_RANDOM_RESET=2, C6_RANDOM_INITIALIZE=3, C6_RANDOM_LOCK=4, C6_RANDOM_UNLOCK=5,
       C6_END_TURN=100 /* explicitly unsupported */ };
typedef struct { uint32_t kind, argument; } C6Action;
typedef struct { uint32_t seed, calls, resets, lock_count, synchronous; } C6RandomState;
uint32_t c6_abi_version(void);
uint32_t c6_plot_valid_v1(int32_t x, int32_t y);
int32_t c6_plot_distance_v1(int32_t width, uint32_t wrap, int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t *result);
int32_t c6_create_v1(uint32_t seed, C6Handle **out);
void c6_destroy_v1(C6Handle *handle);
/* DRAW argument is 0..65535; larger values are invalid, not truncated. */
int32_t c6_apply_v1(C6Handle *handle, const C6Action *action, uint32_t *result);
int32_t c6_random_state_v1(const C6Handle *handle, C6RandomState *out);
/* NULL buffer queries size and returns C6_BUFFER_TOO_SMALL. */
int32_t c6_export_v1(const C6Handle *handle, uint8_t *buffer, size_t capacity, size_t *written);
int32_t c6_import_v1(const uint8_t *bytes, size_t length, C6Handle **out);
#ifdef __cplusplus
}
#endif
#endif
