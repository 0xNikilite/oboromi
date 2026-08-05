#include "MainWindow.h"

#include <QDesktopServices>
#include <QFontDatabase>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QListWidget>
#include <QMenuBar>
#include <QPushButton>
#include <QUrl>
#include <QVBoxLayout>
#include <QWidget>
#include <QtConcurrent/QtConcurrent>

#include "RustBridge.h"

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

    connect(&m_cpuWatcher, &QFutureWatcher<std::vector<std::string>>::finished,
            this, &MainWindow::onCpuTestsFinished);
    connect(&m_gpuWatcher, &QFutureWatcher<std::vector<std::string>>::finished,
            this, &MainWindow::onGpuTestsFinished);

    m_logList->addItem("click 'Run CPU Tests' to begin");
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
    buttons->addStretch();
    layout->addLayout(buttons);

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

void MainWindow::onCpuTestsFinished() {
    appendLines(m_cpuWatcher.result());
    setTestsRunning(false);
}

void MainWindow::onGpuTestsFinished() {
    appendLines(m_gpuWatcher.result());
    setTestsRunning(false);
}
