#pragma once

#include <QDateTime>
#include <QLocale>
#include <QString>

// Thin bindings for QLocale overloads not exposed by cxx-qt-lib.
namespace fluffsetup_clock {
inline QString timePattern(const QString &name) {
    return QLocale(name).timeFormat(QLocale::ShortFormat);
}
inline QString localizedTime(const QDateTime &value, const QString &name, const QString &pattern) {
    return QLocale(name).toString(value, pattern);
}
inline QString localizedDate(const QDateTime &value, const QString &name) {
    const QLocale locale(name);
    return locale.toString(value, locale.dateFormat(QLocale::ShortFormat));
}
}
