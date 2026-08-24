//! Rust behavior exposed to the FluffSetup QML interface.

use cxx_qt::Threading;
use cxx_qt_lib::QString;
use std::io::Write;
use std::pin::Pin;
use std::process::{Command, Stdio};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, busy)]
        #[qproperty(bool, completed)]
        #[qproperty(QString, error_message, cxx_name = "errorMessage")]
        type SetupBackend = super::SetupBackendRust;

        #[qinvokable]
        #[cxx_name = "currentHostname"]
        fn current_hostname(&self) -> QString;

        #[qinvokable]
        #[cxx_name = "defaultName"]
        fn default_name(&self) -> QString;

        #[qinvokable]
        #[cxx_name = "resumePage"]
        fn resume_page(&self) -> i32;

        #[qinvokable]
        #[cxx_name = "savedHostname"]
        fn saved_hostname(&self) -> QString;

        #[qinvokable]
        #[cxx_name = "savedName"]
        fn saved_name(&self) -> QString;

        #[qinvokable]
        #[cxx_name = "accountAlreadyCreated"]
        fn account_already_created(&self) -> bool;

        #[qinvokable]
        #[cxx_name = "saveProgress"]
        fn save_progress(&self, page: i32, hostname: &QString, display_name: &QString);

        #[qinvokable]
        #[cxx_name = "validateHostname"]
        fn validate_hostname(&self, hostname: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "validateName"]
        fn validate_name(&self, display_name: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "startSetup"]
        fn start_setup(
            self: Pin<&mut SetupBackend>,
            hostname: &QString,
            display_name: &QString,
            password: &QString,
        );
    }

    impl cxx_qt::Threading for SetupBackend {}
}

pub struct SetupBackendRust {
    busy: bool,
    completed: bool,
    error_message: QString,
}

impl Default for SetupBackendRust {
    fn default() -> Self {
        Self {
            busy: false,
            completed: false,
            error_message: QString::default(),
        }
    }
}

impl qobject::SetupBackend {
    pub fn current_hostname(&self) -> QString {
        QString::from(&crate::current_or_generated_hostname())
    }

    pub fn default_name(&self) -> QString {
        QString::from(crate::default_display_name())
    }

    pub fn resume_page(&self) -> i32 {
        crate::load_setup_progress().page
    }

    pub fn saved_hostname(&self) -> QString {
        QString::from(&crate::load_setup_progress().hostname)
    }

    pub fn saved_name(&self) -> QString {
        QString::from(&crate::load_setup_progress().display_name)
    }

    pub fn account_already_created(&self) -> bool {
        let progress = crate::load_setup_progress();
        !progress.username.is_empty()
            && crate::account_matches_setup_identity(&progress.username, &progress.display_name)
    }

    pub fn save_progress(&self, page: i32, hostname: &QString, display_name: &QString) {
        // Reuse a previously reserved login name when the display name is
        // unchanged. This makes account creation idempotent across a crash or
        // power loss without ever storing the password in recovery state.
        let previous = crate::load_setup_progress();
        let display_name = display_name.to_string();
        let preserve_account =
            previous.display_name == display_name && !previous.username.is_empty();
        let progress = crate::SetupProgress {
            page,
            hostname: hostname.to_string(),
            display_name,
            username: preserve_account
                .then_some(previous.username)
                .unwrap_or_default(),
            account_created: preserve_account && previous.account_created,
        };
        if let Err(error) = crate::save_setup_progress(&progress) {
            eprintln!("Could not save FluffSetup recovery state: {error}");
        }
    }

    pub fn validate_hostname(&self, hostname: &QString) -> QString {
        let hostname = hostname.to_string();
        if hostname.is_empty() {
            return QString::default();
        }

        QString::from(
            crate::validate_hostname(&hostname)
                .err()
                .unwrap_or_default(),
        )
    }

