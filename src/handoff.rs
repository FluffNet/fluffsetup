//! One-shot, system-managed handoff after successful first-boot finalization.
//!
//! Keep a small root worker outside the setup session, since cleanup unlinks
//! this executable and removes the temporary account before Finish. Only retire
//! setup sessions; FLLM loads per-user settings without restarting its greeter.

use crate::locale::{atomic_write, checked};
use std::fs;
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const UNIT: &str = "fluffsetup-plm-handoff.service";
const READY: &str = "/run/fluffsetup-plm-handoff/ready";
const STARTED: &str = "/run/fluffsetup-plm-handoff/started";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Session {
    id: String,
    uid: u32,
}

fn property<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .find_map(|(name, value)| (name == key).then_some(value))
}

fn setup_session(text: &str) -> Result<Session, String> {
    let id = property(text, "Id").ok_or("Missing setup session ID.")?;
    let uid = property(text, "User")
        .and_then(|uid| uid.parse::<u32>().ok())
        .filter(|uid| *uid >= 1000)
        .ok_or("Invalid setup session owner.")?;
    if id.is_empty()
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric())
        || property(text, "Name") != Some(crate::TEMPORARY_USER)
        || property(text, "Scope") != Some(format!("session-{id}.scope").as_str())
        || !matches!(property(text, "Class"), Some("user" | "user-early"))
        || !matches!(property(text, "Type"), Some("wayland" | "x11" | "tty"))
    {
        return Err("Refusing a PLM handoff outside the temporary setup session.".into());
    }
    Ok(Session { id: id.into(), uid })
}

// /proc start times distinguish an exited process from a reused PID. A comm
// field may itself contain spaces or parentheses, so split at its final ')'.
fn start_time(text: &str) -> Option<u64> {
    text.rsplit_once(") ")?
        .1
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()
}

