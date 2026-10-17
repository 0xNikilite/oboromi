#pragma once

#include <QImage>
#include <QWidget>
#include <cstdint>

class DisplayWidget : public QWidget {
    Q_OBJECT

public:
    explicit DisplayWidget(QWidget *parent = nullptr);
    ~DisplayWidget() override;

protected:
    void paintEvent(QPaintEvent *event) override;

private slots:
    void frameReady();

private:
    QImage m_image;
    uint64_t m_generation = 0;
};

// a window the core presents into through vulkan, or a DisplayWidget where the platform has no vulkan
QWidget *createDisplay(QWidget *parent);
