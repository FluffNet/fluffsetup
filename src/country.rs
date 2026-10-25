//! Geographic regions, independently of interface language and locale dialect.
//! Reuse the installed ICU country names; no extra crate or C++ application code.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::{Mutex, OnceLock};

type Countries = unsafe extern "C" fn() -> *const *const c_char;
type DisplayCountry =
    unsafe extern "C" fn(*const c_char, *const c_char, *mut u16, i32, *mut i32) -> i32;
type LikelySubtags = unsafe extern "C" fn(*const c_char, *mut c_char, i32, *mut i32) -> i32;
type CanonicalZone = unsafe extern "C" fn(*const u16, i32, *mut u16, i32, *mut i8, *mut i32) -> i32;
type DateOpen = unsafe extern "C" fn(
    i32,
    i32,
    *const c_char,
    *const u16,
    i32,
    *const u16,
    i32,
    *mut i32,
) -> *mut c_void;
type DateFormat =
    unsafe extern "C" fn(*const c_void, f64, *mut u16, i32, *mut c_void, *mut i32) -> i32;
type DateClose = unsafe extern "C" fn(*mut c_void);

struct DateNames {
    canonical_zone: CanonicalZone,
    open: DateOpen,
    format: DateFormat,
    close: DateClose,
}

#[cfg_attr(target_os = "linux", link(name = "dl"))]
unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
}

struct Icu {
    countries: Countries,
    display_country: DisplayCountry,
    likely_subtags: Option<LikelySubtags>,
}

fn icu() -> Option<&'static Icu> {
    static ICU: OnceLock<Option<Icu>> = OnceLock::new();
    ICU.get_or_init(|| {
        #[cfg(target_os = "macos")]
        let libraries: Vec<String> = vec![
            "/opt/homebrew/opt/icu4c/lib/libicuuc.dylib".into(),
            "/usr/lib/libicucore.A.dylib".into(),
        ];
        #[cfg(not(target_os = "macos"))]
        let libraries: Vec<String> = std::iter::once("libicuuc.so".into())
            .chain((50..=99).rev().map(|v| format!("libicuuc.so.{v}")))
            .collect();
        for library in libraries {
            let library = CString::new(library).ok()?;
            // SAFETY: NUL-terminated name, RTLD_LAZY. A successful ICU handle
            // intentionally stays loaded for the process lifetime, including
            // every call through the cached function pointers below.
            let handle = unsafe { dlopen(library.as_ptr(), 1) };
            if handle.is_null() {
                continue;
            }
            for suffix in
                std::iter::once(String::new()).chain((50..=99).rev().map(|v| format!("_{v}")))
            {
                let countries = CString::new(format!("uloc_getISOCountries{suffix}")).unwrap();
                let display = CString::new(format!("uloc_getDisplayCountry{suffix}")).unwrap();
                // SAFETY: Both names are ICU's documented C APIs. Only matched
                // symbols from the same library/version are cast and called.
                let (countries, display) = unsafe {
                    (
                        dlsym(handle, countries.as_ptr()),
                        dlsym(handle, display.as_ptr()),
                    )
                };
                if !countries.is_null() && !display.is_null() {
                    let likely = CString::new(format!("uloc_addLikelySubtags{suffix}")).unwrap();
                    // SAFETY: Same library/version as the documented APIs above.
                    let likely = unsafe { dlsym(handle, likely.as_ptr()) };
                    return Some(Icu {
                        // SAFETY: Signatures match uloc.h (UChar=u16, UErrorCode=i32).
                        countries: unsafe {
                            std::mem::transmute::<*mut c_void, Countries>(countries)
                        },
                        display_country: unsafe {
                            std::mem::transmute::<*mut c_void, DisplayCountry>(display)
                        },
                        likely_subtags: if likely.is_null() {
                            None
                        } else {
                            // SAFETY: Signature matches uloc_addLikelySubtags in uloc.h.
                            Some(unsafe {
                                std::mem::transmute::<*mut c_void, LikelySubtags>(likely)
                            })
                        },
                    });
                }
            }
            // SAFETY: No function pointer escaped this unsuccessful handle.
            unsafe {
                dlclose(handle);
            }
        }
        None
    })
    .as_ref()
}

