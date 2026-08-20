#include <QBackingStore>
#include <QEvent>
#include <QGuiApplication>
#include <QImage>
#include <QObject>
#include <QPainter>
#include <QRegion>
#include <QScreen>
#include <QWindow>

#include <memory>
#include <vector>

namespace {

constexpr auto wallpaperPath =
    "/usr/share/wallpapers/Cluster/contents/images/3840x2160.png";

class WallpaperWindow final : public QWindow {
public:
    explicit WallpaperWindow(QScreen *output)
        : backingStore_(this), wallpaper_(wallpaperPath) {
        setScreen(output);
        setFlags(Qt::FramelessWindowHint | Qt::Tool | Qt::WindowStaysOnBottomHint |
                 Qt::WindowDoesNotAcceptFocus | Qt::WindowTransparentForInput);
        setTitle(QStringLiteral("FluffSetup Background"));
        setGeometry(output->geometry());
        showFullScreen();
    }

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
                [this](QScreen *) { rebuild(); });
        connect(qApp, &QGuiApplication::screenRemoved, this,
                [this](QScreen *) { rebuild(); });
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

        QWindow *backgroundWindow = nullptr;
        for (const auto &window : windows_) {
            if (window->screen() == setupWindow->screen()) {
                backgroundWindow = window.get();
                break;
            }
        }

        if (backgroundWindow == nullptr && !windows_.empty()) {
            backgroundWindow = windows_.front().get();
        }

        if (backgroundWindow != nullptr) {
            // On Wayland, the compositor decides stacking. Making the setup
            // dialog transient for its background on the same screen keeps the
            // card above it without a focus polling loop.
            setupWindow->setTransientParent(backgroundWindow);
        }

        setupWindow->raise();
        setupWindow->requestActivate();
    }

private:
    void rebuild() {
        windows_.clear();
        for (QScreen *output : QGuiApplication::screens()) {
            windows_.push_back(std::make_unique<WallpaperWindow>(output));
        }
        bindSetupWindow();
    }

    std::vector<std::unique_ptr<WallpaperWindow>> windows_;
};

std::unique_ptr<WallpaperManager> wallpaperManager;

} // namespace

extern "C" void fluffsetup_initialize_session_background() {
    wallpaperManager = std::make_unique<WallpaperManager>();
}

extern "C" void fluffsetup_bind_setup_window() {
    if (wallpaperManager != nullptr) {
        wallpaperManager->bindSetupWindow();
    }
}
