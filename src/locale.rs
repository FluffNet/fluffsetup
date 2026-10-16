//! Language and geographic formats are persisted only during finalization.

use crate::i18n;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;

pub(crate) fn checked(command: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(arguments)
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| format!("Could not run {command}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{command} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn atomic_write(path: &Path, text: &str, mode: u32) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid configuration path.")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(
        ".{}.fluffsetup-{}.tmp",
        path.file_name()
            .ok_or("Invalid filename.")?
            .to_string_lossy(),
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(&temporary)
        .map_err(|error| format!("Could not create {}: {error}", temporary.display()))?;
    let result = (|| {
        file.write_all(text.as_bytes())
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn language_values(code: &str) -> Result<[(&'static str, String); 3], String> {
    let language = i18n::metadata(code).ok_or("Unsupported language.")?;
    Ok([
        ("LANG", language.locale.into()),
        ("LC_MESSAGES", language.locale.into()),
        (
            "LANGUAGE",
            i18n::language_fallbacks(code).ok_or("Unsupported language.")?,
        ),
    ])
}

const FORMAT_CATEGORIES: [&str; 8] = [
    "LC_TIME",
    "LC_MONETARY",
    "LC_NUMERIC",
    "LC_MEASUREMENT",
    "LC_PAPER",
    "LC_ADDRESS",
    "LC_NAME",
    "LC_TELEPHONE",
];

/// Use actual glibc UTF-8 locales, not every locale name accepted by Qt/ICU.
/// SUPPORTED/locale.gen includes locales we can generate; locale -a includes
/// precompiled locales installed by glibc-locales, even without source entries.
fn utf8_locales(sources: &str, generated: &str) -> Vec<String> {
    let mut locales = Vec::new();
    for (text, compiled) in [(sources, false), (generated, true)] {
        for line in text.lines() {
            let mut fields = line.trim().trim_start_matches('#').split_whitespace();
            let name = fields.next().unwrap_or_default();
            let encoding = fields.next().unwrap_or_default();
            let (_, modifier) = name.split_once('@').unwrap_or((name, ""));
            let utf8 = name
                .split('@')
                .next()
                .unwrap_or_default()
                .split_once('.')
                .is_some_and(|(_, value)| value.replace('-', "").eq_ignore_ascii_case("utf8"));
            if !(utf8 || (!compiled && encoding.eq_ignore_ascii_case("UTF-8")))
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.@-".contains(&byte))
            {
                continue;
            }
            let identifier = i18n::locale_identifier(name);
            let base = identifier.split('@').next().unwrap_or_default();
            if !base.contains('_') {
                continue;
            }
            locales.push(if modifier.is_empty() {
                format!("{base}.UTF-8")
            } else {
                format!("{base}.UTF-8@{modifier}")
            });
        }
    }
    locales.sort();
    locales.dedup();
    locales
}

fn format_locale(code: &str, region: &str, locales: &[String]) -> Result<String, String> {
    let language = i18n::metadata(code).ok_or("Unsupported language.")?;
    if !crate::country::valid(region) {
        return Err("Unsupported geographic region.".into());
    }
    let identifier = i18n::locale_identifier(language.locale);
    let (base, modifier) = identifier.split_once('@').unwrap_or((&identifier, ""));
    let language_name = base.split('_').next().unwrap_or_default();
    let native = crate::country::format_language(region);
    let result = locales
        .iter()
        .filter(|locale| {
            i18n::locale_identifier(locale)
                .split('@')
                .next()
                .unwrap_or_default()
                .split('_')
                .nth(1)
                == Some(region)
        })
        .min_by_key(|locale| {
            let candidate = i18n::locale_identifier(locale);
            let (base, candidate_modifier) = candidate.split_once('@').unwrap_or((&candidate, ""));
            let candidate_language = base.split('_').next().unwrap_or_default();
            let rank = if candidate == identifier {
                0
            } else if candidate_language == language_name && candidate_modifier == modifier {
                1
            } else if candidate_language == language_name && candidate_modifier.is_empty() {
                2
            } else if candidate_language == "en" && candidate_modifier.is_empty() {
                3
            } else if native.as_deref() == Some(candidate_language) && candidate_modifier.is_empty()
            {
                4
            } else if candidate_modifier.is_empty() {
                5
            } else {
                6
            };
            (rank, (*locale).clone())
        });
    // Explicit neutral fallback requested for countries absent from glibc.
    // Do not substitute a different country's currency or warn during setup.
    Ok(result.cloned().unwrap_or_else(|| "C".into()))
}

pub(crate) fn selected_formats(code: &str, region: &str) -> Result<String, String> {
    let mut sources = read_optional(Path::new("/usr/share/i18n/SUPPORTED"))?;
    sources.push('\n');
    sources.push_str(&read_optional(Path::new("/etc/locale.gen"))?);
    let generated = checked("/usr/bin/locale", &["-a"])?;
    format_locale(code, region, &utf8_locales(&sources, &generated))
}

fn locale_values(code: &str, formats: &str) -> Result<Vec<(&'static str, String)>, String> {
    let mut values = language_values(code)?.to_vec();
    values.extend(FORMAT_CATEGORIES.map(|key| (key, formats.to_string())));
    Ok(values)
}

fn environment_contents(existing: &str, code: &str, formats: &str) -> Result<String, String> {
    let values = locale_values(code, formats)?;
    let mut lines: Vec<_> = existing
        .lines()
        .filter(|line| {
            let key = line.split('=').next().unwrap_or_default().trim();
            key != "LC_ALL" && !values.iter().any(|(name, _)| *name == key)
        })
        .map(str::to_string)
        .collect();
    lines.extend(
        values
            .into_iter()
            .map(|(name, value)| format!("{name}={value}")),
    );
    Ok(lines.join("\n") + "\n")
}

pub(crate) fn ini_value(existing: &str, group: &str, key: &str, value: &str) -> String {
    let mut lines = Vec::new();
    let mut current_group = String::new();
    let mut inserted = false;
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if current_group == group && !inserted {
                lines.push(format!("{key}={value}"));
                inserted = true;
            }
            current_group = trimmed[1..trimmed.len() - 1].to_string();
        }
        if current_group == group && trimmed.split('=').next() == Some(key) {
            if !inserted {
                lines.push(format!("{key}={value}"));
                inserted = true;
            }
        } else {
            lines.push(line.to_string());
        }
    }
    if !inserted {
        if current_group != group {
            lines.push(format!("[{group}]"));
        }
        lines.push(format!("{key}={value}"));
    }
    lines.join("\n") + "\n"
}

fn locale_gen_contents(existing: &str, locale: &str) -> Result<String, String> {
    let region = i18n::locale_identifier(locale);
    let mut found = false;
    let lines: Vec<_> = existing
        .lines()
        .map(|line| {
            let entry = line.trim().trim_start_matches('#').trim();
            let mut fields = entry.split_whitespace();
            let name = fields.next().unwrap_or_default();
            let encoding = fields.next().unwrap_or_default();
            if i18n::locale_identifier(name) == region && encoding.eq_ignore_ascii_case("UTF-8") {
                found = true;
                entry.to_string()
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        return Err(format!(
            "The UTF-8 locale {locale} is not available in /etc/locale.gen."
        ));
    }
    Ok(lines.join("\n") + "\n")
}

fn generated_locale(existing: &str, locale: &str) -> bool {
    let canonical = |value: &str| value.to_ascii_lowercase().replace('-', "");
    existing
        .lines()
        .any(|line| canonical(line.trim()) == canonical(locale))
}

fn installed_locale(existing: &str, locale: &str) -> Result<bool, String> {
    if generated_locale(existing, locale) || locale == "C" {
        return Ok(true);
    }
    let identifier = i18n::locale_identifier(locale);
    if !existing.lines().any(|line| line.trim() == identifier) {
        return Ok(false);
    }
    // glibc-locales lists e.g. en_IL and ps_AF without an encoding, while
    // accepting en_IL.UTF-8 / ps_AF.UTF-8 as aliases. Do not assume every bare
    // locale is UTF-8: prove the requested name loads without warnings and
    // actually selects UTF-8, rather than falling back to C or an ISO encoding.
    let output = Command::new("/usr/bin/locale")
        .arg("charmap")
        .env("LC_ALL", locale)
        .output()
        .map_err(|error| format!("Could not check locale {locale}: {error}"))?;
    Ok(output.status.success()
        && output.stderr.is_empty()
        && String::from_utf8_lossy(&output.stdout)
            .trim()
            .eq_ignore_ascii_case("UTF-8"))
}

pub(crate) fn user_identity(passwd: &str, username: &str) -> Result<(u32, String), String> {
    crate::validate_username(username).map_err(str::to_string)?;
    account_identity(passwd, username)
}

// The temporary setup account is deliberately reserved from permanent-account
// helpers. Live session preferences need their own narrowly scoped entry point.
pub(crate) fn setup_session_identity(
    passwd: &str,
    username: &str,
) -> Result<(u32, String), String> {
    if username != crate::TEMPORARY_USER {
        return Err("Live keyboard selection requires the temporary FluffSetup account.".into());
    }
    account_identity(passwd, username)
}

fn account_identity(passwd: &str, username: &str) -> Result<(u32, String), String> {
    let row = passwd
        .lines()
        .find(|line| line.split(':').next() == Some(username))
        .ok_or("User does not exist.")?;
    let fields: Vec<_> = row.split(':').collect();
    let uid: u32 = fields
        .get(2)
        .ok_or("Invalid user record.")?
        .parse()
        .map_err(|_| "Invalid user ID.")?;
    let home = fields.get(5).ok_or("Invalid user record.")?;
    if uid < 1000 || *home != format!("/home/{username}") || fields.get(6) != Some(&"/bin/zsh") {
        return Err("Refusing to modify a protected account or unexpected home directory.".into());
    }
    Ok((uid, home.to_string()))
}

pub fn apply_user(code: &str, region: &str) -> Result<(), String> {
    let formats = selected_formats(code, region)?;
    // Resolve the real effective account, not an inherited HOME from sudo.
    let username = checked("/usr/bin/id", &["-un"])?;
    let passwd = fs::read_to_string("/etc/passwd").map_err(|error| error.to_string())?;
    let (_, home) = user_identity(&passwd, &username)?;
    write_user_configuration(Path::new(&home), code, &formats)
}

fn write_user_configuration(home: &Path, code: &str, formats: &str) -> Result<(), String> {
    let values = locale_values(code, formats)?;
    for relative in [
        ".config/locale.conf",
        ".config/environment.d/10-fluffsetup-language.conf",
    ] {
        let path = home.join(relative);
        let existing = read_optional(&path)?;
        atomic_write(
            &path,
            &environment_contents(&existing, code, formats)?,
            0o600,
        )?;
    }
    let path = home.join(".config/plasma-localerc");
    let mut text = read_optional(&path)?;
    text = ini_value(&text, "Formats", "LANG", &values[0].1);
    text = ini_value(&text, "Formats", "LC_MESSAGES", &values[1].1);
    text = ini_value(&text, "Translations", "LANGUAGE", &values[2].1);
    for category in FORMAT_CATEGORIES {
        text = ini_value(&text, "Formats", category, formats);
    }
    atomic_write(&path, &text, 0o600)
}

pub(crate) fn read_optional(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("Could not read {}: {error}", path.display())),
    }
}

pub fn apply_system_and_user(username: &str, code: &str, region: &str) -> Result<(), String> {
    if checked("/usr/bin/id", &["-u"])? != "0" {
        return Err("Language finalization must run as root.".into());
    }
    let language = i18n::metadata(code).ok_or("Unsupported language.")?;
    let formats = selected_formats(code, region)?;
    let passwd = fs::read_to_string("/etc/passwd").map_err(|error| error.to_string())?;
    let (uid, _) = user_identity(&passwd, username)?;
    let available = checked("/usr/bin/locale", &["-a"])?;
    let required = [language.locale, formats.as_str()];
    let mut missing = Vec::new();
    for locale in required {
        if !installed_locale(&available, locale)? && !missing.contains(&locale) {
            missing.push(locale);
        }
    }
    if !missing.is_empty() {
        let path = Path::new("/etc/locale.gen");
        let mut existing = fs::read_to_string(path).map_err(|error| error.to_string())?;
        for locale in &missing {
            existing = locale_gen_contents(&existing, locale)?;
        }
        atomic_write(path, &existing, 0o644)?;
        checked("/usr/bin/locale-gen", &[])?;
        let generated = checked("/usr/bin/locale", &["-a"])?;
        for locale in &missing {
            if !installed_locale(&generated, locale)? {
                return Err(format!("Locale generation did not produce {locale}."));
            }
        }
    }
    // User-owned preferences are written as that user, not as root. A malicious
    // home-directory symlink cannot redirect privileged configuration writes.
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let executable = executable.to_str().ok_or("Invalid executable path.")?;
    checked(
        "/usr/bin/runuser",
        &[
            "-u",
            username,
            "--",
            executable,
            "--apply-user-language",
            code,
            region,
        ],
    )?;
    if Path::new("/usr/share/dbus-1/system-services/org.freedesktop.Accounts.service").is_file() {
        checked(
            "/usr/bin/busctl",
            &[
                "call",
                "org.freedesktop.Accounts",
                "/org/freedesktop/Accounts",
                "org.freedesktop.Accounts",
                "FindUserByName",
                "s",
                username,
            ],
        )?;
        checked(
            "/usr/bin/busctl",
            &[
                "call",
                "org.freedesktop.Accounts",
                &format!("/org/freedesktop/Accounts/User{uid}"),
                "org.freedesktop.Accounts.User",
                "SetLanguage",
                "s",
                language.locale,
            ],
        )?;
    }
    // localectl validates LANGUAGE as one locale, rejecting gettext's colon-
    // separated fallback list. Persist the complete locale.conf atomically and
    // reload the manager's defaults, as localed does after writing this file.
    let contents = environment_contents(
        &read_optional(Path::new("/etc/locale.conf"))?,
        code,
        &formats,
    )?;
    persist_system_locale(Path::new("/etc/locale.conf"), &contents, checked)?;
    Ok(())
}

fn persist_system_locale(
    path: &Path,
    contents: &str,
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
) -> Result<(), String> {
    atomic_write(path, contents, 0o644)?;
    run("/usr/bin/systemctl", &["daemon-reload"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_persists_gettext_fallbacks_before_reloading_the_manager() {
        use std::os::unix::fs::PermissionsExt;

        let root =
            std::env::temp_dir().join(format!("fluffsetup-system-locale-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let path = root.join("locale.conf");
        let mut languages = i18n::ordered_languages();
        languages.extend(
            i18n::regions()
                .iter()
                .map(|region| i18n::metadata(region.code).unwrap()),
        );
        let expected_calls = languages.len() * 3;
        let mut calls = 0;
        for language in languages {
            // Include native formats, an independent geographic region and
            // the neutral fallback. No language may reintroduce SetLocale.
            for formats in [language.locale, "en_IL.UTF-8", "C"] {
                let fallbacks = i18n::language_fallbacks(language.code).unwrap();
                assert!(fallbacks.contains(':'), "{}: {fallbacks}", language.code);
                assert!(fallbacks.split(':').all(|part| {
                    !part.is_empty()
                        && part
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"_@-".contains(&byte))
                }));
                let previous =
                    "#keep\nLC_CTYPE=he_IL.UTF-8\nTZ=Europe/London\nLC_ALL=C\nLANGUAGE=stale:en\n";
                let contents = environment_contents(previous, language.code, formats).unwrap();
                persist_system_locale(&path, &contents, |command, arguments| {
                    // This runner would reject localectl or a LANGUAGE list
                    // sent as an argument to any locale-validating command.
                    assert_eq!(command, "/usr/bin/systemctl");
                    assert_eq!(arguments, &["daemon-reload"]);
                    let saved = fs::read_to_string(&path).unwrap();
                    assert_eq!(saved, contents);
                    for (name, value) in locale_values(language.code, formats).unwrap() {
                        assert_eq!(
                            saved
                                .lines()
                                .filter_map(|line| line.split_once('='))
                                .filter(|(key, _)| *key == name)
                                .collect::<Vec<_>>(),
                            [(name, value.as_str())],
                            "{}/{formats}/{name}",
                            language.code
                        );
                        if name != "LANGUAGE" {
                            assert!(
                                !value.contains(':'),
                                "{name} is one POSIX locale, not a fallback list"
                            );
                        }
                    }
                    assert!(saved.contains(&format!("LANGUAGE={fallbacks}\n")));
                    assert!(saved.contains("LC_CTYPE=he_IL.UTF-8\n"));
                    assert!(saved.contains("TZ=Europe/London\n"));
                    assert!(!saved.contains("LC_ALL="));
                    assert_eq!(
                        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                        0o644
                    );
                    calls += 1;
                    Ok(String::new())
                })
                .unwrap();
            }
        }
        assert_eq!(calls, expected_calls);
        eprintln!("Validated {calls} system-locale writes with gettext fallback lists.");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires Fluff glibc-locales; read-only locale checks, no system settings applied"]
    fn every_installed_language_profile_loads_without_locale_warnings() {
        let system_before = fs::read("/etc/locale.conf").unwrap();
        let timezone_before = fs::read("/etc/localtime").unwrap();
        let available = checked("/usr/bin/locale", &["-a"]).unwrap();
        let mut languages = i18n::ordered_languages();
        languages.extend(
            i18n::regions()
                .iter()
                .map(|region| i18n::metadata(region.code).unwrap()),
        );
        let mut profiles = 0;
        for language in languages {
            assert!(
                installed_locale(&available, language.locale).unwrap(),
                "{}: missing interface locale {}",
                language.code,
                language.locale
            );
            for formats in [language.locale, "en_IL.UTF-8", "C"] {
                let mut command = Command::new("/usr/bin/locale");
                command.env_clear().env("PATH", "/usr/bin");
                for (key, value) in locale_values(language.code, formats).unwrap() {
                    command.env(key, value);
                }
                let output = command
                    .args(["-k", "charmap", "d_fmt", "int_curr_symbol", "country_ab2"])
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}/{formats}: {:?}",
                    language.code,
                    output.status
                );
                assert!(
                    output.stderr.is_empty(),
                    "{}/{formats}: {}",
                    language.code,
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(
                    String::from_utf8(output.stdout)
                        .unwrap()
                        .contains("charmap=\"UTF-8\""),
                    "{}/{formats}",
                    language.code
                );
                profiles += 1;
            }
            eprintln!(
                "PASS {}: {} / {}",
                language.code,
                language.locale,
                i18n::language_fallbacks(language.code).unwrap()
            );
        }
        assert_eq!(fs::read("/etc/locale.conf").unwrap(), system_before);
        assert_eq!(fs::read("/etc/localtime").unwrap(), timezone_before);
        eprintln!(
            "Loaded {profiles} language/format profiles without locale warnings; system settings unchanged."
        );
    }

    #[test]
    fn locale_generation_never_enables_the_wrong_script_or_dialect() {
        let source = "#sr_RS UTF-8\n#sr_RS@latin UTF-8\n#ca_ES.UTF-8 UTF-8\n#ca_ES@valencia UTF-8\n#pa_IN UTF-8\n#pa_PK UTF-8\n";
        let latin = locale_gen_contents(source, "sr_RS.UTF-8@latin").unwrap();
        assert!(latin.contains("\nsr_RS@latin UTF-8\n"));
        assert!(latin.starts_with("#sr_RS UTF-8\n"));
        let valencian = locale_gen_contents(source, "ca_ES.UTF-8@valencia").unwrap();
        assert!(valencian.contains("\nca_ES@valencia UTF-8\n"));
        assert!(valencian.contains("\n#ca_ES.UTF-8 UTF-8\n"));
        let punjabi = locale_gen_contents(source, "pa_PK.UTF-8").unwrap();
        assert!(punjabi.contains("\npa_PK UTF-8\n"));
        assert!(punjabi.contains("\n#pa_IN UTF-8\n"));
        assert!(locale_gen_contents("#sr_RS UTF-8\n", "sr_RS.UTF-8@latin").is_err());
        assert!(generated_locale("sr_RS.utf8@latin\n", "sr_RS.UTF-8@latin"));
        assert!(!generated_locale("sr_RS.utf8\n", "sr_RS.UTF-8@latin"));
        let values = environment_contents(
            "LC_TIME=en_GB.UTF-8\n",
            "ca_valencia",
            "ca_ES.UTF-8@valencia",
        )
        .unwrap();
        assert!(values.contains("LANG=ca_ES.UTF-8@valencia\n"));
        assert!(values.contains("LC_TIME=ca_ES.UTF-8@valencia\n"));
    }

    #[test]
    fn user_files_are_atomic_idempotent_and_switch_languages_without_touching_system_files() {
        use std::os::unix::fs::PermissionsExt;
        let home =
            std::env::temp_dir().join(format!("fluffsetup-locale-test-{}", std::process::id()));
        // This dedicated test directory is created exclusively, not reused.
        fs::create_dir(&home).unwrap();
        let plasma = home.join(".config/plasma-localerc");
        fs::create_dir(home.join(".config")).unwrap();
        fs::write(
            &plasma,
            "[Formats]\nLANG=en_US.UTF-8\nLC_TIME=en_GB.UTF-8\n[Other]\nKey=value\n",
        )
        .unwrap();
        let mut languages = i18n::ordered_languages();
        languages.extend(
            i18n::regions()
                .iter()
                .map(|region| i18n::metadata(region.code).unwrap()),
        );
        for language in languages {
            write_user_configuration(&home, language.code, "en_IL.UTF-8").unwrap();
            let first = fs::read_to_string(&plasma).unwrap();
            write_user_configuration(&home, language.code, "en_IL.UTF-8").unwrap();
            assert_eq!(fs::read_to_string(&plasma).unwrap(), first);
            assert_eq!(first.matches("LANG=").count(), 1);
            assert_eq!(first.matches("LANGUAGE=").count(), 1);
            for category in FORMAT_CATEGORIES {
                assert!(first.contains(&format!("{category}=en_IL.UTF-8\n")));
                assert_eq!(first.matches(&format!("{category}=")).count(), 1);
            }
            assert!(first.contains("[Other]\nKey=value"));
            assert!(first.contains(&format!("LANG={}\n", language.locale)));
            assert_eq!(
                fs::metadata(&plasma).unwrap().permissions().mode() & 0o777,
                0o600
            );
            for relative in [
                ".config/locale.conf",
                ".config/environment.d/10-fluffsetup-language.conf",
            ] {
                let environment = fs::read_to_string(home.join(relative)).unwrap();
                assert!(environment.contains(&format!("LC_MESSAGES={}\n", language.locale)));
                assert!(!environment.contains("LC_ALL="));
                for category in FORMAT_CATEGORIES {
                    assert!(environment.contains(&format!("{category}=en_IL.UTF-8\n")));
                    assert_eq!(environment.matches(&format!("{category}=")).count(), 1);
                }
            }
        }
        let before = fs::read_to_string(&plasma).unwrap();
        assert!(write_user_configuration(&home, "bad", "C").is_err());
        assert_eq!(fs::read_to_string(&plasma).unwrap(), before);
        fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn locale_generation_preserves_other_entries_and_handles_glibc_names() {
        assert_eq!(
            locale_gen_contents("#en_US.UTF-8 UTF-8\n#hi_IN UTF-8\n", "hi_IN.UTF-8").unwrap(),
            "#en_US.UTF-8 UTF-8\nhi_IN UTF-8\n"
        );
        assert!(locale_gen_contents("#de_DE.UTF-8 UTF-8", "he_IL.UTF-8").is_err());
        assert!(generated_locale("C\nhe_IL.utf8\n", "he_IL.UTF-8"));
        assert!(!generated_locale("he_IL.iso88598", "he_IL.UTF-8"));
    }
    #[test]
    fn user_and_system_configuration_preserve_unrelated_preferences() {
        let result = environment_contents(
            "#keep\nLANG=en_US.UTF-8\nLC_TIME=en_GB.UTF-8\nLC_NUMERIC=C\nLC_MONETARY=C\nLC_MEASUREMENT=C\nLC_PAPER=de_DE.UTF-8\nLC_ADDRESS=C\nLC_NAME=C\nLC_TELEPHONE=C\nLC_TELEPHONE=en_GB.UTF-8\nLC_CTYPE=he_IL.UTF-8\nLC_COLLATE=he_IL.UTF-8\nTZ=Europe/London\nLC_ALL=C\nLANGUAGE=en\n",
            "he",
            "en_US.UTF-8",
        )
        .unwrap();
        for category in FORMAT_CATEGORIES {
            assert!(result.contains(&format!("{category}=en_US.UTF-8\n")));
            assert_eq!(result.matches(&format!("{category}=")).count(), 1);
        }
        assert!(result.contains("LC_CTYPE=he_IL.UTF-8\n"));
        assert!(result.contains("LC_COLLATE=he_IL.UTF-8\n"));
        assert!(result.contains("TZ=Europe/London\n"));
        assert!(result.contains("LANG=he_IL.UTF-8\n"));
        assert!(result.contains("LC_MESSAGES=he_IL.UTF-8\n"));
        assert!(result.contains("LANGUAGE=he_IL:he:en\n"));
        assert!(!result.contains("LC_ALL="));
        let result = ini_value(
            "[Formats]\nLANG=en_US.UTF-8\nLC_TIME=en_GB.UTF-8\n[Other]\nKey=value\n",
            "Translations",
            "LANGUAGE",
            "he:en",
        );
        assert!(result.contains("[Other]\nKey=value"));
        assert!(result.contains("[Translations]\nLANGUAGE=he:en"));
        assert_eq!(
            ini_value(&result, "Translations", "LANGUAGE", "fr:en")
                .matches("LANGUAGE=")
                .count(),
            1
        );
        assert!(environment_contents("", "en;rm", "C").is_err());
    }
    #[test]
    fn format_locales_are_region_specific_without_replacing_interface_language() {
        let locales = utf8_locales(
            "en_US.UTF-8 UTF-8\nen_GB.UTF-8 UTF-8\nen_IL UTF-8\nhe_IL.UTF-8 UTF-8\nfr_FR.UTF-8 UTF-8\nfr_BE.UTF-8 UTF-8\nnl_BE.UTF-8 UTF-8\nde_BE.UTF-8 UTF-8\nde_DE.UTF-8 UTF-8\ncs_CZ.UTF-8 UTF-8\nja_JP.UTF-8 UTF-8\nru_RU.UTF-8 UTF-8\nsr_RS UTF-8\nsr_RS@latin UTF-8\nca_ES.UTF-8 UTF-8\nca_ES@valencia UTF-8\n",
            "C\nPOSIX\nen_CA.utf8\nen_US.utf8\nhe_IL.iso88598\n",
        );
        for (language, region, expected) in [
            ("en", "IL", "en_IL.UTF-8"),
            ("ru", "IL", "en_IL.UTF-8"),
            ("he", "US", "en_US.UTF-8"),
            ("en", "DE", "de_DE.UTF-8"),
            ("fr", "BE", "fr_BE.UTF-8"),
            ("en", "BE", "nl_BE.UTF-8"),
            ("cs", "JP", "ja_JP.UTF-8"),
            ("ja", "US", "en_US.UTF-8"),
            ("en", "GB", "en_GB.UTF-8"),
            ("en", "CA", "en_CA.UTF-8"),
            ("sr_Latn", "RS", "sr_RS.UTF-8@latin"),
            ("ca_valencia", "ES", "ca_ES.UTF-8@valencia"),
            ("en", "BB", "C"),
        ] {
            let formats = format_locale(language, region, &locales).unwrap();
            assert_eq!(formats, expected, "{language}/{region}");
            let environment = environment_contents("", language, &formats).unwrap();
            let interface = i18n::metadata(language).unwrap().locale;
            assert!(environment.contains(&format!("LANG={interface}\n")));
            assert!(environment.contains(&format!("LC_MESSAGES={interface}\n")));
            for category in FORMAT_CATEGORIES {
                assert!(environment.contains(&format!("{category}={expected}\n")));
            }
            assert!(!environment.contains("TZ="));
        }
        assert!(format_locale("he", "PS", &locales).is_err());
        assert!(format_locale("he", "IL;bad", &locales).is_err());
        assert!(format_locale("bad", "IL", &locales).is_err());
    }

    #[test]
    fn format_inventory_does_not_invent_locales_or_accept_non_utf8_data() {
        let locales = utf8_locales(
            "# en_IL UTF-8\nhe_IL.ISO-8859-8 ISO-8859-8\n#sr_RS@latin UTF-8\n# en_GB.UTF-8 UTF-8\nnot_a_locale;bad.UTF-8 UTF-8\n",
            "C\nPOSIX\nhe_IL.iso88598\nsr_RS.utf8@latin\nde_DE.utf8\n",
        );
        assert_eq!(
            locales,
            [
                "de_DE.UTF-8",
                "en_GB.UTF-8",
                "en_IL.UTF-8",
                "sr_RS.UTF-8@latin"
            ]
        );
        assert_eq!(format_locale("he", "IL", &locales).unwrap(), "en_IL.UTF-8");
        assert_eq!(format_locale("he", "IL", &[]).unwrap(), "C");
    }

    #[test]
    fn neutral_formats_are_explicit_and_preserve_language_and_other_categories() {
        let result = environment_contents(
            "LC_TIME=en_US.UTF-8\nLC_TIME=de_DE.UTF-8\nLC_CTYPE=he_IL.UTF-8\n",
            "he",
            "C",
        )
        .unwrap();
        for category in FORMAT_CATEGORIES {
            assert_eq!(result.matches(&format!("{category}=")).count(), 1);
            assert!(result.contains(&format!("{category}=C\n")));
        }
        assert!(result.contains("LC_MESSAGES=he_IL.UTF-8\n"));
        assert!(result.contains("LC_CTYPE=he_IL.UTF-8\n"));
    }

    #[test]
    fn generation_enables_both_interface_and_format_locales_without_other_changes() {
        let source = "#en_US.UTF-8 UTF-8\n#en_IL UTF-8\n#he_IL.UTF-8 UTF-8\n#de_DE.UTF-8 UTF-8\n";
        let updated = locale_gen_contents(source, "en_US.UTF-8").unwrap();
        let updated = locale_gen_contents(&updated, "en_IL.UTF-8").unwrap();
        assert_eq!(
            updated,
            "en_US.UTF-8 UTF-8\nen_IL UTF-8\n#he_IL.UTF-8 UTF-8\n#de_DE.UTF-8 UTF-8\n"
        );
        assert_eq!(
            locale_gen_contents(&updated, "en_IL.UTF-8").unwrap(),
            updated
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires the Fluff glibc-locales inventory; reads only, never applies settings"]
    fn installed_region_formats_match_glibc_categories() {
        let system_before = read_optional(Path::new("/etc/locale.conf")).unwrap();
        let timezone_before = fs::read("/etc/localtime").unwrap();
        let sources = fs::read_to_string("/usr/share/i18n/SUPPORTED").unwrap();
        let available = checked("/usr/bin/locale", &["-a"]).unwrap();
        let locales = utf8_locales(&sources, &available);
        let installed: std::collections::HashSet<_> = locales
            .iter()
            .filter(|locale| installed_locale(&available, locale).unwrap())
            .cloned()
            .collect();
        let mut languages = i18n::ordered_languages();
        languages.extend(
            i18n::regions()
                .iter()
                .map(|region| i18n::metadata(region.code).unwrap()),
        );
        let mut combinations = 0;
        for language in languages {
            for region in crate::country::codes() {
                let formats = format_locale(language.code, region, &locales).unwrap();
                assert!(
                    formats == "C" || installed.contains(&formats),
                    "{formats} is not installed"
                );
                if formats != "C" {
                    assert_eq!(
                        i18n::locale_identifier(&formats)
                            .split('@')
                            .next()
                            .unwrap()
                            .split('_')
                            .nth(1),
                        Some(region.as_str())
                    );
                }
                let environment = environment_contents("", language.code, &formats).unwrap();
                assert!(environment.contains(&format!("LANG={}\n", language.locale)));
                assert!(environment.contains(&format!("LC_MESSAGES={}\n", language.locale)));
                for category in FORMAT_CATEGORIES {
                    assert!(environment.contains(&format!("{category}={formats}\n")));
                }
                combinations += 1;
            }
        }
        eprintln!("Validated {combinations} language/region formatting combinations.");
        for (language, region, expected, date, time, decimal, currency, measurement) in [
            (
                "en",
                "IL",
                "en_IL.UTF-8",
                "%d/%m/%y",
                "%T",
                ".",
                "ILS ",
                "1",
            ),
            (
                "ru",
                "IL",
                "en_IL.UTF-8",
                "%d/%m/%y",
                "%T",
                ".",
                "ILS ",
                "1",
            ),
            (
                "he",
                "US",
                "en_US.UTF-8",
                "%m/%d/%Y",
                "%r",
                ".",
                "USD ",
                "2",
            ),
            (
                "en",
                "DE",
                "de_DE.UTF-8",
                "%d.%m.%Y",
                "%T",
                ",",
                "EUR ",
                "1",
            ),
            (
                "fr",
                "BE",
                "fr_BE.UTF-8",
                "%d/%m/%y",
                "%T",
                ",",
                "EUR ",
                "1",
            ),
            (
                "en",
                "GB",
                "en_GB.UTF-8",
                "%d/%m/%y",
                "%T",
                ".",
                "GBP ",
                "1",
            ),
            (
                "en",
                "CZ",
                "cs_CZ.UTF-8",
                "%-d.%-m.%Y",
                "%H:%M:%S",
                ",",
                "CZK ",
                "1",
            ),
            (
                "cs",
                "JP",
                "ja_JP.UTF-8",
                "%Y年%m月%d日",
                "%H時%M分%S秒",
                ".",
                "JPY ",
                "1",
            ),
            (
                "he",
                "DK",
                "en_DK.UTF-8",
                "%Y-%m-%d",
                "%T",
                ",",
                "DKK ",
                "1",
            ),
            (
                "da",
                "DK",
                "da_DK.UTF-8",
                "%d-%m-%Y",
                "%T",
                ",",
                "DKK ",
                "1",
            ),
            ("he", "BB", "C", "%m/%d/%y", "%H:%M:%S", ".", "", "1"),
        ] {
            let formats = format_locale(language, region, &locales).unwrap();
            assert_eq!(formats, expected);
            let mut command = Command::new("/usr/bin/locale");
            command.env_remove("LC_ALL");
            for (key, value) in locale_values(language, &formats).unwrap() {
                command.env(key, value);
            }
            let output = command
                .args([
                    "-k",
                    "d_fmt",
                    "t_fmt",
                    "decimal_point",
                    "int_curr_symbol",
                    "measurement",
                    "height",
                    "width",
                    "country_ab2",
                    "int_prefix",
                    "int_select",
                    "name_fmt",
                    "postal_fmt",
                ])
                .output()
                .unwrap();
            assert!(output.status.success());
            assert!(
                output.stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result = String::from_utf8(output.stdout).unwrap();
            for (key, value) in [
                ("d_fmt", date),
                ("t_fmt", time),
                ("decimal_point", decimal),
                ("int_curr_symbol", currency),
            ] {
                assert!(
                    result
                        .lines()
                        .any(|line| line == format!("{key}=\"{value}\"")),
                    "{language}/{region}: {result}"
                );
            }
            assert!(
                result
                    .lines()
                    .any(|line| line == format!("measurement={measurement}"))
            );
            let (height, width) = if region == "US" {
                (279, 216)
            } else {
                (297, 210)
            };
            for (key, value) in [("height", height), ("width", width)] {
                assert!(
                    result.lines().any(|line| line == format!("{key}={value}")),
                    "{language}/{region}: {result}"
                );
            }
            let (country, prefix, select, name, postal) = if formats == "C" {
                (
                    "",
                    "",
                    "",
                    "%p%t%g%t%m%t%f",
                    "%a%N%f%N%d%N%b%N%s %h %e %r%N%C-%z %T%N%c%N",
                )
            } else {
                let prefix = match region {
                    "IL" => "972",
                    "US" => "1",
                    "DE" => "49",
                    "BE" => "32",
                    "GB" => "44",
                    "CZ" => "420",
                    "JP" => "81",
                    "DK" => "45",
                    _ => panic!("Add the expected telephone prefix for {region}"),
                };
                let select = match region {
                    "IL" => "00,012,013,014,018",
                    "US" => "11",
                    "JP" => "010",
                    _ => "00",
                };
                let name = if region == "JP" {
                    "%p%t%f%t%g"
                } else {
                    "%d%t%g%t%m%t%f"
                };
                let postal = match region {
                    "JP" => "%z%c%T%s%b%e%r",
                    "US" => "%a%N%f%N%d%N%b%N%h %s %e %r%N%T, %S %z%N%c%N",
                    _ => "%f%N%a%N%d%N%b%N%s %h %e %r%N%z %T%N%c%N",
                };
                (region, prefix, select, name, postal)
            };
            for (key, value) in [
                ("country_ab2", country),
                ("int_prefix", prefix),
                ("int_select", select),
                ("name_fmt", name),
                ("postal_fmt", postal),
            ] {
                assert!(
                    result
                        .lines()
                        .any(|line| line == format!("{key}=\"{value}\"")),
                    "{language}/{region}: {result}"
                );
            }
            eprintln!("{language}/{region}: {formats}\n{result}");
        }
        assert_eq!(
            read_optional(Path::new("/etc/locale.conf")).unwrap(),
            system_before
        );
        assert_eq!(fs::read("/etc/localtime").unwrap(), timezone_before);
    }
    #[test]
    fn protected_accounts_and_unexpected_homes_are_rejected() {
        let passwd = "root:x:0:0:root:/root:/bin/zsh\nuser:x:1000:1000:User:/home/user:/bin/zsh\nother:x:1001:1001:Other:/etc:/bin/zsh\n";
        assert!(user_identity(passwd, "root").is_err());
        assert!(user_identity(passwd, "other").is_err());
        assert_eq!(
            user_identity(passwd, "user").unwrap(),
            (1000, "/home/user".into())
        );
    }

    #[test]
    fn setup_session_identity_is_separate_from_permanent_account_validation() {
        let passwd = "root:x:0:0:root:/root:/bin/zsh\nfluffsetup:x:1000:1000::/home/fluffsetup:/bin/zsh\nuser:x:1001:1001:User:/home/user:/bin/zsh\n";
        assert_eq!(
            setup_session_identity(passwd, "fluffsetup").unwrap(),
            (1000, "/home/fluffsetup".into())
        );
        assert!(user_identity(passwd, "fluffsetup").is_err());
        assert_eq!(
            user_identity(passwd, "user").unwrap(),
            (1001, "/home/user".into())
        );
        for username in ["root", "user", "", "fluffsetup2", "../fluffsetup"] {
            assert!(setup_session_identity(passwd, username).is_err());
        }
    }

    #[test]
    fn setup_session_identity_rejects_missing_or_unexpected_account_records() {
        for passwd in [
            "",
            "fluffsetup:x:0:0::/home/fluffsetup:/bin/zsh\n",
            "fluffsetup:x:999:999::/home/fluffsetup:/bin/zsh\n",
            "fluffsetup:x:invalid:1000::/home/fluffsetup:/bin/zsh\n",
            "fluffsetup:x:1000:1000::/root:/bin/zsh\n",
            "fluffsetup:x:1000:1000::/home/other:/bin/zsh\n",
            "fluffsetup:x:1000:1000::/home/fluffsetup:/bin/bash\n",
            "fluffsetup:x:1000:1000\n",
        ] {
            assert!(
                setup_session_identity(passwd, "fluffsetup").is_err(),
                "Accepted unexpected setup account: {passwd}"
            );
        }
    }
}
