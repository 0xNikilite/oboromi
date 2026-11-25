#include <cstring>
#include "TextureDemo.h"
#include "oboromi_core.h"

#include <QDialog>
#include <QFileDialog>
#include <QGridLayout>
#include <QImageReader>
#include <QLabel>
#include <QMessageBox>
#include <QPainter>
#include <QPushButton>

namespace {
constexpr int width = 128;
constexpr int height = 96;

QImage prepareImage(const QImage &image) {
    QImage canvas(width, height, QImage::Format_RGBA8888);
    canvas.fill(QColor("#22344b"));
    QPainter painter(&canvas);
    painter.setRenderHint(QPainter::SmoothPixmapTransform);
    const QSize size = image.size().scaled(canvas.size(), Qt::KeepAspectRatio);
    painter.drawImage(QRect((width - size.width()) / 2, (height - size.height()) / 2,
                            size.width(), size.height()), image);
    return canvas;
}

void preview(QLabel &label, const QImage &image) {
    label.setPixmap(QPixmap::fromImage(image).scaled(width * 2, height * 2,
                                                     Qt::KeepAspectRatio, Qt::FastTransformation));
}
}

void showTextureDemo(QWidget *parent) {
    QDialog dialog(parent);
    dialog.setWindowTitle("oboromi · texture demo");
    QGridLayout layout(&dialog);
    layout.setContentsMargins(24, 20, 24, 20);
    layout.setHorizontalSpacing(20);
    layout.setVerticalSpacing(12);

    QLabel title("texture transfer");
    auto font = title.font();
    font.setPointSize(font.pointSize() + 7);
    font.setBold(true);
    title.setFont(font);
    layout.addWidget(&title, 0, 0, 1, 2);
    QLabel sourceTitle("Source image");
    QLabel resultTitle("GPU memory");
    layout.addWidget(&sourceTitle, 1, 0);
    layout.addWidget(&resultTitle, 1, 1);
    QLabel sourceView, resultView;
    for (auto *view : {&sourceView, &resultView}) {
        view->setFixedSize(width * 2, height * 2);
        view->setAlignment(Qt::AlignCenter);
        view->setFrameShape(QFrame::StyledPanel);
    }
    layout.addWidget(&sourceView, 2, 0);
    layout.addWidget(&resultView, 2, 1);
    QLabel status;
    status.setWordWrap(true);
    status.setMinimumHeight(48);
    layout.addWidget(&status, 3, 0, 1, 2);
    QPushButton choose("Open image…"), step("Upload texture");
    layout.addWidget(&choose, 4, 0);
    layout.addWidget(&step, 4, 1);
    step.setDefault(true);

    QImage source = prepareImage(QImage(":/oboromi_logo.png"));
    QImage tiled;
    auto reset = [&] {
        preview(sourceView, source);
        tiled = QImage();
        resultTitle.setText("GPU memory");
        resultView.setText("Waiting for upload");
        status.setText("Images are resized to fit 128 × 96 pixels if needed. Their proportions stay the same.");
        step.setText("Upload texture");
        step.setEnabled(true);
    };
    reset();
    QObject::connect(&choose, &QPushButton::clicked, &dialog, [&] {
        QFileDialog::Options options;
#ifdef Q_OS_LINUX
        options |= QFileDialog::DontUseNativeDialog;
#endif
        const auto path = QFileDialog::getOpenFileName(&dialog, "Open image", {},
                                                       "Images (*.png *.jpg *.jpeg *.bmp)", nullptr, options);
        if (path.isEmpty()) return;
        QImageReader reader(path);
        reader.setAutoTransform(true);
        const auto size = reader.size();
        if (size.isValid()) reader.setScaledSize(size.scaled(width, height, Qt::KeepAspectRatio));
        const auto image = reader.read();
        if (image.isNull()) {
            QMessageBox::warning(&dialog, "Open image", reader.errorString());
            return;
        }
        source = prepareImage(image);
        reset();
    });
    QObject::connect(&step, &QPushButton::clicked, &dialog, [&] {
        const bool upload = tiled.isNull();
        const QImage &input = upload ? source : tiled;
        QImage output(width, height, QImage::Format_RGBA8888);
        if (!oboromi_texture_transfer(input.constBits(), output.bits(), output.sizeInBytes(),
                                      width, height, upload)) {
            status.setText("The copy engine rejected this transfer.");
            return;
        }
        preview(resultView, output);
        if (upload) {
            tiled = output;
            resultTitle.setText("GPU memory · raw order");
            status.setText("These are the uploaded bytes drawn in order. Read them back to put the pixels into rows again.");
            step.setText("Read back texture");
        } else {
            resultTitle.setText("Readback image");
            const bool matched = std::memcmp(output.constBits(), source.constBits(), output.sizeInBytes()) == 0;
            status.setText(matched ? "All 12,288 pixels match the source, including their alpha values."
                                   : "The readback differs from the source.");
            tiled = QImage();
            step.setText("Upload again");
        }
    });
    dialog.exec();
}
