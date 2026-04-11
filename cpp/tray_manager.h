// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

#pragma once

#include <QAction>
#include <QApplication>
#include <QMenu>
#include <QObject>
#include <QSystemTrayIcon>

class TrayManager : public QObject {
  Q_OBJECT

public:
  explicit TrayManager(QObject *parent = nullptr);
  ~TrayManager() override;

  Q_INVOKABLE void setShowHideLabel(const QString &label);
  Q_INVOKABLE void setServerLabel(const QString &label);
  Q_INVOKABLE void showWindow();
  Q_INVOKABLE void hideWindow();

signals:
  void showHideTriggered();
  void serverToggleTriggered();
  void quitTriggered();

private:
  QSystemTrayIcon *m_trayIcon;
  QMenu *m_menu;
  QAction *m_showHideAction;
  QAction *m_serverAction;
  QAction *m_quitAction;
};
