CC = x86_64-w64-mingw32-gcc
CFLAGS = -std=c11 -O2 -Wall -Wextra -Werror
OUT = ../../../target/bootstrap/localization_probe.exe
all: $(OUT)
$(OUT): probe.c localization.c localization.h ../xml-import/xml.c ../xml-import/imports.c
	mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -municode -static -Wl,--image-base,0x150000000 -o $@ probe.c localization.c ../xml-import/xml.c ../xml-import/imports.c -lbcrypt
