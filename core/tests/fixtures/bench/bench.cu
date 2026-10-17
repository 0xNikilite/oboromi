// The perf example's kernels: fma_chain, stream_add and empty each have a GLSL twin <kernel>.comp doing the same work.
extern "C" __global__ void fma_chain(float* out, int iterations, float m, float c) {
    float a[8];
    for (int k = 0; k < 8; k++) a[k] = threadIdx.x + k;
    for (int i = 0; i < iterations; i++)
#pragma unroll
        for (int j = 0; j < 32; j++)
#pragma unroll
            for (int k = 0; k < 8; k++) a[k] = fmaf(a[k], m, c);
    out[blockIdx.x * blockDim.x + threadIdx.x] = a[0] + a[1] + a[2] + a[3] + a[4] + a[5] + a[6] + a[7];
}

extern "C" __global__ void stream_add(const float4* in, float4* out, float add) {
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    float4 v = in[i];
    out[i] = make_float4(v.x + add, v.y + add, v.z + add, v.w + add);
}

extern "C" __global__ void empty(int*) {}

extern "C" __global__ void mandelbrot(uchar4* dst, int width, int height, int crunch, float xOff, float yOff, float scale) {
    int ix = blockIdx.x * blockDim.x + threadIdx.x, iy = blockIdx.y * blockDim.y + threadIdx.y;
    if (ix >= width || iy >= height) return;
    float xC = ix * scale + xOff, yC = iy * scale + yOff, x = 0, y = 0, xx = 0, yy = 0;
    int i = crunch;
    while (--i && xx + yy < 4.0f) {
        y = x * y * 2.0f + yC;
        x = xx - yy + xC;
        yy = y * y;
        xx = x * x;
    }
    int m = i > 0 ? crunch - i : 0;
    dst[width * iy + ix] = make_uchar4(m * 3, m * 5, m * 7, 0);
}
