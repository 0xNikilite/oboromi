#include <AudioToolbox/AudioToolbox.h>

#include "AudioBackend.h"
#include "oboromi_core.h"

namespace {

OSStatus render(void *, AudioUnitRenderActionFlags *, const AudioTimeStamp *, UInt32, UInt32 frames, AudioBufferList *buffers) {
    oboromi_audio_pull(static_cast<int16_t *>(buffers->mBuffers[0].mData), frames);
    return noErr;
}

class CoreAudioBackend : public AudioBackend {
public:
    bool open(const AudioFormat &format) {
        AudioComponentDescription description{kAudioUnitType_Output, kAudioUnitSubType_DefaultOutput, kAudioUnitManufacturer_Apple, 0, 0};
        auto component = AudioComponentFindNext(nullptr, &description);
        if (!component || AudioComponentInstanceNew(component, &m_unit) != noErr) return false;
        UInt32 frameBytes = format.channels * sizeof(int16_t);
        AudioStreamBasicDescription stream{double(format.rate), kAudioFormatLinearPCM,
                                           kLinearPCMFormatFlagIsSignedInteger | kLinearPCMFormatFlagIsPacked,
                                           frameBytes, 1, frameBytes, format.channels, 16, 0};
        AURenderCallbackStruct callback{render, nullptr};
        m_frames = format.period;
        UInt32 size = sizeof(m_frames);
        return AudioUnitSetProperty(m_unit, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Input, 0, &stream, sizeof(stream)) == noErr &&
               AudioUnitSetProperty(m_unit, kAudioUnitProperty_SetRenderCallback, kAudioUnitScope_Input, 0, &callback, sizeof(callback)) == noErr &&
               AudioUnitSetProperty(m_unit, kAudioDevicePropertyBufferFrameSize, kAudioUnitScope_Global, 0, &m_frames, size) == noErr &&
               AudioUnitGetProperty(m_unit, kAudioDevicePropertyBufferFrameSize, kAudioUnitScope_Global, 0, &m_frames, &size) == noErr &&
               AudioUnitInitialize(m_unit) == noErr && AudioOutputUnitStart(m_unit) == noErr;
    }

    ~CoreAudioBackend() override {
        if (!m_unit) return;
        AudioOutputUnitStop(m_unit);
        AudioUnitUninitialize(m_unit);
        AudioComponentInstanceDispose(m_unit);
    }

    std::string describe() const override {
        return "coreaudio buffer " + std::to_string(m_frames) + " frames";
    }

private:
    AudioUnit m_unit = nullptr;
    UInt32 m_frames = 0;
};

}

std::unique_ptr<AudioBackend> openCoreAudio(const AudioFormat &format) {
    auto backend = std::make_unique<CoreAudioBackend>();
    if (backend->open(format)) return backend;
    return nullptr;
}
