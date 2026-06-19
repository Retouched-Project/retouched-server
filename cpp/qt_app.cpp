// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

#include "qt_app.h"
#include "tray_manager.h"
#include <QApplication>
#include <QIcon>
#include <QMessageBox>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTimer>
#include <QUrl>

#ifdef Q_OS_WIN
#include <shobjidl.h>
#include <windows.h>
#endif

extern "C" void trayServerToggle();
extern "C" bool trayIsServerRunning();
extern "C" void trayQuitRequested();

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

#ifdef Q_OS_WIN
  if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
    QQuickStyle::setStyle("Fusion");
  }
#endif

  auto *engine = new QQmlApplicationEngine;
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

extern "C" void showAlreadyRunningDialog() {
  static int argc = 1;
  static char arg0[] = "retouched-server";
  static char *argv[] = {arg0, nullptr};

  QApplication app(argc, argv);
  app.setWindowIcon(QIcon(":/assets/retouched_logo_icons.png"));

  QMessageBox box;
  box.setIcon(QMessageBox::Warning);
  box.setWindowTitle("Retouched Server");
  box.setText("Retouched Server is already running.");
  box.setInformativeText(
      "Only one instance can run at a time.\n\n"
      "If you cannot find or close the existing window, the process may have "
      "hung. End it from your system's task manager (Activity Monitor on "
      "macOS), or reboot, then try again.");
  box.setStandardButtons(QMessageBox::Ok);
  box.exec();
}
