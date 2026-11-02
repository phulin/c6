#define _GNU_SOURCE
#include "combat_kernel.h"
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Matching target DWARF: FixedPointT<8> stores a signed 32-bit raw value.
 * This is a real parameter record initialized by its original constructor;
 * no Unit, Map, Combat receiver or GlobalParameters objects are synthesized. */
typedef struct {
    int32_t attacker, defender, defender_max_hp, outer_damage, outer_max_hp;
    uint32_t defense_type;
    int32_t power_scaling, minimum_damage, base_damage, power_dampener, max_extra_damage;
    uint8_t use_average, bypass_defenses, padding[2];
} DamageParameters;
_Static_assert(sizeof(DamageParameters) == 48, "DamageParameters target ABI");

/* All transitive module-owned PLT dependencies used in average mode and
 * parameter construction, plus the two global storage relocations. */
static const struct { uintptr_t slot, target; } core_bindings[] = {
    {0x3b80560, 0x1fe4cb0}, {0x3bd7888, 0x2be20d0},
    {0x3bce2d8, 0x1fcc8b0}, {0x3bb5520, 0x1fe5230},
    {0x3ba8508, 0x1fe4aa0}, {0x3b56ec8, 0x1fceb20},
    {0x3bc5598, 0x1fe2170}, {0x3b58d58, 0x1fe2b30},
    {0x3b22818, 0x1fe55e0}, {0x3b3a6c8, 0x1fcdc00},
    {0x3b91a28, 0x1fccdb0}, {0x3b4ab60, 0x1fceb60},
    {0x3bc06b8, 0x1fcdc10}, {0x3b20fc8, 0x3c051b0},
    {0x3b0ef18, 0x3c051a8},
};
/* Measured genuine host interposition, independently audited against XP2's
 * arithmetic. ET_EXEC addresses are absolute; no bindings are overwritten. */
static const struct { uintptr_t target, host_va; } host_weak_bindings[] = {
    {0x1fcc8b0, 0x32ff470}, {0x1fceb60, 0x321e840},
    {0x1fcdc00, 0x32170a0}, {0x1fcdc10, 0x32ff550},
    {0x1fe2170, 0x3242ee0}, {0x1fe2b30, 0x32ff460},
};

static void floating_environment(FILE *out, const char *label, int (*rounding)(void)) {
    uint32_t mxcsr;
    uint16_t x87_control;
    __asm__ volatile ("stmxcsr %0" : "=m" (mxcsr));
    __asm__ volatile ("fnstcw %0" : "=m" (x87_control));
    fprintf(out, "%s %d %u %u\n", label, rounding(), mxcsr, x87_control);
}

static void *required_symbol(void *handle, const char *name, const char *scope) {
    dlerror();
    void *address = dlsym(handle, name);
    const char *error = dlerror();
    if (error || !address) {
        fprintf(stderr, "C6_LINUX_COMBAT_KERNEL symbol_unavailable scope %s name %s error %s\n",
            scope, name, error ? error : "null address");
        return NULL;
    }
    return address;
}

