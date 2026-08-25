//! Fluff Linux first boot setup and its privileged helper modes.
//!
//! The default mode launches the QML interface. Internal command line modes
//! perform the small privileged steps requested by that interface and retain
//! the original terminal setup as a recovery option.

mod backend;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QUrl};
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SetupProgress {
    pub page: i32,
    pub hostname: String,
    pub display_name: String,
    pub username: String,
    pub account_created: bool,
}

fn serialize_setup_progress(progress: &SetupProgress) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        progress.page,
        progress.hostname,
        progress.display_name,
        progress.username,
        progress.account_created
    )
}

fn parse_setup_progress(contents: &str) -> SetupProgress {
    let mut values = contents.lines();
    let page = values
        .next()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|page| matches!(page, 0 | 1 | 2 | 4))
        .unwrap_or_default();
    let hostname = values.next().unwrap_or_default().to_string();
    let display_name = values.next().unwrap_or_default().to_string();
    let username = values.next().unwrap_or_default().to_string();
    let account_created = values.next() == Some("true");

    SetupProgress {
        page,
        hostname,
        display_name,
        username,
        account_created,
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
    let mut file = File::create(&temporary_path)?;
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

fn validate_hostname(hostname: &str) -> Result<(), String> {
    if hostname.is_empty() {
        return Err("The system name cannot be blank".to_string());
    }

    if hostname.len() > 255 {
        return Err("The system name is too long. Use no more than 255 characters.".to_string());
    }

    if hostname.chars().any(char::is_whitespace) {
        return Err("Spaces and other whitespace are not allowed in the system name.".to_string());
    }

    let invalid_characters = hostname
        .chars()
        .filter(|character| {
            !character.is_ascii_alphanumeric() && *character != '-' && *character != '.'
        })
        .fold(String::new(), |mut invalid, character| {
            if !invalid.contains(character) {
                invalid.push(character);
            }
            invalid
        });

    if !invalid_characters.is_empty() {
        return Err(format!(
            "The character{} '{}' {} not allowed. Use English letters, numbers, dashes (-), and dots (.) only.",
            if invalid_characters.chars().count() == 1 {
                ""
            } else {
                "s"
            },
            invalid_characters,
            if invalid_characters.chars().count() == 1 {
                "is"
            } else {
                "are"
            }
        ));
    }

    if hostname.starts_with('-') {
        return Err("The system name cannot start with a dash (-).".to_string());
    }

    if hostname.starts_with('.') {
        return Err("The system name cannot start with a dot (.).".to_string());
    }

    if hostname.ends_with('-') {
        return Err("The system name cannot end with a dash (-).".to_string());
    }

    if hostname.ends_with('.') {
        return Err("The system name cannot end with a dot (.).".to_string());
    }

    Ok(())
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
    // ICU follows general language transliteration rules. These common Hebrew
    // names use the spellings FluffSetup presents to its users.
    match word {
        "יעקוב" | "יעקב" => "yakov".to_string(),
        "כהן" => "cohen".to_string(),
        _ => transliterate_with_icu(word).unwrap_or_else(|| fallback_transliteration(word)),
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

fn apply_gui_settings(hostname: &str, display_name: &str, username: &str, password: &str) {
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
    save_setup_progress(&SetupProgress {
        page: 2,
        hostname: hostname.to_string(),
        display_name: display_name.to_string(),
        username: username.to_string(),
        account_created: true,
    })
    .unwrap_or_else(|error| {
        exit_with_error(&format!("Could not record the created account: {error}"))
    });

    configure_permanent_user(username, password);
}

fn run_setup() {
    print!("\x1B[2J\x1B[1;1H");
    println!("fluffsetup 1.0 - Fluff Linux Setup\n");
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
    request_cleanup();
}

fn run_gui_apply() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|error| exit_with_error(&format!("Failed to read setup details: {error}")));
    let mut values = input.splitn(4, '\n');
    let hostname = values.next().unwrap_or_default().trim_end_matches('\r');
    let entered_display_name = values.next().unwrap_or_default().trim_end_matches('\r');
    let display_name = effective_display_name(entered_display_name);
    let username = values.next().unwrap_or_default().trim_end_matches('\r');
    let password = values
        .next()
        .unwrap_or_default()
        .trim_end_matches(['\r', '\n']);

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
    apply_gui_settings(hostname, display_name, username, password);

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
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    QGuiApplication::set_desktop_file_name(&"fluffsetup".into());
    QQuickStyle::set_style(&"Fusion".into());

    unsafe {
        // SAFETY: both functions are implemented by native/session.cpp and
        // are called on Qt's GUI thread after QGuiApplication is constructed.
        fluffsetup_initialize_session_background();
    }
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/org/flufflinux/setup/qml/Main.qml"));
    }
    unsafe {
        // SAFETY: the QML engine has created the setup window, and the native
        // helper only inspects and binds Qt top level windows on this thread.
        fluffsetup_bind_setup_window();
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

fn main() {
    if env::args().any(|argument| argument == "--cleanup") {
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
        CLEANUP_FILES, SETUP_STATE_FILE, SETUP_STATE_TEMP_FILE, SetupProgress,
        account_matches_setup_identity_in, parse_setup_progress, serialize_setup_progress,
        setup_progress_with_defaults, unique_username_from_base, username_base_from_display_name,
        validate_display_name, validate_hostname, validate_username, without_autologin_section,
    };

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
            "The characters '@!' are not allowed. Use English letters, numbers, dashes (-), and dots (.) only."
        );
        assert_eq!(
            validate_hostname("שלום").unwrap_err(),
            "The characters 'שלום' are not allowed. Use English letters, numbers, dashes (-), and dots (.) only."
        );
        assert_eq!(
            validate_hostname(&"a".repeat(256)).unwrap_err(),
            "The system name is too long. Use no more than 255 characters."
        );
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
        ] {
            assert_eq!(username_base_from_display_name(display_name), expected);
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
            page: 2,
            hostname: "FL-IW2098".to_string(),
            display_name: "Shai Zedaka".to_string(),
            username: "shaiz".to_string(),
            account_created: false,
        };
        let serialized = serialize_setup_progress(&progress);

        assert_eq!(serialized, "2\nFL-IW2098\nShai Zedaka\nshaiz\nfalse");
        assert_eq!(parse_setup_progress(&serialized), progress);
        assert!(!serialized.to_ascii_lowercase().contains("password"));
    }

    #[test]
    fn older_recovery_state_remains_readable() {
        assert_eq!(
            parse_setup_progress("2\nfluff-pc\nUser"),
            SetupProgress {
                page: 2,
                hostname: "fluff-pc".to_string(),
                display_name: "User".to_string(),
                username: String::new(),
                account_created: false,
            }
        );
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
        assert_eq!(parse_setup_progress("3\nfluff-pc\nUser").page, 0);
        assert_eq!(parse_setup_progress("999\nfluff-pc\nUser").page, 0);
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
