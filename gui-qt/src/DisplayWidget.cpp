#include "DisplayWidget.h"

#include <QPainter>

#include "oboromi_core.h"

namespace {

// runs on the emulator thread, so it may only post to the gui thread
extern "C" void postFrameReady(void *widget) {
    QMetaObject::invokeMethod(static_cast<DisplayWidget *>(widget), "frameReady", Qt::QueuedConnection);
}

}

DisplayWidget::DisplayWidget(QWidget *parent) : QWidget(parent) {
    oboromi_frame_set_callback(&postFrameReady, this);
}

DisplayWidget::~DisplayWidget() {
    oboromi_frame_set_callback(nullptr, nullptr);
}

void DisplayWidget::frameReady() {
    const auto generation = oboromi_frame_generation();
    if (generation == m_generation) return;

    uint32_t width = 0;
    uint32_t height = 0;
    oboromi_frame_read(nullptr, 0, &width, &height);
    QImage image(width, height, QImage::Format_RGBA8888);
    oboromi_frame_read(image.bits(), image.sizeInBytes(), &width, &height);
    if (image.width() != int(width) || image.height() != int(height)) return;

    m_image = image;
    m_generation = generation;
    update();
}

void DisplayWidget::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.fillRect(rect(), Qt::black);
    if (m_image.isNull()) return;

    QRect target(QPoint(), m_image.size().scaled(size(), Qt::KeepAspectRatio));
    target.moveCenter(rect().center());
    painter.setRenderHint(QPainter::SmoothPixmapTransform);
    painter.drawImage(target, m_image);
}
