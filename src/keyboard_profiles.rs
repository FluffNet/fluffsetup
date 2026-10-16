//! Windows-inspired input profiles mapped to installed Linux XKB keyboards.
//! Documented combinations are data, not country guesses embedded in UI code.
//! Unlisted combinations merge region and language profiles explicitly.

use crate::keyboard::{Layout, MAX_LAYOUTS};
use std::collections::HashMap;
use std::sync::OnceLock;

const PROFILES: &str = include_str!("../data/keyboard-profiles.tsv");
const REGIONS: &str = include_str!("../data/keyboard-regions.tsv");

fn profiles() -> &'static HashMap<(&'static str, &'static str), &'static str> {
    static CACHE: OnceLock<HashMap<(&str, &str), &str>> = OnceLock::new();
    CACHE.get_or_init(|| {
        PROFILES
            .lines()
            .skip(1)
            .filter_map(|line| {
                let mut fields = line.split('\t');
                Some(((fields.next()?, fields.next()?), fields.next()?))
            })
            .collect()
    })
}

fn regional_profile(country: &str) -> &'static str {
    static CACHE: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            REGIONS
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once('\t'))
                .collect()
        })
        .get(country)
        .copied()
        .unwrap_or("us")
}

fn locale_country(language: &str) -> &'static str {
    crate::i18n::metadata(language)
        .and_then(|meta| meta.locale.split('_').nth(1))
        .unwrap_or("US")
        .split(['.', '@'])
        .next()
        .unwrap_or("US")
}

fn native_profile(language: &str, installed: &[Layout]) -> Vec<String> {
    let base = crate::i18n::base_language(language).unwrap_or("en");
    resolve(
        profiles()
            .get(&(base, locale_country(language)))
            .or_else(|| profiles().get(&(base, "*")))
            .copied()
            .unwrap_or("us"),
        installed,
    )
}

pub fn native_primary(language: &str, installed: &[Layout]) -> Option<String> {
    native_profile(language, installed).into_iter().next()
}

fn resolve(selectors: &str, installed: &[Layout]) -> Vec<String> {
    let mut result = Vec::new();
    for selector in selectors.split(',') {
        // Alternatives must be explicitly script-compatible. Never fall back
        // from Serbian Latin to Cyrillic, or Punjabi to an Arabic-only layout.
        if let Some(id) = selector
            .split('|')
            .find(|id| installed.iter().any(|layout| layout.id == *id))
            && !result.iter().any(|entry| entry == id)
        {
            result.push(id.to_string());
        }
    }
    result
}

// These known keyboards can type the ASCII needed for hostnames and passwords.
// Being a Latin-script language alone is not sufficient (e.g. Azerbaijani).
pub fn ascii_capable(id: &str) -> bool {
    let (layout, variant) = id.split_once(':').unwrap_or((id, ""));
    match layout {
        "us" | "gb" | "ie" | "nz" | "au" | "be" | "fr" | "ca" | "de" | "ch" | "at" | "dk"
        | "fi" | "se" | "no" | "is" | "es" | "latam" | "pt" | "br" | "nl" | "it" | "pl" | "cz"
        | "sk" | "hu" | "ro" | "hr" | "ba" | "si" | "ee" | "lv" | "lt" | "al" | "tr" | "mt"
        | "ng" | "vn" | "jp" | "kr" => true,
        "rs" | "me" => variant.starts_with("latin"),
        "in" => variant == "eng",
        _ => false,
    }
}

fn push_unique(ids: &mut Vec<String>, id: &str) {
    if !ids.iter().any(|entry| entry == id) {
        ids.push(id.into());
    }
}

