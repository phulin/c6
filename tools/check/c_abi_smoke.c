/* Independent C consumer of the released shared-library ABI. */
#include "c6.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

_Static_assert(sizeof(C6Action) == 8, "C action layout");
_Static_assert(sizeof(C6RandomState) == 20, "C random state layout");

int main(void) {
    C6Handle *a = NULL, *b = NULL;
    uint32_t result = 0xfeed;
    C6RandomState before, after;
    assert(c6_abi_version() == 1);
    assert(c6_create_v1(1, NULL) == C6_INVALID);
    assert(c6_create_v1(1, &a) == C6_OK && a);
    assert(c6_random_state_v1(NULL, &before) == C6_INVALID);
    assert(c6_random_state_v1(a, NULL) == C6_INVALID);
    assert(c6_random_state_v1(a, &before) == C6_OK);
    C6Action draw = {C6_RANDOM_DRAW, 65535};
    assert(c6_apply_v1(NULL, &draw, &result) == C6_INVALID);
    assert(c6_apply_v1(a, NULL, &result) == C6_INVALID);
    assert(c6_apply_v1(a, &draw, NULL) == C6_INVALID);
    C6Action invalid = {C6_RANDOM_DRAW, 65536};
    assert(c6_apply_v1(a, &invalid, &result) == C6_INVALID);
    C6Action unsupported = {C6_END_TURN, 0};
    assert(c6_apply_v1(a, &unsupported, &result) == C6_UNSUPPORTED);
    assert(result == 0xfeed);
    assert(c6_random_state_v1(a, &after) == C6_OK);
    assert(memcmp(&before, &after, sizeof(before)) == 0);
    assert(c6_apply_v1(a, &draw, &result) == C6_OK && result == 16837);
    assert(c6_random_state_v1(a, &after) == C6_OK);
    assert(after.seed == 1103527590 && after.calls == 1);

    size_t size = 0, written = 0;
    assert(c6_export_v1(a, NULL, 0, &size) == C6_BUFFER_TOO_SMALL);
    assert(size == 97); /* v1 envelope with no sections */
    unsigned char *bytes = malloc(size + 23), *copy = malloc(size + 23);
    assert(bytes && copy);
    memset(bytes, 0xa5, size + 23);
    assert(c6_export_v1(a, bytes, size - 1, &written) == C6_BUFFER_TOO_SMALL);
    assert(written == size && bytes[0] == 0xa5 && bytes[size - 1] == 0xa5);
    assert(c6_export_v1(a, bytes, size, &written) == C6_OK && written == size);
    assert(memcmp(bytes, "C6SN\1\0\0\0", 8) == 0);
    /* A bound identity and future opaque section must survive C import/export.
       section-count at 89; u64 lengths; name 'future'; opaque payload 0xff. */
    memset(bytes + 8, 0x42, 64);
    bytes[89] = 1;
    memset(bytes + size, 0, 23);
    bytes[size] = 6;
    memcpy(bytes + size + 8, "future", 6);
    bytes[size + 14] = 1;
    bytes[size + 22] = 0xff;
    size += 23;
    assert(c6_import_v1(bytes, size, &b) == C6_OK && b);
    assert(c6_export_v1(b, copy, size, &written) == C6_OK && written == size);
    assert(memcmp(bytes, copy, size) == 0);
    /* Failed decode must preserve the caller's existing pointer. */
    C6Handle *sentinel = b;
    assert(c6_import_v1(bytes, size - 1, &sentinel) == C6_INVALID && sentinel == b);
    bytes[4] = 2;
    assert(c6_import_v1(bytes, size, &sentinel) == C6_INVALID && sentinel == b);
    bytes[4] = 1;
    assert(c6_import_v1(NULL, size, &sentinel) == C6_INVALID && sentinel == b);
    assert(c6_import_v1(bytes, size, NULL) == C6_INVALID);
    assert(c6_apply_v1(b, &draw, &result) == C6_OK && result == 38525);
    assert(c6_export_v1(b, copy, size, &written) == C6_OK);
    assert(memcmp(bytes + 8, copy + 8, 64) == 0);
    assert(memcmp(bytes + 97, copy + 97, size - 97) == 0);
    c6_destroy_v1(a);
    c6_destroy_v1(b);
    c6_destroy_v1(NULL);
    free(bytes);
    free(copy);
    puts("PASS: compiled C shared-library caller; error atomicity, bound identity, opaque snapshot roundtrip and branching");
    return 0;
}
