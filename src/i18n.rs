//! Embedded UTF-8 catalogs. No runtime files or translation service required.

use std::env;
use std::sync::{Mutex, OnceLock};

const CATALOG: &str = include_str!("../translations/catalog.tsv");
const HOSTNAME_CATALOG: &str = include_str!("../translations/hostname-errors.tsv");
const LANGUAGE_METADATA: &str = include_str!("../translations/languages.tsv");
const REGIONS: &str = include_str!("../translations/regions.tsv");
const NEW_CATALOG: &str = include_str!("../translations/additional.tsv");
const WINDOWS_CATALOG: &str = include_str!("../translations/windows.tsv");
const KEYBOARD_CATALOG: &str = include_str!("../translations/keyboard.tsv");
const COUNTRY_CATALOG: &str = include_str!("../translations/country.tsv");
const TIMEZONE_CATALOG: &str = include_str!("../translations/timezone.tsv");
const REGIONAL_CATALOG: &str = include_str!("../translations/regional.tsv");

#[derive(Clone, Copy, Debug)]
pub struct Language {
    pub code: &'static str,
    pub english: &'static str,
    pub native: &'static str,
    pub title: &'static str,
    pub locale: &'static str,
}

pub fn metadata(code: &str) -> Option<Language> {
    let base = base_language(code)?;
    let region = regions().iter().find(|region| region.code == code);
    LANGUAGE_METADATA.lines().skip(1).find_map(|row| {
        let mut fields = row.split('\t');
        let language = Language {
            code: fields.next()?,
            english: fields.next()?,
            native: fields.next()?,
            title: fields.next()?,
            locale: fields.next()?,
        };
        (language.code == base).then_some(Language {
            code: region.map_or(language.code, |region| region.code),
            locale: region.map_or(language.locale, |region| region.locale),
            ..language
        })
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub base: &'static str,
    pub code: &'static str,
    pub locale: &'static str,
    pub country: &'static str,
}

pub fn regions() -> &'static [Region] {
    static REGIONAL_CHOICES: OnceLock<Vec<Region>> = OnceLock::new();
    REGIONAL_CHOICES
        .get_or_init(|| {
            REGIONS
                .lines()
                .skip(1)
                .filter_map(|row| {
                    let mut fields = row.split('\t');
                    let base = fields.next()?;
                    let locale = fields.next()?;
                    let country = fields.next()?;
                    Some(Region {
                        base,
                        code: locale.split('.').next()?,
                        locale,
                        country,
                    })
                })
                .collect()
        })
        .as_slice()
}

pub fn base_language(code: &str) -> Option<&'static str> {
    LANGUAGES
        .iter()
        .copied()
        .find(|base| *base == code)
        .or_else(|| {
            regions()
                .iter()
                .find(|region| region.code == code)
                .map(|region| region.base)
        })
}

pub fn ordered_regions(code: &str) -> Vec<Region> {
    let Some(language) = metadata(code) else {
        return Vec::new();
    };
    let base = base_language(code).unwrap();
    let default = metadata(base).unwrap().locale;
    let mut result: Vec<_> = regions()
        .iter()
        .copied()
        .filter(|region| region.base == base)
        .collect();
    // Native country names, with the language's established default first.
    result.sort_by_key(|region| (region.locale != default, country_sort_key(region.country)));
    debug_assert_eq!(language.native, metadata(base).unwrap().native);
    result
}

pub(crate) fn country_sort_key(country: &str) -> String {
    country
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

pub fn canonical_choice(code: &str) -> Option<&'static str> {
    if let Some(base) = LANGUAGES.iter().copied().find(|base| *base == code) {
        return Some(base);
    }
    let candidate = locale_identifier(code);
    if let Some(base) = LANGUAGES.iter().copied().find(|base| {
        metadata(base).is_some_and(|language| locale_identifier(language.locale) == candidate)
    }) {
        return Some(base);
    }
    regions()
        .iter()
        .find(|region| region.code == candidate)
        .map(|region| {
            if metadata(region.base).unwrap().locale == region.locale {
                region.base
            } else {
                region.code
            }
        })
}

/// Strip the encoding without losing a glibc script/dialect modifier.
pub fn locale_identifier(locale: &str) -> String {
    let (name, modifier) = locale.split_once('@').unwrap_or((locale, ""));
    let name = name.split('.').next().unwrap_or(name);
    if modifier.is_empty() {
        name.to_string()
    } else {
        format!("{name}@{modifier}")
    }
}