pub fn defaults(language: &str, country: &str, installed: &[Layout]) -> String {
    if !crate::country::valid(country) {
        return "us".into();
    }
    let base = crate::i18n::base_language(language).unwrap_or("en");
    let table = profiles();
    let native = native_profile(language, installed);
    let exact = table.get(&(base, country)).copied().or_else(|| {
        // The unqualified Windows entry describes its language's home region.
        // PT/Brazil is an explicit row because our Portuguese default is PT.
        (country == locale_country(base))
            .then(|| table.get(&(base, "*")).copied())
            .flatten()
    });
    let mut ids = resolve(
        exact.unwrap_or_else(|| regional_profile(country)),
        installed,
    );
    // Israel deliberately starts on U.S. English, regardless of UI language.
    // Regional Hebrew stays second; the selected language follows if needed.
    if country == "IL" {
        ids = resolve("us,il", installed);
    }
    let native_primary = native.first().cloned();
    if exact.is_none() && country != "IL" {
        // Prefer the user's language when that keyboard is already a regional
        // default (e.g. French in Algeria), without inventing another keyboard.
        let index = if base == "en" {
            ids.iter().position(|id| ascii_capable(id))
        } else {
            native_primary
                .as_ref()
                .and_then(|primary| ids.iter().position(|id| id == primary))
        };
        if let Some(index) = index {
            let primary = ids.remove(index);
            ids.insert(0, primary);
        }
    }
    let region_primary = ids.first().cloned();
    let native_required = native_primary.as_ref().is_some_and(|primary| {
        if base == "en" {
            return !ids.iter().any(|id| ascii_capable(id));
        }
        if country == "IL" || exact.is_none() {
            // A generic English keyboard is already covered by another ASCII
            // keyboard. A real localized keyboard (French, Russian, etc.) is not.
            primary != "us" || !ids.iter().any(|id| ascii_capable(id))
        } else {
            // Windows sometimes lists only English for an unsupported regional
            // locale. Do not lose the selected language's actual input keyboard.
            primary != "us"
                && !ids.contains(primary)
                && ids
                    .iter()
                    .all(|id| matches!(id.as_str(), "us" | "gb" | "in:eng"))
        }
    });
    if (exact.is_none() || country == "IL") && native_required {
        for (index, id) in native.iter().enumerate() {
            // Do not add a second Latin fallback when the region or selected
            // language already provides one. Non-Latin auxiliary profiles stay.
            if index > 0 && ascii_capable(id) && ids.iter().any(|entry| ascii_capable(entry)) {
                continue;
            }
            push_unique(&mut ids, id);
        }
    } else if native_required {
        push_unique(&mut ids, native_primary.as_ref().unwrap());
    }
    // A foreign language's native Latin keyboard can replace a region's U.S.
    // secondary. Never remove a regional primary or Israel's intentional U.S.
    // default; US International as a primary remains distinct and selectable.
    if exact.is_none()
        && country != "IL"
        && ids.iter().any(|id| {
            Some(id) != region_primary.as_ref()
                && id != "us"
                && id != "us:intl"
                && ascii_capable(id)
        })
    {
        ids.retain(|id| {
            !matches!(id.as_str(), "us" | "us:intl") || Some(id) == region_primary.as_ref()
        });
    }
    if !ids.iter().any(|id| ascii_capable(id)) && installed.iter().any(|layout| layout.id == "us") {
        push_unique(&mut ids, "us");
    }
    // TODO: integrate system IMEs in Fluff Linux before retiring this explicit
    // U.S. QWERTY fallback. Physical Japanese/Korean keyboards are not IMEs.
    // Apply only to automatic profiles; customized selections remain untouched.
    let ime_fallback = (matches!(base, "ja" | "ko" | "zh_Hans" | "zh_Hant")
        || ids
            .iter()
            .any(|id| matches!(id.split(':').next(), Some("jp" | "kr"))))
        && installed.iter().any(|layout| layout.id == "us");
    if ime_fallback {
        push_unique(&mut ids, "us");
    }
    if ids.is_empty() {
        return "us".into();
    }
    // XKB supports at most four groups. Reserve room for regional primary,
    // required UI-language input, ASCII and temporary IME fallback before
    // optional auxiliary profiles.
    let mut essential = Vec::new();
    if let Some(id) = region_primary {
        push_unique(&mut essential, &id);
    }
    if native_required && let Some(id) = native_primary {
        push_unique(&mut essential, &id);
    }
    if let Some(id) = ids.iter().find(|id| ascii_capable(id)) {
        push_unique(&mut essential, id);
    }
    if ime_fallback {
        push_unique(&mut essential, "us");
    }
    let mut result: Vec<String> = Vec::new();
    for id in &ids {
        let missing = essential.iter().filter(|id| !result.contains(*id)).count();
        if result.len() < MAX_LAYOUTS
            && (essential.contains(id) || MAX_LAYOUTS - result.len() > missing)
        {
            result.push(id.clone());
        }
    }
    result.join(",")
}

