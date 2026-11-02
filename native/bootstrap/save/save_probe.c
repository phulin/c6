/* A strict synthetic Serialization::IStream adapter, not a Civ6 save loader.
 * Real Windows XP2 context Load reads a private, fingerprinted extracted stream.
 * Only established raw-read/completion slots are implemented; all others abort.
 * Call only in an isolated diagnostic child after genuine StartupGame succeeds.
 */
#include "save_probe.h"
#include <bcrypt.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define STREAM_SIZE 76556938u
static const char stream_sha[] = "a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa";
typedef struct {
    void **vtable;
    unsigned char *bytes;
    uint64_t size, position, reads, completions;
} ProbeStream;
static void report(ProbeStream *stream, const char *phase) {
    printf("{\"phase\":\"%s\",\"reads\":%llu,\"completions\":%llu,\"consumed\":%llu,\"available\":%llu,\"initialized_game\":false,\"gameplay_parity\":false}\n",
           phase, (unsigned long long)stream->reads, (unsigned long long)stream->completions,
           (unsigned long long)stream->position, (unsigned long long)stream->size);
    fflush(stdout);
}
static void unsupported(ProbeStream *stream) {
    report(stream, "save_stream_unsupported_slot");
    ExitProcess(88);
}
static void raw_read(ProbeStream *stream, const uint64_t *request, void *destination, uint64_t size) {
    ++stream->reads;
    if (stream->reads <= 64 || stream->reads % 100000 == 0) {
        printf("{\"phase\":\"save_stream_read\",\"read\":%llu,\"offset\":%llu,\"size\":%llu}\n",
               (unsigned long long)stream->reads, (unsigned long long)stream->position,
               (unsigned long long)size);
        fflush(stdout);
    }
    /* Reject unsupported absolute/asynchronous/short reads without partial copy. */
    if (!request || request[0] != UINT64_MAX || request[1] != size ||
        request[2] != 16 || request[3] || request[4] ||
        size > stream->size - stream->position || (!destination && size)) {
        report(stream, "save_stream_refused_read");
        ExitProcess(89);
    }
    if (size) memcpy(destination, stream->bytes + stream->position, (size_t)size);
    stream->position += size;
}
static void complete(ProbeStream *stream) { ++stream->completions; }
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
    void *vtable[256];
    for (unsigned i = 0; i < 256; ++i) vtable[i] = (void *)(uintptr_t)unsupported;
    vtable[0x30 / 8] = (void *)(uintptr_t)raw_read;
    vtable[0x40 / 8] = (void *)(uintptr_t)complete;
    ProbeStream stream = {vtable, bytes, STREAM_SIZE, 0, 0, 0};
    report(&stream, "save_stream_begin");
    int result = ((int (*)(void *, void *))context_vtable[0x170 / 8])(context, &stream);
    report(&stream, "save_context_Load_returned");
    printf("{\"phase\":\"save_context_Load_result\",\"original_result\":%d,\"fully_consumed\":%s,\"initialized_game\":false,\"gameplay_parity\":false}\n",
           result, stream.position == stream.size ? "true" : "false");
    free(bytes);
    /* Diagnostic may record original zero, but cannot assert whole-game success. */
    return result == 0 && stream.position == stream.size ? 0 : 85;
}
