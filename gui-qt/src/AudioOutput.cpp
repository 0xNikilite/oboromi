#include "AudioOutput.h"

#include <QAudioFormat>
#include <QAudioSink>
#include <QDebug>
#include <algorithm>
#include <vector>

#include "oboromi_core.h"

namespace {

constexpr qint64 kJitterHeadroomUs = 50000;

}

AudioOutput::AudioOutput(QObject *parent) : QObject(parent) {
    uint32_t rate, channels;
    oboromi_audio_format(&rate, &channels);
    QAudioFormat format;
    format.setSampleRate(rate);
    format.setChannelCount(channels);
    format.setSampleFormat(QAudioFormat::Int16);
    m_sink = new QAudioSink(format, this);
    connect(&m_timer, &QTimer::timeout, this, &AudioOutput::tick);
    m_timer.start(10);
}

void AudioOutput::tick() {
    if (auto generation = oboromi_audio_generation(); generation != m_generation) {
        m_generation = generation;
        m_lastProduced.start();
        if (!m_device) {
            if (!(m_device = m_sink->start())) return void(qWarning() << "audio sink:" << m_sink->error());
            m_device->write(QByteArray(m_sink->format().bytesForDuration(kJitterHeadroomUs), 0));
        }
    }
    if (!m_device) return;
    if (m_lastProduced.hasExpired(1000)) {
        m_sink->stop();
        m_device = nullptr;
        return;
    }
    auto frameBytes = m_sink->format().bytesPerFrame();
    std::vector<int16_t> samples(std::min<uint64_t>(m_sink->bytesFree() / frameBytes, m_generation - m_pulled) * 2);
    m_pulled += oboromi_audio_pull(samples.data(), samples.size() / 2);
    m_device->write(reinterpret_cast<const char *>(samples.data()), samples.size() * sizeof(int16_t));
}