int c6_linux_combat_kernel_probe(void *game) {
    const char *input = getenv("C6_COMBAT_KERNEL_INPUT");
    const char *output = getenv("C6_COMBAT_KERNEL_OUTPUT");
    if (!input || !output) return 180;
    void (*construct)(void *) = required_symbol(game, "_ZN8GameCore5Rules6Combat8Instance16DamageParametersC2Ev", "xp2");
    int (*calculate)(void *, const void *) = required_symbol(game, "_ZN8GameCore5Rules6Combat8Instance15CalculateDamageERKNS2_16DamageParametersE", "xp2");
    void *(*parameters)(void) = required_symbol(game, "_ZN8GameCore7Context7Globals13GetParametersEv", "xp2");
    void *(*current_game)(void) = required_symbol(game, "_ZN8GameCore7Context7Globals7GetGameEv", "xp2");
    int (*max_extra)(const void *) = required_symbol(game, "_ZNK8GameCore4Game16GlobalParameters23COMBAT_MAX_EXTRA_DAMAGEEv", "xp2");
    /* fegetround is a read-only host observer, not a GameCore import. GameCore
     * has no DT_NEEDED graph exposing the host's libm through handle lookup. */
    int (*rounding)(void) = required_symbol(RTLD_DEFAULT, "fegetround", "host");
    Dl_info module, libm, floor_module;
    if (!construct || !calculate || !parameters || !current_game || !max_extra
        || !rounding) return 181;
    if (!dladdr(calculate, &module) || !module.dli_fbase) {
        fprintf(stderr, "C6_LINUX_COMBAT_KERNEL calculate_module_unavailable\n");
        return 181;
    }
    const char *leaf = module.dli_fname ? strrchr(module.dli_fname, '/') : NULL;
    if (!module.dli_fname || strcmp(leaf ? leaf+1 : module.dli_fname, "libGameCore_XP2.so")) return 187;
    uintptr_t base = (uintptr_t)module.dli_fbase;
    if ((uintptr_t)calculate != base+0x2bd8980 || (uintptr_t)construct != base+0x2be20e0
        || (uintptr_t)parameters != base+0x1fe4cb0 || (uintptr_t)current_game != base+0x1fcc320
        || (uintptr_t)max_extra != base+0x2be20d0) return 187;
    unsigned host_count = 0, xp2_count = 0;
    for (size_t i=0; i<sizeof core_bindings/sizeof core_bindings[0]; ++i) {
        uintptr_t expected = base+core_bindings[i].target;
        int host_binding = 0;
        for (size_t j=0; j<sizeof host_weak_bindings/sizeof host_weak_bindings[0]; ++j) {
            if (core_bindings[i].target == host_weak_bindings[j].target) {
                expected = host_weak_bindings[j].host_va;
                host_binding = 1;
            }
        }
        uintptr_t actual = *(uintptr_t *)(base+core_bindings[i].slot);
        if (actual != expected) {
            fprintf(stderr, "C6_LINUX_COMBAT_KERNEL interposed_core_binding %zu\n", i);
            return 187;
        }
        if (host_binding) {
            Dl_info owner;
            if (!dladdr((void *)actual, &owner) || !owner.dli_fname
                || (uintptr_t)owner.dli_fbase != 0x400000) return 187;
            const char *name = strrchr(owner.dli_fname, '/');
            if (strcmp(name ? name+1 : owner.dli_fname, "Civ6")) return 187;
            ++host_count;
        } else ++xp2_count;
    }
    fprintf(stderr, "C6_LINUX_COMBAT_KERNEL core_bindings xp2 %u host %u direct_entries 5\n", xp2_count, host_count);
    /* Read the actual eager-resolved GOT bindings reached by the original
     * helper's PLT calls, not a separately resolved expf discovery symbol.
     * Both slot identities and PLT jumps are checked in the target audit. */
    void *exponential = *(void **)((unsigned char *)module.dli_fbase + 0x3bed160);
    void *floor_function = *(void **)((unsigned char *)module.dli_fbase + 0x3bb4398);
    if (!dladdr(exponential, &libm) || !libm.dli_fname
        || !dladdr(floor_function, &floor_module) || libm.dli_fbase != floor_module.dli_fbase
        || libm.dli_fbase == module.dli_fbase) {
        fprintf(stderr, "C6_LINUX_COMBAT_KERNEL original_math_module_unavailable\n");
        return 181;
    }
    void *rules = parameters(), *instance = current_game();
    if (!rules || !instance) return 182;
    FILE *in = fopen(input, "r"), *out = fopen(output, "wx");
    if (!in || !out) { if (in) fclose(in); if (out) fclose(out); return 183; }
    /* Actual game owns this original Random; snapshot all 32 bytes, including
     * vptr/padding. DWARF: m_kSyncRandom wrapper +232, contained value +248. */
    const unsigned char *random = (const unsigned char *)instance + 248;
    unsigned char before_random[32];
    memcpy(before_random, random, sizeof before_random);
    int extra = max_extra(rules), rc = 0;
    unsigned count = 0;
    fprintf(out, "max_extra_damage %d\n", extra);
    fprintf(out, "libm_path %s\n", libm.dli_fname);
    fprintf(out, "libm_binding_offsets %zu %zu\n",
        (size_t)((uintptr_t)exponential - (uintptr_t)libm.dli_fbase),
        (size_t)((uintptr_t)floor_function - (uintptr_t)libm.dli_fbase));
    floating_environment(out, "floating_environment_before", rounding);
    for (;;) {
        DamageParameters p;
        memset(&p, 0xa5, sizeof p);
        construct(&p);
        unsigned bypass;
        int fields = fscanf(in, "%d %d %d %d %d %u %d %d %d %d %d %u",
            &p.attacker, &p.defender, &p.defender_max_hp, &p.outer_damage,
            &p.outer_max_hp, &p.defense_type, &p.power_scaling, &p.minimum_damage,
            &p.base_damage, &p.power_dampener, &p.max_extra_damage, &bypass);
        if (fields == EOF) break;
        if (fields != 12 || bypass > 1) { rc = 184; break; }
        p.use_average = 1;
        p.bypass_defenses = (uint8_t)bypass;
        DamageParameters before = p;
        /* Original instructions never access the receiver. Null deliberately
         * detects future unproven receiver use instead of inventing an object. */
        int damage = calculate(NULL, &p);
        if (memcmp(&before, &p, sizeof p) || memcmp(before_random, random, sizeof before_random)
            || max_extra(rules) != extra || parameters() != rules || current_game() != instance) {
            rc = 185; break;
        }
        fprintf(out, "%u %d\n", count++, damage);
    }
    floating_environment(out, "floating_environment_after", rounding);
    if (ferror(in) || ferror(out)) rc = 186;
    if (fclose(in)) rc = 186;
    if (fclose(out)) rc = 186;
    fprintf(stderr, "C6_LINUX_COMBAT_KERNEL average_cases %u random_unchanged %d parameter_unchanged %d result %d\n", count, rc == 0, rc == 0, rc);
    return rc;
}
