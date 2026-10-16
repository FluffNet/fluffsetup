//! Geographic regions, independently of interface language and locale dialect.
//! Reuse the installed ICU country names; no extra crate or C++ application code.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::{Mutex, OnceLock};

type Countries = unsafe extern "C" fn() -> *const *const c_char;
type DisplayCountry =
    unsafe extern "C" fn(*const c_char, *const c_char, *mut u16, i32, *mut i32) -> i32;
type LikelySubtags = unsafe extern "C" fn(*const c_char, *mut c_char, i32, *mut i32) -> i32;
type ResourceOpen = unsafe extern "C" fn(*const c_char, *const c_char, *mut i32) -> *mut c_void;
type ResourceKey =
    unsafe extern "C" fn(*const c_void, *const c_char, *mut c_void, *mut i32) -> *mut c_void;
type ResourceString =
    unsafe extern "C" fn(*const c_void, *const c_char, *mut i32, *mut i32) -> *const u16;
type ResourceClose = unsafe extern "C" fn(*mut c_void);

struct Resources {
    open: ResourceOpen,
    key: ResourceKey,
    string: ResourceString,
    close: ResourceClose,
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
    resources: Option<Resources>,
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
                    let symbol = |name: &str| {
                        let name = CString::new(format!("{name}{suffix}")).unwrap();
                        // SAFETY: This version's documented ICU C resource APIs.
                        unsafe { dlsym(handle, name.as_ptr()) }
                    };
                    let (open, key, string, close) = (
                        symbol("ures_open"),
                        symbol("ures_getByKey"),
                        symbol("ures_getStringByKey"),
                        symbol("ures_close"),
                    );
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
                        resources: if [open, key, string, close]
                            .iter()
                            .any(|pointer| pointer.is_null())
                        {
                            None
                        } else {
                            // SAFETY: Signatures match ures.h, same persistent handle.
                            Some(unsafe {
                                Resources {
                                    open: std::mem::transmute::<*mut c_void, ResourceOpen>(open),
                                    key: std::mem::transmute::<*mut c_void, ResourceKey>(key),
                                    string: std::mem::transmute::<*mut c_void, ResourceString>(
                                        string,
                                    ),
                                    close: std::mem::transmute::<*mut c_void, ResourceClose>(close),
                                }
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
    let base = crate::i18n::base_language(language).unwrap_or("en");
    // Preserve script choices such as Serbian Latin and Traditional Chinese.
    let locale = CString::new(format!("{base}_{}", default_country(language))).unwrap();
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
/// Missing names stay searchable via the English IANA city, never transliteration.
pub(crate) fn timezone_city(zone: &str, language: &str) -> Option<String> {
    struct Bundle<'a> {
        pointer: *mut c_void,
        resources: &'a Resources,
    }
    impl Drop for Bundle<'_> {
        fn drop(&mut self) {
            if !self.pointer.is_null() {
                // SAFETY: Every owned bundle is closed once, after copied strings.
                unsafe { (self.resources.close)(self.pointer) };
            }
        }
    }
    let resources = icu()?.resources.as_ref()?;
    let base = crate::i18n::base_language(language)?;
    let locale = CString::new(format!("{base}_{}", default_country(language))).ok()?;
    let zone = CString::new(zone.replace('/', ":")).ok()?;
    let mut status = 0;
    // SAFETY: ICU's documented zone resource package and NUL-terminated locale.
    let root = Bundle {
        pointer: unsafe {
            (resources.open)(c"ICUDATA-zone".as_ptr(), locale.as_ptr(), &mut status)
        },
        resources,
    };
    if status > 0 || root.pointer.is_null() {
        return None;
    }
    let zones = Bundle {
        pointer: unsafe {
            (resources.key)(
                root.pointer,
                c"zoneStrings".as_ptr(),
                std::ptr::null_mut(),
                &mut status,
            )
        },
        resources,
    };
    if status > 0 || zones.pointer.is_null() {
        return None;
    }
    let city = Bundle {
        pointer: unsafe {
            (resources.key)(
                zones.pointer,
                zone.as_ptr(),
                std::ptr::null_mut(),
                &mut status,
            )
        },
        resources,
    };
    if status > 0 || city.pointer.is_null() {
        return None;
    }
    let mut length = 0;
    // SAFETY: Borrowed UTF-16 remains valid until city bundle is closed below.
    let text =
        unsafe { (resources.string)(city.pointer, c"ec".as_ptr(), &mut length, &mut status) };
    if status > 0 || text.is_null() || !(1..=512).contains(&length) {
        return None;
    }
    Some(String::from_utf16_lossy(unsafe {
        std::slice::from_raw_parts(text, length as usize)
    }))
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
}
