#include "MainWindow.h"

#include <QDesktopServices>
#include <QCheckBox>
#include <QComboBox>
#include <QFontDatabase>
#include <QFrame>
#include <QHBoxLayout>
#include <QKeyEvent>
#include <QLabel>
#include <QListWidget>
#include <QMenuBar>
#include <QPushButton>
#include <QThreadPool>
#include <QUrl>
#include <QVBoxLayout>
#include <QWidget>
#include <QtConcurrent/QtConcurrent>

#include "AudioOutput.h"
#include "DisplayWidget.h"
#include "RustBridge.h"
#include "oboromi_core.h"

namespace {

QColor colorForLine(const QString &line) {
    if (line.contains("PASS")) return QColor(50, 255, 50);
    if (line.contains("FAIL")) return QColor(255, 50, 50);
    return QColor(200, 200, 200);
}

}

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
    setWindowTitle("oboromi");
    resize(1200, 800);

    buildMenuBar();
    buildUi();
    m_audio = new AudioOutput(this);

    connect(&m_cpuWatcher, &QFutureWatcher<std::vector<std::string>>::finished,
            this, &MainWindow::onCpuTestsFinished);
    connect(&m_gpuWatcher, &QFutureWatcher<std::vector<std::string>>::finished,
            this, &MainWindow::onGpuTestsFinished);

    m_logList->addItem("click 'Run CPU Tests' to begin");
    // the vulkan display is a separate native window, so keys are taken before any widget sees them
    qApp->installEventFilter(this);
}

bool MainWindow::eventFilter(QObject *watched, QEvent *event) {
    if (event->type() == QEvent::KeyPress || event->type() == QEvent::KeyRelease)
        oboromi_input_key(static_cast<QKeyEvent *>(event)->key(), event->type() == QEvent::KeyPress);
    return QMainWindow::eventFilter(watched, event);
}

void MainWindow::buildMenuBar() {
    auto *fileMenu = menuBar()->addMenu("&File");
    auto *quitAction = fileMenu->addAction("&Quit");
    quitAction->setMenuRole(QAction::QuitRole);
    connect(quitAction, &QAction::triggered, this, &QWidget::close);

    auto *aboutMenu = menuBar()->addMenu("&About");
    auto *codeAction = aboutMenu->addAction("See the code");
    connect(codeAction, &QAction::triggered, this, [] {
        QDesktopServices::openUrl(QUrl("https://git.eden-emu.dev/Nikilite/oboromi/"));
    });
}

