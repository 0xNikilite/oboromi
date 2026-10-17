#include <stdint.h>

extern "C" __global__ void guest(uint64_t *a, uint64_t *b, uint64_t *c, uint64_t *d) {
    __shared__ uint8_t shared[4096];
    unsigned i = threadIdx.x;
    shared[i] = i;
    if (i == 0)
        a[0] = (uintptr_t)b ^ (uintptr_t)c ^ (uintptr_t)d ^ shared[0];
}
