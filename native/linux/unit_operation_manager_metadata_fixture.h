#ifndef C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_FIXTURE_H
#define C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_FIXTURE_H
/* Independent synchronous raw Manager metadata scope. Unknown/live state terminates
 * the isolated process before observations or teardown. Caller retains the
 * genuine module/context and clears admission before native Uninitialize. */
int c6_linux_unit_operation_manager_metadata_admission_begin(void *module,void *borrowed_context);
int c6_linux_unit_operation_manager_metadata_quiescent(void *module,void *borrowed_context);
int c6_linux_unit_operation_manager_metadata_admission_end(void *module,void *borrowed_context);
int c6_linux_unit_operation_manager_metadata_fixture_probe(void *module,void *borrowed_context);
#endif
