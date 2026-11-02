CC = x86_64-w64-mingw32-gcc
CFLAGS = -std=c11 -O2 -Wall -Wextra -Werror
OUT = ../../../target/bootstrap/xml_import_probe.exe

.PHONY: all
all: $(OUT)
$(OUT): probe.c imports.c imports.h
	mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -municode -static -Wl,--image-base,0x150000000 -o $@ probe.c imports.c -lbcrypt
