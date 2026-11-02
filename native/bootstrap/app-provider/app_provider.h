#ifndef C6_BOOTSTRAP_APP_PROVIDER_H
#define C6_BOOTSTRAP_APP_PROVIDER_H
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
/* Requires fingerprinted original EXE at preferred base and genuine bound
 * imports. Calls original allocator and complete original Civ6App constructor.
 * Publishes the constructed object at the verified native singleton location.
 * Does not call original full AppInit or synthesize application fields. */
void *bootstrap_original_app_construct(HMODULE executable);
/* Native wrapper ctor and URI connection factory; explicit fixture install
 * into the already constructed app. Does not run schema/content bootstrap. */
int bootstrap_original_app_gameplay_database(HMODULE executable, void *app, void *expected_gameplay_connection);
#endif
