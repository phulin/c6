CC ?= cc
CFLAGS ?= -O2 -g -Wall -Wextra -Werror
all: build/libc6_oracle.so
build/libc6_oracle.so: image_host.c
	mkdir -p build
	$(CC) $(CFLAGS) -DC6_SOURCE_SHA256=\"$(shell sha256sum image_host.c | cut -d\  -f1)\" -fPIC -shared -o $@ $<
clean:
	rm -rf build
.PHONY: all clean
