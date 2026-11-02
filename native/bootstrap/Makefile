CC = x86_64-w64-mingw32-gcc
CFLAGS = -std=c11 -O2 -Wall -Wextra -Werror
OUT = ../../target/bootstrap/context_host.exe

.PHONY: all clean
all: $(OUT)
$(OUT): context_host.c services/service_fixture.c services/service_fixture.h save/save_probe.c save/save_probe.h
	mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -municode -static -Wl,--image-base,0x150000000 -o $@ context_host.c services/service_fixture.c save/save_probe.c -lbcrypt
clean:
	rm -f $(OUT)