pub fn default_country(language: &str) -> &'static str {
    locale_country(language)
}

pub fn recovered_selection(
    saved: &str,
    customized: bool,
    language: &str,
    country: &str,
    installed: &[Layout],
) -> String {
    if customized && crate::keyboard::validate_selection(saved, installed).is_ok() {
        saved.into()
    } else {
        defaults(language, country, installed)
    }
}

pub fn required_ids() -> impl Iterator<Item = &'static str> {
    PROFILES
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').nth(2))
        .chain(
            REGIONS
                .lines()
                .skip(1)
                .filter_map(|line| line.split('\t').nth(1)),
        )
        .flat_map(|ids| ids.split(','))
        .flat_map(|alternatives| alternatives.split('|'))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<Layout> {
        crate::keyboard::parse_registry(include_str!("../tests/fixtures/keyboard-profiles.lst"))
    }
    #[test]
    fn windows_profile_data_covers_every_language_and_has_no_bad_ids() {
        let installed = fixture();
        assert_eq!(profiles().len(), PROFILES.lines().skip(1).count());
        for language in crate::i18n::LANGUAGES {
            assert!(profiles().contains_key(&(*language, "*")), "{language}");
        }
        for ((language, country), selectors) in profiles() {
            assert!(crate::i18n::LANGUAGES.contains(language));
            assert!(*country == "*" || crate::country::valid(country));
            assert!(!resolve(selectors, &installed).is_empty());
        }
        for id in required_ids() {
            assert!(crate::keyboard::parse_selection(id).is_some(), "{id}");
            assert!(installed.iter().any(|layout| layout.id == id), "{id}");
        }
    }
    #[test]
    fn every_language_region_pair_is_unique_installed_and_can_type_ascii() {
        let installed = fixture();
        let mut pairs = 0;
        for language in crate::i18n::LANGUAGES {
            for country in crate::country::codes() {
                let selected = defaults(language, country, &installed);
                crate::keyboard::validate_selection(&selected, &installed)
                    .unwrap_or_else(|error| panic!("{language}/{country}: {selected}: {error}"));
                let ids = crate::keyboard::parse_selection(&selected).unwrap();
                assert!(
                    ids.iter().any(|id| ascii_capable(id)),
                    "{language}/{country}: {selected}"
                );
                if matches!(*language, "ja" | "ko" | "zh_Hans" | "zh_Hant")
                    || ids.iter().any(|id| matches!(*id, "jp" | "kr"))
                {
                    assert!(ids.contains(&"us"), "{language}/{country}: {selected}");
                }
                if country == "IL" {
                    assert!(selected.starts_with("us,il"), "{language}: {selected}");
                    let primary = native_primary(language, &installed).unwrap();
                    if crate::i18n::base_language(language) != Some("en") && primary != "us" {
                        assert!(ids.contains(&primary.as_str()), "{language}: {selected}");
                    }
                }
                pairs += 1;
            }
        }
        assert_eq!(
            pairs,
            crate::i18n::LANGUAGES.len() * crate::country::codes().len()
        );
    }
    #[test]
    fn sensible_cross_language_cases_do_not_add_redundant_english() {
        let installed = fixture();
        for (language, country, expected) in [
            ("en", "BE", "be"),
            ("fr", "BE", "be"),
            ("de", "BE", "be"),
            ("ru", "IL", "us,il,ru"),
            ("ru", "RU", "ru,us"),
            ("uk", "IL", "us,il,ua"),
            ("he", "IL", "us,il"),
            ("en_GB", "IL", "us,il"),
            ("ru", "DE", "de,ru"),
            ("ru", "BE", "be,ru"),
            ("fr", "RU", "ru,fr"),
            ("en", "RU", "us,ru"),
            ("he", "US", "us,il"),
            ("de", "US", "us,de"),
            ("ar", "DZ", "ara:azerty,fr"),
            ("fr", "DZ", "fr,ara:azerty"),
            ("en", "NL", "us:intl"),
            ("nl", "NL", "us:intl"),
            ("fr", "CA", "ca"),
            ("en", "CH", "de"),
            ("de", "CH", "ch,ch:fr"),
            ("fr", "CH", "ch:fr,ch"),
            ("de", "IT", "us,de"),
            ("ru", "MD", "us,ru"),
            ("pa_Arab", "PK", "pk:urd-nla,us"),
            ("uz", "UZ", "us"),
            ("sr_Latn", "ME", "rs:latin"),
            ("sr", "ME", "rs,us:intl"),
        ] {
            assert_eq!(
                defaults(language, country, &installed),
                expected,
                "{language}/{country}"
            );
        }
    }
    #[test]
    fn four_group_limit_reserves_language_region_and_latin_input() {
        let installed = fixture();
        let selected = defaults("hy", "IL", &installed);
        assert_eq!(selected, "us,il,am:phonetic,am");
        let selected = defaults("ta", "AM", &installed);
        let ids = crate::keyboard::parse_selection(&selected).unwrap();
        assert_eq!(ids.len(), MAX_LAYOUTS);
        assert!(
            ids.contains(&"am:phonetic") && ids.contains(&"in:tamilnet") && ids.contains(&"in:eng")
        );
    }
    #[test]
    fn temporary_ime_fallback_is_unique_reserved_and_never_overrides_manual_choices() {
        let installed = fixture();
        for (language, country, expected) in [
            ("ja", "JP", "jp,us"),
            ("ko", "KR", "kr,us"),
            ("en", "JP", "jp,us"),
            ("en", "KR", "kr,us"),
            ("ja", "FR", "fr,jp,us"),
            ("ko", "FR", "fr,kr,us"),
            ("zh_Hans", "FR", "fr,us"),
            ("zh_Hant", "TW", "us"),
            ("ja", "IL", "us,il,jp"),
            ("hy", "JP", "jp,am:phonetic,am,us"),
            ("ja", "AM", "am:phonetic,am,jp,us"),
        ] {
            assert_eq!(defaults(language, country, &installed), expected);
        }
        assert_eq!(
            recovered_selection("jp", false, "ja", "JP", &installed),
            "jp,us"
        );
        for saved in ["jp", "us,jp", "de,jp"] {
            assert_eq!(
                recovered_selection(saved, true, "ja", "JP", &installed),
                saved
            );
        }
        let without_us: Vec<_> = installed
            .into_iter()
            .filter(|layout| layout.id != "us")
            .collect();
        assert_eq!(defaults("ja", "JP", &without_us), "jp");
        assert_eq!(defaults("ja", "FR", &without_us), "fr,jp");
    }
    #[test]
    fn unavailable_variants_do_not_change_script_and_saved_edits_survive() {
        let installed = crate::keyboard::parse_registry(
            "! layout\n us English\n rs Serbian\n de German\n be Belgian\n",
        );
        assert_eq!(defaults("sr_Latn", "RS", &installed), "us");
        assert_eq!(defaults("en", "BE", &installed), "be");
        assert_eq!(
            recovered_selection("be,us", false, "en", "BE", &installed),
            "be"
        );
        assert_eq!(
            recovered_selection("us,be,de", true, "en", "BE", &installed),
            "us,be,de"
        );
        assert_eq!(
            recovered_selection("us,be,de", true, "he", "IL", &installed),
            "us,be,de"
        );
        assert_eq!(
            recovered_selection("be,invalid", true, "en", "BE", &installed),
            "be"
        );
    }
}
