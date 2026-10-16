//! System XKB registry and per-user Plasma keyboard preferences, without new crates.

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

pub const MAX_LAYOUTS: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Layout {
    pub id: String,
    pub layout: String,
    pub variant: String,
    pub name: String,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
}

pub fn parse_selection(value: &str) -> Option<Vec<&str>> {
    let ids: Vec<_> = value.split(',').collect();
    if ids.is_empty() || ids.len() > MAX_LAYOUTS {
        return None;
    }
    for (index, id) in ids.iter().enumerate() {
        let (layout, variant) = id.split_once(':').unwrap_or((id, ""));
        if !identifier(layout)
            || (id.contains(':') && !identifier(variant))
            || ids[..index].contains(id)
        {
            return None;
        }
    }
    Some(ids)
}

// xkeyboard-config ships this simple, stable list alongside evdev.xml.
// Only layout and variant sections are used. Models/options cannot become layouts.
pub fn parse_registry(contents: &str) -> Vec<Layout> {
    let mut section = "";
    let mut layouts = Vec::new();
    let mut variants = Vec::new();
    for line in contents.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('!') {
            section = header.trim();
            continue;
        }
        let Some((id, text)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        if !identifier(id) {
            continue;
        }
        let (layout, variant, name) = match section {
            "layout" => (id, "", text.trim()),
            "variant" => {
                let Some((layout, name)) = text.split_once(':') else {
                    continue;
                };
                (layout.trim(), id, name.trim())
            }
            _ => continue,
        };
        if !identifier(layout) || name.is_empty() || layout == "custom" {
            continue;
        }
        let entry = Layout {
            id: if variant.is_empty() {
                layout.into()
            } else {
                format!("{layout}:{variant}")
            },
            layout: layout.into(),
            variant: variant.into(),
            name: name.into(),
        };
        if variant.is_empty() {
            layouts.push(entry);
        } else {
            variants.push(entry);
        }
    }
    variants.retain(|variant| layouts.iter().any(|layout| layout.layout == variant.layout));
    layouts.extend(variants);
    layouts.sort_by_cached_key(|layout| (layout.name.to_lowercase(), layout.id.clone()));
    let mut ids = std::collections::HashSet::new();
    layouts.retain(|layout| ids.insert(layout.id.clone()));
    layouts
}

pub fn catalog() -> &'static [Layout] {
    static CATALOG: OnceLock<Vec<Layout>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        // Overrides exist only in safe preview mode, never privileged helpers.
        let preview_path = crate::preview_argument("keyboard-registry");
        let paths = preview_path.iter().map(String::as_str).chain([
            "/usr/share/X11/xkb/rules/evdev.lst",
            "/usr/share/xkeyboard-config-2/rules/evdev.lst",
            "/usr/share/X11/xkb/rules/base.lst",
            "/usr/share/xkeyboard-config-2/rules/base.lst",
        ]);
        for path in paths {
            if let Ok(contents) = fs::read_to_string(path) {
                let layouts = parse_registry(&contents);
                if layouts.iter().any(|layout| layout.id == "us") {
                    return layouts;
                }
            }
        }
        eprintln!("XKB registry unavailable; offering U.S. English only.");
        vec![Layout {
            id: "us".into(),
            layout: "us".into(),
            variant: String::new(),
            name: "English (US)".into(),
        }]
    })
}

pub fn validate_selection(value: &str, layouts: &[Layout]) -> Result<(), String> {
    let ids = parse_selection(value).ok_or("Invalid keyboard selection.")?;
    if ids
        .iter()
        .any(|id| !layouts.iter().any(|layout| layout.id == *id))
    {
        return Err("A selected keyboard layout is not installed.".into());
    }
    Ok(())
}

pub fn defaults(language: &str, installed: &[Layout]) -> String {
    defaults_for_region(
        language,
        crate::keyboard_profiles::default_country(language),
        installed,
    )
}

pub fn defaults_for_region(language: &str, country: &str, installed: &[Layout]) -> String {
    crate::keyboard_profiles::defaults(language, country, installed)
}

// Keep one entry per major alternative arrangement, not its many dead-key,
// punctuation, hardware or ergonomic subvariants. Language/script and regional
// defaults are admitted separately even when their IDs are not in this list.
fn significant_variant(layout: &Layout) -> bool {
    matches!(
        layout.variant.as_str(),
        "qwerty" | "qwertz" | "azerty" | "dvorak" | "colemak" | "bepo" | "neo"
    ) && base_arrangement(&layout.layout) != Some(layout.variant.to_ascii_uppercase().as_str())
        || matches!(
            layout.id.as_str(),
            "tr:f" | "rs:latinyz" | "me:latinyz" | "ca:fr-dvorak"
        )
}

// Only annotate known arrangements. Do not guess a letter order for non-Latin
// layouts, or infer it from a language/country name. The XKB IDs stay unchanged.
fn base_arrangement(layout: &str) -> Option<&'static str> {
    match layout {
        "fr" | "be" => Some("AZERTY"),
        "de" | "at" | "ch" | "cz" | "sk" | "hu" | "hr" | "ba" | "si" => Some("QWERTZ"),
        "us" | "gb" | "au" | "nz" | "ca" | "es" | "latam" | "pt" | "br" | "it" | "nl" | "dk"
        | "no" | "se" | "fi" | "is" | "ee" | "lv" | "pl" | "ro" | "tr" | "ie" => Some("QWERTY"),
        _ => None,
    }
}

pub fn add(value: &str, id: &str, layouts: &[Layout]) -> Option<String> {
    let mut ids = parse_selection(value)?;
    if ids.contains(&id)
        || ids.len() >= MAX_LAYOUTS
        || !layouts.iter().any(|layout| layout.id == id)
    {
        return None;
    }
    ids.push(id);
    Some(ids.join(","))
}

pub fn remove(value: &str, id: &str) -> Option<String> {
    let mut ids = parse_selection(value)?;
    if ids.len() <= 1 || !ids.contains(&id) {
        return None;
    }
    ids.retain(|entry| *entry != id);
    Some(ids.join(","))
}

pub fn make_default(value: &str, id: &str) -> Option<String> {
    let mut ids = parse_selection(value)?;
    let index = ids.iter().position(|entry| *entry == id)?;
    if index == 0 {
        return None;
    }
    ids.remove(index);
    ids.insert(0, id);
    Some(ids.join(","))
}