/// One user-selected language, not a list of catalog lookup candidates. Plasma
/// displays every colon-separated LANGUAGE element as a separate preference.
pub fn language_preference(code: &str) -> Option<String> {
    let code = canonical_choice(code)?;
    Some(
        match code {
            // Use established gettext/KDE identifiers, not internal script tags.
            "en" => "en_US",
            "pt" => "pt_PT",
            "zh_Hans" => "zh_CN",
            "zh_Hant" => "zh_TW",
            "sr_Latn" => "sr@latin",
            "pa_Arab" => "pa_PK",
            "ca_valencia" => "ca@valencia",
            other => other,
        }
        .into(),
    )
}

/// Internal lookup order for our own catalog reader only. Do not persist this
/// as LANGUAGE: fallback aliases are not additional user-selected languages.
pub fn language_fallbacks(code: &str) -> Option<String> {
    let language = metadata(code)?;
    let base = base_language(code)?;
    let regional = locale_identifier(language.locale);
    let mut chain = vec![regional.as_str(), base];
    // Regional Chinese inherits the matching script, never the other script.
    if base.starts_with("zh_") {
        chain = vec![
            regional.as_str(),
            if base == "zh_Hant" { "zh_TW" } else { "zh_CN" },
            base,
        ];
    }
    // Never fall back from a script-specific choice to the other script.
    if base == "sr_Latn" {
        chain = vec![regional.as_str(), "sr@latin", base];
    } else if base == "pa_Arab" {
        chain = vec![regional.as_str(), "pa@arabic", base];
    } else if base == "ca_valencia" {
        chain = vec![regional.as_str(), "ca@valencia", "ca", base];
    }
    chain.push("en");
    chain.dedup();
    Some(chain.join(":"))
}

pub fn ordered_languages() -> Vec<Language> {
    const FIRST: &[&str] = &["en", "es", "fr", "pt", "uk", "he", "ru", "ar", "de"];
    let mut languages: Vec<_> = LANGUAGES.iter().filter_map(|code| metadata(code)).collect();
    languages.sort_by_key(|language| {
        (
            FIRST
                .iter()
                .position(|code| *code == language.code)
                .unwrap_or(FIRST.len()),
            language.english,
        )
    });
    languages
}

fn current_language() -> &'static Mutex<&'static str> {
    static LANGUAGE: OnceLock<Mutex<&'static str>> = OnceLock::new();
    LANGUAGE.get_or_init(|| Mutex::new(environment_language()))
}

pub fn set_language(code: &str) -> bool {
    let Some(language) = canonical_choice(code) else {
        return false;
    };
    *current_language()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = language;
    true
}
pub const LANGUAGES: &[&str] = &[
    "as",
    "my",
    "ha",
    "ig",
    "kn",
    "km",
    "ku",
    "lo",
    "ml",
    "or",
    "om",
    "ps",
    "pa",
    "si",
    "so",
    "xh",
    "yo",
    "zu",
    "af",
    "sq",
    "am",
    "hy",
    "az",
    "eu",
    "be",
    "bs",
    "ca",
    "zh_Hans",
    "zh_Hant",
    "gl",
    "ka",
    "gu",
    "is",
    "kk",
    "mk",
    "mr",
    "mn",
    "ne",
    "nn",
    "ta",
    "te",
    "uz",
    "ar",
    "bg",
    "bn",
    "cs",
    "da",
    "de",
    "el",
    "en",
    "es",
    "et",
    "fa",
    "fi",
    "fil",
    "fr",
    "ga",
    "he",
    "hi",
    "hr",
    "hu",
    "id",
    "it",
    "ja",
    "ko",
    "lt",
    "lv",
    "ms",
    "mt",
    "nb",
    "nl",
    "pl",
    "pt",
    "ro",
    "ru",
    "sk",
    "sl",
    "sr",
    "sv",
    "sw",
    "th",
    "tr",
    "uk",
    "ur",
    "vi",
    "kok",
    "lb",
    "mi",
    "quz",
    "gd",
    "tt",
    "ug",
    "ca_valencia",
    "cy",
    "sr_Latn",
    "pa_Arab",
];

