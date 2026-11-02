#ifndef C6_LINUX_OPERATION_REGISTRY_H
#define C6_LINUX_OPERATION_REGISTRY_H
/* Exact target/configuration gate and initialized loaded game required.
 * Caller synchronizes native work and brackets this query with full Core Saves.
 * All original objects are borrowed; this observer owns no engine facade. */
int c6_linux_operation_registry(void *module, void *context);
#endif
