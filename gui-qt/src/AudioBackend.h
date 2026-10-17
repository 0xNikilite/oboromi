#pragma once

#include <cstdint>
#include <memory>
#include <string>

// a host output stream that pulls the core ring from its own callback until destroyed
struct AudioBackend {
    virtual ~AudioBackend() = default;
    virtual std::string describe() const = 0;
};

struct AudioFormat {
    uint32_t rate, channels, period;
};

std::unique_ptr<AudioBackend> openPipeWire(const AudioFormat &format);
std::unique_ptr<AudioBackend> openAlsa(const AudioFormat &format);
std::unique_ptr<AudioBackend> openWasapi(const AudioFormat &format);
std::unique_ptr<AudioBackend> openCoreAudio(const AudioFormat &format);
