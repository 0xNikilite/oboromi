#include <QAbstractAnimation>
#include <QApplication>
#include <QGraphicsOpacityEffect>
#include <QPixmap>
#include <QPropertyAnimation>
#include <QSplashScreen>
#include <QTimer>

#include "MainWindow.h"

namespace {

// same splash timing as the old egui frontend: 400ms in, 800ms hold, 400ms out.
void showSplashThenMainWindow(QApplication &app) {
    (void)app;
    QPixmap logo(":/oboromi_logo.png");

    auto *splash = new QSplashScreen(logo);
    splash->setWindowFlag(Qt::WindowStaysOnTopHint);

    auto *effect = new QGraphicsOpacityEffect(splash);
    splash->setGraphicsEffect(effect);
    effect->setOpacity(0.0);

    splash->showMessage(
        "Experimental\n"
        "This is an experimental foundation for Switch 2 emulation.\n"
        "Without a kernel exploit, running retail games is currently impossible.",
        Qt::AlignHCenter | Qt::AlignBottom, QColor(180, 180, 180));
    splash->show();

    auto *fadeIn = new QPropertyAnimation(effect, "opacity", splash);
    fadeIn->setDuration(400);
    fadeIn->setStartValue(0.0);
    fadeIn->setEndValue(1.0);

    auto *fadeOut = new QPropertyAnimation(effect, "opacity", splash);
    fadeOut->setDuration(400);
    fadeOut->setStartValue(1.0);
    fadeOut->setEndValue(0.0);

    auto *window = new MainWindow();
    window->setAttribute(Qt::WA_DeleteOnClose);

    QObject::connect(fadeOut, &QPropertyAnimation::finished, splash, [splash, window] {
        splash->close();
        splash->deleteLater();
        window->show();
    });

    QObject::connect(fadeIn, &QPropertyAnimation::finished, splash, [fadeOut] {
        QTimer::singleShot(800, fadeOut, [fadeOut] {
            fadeOut->start(QAbstractAnimation::DeleteWhenStopped);
        });
    });

    fadeIn->start(QAbstractAnimation::DeleteWhenStopped);
}

} // namespace

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);

    // flat classic menubar, no rounded windows11 hover. rest of the app
    // keeps the native style.
    app.setStyleSheet(R"(
        QMenuBar {
            background: #1f1f1f;
        }
        QMenuBar::item {
            background: transparent;
            border-radius: 0px;
            padding: 4px 10px;
        }
        QMenuBar::item:selected {
            background: #3a3a3a;
        }
        QMenuBar::item:pressed {
            background: #454545;
        }
        QMenu {
            background: #242424;
            border: 1px solid #383838;
        }
        QMenu::item {
            padding: 6px 28px;
        }
        QMenu::item:selected {
            background: #3a3a3a;
        }
        QMenu::separator {
            height: 1px;
            background: #383838;
            margin: 4px 8px;
        }
    )");

    app.setApplicationName("oboromi");
    app.setOrganizationName("Nikilite");

    showSplashThenMainWindow(app);

    return app.exec();
}
