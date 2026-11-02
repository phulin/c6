#ifndef C6_BOOTSTRAP_SERVICE_FIXTURE_H
#define C6_BOOTSTRAP_SERVICE_FIXTURE_H
#include <stdio.h>
#include <windows.h>
/* Bounded diagnostic host. Real provider setup is explicit; gameplay parity is unproven. */
void *bootstrap_service_fixture_create(HMODULE gamecore);
/* Real DatabaseDLL connections to explicitly empty named-memory fixtures.
 * Caller must gate DatabaseDLL identity before invoking this function. */
/* Explicit original EXE import binding + original archive ctor. EXE/provider SHA gates required. */
int bootstrap_service_fixture_prepare_original_host_services(HMODULE original_host);
int bootstrap_service_fixture_prepare_empty_databases(HMODULE database);
/* Original InitializeDatabase returns true even when prepared writer queries fail.
 * Caller owns authentic phase transaction and must verify affected SQL state. */
int bootstrap_original_context_initialize_database(void *context, void *connection);
void *bootstrap_service_fixture_database_connection(int gameplay);
int bootstrap_service_fixture_execute_sql(int gameplay, const char *sql, int length);
/* Nonmutating child-progress report, invoked before explicit unsupported exits. */
void bootstrap_service_fixture_set_progress_reporter(void (*reporter)(FILE *));
void bootstrap_service_fixture_report(FILE *output);
#endif
