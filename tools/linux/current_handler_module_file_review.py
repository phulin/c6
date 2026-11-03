#!/usr/bin/env python3
"""Focused synthetic identity-race regressions; never loads/executes game code."""
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
HARNESS = r'''
#define _GNU_SOURCE
#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/mman.h>
#include <fcntl.h>
static char *review_realpath(const char *,char *);
static ssize_t review_pread(int,void *,size_t,off_t);
#define realpath review_realpath
#define pread review_pread
#include "HELPER"
#undef realpath
#undef pread
static unsigned step;
static char canonical[4096],keep[4096],replacement[4096];
static char *review_realpath(const char *p,char *out){
 char *result=realpath(p,out);
 if(result&&step==1){step=0;assert(!unlink(canonical));assert(!symlink(keep,canonical));}
 return result;
}
static ssize_t review_pread(int fd,void *out,size_t n,off_t offset){
 if(step==3){
  step=0;int writer=open(canonical,O_WRONLY|O_CLOEXEC);assert(writer>=0);
  assert(pwrite(writer,"xyz",3,0)==3);assert(!close(writer));
 }
 ssize_t result=pread(fd,out,n,offset);
 if(result>0&&step==2){step=0;assert(!rename(replacement,canonical));}
 return result;
}
static void make(const char *path){
 int fd=open(path,O_WRONLY|O_CREAT|O_EXCL|O_CLOEXEC,0600);assert(fd>=0);
 assert(write(fd,"abc",3)==3);assert(!close(fd));
}
int main(int argc,char **argv){
 assert(argc==2);
 int n=snprintf(canonical,sizeof canonical,"%s/canonical",argv[1]);assert(n>0&&(size_t)n<sizeof canonical);
 n=snprintf(keep,sizeof keep,"%s/keep",argv[1]);assert(n>0&&(size_t)n<sizeof keep);
 n=snprintf(replacement,sizeof replacement,"%s/replacement",argv[1]);assert(n>0&&(size_t)n<sizeof replacement);
 const char *expected="ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
 make(canonical);make(keep);make(replacement);ModuleFile file={.fd=-1};
 step=1;const char *reason=module_file_open(&file,canonical,expected);
 assert(reason&&!strcmp(reason,"module_canonical_open"));module_file_close(&file);
 assert(file.fd==-1&&!file.loader_path&&!file.canonical_path);
 assert(!unlink(canonical));make(canonical);
 step=2;reason=module_file_open(&file,canonical,expected);
 assert(reason&&!strcmp(reason,"module_identity_changed_during_hash"));module_file_close(&file);
 step=3;reason=module_file_open(&file,canonical,expected);
 assert(reason&&!strcmp(reason,"module_sha256_mismatch"));module_file_close(&file);
 assert(!unlink(canonical));make(canonical);
 assert(!module_file_open(&file,canonical,expected));
 void *mapped=mmap(NULL,4096,PROT_READ|PROT_EXEC,MAP_PRIVATE,file.fd,0);assert(mapped!=MAP_FAILED);
 Image image={.fd=file.fd,.count=1,.segments={{.lo=(uintptr_t)mapped,.hi=(uintptr_t)mapped+3,.offset=1,.flags=PF_R|PF_X}}};
 assert(!runtime_code_mapping(&image,(uintptr_t)mapped,3));
 image.segments[0].offset=0;assert(runtime_code_mapping(&image,(uintptr_t)mapped,3));
 assert(!munmap(mapped,4096));module_file_close(&file);
 puts("review_identity_cases=4");return 0;
}
'''


def main():
    with tempfile.TemporaryDirectory(prefix='c6-module-identity-review-') as temporary:
        work = Path(temporary)
        source = work/'test.c'
        source.write_text(HARNESS.replace('HELPER', str(ROOT/'native/linux/current_handler_interruption.c')))
        executable = work/'test'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-Wconversion',
                        '-Wshadow', '-O2', '-fsanitize=undefined', str(source), '-ldl',
                        '-o', str(executable)], check=True)
        result = subprocess.run([str(executable), str(work)], check=True, capture_output=True, text=True)
        assert result.stdout.strip() == 'review_identity_cases=4'
    print(json.dumps(dict(independent_identity_cases=4, ubsan=True, original_execution=False,
        cases=['canonical replaced by symlink after realpath rejects before open',
               'same-hash inode replacement during hash rejects at endpoint',
               'opened-fd changed before first read fails actual hash',
               'correct inode/RX with wrong file offset refuses mapping'])))


if __name__ == '__main__':
    main()