fn date_names() -> Option<&'static DateNames> {
    static NAMES: OnceLock<Option<DateNames>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            #[cfg(target_os = "macos")]
            let libraries: Vec<String> = vec![
                "/opt/homebrew/opt/icu4c/lib/libicui18n.dylib".into(),
                "/usr/lib/libicucore.A.dylib".into(),
            ];
            #[cfg(not(target_os = "macos"))]
            let libraries: Vec<String> = std::iter::once("libicui18n.so".into())
                .chain((50..=99).rev().map(|v| format!("libicui18n.so.{v}")))
                .collect();
            for library in libraries {
                let library = CString::new(library).ok()?;
                // SAFETY: NUL-terminated name, RTLD_LAZY. Successful handles stay
                // loaded while their function pointers are cached for this process.
                let handle = unsafe { dlopen(library.as_ptr(), 1) };
                if handle.is_null() {
                    continue;
                }
                for suffix in
                    std::iter::once(String::new()).chain((50..=99).rev().map(|v| format!("_{v}")))
                {
                    let symbol = |name: &str| {
                        let name = CString::new(format!("{name}{suffix}")).unwrap();
                        // SAFETY: Public ICU C APIs, all from one library/version.
                        unsafe { dlsym(handle, name.as_ptr()) }
                    };
                    let (canonical_zone, open, format, close) = (
                        symbol("ucal_getCanonicalTimeZoneID"),
                        symbol("udat_open"),
                        symbol("udat_format"),
                        symbol("udat_close"),
                    );
                    if [canonical_zone, open, format, close]
                        .iter()
                        .all(|symbol| !symbol.is_null())
                    {
                        // SAFETY: Signatures match udat.h/ucal.h (UDate=f64,
                        // UChar=u16, UBool=int8_t, UErrorCode=int32_t).
                        return Some(unsafe {
                            DateNames {
                                canonical_zone: std::mem::transmute::<*mut c_void, CanonicalZone>(
                                    canonical_zone,
                                ),
                                open: std::mem::transmute::<*mut c_void, DateOpen>(open),
                                format: std::mem::transmute::<*mut c_void, DateFormat>(format),
                                close: std::mem::transmute::<*mut c_void, DateClose>(close),
                            }
                        });
                    }
                }
                // SAFETY: No pointer escaped this unsuccessful handle.
                unsafe { dlclose(handle) };
            }
            None
        })
        .as_ref()
}

fn name_locale(language: &str) -> Option<String> {
    let base = crate::i18n::base_language(language)?;
    // Project/glibc choice IDs differ from ICU's locale IDs for these two.
    // Preserve every other language's explicit script and selected dialect.
    Some(match base {
        "quz" => format!("qu_{}", default_country(language)),
        "ca_valencia" => "ca_ES_VALENCIA".into(),
        _ => format!("{base}_{}", default_country(language)),
    })
}

