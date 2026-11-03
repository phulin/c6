#ifndef C6_LINUX_PLACEMENT_METADATA_FIXTURE_H
#define C6_LINUX_PLACEMENT_METADATA_FIXTURE_H
/* Independent synchronous placement-query scope. Unknown/live state terminates
 * the isolated process before observations or teardown. Caller retains the
 * genuine module/context and clears admission before native Uninitialize. */
int c6_linux_placement_metadata_admission_begin(void *module,void *borrowed_context);
int c6_linux_placement_metadata_quiescent(void *module,void *borrowed_context);
int c6_linux_placement_metadata_admission_end(void *module,void *borrowed_context);
#endif
