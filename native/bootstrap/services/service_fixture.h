#ifndef C6_BOOTSTRAP_SERVICE_FIXTURE_H
#define C6_BOOTSTRAP_SERVICE_FIXTURE_H
#include <stdio.h>
#include <windows.h>
/* Diagnostic host only. No initialized game, script VM, database or event delivery is supplied. */
void *bootstrap_service_fixture_create(HMODULE gamecore);
/* Real DatabaseDLL connections to explicitly empty named-memory fixtures.
 * Caller must gate DatabaseDLL identity before invoking this function. */
int bootstrap_service_fixture_prepare_empty_databases(HMODULE database);
int bootstrap_service_fixture_execute_sql(int gameplay, const char *sql, int length);
void bootstrap_service_fixture_report(FILE *output);
#endif
