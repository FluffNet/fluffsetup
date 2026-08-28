#include <QBackingStore>
#include <QEvent>
#include <QGuiApplication>
#include <QImage>
#include <QList>
#include <QObject>
#include <QPainter>
#include <QPointer>
#include <QRegion>
#include <QScreen>
#include <QSize>
#include <QTimer>
#include <QWindow>

#include <memory>
#include <vector>

namespace {

constexpr auto wallpaperPath =
    "/usr/share/wallpapers/Cluster/contents/images/3840x2160.png";

class WallpaperWindow final : public QWindow {
public:
    explicit WallpaperWindow(QScreen *output)
        : output_(output), backingStore_(this), wallpaper_(wallpaperPath) {
        setScreen(output);
        setFlags(Qt::FramelessWindowHint | Qt::Tool | Qt::WindowStaysOnBottomHint |
                 Qt::WindowDoesNotAcceptFocus | Qt::WindowTransparentForInput);
        setTitle(QStringLiteral("FluffSetup Background"));
        setGeometry(output->geometry());
        showFullScreen();
    }

    QScreen *output() const { return output_.data(); }

protected:
    bool event(QEvent *event) override {
        if (event->type() == QEvent::Expose || event->type() == QEvent::Resize) {
            render();
        }
        return QWindow::event(event);
    }

private:
    void render() {
        if (!isExposed()) {
            return;
        }

        backingStore_.resize(size());
        backingStore_.beginPaint(QRegion(QRect(QPoint(), size())));
        QPaintDevice *device = backingStore_.paintDevice();
        QPainter painter(device);
        const QSize deviceSize(device->width(), device->height());
        painter.fillRect(QRect(QPoint(), deviceSize), Qt::black);

        if (!wallpaper_.isNull()) {
            const QImage scaled = wallpaper_.scaled(
                deviceSize, Qt::KeepAspectRatioByExpanding, Qt::SmoothTransformation);
            const QPoint origin((device->width() - scaled.width()) / 2,
                                (device->height() - scaled.height()) / 2);
            painter.drawImage(origin, scaled);
        }

        painter.end();
        backingStore_.endPaint();
        backingStore_.flush(QRegion(QRect(QPoint(), size())));
    }

    QPointer<QScreen> output_;
    QBackingStore backingStore_;
    QImage wallpaper_;
};

class WallpaperManager final : public QObject {
public:
    explicit WallpaperManager(QObject *parent = nullptr) : QObject(parent) {
        // Own one input transparent wallpaper window per connected output so
        // connecting a monitor cannot reveal an unpainted setup session.
        rebuild();
        connect(qApp, &QGuiApplication::screenAdded, this,
                [this](QScreen *addedScreen) {
                    addWallpaperWindow(addedScreen);
                    restoreSetupToAddedScreen(addedScreen);
                    scheduleSetupRebind();
                });
        connect(qApp, &QGuiApplication::screenRemoved, this,
                [this](QScreen *removedScreen) {
                    moveSetupFromRemovedScreen(removedScreen);
                    removeWallpaperWindow(removedScreen);
                    scheduleSetupRebind();
                });
        connect(qApp, &QGuiApplication::primaryScreenChanged, this,
                [this](QScreen *) { scheduleSetupRebind(); });
    }