pub fn normalize(locale: &str) -> Option<&'static str> {
    if let Some(choice) = canonical_choice(locale) {
        return Some(choice);
    }
    let tag = locale.to_ascii_lowercase().replace('-', "_");
    if tag.starts_with("sr_latn") || (tag.starts_with("sr") && tag.contains("@latin")) {
        return Some("sr_Latn");
    }
    if tag.starts_with("pa_arab") || tag.starts_with("pa_pk") || tag.starts_with("pa@arabic") {
        return Some("pa_Arab");
    }
    if tag.starts_with("ca") && tag.contains("valencia") {
        return Some("ca_valencia");
    }
    if locale.starts_with("zh") {
        return Some(
            if locale.contains("Hant")
                || locale.contains("TW")
                || locale.contains("HK")
                || locale.contains("MO")
            {
                "zh_Hant"
            } else {
                "zh_Hans"
            },
        );
    }
    let base = locale
        .split(['_', '-', '.', '@'])
        .next()?
        .to_ascii_lowercase();
    let base = match base.as_str() {
        "iw" => "he",
        "tl" => "fil",
        "no" => "nb",
        other => other,
    };
    LANGUAGES.iter().copied().find(|language| *language == base)
}

pub fn language() -> &'static str {
    *current_language()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

fn environment_language() -> &'static str {
    for variable in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = env::var(variable) {
            if let Some(language) = value.split(':').find_map(normalize) {
                return language;
            }
            // C/POSIX explicitly requests the source language.
            if value == "C" || value == "POSIX" || value.starts_with("C.") {
                return "en";
            }
        }
    }
    "en"
}

pub fn rtl(language: &str) -> bool {
    matches!(
        base_language(language),
        Some("ar" | "fa" | "he" | "ur" | "ps" | "ug" | "pa_Arab")
    )
}

pub fn translate_for(language: &str, key: &str) -> String {
    let base = base_language(language).unwrap_or("en");
    let locale = metadata(language).map(|meta| meta.locale.split('.').next().unwrap());
    // Sparse regional overrides inherit the complete base-language catalog.
    for row in REGIONAL_CATALOG.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        if fields.len() == 3 && Some(fields[0]) == locale && fields[1] == key {
            return fields[2].replace("\\n", "\n");
        }
    }
    if key == "language" {
        return metadata(language)
            .unwrap_or_else(|| metadata("en").unwrap())
            .title
            .to_string();
    }
    for catalog in [
        TIMEZONE_CATALOG,
        COUNTRY_CATALOG,
        KEYBOARD_CATALOG,
        WINDOWS_CATALOG,
        NEW_CATALOG,
        CATALOG,
        HOSTNAME_CATALOG,
    ] {
        let mut lines = catalog.lines();
        let Some(index) = lines
            .next()
            .and_then(|header| header.split('\t').position(|field| field == key))
        else {
            continue;
        };
        let source = lines.clone().find(|line| line.starts_with("en\t"));
        let translated = lines.find(|line| line.split('\t').next() == Some(base));
        let Some(value) = translated
            .or(source)
            .and_then(|line| line.split('\t').nth(index))
        else {
            continue;
        };
        let text = value.replace("\\n", "\n");
        return if locale.is_some_and(|locale| locale == "de_CH" || locale == "de_LI") {
            text.replace('ß', "ss").replace('ẞ', "SS")
        } else {
            text
        };
    }
    key.to_string()
}

pub fn translate(key: &str) -> String {
    translate_for(language(), key)
}

