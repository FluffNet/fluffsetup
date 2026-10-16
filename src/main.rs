//! Fluff Linux first boot setup and its privileged helper modes.
//!
//! The default mode launches the QML interface. Internal command line modes
//! perform the small privileged steps requested by that interface and retain
//! the original terminal setup as a recovery option.

mod backend;
mod clock;
mod country;
mod i18n;
mod keyboard;
mod keyboard_names;
mod keyboard_profiles;
mod locale;
mod time_sync;
mod timezone;

use cxx_qt_lib::{QFont, QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const BOLD_RED: &str = "\x1b[1;31m";
const BOLD_GREEN: &str = "\x1b[1;32m";
const YELLOW: &str = "\x1b[33m";
const PURPLE: &str = "\x1b[35m";
const RESET: &str = "\x1b[0m";
const TEMPORARY_USER: &str = "fluffsetup";
const PLM_CONFIG: &str = "/etc/plasmalogin.conf.d/flufflinux.conf";
const SETUP_STATE_FILE: &str = "/home/fluffsetup/.local/state/fluffsetup/progress";
const SETUP_STATE_TEMP_FILE: &str = "/home/fluffsetup/.local/state/fluffsetup/progress.tmp";
const CLEANUP_FILES: &[&str] = &[
    SETUP_STATE_FILE,
    SETUP_STATE_TEMP_FILE,
    "/usr/share/wayland-sessions/fluffsetup.desktop",
    "/usr/lib/fluffsetup/fluffsetup-session",
    "/etc/sudoers.d/90-fluffsetup-temporary",
    "/var/lib/AccountsService/users/fluffsetup",
    "/var/lib/systemd/linger/fluffsetup",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SetupProgress {
    pub page: i32,
    pub hostname: String,
    pub display_name: String,
    pub username: String,
    pub account_created: bool,
    pub language: String,
    pub keyboards: String,
    pub keyboards_customized: bool,
    pub region: String,
    pub region_confirmed: bool,
    pub timezone: String,
    pub timezone_customized: bool,
}

impl Default for SetupProgress {
    fn default() -> Self {
        Self {
            page: 0,
            hostname: String::new(),
            display_name: String::new(),
            username: String::new(),
            account_created: false,
            language: "en".into(),
            keyboards: String::new(),
            keyboards_customized: false,
            region: String::new(),
            region_confirmed: false,
            timezone: String::new(),
            timezone_customized: false,
        }
    }
}

impl SetupProgress {
    pub(crate) fn resume_page(&self) -> i32 {
        if matches!(self.page, 2 | 3 | 4 | 5 | 7 | 9)
            && (!self.region_confirmed || !country::valid(&self.region))
        {
            return 8;
        }
        // A saved page number cannot certify successful finalization. Only
        // this process's completion result may unlock the Finish action.
        if self.page == 5 {
            return if timezone::valid(&self.timezone) {
                3
            } else {
                9
            };
        }
        self.page
    }
}

fn serialize_setup_progress(progress: &SetupProgress) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\nv6\n{}\n{}\n{}\n{}\n{}\n{}",
        progress.page,
        progress.hostname,
        progress.display_name,
        progress.username,
        progress.account_created,
        progress.language,
        progress.keyboards,
        progress.keyboards_customized,
        progress.region,
        progress.region_confirmed,
        progress.timezone,
        progress.timezone_customized
    )
}

fn parse_setup_progress(contents: &str) -> SetupProgress {
    let mut values = contents.lines();
    let stored_page = values.next().and_then(|value| value.parse::<i32>().ok());
    let hostname = values.next().unwrap_or_default().to_string();
    let display_name = values.next().unwrap_or_default().to_string();
    let username = values.next().unwrap_or_default().to_string();
    let account_created = values.next() == Some("true");
    let stored_language = values.next();
    let language = stored_language
        .and_then(i18n::normalize)
        .unwrap_or("en")
        .to_string();
    let version = values.next();
    let page = match (stored_page, version, stored_language) {
        (Some(page @ (0 | 1 | 2 | 3 | 5)), Some("v2" | "v3" | "v4" | "v5" | "v6"), _) => page,
        (Some(6), Some("v3" | "v4" | "v5" | "v6"), _) => 6,
        (Some(7), Some("v4" | "v5" | "v6"), _) => 7,
        (Some(8), Some("v5" | "v6"), _) => 8,
        (Some(9), Some("v6"), _) => 9,
        (Some(page @ (0 | 1 | 2 | 4)), None, None) => page + 1,
        _ => 0,
    };

    let stored_keyboards = values.next().unwrap_or_default();
    let keyboards = if matches!(version, Some("v4" | "v5" | "v6"))
        && keyboard::parse_selection(stored_keyboards).is_some()
    {
        stored_keyboards.to_string()
    } else {
        String::new()
    };
    let stored_customized = values.next() == Some("true");
    let keyboards_customized = !keyboards.is_empty() && stored_customized;
    let stored_region = values.next().unwrap_or_default();
    let region = if matches!(version, Some("v5" | "v6")) && country::valid(stored_region) {
        stored_region.into()
    } else {
        String::new()
    };
    let stored_confirmed = values.next() == Some("true");
    let region_confirmed = !region.is_empty() && stored_confirmed;
    let stored_timezone = values.next().unwrap_or_default();
    let timezone = if version == Some("v6") && timezone::valid(stored_timezone) {
        stored_timezone.into()
    } else {
        String::new()
    };
    let timezone_customized = values.next() == Some("true") && !timezone.is_empty();
    let page = if page == 3 && timezone.is_empty() {
        9
    } else {
        page
    };
    SetupProgress {
        page,
        hostname,
        display_name,
        username,
        account_created,
        language,
        keyboards,
        keyboards_customized,
        region,
        region_confirmed,
        timezone,
        timezone_customized,
    }
}

pub(crate) fn load_setup_progress() -> SetupProgress {
    fs::read_to_string(SETUP_STATE_FILE)
        .map(|contents| parse_setup_progress(&contents))
        .unwrap_or_default()
}

fn setup_progress_with_defaults(progress: &SetupProgress, default_hostname: &str) -> SetupProgress {
    let mut progress = progress.clone();
    if progress.hostname.is_empty() {
        progress.hostname = default_hostname.to_string();
    }
    if progress.display_name.trim().is_empty() {
        progress.display_name = default_display_name().to_string();
    }
    progress
}

pub(crate) fn save_setup_progress(progress: &SetupProgress) -> io::Result<()> {
    let progress = setup_progress_with_defaults(progress, &current_or_generated_hostname());

    let path = Path::new(SETUP_STATE_FILE);
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid setup state path"))?;
    fs::create_dir_all(parent)?;

    // Write and sync a complete replacement before changing the live state
    // file. A power loss can therefore leave either the old or new state, but
    // never a partially written recovery record.
    let temporary_path = Path::new(SETUP_STATE_TEMP_FILE);
    let mut file = File::create(temporary_path)?;
    file.write_all(serialize_setup_progress(&progress).as_bytes())?;
    file.sync_all()?;
    fs::rename(temporary_path, path)?;
    File::open(parent)?.sync_all()
}

unsafe extern "C" {
    fn fluffsetup_bind_setup_window();
    fn fluffsetup_initialize_session_background();
}