fn process_start(pid: u32) -> Result<Option<u64>, String> {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(text) => start_time(&text)
            .map(Some)
            .ok_or_else(|| "Invalid handoff process identity.".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn alive(pid: u32, start: u64) -> Result<bool, String> {
    Ok(process_start(pid)? == Some(start))
}

fn worker_arguments(controller: u32, start: u64, session: &Session) -> Result<Vec<String>, String> {
    let parent = std::process::id();
    let parent_start = process_start(parent)?.ok_or("Missing handoff parent process.")?;
    Ok(vec![
        "--quiet".into(),
        "--collect".into(),
        "--service-type=exec".into(),
        format!("--unit={UNIT}"),
        "--property=RuntimeDirectory=fluffsetup-plm-handoff".into(),
        "--property=RuntimeDirectoryMode=0700".into(),
        "--property=Restart=no".into(),
        "--".into(),
        // systemd-run waits for exec to succeed before this parent can exit.
        // The worker keeps the original executable mapped after cleanup.
        format!("/proc/{parent}/exe"),
        "--plm-handoff".into(),
        controller.to_string(),
        start.to_string(),
        parent.to_string(),
        parent_start.to_string(),
        session.id.clone(),
        session.uid.to_string(),
    ])
}

fn wait_for_worker() -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match fs::read_to_string(STARTED) {
            Ok(text) if text == "started\n" => return Ok(()),
            Ok(_) => return Err("Invalid PLM worker readiness record.".into()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        if Instant::now() >= deadline {
            return Err("The PLM handoff worker did not become ready.".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn after_cleanup(
    cleanup: impl FnOnce() -> Result<(), String>,
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
    commit: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let result = cleanup().and_then(|()| commit());
    if result.is_err() {
        // An uncommitted or failed setup must never terminate setup sessions.
        // There is intentionally no ExecStop/ExecStopPost hook on this unit.
        if let Err(error) = run("/usr/bin/systemctl", &["stop", UNIT]) {
            eprintln!("Could not cancel the unfinished PLM handoff: {error}");
        }
    }
    result
}

pub fn finalize(
    controller: u32,
    cleanup: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let manager = checked(
        "/usr/bin/systemctl",
        &["show", "plasmalogin.service", "-p", "ActiveState"],
    )?;
    if property(&manager, "ActiveState") != Some("active") {
        // Preserve the terminal fallback on machines not running PLM.
        return cleanup();
    }
    let session = setup_session(&checked(
        "/usr/bin/loginctl",
        &[
            "show-session",
            "self",
            "-p",
            "Id",
            "-p",
            "User",
            "-p",
            "Name",
            "-p",
            "Scope",
            "-p",
            "Type",
            "-p",
            "Class",
        ],
    )?)?;
    let scope = format!("/session-{}.scope", session.id);
    let groups = fs::read_to_string(format!("/proc/{controller}/cgroup"))
        .map_err(|error| error.to_string())?;
    if !groups.lines().any(|line| line.ends_with(&scope)) {
        return Err("The Finish process does not belong to this setup session.".into());
    }
    let status = fs::read_to_string(format!("/proc/{controller}/status"))
        .map_err(|error| error.to_string())?;
    let uid = status.lines().find_map(|line| {
        line.strip_prefix("Uid:")?
            .split_whitespace()
            .next()?
            .parse::<u32>()
            .ok()
    });
    if uid != Some(session.uid) && !(controller == std::process::id() && uid == Some(0)) {
        return Err("Invalid Finish process owner.".into());
    }
    let start = process_start(controller)?.ok_or("The Finish process has already exited.")?;
    let arguments = worker_arguments(controller, start, &session)?;
    checked(
        "/usr/bin/systemd-run",
        &arguments.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    after_cleanup(
        || {
            wait_for_worker()?;
            cleanup()
        },
        checked,
        || atomic_write(Path::new(READY), "ready\n", 0o600),
    )
}

#[derive(Debug, PartialEq, Eq)]
enum Decision {
    Wait,
    Abort,
    Finish,
}

fn decision(committed: bool, controller_alive: bool, parent_alive: bool) -> Decision {
    if !committed && !parent_alive {
        Decision::Abort
    } else if committed && !controller_alive {
        Decision::Finish
    } else {
        Decision::Wait
    }
}

fn session_rows(text: &str) -> impl Iterator<Item = (&str, u32)> {
    text.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        Some((fields.next()?, fields.next()?.parse().ok()?))
    })
}

fn other_graphical_session(text: &str, setup_uid: u32) -> bool {
    property(text, "User").and_then(|uid| uid.parse::<u32>().ok()) != Some(setup_uid)
        && matches!(property(text, "Class"), Some("user" | "user-early"))
        && matches!(property(text, "Type"), Some("wayland" | "x11"))
}

// Finish and logind cleanup run concurrently. Ignore a failed query/termination
// only after a fresh listing confirms that its target has actually disappeared.
// Permission, D-Bus and other errors for a still-present target remain errors.
fn session_command(
    run: &mut impl FnMut(&str, &[&str]) -> Result<String, String>,
    command: &str,
    args: &[&str],
    still_present: impl FnOnce(&str) -> bool,
) -> Result<Option<String>, String> {
    match run(command, args) {
        Ok(output) => Ok(Some(output)),
        Err(error) => {
            let current = run("/usr/bin/loginctl", &["list-sessions", "--no-legend"])?;
            if still_present(&current) {
                Err(error)
            } else {
                Ok(None)
            }
        }
    }
}

fn terminate_user_sessions(
    uid: u32,
    run: &mut impl FnMut(&str, &[&str]) -> Result<String, String>,
) -> Result<(), String> {
    // loginctl resolves even numeric UIDs through NSS, which fails after
    // userdel. logind accepts the captured UID through its native system API.
    session_command(
        run,
        "/usr/bin/busctl",
        &[
            "call",
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "TerminateUser",
            "u",
            &uid.to_string(),
        ],
        |current| session_rows(current).any(|(_, owner)| owner == uid),
    )?;
    // TerminateUser is asynchronous. Wait for the temporary user's sessions
    // to close without interfering with the display manager's own sessions.
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let current = run("/usr/bin/loginctl", &["list-sessions", "--no-legend"])?;
        if !session_rows(&current).any(|(_, owner)| owner == uid) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Timed out waiting for user {uid}'s old login sessions to close."
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn finish_with(
    session: &Session,
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
    uid_unused: impl FnOnce() -> Result<bool, String>,
) -> Result<(), String> {
    let sessions = run("/usr/bin/loginctl", &["list-sessions", "--no-legend"])?;
    for (id, uid) in session_rows(&sessions) {
        let Some(details) = session_command(
            &mut run,
            "/usr/bin/loginctl",
            &[
                "show-session",
                id,
                "-p",
                "User",
                "-p",
                "Type",
                "-p",
                "Class",
            ],
            |current| session_rows(current).any(|row| row == (id, uid)),
        )?
        else {
            continue;
        };
        if other_graphical_session(&details, session.uid) {
            return Err(
                "Setup handoff deferred: another user's graphical session is active.".into(),
            );
        }
    }
    if session_rows(&sessions).any(|(id, uid)| id == session.id && uid == session.uid) {
        session_command(
            &mut run,
            "/usr/bin/loginctl",
            &["terminate-session", &session.id],
            |current| session_rows(current).any(|(id, uid)| id == session.id && uid == session.uid),
        )?;
    }
    // Account cleanup has already removed fluffsetup. Clear its remaining SSH,
    // TTY and user-manager sessions too, but never a newly reused UID.
    if uid_unused()? {
        terminate_user_sessions(session.uid, &mut run)?;
    }
    Ok(())
}

fn finish(session: &Session) -> Result<(), String> {
    finish_with(session, checked, || {
        let passwd = fs::read_to_string("/etc/passwd").map_err(|error| error.to_string())?;
        Ok(!passwd.lines().any(|line| {
            line.split(':')
                .nth(2)
                .and_then(|uid| uid.parse::<u32>().ok())
                == Some(session.uid)
        }))
    })
}

pub fn run_worker(arguments: &[String]) -> Result<(), String> {
    let [controller, start, parent, parent_start, id, uid] = arguments else {
        return Err("Invalid PLM handoff arguments.".into());
    };
    let number = |value: &str| {
        value
            .parse::<u64>()
            .map_err(|_| "Invalid handoff identity.".to_string())
    };
    let controller = u32::try_from(number(controller)?).map_err(|error| error.to_string())?;
    let start = number(start)?;
    let parent = u32::try_from(number(parent)?).map_err(|error| error.to_string())?;
    let parent_start = number(parent_start)?;
    let uid = u32::try_from(number(uid)?).map_err(|error| error.to_string())?;
    if controller == 0
        || parent == 0
        || uid < 1000
        || id.is_empty()
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err("Invalid PLM handoff identity.".into());
    }
    atomic_write(Path::new(STARTED), "started\n", 0o600)?;
    eprintln!("PLM handoff ready; waiting for successful cleanup and Finish.");
    loop {
        let committed = match fs::read_to_string(READY) {
            Ok(text) if text == "ready\n" => true,
            Ok(_) => return Err("Invalid PLM handoff completion record.".into()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.to_string()),
        };
        match decision(
            committed,
            alive(controller, start)?,
            alive(parent, parent_start)?,
        ) {
            Decision::Abort => {
                eprintln!("PLM handoff cancelled before setup was committed.");
                return Ok(());
            }
            Decision::Finish => {
                eprintln!("Finish received; closing temporary setup sessions.");
                return finish(&Session {
                    id: id.clone(),
                    uid,
                });
            }
            Decision::Wait => thread::sleep(Duration::from_millis(200)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_requires_successful_cleanup_and_finish_even_after_parent_exits() {
        assert_eq!(decision(false, true, true), Decision::Wait);
        assert_eq!(decision(false, false, true), Decision::Wait);
        assert_eq!(decision(false, true, false), Decision::Abort);
        assert_eq!(decision(false, false, false), Decision::Abort);
        assert_eq!(decision(true, true, true), Decision::Wait);
        assert_eq!(decision(true, true, false), Decision::Wait);
        assert_eq!(decision(true, false, true), Decision::Finish);
        assert_eq!(decision(true, false, false), Decision::Finish);
    }

    #[test]
    fn failed_cleanup_or_commit_cancels_without_refreshing_plm() {
        let mut calls = Vec::new();
        let result = after_cleanup(
            || Err("cleanup failed".into()),
            |command, args| {
                calls.push((
                    command.to_string(),
                    args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                ));
                Ok(String::new())
            },
            || panic!("must not commit failed cleanup"),
        );
        assert_eq!(result.unwrap_err(), "cleanup failed");
        assert_eq!(
            calls,
            [(
                "/usr/bin/systemctl".into(),
                vec!["stop".into(), UNIT.into()]
            )]
        );
        let result = after_cleanup(
            || Ok(()),
            |_, args| {
                assert_eq!(args, ["stop", UNIT]);
                Ok(String::new())
            },
            || Err("commit failed".into()),
        );
        assert_eq!(result.unwrap_err(), "commit failed");
        after_cleanup(
            || Ok(()),
            |_, _| panic!("no command before Finish"),
            || Ok(()),
        )
        .unwrap();
    }

    #[test]
    fn identities_reject_unrelated_sessions_and_pid_reuse() {
        let valid =
            "Id=9\nUser=1000\nName=fluffsetup\nScope=session-9.scope\nType=wayland\nClass=user\n";
        assert_eq!(
            setup_session(valid).unwrap(),
            Session {
                id: "9".into(),
                uid: 1000
            }
        );
        for (old, new) in [
            ("Name=fluffsetup", "Name=root"),
            ("User=1000", "User=0"),
            ("Scope=session-9.scope", "Scope=session-11.scope"),
            ("Class=user", "Class=greeter"),
            ("Type=wayland", "Type=unspecified"),
            ("Id=9", "Id=9;bad"),
        ] {
            assert!(setup_session(&valid.replace(old, new)).is_err());
        }
        let fields = (4..=21)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            start_time(&format!("123 (a strange ) name) S {fields} 98765 0")),
            Some(98765)
        );
        assert_eq!(start_time("123 broken"), None);
    }

    #[test]
    fn unrelated_desktops_are_protected_but_greeters_and_ssh_do_not_block() {
        assert!(other_graphical_session(
            "User=1001\nClass=user\nType=wayland",
            1000
        ));
        for details in [
            "User=1000\nClass=user\nType=wayland",
            "User=1001\nClass=user\nType=tty",
            "User=956\nClass=greeter\nType=wayland",
            "User=1000\nClass=manager\nType=unspecified",
        ] {
            assert!(!other_graphical_session(details, 1000));
        }
    }

    #[test]
    fn finish_only_retires_setup_sessions_and_preserves_the_greeter() {
        for reused_uid in [false, true] {
            let mut setup_present = true;
            let mut calls = Vec::new();
            finish_with(
                &Session {
                    id: "12".into(),
                    uid: 1000,
                },
                |command, args| {
                    calls.push(format!("{command} {}", args.join(" ")));
                    match (command, args[0]) {
                        ("/usr/bin/loginctl", "list-sessions") => Ok(if setup_present {
                            "12 1000 fluffsetup\nc3 956 greeter\n"
                        } else {
                            "c3 956 greeter\n"
                        }
                        .into()),
                        ("/usr/bin/loginctl", "show-session") => Ok(if args[1] == "12" {
                            "User=1000\nType=wayland\nClass=user\n"
                        } else {
                            "User=956\nType=wayland\nClass=greeter\n"
                        }
                        .into()),
                        ("/usr/bin/loginctl", "terminate-session") => {
                            assert_eq!(args[1], "12");
                            setup_present = false;
                            Ok(String::new())
                        }
                        ("/usr/bin/busctl", "call") => {
                            assert!(!reused_uid);
                            assert_eq!(args.last(), Some(&"1000"));
                            Ok(String::new())
                        }
                        _ => panic!("Unexpected handoff command: {command} {args:?}"),
                    }
                },
                || Ok(!reused_uid),
            )
            .unwrap();
            assert!(!setup_present);
            assert_eq!(
                calls
                    .iter()
                    .filter(|call| call.contains("TerminateUser"))
                    .count(),
                usize::from(!reused_uid)
            );
            assert!(!calls.iter().any(|call| call.contains("systemctl")));
        }
    }

    #[test]
    fn disappearing_setup_session_does_not_prevent_handoff() {
        finish_with(
            &Session {
                id: "12".into(),
                uid: 1000,
            },
            |command, args| match (command, args[0]) {
                (_, "list-sessions") => Ok("c3 956 greeter\n".into()),
                (_, "show-session") => Ok("User=956\nType=wayland\nClass=greeter\n".into()),
                ("/usr/bin/busctl", "call") => {
                    assert_eq!(args.last(), Some(&"1000"));
                    Err("No session known".into())
                }
                _ => panic!("Unexpected handoff command: {command} {args:?}"),
            },
            || Ok(true),
        )
        .unwrap();
    }

    #[test]
    fn live_session_errors_are_not_silenced() {
        let error = session_command(
            &mut |_, args| {
                if args[0] == "list-sessions" {
                    Ok("12 1000 fluffsetup\n".into())
                } else {
                    Err("permission denied".into())
                }
            },
            "/usr/bin/loginctl",
            &["terminate-session", "12"],
            |current| session_rows(current).any(|row| row == ("12", 1000)),
        )
        .unwrap_err();
        assert_eq!(error, "permission denied");
    }
}
