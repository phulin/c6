/* Original Windows MemoryIStream and Context::Load over a private, fingerprinted
 * extracted stream. This is not a complete Civ6 container loader. Native reads
 * retain original short-read and bulk-buffer behavior. Call only in an isolated
 * diagnostic child after genuine StartupGame succeeds. */
#include "save_probe.h"
#include <bcrypt.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define STREAM_SIZE 76556938u
static const char stream_sha[] = "a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa";
/* Layout recovered from the original constructor, GetOffset/GetSize, and
 * nondeleting destructor. Bytes are borrowed: allocator==NULL, so the original
 * destructor does not free the caller buffer. No synthetic bulk results. */
typedef struct {
    void **vtable;
    unsigned char *bytes;
    uint64_t position, size;
    void *allocator;
} NativeMemoryIStream;
static NativeMemoryIStream *active_stream;
void bootstrap_save_probe_report_fault(FILE *output) {
    if (!active_stream) return;
    fprintf(output, "{\"phase\":\"save_stream_at_fault\",\"original_memory_istream\":true,\"consumed\":%llu,\"available\":%llu,\"initialized_game\":false}\n",
            (unsigned long long)active_stream->position,
            (unsigned long long)active_stream->size);
    fflush(output);
}
_Static_assert(sizeof(NativeMemoryIStream) == 0x28, "original Windows MemoryIStream size");
static int verify_stream_contract(HMODULE image, const NativeMemoryIStream *stream,
                                  const unsigned char *bytes, uint64_t size) {
    static const uintptr_t expected[12] = {
        0x9a9fe0, 0x61c8d0, 0x9aa090, 0x9aa080, 0x9aa2d0, 0x9aa140,
        0x9aa250, 0x9aa0e0, 0x8972d0, 0x8972d0, 0x59eda0, 0x9b1040
    };
    if (sizeof(*stream) != 0x28 || stream->bytes != bytes || stream->position ||
        stream->size != size || stream->allocator ||
        (uintptr_t)stream->vtable - (uintptr_t)image != 0xabfec0) return 0;
    for (unsigned i = 0; i < 12; ++i)
        if ((uintptr_t)stream->vtable[i] - (uintptr_t)image != expected[i]) return 0;
    return 1;
}
static int digest_matches(const unsigned char *bytes, ULONG size) {
    BCRYPT_ALG_HANDLE algorithm = NULL;
    BCRYPT_HASH_HANDLE hash = NULL;
    unsigned char digest[32];
    char hex[65];
    int ok = 0;
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, NULL, 0) < 0 ||
        BCryptCreateHash(algorithm, &hash, NULL, 0, NULL, 0, 0) < 0 ||
        BCryptHashData(hash, (PUCHAR)bytes, size, 0) < 0 ||
        BCryptFinishHash(hash, digest, sizeof digest, 0) < 0) goto done;
    for (unsigned i = 0; i < sizeof digest; ++i) sprintf(hex + 2 * i, "%02x", digest[i]);
    ok = strcmp(hex, stream_sha) == 0;
done:
    if (hash) BCryptDestroyHash(hash);
    if (algorithm) BCryptCloseAlgorithmProvider(algorithm, 0);
    return ok;
}
int bootstrap_save_probe(HMODULE image, void *context, const wchar_t *path) {
    FILE *file = _wfopen(path, L"rb");
    if (!file) return 80;
    if (fseek(file, 0, SEEK_END) || ftell(file) != STREAM_SIZE || fseek(file, 0, SEEK_SET)) {
        fclose(file); return 81;
    }
    unsigned char *bytes = malloc(STREAM_SIZE);
    if (!bytes) { fclose(file); return 82; }
    int read_ok = fread(bytes, 1, STREAM_SIZE, file) == STREAM_SIZE;
    fclose(file);
    if (!read_ok || !digest_matches(bytes, STREAM_SIZE)) { free(bytes); return 83; }
    void **context_vtable = *(void ***)context;
    if ((uintptr_t)context_vtable - (uintptr_t)image != 0xa101e8 ||
        (uintptr_t)context_vtable[0x170 / 8] - (uintptr_t)image != 0x5906e0) {
        free(bytes); return 84;
    }
    NativeMemoryIStream stream;
    memset(&stream, 0xa5, sizeof stream);
    ((void *(*)(void *, const void *, uint64_t))((uintptr_t)image + 0x9a9ec0))(
        &stream, bytes, STREAM_SIZE);
    if (!verify_stream_contract(image, &stream, bytes, STREAM_SIZE)) {
        free(bytes); return 86;
    }
    puts("{\"phase\":\"save_stream_begin\",\"original_memory_istream\":true,\"borrowed_buffer\":true,\"available\":76556938,\"initialized_game\":false,\"gameplay_parity\":false}");
    fflush(stdout);
    active_stream = &stream;
    int result = ((int (*)(void *, void *))context_vtable[0x170 / 8])(context, &stream);
    uint64_t consumed = ((uint64_t (*)(void *))stream.vtable[0x18 / 8])(&stream);
    printf("{\"phase\":\"save_context_Load_result\",\"original_result\":%d,\"consumed\":%llu,\"available\":%u,\"fully_consumed\":%s,\"original_memory_istream\":true,\"initialized_game\":false,\"gameplay_parity\":false}\n",
           result, (unsigned long long)consumed, STREAM_SIZE,
           consumed == STREAM_SIZE ? "true" : "false");
    active_stream = NULL;
    ((void (*)(void *))((uintptr_t)image + 0x9a9f80))(&stream);
    free(bytes);
    /* Diagnostic may record original zero, but cannot assert whole-game success. */
    return result == 0 && consumed == STREAM_SIZE ? 0 : 85;
}
