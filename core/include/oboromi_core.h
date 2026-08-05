#pragma once

#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

constexpr static const uintptr_t CORE_COUNT = 8;

constexpr static const uint64_t MEMORY_SIZE = (((12 * 1024) * 1024) * 1024);

constexpr static const uint64_t MEMORY_BASE = 0;

constexpr static const uint32_t SHADER = 1;

constexpr static const uint32_t FLOAT16 = 9;

constexpr static const uint32_t FLOAT64 = 10;

constexpr static const uint32_t INT64 = 11;

constexpr static const uint32_t INT16 = 22;

constexpr static const uint32_t INT8 = 39;

constexpr static const uint32_t STORAGE_BUFFER_16BIT = 4433;

constexpr static const uint32_t STORAGE_BUFFER_8BIT = 4448;

constexpr static const uint32_t PHYSICAL_STORAGE_BUFFER_ADDRESSES = 5348;

constexpr static const uint32_t VARIABLE_POINTERS = 4442;

constexpr static const uint32_t UNIFORM_CONSTANT = 0;

constexpr static const uint32_t INPUT = 1;

constexpr static const uint32_t UNIFORM = 2;

constexpr static const uint32_t OUTPUT = 3;

constexpr static const uint32_t WORKGROUP = 4;

constexpr static const uint32_t CROSS_WORKGROUP = 5;

constexpr static const uint32_t PRIVATE = 6;

constexpr static const uint32_t FUNCTION = 7;

constexpr static const uint32_t PUSH_CONSTANT = 9;

constexpr static const uint32_t STORAGE_BUFFER = 12;

constexpr static const uint32_t PHYSICAL_STORAGE_BUFFER = 5349;

constexpr static const uint32_t VERTEX = 0;

constexpr static const uint32_t FRAGMENT = 4;

constexpr static const uint32_t GLCOMPUTE = 5;

constexpr static const uint32_t LOCAL_SIZE = 17;

constexpr static const uint32_t ORIGIN_UPPER_LEFT = 7;

constexpr static const uint32_t BLOCK = 2;

constexpr static const uint32_t BUFFER_BLOCK = 3;

constexpr static const uint32_t ROW_MAJOR = 4;

constexpr static const uint32_t COL_MAJOR = 5;

constexpr static const uint32_t ARRAY_STRIDE = 6;

constexpr static const uint32_t MATRIX_STRIDE = 7;

constexpr static const uint32_t BUILTIN = 11;

constexpr static const uint32_t NO_PERSPECTIVE = 13;

constexpr static const uint32_t FLAT = 14;

constexpr static const uint32_t NON_WRITABLE = 24;

constexpr static const uint32_t NON_READABLE = 25;

constexpr static const uint32_t LOCATION = 30;

constexpr static const uint32_t BINDING = 33;

constexpr static const uint32_t DESCRIPTOR_SET = 34;

constexpr static const uint32_t OFFSET = 35;

constexpr static const uint32_t ROUND = 1;

constexpr static const uint32_t ROUND_EVEN = 2;

constexpr static const uint32_t TRUNC = 3;

constexpr static const uint32_t F_ABS = 4;

constexpr static const uint32_t S_ABS = 5;

constexpr static const uint32_t F_SIGN = 6;

constexpr static const uint32_t S_SIGN = 7;

constexpr static const uint32_t FLOOR = 8;

constexpr static const uint32_t CEIL = 9;

constexpr static const uint32_t FRACT = 10;

constexpr static const uint32_t SIN = 13;

constexpr static const uint32_t COS = 14;

constexpr static const uint32_t TAN = 15;

constexpr static const uint32_t ASIN = 16;

constexpr static const uint32_t ACOS = 17;

constexpr static const uint32_t ATAN = 18;

constexpr static const uint32_t SINH = 19;

constexpr static const uint32_t COSH = 20;

constexpr static const uint32_t TANH = 21;

constexpr static const uint32_t ATAN2 = 25;

constexpr static const uint32_t POW = 26;

constexpr static const uint32_t EXP = 27;

constexpr static const uint32_t LOG = 28;

constexpr static const uint32_t EXP2 = 29;

constexpr static const uint32_t LOG2 = 30;

constexpr static const uint32_t SQRT = 31;

constexpr static const uint32_t INVERSE_SQRT = 32;

constexpr static const uint32_t F_MIN = 37;

constexpr static const uint32_t U_MIN = 38;

constexpr static const uint32_t S_MIN = 39;

constexpr static const uint32_t F_MAX = 40;

constexpr static const uint32_t U_MAX = 41;

constexpr static const uint32_t S_MAX = 42;

constexpr static const uint32_t F_CLAMP = 43;

constexpr static const uint32_t U_CLAMP = 44;

constexpr static const uint32_t S_CLAMP = 45;

constexpr static const uint32_t F_MIX = 46;

constexpr static const uint32_t FMA = 50;

constexpr static const uint32_t FIND_I_LSB = 73;

constexpr static const uint32_t FIND_S_MSB = 74;

constexpr static const uint32_t FIND_U_MSB = 75;

/// called once per output line. the pointer is only valid during the call.
using LineCallback = void(*)(const char*);

extern "C" {

/// runs the arm64 cpu test suite. blocks the calling thread, so call it from
/// a worker thread on the gui side, never from the ui thread.
void oboromi_run_cpu_tests(LineCallback cb);

/// runs the sm86 gpu decoder/translation test suite. same threading rules
/// as "oboromi_run_cpu_tests"
void oboromi_run_gpu_tests(LineCallback cb);

}  // extern "C"