#[cfg(test)]
mod tests {
    #[test]
    fn timezone_messages_cover_every_language_and_region() {
        let rows: Vec<_> = super::TIMEZONE_CATALOG.lines().skip(1).collect();
        assert_eq!(rows.len(), super::LANGUAGES.len());
        for language in super::LANGUAGES {
            assert_eq!(
                rows.iter()
                    .filter(|row| row.split('\t').next() == Some(language))
                    .count(),
                1
            );
            for key in [
                "timezoneTitle",
                "timezoneSearch",
                "timezoneEmpty",
                "timezoneMapUnavailable",
            ] {
                assert_ne!(super::translate_for(language, key), key);
                assert!(!super::translate_for(language, key).trim().is_empty());
            }
        }
        assert_eq!(super::translate_for("he", "timezoneTitle"), "אזורי זמן");
        for region in super::regions() {
            assert_ne!(
                super::translate_for(region.code, "timezoneTitle"),
                "timezoneTitle"
            );
        }
    }
    #[test]
    fn country_messages_cover_every_language() {
        let rows: Vec<_> = super::COUNTRY_CATALOG.lines().skip(1).collect();
        assert_eq!(rows.len(), super::LANGUAGES.len());
        for language in super::LANGUAGES {
            let row = rows
                .iter()
                .find(|row| row.split('\t').next() == Some(language))
                .unwrap();
            assert_eq!(row.split('\t').count(), 4);
            for key in ["regionTitle", "regionSuggested", "regionOther"] {
                assert_ne!(super::translate_for(language, key), key);
                assert!(!super::translate_for(language, key).is_empty());
            }
        }
    }
    use super::*;
    #[test]
    fn keyboard_messages_cover_every_language_and_region() {
        let keys: Vec<_> = KEYBOARD_CATALOG
            .lines()
            .next()
            .unwrap()
            .split('\t')
            .skip(1)
            .collect();
        let rows: Vec<_> = KEYBOARD_CATALOG.lines().skip(1).collect();
        assert_eq!(rows.len(), LANGUAGES.len());
        for language in LANGUAGES {
            let row = rows
                .iter()
                .find(|row| row.split('\t').next() == Some(language))
                .unwrap();
            assert_eq!(row.split('\t').count(), keys.len() + 1);
            for key in &keys {
                assert_ne!(translate_for(language, key), *key);
                assert!(!translate_for(language, key).trim().is_empty());
            }
        }
        for region in regions() {
            for key in &keys {
                assert_ne!(translate_for(region.code, key), *key);
            }
        }
    }

    #[test]
    fn picker_metadata_is_complete_safe_and_ordered() {
        let languages = ordered_languages();
        assert_eq!(languages.len(), LANGUAGES.len());
        let pinned: Vec<_> = languages
            .iter()
            .take(9)
            .map(|language| language.code)
            .collect();
        assert_eq!(
            pinned,
            ["en", "es", "fr", "pt", "uk", "he", "ru", "ar", "de"]
        );
        assert!(
            languages[9..]
                .windows(2)
                .all(|pair| pair[0].english < pair[1].english)
        );
        for language in languages {
            assert_eq!(normalize(language.locale), Some(language.code));
            assert!(
                language
                    .locale
                    .split('@')
                    .next()
                    .unwrap()
                    .ends_with(".UTF-8")
            );
            assert_eq!(translate_for(language.code, "language"), language.title);
            for value in [
                language.code,
                language.english,
                language.native,
                language.title,
                language.locale,
            ] {
                assert!(!value.is_empty());
                assert!(!value.contains(['"', '\\', '\n', '\r']));
            }
        }
    }

