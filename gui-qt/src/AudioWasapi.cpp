#include <audioclient.h>
#include <mmdeviceapi.h>
#include <windows.h>

#include <atomic>
#include <thread>

#include "AudioBackend.h"
#include "oboromi_core.h"

namespace {

template <typename T> void release(T *&com) {
    if (com) com->Release();
    com = nullptr;
}

class WasapiBackend : public AudioBackend {
public:
    bool open(const AudioFormat &format) {
        CoInitializeEx(nullptr, COINIT_MULTITHREADED);
        IMMDeviceEnumerator *enumerator = nullptr;
        IMMDevice *device = nullptr;
        if (FAILED(CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL, IID_PPV_ARGS(&enumerator)))) return false;
        auto found = SUCCEEDED(enumerator->GetDefaultAudioEndpoint(eRender, eConsole, &device)) &&
                     SUCCEEDED(device->Activate(__uuidof(IAudioClient3), CLSCTX_ALL, nullptr, reinterpret_cast<void **>(&m_client)));
        release(device);
        release(enumerator);
        if (!found) return false;
        WAVEFORMATEX wave{WAVE_FORMAT_PCM, WORD(format.channels), format.rate, format.rate * format.channels * 2, WORD(format.channels * 2), 16, 0};
        UINT32 defaultPeriod, fundamentalPeriod, maxPeriod;
        if (FAILED(m_client->GetSharedModeEnginePeriod(&wave, &defaultPeriod, &fundamentalPeriod, &m_period, &maxPeriod))) return false;
        if (FAILED(m_client->InitializeSharedAudioStream(AUDCLNT_STREAMFLAGS_EVENTCALLBACK, m_period, &wave, nullptr))) return false;
        m_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        UINT32 bufferFrames;
        if (FAILED(m_client->SetEventHandle(m_event)) || FAILED(m_client->GetBufferSize(&bufferFrames)) ||
            FAILED(m_client->GetService(IID_PPV_ARGS(&m_render))) || FAILED(m_client->Start()))
            return false;
        m_thread = std::thread([this, bufferFrames] {
            while (m_running) {
                WaitForSingleObject(m_event, 100);
                UINT32 padding;
                BYTE *data;
                if (FAILED(m_client->GetCurrentPadding(&padding))) continue;
                if (FAILED(m_render->GetBuffer(bufferFrames - padding, &data))) continue;
                oboromi_audio_pull(reinterpret_cast<int16_t *>(data), bufferFrames - padding);
                m_render->ReleaseBuffer(bufferFrames - padding, 0);
            }
        });
        return true;
    }

    ~WasapiBackend() override {
        m_running = false;
        if (m_thread.joinable()) m_thread.join();
        if (m_client) m_client->Stop();
        release(m_render);
        release(m_client);
        if (m_event) CloseHandle(m_event);
    }

    std::string describe() const override {
        return "wasapi period " + std::to_string(m_period) + " frames";
    }

private:
    IAudioClient3 *m_client = nullptr;
    IAudioRenderClient *m_render = nullptr;
    HANDLE m_event = nullptr;
    UINT32 m_period = 0;
    std::atomic<bool> m_running = true;
    std::thread m_thread;
};

}

std::unique_ptr<AudioBackend> openWasapi(const AudioFormat &format) {
    auto backend = std::make_unique<WasapiBackend>();
    if (backend->open(format)) return backend;
    return nullptr;
}
