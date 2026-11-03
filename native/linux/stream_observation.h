#ifndef C6_LINUX_STREAM_OBSERVATION_H
#define C6_LINUX_STREAM_OBSERVATION_H
#include <stdint.h>
/* Caller-owned passive instrumentation, not a native game service. All fields
 * describe an actual constructor-created stream; pointers remain borrowed.
 * release events retire attribution before destruction, destroyed events never
 * authorize dereferencing the recorded pointer. No native getter is invoked. */
typedef struct {
    uint64_t schema,event,occurrence,context,stream,buffer,size,path,result,offset;
} C6StreamObservation;
void c6_linux_stream_observation_marker(const C6StreamObservation *event);
#endif