pub fn move_layout(value: &str, id: &str, direction: i32) -> Option<String> {
    if direction != -1 && direction != 1 {
        return None;
    }
    let mut ids = parse_selection(value)?;
    let index = ids.iter().position(|entry| *entry == id)?;
    let destination = index.checked_add_signed(direction as isize)?;
    if destination >= ids.len() {
        return None;
    }
    ids.swap(index, destination);
    Some(ids.join(","))
}

pub fn reorder_layout(value: &str, id: &str, destination: i32) -> Option<String> {
    let mut ids = parse_selection(value)?;
    let destination = usize::try_from(destination).ok()?;
    let source = ids.iter().position(|entry| *entry == id)?;
    if destination >= ids.len() || source == destination {
        return None;
    }
    let moved = ids.remove(source);
    ids.insert(destination, moved);
    Some(ids.join(","))
}

pub fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            value if value.is_control() => result.push_str(&format!("\\u{:04x}", value as u32)),
            value => result.push(value),
        }
    }
    result.push('"');
    result
}

pub fn catalog_json(language: &str, selection: &str, country: &str) -> String {
    static METADATA: OnceLock<PickerMetadata> = OnceLock::new();
    let metadata = METADATA.get_or_init(|| picker_metadata(catalog()));
    let translated = crate::keyboard_names::catalog(language);
    let defaults = defaults_for_region(language, country, catalog());
    catalog_json_using(
        language,
        selection,
        catalog(),
        &translated,
        &defaults,
        metadata,
    )
}