    #[test]
    fn every_language_has_every_message() {
        let columns = NEW_CATALOG.lines().next().unwrap().split('\t').count();
        assert_eq!(
            NEW_CATALOG.lines().skip(1).count() + WINDOWS_CATALOG.lines().skip(1).count(),
            LANGUAGES.len() - 43
        );
        for row in NEW_CATALOG
            .lines()
            .skip(1)
            .chain(WINDOWS_CATALOG.lines().skip(1))
        {
            assert_eq!(row.split('\t').count(), columns);
            assert!(row.split('\t').all(|field| !field.trim().is_empty()));
        }
        for catalog in [CATALOG, HOSTNAME_CATALOG] {
            let columns = catalog.lines().next().unwrap().split('\t').count();
            let rows: Vec<_> = catalog.lines().skip(1).collect();
            assert_eq!(rows.len(), 43);
            for language in LANGUAGES {
                if let Some(extended) =
                    [NEW_CATALOG, WINDOWS_CATALOG].into_iter().find(|extended| {
                        extended
                            .lines()
                            .skip(1)
                            .any(|row| row.split('\t').next() == Some(language))
                    })
                {
                    for key in catalog.lines().next().unwrap().split('\t').skip(1) {
                        let index = extended
                            .lines()
                            .next()
                            .unwrap()
                            .split('\t')
                            .position(|field| field == key)
                            .unwrap();
                        let row = extended
                            .lines()
                            .skip(1)
                            .find(|row| row.split('\t').next() == Some(language))
                            .unwrap();
                        assert!(
                            !row.split('\t').nth(index).unwrap().trim().is_empty(),
                            "{language}/{key}"
                        );
                    }
                    continue;
                }
                let row = rows
                    .iter()
                    .find(|row| row.split('\t').next() == Some(language))
                    .unwrap();
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), columns, "{language}");
                assert!(
                    fields.iter().all(|field| !field.trim().is_empty()),
                    "{language}"
                );
                assert!(!translate_for(language, "welcome").contains("welcome"));
            }
        }
        for language in LANGUAGES {
            assert_eq!(
                translate_for(language, "hostnameCharacters")
                    .matches("{characters}")
                    .count(),
                1
            );
        }
    }

    #[test]
    fn hebrew_brand_prefixes_have_no_separators() {
        assert!(translate_for("he", "welcome").contains("לFluff Linux"));
        assert!(translate_for("he", "welcomeBody").contains("בFluff Linux"));
        assert!(translate_for("he", "readyBody").contains("לFluff Linux"));
        for key in CATALOG.lines().next().unwrap().split('\t').skip(1) {
            let text = translate_for("he", key);
            for prefix in ["ל", "ב"] {
                for separator in ["־", "-", "–", "—", " "] {
                    assert!(!text.contains(&format!("{prefix}{separator}Fluff Linux")));
                }
            }
        }
    }

    #[test]
    fn locales_and_direction_are_normalized() {
        assert_eq!(normalize("he_IL.UTF-8"), Some("he"));
        assert_eq!(normalize("de_AT.UTF-8"), Some("de_AT"));
        assert_eq!(normalize("fil_PH"), Some("fil"));
        assert_eq!(normalize("tl_PH"), Some("fil"));
        assert_eq!(normalize("nb-NO"), Some("nb"));
        assert_eq!(normalize("zz"), None);
        for language in LANGUAGES {
            assert_eq!(
                rtl(language),
                ["ar", "fa", "he", "ur", "ps", "ug", "pa_Arab"].contains(language)
            );
        }
        assert_eq!(translate_for("zz", "next"), "Next");
    }

    #[test]
    fn script_and_dialect_choices_keep_their_locale_and_translation() {
        for (input, expected) in [
            ("sr_RS.UTF-8@latin", "sr_Latn"),
            ("sr_RS.utf8@latin", "sr_Latn"),
            ("sr-Latn-RS", "sr_Latn"),
            ("sr@latin", "sr_Latn"),
            ("sr_RS.UTF-8", "sr"),
            ("pa_PK.UTF-8", "pa_Arab"),
            ("pa-Arab-PK", "pa_Arab"),
            ("pa@arabic", "pa_Arab"),
            ("pa_IN.UTF-8", "pa"),
            ("ca_ES.UTF-8@valencia", "ca_valencia"),
            ("ca-ES-valencia", "ca_valencia"),
            ("ca@valencia", "ca_valencia"),
            ("ca_ES.UTF-8", "ca"),
        ] {
            assert_eq!(normalize(input), Some(expected), "{input}");
        }
        assert_eq!(
            language_fallbacks("sr_Latn").unwrap(),
            "sr_RS@latin:sr@latin:sr_Latn:en"
        );
        assert_eq!(
            language_fallbacks("pa_Arab").unwrap(),
            "pa_PK:pa@arabic:pa_Arab:en"
        );
        assert_eq!(
            language_fallbacks("ca_valencia").unwrap(),
            "ca_ES@valencia:ca@valencia:ca:ca_valencia:en"
        );
        assert!(translate_for("sr_Latn", "welcome").starts_with("Dobro došli"));
        assert!(translate_for("sr", "welcome").starts_with("Добро дошли"));
        assert!(rtl("pa_Arab"));
        assert!(rtl("ug"));
        assert!(!rtl("pa"));
        assert!(metadata("chr").is_none());
    }

    #[test]
    fn desktop_preferences_contain_only_the_chosen_language() {
        for (code, expected) in [
            ("he", "he"),
            ("he_IL.UTF-8", "he"),
            ("en", "en_US"),
            ("en_US", "en_US"),
            ("en_GB", "en_GB"),
            ("fr", "fr"),
            ("fr_FR", "fr"),
            ("fr_CA", "fr_CA"),
            ("de_AT", "de_AT"),
            ("pt", "pt_PT"),
            ("pt_BR", "pt_BR"),
            ("zh_Hans", "zh_CN"),
            ("zh_SG", "zh_SG"),
            ("zh_Hant", "zh_TW"),
            ("zh_HK", "zh_HK"),
            ("sr", "sr"),
            ("sr_Latn", "sr@latin"),
            ("pa", "pa"),
            ("pa_Arab", "pa_PK"),
            ("ca_valencia", "ca@valencia"),
        ] {
            assert_eq!(
                language_preference(code).as_deref(),
                Some(expected),
                "{code}"
            );
        }
        for language in ordered_languages().into_iter().chain(
            regions()
                .iter()
                .map(|region| metadata(region.code).unwrap()),
        ) {
            let preference = language_preference(language.code).unwrap();
            assert!(!preference.is_empty());
            assert_eq!(preference.split(':').count(), 1, "{}", language.code);
            assert!(
                preference
                    .bytes()
                    .all(|byte| { byte.is_ascii_alphanumeric() || b"_@".contains(&byte) })
            );
            assert_eq!(
                base_language(normalize(&preference).unwrap()),
                base_language(language.code),
                "{}: {preference}",
                language.code
            );
            let base = base_language(language.code).unwrap();
            if language.locale != metadata(base).unwrap().locale {
                assert_eq!(
                    preference, language.code,
                    "retain the explicitly selected regional variant"
                );
            }
        }
        assert!(language_preference("bad").is_none());
        assert!(language_preference("he:en_US").is_none());
    }

    #[test]
    fn english_picker_offers_a_short_variant_list_and_recovers_removed_locales() {
        let locales: Vec<_> = ordered_regions("en")
            .iter()
            .map(|region| region.locale)
            .collect();
        assert_eq!(
            locales,
            [
                "en_US.UTF-8",
                "en_AU.UTF-8",
                "en_CA.UTF-8",
                "en_IE.UTF-8",
                "en_NZ.UTF-8",
                "en_ZA.UTF-8",
                "en_GB.UTF-8",
            ]
        );
        for locale in ["en_IL.UTF-8", "en_DK.UTF-8", "en_SE.UTF-8"] {
            assert_eq!(normalize(locale), Some("en"));
            assert_eq!(
                metadata(normalize(locale).unwrap()).unwrap().locale,
                "en_US.UTF-8"
            );
        }
    }

    #[test]
    fn regional_choices_are_safe_complete_and_preserve_script_and_defaults() {
        let mut seen = std::collections::HashSet::new();
        for region in regions() {
            assert!(seen.insert(region.code));
            assert!(LANGUAGES.contains(&region.base));
            let code = canonical_choice(region.code).unwrap();
            assert_eq!(normalize(region.locale), Some(code));
            assert_eq!(base_language(code), Some(region.base));
            assert_eq!(metadata(code).unwrap().locale, region.locale);
            assert_eq!(rtl(code), rtl(region.base));
            assert!(region.locale.ends_with(".UTF-8"));
            assert!(
                region
                    .code
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            );
            assert!(!region.country.contains(['"', '\\', '\t', '\n']));
            for key in NEW_CATALOG.lines().next().unwrap().split('\t').skip(1) {
                assert_ne!(translate_for(code, key), key, "{code}/{key}");
            }
            assert_eq!(
                ordered_regions(code)[0].locale,
                metadata(region.base).unwrap().locale
            );
        }
        assert!(ordered_regions("he").is_empty());
        assert!(metadata("ar_PS").is_none());
        assert!(metadata("de_AT").is_some());
        assert!(metadata("fr_BE").is_some());
        assert!(metadata("nl_BE").is_some());
        assert_eq!(language_fallbacks("fr_CA").unwrap(), "fr_CA:fr:en");
        assert_eq!(
            language_fallbacks("zh_HK").unwrap(),
            "zh_HK:zh_TW:zh_Hant:en"
        );
        assert_eq!(language_fallbacks("zh_Hans").unwrap(), "zh_CN:zh_Hans:en");
        assert_eq!(translate_for("zh_HK", "name"), "姓名");
        assert_eq!(translate_for("pt_BR", "password"), "Senha");
        assert!(!translate_for("de_CH", "finish").contains('ß'));
        assert!(metadata("en;command").is_none());
    }
}
