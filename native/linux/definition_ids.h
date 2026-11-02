#ifndef C6_LINUX_DEFINITION_IDS_H
#define C6_LINUX_DEFINITION_IDS_H

/* Call only inside the hash/configuration-gated native runner after definitions
 * were built, between canonical Save captures. This never initializes a cache. */
int c6_linux_definition_ids(void *module, void *context);

#endif