const HOSTNAME_REQUIREMENTS: &str = "System Name Requirements:
Allowed characters: letters (a-z and A-Z), digits (0-9), dash (-), and dot (.).
Cannot start or end with a dash (-) or a dot (.).
Special characters are not allowed (for example: @ and !)
No spaces allowed.
Max length: 255 characters.";

const NAME_REQUIREMENTS: &str = "Name Requirements:
Enter the name you want Fluff Linux to display for your account.
Names may use international characters, spaces, apostrophes, and hyphens.
A colon (:) and control characters are not allowed.
Max length: 128 characters.";

fn pause() {
    print!("\nPress Enter to close...");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
}

fn exit_with_error(message: &str) -> ! {
    eprintln!("\n{BOLD_RED}Error:{RESET} {message}");
    pause();
    std::process::exit(1);
}

fn run_sudo(command: &str, args: &[&str]) {
    let status = Command::new("/usr/bin/sudo")
        .arg("--")
        .arg(command)
        .args(args)
        .status()
        .unwrap_or_else(|error| {
            exit_with_error(&format!("Failed to run {command}: {error}"));
        });

    if !status.success() {
        exit_with_error(&format!("{command} failed."));
    }
}

fn run_sudo_with_input(command: &str, args: &[&str], input: &[u8]) {
    let mut child = Command::new("/usr/bin/sudo")
        .arg("--")
        .arg(command)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| {
            exit_with_error(&format!("Failed to run {command}: {error}"));
        });

    child
        .stdin
        .as_mut()
        .unwrap_or_else(|| exit_with_error(&format!("Failed to open {command} input.")))
        .write_all(input)
        .unwrap_or_else(|error| {
            exit_with_error(&format!("Failed to write to {command}: {error}"));
        });

    let status = child.wait().unwrap_or_else(|error| {
        exit_with_error(&format!("Failed to wait for {command}: {error}"));
    });

    if !status.success() {
        exit_with_error(&format!("{command} failed."));
    }
}

fn request_cleanup() {
    let status = Command::new("/usr/bin/sudo")
        .args(["--", "/usr/bin/fluffsetup", "--cleanup"])
        .status()
        .unwrap_or_else(|error| {
            exit_with_error(&format!("Failed to start FluffSetup cleanup: {error}"));
        });

    if !status.success() {
        exit_with_error("FluffSetup cleanup was not completed.");
    }
}

fn is_root() -> bool {
    Command::new("/usr/bin/id")
        .arg("-u")
        .output()
        .is_ok_and(|output| output.status.success() && output.stdout == b"0\n")
}

fn remove_file_if_present(path: &str) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn without_autologin_section(config: &str) -> String {
    let mut skip_autologin = false;
    let mut cleaned = String::new();
    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            skip_autologin = trimmed.eq_ignore_ascii_case("[Autologin]");
        }

        if !skip_autologin {
            cleaned.push_str(line);
            cleaned.push('\n');
        }
    }
    cleaned
}

fn clear_temporary_autologin() -> io::Result<()> {
    let config = match fs::read_to_string(PLM_CONFIG) {
        Ok(config) => config,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let cleaned = without_autologin_section(&config);

    if cleaned != config {
        fs::write(PLM_CONFIG, cleaned)?;
    }
    Ok(())
}

fn run_disable_autostart() -> ! {
    if !is_root() {
        eprintln!("Disabling FluffSetup automatic login must be run as root.");
        std::process::exit(1);
    }

    if let Err(error) = clear_temporary_autologin() {
        eprintln!("Could not disable FluffSetup automatic login: {error}");
        std::process::exit(1);
    }

    std::process::exit(0);
}

fn temporary_login_sessions() -> Vec<String> {
    let Ok(output) = Command::new("/usr/bin/loginctl")
        .args(["list-sessions", "--no-legend"])
        .output()
    else {
        return Vec::new();
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let columns: Vec<_> = line.split_whitespace().collect();
            (columns.len() >= 3 && columns[2] == TEMPORARY_USER).then(|| columns[0].to_string())
        })
        .collect()
}

fn remove_temporary_setup() -> Result<(), String> {
    // Disable automatic login first so an interruption cannot start another
    // temporary setup session on the next boot.
    clear_temporary_autologin()
        .map_err(|error| format!("Cleanup failed while disabling temporary login: {error}"))?;
    for path in CLEANUP_FILES {
        remove_file_if_present(path)
            .map_err(|error| format!("Cleanup failed while removing {path}: {error}"))?;
    }

    if user_exists(TEMPORARY_USER) {
        let account_removed = Command::new("/usr/bin/userdel")
            .args(["--remove", "--force", TEMPORARY_USER])
            .status()
            .is_ok_and(|status| status.success());
        if !account_removed {
            return Err("Cleanup failed while removing the temporary account.".to_string());
        }
    }

    if let Err(error) = fs::remove_dir_all("/home/fluffsetup")
        && error.kind() != io::ErrorKind::NotFound
    {
        return Err(format!(
            "Cleanup failed while removing the temporary home directory: {error}"
        ));
    }
    let _ = fs::remove_dir("/usr/lib/fluffsetup");
    remove_file_if_present("/usr/bin/fluffsetup")
        .map_err(|error| format!("Cleanup failed while removing FluffSetup: {error}"))?;
    Ok(())
}

fn run_prepare_summary() -> ! {
    if !is_root() {
        eprintln!("Preparing the completed setup summary must be run as root.");
        std::process::exit(1);
    }

    // The GUI only shows its success summary after this removal completes.
    // Linux keeps the running executable mapped until it exits.
    if let Err(error) = remove_temporary_setup() {
        eprintln!("{error}");
        std::process::exit(1);
    }
    std::process::exit(0);
}

fn run_cleanup() -> ! {
    if !is_root() {
        eprintln!("FluffSetup cleanup must be run as root.");
        std::process::exit(1);
    }

    let sessions = temporary_login_sessions();
    if let Err(error) = remove_temporary_setup() {
        eprintln!("{error}");
        std::process::exit(1);
    }

    // This is deliberately last: ending the temporary login returns PLM to its
    // greeter and may also terminate this helper's inherited session scope.
    if sessions.is_empty() {
        let _ = Command::new("/usr/bin/loginctl")
            .args(["terminate-user", TEMPORARY_USER])
            .status();
    } else {
        for session in sessions {
            let _ = Command::new("/usr/bin/loginctl")
                .args(["terminate-session", &session])
                .status();
        }
    }
    std::process::exit(0);
}

fn generate_hostname() -> String {
    let mut bytes = [0_u8; 6];

    let mut file = match File::open("/dev/urandom") {
        Ok(file) => file,
        Err(_) => return "flufflinux".to_string(),
    };

    if file.read_exact(&mut bytes).is_err() {
        return "flufflinux".to_string();
    }

    let first_letter = (b'A' + (bytes[0] % 26)) as char;
    let second_letter = (b'A' + (bytes[1] % 26)) as char;
    let mut digits = String::new();

    for byte in &bytes[2..6] {
        digits.push((b'0' + (byte % 10)) as char);
    }

    format!("FL-{first_letter}{second_letter}{digits}")
}

fn current_or_generated_hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|hostname| hostname.trim().to_string())
        .filter(|hostname| validate_hostname(hostname).is_ok())
        .unwrap_or_else(generate_hostname)
}

fn default_display_name() -> &'static str {
    "User"
}