    pub fn validate_name(&self, display_name: &QString) -> QString {
        let display_name = display_name.to_string();
        if display_name.trim().is_empty() {
            return QString::default();
        }

        let error = crate::validate_display_name(&display_name)
            .err()
            .map(str::to_string)
            .unwrap_or_default();
        QString::from(&error)
    }

    pub fn start_setup(
        mut self: Pin<&mut Self>,
        hostname: &QString,
        display_name: &QString,
        password: &QString,
    ) {
        if *self.busy() || *self.completed() {
            return;
        }

        let entered_hostname = hostname.to_string();
        let hostname = if entered_hostname.is_empty() {
            crate::current_or_generated_hostname()
        } else {
            entered_hostname
        };
        let entered_display_name = display_name.to_string();
        let display_name = crate::effective_display_name(&entered_display_name).to_string();
        let password = password.to_string();

        if let Err(error) = crate::validate_hostname(&hostname) {
            self.as_mut().set_error_message(QString::from(error));
            return;
        }
        if let Err(error) = crate::validate_display_name(&display_name) {
            self.as_mut().set_error_message(QString::from(error));
            return;
        }
        if password.is_empty() {
            self.as_mut()
                .set_error_message(QString::from("The password cannot be blank"));
            return;
        }

        let previous = crate::load_setup_progress();
        let username = if !previous.username.is_empty() && previous.display_name == display_name {
            previous.username
        } else {
            crate::generated_username(&display_name)
        };
        if let Err(error) = crate::validate_username(&username) {
            self.as_mut().set_error_message(QString::from(error));
            return;
        }
        if crate::user_exists(&username)
            && !crate::account_matches_setup_identity(&username, &display_name)
        {
            self.as_mut().set_error_message(QString::from(
                "The reserved account name belongs to a different system account. FluffSetup will not modify it.",
            ));
            return;
        }
        if let Err(error) = crate::save_setup_progress(&crate::SetupProgress {
            page: 2,
            hostname: hostname.clone(),
            display_name: display_name.clone(),
            username: username.clone(),
            account_created: crate::account_matches_setup_identity(&username, &display_name),
        }) {
            self.as_mut().set_error_message(QString::from(&format!(
                "Could not reserve the account for recovery: {error}"
            )));
            return;
        }

        self.as_mut().set_error_message(QString::default());
        self.as_mut().set_completed(false);
        self.as_mut().set_busy(true);
        let qt_thread = self.qt_thread();

        // Privileged commands must not block Qt's event loop. Their result is
        // sent back through the CXX Qt thread safe queue.
        std::thread::spawn(move || {
            let result = apply_in_child(&hostname, &display_name, &username, &password);
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_busy(false);
                match result {
                    Ok(()) => {
                        backend.as_mut().set_completed(true);
                        backend.as_mut().set_error_message(QString::default());
                    }
                    Err(error) => {
                        backend.as_mut().set_completed(false);
                        backend.as_mut().set_error_message(QString::from(&error));
                    }
                }
            });
        });
    }
}

fn apply_in_child(
    hostname: &str,
    display_name: &str,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("Cannot locate FluffSetup: {error}"))?;
    let mut child = Command::new(executable)
        .arg("--apply-setup")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start system setup: {error}"))?;

    // Send the password through the child's standard input rather than a
    // command line argument, where it would be visible in the process list.
    let details = format!("{hostname}\n{display_name}\n{username}\n{password}");
    child
        .stdin
        .take()
        .ok_or_else(|| "Could not send the setup details.".to_string())?
        .write_all(details.as_bytes())
        .map_err(|error| format!("Could not send the setup details: {error}"))?;

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Could not wait for system setup: {error}"))?;
    if output.status.success() {
        return Ok(());
    }

    let error = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let error = error.trim();
    if error.is_empty() {
        Err("Setup could not be completed. Review your details and try again.".to_string())
    } else {
        Err(error.to_string())
    }
}

fn strip_ansi(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' {
            for part in characters.by_ref() {
                if part.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            result.push(character);
        }
    }
    result
}
