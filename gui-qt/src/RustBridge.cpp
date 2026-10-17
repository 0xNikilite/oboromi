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

bool renderTestFrame() {
    return oboromi_render_test_frame();
}

bool renderDemoFrame(float seconds) {
    return oboromi_render_demo_frame(seconds);
}

bool renderTriangleDemo(float seconds) {
    return oboromi_render_triangle_demo(seconds);
}

bool flushDemoCaches() {
    return oboromi_flush_demo_caches();
}

bool renderBoomBoxDemo(float seconds) {
    return oboromi_render_boombox_demo(seconds);
}

bool renderHelmetDemo(float seconds) {
    return oboromi_render_helmet_demo(seconds);
}

bool renderPostFrame(float seconds) {
    return oboromi_render_post_frame(seconds);
}

bool renderSpritesFrame(float seconds) {
    return oboromi_render_sprites_frame(seconds);
}

bool renderParticlesFrame(float seconds) {
    return oboromi_render_particles_frame(seconds);
}

}