fn effective_display_name(display_name: &str) -> &str {
    if display_name.trim().is_empty() {
        default_display_name()
    } else {
        display_name
    }
}

#[derive(Debug, PartialEq)]
enum HostnameIssue {
    Blank,
    TooLong,
    Whitespace,
    InvalidCharacters(String),
    StartDash,
    StartDot,
    EndDash,
    EndDot,
}

fn hostname_issues(hostname: &str) -> Vec<HostnameIssue> {
    if hostname.is_empty() {
        return vec![HostnameIssue::Blank];
    }
    let mut issues = Vec::new();
    if hostname.chars().count() > 255 {
        issues.push(HostnameIssue::TooLong);
    }
    if hostname.chars().any(char::is_whitespace) {
        issues.push(HostnameIssue::Whitespace);
    }
    // Whitespace has its own error. Do not report it again as an invalid symbol.
    let invalid_characters = hostname
        .chars()
        .filter(|character| {
            !character.is_whitespace()
                && !character.is_ascii_alphanumeric()
                && *character != '-'
                && *character != '.'
        })
        .fold(String::new(), |mut invalid, character| {
            if !invalid.contains(character) {
                invalid.push(character);
            }
            invalid
        });

    if !invalid_characters.is_empty() {
        issues.push(HostnameIssue::InvalidCharacters(invalid_characters));
    }

    if hostname.starts_with('-') {
        issues.push(HostnameIssue::StartDash);
    }

    if hostname.starts_with('.') {
        issues.push(HostnameIssue::StartDot);
    }

    if hostname.ends_with('-') {
        issues.push(HostnameIssue::EndDash);
    }

    if hostname.ends_with('.') {
        issues.push(HostnameIssue::EndDot);
    }
    issues
}

