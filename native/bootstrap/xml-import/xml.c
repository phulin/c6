#include "xml.h"
#include <stdint.h>
#include <string.h>
typedef void *(*Constructor)(void *, void *);
typedef void *(*XmlConstructor)(void *, void *, unsigned char);
typedef int (*Parse)(void *, const char *, char *, int, void *);
typedef void (*Destructor)(void *);
void bootstrap_original_xml_construct(struct bootstrap_original_xml *self,
                                      HMODULE executable, void *connection) {
    self->executable = executable;
    memset(self->storage, 0, sizeof self->storage);
    ((Constructor)((unsigned char *)executable + 0x799910))(self->storage + 8, connection);
    ((XmlConstructor)((unsigned char *)executable + 0x799d80))(self->storage + 0x50, connection, 1);
}
int bootstrap_original_xml_parse(struct bootstrap_original_xml *self,
                                const char *filename, char *utf8) {
    return ((Parse)((unsigned char *)self->executable + 0x79d3a0))(
        self->storage + 0x50, filename, utf8, -1, self->storage + 8);
}
void bootstrap_original_xml_destroy(struct bootstrap_original_xml *self) {
    ((Destructor)((unsigned char *)self->executable + 0x799e30))(self->storage + 0x50);
    ((Destructor)((unsigned char *)self->executable + 0x799930))(self->storage + 8);
}
int bootstrap_original_make_hash(HMODULE executable, void *connection) {
    typedef void *(*Allocate)(size_t);
    typedef void *(*ScalarConstructor)(void *);
    typedef int (*RegisterFunction)(void *, const char *, int, unsigned char, void *);
    void *callback = ((Allocate)((unsigned char *)executable + 0x982060))(0x410);
    if (!callback) return -1;
    ((ScalarConstructor)((unsigned char *)executable + 0xbd050))(callback);
    int status = ((RegisterFunction)(*(uintptr_t **)connection)[30])(
        connection, "Make_Hash", 1, 1, callback);
    /* Native registration owns callback on success. sqlite_create_function_v2
     * also invokes its deletion callback on failure; do not double-free. */
    return status;
}
