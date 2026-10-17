#pragma once

#include <QElapsedTimer>
#include <QFutureWatcher>
#include <QMainWindow>
#include <QTimer>
#include <string>
#include <vector>

class QListWidget;
class QPushButton;

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);

private slots:
    void onRunCpuTestsClicked();
    void onRunGpuTestsClicked();
    void onRenderTestFrameClicked();
    void onCpuTestsFinished();
    void onGpuTestsFinished();

private:
    void buildMenuBar();
    void buildUi();
    void appendLines(const std::vector<std::string> &lines);
    void setTestsRunning(bool running);

    QListWidget *m_logList = nullptr;
    QPushButton *m_cpuButton = nullptr;
    QPushButton *m_gpuButton = nullptr;

    QFutureWatcher<std::vector<std::string>> m_cpuWatcher;
    QFutureWatcher<std::vector<std::string>> m_gpuWatcher;

    QTimer m_demoTimer;
    QElapsedTimer m_demoClock;
    QFuture<bool> m_demoFrame;
};