    void bindSetupWindow() {
        QWindow *setupWindow = nullptr;
        for (QWindow *window : QGuiApplication::topLevelWindows()) {
            if (window->title() == QStringLiteral("Fluff Linux Setup")) {
                setupWindow = window;
                break;
            }
        }

        if (setupWindow == nullptr) {
            return;
        }

        if (setupWindow_ != setupWindow) {
            if (screenChangedConnection_) {
                disconnect(screenChangedConnection_);
            }
            setupWindow_ = setupWindow;
            if (QScreen *primaryScreen = QGuiApplication::primaryScreen()) {
                setupWindow->setScreen(primaryScreen);
            }
            screenChangedConnection_ = connect(
                setupWindow, &QWindow::screenChanged, this,
                [this](QScreen *screen) {
                    fitSetupWindowToScreen(screen);
                    scheduleSetupRebind();
                });
        }

        if (windows_.empty()) {
            waitingForScreen_ = true;
            setupWindow_->setTransientParent(nullptr);
            setupWindow_->hide();
            return;
        }

        if (waitingForScreen_) {
            return;
        }

        fitSetupWindowToScreen(setupWindow->screen());

        QWindow *backgroundWindow = nullptr;
        for (const auto &window : windows_) {
            if (window->output() == setupWindow->screen()) {
                backgroundWindow = window.get();
                break;
            }
        }

        if (backgroundWindow == nullptr && !windows_.empty()) {
            QScreen *fallbackScreen = availableFallbackScreen();
            if (fallbackScreen != nullptr) {
                setupWindow->setScreen(fallbackScreen);
                addWallpaperWindow(fallbackScreen);
                for (const auto &window : windows_) {
                    if (window->output() == fallbackScreen) {
                        backgroundWindow = window.get();
                        break;
                    }
                }
            }
        }

        if (backgroundWindow != nullptr) {
            // On Wayland, the compositor decides stacking. Making the setup
            // dialog transient for its background on the same screen keeps the
            // card above it without a focus polling loop.
            setupWindow->setTransientParent(backgroundWindow);
        }

        setupWindow->showNormal();
        setupWindow->raise();
        setupWindow->requestActivate();
    }

private:
    void fitSetupWindowToScreen(QScreen *screen) {
        if (setupWindow_ == nullptr || screen == nullptr) {
            return;
        }

        const QRect screenGeometry = screen->geometry();
        const bool compactScreen =
            screenGeometry.width() <= 1280 || screenGeometry.height() <= 720;
        const int width = compactScreen
                              ? screenGeometry.width()
                              : qRound(screenGeometry.width() * 0.90);
        const int height = compactScreen
                               ? screenGeometry.height()
                               : qRound(screenGeometry.height() * 0.88);
        const QSize targetSize(width, height);
        const QPoint targetPosition(
            screenGeometry.x() + (screenGeometry.width() - width) / 2,
            screenGeometry.y() + (screenGeometry.height() - height) / 2);

        // Clear the old output's fixed constraints before resizing. Without
        // this, a 1440p minimum size can prevent the compositor from shrinking
        // the existing surface when it falls back to a 1080p output.
        setupWindow_->setMinimumSize(QSize(0, 0));
        setupWindow_->setMaximumSize(screenGeometry.size());
        setupWindow_->setGeometry(QRect(targetPosition, targetSize));
        setupWindow_->setMinimumSize(targetSize);
        setupWindow_->setMaximumSize(targetSize);
    }

    void scheduleSetupRebind() {
        const unsigned long long generation = ++setupRebindGeneration_;
        // A monitor power cycle can withdraw and restore its Wayland output
        // over roughly two seconds. Reassert the final screen, size, parent,
        // and visibility throughout that bounded topology-settling period.
        scheduleSetupRebindAfter(0, generation);
        scheduleSetupRebindAfter(250, generation);
        scheduleSetupRebindAfter(1000, generation);
        scheduleSetupRebindAfter(2000, generation);
    }

    void scheduleSetupRebindAfter(int delay,
                                  unsigned long long generation) {
        QTimer::singleShot(delay, this, [this, generation]() {
            if (generation == setupRebindGeneration_) {
                recoverDisplayTopology();
            }
        });
    }

