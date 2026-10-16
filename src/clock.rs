//! Read-only selected-zone clock preview. Qt supplies installed timezone/DST
//! rules and locale formatting; no process timezone or system clock is changed.

use cxx_qt_lib::{QByteArray, QDateTime, QString, QTimeZone};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qdatetime.h");
        include!("cxx-qt-lib/qstring.h");
        include!("native/clock.h");
        type QDateTime = cxx_qt_lib::QDateTime;
        type QString = cxx_qt_lib::QString;
        #[rust_name = "localized_time"]
        #[namespace = "fluffsetup_clock"]
        fn localizedTime(value: &QDateTime, name: &QString, pattern: &QString) -> QString;
        #[rust_name = "time_pattern"]
        #[namespace = "fluffsetup_clock"]
        fn timePattern(name: &QString) -> QString;
        #[rust_name = "localized_date"]
        #[namespace = "fluffsetup_clock"]
        fn localizedDate(value: &QDateTime, name: &QString) -> QString;
    }
}

fn qt_locale(formats: &str) -> String {
    let name = formats.split('.').next().unwrap_or("C");
    if formats.ends_with("@latin")
        && let Some((language, country)) = name.split_once('_')
    {
        format!("{language}_Latn_{country}")
    } else {
        name.split('@').next().unwrap_or("C").into()
    }
}

fn at(id: &str, formats: &str, milliseconds: i64) -> Option<(String, String)> {
    if !crate::timezone::valid(id) {
        return None;
    }
    let zone = QTimeZone::from_iana(&QByteArray::from(id));
    let zone = zone.as_ref().filter(|zone| zone.is_valid())?;
    let value = QDateTime::from_msecs_since_epoch(milliseconds, zone);
    let locale = QString::from(&qt_locale(formats));
    let pattern = QString::from(&without_zone_label(&ffi::time_pattern(&locale).to_string()));
    Some((
        ffi::localized_time(&value, &locale, &pattern).to_string(),
        ffi::localized_date(&value, &locale).to_string(),
    ))
}

// Keep native hour/minute and AM/PM formatting without repeating a
// potentially long timezone name under the icon. Quoted literals are retained.
fn without_zone_label(pattern: &str) -> String {
    let mut chars = pattern.chars().peekable();
    let mut quoted = false;
    let mut result = String::new();
    while let Some(character) = chars.next() {
        if character == '\'' {
            result.push(character);
            if chars.peek() == Some(&'\'') {
                result.push(chars.next().unwrap());
            } else {
                quoted = !quoted;
            }
        } else if character != 't' || quoted {
            result.push(character);
        }
    }
    result.trim().into()
}

pub fn preview(id: &str, language: &str, region: &str, synchronized: bool, source: &str) -> String {
    static FORMATS: OnceLock<Mutex<HashMap<(String, String), String>>> = OnceLock::new();
    if !synchronized || id.is_empty() {
        return format!(
            "{{\"time\":\"\",\"date\":\"\",\"synchronized\":{synchronized},\"source\":{}}}",
            crate::keyboard::json_string(if synchronized { source } else { "" })
        );
    }
    let mut formats = FORMATS.get_or_init(Default::default).lock().unwrap();
    let formats = formats
        .entry((language.into(), region.into()))
        .or_insert_with(|| {
            crate::locale::selected_formats(language, region).unwrap_or_else(|_| "C".into())
        });
    let (time, date) = at(id, formats, QDateTime::current_msecs_since_epoch()).unwrap_or_default();
    format!(
        "{{\"time\":{},\"date\":{},\"synchronized\":true,\"source\":{}}}",
        crate::keyboard::json_string(&time),
        crate::keyboard::json_string(&date),
        crate::keyboard::json_string(source)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_uses_selected_zone_with_dst_and_date_rollover() {
        // 2026-07-01 23:30 UTC: New York is on daylight time, Tokyo is tomorrow.
        let instant = 1782948600000;
        let (new_york, date) = at("America/New_York", "en_GB.UTF-8", instant).unwrap();
        assert_eq!(new_york, "19:30");
        assert_eq!(date, "01/07/2026");
        let (tokyo, date) = at("Asia/Tokyo", "en_GB.UTF-8", instant).unwrap();
        assert_eq!(tokyo, "08:30");
        assert_eq!(date, "02/07/2026");
        let (us, _) = at("America/New_York", "en_US.UTF-8", instant).unwrap();
        assert!(
            us.contains("7:30") && !us.contains("7:30:") && us.contains("PM"),
            "{us}"
        );
        assert_eq!(
            at("Asia/Tokyo", "en_GB.UTF-8", instant + 59_999).unwrap().0,
            tokyo
        );
        assert_eq!(
            at("Asia/Tokyo", "en_GB.UTF-8", instant + 60_000).unwrap().0,
            "08:31"
        );
        assert!(at("../etc/passwd", "C", instant).is_none());
    }
    #[test]
    fn format_locale_names_preserve_latin_script_and_neutral_fallback() {
        assert_eq!(qt_locale("sr_RS.UTF-8@latin"), "sr_Latn_RS");
        assert_eq!(qt_locale("he_IL.UTF-8"), "he_IL");
        assert_eq!(qt_locale("C"), "C");
        assert_eq!(
            preview("", "en", "US", false, ""),
            "{\"time\":\"\",\"date\":\"\",\"synchronized\":false,\"source\":\"\"}"
        );
        assert_eq!(
            preview("Asia/Tokyo", "en", "US", false, "ignored"),
            preview("", "en", "US", false, "")
        );
        assert_eq!(without_zone_label("h:mm:ss AP tttt"), "h:mm:ss AP");
        assert_eq!(without_zone_label("HH:mm:ss 't' t"), "HH:mm:ss 't'");
        assert_eq!(without_zone_label("HH:mm:ss '' tt"), "HH:mm:ss ''");
    }
    #[test]
    fn every_language_and_regional_time_format_omits_seconds() {
        let instant = 1782948600000; // Exactly a minute boundary.
        let locales = crate::i18n::LANGUAGES
            .iter()
            .map(|code| crate::i18n::metadata(code).unwrap().locale)
            .chain(crate::i18n::regions().iter().map(|region| region.locale))
            .chain(std::iter::once("C"));
        for locale in locales {
            let first = at("Asia/Jerusalem", locale, instant).unwrap().0;
            let later = at("Asia/Jerusalem", locale, instant + 59_999).unwrap().0;
            assert!(!first.is_empty(), "{locale}");
            assert_eq!(first, later, "Seconds leaked into {locale}");
        }
    }
}