pub fn codes() -> &'static [String] {
    static CODES: OnceLock<Vec<String>> = OnceLock::new();
    CODES.get_or_init(|| {
        let mut codes = Vec::new();
        if let Some(icu) = icu() {
            // SAFETY: ICU returns a static NUL-terminated array of C strings.
            let countries = unsafe { (icu.countries)() };
            if !countries.is_null() {
                for index in 0..512 {
                    // SAFETY: Stop at ICU's array sentinel before reading further.
                    let code = unsafe { *countries.add(index) };
                    if code.is_null() {
                        break;
                    }
                    // SAFETY: Each array entry is a NUL-terminated ISO country code.
                    codes.push(
                        unsafe { CStr::from_ptr(code) }
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        // The system zoneinfo table provides a usable English fallback if ICU
        // is missing. ICU is already a documented FluffSetup runtime dependency.
        if codes.is_empty() {
            if let Ok(table) = std::fs::read_to_string("/usr/share/zoneinfo/iso3166.tab") {
                codes.extend(
                    table
                        .lines()
                        .filter(|line| !line.starts_with('#'))
                        .filter_map(|line| line.split_once('\t').map(|(code, _)| code.to_string())),
                );
            }
            codes.extend(["US".into(), "IL".into()]);
        }
        codes.retain(|code| {
            code != "PS" && code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase())
        });
        codes.sort();
        codes.dedup();
        codes
    })
}

pub fn valid(code: &str) -> bool {
    codes().iter().any(|entry| entry == code)
}

/// CLDR's default language for a country, used only to rank existing glibc
/// formatting locales. Never invent an environment locale from ICU data.
pub(crate) fn format_language(code: &str) -> Option<String> {
    if !valid(code) {
        return None;
    }
    let likely = icu()?.likely_subtags?;
    let locale = CString::new(format!("und_{code}")).ok()?;
    let mut buffer = [0 as c_char; 256];
    let mut status = 0;
    // SAFETY: NUL-terminated input and output capacity in bytes, per uloc.h.
    let length = unsafe {
        likely(
            locale.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            &mut status,
        )
    };
    if status > 0 || length <= 0 || length as usize >= buffer.len() {
        return None;
    }
    // SAFETY: ICU NUL-terminates a successful result below the buffer capacity.
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_str()
        .ok()?
        .split('_')
        .next()
        .map(str::to_string)
}

pub fn default_country(language: &str) -> String {
    let country = crate::i18n::metadata(language)
        .and_then(|meta| meta.locale.split('_').nth(1))
        .unwrap_or("US")
        .split(['.', '@'])
        .next()
        .unwrap_or("US");
    if valid(country) {
        country.into()
    } else {
        "US".into()
    }
}

pub fn suggested(language: &str) -> Vec<String> {
    let mut result = vec![default_country(language)];
    for region in crate::i18n::ordered_regions(language) {
        let country = region
            .locale
            .split('_')
            .nth(1)
            .unwrap_or("")
            .split(['.', '@'])
            .next()
            .unwrap_or("");
        if valid(country) && !result.iter().any(|entry| entry == country) {
            result.push(country.into());
        }
    }
    result
}

pub(crate) fn display_name(code: &str, language: &str) -> String {
    // Preserve script choices such as Serbian Latin and Traditional Chinese.
    let locale = CString::new(name_locale(language).unwrap_or_else(|| "en_US".into())).unwrap();
    let country = CString::new(format!("und_{code}")).unwrap();
    if let Some(icu) = icu() {
        let mut buffer = [0u16; 512];
        let mut status = 0;
        // SAFETY: Valid C strings and a writable buffer of the declared size.
        let length = unsafe {
            (icu.display_country)(
                country.as_ptr(),
                locale.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as i32,
                &mut status,
            )
        };
        if status <= 0 && length > 0 && (length as usize) < buffer.len() {
            return String::from_utf16_lossy(&buffer[..length as usize]);
        }
    }
    // Reuse native names we already ship before falling back to zoneinfo.
    if let Some(region) = crate::i18n::ordered_regions(language)
        .into_iter()
        .find(|region| {
            region
                .locale
                .split('_')
                .nth(1)
                .is_some_and(|value| value.starts_with(code))
        })
    {
        return region.country.into();
    }
    std::fs::read_to_string("/usr/share/zoneinfo/iso3166.tab")
        .ok()
        .and_then(|table| {
            table.lines().find_map(|line| {
                line.split_once('\t')
                    .filter(|(entry, _)| *entry == code)
                    .map(|(_, name)| name.to_string())
            })
        })
        .unwrap_or_else(|| code.into())
}

/// CLDR exemplar-city names from the ICU data already installed with Qt/KDE.
/// ICU's public VVV formatter resolves aliases and nested locale inheritance;
/// directly walking zoneStrings loses names inherited by regional dialects.
/// Missing names stay searchable via the English IANA city, never transliteration.
pub(crate) fn timezone_city(zone: &str, language: &str) -> Option<String> {
    struct Formatter<'a> {
        pointer: *mut c_void,
        names: &'a DateNames,
    }
    impl Drop for Formatter<'_> {
        fn drop(&mut self) {
            if !self.pointer.is_null() {
                // SAFETY: Every owned formatter is closed exactly once.
                unsafe { (self.names.close)(self.pointer) };
            }
        }
    }
    let names = date_names()?;
    let locale = CString::new(name_locale(language)?).ok()?;
    // UTC is not a city. Never label it as ICU's "Unknown City" fallback.
    if zone == "UTC" || zone.starts_with("Etc/") || zone.len() > 128 || zone.contains('\0') {
        return None;
    }
    // CLDR deliberately retains older canonical IDs for localized names (e.g.
    // Asmera, Buenos_Aires, Saigon). Resolve aliases with the installed ICU's
    // public API, not a frozen hand-maintained list. Only this display lookup
    // changes: the actual IANA ID used by the map, recovery and finalization
    // stays untouched.
    let id = zone.encode_utf16().collect::<Vec<_>>();
    let mut zone = [0u16; 256];
    let mut known = 0;
    let mut status = 0;
    // SAFETY: Bounded UTF-16 input and writable output/flag/status pointers.
    let length = unsafe {
        (names.canonical_zone)(
            id.as_ptr(),
            id.len() as i32,
            zone.as_mut_ptr(),
            zone.len() as i32,
            &mut known,
            &mut status,
        )
    };
    if status > 0 || known == 0 || !(1..zone.len() as i32).contains(&length) {
        return None;
    }
    let zone = &zone[..length as usize];
    let pattern = [b'V' as u16; 3];
    status = 0;
    // SAFETY: UDAT_PATTERN=-2 selects the supplied pattern. The UTF-16 inputs
    // and their lengths remain valid through open; locale is NUL-terminated.
    let formatter = Formatter {
        pointer: unsafe {
            (names.open)(
                -2,
                -2,
                locale.as_ptr(),
                zone.as_ptr(),
                zone.len() as i32,
                pattern.as_ptr(),
                pattern.len() as i32,
                &mut status,
            )
        },
        names,
    };
    if status > 0 || formatter.pointer.is_null() {
        return None;
    }
    let mut buffer = [0u16; 512];
    // SAFETY: Valid live formatter and writable buffer with the declared size.
    // Only the exemplar-city field is formatted, so this fixed instant cannot
    // affect the label or change the system clock, timezone or process locale.
    let length = unsafe {
        (names.format)(
            formatter.pointer,
            0.0,
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            std::ptr::null_mut(),
            &mut status,
        )
    };
    if status > 0 || !(1..buffer.len() as i32).contains(&length) {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..length as usize]))
}

