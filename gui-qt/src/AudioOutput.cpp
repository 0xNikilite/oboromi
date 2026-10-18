#include "AudioOutput.h"

#include <QAudioFormat>
#include <QAudioSink>
#include <QDebug>
#include <algorithm>
#include <vector>

#include "oboromi_core.h"

namespace {

constexpr qint64 kJitterHeadroomUs = 50000;

class QtSinkBackend : public AudioBackend {
public:
    explicit QtSinkBackend(const AudioFormat &format) {
        QAudioFormat qtFormat;
        qtFormat.setSampleRate(format.rate);
        qtFormat.setChannelCount(format.channels);
        qtFormat.setSampleFormat(QAudioFormat::Int16);
        m_sink = std::make_unique<QAudioSink>(qtFormat);
        QObject::connect(&m_timer, &QTimer::timeout, [this] { tick(); });
    }

    bool start() {
        if (!(m_device = m_sink->start())) return false;
        m_device->write(QByteArray(m_sink->format().bytesForDuration(kJitterHeadroomUs), 0));
        m_timer.start(10);
        return true;
    }

    std::string describe() const override {
        return "qt buffer " + std::to_string(m_sink->bufferSize() / m_sink->format().bytesPerFrame()) + " frames";
    }

private:
    void tick() {
        size_t queued;
        uint64_t underruns;
        oboromi_audio_stats(&queued, &underruns);
        std::vector<int16_t> samples(std::min<size_t>(m_sink->bytesFree() / m_sink->format().bytesPerFrame(), queued) * 2);
        oboromi_audio_pull(samples.data(), samples.size() / 2);
        m_device->write(reinterpret_cast<const char *>(samples.data()), samples.size() * sizeof(int16_t));
    }

    std::unique_ptr<QAudioSink> m_sink;
    QIODevice *m_device = nullptr;
    QTimer m_timer;
};

std::unique_ptr<AudioBackend> openQtSink(const AudioFormat &format) {
    auto backend = std::make_unique<QtSinkBackend>(format);
    if (backend->start()) return backend;
    return nullptr;
}

}

AudioOutput::AudioOutput(QObject *parent) : QObject(parent) {
    oboromi_audio_format(&m_format.rate, &m_format.channels, &m_format.period);
    connect(&m_timer, &QTimer::timeout, this, &AudioOutput::tick);
    m_timer.start(10);
}

AudioOutput::~AudioOutput() = default;

QString AudioOutput::status() const {
    size_t queued;
    uint64_t underruns;
    oboromi_audio_stats(&queued, &underruns);
    return QString("audio %1, requested %2/%3, %4 frames queued, %5 underruns")
        .arg(m_backend ? QString::fromStdString(m_backend->describe()) : "stopped")
        .arg(m_format.period).arg(m_format.rate).arg(queued).arg(underruns);
}

void AudioOutput::tick() {
    if (auto generation = oboromi_audio_generation(); generation != m_generation) {
        m_generation = generation;
        m_lastProduced.start();
        auto choice = qgetenv("OBOROMI_AUDIO");
        std::pair<const char *, std::unique_ptr<AudioBackend> (*)(const AudioFormat &)> backends[] = {
#ifdef OBOROMI_PIPEWIRE
            {"pipewire", openPipeWire},
#endif
#ifdef OBOROMI_ALSA
            {"alsa", openAlsa},
#endif
#ifdef _WIN32
            {"wasapi", openWasapi},
#endif
#ifdef __APPLE__
            {"coreaudio", openCoreAudio},
#endif
            {"qt", openQtSink},
        };
        for (auto &[name, open] : backends)
            if (!m_backend && (choice.isEmpty() || choice == name)) m_backend = open(m_format);
        if (!m_backend) qWarning() << "audio: no output opened";
    }
    if (m_backend && m_lastProduced.hasExpired(1000)) m_backend.reset();
}
