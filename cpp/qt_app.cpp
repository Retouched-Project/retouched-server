// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

#include "qt_app.h"
#include "tray_manager.h"
#include <QApplication>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTimer>
#include <QUrl>
#include <cstring>

#ifdef Q_OS_LINUX
#if __has_include(<wayland-client.h>)
#include <wayland-client.h>
#define HAS_WAYLAND 1
#endif
#endif

#ifdef Q_OS_WIN
#include <shobjidl.h>
#include <windows.h>
#endif

extern "C" void trayServerToggle();
extern "C" bool trayIsServerRunning();
extern "C" void trayQuitRequested();

#ifdef HAS_WAYLAND
static bool s_hasDecorationManager = false;

static void registryGlobal(void *, struct wl_registry *, uint32_t,
                           const char *interface, uint32_t) {
  if (strcmp(interface, "zxdg_decoration_manager_v1") == 0)
    s_hasDecorationManager = true;
}

static void registryGlobalRemove(void *, struct wl_registry *, uint32_t) {}

static const struct wl_registry_listener s_registryListener = {
    registryGlobal,
    registryGlobalRemove,
};
#endif

static bool needsClientSideDecorations() {
#if defined(Q_OS_LINUX) && defined(HAS_WAYLAND)
  if (QGuiApplication::platformName() != QLatin1String("wayland"))
    return false;

  struct wl_display *display = wl_display_connect(nullptr);
  if (!display)
    return true;

  s_hasDecorationManager = false;
  struct wl_registry *registry = wl_display_get_registry(display);
  wl_registry_add_listener(registry, &s_registryListener, nullptr);
  wl_display_roundtrip(display);
  wl_registry_destroy(registry);
  wl_display_disconnect(display);

  return !s_hasDecorationManager;
#elif defined(Q_OS_LINUX)
  return qgetenv("XDG_SESSION_TYPE") == "wayland";
#else
  return false;
#endif
}

extern "C" int runQtApp(const char *appName, const char *appVersion) {
#ifdef Q_OS_WIN
  SetCurrentProcessExplicitAppUserModelID(L"com.retouched.server");
#endif
  static int argc = 1;
  static char arg0[] = "retouched-server";
  static char *argv[] = {arg0, nullptr};

  QApplication app(argc, argv);
  app.setApplicationName(appName);
  app.setApplicationVersion(appVersion);
  app.setWindowIcon(QIcon(":/assets/retouched_logo_icons.png"));

  bool useCsd = needsClientSideDecorations();

  if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
#ifdef Q_OS_WIN
    QQuickStyle::setStyle("Fusion");
#elif defined(Q_OS_LINUX)
    if (useCsd)
      QQuickStyle::setStyle("Fusion");
#endif
  }

  auto *engine = new QQmlApplicationEngine;
  engine->rootContext()->setContextProperty("useCsd", useCsd);
  engine->load(
      QUrl(QStringLiteral("qrc:/qt/qml/com/retouched/server/qml/main.qml")));

  if (engine->rootObjects().isEmpty()) {
    delete engine;
    return 1;
  }

  TrayManager tray;

  auto *window = qobject_cast<QQuickWindow *>(engine->rootObjects().first());
  if (window) {
    QObject::connect(&tray, &TrayManager::showHideTriggered, [window, &tray]() {
      if (window->isVisible()) {
        window->hide();
        tray.hideWindow();
      } else {
        window->show();
        window->raise();
        window->requestActivate();
        tray.showWindow();
      }
    });

    QObject::connect(window, &QQuickWindow::visibilityChanged,
                     [&tray](QWindow::Visibility vis) {
                       if (vis == QWindow::Hidden)
                         tray.hideWindow();
                       else
                         tray.showWindow();
                     });
  }

  QObject::connect(&tray, &TrayManager::serverToggleTriggered,
                   []() { trayServerToggle(); });

  QObject::connect(&tray, &TrayManager::quitTriggered, [&app, &engine]() {
    trayQuitRequested();
    delete engine;
    engine = nullptr;
    app.quit();
  });

  QTimer labelTimer;
  QObject::connect(&labelTimer, &QTimer::timeout, [&tray]() {
    tray.setServerLabel(trayIsServerRunning() ? "Stop Server" : "Start Server");
  });
  labelTimer.start(500);

  int result = app.exec();
  delete engine;
  return result;
}