struct PickerMetadata {
    native_names: Vec<(String, &'static str)>,
    main_ids: std::collections::HashSet<String>,
}

fn picker_metadata(installed: &[Layout]) -> PickerMetadata {
    let languages = crate::i18n::ordered_languages();
    let native_names: Vec<_> = languages
        .iter()
        .map(|meta| {
            (
                crate::keyboard_profiles::native_primary(meta.code, installed).unwrap_or_default(),
                meta.native,
            )
        })
        .collect();
    let mut main_ids: std::collections::HashSet<String> = native_names
        .iter()
        .flat_map(|(ids, _)| ids.split(',').map(str::to_string))
        .collect();
    for language in languages {
        for region in crate::i18n::ordered_regions(language.code) {
            main_ids.extend(
                defaults(
                    crate::i18n::canonical_choice(region.code).unwrap(),
                    installed,
                )
                .split(',')
                .map(str::to_string),
            );
        }
    }
    main_ids.extend(crate::keyboard_profiles::required_ids().map(str::to_string));
    PickerMetadata {
        native_names,
        main_ids,
    }
}

#[cfg(test)]
fn catalog_json_with(
    language: &str,
    selection: &str,
    installed: &[Layout],
    translated: &std::collections::HashMap<String, String>,
) -> String {
    catalog_json_grouped(language, selection, installed, translated, "")
}

#[cfg(test)]
fn catalog_json_grouped(
    language: &str,
    selection: &str,
    installed: &[Layout],
    translated: &std::collections::HashMap<String, String>,
    regional_defaults: &str,
) -> String {
    catalog_json_using(
        language,
        selection,
        installed,
        translated,
        regional_defaults,
        &picker_metadata(installed),
    )
}

fn catalog_json_using(
    language: &str,
    selection: &str,
    installed: &[Layout],
    translated: &std::collections::HashMap<String, String>,
    regional_defaults: &str,
    metadata: &PickerMetadata,
) -> String {
    let current = crate::i18n::metadata(language);
    let preferred = crate::keyboard_profiles::native_primary(language, installed);
    // Keep the main national layouts plus the script/language variants needed
    // by supported languages and regions, and major alternative arrangements.
    // No minor/dead-key subvariants in the add list. An older saved choice stays visible so recovery
    // never silently replaces or loses a user's keyboard.
    let selected = parse_selection(selection).unwrap_or_default();
    let main_id = |id: &str| {
        metadata.main_ids.contains(id) || regional_defaults.split(',').any(|entry| entry == id)
    };
    let mut layouts: Vec<_> = installed
        .iter()
        .filter(|layout| {
            layout.variant.is_empty()
                || significant_variant(layout)
                || selected.contains(&layout.id.as_str())
                || main_id(&layout.id)
        })
        .collect();
    // Only absent regional defaults are suggested. Selected defaults disappear
    // from the add list in QML; removing one makes that exact choice suggested.
    let missing: Vec<_> = regional_defaults
        .split(',')
        .filter(|id| !selected.contains(id))
        .collect();
    layouts.sort_by_key(|layout| {
        missing
            .iter()
            .position(|id| *id == layout.id)
            .unwrap_or(usize::MAX)
    });
    let rows: Vec<_> = layouts
        .into_iter()
        .map(|layout| {
            let mut name = if let Some(name) = translated.get(&layout.name) {
                name.clone()
            } else if layout.id == "us" {
                match crate::i18n::base_language(language).unwrap_or("en") {
                    "he" => "אנגלית (ארצות הברית)",
                    "ar" => "الإنجليزية (الولايات المتحدة)",
                    "de" => "Englisch (USA)",
                    "fr" => "Anglais (États-Unis)",
                    _ => "U.S. English",
                }
                .to_string()
            } else if preferred.as_deref() == Some(layout.id.as_str())
                && layout.variant.is_empty()
                && crate::i18n::base_language(language) != Some("en")
            {
                current.map_or_else(|| layout.name.clone(), |meta| meta.native.to_string())
            } else {
                layout.name.clone()
            };
            let arrangement = match layout.id.as_str() {
                "rs:latin" | "me:latin" => Some("QWERTZ"),
                _ if layout.variant.is_empty() || main_id(&layout.id) => {
                    base_arrangement(&layout.layout)
                }
                _ => None,
            };
            if let Some(arrangement) = arrangement
                && !name.contains(arrangement)
            {
                name.push_str(&format!(" ({arrangement})"));
            }
            // Search both the displayed translation and untouched English name,
            // plus native language aliases and stable XKB IDs.
            let native_names: Vec<_> = metadata
                .native_names
                .iter()
                .filter(|(selection, _)| {
                    layout.id != "us" && selection.split(',').any(|id| id == layout.id)
                })
                .map(|(_, native)| *native)
                .collect();
            format!(
                "{{\"id\":{},\"name\":{},\"group\":{},\"search\":{}}}",
                json_string(&layout.id),
                json_string(&name),
                json_string(if missing.contains(&layout.id.as_str()) {
                    "suggested"
                } else {
                    "other"
                }),
                json_string(&format!(
                    "{} {} {} {}",
                    name,
                    layout.name,
                    layout.id,
                    native_names.join(" ")
                ))
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

fn configuration(existing: &str, selection: &str, layouts: &[Layout]) -> Result<String, String> {
    validate_selection(selection, layouts)?;
    let ids = parse_selection(selection).unwrap();
    let selected: Vec<_> = ids
        .iter()
        .map(|id| layouts.iter().find(|layout| layout.id == *id).unwrap())
        .collect();
    let mut text = existing.to_string();
    for (key, value) in [
        ("Use", "true".into()),
        (
            "LayoutList",
            selected
                .iter()
                .map(|layout| layout.layout.as_str())
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "VariantList",
            selected
                .iter()
                .map(|layout| layout.variant.as_str())
                .collect::<Vec<_>>()
                .join(","),
        ),
        ("DisplayNames", String::new()),
        ("LayoutLoopCount", "-1".into()),
        ("SwitchMode", "Global".into()),
    ] {
        text = crate::locale::ini_value(&text, "Layout", key, &value);
    }
    // Preserve compose and other skeleton options, and always provide both
    // requested group-switch shortcuts without duplicating existing ones.
    let options = group_options(existing);
    let mut options: Vec<_> = options
        .split(',')
        .filter(|option| !option.is_empty())
        .collect();
    for option in ["grp:alt_shift_toggle", "grp:win_space_toggle"] {
        if !options.contains(&option) {
            options.push(option);
        }
    }
    text = crate::locale::ini_value(&text, "Layout", "Options", &options.join(","));
    text = crate::locale::ini_value(&text, "Layout", "ResetOldOptions", "true");
    Ok(text)
}

pub fn apply_user(selection: &str) -> Result<(), String> {
    let username = crate::locale::checked("/usr/bin/id", &["-un"])?;
    let passwd = fs::read_to_string("/etc/passwd").map_err(|error| error.to_string())?;
    let (_, home) = crate::locale::user_identity(&passwd, &username)?;
    let path = Path::new(&home).join(".config/kxkbrc");
    let text = configuration(&crate::locale::read_optional(&path)?, selection, catalog())?;
    crate::locale::atomic_write(&path, &text, 0o600)
}

fn group_options(text: &str) -> &str {
    let mut group = "";
    let mut options = "";
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            group = &line[1..line.len() - 1];
        }
        if group == "Layout"
            && let Some(value) = line.strip_prefix("Options=")
        {
            options = value;
        }
    }
    options
}

fn dbus_layout_ids(reply: &str) -> Vec<&str> {
    reply
        .split("struct {")
        .skip(1)
        .filter_map(|entry| {
            entry
                .split_once("string \"")?
                .1
                .split_once('"')
                .map(|(id, _)| id)
        })
        .collect()
}

fn layout_call(
    method: &str,
    argument: Option<&str>,
    run: &mut impl FnMut(&str, &[&str]) -> Result<String, String>,
) -> Result<String, String> {
    // The service name remains lowercase. Current KWin's interface is not.
    // Fall back only for an older interface, never for a failed layout switch.
    for interface in ["org.kde.KeyboardLayouts", "org.kde.keyboard"] {
        let method = format!("{interface}.{method}");
        let mut args = vec![
            "--session",
            "--type=method_call",
            "--print-reply",
            "--reply-timeout=1500",
            "--dest=org.kde.keyboard",
            "/Layouts",
            &method,
        ];
        if let Some(argument) = argument {
            args.push(argument);
        }
        match run("/usr/bin/dbus-send", &args) {
            Err(error)
                if interface == "org.kde.KeyboardLayouts"
                    && (error.contains("UnknownInterface") || error.contains("UnknownMethod")) => {}
            result => return result,
        }
    }
    unreachable!()
}

fn active_layout_with(
    selection: &str,
    requested: Option<usize>,
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
) -> Result<i32, String> {
    let count = parse_selection(selection)
        .ok_or("Invalid keyboard selection.")?
        .len();
    if let Some(index) = requested {
        if index >= count {
            return Err("Invalid keyboard layout index.".into());
        }
        let reply = layout_call("setLayout", Some(&format!("uint32:{index}")), &mut run)?;
        if !reply.lines().any(|line| line.trim() == "boolean true") {
            return Err("KWin could not switch the keyboard layout.".into());
        }
    }
    let reply = layout_call("getLayout", None, &mut run)?;
    let index = reply
        .lines()
        .find_map(|line| line.trim().strip_prefix("uint32 ")?.parse::<usize>().ok())
        .filter(|index| *index < count)
        .ok_or("KWin returned an invalid keyboard layout index.")?;
    if requested.is_some_and(|requested| requested != index) {
        return Err("KWin did not switch to the requested keyboard layout.".into());
    }
    Ok(index as i32)
}

fn checked_active_layout_with(
    selection: &str,
    requested: Option<usize>,
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
) -> Result<i32, String> {
    let ids = parse_selection(selection).ok_or("Invalid keyboard selection.")?;
    if requested.is_some_and(|index| index >= ids.len()) {
        return Err("Invalid keyboard layout index.".into());
    }
    let expected: Vec<_> = ids
        .into_iter()
        .map(|id| id.split(':').next().unwrap())
        .collect();
    let reply = layout_call("getLayoutsList", None, &mut run)?;
    if dbus_layout_ids(&reply) != expected {
        return Err("KWin keyboard configuration changed.".into());
    }
    active_layout_with(selection, requested, run)
}

fn check_session_account() -> Result<String, String> {
    let username = crate::locale::checked("/usr/bin/id", &["-un"])?;
    if username != crate::TEMPORARY_USER
        || std::env::var_os("WAYLAND_DISPLAY").is_none()
        || std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none()
    {
        return Err(
            "Live keyboard selection requires the temporary FluffSetup KWin session.".into(),
        );
    }
    Ok(username)
}

pub fn active_layout(selection: &str, requested: Option<usize>) -> Result<i32, String> {
    check_session_account()?;
    checked_active_layout_with(selection, requested, crate::locale::checked)
}

fn sync_session_with(
    path: &Path,
    selection: &str,
    layouts: &[Layout],
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
) -> Result<(), String> {
    validate_selection(selection, layouts)?;
    if !path.is_absolute() || path.file_name() != Some(std::ffi::OsStr::new("kxkbrc")) {
        return Err("Invalid session configuration path.".into());
    }
    let owner = run(
        "/usr/bin/dbus-send",
        &[
            "--session",
            "--type=method_call",
            "--print-reply",
            "--reply-timeout=1500",
            "--dest=org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus.NameHasOwner",
            "string:org.kde.keyboard",
        ],
    )?;
    if !owner.lines().any(|line| line.trim() == "boolean true") {
        return Err("KWin's keyboard service is unavailable.".into());
    }
    let existing = crate::locale::read_optional(path)?;
    let text = configuration(&existing, selection, layouts)?;
    crate::locale::atomic_write(path, &text, 0o600)?;
    // Notify KConfig's relative name after atomically saving the complete map.
    // Its CLI suppresses absolute-path notifications. A directly flushed signal
    // also handles unchanged Options without a temporary empty-options write.
    let notified = run(
        "/usr/bin/busctl",
        &[
            "--user",
            "--timeout=1500ms",
            "emit",
            "/kxkbrc",
            "org.kde.kconfig.notify",
            "ConfigChanged",
            "a{saay}",
            "1",
            "Layout",
            "1",
            // One changed key, the byte array "Options".
            "7",
            "79",
            "112",
            "116",
            "105",
            "111",
            "110",
            "115",
        ],
    );
    if let Err(error) = notified {
        crate::locale::atomic_write(path, &existing, 0o600)?;
        return Err(error);
    }
    // Older versions use this signal. These notifications only reload a map;
    // they do not toggle a group, so they cannot double-switch the keyboard.
    run(
        "/usr/bin/dbus-send",
        &[
            "--session",
            "--type=signal",
            "/Layouts",
            "org.kde.keyboard.reloadConfig",
        ],
    )?;
    let expected: Vec<_> = parse_selection(selection)
        .unwrap()
        .into_iter()
        .map(|id| id.split(':').next().unwrap())
        .collect();
    for _ in 0..15 {
        let reply = layout_call("getLayoutsList", None, &mut run)?;
        if dbus_layout_ids(&reply) == expected {
            let reply = layout_call("setLayout", Some("uint32:0"), &mut run)?;
            if reply.lines().any(|line| line.trim() == "boolean true") {
                return Ok(());
            }
            return Err("KWin could not select the default keyboard layout.".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(60));
    }
    Err("KWin did not load the selected keyboard layouts.".into())
}

pub fn sync_session(selection: &str) -> Result<(), String> {
    if crate::preview_mode() {
        return Ok(());
    }
    // Never alter a developer's desktop or another account's preferences.
    // Only the temporary, dedicated first-boot session may be changed here.
    let username = check_session_account()?;
    let passwd = fs::read_to_string("/etc/passwd").map_err(|error| error.to_string())?;
    let (_, home) = crate::locale::setup_session_identity(&passwd, &username)?;
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| Path::new(&home).join(".config"));
    sync_session_with(
        &root.join("kxkbrc"),
        selection,
        catalog(),
        crate::locale::checked,
    )
}

pub fn apply_for_user(username: &str, selection: &str) -> Result<(), String> {
    if crate::locale::checked("/usr/bin/id", &["-u"])? != "0" {
        return Err("Keyboard finalization must run as root.".into());
    }
    let passwd = fs::read_to_string("/etc/passwd").map_err(|error| error.to_string())?;
    crate::locale::user_identity(&passwd, username)?;
    validate_selection(selection, catalog())?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    crate::locale::checked(
        "/usr/bin/runuser",
        &[
            "-u",
            username,
            "--",
            executable.to_str().ok_or("Invalid executable path.")?,
            "--apply-user-keyboard",
            selection,
        ],
    )?;
    apply_console(selection)?;
    Ok(())
}

fn console_keymap(selection: &str, mapping: &str, available: &str) -> Option<String> {
    let id = parse_selection(selection)?.first().copied()?;
    let (layout, variant) = id.split_once(':').unwrap_or((id, ""));
    let installed =
        |name: &str| identifier(name) && available.lines().any(|line| line.trim() == name);
    let converted = if variant.is_empty() {
        layout.to_string()
    } else {
        format!("{layout}-{variant}")
    };
    // kbd also ships console maps generated from XKB, when available.
    if installed(&converted) {
        return Some(converted);
    }
    for line in mapping
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 4 {
            continue;
        }
        let source_layout = fields[1].split(',').next().unwrap_or_default();
        let source_variant = fields[3].split(',').next().unwrap_or_default();
        let source_variant = if source_variant == "-" {
            ""
        } else {
            source_variant
        };
        if source_layout == layout && source_variant == variant && installed(fields[0]) {
            return Some(fields[0].to_string());
        }
    }
    // AZERTY variants share the standard console geometry unless kbd offers
    // an exact converted map. BEPO/Dvorak/QWERTY must never become AZERTY.
    if layout == "fr"
        && matches!(
            variant,
            "azerty"
                | "afnor"
                | "oss"
                | "oss_latin9"
                | "latin9"
                | "nodeadkeys"
                | "oss_nodeadkeys"
                | "latin9_nodeadkeys"
                | "mac"
        )
    {
        return console_keymap("fr", mapping, available);
    }
    if layout == "be"
        && matches!(
            variant,
            "oss" | "nodeadkeys" | "oss_latin9" | "oss_nodeadkeys" | "iso-alternate" | "wang"
        )
    {
        return console_keymap("be", mapping, available);
    }
    if layout == "ara" && variant == "azerty" {
        // If kbd has no exact Arabic AZERTY map, keep the same physical French
        // geometry on the console instead of silently leaving a QWERTY map.
        return console_keymap("fr", mapping, available);
    }
    None
}

fn apply_console(selection: &str) -> Result<(), String> {
    let available = crate::locale::checked("/usr/bin/localectl", &["list-keymaps"])?;
    let mapping = crate::locale::read_optional(Path::new("/usr/share/systemd/kbd-model-map"))?;
    let Some(keymap) = console_keymap(selection, &mapping, &available) else {
        // Not every graphical layout has a Linux console counterpart. Keep the
        // existing map rather than replacing it with an unrelated layout.
        eprintln!("No console counterpart for {selection}; retaining the console keyboard.");
        return Ok(());
    };
    // Let systemd preserve FONT and other vconsole settings. No conversion to
    // system X11/Wayland layouts; those belong to the new user's kxkbrc.
    crate::locale::checked(
        "/usr/bin/localectl",
        &["--no-convert", "set-keymap", &keymap],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn geographic_defaults_are_installed_and_language_sensitive() {
        let installed = parse_registry(include_str!("../tests/fixtures/keyboard-profiles.lst"));
        for (language, region, expected) in [
            ("en", "DE", "de"),
            ("de", "FR", "fr,de"),
            ("en", "BE", "be"),
            ("fr", "BE", "be"),
            ("fr", "CA", "ca"),
            ("en", "CA", "us"),
            ("fr", "CH", "ch:fr,ch"),
            ("de", "CH", "ch,ch:fr"),
            ("he", "IL", "us,il"),
            ("en", "IL", "us,il"),
            ("ru", "IL", "us,il,ru"),
            ("he", "US", "us,il"),
            ("ar", "DZ", "ara:azerty,fr"),
            ("fr", "DZ", "fr,ara:azerty"),
            ("en", "GB", "gb"),
            ("es", "MX", "latam"),
            ("pt", "BR", "br"),
            ("pt", "PT", "pt"),
            ("en", "JP", "jp,us"),
            ("hi", "IN", "in,us"),
            ("en", "IN", "in:eng"),
            ("zh_Hans", "CN", "us"),
            ("en", "PS", "us"),
            ("en", "XX", "us"),
        ] {
            let actual = defaults_for_region(language, region, &installed);
            assert_eq!(actual, expected, "{language}/{region}");
            validate_selection(&actual, &installed).unwrap();
        }
        assert_eq!(
            defaults_for_region("ar", "DZ", &parse_registry(REGISTRY)),
            "fr"
        );
    }

    #[test]
    fn suggestions_are_only_missing_regional_defaults() {
        let installed = parse_registry(REGISTRY);
        let translated = std::collections::HashMap::new();
        for (selection, expected_count) in [("us,il", 0), ("us", 1), ("fr", 2)] {
            let json = catalog_json_grouped("he", selection, &installed, &translated, "us,il");
            assert_eq!(
                json.matches("\"group\":\"suggested\"").count(),
                expected_count
            );
            if selection == "us" {
                assert!(json.starts_with("[{\"id\":\"il\""));
            }
        }
        let json = catalog_json_grouped("ar", "us", &installed, &translated, "us,il");
        assert!(json.contains("\"id\":\"us:intl\""));
        let installed = parse_registry(include_str!("../tests/fixtures/keyboard-profiles.lst"));
        let defaults = defaults_for_region("ja", "JP", &installed);
        let removed = catalog_json_grouped("ja", "jp", &installed, &translated, &defaults);
        assert!(removed.starts_with("[{\"id\":\"us\""));
        assert_eq!(removed.matches("\"group\":\"suggested\"").count(), 1);
        let reordered = catalog_json_grouped("ja", "us,jp", &installed, &translated, &defaults);
        assert!(!reordered.contains("\"group\":\"suggested\""));
    }

    #[test]
    fn regional_keyboards_preserve_serbian_scripts() {
        let installed = parse_registry(
            "! layout\n us English (US)\n rs Serbian\n me Montenegrin\n ru Russian\n ua Ukrainian\n! variant\n latin rs: Serbian (Latin)\n latin me: Montenegrin (Latin)\n",
        );
        for (language, region, expected) in [
            ("sr_Latn", "ME", "rs:latin"),
            ("sr", "ME", "rs,us"),
            ("sr_Latn", "BA", "rs:latin"),
            ("sr", "BA", "rs,us"),
            ("ru", "UA", "ua,us,ru"),
            ("uk", "UA", "ua,us"),
        ] {
            assert_eq!(defaults_for_region(language, region, &installed), expected);
        }
    }
    const REGISTRY: &str = "! model\n pc104 Generic PC\n! layout\n us English (US)\n il Hebrew\n de German\n fr French\n rs Serbian\n pk Urdu\n! variant\n intl us: English (US, international)\n latin rs: Serbian (Latin)\n ara pk: Punjabi (Shahmukhi)\n bad unknown: Invalid\n! option\n grp:win_space_toggle Switch\n";
    #[test]
    fn displayed_layouts_and_search_use_translations_and_english_together() {
        let installed = parse_registry(REGISTRY);
        let translated = std::collections::HashMap::from([
            ("German".into(), "גרמנית".into()),
            (
                "English (US, international)".into(),
                "אנגלית (ארצות הברית, בינלאומית)".into(),
            ),
        ]);
        let json = catalog_json_with("he", "us:intl", &installed, &translated);
        assert!(json.contains("\"name\":\"גרמנית (QWERTZ)\""));
        assert!(json.contains("\"search\":\"גרמנית (QWERTZ) German de"));
        assert!(json.contains("\"name\":\"אנגלית (ארצות הברית, בינלאומית) (QWERTY)\""));
        assert!(json.contains(
            "אנגלית (ארצות הברית, בינלאומית) (QWERTY) English (US, international) us:intl"
        ));
        assert_eq!(installed, parse_registry(REGISTRY));
    }

    #[test]
    fn indicator_reads_and_switches_only_the_active_group() {
        let selection = "us,il,us:intl,de";
        for index in 0..4 {
            let mut calls = 0;
            assert_eq!(
                active_layout_with(selection, Some(index), |command, args| {
                    assert_eq!(command, "/usr/bin/dbus-send");
                    assert!(args.contains(&"--reply-timeout=1500"));
                    calls += 1;
                    if args.contains(&"org.kde.KeyboardLayouts.setLayout") {
                        assert_eq!(args.last().unwrap(), &format!("uint32:{index}"));
                        Ok("boolean true".into())
                    } else {
                        assert!(args.contains(&"org.kde.KeyboardLayouts.getLayout"));
                        Ok(format!("method return serial=1\n uint32 {index}\n"))
                    }
                })
                .unwrap(),
                index as i32
            );
            assert_eq!(calls, 2);
        }
        assert_eq!(
            active_layout_with(selection, None, |_, args| {
                assert!(args.contains(&"org.kde.KeyboardLayouts.getLayout"));
                Ok("uint32 2".into())
            })
            .unwrap(),
            2
        );
    }

    #[test]
    fn indicator_rejects_bad_indices_replies_and_failed_switches() {
        for selection in ["", "bad/id", "us,us", "us,il,fr,de,ru"] {
            assert!(
                active_layout_with(selection, None, |_, _| panic!(
                    "Invalid selection queried KWin"
                ))
                .is_err()
            );
        }
        assert!(
            active_layout_with("us,il", Some(2), |_, _| panic!(
                "Invalid index queried KWin"
            ))
            .is_err()
        );
        for reply in ["", "uint32 4", "uint32 -1", "uint32 nope", "boolean true"] {
            assert!(active_layout_with("us,il", None, |_, _| Ok(reply.into())).is_err());
        }
        assert!(
            active_layout_with("us,il", Some(1), |_, args| {
                assert!(args.contains(&"org.kde.KeyboardLayouts.setLayout"));
                Ok("boolean false".into())
            })
            .is_err()
        );
        assert!(
            active_layout_with("us,il", Some(1), |_, args| {
                Ok(if args.contains(&"org.kde.KeyboardLayouts.setLayout") {
                    "boolean true"
                } else {
                    "uint32 0"
                }
                .into())
            })
            .is_err()
        );
    }

    #[test]
    fn indicator_supports_legacy_kwin_without_retrying_other_failures() {
        let mut legacy_calls = 0;
        assert_eq!(
            active_layout_with("us,il", Some(1), |_, args| {
                if args
                    .iter()
                    .any(|arg| arg.starts_with("org.kde.KeyboardLayouts."))
                {
                    return Err("org.freedesktop.DBus.Error.UnknownInterface".into());
                }
                legacy_calls += 1;
                Ok(if args.contains(&"org.kde.keyboard.setLayout") {
                    "boolean true"
                } else {
                    "uint32 1"
                }
                .into())
            })
            .unwrap(),
            1
        );
        assert_eq!(legacy_calls, 2);
        let mut calls = 0;
        assert!(
            active_layout_with("us,il", Some(1), |_, _| {
                calls += 1;
                Err("ServiceUnknown".into())
            })
            .is_err()
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn indicator_does_not_mislabel_a_restarted_or_changed_compositor_map() {
        let reply = "struct {\n string \"il\"\n}\nstruct {\n string \"us\"\n}\n";
        assert_eq!(
            checked_active_layout_with("us,il", None, |_, args| {
                assert!(args.contains(&"org.kde.KeyboardLayouts.getLayoutsList"));
                Ok(reply.into())
            })
            .unwrap_err(),
            "KWin keyboard configuration changed."
        );
        let reply = "struct {\n string \"us\"\n}\nstruct {\n string \"il\"\n}\n";
        assert_eq!(
            checked_active_layout_with("us,il", Some(1), |_, args| {
                Ok(
                    if args.contains(&"org.kde.KeyboardLayouts.getLayoutsList") {
                        reply
                    } else if args.contains(&"org.kde.KeyboardLayouts.setLayout") {
                        "boolean true"
                    } else {
                        "uint32 1"
                    }
                    .into(),
                )
            })
            .unwrap(),
            1
        );
    }

    #[test]
    #[ignore = "requires the temporary FluffSetup account, disposable headless KWin and private session bus"]
    fn isolated_kwin_indicator_switches_real_groups_without_reordering() {
        let root = std::env::var("XDG_CONFIG_HOME").unwrap();
        assert!(root.starts_with("/tmp/fluffsetup-kwin-layouts-"));
        let path = Path::new(&root).join("kxkbrc");
        for _ in 0..3 {
            for selection in [
                "us",
                "us,il",
                "il,us",
                "fr,us",
                "de,il,us:intl",
                "us,il,fr,de",
                "us,il,us:intl,de",
                "us",
            ] {
                // Exercise the public entry point, including the reserved setup
                // account check that a helper-only test would otherwise bypass.
                sync_session(selection).unwrap_or_else(|error| {
                    let loaded = layout_call("getLayoutsList", None, &mut crate::locale::checked);
                    panic!("Could not activate {selection}: {error}; loaded layouts: {loaded:?}")
                });
                let ids = parse_selection(selection).unwrap();
                let expected: Vec<_> = ids.iter().map(|id| id.split(':').next().unwrap()).collect();
                let before = fs::read_to_string(&path).unwrap();
                let list =
                    layout_call("getLayoutsList", None, &mut crate::locale::checked).unwrap();
                assert_eq!(dbus_layout_ids(&list), expected);
                assert_eq!(
                    checked_active_layout_with(selection, None, crate::locale::checked).unwrap(),
                    0
                );
                for index in (0..ids.len()).rev().chain(0..ids.len()) {
                    assert_eq!(
                        checked_active_layout_with(selection, Some(index), crate::locale::checked)
                            .unwrap(),
                        index as i32
                    );
                    assert_eq!(
                        checked_active_layout_with(selection, None, crate::locale::checked)
                            .unwrap(),
                        index as i32
                    );
                    assert_eq!(fs::read_to_string(&path).unwrap(), before);
                }
            }
        }
    }

    #[test]
    fn live_session_notifies_old_and_new_kwin_preserving_order_and_options() {
        let root = std::env::temp_dir().join(format!(
            "fluffsetup-keyboard-session-{}",
            std::process::id()
        ));
        let path = root.join("kxkbrc");
        let existing = "[Layout]\nModel=pc105\nOptions=compose:ralt,grp:alt_shift_toggle\n[Other]\nKeep=true\n";
        crate::locale::atomic_write(&path, existing, 0o600).unwrap();
        let installed = parse_registry(REGISTRY);
        for selection in ["us,il", "il,us:intl,de", "de,il,us:intl", "us"] {
            let mut notified = false;
            let mut legacy = false;
            let mut probes = 0;
            sync_session_with(&path, selection, &installed, |command, args| {
                if args.contains(&"org.freedesktop.DBus.NameHasOwner") { return Ok(" boolean true\n".into()); }
                if command == "/usr/bin/busctl" {
                    assert_eq!(args, &["--user", "--timeout=1500ms", "emit", "/kxkbrc", "org.kde.kconfig.notify", "ConfigChanged", "a{saay}", "1", "Layout", "1", "7", "79", "112", "116", "105", "111", "110", "115"]);
                    let text = fs::read_to_string(&path).unwrap();
                    assert_eq!(group_options(&text), "compose:ralt,grp:alt_shift_toggle,grp:win_space_toggle");
                    assert!(text.contains("Model=pc105"));
                    assert!(text.contains("Keep=true"));
                    notified = true;
                } else if args.contains(&"org.kde.keyboard.reloadConfig") {
                    assert!(notified);
                    legacy = true;
                } else if args.contains(&"org.kde.KeyboardLayouts.getLayoutsList") {
                    assert!(legacy);
                    probes += 1;
                    if probes == 1 { return Ok(String::new()); }
                    return Ok(parse_selection(selection).unwrap().into_iter().map(|id|
                        format!("struct {{\n string \"{}\"\n string \"\"\n string \"Translated description\"\n}}\n", id.split(':').next().unwrap())
                    ).collect());
                } else {
                    assert!(args.contains(&"org.kde.KeyboardLayouts.setLayout"));
                    assert_eq!(args.last(), Some(&"uint32:0"));
                    return Ok(" boolean true\n".into());
                }
                Ok(String::new())
            }).unwrap();
            assert_eq!(probes, 2);
            let text = fs::read_to_string(&path).unwrap();
            assert_eq!(configuration(&text, selection, &installed).unwrap(), text);
        }
        let before = fs::read_to_string(&path).unwrap();
        assert!(
            sync_session_with(
                &path,
                "us,il",
                &installed,
                |_, _| Ok("boolean false".into())
            )
            .is_err()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
        assert!(
            sync_session_with(&path, "bad", &installed, |_, _| panic!(
                "Invalid selection ran a command"
            ))
            .is_err()
        );
        assert!(
            sync_session_with(&path, "us,il", &installed, |command, _| {
                if command == "/usr/bin/busctl" {
                    Err("Notify failed".into())
                } else {
                    Ok("boolean true".into())
                }
            })
            .is_err()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
        for invalid_path in [Path::new("kxkbrc"), Path::new("/tmp/not-kxkbrc")] {
            assert!(
                sync_session_with(invalid_path, "us", &installed, |_, _| panic!(
                    "Invalid config path ran a command"
                ))
                .is_err()
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn simple_picker_keeps_main_script_variants_and_recovers_old_choices() {
        let installed = parse_registry(
            "! layout\n us English (US)\n fr French\n rs Serbian\n in Indian\n! variant\n intl us: English (US, intl., with dead keys)\n nodeadkeys fr: French (no dead keys)\n oss fr: French (alt.)\n latin rs: Serbian (Latin)\n guru in: Punjabi (Gurmukhi)\n",
        );
        let json = catalog_json_with("en", "us", &installed, &Default::default());
        for id in ["us", "us:intl", "fr", "rs:latin", "in:guru"] {
            assert!(json.contains(&format!("\"id\":\"{id}\"")));
        }
        for id in ["fr:nodeadkeys", "fr:oss"] {
            assert!(!json.contains(&format!("\"id\":\"{id}\"")));
        }
        let recovered = catalog_json_with("en", "us,fr:oss", &installed, &Default::default());
        assert!(recovered.contains("\"id\":\"fr:oss\""));
        assert!(recovered.contains("\"id\":\"us:intl\""));
    }

    #[test]
    fn console_uses_default_and_handles_azerty_without_touching_other_variants() {
        let mapping = "# Comment\nfr fr pc105 - options\nbe-latin1 be pc105 - options\ncf ca pc105 - options\nus us pc105 - options\nus-acentos us pc105 intl options\nil il pc105 - options\n";
        let available = "fr\nbe-latin1\ncf\nus\nil\nus-acentos\nfr-bepo\n";
        for (selection, expected) in [
            ("us,fr", "us"),
            ("fr,us", "fr"),
            ("be,us", "be-latin1"),
            ("ca,us", "cf"),
            ("us:intl,fr", "us-acentos"),
            ("fr:azerty,us", "fr"),
            ("fr:oss,us", "fr"),
            ("fr:bepo,us", "fr-bepo"),
            ("ara:azerty,fr", "fr"),
            ("il,us", "il"),
        ] {
            assert_eq!(
                console_keymap(selection, mapping, available).as_deref(),
                Some(expected)
            );
        }
        assert_eq!(console_keymap("fr:dvorak,us", mapping, available), None);
        assert_eq!(console_keymap("fr,us", mapping, "us\n"), None);
        assert_eq!(console_keymap("../fr", mapping, available), None);
    }

    #[test]
    fn registry_and_names_are_safe_and_complete() {
        let layouts = parse_registry(REGISTRY);
        assert_eq!(layouts.len(), 9);
        assert!(layouts.iter().any(|entry| entry.id == "us:intl"));
        assert!(!layouts.iter().any(|entry| entry.id == "bad"));
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }
    #[test]
    fn defaults_use_real_layouts_and_preserve_scripts() {
        let layouts = parse_registry(REGISTRY);
        assert_eq!(defaults("he", &layouts), "us,il");
        assert_eq!(defaults("de", &layouts), "de");
        assert_eq!(defaults("fr", &layouts), "fr");
        assert_eq!(defaults("sr_Latn", &layouts), "rs:latin");
        assert_eq!(defaults("pa_Arab", &layouts), "pk,us");
        assert_eq!(defaults("zh_Hans", &layouts), "us");
        let collisions = parse_registry(
            "! layout\nus English (US)\naf Dari\nam Armenian\net Amharic\nkz Kazakh\nba Bosnian\n",
        );
        assert_eq!(defaults("af", &collisions), "us");
        assert_eq!(defaults("am", &collisions), "et,us");
        assert_eq!(defaults("hy", &collisions), "am,us");
        assert_eq!(defaults("kk", &collisions), "kz,us");
        assert_eq!(defaults("bs", &collisions), "ba");
    }
    #[test]
    fn main_arrangements_are_distinct_without_suggestion_groups_or_minor_variants() {
        let layouts = parse_registry(
            "! layout\nus English (US)\nfr French\nde German\nbe Belgian\nca French (Canada)\nch German (Switzerland)\nrs Serbian\nil Hebrew\n\
             ! variant\nqwerty de: German (QWERTY)\nqwerty fr: French (QWERTY)\nazerty fr: French (AZERTY)\n\
             dvorak us: English (Dvorak)\ncolemak us: English (Colemak)\nbepo fr: French (BEPO)\nneo de: German (Neo 2)\n\
             nodeadkeys de: German (no dead keys)\nbepo_latin9 fr: French (BEPO, Latin-9 only)\ndvorak-intl us: English (Dvorak, intl., with dead keys)\n\
             eng ca: English (Canada)\nfr ch: French (Switzerland)\nlatin rs: Serbian (Latin)\nlatinyz rs: Serbian (Latin, QWERTY)\n",
        );
        let json = catalog_json_with("fr", "fr,us", &layouts, &Default::default());
        for id in [
            "de:qwerty",
            "fr:qwerty",
            "us:dvorak",
            "us:colemak",
            "fr:bepo",
            "de:neo",
            "ch:fr",
            "rs:latin",
            "rs:latinyz",
        ] {
            assert!(json.contains(&format!("\"id\":\"{id}\"")), "{id}");
        }
        for id in [
            "ca:eng",
            "de:nodeadkeys",
            "fr:bepo_latin9",
            "us:dvorak-intl",
            "fr:azerty",
        ] {
            assert!(!json.contains(&format!("\"id\":\"{id}\"")), "{id}");
        }
        assert!(json.contains("\"name\":\"Français (AZERTY)\""));
        assert!(json.contains("\"name\":\"German (QWERTZ)\""));
        assert!(json.contains("\"name\":\"French (Switzerland) (QWERTZ)\""));
        assert!(!json.contains("\"id\":\"ca:eng\""));
        assert!(!json.contains("suggested"));
        assert_eq!(defaults("fr", &layouts), "fr");
        assert_eq!(defaults("he", &layouts), "us,il");
    }
    #[test]
    fn editing_protects_default_last_layout_duplicates_and_limit() {
        let layouts = parse_registry(REGISTRY);
        assert_eq!(make_default("us,il,de", "il").as_deref(), Some("il,us,de"));
        assert_eq!(remove("il,us,de", "il").as_deref(), Some("us,de"));
        assert_eq!(remove("us", "us"), None);
        assert_eq!(add("us,il", "us", &layouts), None);
        assert_eq!(add("us,il,de,fr", "rs", &layouts), None);
        assert_eq!(add("us", "invented", &layouts), None);
        for invalid in [
            "",
            "us,us",
            "us,il,de,fr,rs",
            "../us",
            "us:\nOptions=bad",
            "us:",
            "us::intl",
        ] {
            assert!(parse_selection(invalid).is_none(), "{invalid}");
        }
    }
    #[test]
    fn drag_reorder_moves_atomically_and_top_is_the_default() {
        let original = "us,il,de,us:intl";
        assert_eq!(
            reorder_layout(original, "us:intl", 0).as_deref(),
            Some("us:intl,us,il,de")
        );
        assert_eq!(
            reorder_layout(original, "us", 3).as_deref(),
            Some("il,de,us:intl,us")
        );
        assert_eq!(
            reorder_layout(original, "us", 2).as_deref(),
            Some("il,de,us,us:intl")
        );
        for (selection, id, destination) in [
            (original, "us", 0),
            (original, "de", -1),
            (original, "de", 4),
            (original, "missing", 0),
            ("us,us", "us", 1),
            ("us", "us", 1),
            (original, "us", i32::MAX),
            ("us,il,de,fr,gb", "us", 1),
        ] {
            assert_eq!(reorder_layout(selection, id, destination), None);
        }
        let reordered = reorder_layout(original, "us:intl", 0).unwrap();
        let text = configuration("", &reordered, &parse_registry(REGISTRY)).unwrap();
        assert!(text.contains("LayoutList=us,us,il,de\n"));
        assert!(text.contains("VariantList=intl,,,\n"));
        let installed = parse_registry(REGISTRY);
        assert_eq!(
            crate::keyboard_profiles::recovered_selection(&reordered, true, "he", "IL", &installed),
            reordered
        );
    }
    #[test]
    fn reordering_swaps_neighbors_and_preserves_variants_and_default_order() {
        assert_eq!(
            move_layout("us,il,fr,de", "fr", -1).as_deref(),
            Some("us,fr,il,de")
        );
        assert_eq!(
            move_layout("us,il,fr,de", "il", 1).as_deref(),
            Some("us,fr,il,de")
        );
        assert_eq!(move_layout("us,il", "il", -1).as_deref(), Some("il,us"));
        assert_eq!(move_layout("us,il", "us", 1).as_deref(), Some("il,us"));
        for (selection, id, direction) in [
            ("us,il", "us", -1),
            ("us,il", "il", 1),
            ("us", "us", 1),
            ("us,il", "de", 1),
            ("us,il", "il", 0),
            ("us,il", "il", i32::MIN),
            ("us,us", "us", 1),
            ("us,il,de,fr,gb", "us", 1),
        ] {
            assert_eq!(move_layout(selection, id, direction), None);
        }
        let original = "us,il,de,us:intl";
        let reordered = move_layout(original, "us:intl", -1).unwrap();
        assert_eq!(reordered, "us,il,us:intl,de");
        assert_eq!(
            move_layout(&reordered, "us:intl", 1).as_deref(),
            Some(original)
        );
        let text = configuration("", &reordered, &parse_registry(REGISTRY)).unwrap();
        assert!(text.contains("LayoutList=us,il,us,de\n"));
        assert!(text.contains("VariantList=,,intl,\n"));
    }
    #[test]
    fn plasma_configuration_orders_default_and_aligns_empty_variants() {
        let layouts = parse_registry(REGISTRY);
        let existing = "[Other]\nKeep=true\n[Layout]\nModel=pc105\nOptions=compose:ralt\nLayoutList=fr\nVariantList=oss\nDisplayNames=old\n";
        let text = configuration(existing, "il,us:intl,de", &layouts).unwrap();
        for expected in [
            "Keep=true",
            "Model=pc105",
            "LayoutList=il,us,de\n",
            "VariantList=,intl,\n",
            "DisplayNames=\n",
            "Options=compose:ralt,grp:alt_shift_toggle,grp:win_space_toggle",
            "ResetOldOptions=true",
            "Use=true",
        ] {
            assert!(text.contains(expected), "{expected}");
        }
        assert_eq!(
            configuration(&text, "il,us:intl,de", &layouts).unwrap(),
            text
        );
        let single =
            configuration("[Layout]\nOptions=grp:alt_shift_toggle\n", "us", &layouts).unwrap();
        assert!(single.contains("VariantList=\n"));
        assert!(single.contains("Options=grp:alt_shift_toggle,grp:win_space_toggle"));
        assert!(configuration(existing, "fake", &layouts).is_err());
    }
}
