CC = x86_64-w64-mingw32-gcc
CFLAGS = -std=c11 -O2 -Wall -Wextra -Werror
OUT = ../../target/bootstrap/context_host.exe

.PHONY: all clean
all: $(OUT)
$(OUT): context_host.c services/service_fixture.c services/service_fixture.h save/save_probe.c save/save_probe.h xml-import/imports.c xml-import/imports.h xml-import/xml.c xml-import/xml.h xml-import/content.c xml-import/content.h localization/localization.c localization/localization.h database-backup/backup.c database-backup/backup.h script-system/script_system.c script-system/script_system.h app-provider/app_provider.c app-provider/app_provider.h job-manager/job_manager.c job-manager/job_manager.h
	mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -municode -static -Wl,--image-base,0x150000000 -o $@ context_host.c services/service_fixture.c save/save_probe.c xml-import/imports.c xml-import/xml.c xml-import/content.c localization/localization.c database-backup/backup.c script-system/script_system.c app-provider/app_provider.c job-manager/job_manager.c -lbcrypt
clean:
	rm -f $(OUT)
