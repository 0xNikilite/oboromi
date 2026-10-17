#pragma once

#include <string>
#include <vector>

namespace RustBridge {

// Runs the ARM64 CPU test suite
std::vector<std::string> runCpuTests();

// Runs the SM86 GPU decoder/translation test suite
std::vector<std::string> runGpuTests();

// Renders a GPU gradient into the display, false if Vulkan is unavailable
bool renderTestFrame();

// Renders the Julia set demo at the given time into the display
bool renderDemoFrame(float seconds);

}