    void recoverDisplayTopology() {
        const QList<QScreen *> screens = QGuiApplication::screens();

        // screenAdded/screenRemoved can be emitted while Wayland is still
        // publishing the rest of a new output topology. Reconcile the complete
        // set on every retry instead of assuming that the individual signal
        // sequence was complete.
        for (auto window = windows_.begin(); window != windows_.end();) {
            if ((*window)->output() == nullptr ||
                !screens.contains((*window)->output())) {
                window = windows_.erase(window);
            } else {
                ++window;
            }
        }
        for (QScreen *screen : screens) {
            addWallpaperWindow(screen);
        }

        if (screens.empty()) {
            waitingForScreen_ = true;
            if (setupWindow_ != nullptr) {
                setupWindow_->setTransientParent(nullptr);
                setupWindow_->hide();
            }
            return;
        }

        if (setupWindow_ == nullptr) {
            bindSetupWindow();
            return;
        }

        QScreen *targetScreen = QGuiApplication::primaryScreen();
        if (targetScreen == nullptr || !screens.contains(targetScreen)) {
            targetScreen = screens.first();
        }

        waitingForScreen_ = false;
        if (setupWindow_->screen() != targetScreen) {
            setupWindow_->setTransientParent(nullptr);
            setupWindow_->setScreen(targetScreen);
        }
        fitSetupWindowToScreen(targetScreen);
        bindSetupWindow();
    }

    QScreen *availableFallbackScreen(QScreen *excludedScreen = nullptr) const {
        const QList<QScreen *> screens = QGuiApplication::screens();
        QScreen *primaryScreen = QGuiApplication::primaryScreen();
        if (primaryScreen != excludedScreen && screens.contains(primaryScreen)) {
            return primaryScreen;
        }
        for (QScreen *screen : screens) {
            if (screen != excludedScreen) {
                return screen;
            }
        }
        return nullptr;
    }

    void moveSetupFromRemovedScreen(QScreen *removedScreen) {
        if (setupWindow_ == nullptr ||
            (setupWindow_->screen() != nullptr &&
             setupWindow_->screen() != removedScreen)) {
            return;
        }

        QScreen *fallbackScreen = availableFallbackScreen(removedScreen);

        if (fallbackScreen != nullptr) {
            waitingForScreen_ = false;
            setupWindow_->setScreen(fallbackScreen);
            fitSetupWindowToScreen(fallbackScreen);
            scheduleSetupRebind();
        } else {
            waitingForScreen_ = true;
            setupWindow_->setTransientParent(nullptr);
            setupWindow_->hide();
        }
    }

    void restoreSetupToAddedScreen(QScreen *addedScreen) {
        if (setupWindow_ == nullptr) {
            bindSetupWindow();
            return;
        }

        bool currentScreenIsAvailable = false;
        for (QScreen *screen : QGuiApplication::screens()) {
            if (screen == setupWindow_->screen()) {
                currentScreenIsAvailable = true;
                break;
            }
        }

        if (waitingForScreen_ || !currentScreenIsAvailable) {
            waitingForScreen_ = false;
            setupWindow_->setScreen(addedScreen);
            fitSetupWindowToScreen(addedScreen);
        }
        bindSetupWindow();
        scheduleSetupRebind();
    }

    void addWallpaperWindow(QScreen *screen) {
        if (screen == nullptr) {
            return;
        }
        for (const auto &window : windows_) {
            if (window->output() == screen) {
                return;
            }
        }
        windows_.push_back(std::make_unique<WallpaperWindow>(screen));
    }

    void removeWallpaperWindow(QScreen *screen) {
        for (auto window = windows_.begin(); window != windows_.end();) {
            if ((*window)->output() == screen || (*window)->output() == nullptr) {
                window = windows_.erase(window);
            } else {
                ++window;
            }
        }
    }

    void rebuild() {
        windows_.clear();
        for (QScreen *output : QGuiApplication::screens()) {
            windows_.push_back(std::make_unique<WallpaperWindow>(output));
        }
        bindSetupWindow();
    }

    std::vector<std::unique_ptr<WallpaperWindow>> windows_;
    QPointer<QWindow> setupWindow_;
    QMetaObject::Connection screenChangedConnection_;
    bool waitingForScreen_ = false;
    unsigned long long setupRebindGeneration_ = 0;
};

std::unique_ptr<WallpaperManager> wallpaperManager;

} // namespace

extern "C" void fluffsetup_initialize_session_background() {
    qGuiApp->setQuitOnLastWindowClosed(false);
    wallpaperManager = std::make_unique<WallpaperManager>();
}

extern "C" void fluffsetup_bind_setup_window() {
    if (wallpaperManager != nullptr) {
        wallpaperManager->bindSetupWindow();
    }
}
