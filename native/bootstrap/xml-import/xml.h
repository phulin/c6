#ifndef C6_BOOTSTRAP_ORIGINAL_XML_H
#define C6_BOOTSTRAP_ORIGINAL_XML_H
#include "imports.h"
/* Real original constructors initialize this storage. Not simulator state.
 * Exact-image and genuine-provider gates are the caller's responsibility. */
struct bootstrap_original_xml {
    HMODULE executable;
    _Alignas(16) unsigned char storage[0x320];
};
void bootstrap_original_xml_construct(struct bootstrap_original_xml *self,
                                      HMODULE executable, void *connection);
int bootstrap_original_xml_parse(struct bootstrap_original_xml *self,
                                const char *filename, char *utf8);
void bootstrap_original_xml_destroy(struct bootstrap_original_xml *self);
int bootstrap_original_make_hash(HMODULE executable, void *connection);
#endif