fn hostname_validation_message(hostname: &str, language: &str) -> String {
    hostname_issues(hostname)
        .into_iter()
        .map(|issue| {
            let key = match &issue {
                HostnameIssue::Blank => "hostnameBlank",
                HostnameIssue::TooLong => "hostnameLong",
                HostnameIssue::Whitespace => "hostnameWhitespace",
                HostnameIssue::InvalidCharacters(_) => "hostnameCharacters",
                HostnameIssue::StartDash => "hostnameStartDash",
                HostnameIssue::StartDot => "hostnameStartDot",
                HostnameIssue::EndDash => "hostnameEndDash",
                HostnameIssue::EndDot => "hostnameEndDot",
            };
            let text = i18n::translate_for(language, key);
            if let HostnameIssue::InvalidCharacters(characters) = issue {
                // Isolate user-entered symbols from surrounding RTL punctuation.
                let characters = if i18n::rtl(language) {
                    format!("\u{2068}{characters}\u{2069}")
                } else {
                    characters
                };
                text.replace("{characters}", &characters)
            } else {
                text
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_hostname(hostname: &str) -> Result<(), String> {
    let message = hostname_validation_message(hostname, "en");
    if message.is_empty() {
        Ok(())
    } else {
        Err(message)
    }
}

fn validate_username(username: &str) -> Result<(), &'static str> {
    if username.is_empty() {
        return Err("The user name cannot be blank");
    }

    if username.len() > 32 {
        return Err("The user name must be 32 characters or less");
    }

    if username.chars().any(char::is_whitespace) {
        return Err("The user name cannot have spaces");
    }

    if username.starts_with('-')
        || username.starts_with('_')
        || username.ends_with('-')
        || username.ends_with('_')
    {
        return Err("The user name cannot start or end with '_' or '-'");
    }

    if username
        .chars()
        .any(|character| !character.is_ascii_alphanumeric() && character != '-' && character != '_')
    {
        return Err(
            "The user name cannot have any special characters (Check requirements and try again)",
        );
    }

    if username
        .chars()
        .any(|character| character.is_ascii_uppercase())
    {
        return Err("The user name contains uppercase letters, only lowercase letters are allowed");
    }

    if !username
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_lowercase())
    {
        return Err("The user name must start with a lowercase letter");
    }

    if username.chars().all(|character| character.is_ascii_digit()) {
        return Err("The user name cannot be numbers only");
    }

    if username == "fluffsetup" {
        return Err("The user name \"fluffsetup\" is reserved for system setup");
    }

    Ok(())
}

fn validate_display_name(display_name: &str) -> Result<(), &'static str> {
    if display_name.trim().is_empty() {
        return Ok(());
    }

    if display_name.chars().count() > 128 {
        return Err("The name must be 128 characters or less");
    }

    if display_name.contains(':') {
        return Err("The name cannot contain a colon (:)");
    }

    if display_name.chars().any(char::is_control) {
        return Err("The name cannot contain control characters");
    }

    Ok(())
}

fn transliterate_with_icu(value: &str) -> Option<String> {
    let mut child = Command::new("/usr/bin/uconv")
        .args(["-x", "Any-Latin; Latin-ASCII", "-f", "UTF-8", "-t", "UTF-8"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    child.stdin.take()?.write_all(value.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn fallback_transliteration(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        let replacement = match character {
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => {
                "a"
            }
            'Æ' | 'æ' => "ae",
            'Ç' | 'ç' => "c",
            'Ð' | 'ð' => "d",
            'È' | 'É' | 'Ê' | 'Ë' | 'è' | 'é' | 'ê' | 'ë' => "e",
            'Ì' | 'Í' | 'Î' | 'Ï' | 'ì' | 'í' | 'î' | 'ï' => "i",
            'Ñ' | 'ñ' => "n",
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => {
                "o"
            }
            'Œ' | 'œ' => "oe",
            'Š' | 'š' => "s",
            'Ù' | 'Ú' | 'Û' | 'Ü' | 'ù' | 'ú' | 'û' | 'ü' => "u",
            'Ý' | 'Ÿ' | 'ý' | 'ÿ' => "y",
            'Ž' | 'ž' => "z",
            'א' => "a",
            'ב' => "b",
            'ג' => "g",
            'ד' => "d",
            'ה' => "h",
            'ו' => "o",
            'ז' => "z",
            'ח' => "h",
            'ט' => "t",
            'י' => "y",
            'כ' | 'ך' => "k",
            'ל' => "l",
            'מ' | 'ם' => "m",
            'נ' | 'ן' => "n",
            'ס' => "s",
            'ע' => "a",
            'פ' | 'ף' => "f",
            'צ' | 'ץ' => "ts",
            'ק' => "k",
            'ר' => "r",
            'ש' => "sh",
            'ת' => "t",
            _ if character.is_ascii() => {
                result.push(character);
                continue;
            }
            _ => "",
        };
        result.push_str(replacement);
    }
    result
}

fn transliterate_word(word: &str) -> String {
    transliterate_word_with(word, transliterate_with_icu)
}

fn transliterate_word_with(word: &str, convert: impl FnOnce(&str) -> Option<String>) -> String {
    // ICU follows general language transliteration rules. These common Hebrew
    // names use the spellings FluffSetup presents to its users.
    match word {
        "יעקוב" | "יעקב" => "yakov".to_string(),
        "כהן" => "cohen".to_string(),
        _ => {
            // ICU's Latin-ASCII step folds shin to "s". Preserve the intended
            // "sh" spelling before ICU runs, also when mixed with other scripts.
            let word = word.replace('ש', "sh");
            convert(&word).unwrap_or_else(|| fallback_transliteration(&word))
        }
    }
}

fn normalized_username_word(word: &str) -> String {
    transliterate_word(word)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn username_base_from_display_name(display_name: &str) -> String {
    let words: Vec<String> = display_name
        .split_whitespace()
        .map(normalized_username_word)
        .filter(|word| !word.is_empty())
        .collect();

    let mut username = match words.as_slice() {
        [] => "user".to_string(),
        [word] => word.clone(),
        [first, rest @ ..] => {
            let mut generated = first.clone();
            for word in rest {
                if let Some(initial) = word.chars().find(char::is_ascii_alphabetic) {
                    generated.push(initial);
                }
            }
            generated
        }
    };

    if !username
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_lowercase())
    {
        username.insert_str(0, "user");
    }
    username.truncate(32);
    username
}

fn unique_username_from_base<F>(base: &str, mut is_unavailable: F) -> String
where
    F: FnMut(&str) -> bool,
{
    let base = if base.is_empty() { "user" } else { base };
    let first = &base[..base.len().min(32)];
    if !is_unavailable(first) {
        return first.to_string();
    }

    for suffix in 2_u32.. {
        let suffix = suffix.to_string();
        let keep = 32_usize.saturating_sub(suffix.len());
        let stem = &base[..base.len().min(keep)];
        let candidate = format!("{stem}{suffix}");
        if !is_unavailable(&candidate) {
            return candidate;
        }
    }

    unreachable!("the numeric username suffix space is effectively unbounded")
}

fn generated_username(display_name: &str) -> String {
    let base = username_base_from_display_name(display_name);
    unique_username_from_base(&base, |candidate| {
        candidate == TEMPORARY_USER || user_exists(candidate)
    })
}

fn confirm_yes_default() -> bool {
    loop {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            exit_with_error("Failed to read your answer.");
        }

        let input = input.trim();
        if input.is_empty() || input.eq_ignore_ascii_case("y") || input.eq_ignore_ascii_case("yes")
        {
            return true;
        }

        if input.eq_ignore_ascii_case("n") || input.eq_ignore_ascii_case("no") {
            return false;
        }
    }
}

fn hostname_setup() -> String {
    loop {
        println!("\nPlease enter the hostname/system name you'd like to have");
        println!("(for example: my-pc, pc1, fluff-laptop)\n");
        println!(
            "If you are not sure, press Enter and FluffSetup will offer a randomized hostname"
        );
        print!("\nHostname: ");
        io::stdout().flush().unwrap_or_else(|error| {
            exit_with_error(&format!("Failed to display the prompt: {error}"));
        });

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap_or_else(|error| {
            exit_with_error(&format!("Failed to read the hostname: {error}"))
        });
        let hostname = input.trim();

        if hostname.is_empty() {
            let generated_hostname = generate_hostname();
            println!("\nNo hostname was entered. Offering a randomized hostname.\n");
            print!("Use generated hostname \"{generated_hostname}\"? [Y/n]: ");
            io::stdout().flush().ok();

            if confirm_yes_default() {
                return generated_hostname;
            }
            continue;
        }

        if let Err(error) = validate_hostname(hostname) {
            println!("\n{PURPLE}{HOSTNAME_REQUIREMENTS}{RESET}");
            println!("\n{BOLD_RED}Error:{RESET} {error}\n");
            continue;
        }

        print!("\nSet \"{hostname}\" as the hostname? [Y/n]: ");
        io::stdout().flush().ok();
        if confirm_yes_default() {
            return hostname.to_string();
        }
    }
}

fn user_exists(username: &str) -> bool {
    let found_by_getent = Command::new("/usr/bin/getent")
        .args(["passwd", username])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if found_by_getent {
        return true;
    }

    let found_in_passwd = fs::read_to_string("/etc/passwd").is_ok_and(|passwd| {
        passwd.lines().any(|line| {
            line.split_once(':')
                .is_some_and(|(account, _)| account == username)
        })
    });
    if found_in_passwd {
        return true;
    }

    Command::new("/usr/bin/id")
        .args(["-u", username])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub(crate) fn account_matches_setup_identity(username: &str, display_name: &str) -> bool {
    let Ok(passwd) = fs::read_to_string("/etc/passwd") else {
        return false;
    };

    account_matches_setup_identity_in(&passwd, username, display_name)
}

fn account_matches_setup_identity_in(passwd: &str, username: &str, display_name: &str) -> bool {
    let expected_home = format!("/home/{username}");

    passwd.lines().any(|line| {
        let mut fields = line.split(':');
        fields.next() == Some(username)
            && fields.next().is_some()
            && fields.next().is_some()
            && fields.next().is_some()
            && fields.next() == Some(display_name)
            && fields.next() == Some(expected_home.as_str())
            && fields.next() == Some("/bin/zsh")
    })
}

fn display_name_setup() -> String {
    loop {
        print!("\nName: ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .unwrap_or_else(|error| exit_with_error(&format!("Failed to read the name: {error}")));
        let display_name = input.trim_end_matches(['\r', '\n']);

        let selected = if display_name.trim().is_empty() {
            println!(
                "\nNo name was entered. Using \"{}\".\n",
                default_display_name()
            );
            default_display_name().to_string()
        } else {
            if let Err(error) = validate_display_name(display_name) {
                println!("\n{PURPLE}{NAME_REQUIREMENTS}{RESET}");
                println!("\n{BOLD_RED}Error:{RESET} {error}\n");
                continue;
            }

            print!("\nUse \"{display_name}\" as the name? [Y/n]: ");
            io::stdout().flush().ok();
            if !confirm_yes_default() {
                continue;
            }
            display_name.to_string()
        };

        return selected;
    }
}

fn password_setup() -> String {
    println!(
        "\n{YELLOW}NOTICE!{RESET} For your convenience, the password is visible.\n\
         Make sure only you can see the password.\n"
    );

    loop {
        print!("Please enter a password: ");
        io::stdout().flush().ok();

        let mut password = String::new();
        io::stdin()
            .read_line(&mut password)
            .unwrap_or_else(|error| {
                exit_with_error(&format!("Failed to read the password: {error}"))
            });
        let password = password.trim().to_string();

        if password.is_empty() {
            println!("\n{BOLD_RED}Error:{RESET} The password cannot be blank.\n");
            continue;
        }

        print!("\nRe-enter your password to confirm: ");
        io::stdout().flush().ok();

        let mut password_recheck = String::new();
        io::stdin()
            .read_line(&mut password_recheck)
            .unwrap_or_else(|error| {
                exit_with_error(&format!("Failed to read the password: {error}"))
            });

        if password != password_recheck.trim() {
            println!("\n{BOLD_RED}Error:{RESET} Passwords do not match. Please try again.\n");
            continue;
        }

        return password;
    }
}

fn create_permanent_user(display_name: &str, username: &str) {
    run_sudo(
        "/usr/bin/useradd",
        &[
            "-m",
            "-G",
            "uucp,wheel,kvm,libvirt",
            "-s",
            "/bin/zsh",
            "-c",
            display_name,
            username,
        ],
    );
}

fn configure_permanent_user(username: &str, password: &str) {
    let user_home = format!("/home/{username}");
    run_sudo(
        "/usr/bin/setfacl",
        &["-m", "u:libvirt-qemu:rwx", &user_home],
    );
    run_sudo(
        "/usr/bin/flatpak",
        &[
            "override",
            "--filesystem=home",
            "org.virt_manager.virt-manager",
        ],
    );
    let dolphinrc = format!("/home/{username}/.config/dolphinrc");
    let home_url = format!("s|^HomeUrl=/home/|HomeUrl=/home/{username}/|");
    run_sudo("/usr/bin/sed", &["-i", &home_url, &dolphinrc]);
    let trash_entry = format!("/home/{username}/Desktop/trash:⁄.desktop");
    run_sudo("/usr/bin/chown", &["root:root", &trash_entry]);
    let chpasswd_input = format!("{username}:{password}\n");
    run_sudo_with_input("/usr/bin/chpasswd", &[], chpasswd_input.as_bytes());
}

fn apply_settings(hostname: &str, display_name: &str, username: &str, password: &str) {
    run_sudo("/usr/bin/hostnamectl", &["set-hostname", hostname]);
    create_permanent_user(display_name, username);
    configure_permanent_user(username, password);
}

#[derive(Clone, Copy)]
struct SetupRequest<'a> {
    hostname: &'a str,
    display_name: &'a str,
    username: &'a str,
    password: &'a str,
    language: &'a str,
    region: &'a str,
    keyboards: &'a str,
    timezone: &'a str,
}

fn apply_gui_settings(request: SetupRequest<'_>) {
    let SetupRequest {
        hostname,
        display_name,
        username,
        password,
        language,
        region,
        keyboards,
        timezone,
    } = request;
    run_sudo("/usr/bin/hostnamectl", &["set-hostname", hostname]);

    if user_exists(username) {
        if !account_matches_setup_identity(username, display_name) {
            exit_with_error(
                "The reserved account name now belongs to a different system account. FluffSetup will not modify it.",
            );
        }
    } else {
        create_permanent_user(display_name, username);
    }

    // Record creation immediately after useradd. If power is lost before this
    // write, the saved reservation still identifies the same account.
    let previous = load_setup_progress();
    save_setup_progress(&SetupProgress {
        page: 3,
        hostname: hostname.to_string(),
        display_name: display_name.to_string(),
        username: username.to_string(),
        account_created: true,
        language: language.to_string(),
        keyboards: keyboards.to_string(),
        keyboards_customized: previous.keyboards_customized,
        region: region.to_string(),
        region_confirmed: previous.region_confirmed,
        timezone: timezone.to_string(),
        timezone_customized: previous.timezone_customized,
    })
    .unwrap_or_else(|error| {
        exit_with_error(&format!("Could not record the created account: {error}"))
    });

    configure_permanent_user(username, password);
    apply_selected_language(username, language, region);
    apply_selected_keyboards(username, keyboards);
    apply_selected_timezone(timezone);
}

fn apply_selected_timezone(timezone: &str) {
    let executable = env::current_exe()
        .unwrap_or_else(|error| exit_with_error(&format!("Cannot locate FluffSetup: {error}")));
    let executable = executable
        .to_str()
        .unwrap_or_else(|| exit_with_error("Invalid executable path."));
    run_sudo(executable, &["--apply-timezone", timezone]);
}

fn apply_selected_keyboards(username: &str, keyboards: &str) {
    let executable = env::current_exe()
        .unwrap_or_else(|error| exit_with_error(&format!("Cannot locate FluffSetup: {error}")));
    let executable = executable
        .to_str()
        .unwrap_or_else(|| exit_with_error("Invalid executable path."));
    run_sudo(executable, &["--apply-keyboard", username, keyboards]);
}

fn apply_selected_language(username: &str, language: &str, region: &str) {
    let executable = env::current_exe()
        .unwrap_or_else(|error| exit_with_error(&format!("Cannot locate FluffSetup: {error}")));
    let executable = executable
        .to_str()
        .unwrap_or_else(|| exit_with_error("Invalid executable path."));
    run_sudo(
        executable,
        &["--apply-language", username, language, region],
    );
}

fn run_setup() {
    print!("\x1B[2J\x1B[1;1H");
    println!("fluffsetup 1.5 - Fluff Linux Setup\n");
    println!("Welcome to FluffSetup.");
    println!("This temporary setup will create your account and name your system.");

    let hostname = hostname_setup();
    let display_name = display_name_setup();
    let username = generated_username(&display_name);
    let password = password_setup();

    println!("\nApplying your settings...");
    apply_settings(&hostname, &display_name, &username, &password);

    println!(
        "\n{BOLD_GREEN}Account setup completed successfully.{RESET}\n\
         Name: {display_name}\n\
         System name: {hostname}"
    );
    print!("\nPress Enter to finish setup and open the login screen...");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    let progress = load_setup_progress();
    let region = if country::valid(&progress.region) {
        progress.region.clone()
    } else {
        country::default_country(&progress.language)
    };
    apply_selected_language(&username, &progress.language, &region);
    let keyboards =
        if keyboard::validate_selection(&progress.keyboards, keyboard::catalog()).is_ok() {
            progress.keyboards
        } else {
            keyboard::defaults(&progress.language, keyboard::catalog())
        };
    apply_selected_keyboards(&username, &keyboards);
    // The terminal fallback has no timezone picker. Apply a recovered explicit
    // choice, but do not guess or alter the existing zone without one.
    if timezone::valid(&progress.timezone) {
        apply_selected_timezone(&progress.timezone);
    }
    request_cleanup();
}

pub(crate) fn setup_payload(fields: [&str; 8]) -> String {
    fields.join("\n")
}

fn parse_setup_payload(input: &str) -> Result<[&str; 8], &'static str> {
    let fields: Vec<_> = input.splitn(8, '\n').collect();
    let fields: [&str; 8] = fields.try_into().map_err(|_| "Incomplete setup details.")?;
    Ok(fields.map(|value| value.trim_end_matches('\r')))
}

fn run_gui_apply() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|error| exit_with_error(&format!("Failed to read setup details: {error}")));
    let [
        hostname,
        entered_display_name,
        username,
        language,
        region,
        keyboards,
        timezone,
        password,
    ] = parse_setup_payload(&input).unwrap_or_else(|error| exit_with_error(error));
    let display_name = effective_display_name(entered_display_name);
    let password = password.trim_end_matches(['\r', '\n']);

    if let Err(error) = validate_hostname(hostname) {
        exit_with_error(&error);
    }
    if let Err(error) = validate_display_name(display_name) {
        exit_with_error(error);
    }
    if password.is_empty() {
        exit_with_error("The password cannot be blank.");
    }

    if let Err(error) = validate_username(username) {
        exit_with_error(error);
    }
    if i18n::metadata(language).is_none() {
        exit_with_error("Unsupported setup language.");
    }
    if !country::valid(region) {
        exit_with_error("Unsupported geographic region.");
    }
    keyboard::validate_selection(keyboards, keyboard::catalog())
        .unwrap_or_else(|error| exit_with_error(&error));
    if !timezone::valid(timezone) {
        exit_with_error("Invalid or unavailable timezone.");
    }
    apply_gui_settings(SetupRequest {
        hostname,
        display_name,
        username,
        password,
        language,
        region,
        keyboards,
        timezone,
    });

    // The summary is shown only after every temporary setup artifact is gone.
    // The running Wayland session remains alive in memory until Finish exits
    // this process, at which point the session launcher returns to PLM.
    let executable = env::current_exe()
        .unwrap_or_else(|error| exit_with_error(&format!("Cannot locate FluffSetup: {error}")));
    let executable = executable
        .to_str()
        .unwrap_or_else(|| exit_with_error("FluffSetup's executable path is not valid UTF-8."));
    run_sudo(executable, &["--prepare-summary"]);
}

fn launch_gui() {
    let language = if preview_mode() {
        preview_argument("language").unwrap_or_else(|| i18n::language().to_string())
    } else {
        load_setup_progress().language
    };
    if !i18n::set_language(&language) {
        i18n::set_language("en");
    }
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    QGuiApplication::set_desktop_file_name(&"fluffsetup".into());
    QQuickStyle::set_style(&"Fusion".into());

    if let Some(family) = preview_argument("font") {
        let mut font = QFont::default();
        font.set_family(&QString::from(&family));
        if let Some(app) = app.as_mut() {
            app.set_application_font(&font);
        }
    }

    if !preview_mode() {
        unsafe {
            // SAFETY: both functions are implemented by native/session.cpp and
            // are called on Qt's GUI thread after QGuiApplication is constructed.
            fluffsetup_initialize_session_background();
        }
    }
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/org/flufflinux/setup/qml/Main.qml"));
    }
    if !preview_mode() {
        unsafe {
            // SAFETY: the QML engine has created the setup window, and the native
            // helper only inspects and binds Qt top level windows on this thread.
            fluffsetup_bind_setup_window();
        }
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

fn preview_mode() -> bool {
    env::args().any(|argument| argument == "--preview")
}

fn preview_argument(name: &str) -> Option<String> {
    if !preview_mode() {
        return None;
    }
    let prefix = format!("--{name}=");
    env::args().find_map(|argument| argument.strip_prefix(&prefix).map(str::to_owned))
}

fn main() {
    if preview_mode() {
        launch_gui();
    } else if env::args().any(|argument| argument == "--apply-keyboard") {
        let arguments: Vec<_> = env::args().collect();
        if arguments.len() != 4 || arguments[1] != "--apply-keyboard" {
            exit_with_error("Invalid keyboard helper arguments.");
        }
        keyboard::apply_for_user(&arguments[2], &arguments[3])
            .unwrap_or_else(|error| exit_with_error(&error));
    } else if env::args().any(|argument| argument == "--apply-user-keyboard") {
        let arguments: Vec<_> = env::args().collect();
        if arguments.len() != 3 || arguments[1] != "--apply-user-keyboard" {
            exit_with_error("Invalid user keyboard helper arguments.");
        }
        keyboard::apply_user(&arguments[2]).unwrap_or_else(|error| exit_with_error(&error));
    } else if env::args().any(|argument| argument == "--apply-timezone") {
        let arguments: Vec<_> = env::args().collect();
        if arguments.len() != 3 || arguments[1] != "--apply-timezone" {
            exit_with_error("Invalid timezone helper arguments.");
        }
        timezone::apply(&arguments[2]).unwrap_or_else(|error| exit_with_error(&error));
    } else if env::args().any(|argument| argument == "--apply-language") {
        let arguments: Vec<_> = env::args().collect();
        if arguments.len() != 5 || arguments[1] != "--apply-language" {
            exit_with_error("Invalid language helper arguments.");
        }
        locale::apply_system_and_user(&arguments[2], &arguments[3], &arguments[4])
            .unwrap_or_else(|error| exit_with_error(&error));
    } else if env::args().any(|argument| argument == "--apply-user-language") {
        let arguments: Vec<_> = env::args().collect();
        if arguments.len() != 4 || arguments[1] != "--apply-user-language" {
            exit_with_error("Invalid user language helper arguments.");
        }
        locale::apply_user(&arguments[2], &arguments[3])
            .unwrap_or_else(|error| exit_with_error(&error));
    } else if env::args().any(|argument| argument == "--cleanup") {
        run_cleanup();
    } else if env::args().any(|argument| argument == "--prepare-summary") {
        run_prepare_summary();
    } else if env::args().any(|argument| argument == "--disable-autostart") {
        run_disable_autostart();
    } else if env::args().any(|argument| argument == "--apply-setup") {
        run_gui_apply();
    } else if env::args().any(|argument| argument == "--terminal") {
        run_setup();
    } else {
        launch_gui();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CLEANUP_FILES, HostnameIssue, SETUP_STATE_FILE, SETUP_STATE_TEMP_FILE, SetupProgress,
        account_matches_setup_identity_in, fallback_transliteration, hostname_issues,
        hostname_validation_message, i18n, parse_setup_progress, serialize_setup_progress,
        setup_progress_with_defaults, transliterate_word_with, unique_username_from_base,
        username_base_from_display_name, validate_display_name, validate_hostname,
        validate_username, without_autologin_section,
    };

    #[test]
    fn recovery_requires_a_confirmed_region_and_never_unlocks_finish() {
        let mut progress = SetupProgress {
            language: "en".into(),
            region: "IL".into(),
            region_confirmed: true,
            timezone: "Asia/Jerusalem".into(),
            ..SetupProgress::default()
        };
        for page in [2, 3, 4, 5, 7, 9] {
            progress.page = page;
            assert_eq!(progress.resume_page(), if page == 5 { 3 } else { page });
            progress.region_confirmed = false;
            assert_eq!(progress.resume_page(), 8);
            progress.region_confirmed = true;
            progress.region = "XX".into();
            assert_eq!(progress.resume_page(), 8);
            progress.region = "IL".into();
        }
        progress.page = 5;
        progress.timezone.clear();
        assert_eq!(progress.resume_page(), 9);
        for page in [0, 1, 6, 8] {
            progress.page = page;
            assert_eq!(progress.resume_page(), page);
        }
    }

    #[test]
    fn validates_hostnames() {
        assert!(validate_hostname("fluff-laptop").is_ok());
        assert!(validate_hostname("FL-PC.example").is_ok());
        assert_eq!(
            validate_hostname("-bad").unwrap_err(),
            "The system name cannot start with a dash (-)."
        );
        assert_eq!(
            validate_hostname("bad.").unwrap_err(),
            "The system name cannot end with a dot (.)."
        );
        assert_eq!(
            validate_hostname("bad name").unwrap_err(),
            "Spaces and other whitespace are not allowed in the system name."
        );
        assert_eq!(
            validate_hostname("bad@name!").unwrap_err(),
            "The following characters are not allowed in the system name: '@!'."
        );
        assert_eq!(
            validate_hostname("שלום").unwrap_err(),
            "The following characters are not allowed in the system name: 'שלום'."
        );
        assert_eq!(
            validate_hostname(&"a".repeat(256)).unwrap_err(),
            "The system name is too long. Use no more than 255 characters."
        );
    }

    #[test]
    fn hostname_errors_are_specific_and_combined() {
        assert_eq!(
            hostname_issues("my laptop"),
            vec![HostnameIssue::Whitespace]
        );
        assert_eq!(
            hostname_issues("my\tlaptop\n"),
            vec![HostnameIssue::Whitespace]
        );
        assert_eq!(
            hostname_issues("fluff@@pc"),
            vec![HostnameIssue::InvalidCharacters("@".into())]
        );
        assert_eq!(
            hostname_issues("-my laptop@."),
            vec![
                HostnameIssue::Whitespace,
                HostnameIssue::InvalidCharacters("@".into()),
                HostnameIssue::StartDash,
                HostnameIssue::EndDot,
            ]
        );
        assert_eq!(hostname_issues(""), vec![HostnameIssue::Blank]);
        assert_eq!(
            hostname_issues(".pc-"),
            vec![HostnameIssue::StartDot, HostnameIssue::EndDash]
        );
        assert_eq!(
            hostname_issues(&format!("-{} @.", "a".repeat(256))).len(),
            5
        );
        assert!(hostname_issues("fluff-pc.example").is_empty());
        assert_eq!(
            hostname_issues(&"א".repeat(100)),
            vec![HostnameIssue::InvalidCharacters("א".into())]
        );
        assert_eq!(
            hostname_validation_message("my laptop", "he"),
            "אין להשתמש ברווחים או בתווי רווח אחרים בשם המערכת."
        );
        for language in i18n::LANGUAGES {
            let message = hostname_validation_message("-my laptop@.", language);
            assert_eq!(message.lines().count(), 4, "{language}");
            assert!(!message.contains("{characters}"), "{language}");
            assert!(!message.contains("hostname"), "{language}");
        }
        assert_eq!(
            hostname_validation_message("my laptop", "en")
                .lines()
                .count(),
            1
        );
        assert!(hostname_validation_message("fluff@pc", "he").contains("\u{2068}@\u{2069}"));
    }

    #[test]
    fn validates_user_names() {
        assert!(validate_username("shai").is_ok());
        assert!(validate_username("fluff_user-1").is_ok());
        assert!(validate_username("Fluff").is_err());
        assert!(validate_username("fluffsetup").is_err());
        assert!(validate_username("1234").is_err());
    }

    #[test]
    fn validates_display_names_without_restricting_normal_names() {
        assert!(validate_display_name("Shai Zedaka").is_ok());
        assert!(validate_display_name("O'Connor-Smith").is_ok());
        assert!(validate_display_name("יעקוב כהן").is_ok());
        assert!(validate_display_name("").is_ok());
        assert!(validate_display_name(" ").is_ok());
        assert!(validate_display_name("Name:Root").is_err());
        assert!(validate_display_name("Name\nRoot").is_err());
    }

    #[test]
    fn derives_expected_internal_usernames() {
        for (display_name, expected) in [
            ("shy", "shy"),
            ("Shy", "shy"),
            ("SHY", "shy"),
            ("Shai", "shai"),
            ("Shai Zedaka", "shaiz"),
            ("John Michael Smith", "johnms"),
            ("יעקוב", "yakov"),
            ("יעקוב כהן", "yakovc"),
            ("שי", "shy"),
            ("שי כהן", "shyc"),
        ] {
            assert_eq!(username_base_from_display_name(display_name), expected);
        }
    }

    #[test]
    fn preserves_hebrew_shin_with_or_without_icu() {
        for (word, prepared, expected) in [
            ("שי", "shי", "shy"),
            ("ש", "sh", "sh"),
            ("שש", "shsh", "shsh"),
            ("aשb", "ashb", "ashb"),
        ] {
            let converted = transliterate_word_with(word, |input| {
                assert_eq!(input, prepared);
                // Model ICU's existing shin-to-s behavior so this regression
                // also detects a failure when ICU is absent on the test host.
                Some(fallback_transliteration(&input.replace('ש', "s")))
            });
            assert_eq!(converted, expected);
            assert_eq!(transliterate_word_with(word, |_| None), expected);
        }
    }

    #[test]
    fn adds_a_numeric_suffix_until_the_username_is_unused() {
        let generated =
            unique_username_from_base("shy", |candidate| matches!(candidate, "shy" | "shy2"));
        assert_eq!(generated, "shy3");

        let protected_system_name =
            unique_username_from_base("root", |candidate| candidate == "root");
        assert_eq!(protected_system_name, "root2");
    }

    #[test]
    fn generated_username_base_is_safe_even_without_transliterable_text() {
        let generated = username_base_from_display_name("🎉 🎈");
        assert_eq!(generated, "user");
        assert!(validate_username(&generated).is_ok());
    }

    #[test]
    fn cleanup_covers_every_temporary_payload() {
        for path in [
            SETUP_STATE_FILE,
            SETUP_STATE_TEMP_FILE,
            "/usr/share/wayland-sessions/fluffsetup.desktop",
            "/usr/lib/fluffsetup/fluffsetup-session",
            "/etc/sudoers.d/90-fluffsetup-temporary",
        ] {
            assert!(CLEANUP_FILES.contains(&path));
        }
    }

    #[test]
    fn recovery_state_round_trips_without_a_password() {
        let progress = SetupProgress {
            page: 3,
            hostname: "FL-IW2098".to_string(),
            display_name: "Shai Zedaka".to_string(),
            username: "shaiz".to_string(),
            account_created: false,
            language: "he".into(),
            keyboards: "us,il".into(),
            keyboards_customized: true,
            region: "IL".into(),
            region_confirmed: true,
            timezone: "Asia/Jerusalem".into(),
            timezone_customized: true,
        };
        let serialized = serialize_setup_progress(&progress);

        assert_eq!(
            serialized,
            "3\nFL-IW2098\nShai Zedaka\nshaiz\nfalse\nhe\nv6\nus,il\ntrue\nIL\ntrue\nAsia/Jerusalem\ntrue"
        );
        assert_eq!(parse_setup_progress(&serialized), progress);
        assert!(!serialized.to_ascii_lowercase().contains("password"));
        let mut reordered = progress;
        for selection in ["il,us", "us,fr:oss,il,de:neo", "de:neo,us,il,fr:oss"] {
            reordered.keyboards = selection.into();
            let restored = parse_setup_progress(&serialize_setup_progress(&reordered));
            assert_eq!(restored, reordered);
            assert_eq!(restored.keyboards, selection);
            assert!(restored.keyboards_customized);
        }
        for language in ["en", "he", "fr", "ru", "ja", "sr_Latn", "ca_valencia"] {
            for region in crate::country::codes() {
                reordered.language = language.into();
                reordered.region = region.clone();
                assert_eq!(
                    parse_setup_progress(&serialize_setup_progress(&reordered)),
                    reordered
                );
            }
        }
    }

    #[test]
    fn finalization_payload_keeps_geography_language_and_keyboard_order_separate() {
        for (language, region) in [("en", "IL"), ("he", "US"), ("fr", "BE"), ("cs", "JP")] {
            let fields = [
                "fluff-pc",
                "User",
                "user",
                language,
                region,
                "il,us",
                "Asia/Jerusalem",
                "example-secret",
            ];
            let payload = super::setup_payload(fields);
            assert_eq!(super::parse_setup_payload(&payload).unwrap(), fields);
            let crlf = payload.replace('\n', "\r\n");
            assert_eq!(super::parse_setup_payload(&crlf).unwrap(), fields);
            assert!(
                super::parse_setup_payload("fluff-pc\nUser\nuser\nen\nus,il\nexample-secret")
                    .is_err()
            );
        }
    }

    #[test]
    fn older_recovery_state_remains_readable() {
        assert_eq!(
            parse_setup_progress("2\nfluff-pc\nUser"),
            SetupProgress {
                page: 9,
                hostname: "fluff-pc".to_string(),
                display_name: "User".to_string(),
                username: String::new(),
                account_created: false,
                language: "en".into(),
                ..SetupProgress::default()
            }
        );
    }

    #[test]
    fn timezone_recovery_preserves_selection_and_migrates_unfinished_accounts() {
        let state = SetupProgress {
            page: 9,
            language: "he".into(),
            region: "IL".into(),
            region_confirmed: true,
            timezone: "Europe/Brussels".into(),
            ..SetupProgress::default()
        };
        assert_eq!(
            parse_setup_progress(&serialize_setup_progress(&state)),
            state
        );
        for id in ["", "../etc/passwd", "Asia/Unavailable"] {
            let invalid = SetupProgress {
                page: 3,
                timezone: id.into(),
                ..state.clone()
            };
            let recovered = parse_setup_progress(&serialize_setup_progress(&invalid));
            assert_eq!(recovered.page, 9);
            assert!(recovered.timezone.is_empty());
            assert!(!recovered.timezone_customized);
            assert_eq!(recovered.region, "IL");
        }
        assert_eq!(
            parse_setup_progress("3\nfluff-pc\nUser\nuser\nfalse\nhe\nv5\nus,il\ntrue\nIL\ntrue")
                .page,
            9
        );
        let corrupt = parse_setup_progress(
            "9\nfluff-pc\nUser\nuser\nfalse\nhe\nv6\nus;bad\ntrue\nXX\ntrue\nUTC",
        );
        assert_eq!(corrupt.timezone, "UTC");
        assert!(corrupt.region.is_empty() && corrupt.keyboards.is_empty());
        for customized in [false, true] {
            let selection = SetupProgress {
                timezone_customized: customized,
                ..state.clone()
            };
            let restored = parse_setup_progress(&serialize_setup_progress(&selection));
            assert_eq!(restored.timezone_customized, customized);
            assert_eq!(restored.timezone, "Europe/Brussels");
        }
    }

    #[test]
    fn recovery_state_replaces_empty_gui_values_with_defaults() {
        let normalized = setup_progress_with_defaults(
            &SetupProgress {
                page: 2,
                hostname: String::new(),
                display_name: "   ".to_string(),
                username: String::new(),
                account_created: false,
                language: "en".into(),
                ..SetupProgress::default()
            },
            "FL-IW2098",
        );

        assert_eq!(normalized.hostname, "FL-IW2098");
        assert_eq!(normalized.display_name, "User");
    }

    #[test]
    fn only_recognizes_the_exact_reserved_account_identity() {
        let passwd =
            "root:x:0:0:root:/root:/bin/bash\nshaiz:x:1000:1000:Shai Zedaka:/home/shaiz:/bin/zsh\n";
        assert!(account_matches_setup_identity_in(
            passwd,
            "shaiz",
            "Shai Zedaka"
        ));
        assert!(!account_matches_setup_identity_in(
            passwd,
            "shaiz",
            "Someone Else"
        ));
        assert!(!account_matches_setup_identity_in(passwd, "root", "root"));
    }

    #[test]
    fn recovery_state_rejects_non_resumable_pages() {
        let state =
            parse_setup_progress("8\nfluff-pc\nUser\n\nfalse\nhe\nv5\nil,us\ntrue\nFR\ntrue");
        assert_eq!(state.page, 8);
        assert_eq!(state.language, "he");
        assert_eq!(state.region, "FR");
        assert!(state.region_confirmed && state.keyboards_customized);
        for invalid in ["PS", "XX", "", "il", "IL;bad"] {
            let state = parse_setup_progress(&format!(
                "8\n\n\n\nfalse\nhe\nv5\nus,il\ntrue\n{invalid}\ntrue"
            ));
            assert!(state.region.is_empty());
            assert!(!state.region_confirmed);
            assert_eq!(state.keyboards, "us,il");
        }
        assert_eq!(
            parse_setup_progress("8\n\n\n\nfalse\nhe\nv4\nus,il\ntrue").page,
            0
        );
        assert_eq!(
            parse_setup_progress("7\nfluff-pc\nUser\nuser\nfalse\nhe\nv4\nil,us:intl\ntrue")
                .keyboards,
            "il,us:intl"
        );
        assert!(
            parse_setup_progress("7\nfluff-pc\nUser\nuser\nfalse\nhe\nv4\nil,us:intl\ntrue")
                .keyboards_customized
        );
        for version in ["v2", "v3"] {
            let old = parse_setup_progress(&format!("1\nfluff-pc\nUser\n\nfalse\nhe\n{version}"));
            assert_eq!(old.page, 1);
            assert!(old.keyboards.is_empty());
            assert!(!old.keyboards_customized);
        }
        for invalid in ["us,us", "", "us;bad", "us,il,de,fr,gb"] {
            let state = parse_setup_progress(&format!(
                "7\nfluff-pc\nUser\n\nfalse\nhe\nv4\n{invalid}\ntrue"
            ));
            assert!(state.keyboards.is_empty());
            assert!(!state.keyboards_customized);
        }
        assert_eq!(parse_setup_progress("").page, 0);
        assert_eq!(parse_setup_progress("invalid\nfluff-pc\nUser").page, 0);
        assert_eq!(parse_setup_progress("3\nfluff-pc\nUser").page, 0);
        assert_eq!(parse_setup_progress("999\nfluff-pc\nUser").page, 0);
        assert_eq!(
            parse_setup_progress("4\nfluff-pc\nUser\nuser\ntrue\nhe\nv2").page,
            0
        );
    }

    #[test]
    fn language_and_page_recover_for_every_choice_and_legacy_pages_migrate() {
        assert_eq!(SetupProgress::default().language, "en");
        assert_eq!(SetupProgress::default().page, 0);
        for language in crate::i18n::LANGUAGES {
            for page in [0, 1, 2, 3, 5, 6, 7, 8, 9] {
                let progress = SetupProgress {
                    page,
                    language: language.to_string(),
                    timezone: "UTC".into(),
                    ..SetupProgress::default()
                };
                assert_eq!(
                    parse_setup_progress(&serialize_setup_progress(&progress)),
                    progress
                );
            }
        }
        for region in crate::i18n::regions() {
            let progress = SetupProgress {
                page: 6,
                language: crate::i18n::canonical_choice(region.code).unwrap().into(),
                ..SetupProgress::default()
            };
            assert_eq!(
                parse_setup_progress(&serialize_setup_progress(&progress)),
                progress
            );
        }
        for (old, new) in [(0, 1), (1, 2), (2, 9), (4, 5)] {
            assert_eq!(
                parse_setup_progress(&format!("{old}\nfluff-pc\nUser\nuser\ntrue")).page,
                new
            );
        }
        assert_eq!(
            parse_setup_progress("0\n\n\n\nfalse\nzz\nv2").language,
            "en"
        );
        assert_eq!(
            parse_setup_progress("3\nfluff-pc\nUser\nuser\ntrue\nhe\nv99").page,
            0
        );
    }

    #[test]
    fn cleanup_preserves_general_login_settings_and_removes_autologin() {
        let config = "[General]\nNumLock=on\n\n[Autologin]\nUser=fluffsetup\nSession=fluffsetup.desktop\nRelogin=true\n";
        assert_eq!(
            without_autologin_section(config),
            "[General]\nNumLock=on\n\n"
        );
    }
}
