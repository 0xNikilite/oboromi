#pragma once

#include <QElapsedTimer>
#include <QString>
#include <QTimer>
#include <cstdint>
#include <memory>

#include "AudioBackend.h"

class AudioOutput : public QObject {
public:
    explicit AudioOutput(QObject *parent = nullptr);
    ~AudioOutput() override;
    QString status() const;

private:
    void tick();

    AudioFormat m_format{};
    std::unique_ptr<AudioBackend> m_backend;
    QTimer m_timer;
    QElapsedTimer m_lastProduced;
    uint64_t m_generation = 0;
};
