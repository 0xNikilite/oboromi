#include "DisplayWidget.h"

#include <QLayout>
#include <QPainter>
#include <QPlatformSurfaceEvent>
#include <QVulkanInstance>
#include <QWindow>
#include <functional>

#include "oboromi_core.h"

namespace {

// runs on the emulator thread, so it may only post to the gui thread
extern "C" void postFrameReady(void *widget) {
    QMetaObject::invokeMethod(static_cast<DisplayWidget *>(widget), "frameReady", Qt::QueuedConnection);
}

class VulkanDisplay : public QWindow {
public:
    explicit VulkanDisplay(QVulkanInstance *instance) {
        setSurfaceType(VulkanSurface);
        setVulkanInstance(instance);
    }

    std::function<void()> cannotPresent;

protected:
    void exposeEvent(QExposeEvent *) override { attach(); }
    void resizeEvent(QResizeEvent *) override { attach(); }

    bool event(QEvent *event) override {
        if (event->type() == QEvent::PlatformSurface &&
            static_cast<QPlatformSurfaceEvent *>(event)->surfaceEventType() ==
                QPlatformSurfaceEvent::SurfaceAboutToBeDestroyed)
            oboromi_display_surface(0, 0, 0);
        return QWindow::event(event);
    }

private:
    void attach() {
        if (!isExposed() || !cannotPresent) return;
        const QSize pixels = size() * devicePixelRatio();
        const auto surface = uint64_t(QVulkanInstance::surfaceForWindow(this));
        if (oboromi_display_surface(surface, pixels.width(), pixels.height())) return;
        qWarning("the vulkan surface cannot present, falling back to DisplayWidget");
        oboromi_display_surface(0, 0, 0);
        std::exchange(cannotPresent, nullptr)();
    }
};

}

QWidget *createDisplay(QWidget *parent) {
    // adopts the core's instance and outlives every window, so it is never destroyed
    static QVulkanInstance *instance = []() -> QVulkanInstance * {
        const auto handle = static_cast<VkInstance>(oboromi_vk_instance());
        if (!handle) return nullptr;
        auto *vulkan = new QVulkanInstance;
        vulkan->setVkInstance(handle);
        if (vulkan->create()) return vulkan;
        delete vulkan;
        return nullptr;
    }();
    if (!instance) return new DisplayWidget(parent);
    auto *window = new VulkanDisplay(instance);
    auto *container = QWidget::createWindowContainer(window, parent);
    window->cannotPresent = [container] {
        auto *widget = new DisplayWidget(container->parentWidget());
        delete container->parentWidget()->layout()->replaceWidget(container, widget);
        widget->show();
        container->deleteLater();
    };
    return container;
}

DisplayWidget::DisplayWidget(QWidget *parent) : QWidget(parent) {
    oboromi_frame_set_callback(&postFrameReady, this);
    frameReady();
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
