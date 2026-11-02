#include "imports.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* Bind only genuine CRT, Win32 kernel and original DatabaseDLL imports. These
 * providers receive normal Windows initialization; no original EXE entry/TLS/
 * global constructor is invoked. All other IAT slots retain loader image bytes
 * and any accidental use is a failed diagnostic, never fabricated success. */
int bootstrap_bind_original_imports(HMODULE original) {
    unsigned char *base = (unsigned char *)original;
    IMAGE_NT_HEADERS64 *nt = (void *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    IMAGE_IMPORT_DESCRIPTOR *entry = (void *)(base + nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress);
    unsigned bound = 0;
    for (; entry->Name; ++entry) {
        const char *name = (char *)base + entry->Name;
        int allowed = !_stricmp(name, "MSVCP140.dll") || !_stricmp(name, "VCRUNTIME140.dll") ||
                      !_stricmp(name, "KERNEL32.dll") || !_stricmp(name, "DatabaseDLL_FinalRelease.dll") ||
                      !_strnicmp(name, "api-ms-win-crt-", 15);
        if (!allowed) continue;
        /* CRT API-set contracts resolve to the genuine supplied UCRT provider.
         * Wine does not expose each API-set name to explicit LoadLibraryA. */
        HMODULE module = LoadLibraryA(!_strnicmp(name, "api-ms-win-crt-", 15) ? "ucrtbase.dll" : name);
        if (!module) { fprintf(stderr, "provider load failed: %s %lu\n", name, GetLastError()); return 0; }
        IMAGE_THUNK_DATA64 *lookup = (void *)(base + entry->OriginalFirstThunk);
        IMAGE_THUNK_DATA64 *iat = (void *)(base + entry->FirstThunk);
        for (; lookup->u1.AddressOfData; ++lookup, ++iat) {
            const char *symbol = IMAGE_SNAP_BY_ORDINAL64(lookup->u1.Ordinal) ?
                (const char *)(uintptr_t)IMAGE_ORDINAL64(lookup->u1.Ordinal) :
                (char *)((IMAGE_IMPORT_BY_NAME *)(base + lookup->u1.AddressOfData))->Name;
            FARPROC target = GetProcAddress(module, symbol);
            DWORD protection;
            if (!target || !VirtualProtect(iat, sizeof *iat, PAGE_READWRITE, &protection)) return 0;
            iat->u1.Function = (uintptr_t)target;
            DWORD unused;
            if (!VirtualProtect(iat, sizeof *iat, protection, &unused)) return 0;
            ++bound;
        }
    }
    return (int)bound;
}
