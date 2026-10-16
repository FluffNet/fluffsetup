//! Read the installed xkeyboard-config gettext catalog without changing process
//! locale, adding gettext bindings, or confusing Chinese/Punjabi/Serbian scripts.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn parse_mo(bytes: &[u8]) -> Option<HashMap<String, String>> {
    let big = match bytes.get(..4)? {
        [0xde, 0x12, 0x04, 0x95] => false,
        [0x95, 0x04, 0x12, 0xde] => true,
        _ => return None,
    };
    let word = |offset: usize| -> Option<usize> {
        let value: [u8; 4] = bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
        Some(if big {
            u32::from_be_bytes(value)
        } else {
            u32::from_le_bytes(value)
        } as usize)
    };
    if word(4)? >> 16 != 0 {
        return None;
    }
    let count = word(8)?;
    let originals = word(12)?;
    let translations = word(16)?;
    let size = count.checked_mul(8)?;
    bytes.get(originals..originals.checked_add(size)?)?;
    bytes.get(translations..translations.checked_add(size)?)?;
    let string = |table: usize, index: usize| -> Option<&str> {
        let entry = table.checked_add(index.checked_mul(8)?)?;
        let length = word(entry)?;
        let offset = word(entry.checked_add(4)?)?;
        std::str::from_utf8(bytes.get(offset..offset.checked_add(length)?)?).ok()
    };
    let mut entries = HashMap::new();
    for index in 0..count {
        let original = string(originals, index)?;
        let translation = string(translations, index)?;
        // Singular entries only. Preserve contextual keys verbatim instead of
        // silently treating a contextual translation as an ordinary msgid.
        if !original.is_empty()
            && !translation.is_empty()
            && !original.contains('\0')
            && !translation.contains('\0')
        {
            entries.insert(original.to_string(), translation.to_string());
        }
    }
    Some(entries)
}

pub fn timezone_map_message(language: &str, context: &str, source: &str) -> String {
    static CATALOGS: OnceLock<Mutex<HashMap<String, HashMap<String, String>>>> = OnceLock::new();
    let mut catalogs = CATALOGS.get_or_init(Default::default).lock().unwrap();
    let entries = catalogs.entry(language.into()).or_insert_with(|| {
        let mut entries = HashMap::new();
        let chain = crate::i18n::language_fallbacks(language).unwrap_or_else(|| "en".into());
        for code in chain.split(':').rev() {
            if let Ok(bytes) = fs::read(
                Path::new("/usr/share/locale")
                    .join(code)
                    .join("LC_MESSAGES/plasmashellprivateplugin.mo"),
            ) && let Some(catalog) = parse_mo(&bytes)
            {
                entries.extend(catalog);
            }
        }
        entries
    });
    let key = if context.is_empty() {
        source.to_string()
    } else {
        format!("{context}\u{4}{source}")
    };
    entries.get(&key).cloned().unwrap_or_else(|| source.into())
}

fn load(language: &str, root: &Path) -> HashMap<String, String> {
    let mut entries = HashMap::new();
    let chain = crate::i18n::language_fallbacks(language).unwrap_or_else(|| "en".into());
    // Base first, then exact regional entries override it. An absent regional
    // catalog must not erase available translations in the matching base.
    for code in chain.split(':').rev() {
        if let Ok(bytes) = fs::read(root.join(code).join("LC_MESSAGES/xkeyboard-config.mo"))
            && let Some(catalog) = parse_mo(&bytes)
        {
            entries.extend(catalog);
        }
    }
    entries
}

pub fn catalog(language: &str) -> HashMap<String, String> {
    static CATALOGS: OnceLock<Mutex<HashMap<String, HashMap<String, String>>>> = OnceLock::new();
    let mut catalogs = CATALOGS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap();
    catalogs
        .entry(language.to_string())
        .or_insert_with(|| {
            let root = crate::preview_argument("keyboard-translations")
                .unwrap_or_else(|| "/usr/share/locale".into());
            load(language, Path::new(&root))
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(big: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [
            0x950412de_u32,
            0,
            2,
            28,
            44,
            0,
            0,
            6,
            60,
            6,
            67,
            12,
            74,
            10,
            87,
        ] {
            bytes.extend(if big {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            });
        }
        bytes.extend(b"French\0Hebrew\0");
        bytes.extend("צרפתית\0עברית\0".as_bytes());
        bytes
    }

    #[test]
    fn gettext_catalog_handles_endianness_and_rejects_corruption() {
        for big in [false, true] {
            let bytes = fixture(big);
            let entries = parse_mo(&bytes).unwrap();
            assert_eq!(entries.get("French").map(String::as_str), Some("צרפתית"));
            assert_eq!(entries.get("Hebrew").map(String::as_str), Some("עברית"));
            for length in 0..97 {
                assert!(parse_mo(&bytes[..length]).is_none());
            }
            let mut corrupt = bytes;
            corrupt[12..16].fill(0xff);
            assert!(parse_mo(&corrupt).is_none());
        }
    }
    #[test]
    fn gettext_context_is_preserved_instead_of_replacing_an_ordinary_message() {
        let mut bytes = fixture(false);
        // Same fixture size; contextual key and its translation remain exact.
        bytes[60..66].copy_from_slice(b"x\x04Name");
        let entries = parse_mo(&bytes).unwrap();
        assert!(entries.contains_key("x\u{4}Name"));
        assert!(!entries.contains_key("Name"));
        assert_eq!(
            timezone_map_message("en", "missing-context", "Missing message"),
            "Missing message"
        );
    }

    #[test]
    fn gettext_loading_follows_region_and_never_crosses_scripts() {
        let root =
            std::env::temp_dir().join(format!("fluffsetup-keyboard-names-{}", std::process::id()));
        for code in ["fr", "he", "zh_CN", "sr", "pa"] {
            let path = root.join(code).join("LC_MESSAGES");
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("xkeyboard-config.mo"), fixture(false)).unwrap();
        }
        assert!(load("fr_CA", &root).contains_key("French"));
        assert!(load("he", &root).contains_key("Hebrew"));
        for code in ["zh_Hant", "sr_Latn", "pa_Arab", "xx"] {
            assert!(load(code, &root).is_empty(), "{code}");
        }
        fs::remove_dir_all(root).unwrap();
    }
}
