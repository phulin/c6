#ifndef C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_H
#define C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_H
/* Preparation only: runner must SHA/source/static-audit gate this new profile,
 * retain genuine loaded Context/Game/configuration/root-map owners, and bracket
 * it with complete saves and known RNG checks. No admission predicates, scripts,
 * handlers, requests, actions or turn advancement are executed. The separate
 * c6_linux_unit_operation_manager_metadata_quiescent guard is required; unknown
 * work exits immediately before further observation or cleanup. Output is raw
 * JSONL facts at exclusive C6_UNIT_OPERATION_MANAGER_METADATA_OUTPUT. Unsupported
 * returns nonzero and never substitutes a zero/false observation. Partial output
 * on failure is not an accepted capture. All native pointers are borrowed. */
int c6_linux_unit_operation_manager_metadata_probe(void *module,void *borrowed_context);
#endif