pub fn list_json(language: &str) -> String {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry(language.into())
        .or_insert_with(|| {
            let recommendations = suggested(language);
            let mut rows: Vec<_> = codes()
                .iter()
                .map(|code| (code, display_name(code, language)))
                .collect();
            rows.sort_by_key(|(code, name)| {
                (
                    recommendations
                        .iter()
                        .position(|entry| entry == *code)
                        .unwrap_or(usize::MAX),
                    crate::i18n::country_sort_key(name),
                )
            });
            let rows: Vec<_> = rows
                .into_iter()
                .map(|(code, name)| {
                    let group = if recommendations.contains(code) {
                        "regionSuggested"
                    } else {
                        "regionOther"
                    };
                    format!(
                        "{{\"code\":{},\"label\":{},\"english\":{},\"cycle\":{},\"group\":{}}}",
                        crate::keyboard::json_string(code),
                        crate::keyboard::json_string(&name),
                        crate::keyboard::json_string(&display_name(code, "en")),
                        crate::keyboard::json_string(&name),
                        crate::keyboard::json_string(&crate::i18n::translate_for(language, group))
                    )
                })
                .collect();
            format!("[{}]", rows.join(","))
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regions_are_unique_and_exclude_ps_even_in_recovery() {
        assert!(codes().len() >= 240);
        assert!(!valid("PS"));
        for obsolete in ["AN", "BU", "CS", "DD", "FX", "SU", "TP", "YD", "YU", "ZR"] {
            assert!(!valid(obsolete));
        }
        assert!(!valid("il"));
        assert!(valid("IL") && valid("US"));
        for language in crate::i18n::LANGUAGES {
            assert!(valid(&default_country(language)));
            let suggested = suggested(language);
            assert_eq!(suggested[0], default_country(language));
            assert!(suggested.iter().all(|code| valid(code)));
            let json = list_json(language);
            assert!(!json.contains("\"code\":\"PS\""));
            assert_eq!(json.matches("\"code\":").count(), codes().len());
        }
    }
    #[test]
    fn names_and_recommendations_follow_language_not_keyboard() {
        assert_eq!(suggested("he"), ["IL"]);
        assert_eq!(default_country("fr_CA"), "CA");
        assert_eq!(display_name("IL", "he"), "ישראל");
        assert_eq!(display_name("DE", "de"), "Deutschland");
        assert_ne!(display_name("DE", "fr"), display_name("DE", "de"));
        assert!(suggested("fr").contains(&"BE".into()));
    }
    #[test]
    fn timezone_names_resolve_cldr_aliases_without_changing_the_zone() {
        for (zone, city) in [
            ("Africa/Asmara", "אסמרה"),
            ("America/Argentina/Buenos_Aires", "בואנוס איירס"),
            ("Asia/Ho_Chi_Minh", "הו צ׳י מין סיטי"),
        ] {
            assert_eq!(timezone_city(zone, "he").as_deref(), Some(city), "{zone}");
        }
        let languages = crate::i18n::LANGUAGES
            .iter()
            .copied()
            .chain(crate::i18n::regions().iter().map(|region| region.code));
        for language in languages {
            for (modern, legacy) in [
                ("Africa/Asmara", "Africa/Asmera"),
                ("America/Argentina/Buenos_Aires", "America/Buenos_Aires"),
                ("America/Atikokan", "America/Coral_Harbour"),
                ("America/Indiana/Indianapolis", "America/Indianapolis"),
                ("Asia/Ho_Chi_Minh", "Asia/Saigon"),
                ("Asia/Kolkata", "Asia/Calcutta"),
                ("Asia/Kathmandu", "Asia/Katmandu"),
                ("Asia/Yangon", "Asia/Rangoon"),
                ("Atlantic/Faroe", "Atlantic/Faeroe"),
                ("Europe/Kyiv", "Europe/Kiev"),
                ("Pacific/Chuuk", "Pacific/Truk"),
                ("Pacific/Kanton", "Pacific/Enderbury"),
                ("Pacific/Pohnpei", "Pacific/Ponape"),
            ] {
                let city = timezone_city(modern, language);
                assert!(city.is_some(), "{language}: {modern}");
                assert_eq!(
                    city,
                    timezone_city(legacy, language),
                    "{language}: {modern}"
                );
            }
        }
    }
    #[test]
    fn city_names_preserve_dialect_script_and_locale_aliases() {
        for (language, id, city) in [
            ("fr_CA", "Europe/London", "Londres"),
            ("pt_PT", "Europe/Moscow", "Moscovo"),
            ("pt_BR", "Europe/Moscow", "Moscou"),
            ("sr", "Europe/Belgrade", "Београд"),
            ("sr_Latn", "Europe/Belgrade", "Beograd"),
            ("zh_Hans", "Asia/Tokyo", "东京"),
            ("zh_Hant", "Asia/Tokyo", "東京"),
        ] {
            assert_eq!(
                timezone_city(id, language).as_deref(),
                Some(city),
                "{language}"
            );
        }
        assert_eq!(name_locale("quz").as_deref(), Some("qu_PE"));
        assert_eq!(
            name_locale("ca_valencia").as_deref(),
            Some("ca_ES_VALENCIA")
        );
        for zone in ["UTC", "Etc/UTC", "Not/A_Real_Zone", "Asia/Tokyo\0junk"] {
            assert_eq!(timezone_city(zone, "he"), None, "{zone}");
        }
        assert_eq!(timezone_city("Asia/Tokyo", "not-a-language"), None);
    }
}
