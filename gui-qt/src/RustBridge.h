#pragma once

#include <string>
#include <vector>

namespace RustBridge {

// Runs the ARM64 CPU test suite
std::vector<std::string> runCpuTests();

// Runs the SM86 GPU decoder/translation test suite
std::vector<std::string> runGpuTests();

}
