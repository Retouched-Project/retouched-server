// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

#include "tray_manager.h"
#include <QGuiApplication>
#include <QIcon>

TrayManager::TrayManager(QObject *parent) : QObject(parent) {
  QGuiApplication::setQuitOnLastWindowClosed(false);

  m_menu = new QMenu();
  m_showHideAction = m_menu->addAction("Hide Window");
  m_menu->addSeparator();
  m_serverAction = m_menu->addAction("Start Server");
  m_menu->addSeparator();
  m_quitAction = m_menu->addAction("Quit");

  connect(m_showHideAction, &QAction::triggered, this,
          &TrayManager::showHideTriggered);
  connect(m_serverAction, &QAction::triggered, this,
          &TrayManager::serverToggleTriggered);
  connect(m_quitAction, &QAction::triggered, this, &TrayManager::quitTriggered);

  m_trayIcon = new QSystemTrayIcon(this);
  m_trayIcon->setContextMenu(m_menu);
  m_trayIcon->setToolTip("Retouched Server");
  m_trayIcon->setIcon(QIcon(":/assets/retouched_logo_icons.png"));

  connect(m_trayIcon, &QSystemTrayIcon::activated, this,
          [this](QSystemTrayIcon::ActivationReason reason) {
            if (reason == QSystemTrayIcon::Trigger)
              emit showHideTriggered();
          });

  m_trayIcon->show();
}

TrayManager::~TrayManager() { delete m_menu; }

void TrayManager::setShowHideLabel(const QString &label) {
  m_showHideAction->setText(label);
}

void TrayManager::setServerLabel(const QString &label) {
  m_serverAction->setText(label);
}

void TrayManager::showWindow() { m_showHideAction->setText("Hide Window"); }

void TrayManager::hideWindow() { m_showHideAction->setText("Show Window"); }