void MainWindow::buildUi() {
    auto *central = new QWidget(this);
    auto *layout = new QVBoxLayout(central);

    auto *heading = new QLabel("oboromi", central);
    auto font = heading->font();
    font.setPointSize(font.pointSize() + 8);
    font.setBold(true);
    heading->setFont(font);
    layout->addWidget(heading);

    auto *separator = new QFrame(central);
    separator->setFrameShape(QFrame::HLine);
    separator->setFrameShadow(QFrame::Sunken);
    layout->addWidget(separator);

    auto *buttons = new QHBoxLayout();
    m_cpuButton = new QPushButton("Run CPU Tests", central);
    m_gpuButton = new QPushButton("Run GPU Tests", central);
    connect(m_cpuButton, &QPushButton::clicked, this, &MainWindow::onRunCpuTestsClicked);
    connect(m_gpuButton, &QPushButton::clicked, this, &MainWindow::onRunGpuTestsClicked);
    buttons->addWidget(m_cpuButton);
    buttons->addWidget(m_gpuButton);
    auto *frameButton = new QPushButton("Render Test Frame", central);
    connect(frameButton, &QPushButton::clicked, this, &MainWindow::onRenderTestFrameClicked);
    buttons->addWidget(frameButton);
    for (auto [name, render] : {std::pair{"Demo", RustBridge::renderDemoFrame}, {"Triangle", RustBridge::renderTriangleDemo}, {"Post", RustBridge::renderPostFrame},
                                {"Dungeon", RustBridge::renderDungeonFrame}, {"Sprites", RustBridge::renderSpritesFrame}, {"BoomBox", RustBridge::renderBoomBoxDemo}, {"Ray traced", RustBridge::renderRaytracedDemo}, {"Helmet", RustBridge::renderHelmetDemo},
                                {"Particles", RustBridge::renderParticlesFrame}, {"Music", RustBridge::renderMusicDemo}, {"Bounce", RustBridge::renderBounceDemo}}) {
        auto *demoButton = new QPushButton(name, central);
        demoButton->setCheckable(true);
        connect(demoButton, &QPushButton::toggled, this, [this, render](bool on) {
            if (!on) {
                m_demoTimer.stop();
                m_demoFrameClock.invalidate();
                m_frameLabel->clear();
                RustBridge::flushDemoCaches();
                return;
            }
            m_demoRender = render;
            m_demoClock.start();
            m_demoTimer.start(16);
        });
        buttons->addWidget(demoButton);
    }
    auto *rayTracing = new QCheckBox("Ray tracing", central);
    rayTracing->setChecked(true);
    connect(rayTracing, &QCheckBox::toggled, this, RustBridge::setRaytracingEnabled);
    buttons->addWidget(rayTracing);
    auto *quality = new QComboBox(central);
    quality->addItems({"Dungeon: showcase", "Dungeon: realtime"});
    auto *rays = new QCheckBox("Ray tracing", central);
    rays->setEnabled(oboromi_dungeon_ray_queries());
    rays->setChecked(rays->isEnabled());
    connect(quality, &QComboBox::currentIndexChanged, this, [quality, rays](int index) {
        rays->setChecked(index == 0 && rays->isEnabled());
        oboromi_dungeon_settings(index == 0, rays->isChecked());
    });
    connect(rays, &QCheckBox::toggled, this, [quality](bool enabled) { oboromi_dungeon_settings(quality->currentIndex() == 0, enabled); });
    buttons->addWidget(quality);
    buttons->addWidget(rays);
    auto *spatialMode = new QComboBox(central);
    spatialMode->addItems({"Headphones", "Speakers"});
    connect(spatialMode, &QComboBox::currentIndexChanged, this, [](int mode) { oboromi_set_dungeon_speakers(mode == 1); });
    buttons->addWidget(spatialMode);
    connect(&m_demoTimer, &QTimer::timeout, this, [this] {
        if (m_demoFrame.isFinished()) {
            if (m_demoFrameClock.isValid()) m_frameLabel->setText(QString("%1 ms").arg(m_demoFrameClock.restart()));
            else m_demoFrameClock.start();
            m_demoFrame = QtConcurrent::run(m_demoRender, m_demoClock.elapsed() / 1000.0f);
        }
        OboromiInput input;
        oboromi_input_state(&input);
        m_gamepadLabel->setText(input.gamepad[0] ? QString("gamepad: ") + input.gamepad : QString());
    });
    auto *toneButton = new QPushButton("Tone", central);
    toneButton->setCheckable(true);
    connect(toneButton, &QPushButton::toggled, this, [this](bool on) {
        if (on) return m_toneTimer.start(5);
        m_toneTimer.stop();
        m_logList->addItem(m_audio->status());
    });
    connect(&m_toneTimer, &QTimer::timeout, this, [] { oboromi_audio_test_tone(0.1f); });
    buttons->addWidget(toneButton);
    buttons->addStretch();
    m_frameLabel = new QLabel(central);
    buttons->addWidget(m_frameLabel);
    m_gamepadLabel = new QLabel(central);
    buttons->addWidget(m_gamepadLabel);
    layout->addLayout(buttons);

    layout->addWidget(createDisplay(central), /*stretch=*/3);

    auto *resultsLabel = new QLabel("Results:", central);
    resultsLabel->setStyleSheet("color: rgb(200, 200, 200);");
    layout->addWidget(resultsLabel);

    m_logList = new QListWidget(central);
    m_logList->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
    layout->addWidget(m_logList, /*stretch=*/1);

    setCentralWidget(central);
}

void MainWindow::setTestsRunning(bool running) {
    m_cpuButton->setEnabled(!running);
    m_gpuButton->setEnabled(!running);
}

void MainWindow::appendLines(const std::vector<std::string> &lines) {
    m_logList->clear();
    for (const auto &line : lines) {
        auto *item = new QListWidgetItem(QString::fromStdString(line));
        item->setForeground(colorForLine(item->text()));
        m_logList->addItem(item);
    }
}

void MainWindow::onRunCpuTestsClicked() {
    setTestsRunning(true);
    m_logList->clear();
    m_logList->addItem("Warming up JIT compiler...");
    m_logList->addItem("Running ARM64 tests...");
    m_cpuWatcher.setFuture(QtConcurrent::run(RustBridge::runCpuTests));
}

void MainWindow::onRunGpuTestsClicked() {
    setTestsRunning(true);
    m_logList->clear();
    m_logList->addItem("Initializing GPU decoder environment...");
    m_logList->addItem("Running SM86 instruction translations...");
    m_gpuWatcher.setFuture(QtConcurrent::run(RustBridge::runGpuTests));
}

void MainWindow::onRenderTestFrameClicked() {
    QThreadPool::globalInstance()->start([this] {
        if (!RustBridge::renderTestFrame())
            QMetaObject::invokeMethod(this, [this] { m_logList->addItem("Render test frame failed"); });
    });
}

void MainWindow::onCpuTestsFinished() {
    appendLines(m_cpuWatcher.result());
    setTestsRunning(false);
}

void MainWindow::onGpuTestsFinished() {
    appendLines(m_gpuWatcher.result());
    setTestsRunning(false);
}
