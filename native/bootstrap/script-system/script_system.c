#include "script_system.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* The existing import binder owns CRT/kernel/DatabaseDLL binding. This bounded
 * extension binds only the original EXE's HavokScript descriptor. */
static int bind_havok(HMODULE executable) {
    unsigned char *base = (void *)executable;
    IMAGE_NT_HEADERS64 *nt = (void *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    IMAGE_IMPORT_DESCRIPTOR *entry = (void *)(base + nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress);
    int count = 0;
    for (; entry->Name; ++entry) {
        if (_stricmp((char *)base + entry->Name, "HavokScript_FinalRelease.dll")) continue;
        HMODULE module = LoadLibraryW(L"HavokScript_FinalRelease.dll");
        if (!module) return 0;
        IMAGE_THUNK_DATA64 *lookup = (void *)(base + entry->OriginalFirstThunk);
        IMAGE_THUNK_DATA64 *iat = (void *)(base + entry->FirstThunk);
        for (; lookup->u1.AddressOfData; ++lookup, ++iat) {
            const char *name = IMAGE_SNAP_BY_ORDINAL64(lookup->u1.Ordinal) ?
                (const char *)(uintptr_t)IMAGE_ORDINAL64(lookup->u1.Ordinal) :
                (char *)((IMAGE_IMPORT_BY_NAME *)(base + lookup->u1.AddressOfData))->Name;
            FARPROC target = GetProcAddress(module, name);
            DWORD protection, ignored;
            if (!target || !VirtualProtect(iat, sizeof *iat, PAGE_READWRITE, &protection)) return 0;
            iat->u1.Function = (uintptr_t)target;
            if (!VirtualProtect(iat, sizeof *iat, protection, &ignored)) return 0;
            ++count;
        }
    }
    return count;
}
void *bootstrap_original_script_system(HMODULE executable) {
    unsigned char *base = (void *)executable;
    if ((uintptr_t)base != UINT64_C(0x140000000)) return NULL;
    /* Refuse pre-existing state: this helper owns only a fresh process path. */
    if (*(void **)(base + 0x2d08f88) || *(void **)(base + 0x22d8140)) return NULL;
    int bound = bind_havok(executable);
    if (!bound) return NULL;
    printf("{\"phase\":\"original_havok_imports_bound\",\"count\":%d}\n", bound);
    ((void *(*)(void *))(base + 0xf607c0))(base + 0x3552fa0);
    puts("{\"phase\":\"original_lua_lock_constructed\"}");
    ((void (*)(void))(base + 0x4d4c70))();
    void *manager = *(void **)(base + 0x2d08f88);
    if (!manager || *(void **)manager != base + 0xfe9cf8 || !((void **)manager)[2]) return NULL;
    puts("{\"phase\":\"original_lua_manager_created\",\"vtable_rva\":\"0xfe9cf8\",\"lua_state_nonnull\":true}");
    void *system = ((void *(*)(void))(base + 0x1c9890))();
    if (!system || *(void **)system != base + 0xfc0618 || system != ((void *(*)(void))(base + 0x1c9880))()) return NULL;
    puts("{\"phase\":\"original_script_system_created\",\"vtable_rva\":\"0xfc0618\",\"getter_identity\":true}");
    return system;
}
