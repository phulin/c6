#ifndef C6_LINUX_PLACEMENT_METADATA_H
#define C6_LINUX_PLACEMENT_METADATA_H
/* Runner-owned admission scope, independent of the frozen domain profile.
 * begin/end retain the genuine loaded Context/Game owners; quiescent must exit
 * immediately on unknown/active work, before facade release or post-state reads.
 * Runner gates exact module/host/dependency/input/rules/config/source identities.
 * This API does not authorize original execution or construct game objects. */
int c6_linux_placement_metadata_admission_begin(void *module, void *context);
int c6_linux_placement_metadata_quiescent(void *module, void *context);
int c6_linux_placement_metadata_admission_end(void *module, void *context);
/* Private exclusive-create NDJSON: C6_PLACEMENT_METADATA_OUTPUT contains genuine
 * input facts; EXPECTED_OUTPUT contains original count answers only. Also require
 * PROFILE_SHA256, SNAPSHOT_BEFORE/CONTROL/AFTER/REPEAT with the same prefix.
 * Partial files/nonzero status are unusable. Complete original stream + matching
 * owner trace supply wire records; this helper supplies complete lookup members,
 * not recycled allocator state. Four saves/two known streams are bounded purity
 * controls, not complete state/RNG evidence. No action, handler or turn support. */
int c6_linux_placement_metadata_probe(void *module, void *context);
#endif
