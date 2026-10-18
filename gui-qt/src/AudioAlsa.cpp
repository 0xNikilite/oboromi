#include <alsa/asoundlib.h>

#include <atomic>
#include <thread>
#include <vector>

#include "AudioBackend.h"
#include "oboromi_core.h"

namespace {

class AlsaBackend : public AudioBackend {
public:
    bool open(const AudioFormat &format) {
        if (snd_pcm_open(&m_pcm, "default", SND_PCM_STREAM_PLAYBACK, 0) < 0) return false;
        auto latencyUs = 2 * format.period * 1000000 / format.rate;
        if (snd_pcm_set_params(m_pcm, SND_PCM_FORMAT_S16_LE, SND_PCM_ACCESS_RW_INTERLEAVED, format.channels, format.rate, 1, latencyUs) < 0) return false;
        snd_pcm_get_params(m_pcm, &m_buffer, &m_period);
        m_thread = std::thread([this, channels = format.channels] {
            std::vector<int16_t> samples(m_period * channels);
            while (m_running) {
                oboromi_audio_pull(samples.data(), m_period);
                if (auto written = snd_pcm_writei(m_pcm, samples.data(), m_period); written < 0) snd_pcm_recover(m_pcm, written, 1);
            }
        });
        return true;
    }

    ~AlsaBackend() override {
        m_running = false;
        if (m_thread.joinable()) m_thread.join();
        if (m_pcm) snd_pcm_close(m_pcm);
    }

    std::string describe() const override {
        return "alsa period " + std::to_string(m_period) + " buffer " + std::to_string(m_buffer) + " frames";
    }

private:
    snd_pcm_t *m_pcm = nullptr;
    snd_pcm_uframes_t m_buffer = 0, m_period = 0;
    std::atomic<bool> m_running = true;
    std::thread m_thread;
};

}

std::unique_ptr<AudioBackend> openAlsa(const AudioFormat &format) {
    auto backend = std::make_unique<AlsaBackend>();
    if (backend->open(format)) return backend;
    return nullptr;
}
