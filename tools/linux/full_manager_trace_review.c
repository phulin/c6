/* Observational end-policy and hard-stop checks; no originals are loaded. */
#include "action_execution_trace.c"
#include <assert.h>
#include <limits.h>
#include <sys/wait.h>

static FILE *start(unsigned q,unsigned r,unsigned e,unsigned p) {
    trace_log=tmpfile();assert(trace_log);atomic_store(&armed,true);
    atomic_store(&queued,q);atomic_store(&received,r);atomic_store(&executed,e);atomic_store(&reported,p);
    return trace_log;
}
int main(void) {
    atomic_store(&armed,false);trace_log=NULL;
    assert(c6_linux_action_trace_end_observational()==207);
    atomic_store(&armed,true);assert(c6_linux_action_trace_end_observational()==207);
    FILE *orphan=tmpfile();assert(orphan);trace_log=orphan;atomic_store(&armed,false);
    assert(c6_linux_action_trace_end_observational()==207);assert(!fclose(orphan));trace_log=NULL;
    const unsigned rows[][4]={{0,0,0,0},{2,0,0,5},{1,2,3,7},{UINT_MAX,1,UINT_MAX,UINT_MAX}};
    for(unsigned i=0;i<sizeof rows/sizeof rows[0];i++) {
        FILE *out=start(rows[i][0],rows[i][1],rows[i][2],rows[i][3]);int fd=dup(fileno(out));assert(fd>=0);
        assert(c6_linux_action_trace_end_observational()==0);
        assert(!atomic_load(&armed)&&!trace_log&&!trace_module);
        assert(atomic_load(&queued)==rows[i][0]&&atomic_load(&received)==rows[i][1]&&
               atomic_load(&executed)==rows[i][2]&&atomic_load(&reported)==rows[i][3]);
        FILE *readback=fdopen(fd,"r");assert(readback);rewind(readback);char text[2048]={0};
        assert(fread(text,1,sizeof text-1,readback)>0);
        assert(strstr(text,"observe_callback_effects")&&strstr(text,"\"expected_zero\":false")&&
               strstr(text,"\"all_policies_observed\":false"));
        char expected[256];snprintf(expected,sizeof expected,"\"queued\":%u,\"received\":%u,\"unit_executed\":%u,\"reports\":%u",rows[i][0],rows[i][1],rows[i][2],rows[i][3]);
        assert(strstr(text,expected));assert(!fclose(readback));
    }
    start(1,1,1,4);assert(c6_linux_action_trace_end(1)==0);
    start(2,1,3,4);assert(c6_linux_action_trace_end(0)==209);
    trace_log=fopen("/dev/full","w");assert(trace_log);atomic_store(&armed,true);
    assert(c6_linux_action_trace_end_observational()==208);
    for(unsigned i=0;i<4;i++) {
        FILE *error=tmpfile(),*log=tmpfile();assert(error&&log);pid_t child=fork();assert(child>=0);
        if(!child) {
            assert(dup2(fileno(error),STDERR_FILENO)>=0);trace_log=log;atomic_store(&armed,i%2==0);
            if(i<2)stop_game_update(NULL);else (void)stop_diplomatic_deal(NULL,0,NULL,false);
            _exit(1);
        }
        int status;assert(waitpid(child,&status,0)==child&&WIFEXITED(status)&&WEXITSTATUS(status)==198);
        rewind(error);char text[1024]={0};assert(fread(text,1,sizeof text-1,error)>0);
        assert(strstr(text,"action_acceptance 0")&&strstr(text,"failing_boundary"));
        rewind(log);memset(text,0,sizeof text);assert(fread(text,1,sizeof text-1,log)>0&&strstr(text,"failing_boundary"));
        assert(!fclose(error)&&!fclose(log));
    }
    puts("14 independent trace structural/IO/counter/legacy/hard-stop cases passed");
    return 0;
}
