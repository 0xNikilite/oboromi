#include <stdint.h>

extern "C" __global__ void mul_add(const float* a, const float* b, float* c, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) c[i] = a[i] * b[i] + i;
}

#define WIDTH 1920u
#define HEIGHT 1080u

extern "C" __global__ void gradient(uint32_t* frame) {
    uint32_t x = blockIdx.x * blockDim.x + threadIdx.x, y = blockIdx.y * blockDim.y + threadIdx.y;
    if (x < WIDTH && y < HEIGHT) frame[y * WIDTH + x] = 0xff800000 | y * 255 / HEIGHT << 8 | x * 255 / WIDTH;
}
