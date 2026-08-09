use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

const BOLD_RED: &str = "\x1b[1;31m";
const BOLD_GREEN: &str = "\x1b[1;32m";
const YELLOW: &str = "\x1b[33m";
const PURPLE: &str = "\x1b[35m";
const RESET: &str = "\x1b[0m";

const HOSTNAME_REQUIREMENTS: &str = "Hostname Requirements:
Allowed characters: letters (a-z and A-Z), digits (0-9), dash (-), and dot (.).
Cannot start or end with a dash (-) or a dot (.).
Special characters are not allowed (for example: @ and !)
No spaces allowed.
Max length: 255 characters.";

const USERNAME_REQUIREMENTS: &str = "User Name Requirements:
Only lowercase letters (a-z), digits (0-9), underscore (_), or dash (-).
No spaces allowed.
Cannot start or end with a dash (-) or an underscore (_)
Special characters are not allowed (for example: @ and !)
Must start with a lowercase letter; cannot be only numbers
Max length: 32 characters";

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

fn validate_hostname(hostname: &str) -> Result<(), &'static str> {
    if hostname.is_empty() {
        return Err("The hostname cannot be blank");
    }

    if hostname.len() > 255 {
        return Err("The hostname must be 255 characters or less");
    }

    if hostname.chars().any(char::is_whitespace) {
        return Err("The hostname cannot have spaces");
    }

    if hostname
        .chars()
        .any(|character| !character.is_ascii_alphanumeric() && character != '-' && character != '.')
    {
        return Err(
            "The hostname cannot have any special characters (Check requirements and try again)",
        );
    }

    if hostname.starts_with('-')
        || hostname.starts_with('.')
        || hostname.ends_with('-')
        || hostname.ends_with('.')
    {
        return Err("The hostname cannot start or end with '.' or '-'");
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
    Command::new("/usr/bin/id")
        .args(["-u", username])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn username_setup() -> String {
    loop {
        print!("\nPlease enter the user name you'd like to have: ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap_or_else(|error| {
            exit_with_error(&format!("Failed to read the user name: {error}"))
        });
        let username = input.trim();

        let selected = if username.is_empty() {
            println!("\nNo user name was entered. Offering default user name.\n");
            print!("Set \"user\" as the user name? [Y/n]: ");
            io::stdout().flush().ok();
            if confirm_yes_default() {
                "user".to_string()
            } else {
                continue;
            }
        } else {
            if let Err(error) = validate_username(username) {
                println!("\n{PURPLE}{USERNAME_REQUIREMENTS}{RESET}");
                println!("\n{BOLD_RED}Error:{RESET} {error}\n");
                continue;
            }

            print!("\nSet \"{username}\" as the user name? [Y/n]: ");
            io::stdout().flush().ok();
            if !confirm_yes_default() {
                continue;
            }
            username.to_string()
        };

        if user_exists(&selected) {
            println!("\n{BOLD_RED}Error:{RESET} The user name \"{selected}\" already exists.\n");
            continue;
        }

        return selected;
    }
}

fn password_setup(username: &str) {
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

        let chpasswd_input = format!("{username}:{password}\n");
        run_sudo_with_input("/usr/bin/chpasswd", &[], chpasswd_input.as_bytes());
        return;
    }
}

fn run_setup() {
    print!("\x1B[2J\x1B[1;1H");
    println!("fluffsetup 1.0 - Fluff Linux Setup\n");
    println!("Welcome to FluffSetup.");
    println!("This temporary setup will create your account and name your system.");

    let hostname = hostname_setup();
    let username = username_setup();

    println!("\nApplying your settings...");
    run_sudo("/usr/bin/hostnamectl", &["set-hostname", &hostname]);
    run_sudo(
        "/usr/bin/useradd",
        &[
            "-m",
            "-G",
            "uucp,wheel,kvm,libvirt",
            "-s",
            "/bin/zsh",
            &username,
        ],
    );
    password_setup(&username);

    println!(
        "\n{BOLD_GREEN}Account setup completed successfully.{RESET}\n\
         User name: {username}\n\
         System name: {hostname}"
    );
    pause();
}

fn launch_konsole() {
    let executable = env::current_exe()
        .unwrap_or_else(|error| exit_with_error(&format!("Cannot locate FluffSetup: {error}")));
    let status = Command::new("/usr/bin/konsole")
        .args(["--separate", "-e"])
        .arg(executable)
        .arg("--terminal")
        .status()
        .unwrap_or_else(|error| {
            exit_with_error(&format!("Failed to open FluffSetup in Konsole: {error}"));
        });

    if !status.success() {
        exit_with_error("Konsole closed unexpectedly.");
    }
}

fn main() {
    if env::args().any(|argument| argument == "--terminal") {
        run_setup();
    } else {
        launch_konsole();
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_hostname, validate_username};

    #[test]
    fn validates_hostnames() {
        assert!(validate_hostname("fluff-laptop").is_ok());
        assert!(validate_hostname("FL-PC.example").is_ok());
        assert!(validate_hostname("-bad").is_err());
        assert!(validate_hostname("bad name").is_err());
    }

    #[test]
    fn validates_user_names() {
        assert!(validate_username("shai").is_ok());
        assert!(validate_username("fluff_user-1").is_ok());
        assert!(validate_username("Fluff").is_err());
        assert!(validate_username("fluffsetup").is_err());
        assert!(validate_username("1234").is_err());
    }
}
