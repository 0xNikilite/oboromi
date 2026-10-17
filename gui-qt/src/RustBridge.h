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

// Draws the pushbuffer triangle demo at the given time into the display
bool renderTriangleDemo(float seconds);

// Draws the sprites demo at the given time into the display
bool renderSpritesFrame(float seconds);

// Draws the pbr BoomBox and FlightHelmet demos at the given time into the display
bool renderBoomBoxDemo(float seconds);
bool renderHelmetDemo(float seconds);

// Draws the post-effect demo at the given time into the display
bool renderPostFrame(float seconds);

}
