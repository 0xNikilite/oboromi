#include "RustBridge.h"

#include "oboromi_core.h"

namespace {

thread_local std::vector<std::string> g_collected;

extern "C" void collectLine(const char *line) {
    if (line != nullptr) {
        g_collected.emplace_back(line);
    }
}

}

namespace RustBridge {

std::vector<std::string> runCpuTests() {
    g_collected.clear();
    oboromi_run_cpu_tests(&collectLine);
    return std::move(g_collected);
}

std::vector<std::string> runGpuTests() {
    g_collected.clear();
    oboromi_run_gpu_tests(&collectLine);
    return std::move(g_collected);
}

}
