#pragma once

#include <QElapsedTimer>
#include <QTimer>
#include <cstdint>

class QAudioSink;
class QIODevice;

class AudioOutput : public QObject {
public:
    explicit AudioOutput(QObject *parent = nullptr);

private:
    void tick();

    QAudioSink *m_sink = nullptr;
    QIODevice *m_device = nullptr;
    QTimer m_timer;
    QElapsedTimer m_lastProduced;
    uint64_t m_generation = 0;
    uint64_t m_pulled = 0;
};
