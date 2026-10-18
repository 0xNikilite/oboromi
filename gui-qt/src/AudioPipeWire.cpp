#include <pipewire/pipewire.h>
#include <spa/param/audio/format-utils.h>

#include <algorithm>
#include <atomic>

#include "AudioBackend.h"
#include "oboromi_core.h"

namespace {

class PipeWireBackend : public AudioBackend {
public:
    bool open(const AudioFormat &format) {
        pw_init(nullptr, nullptr);
        if (!(m_loop = pw_thread_loop_new("oboromi-audio", nullptr))) return false;
        auto *props = pw_properties_new(PW_KEY_MEDIA_TYPE, "Audio", PW_KEY_MEDIA_CATEGORY, "Playback", PW_KEY_MEDIA_ROLE, "Game", nullptr);
        pw_properties_setf(props, PW_KEY_NODE_LATENCY, "%u/%u", format.period, format.rate);
        m_stream = pw_stream_new_simple(pw_thread_loop_get_loop(m_loop), "oboromi", props, &kEvents, this);
        if (!m_stream) return false;
        uint8_t pod[1024];
        auto builder = SPA_POD_BUILDER_INIT(pod, sizeof(pod));
        spa_audio_info_raw info{};
        info.format = SPA_AUDIO_FORMAT_S16;
        info.rate = format.rate;
        info.channels = format.channels;
        info.position[0] = SPA_AUDIO_CHANNEL_FL;
        info.position[1] = SPA_AUDIO_CHANNEL_FR;
        const spa_pod *params[] = {spa_format_audio_raw_build(&builder, SPA_PARAM_EnumFormat, &info)};
        auto flags = pw_stream_flags(PW_STREAM_FLAG_AUTOCONNECT | PW_STREAM_FLAG_MAP_BUFFERS | PW_STREAM_FLAG_RT_PROCESS);
        if (pw_stream_connect(m_stream, PW_DIRECTION_OUTPUT, PW_ID_ANY, flags, params, 1) < 0) return false;
        m_stride = format.channels * sizeof(int16_t);
        return pw_thread_loop_start(m_loop) == 0;
    }

    ~PipeWireBackend() override {
        if (m_loop) pw_thread_loop_stop(m_loop);
        if (m_stream) pw_stream_destroy(m_stream);
        if (m_loop) pw_thread_loop_destroy(m_loop);
    }

    std::string describe() const override {
        return "pipewire quantum " + std::to_string(m_quantum.load()) + " frames";
    }

private:
    static void process(void *self) {
        static_cast<PipeWireBackend *>(self)->process();
    }

    void process() {
        auto *buffer = pw_stream_dequeue_buffer(m_stream);
        if (!buffer) return;
        auto &data = buffer->buffer->datas[0];
        uint32_t frames = std::min<uint64_t>(buffer->requested, data.maxsize / m_stride);
        oboromi_audio_pull(static_cast<int16_t *>(data.data), frames);
        data.chunk->offset = 0;
        data.chunk->stride = m_stride;
        data.chunk->size = frames * m_stride;
        m_quantum = frames;
        pw_stream_queue_buffer(m_stream, buffer);
    }

    static constexpr pw_stream_events kEvents = {.version = PW_VERSION_STREAM_EVENTS, .process = process};

    pw_thread_loop *m_loop = nullptr;
    pw_stream *m_stream = nullptr;
    uint32_t m_stride = 0;
    std::atomic<uint32_t> m_quantum = 0;
};

}

std::unique_ptr<AudioBackend> openPipeWire(const AudioFormat &format) {
    auto backend = std::make_unique<PipeWireBackend>();
    if (backend->open(format)) return backend;
    return nullptr;
}
