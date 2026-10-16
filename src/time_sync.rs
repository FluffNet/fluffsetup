//! Read the existing system time service and network state. Never enable NTP,
//! select a server, change networking, or set the clock from this observer.

use std::time::{Duration, Instant};

const STATUS_LIFETIME: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Receipt {
    destination: u64,
    packets: u64,
}

#[derive(Default)]
pub struct Observation {
    trusted: bool,
    receipt: Option<Receipt>,
    source: String,
    valid_for: Duration,
}

#[derive(Default)]
pub struct Gate {
    expires: Option<Instant>,
    receipt: Option<Receipt>,
    last_source: String,
    waiting_for_receipt: bool,
}

impl Gate {
    pub fn ready(&self, now: Instant) -> bool {
        self.expires.is_some_and(|expires| now < expires)
    }

    pub fn source(&self, now: Instant) -> &str {
        if self.ready(now) {
            &self.last_source
        } else {
            ""
        }
    }

    pub fn update(&mut self, observation: Observation, now: Instant) -> bool {
        if !observation.trusted {
            self.expires = None;
            self.waiting_for_receipt = true;
            if observation.receipt.is_some() {
                self.receipt = observation.receipt;
            }
            return false;
        }
        let changed = self.receipt.is_some() && self.receipt != observation.receipt;
        let source_changed = !self.last_source.is_empty() && self.last_source != observation.source;
        if (self.waiting_for_receipt || source_changed) && !changed {
            self.expires = None;
            self.waiting_for_receipt = true;
            self.receipt = observation.receipt;
            return false;
        }
        self.waiting_for_receipt = false;
        self.receipt = observation.receipt;
        self.last_source = observation.source;
        self.expires = now.checked_add(observation.valid_for.min(STATUS_LIFETIME));
        self.ready(now)
    }

    // Only used by the explicit, non-mutating --preview mode. This must never
    // substitute for a real synchronization observation in normal setup.
    pub fn preview_confirmed() -> Self {
        Self {
            expires: Instant::now().checked_add(Duration::from_secs(60)),
            last_source: "synthetic-preview".into(),
            ..Self::default()
        }
    }
}

#[cfg(any(target_os = "linux", test))]
fn query(
    run: &mut impl FnMut(&str, &[&str]) -> Result<String, String>,
    service: &str,
    path: &str,
    interface: &str,
    properties: &[&str],
) -> Option<String> {
    let mut arguments = vec!["--system", "--timeout=1s"];
    // Metadata-only timedated may activate on demand. Never start a stopped
    // network manager or NTP daemon merely by reading its properties.
    if service != "org.freedesktop.timedate1" {
        arguments.push("--auto-start=no");
    }
    arguments.extend(["get-property", service, path, interface]);
    arguments.extend(properties);
    run("/usr/bin/busctl", &arguments).ok()
}

#[cfg(any(target_os = "linux", test))]
fn packet(text: &str) -> Option<Receipt> {
    let fields = text.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 20
        || fields[0] != "(uuuuittayttttbtt)"
        || fields[8] != "4"
        || fields[17] != "false"
    {
        return None;
    }
    let number = |index: usize| fields[index].parse::<u64>().ok();
    if number(1)? >= 3
        || !matches!(number(2)?, 3 | 4)
        || number(3)? != 4
        || !(1..=15).contains(&number(4)?)
        || fields[5].parse::<i32>().is_err()
        || (number(6)? / 2).checked_add(number(7)?)? > 5_000_000
        || number(19)? > 5_000_000
        || (9..=12).any(|index| number(index).is_none_or(|byte| byte > 255))
        || (13..=16).any(|index| number(index).is_none_or(|time| time == 0))
        || number(18)? == 0
    {
        return None;
    }
    Some(Receipt {
        destination: number(16)?,
        packets: number(18)?,
    })
}

#[cfg(any(target_os = "linux", test))]
fn inspect_with(
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
    age: Option<Duration>,
) -> Observation {
    // Still obtain the last receipt when offline. On reconnection the gate then
    // requires a new successful NTP sample, not an old kernel synchronized flag.
    let network = query(
        &mut run,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
        &[
            "State",
            "Connectivity",
            "ConnectivityCheckEnabled",
            "ConnectivityCheckAvailable",
        ],
    );
    let timedate = query(
        &mut run,
        "org.freedesktop.timedate1",
        "/org/freedesktop/timedate1",
        "org.freedesktop.timedate1",
        &["NTP", "NTPSynchronized"],
    );
    let service = query(
        &mut run,
        "org.freedesktop.timesync1",
        "/org/freedesktop/timesync1",
        "org.freedesktop.timesync1.Manager",
        &["NTPMessage", "PollIntervalUSec", "ServerName"],
    );
    let Some(service) = service else {
        return Observation::default();
    };
    let fields = service.lines().collect::<Vec<_>>();
    if fields.len() != 3 {
        return Observation::default();
    }
    let receipt = packet(fields[0]);
    let mut result = Observation {
        receipt,
        ..Observation::default()
    };
    let Some(poll) = fields[1]
        .strip_prefix("t ")
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return result;
    };
    let Some(source) = fields[2]
        .strip_prefix("s \"")
        .and_then(|value| value.strip_suffix('"'))
    else {
        return result;
    };
    if source.is_empty()
        || source.len() > 255
        || !source
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-_:[]".contains(&byte))
        || poll == 0
        || poll > 86_400_000_000
    {
        return result;
    }
    let maximum_age = Duration::from_micros(poll) + Duration::from_secs(30);
    let Some(remaining) = age.and_then(|age| maximum_age.checked_sub(age)) else {
        return result;
    };
    result.trusted = network.as_deref() == Some("u 70\nu 4\nb true\nb true")
        && timedate.as_deref() == Some("b true\nb true")
        && receipt.is_some()
        && !remaining.is_zero();
    result.source = source.into();
    result.valid_for = remaining.min(STATUS_LIFETIME);
    result
}

pub fn inspect() -> Observation {
    #[cfg(target_os = "linux")]
    {
        let started = Instant::now();
        let age = std::fs::metadata("/run/systemd/timesync/synchronized")
            .ok()
            .and_then(|file| file.modified().ok())
            .and_then(|time| std::time::SystemTime::now().duration_since(time).ok());
        let mut result = inspect_with(crate::locale::checked, age);
        result.valid_for = result.valid_for.saturating_sub(started.elapsed());
        result.trusted &= !result.valid_for.is_zero();
        result
    }
    #[cfg(not(target_os = "linux"))]
    {
        Observation::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PACKET: &str = "(uuuuittayttttbtt) 0 4 4 2 -25 13183 823 4 94 198 159 15 1791805044426183 1791805044466970 1791805044466988 1791805044503176 false 250 3386";
    fn sample(network: &str, flags: &str, message: &str, age: Option<Duration>) -> Observation {
        inspect_with(
            |command, args| {
                assert_eq!(command, "/usr/bin/busctl");
                assert!(args.contains(&"--timeout=1s") && args.contains(&"get-property"));
                assert!(!args.contains(&"call"));
                if !args.contains(&"org.freedesktop.timedate1") {
                    assert!(args.contains(&"--auto-start=no"));
                }
                Ok(if args.contains(&"org.freedesktop.NetworkManager") {
                    network.into()
                } else if args.contains(&"org.freedesktop.timedate1") {
                    flags.into()
                } else {
                    format!("{message}\nt 2048000000\ns \"2.arch.pool.ntp.org\"")
                })
            },
            age,
        )
    }
    fn online() -> Observation {
        sample(
            "u 70\nu 4\nb true\nb true",
            "b true\nb true",
            PACKET,
            Some(Duration::from_secs(60)),
        )
    }
    #[test]
    fn confirmed_online_sync_has_a_bounded_lifetime_and_identifies_the_server() {
        let mut gate = Gate::default();
        let now = Instant::now();
        assert!(gate.update(online(), now));
        assert_eq!(gate.source(now), "2.arch.pool.ntp.org");
        assert!(!gate.ready(now + STATUS_LIFETIME));
    }
    #[test]
    fn offline_unsynchronized_stale_missing_and_malformed_data_fail_closed() {
        for network in [
            "u 20\nu 1\nb true\nb true",
            "u 50\nu 2\nb true\nb true",
            "u 70\nu 3\nb true\nb true",
            "u 70\nu 4\nb false\nb true",
            "u 70\nu 4\nb true\nb false",
            "",
        ] {
            assert!(
                !sample(
                    network,
                    "b true\nb true",
                    PACKET,
                    Some(Duration::from_secs(1))
                )
                .trusted
            );
        }
        for flags in ["b false\nb true", "b true\nb false", "b true", ""] {
            assert!(
                !sample(
                    "u 70\nu 4\nb true\nb true",
                    flags,
                    PACKET,
                    Some(Duration::from_secs(1))
                )
                .trusted
            );
        }
        for age in [
            None,
            Some(Duration::from_secs(2078)),
            Some(Duration::from_secs(10_000)),
        ] {
            assert!(!sample("u 70\nu 4\nb true\nb true", "b true\nb true", PACKET, age).trusted);
        }
        assert!(!inspect_with(|_, _| Err("timeout".into()), Some(Duration::ZERO)).trusted);
        for message in [
            String::new(),
            PACKET.replace("false", "true"),
            PACKET.replace(" 250 ", " 0 "),
            PACKET.replacen(" 0 4 4 2 ", " 3 4 4 2 ", 1),
            PACKET.replacen(" 0 4 4 2 ", " 0 4 4 16 ", 1),
            PACKET.replace("13183 823", "18446744073709551615 18446744073709551615"),
        ] {
            assert!(packet(&message).is_none());
        }
    }
    #[test]
    fn recovery_requires_a_new_receipt_after_offline_or_status_failure() {
        let now = Instant::now();
        let mut gate = Gate::default();
        assert!(gate.update(online(), now));
        assert!(!gate.update(
            sample(
                "u 20\nu 1\nb true\nb true",
                "b true\nb true",
                PACKET,
                Some(Duration::from_secs(60))
            ),
            now
        ));
        assert!(!gate.update(online(), now));
        let fresh = PACKET.replace(" 250 ", " 251 ");
        assert!(gate.update(
            sample(
                "u 70\nu 4\nb true\nb true",
                "b true\nb true",
                &fresh,
                Some(Duration::ZERO)
            ),
            now
        ));
        assert!(!gate.update(Observation::default(), now));
        assert!(!gate.update(
            sample(
                "u 70\nu 4\nb true\nb true",
                "b true\nb true",
                &fresh,
                Some(Duration::ZERO)
            ),
            now
        ));
        assert!(gate.update(online(), now)); // A daemon restart can reset its packet count.
    }
    #[test]
    fn starting_offline_does_not_reuse_a_stale_sync_flag_on_reconnection() {
        let now = Instant::now();
        let mut gate = Gate::default();
        assert!(!gate.update(Observation::default(), now));
        assert!(!gate.update(online(), now));
        let mut fresh = online();
        fresh.receipt.as_mut().unwrap().packets += 1;
        assert!(gate.update(fresh, now));
    }
    #[test]
    fn changing_servers_requires_a_new_receipt_and_refreshes_expiration() {
        let now = Instant::now();
        let mut gate = Gate::default();
        assert!(gate.update(online(), now));
        let mut other = online();
        other.source = "time.example.org".into();
        assert!(!gate.update(other, now));
        assert_eq!(gate.source(now), "");
        let mut fresh = online();
        fresh.source = "time.example.org".into();
        fresh.receipt.as_mut().unwrap().packets += 1;
        fresh.valid_for = Duration::from_secs(2);
        assert!(gate.update(fresh, now));
        assert_eq!(gate.source(now), "time.example.org");
        assert!(!gate.ready(now + Duration::from_secs(2)));
    }
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "reads the Fluff VM's real existing synchronization status"]
    fn installed_time_sync_status_is_read_only() {
        let mut gate = Gate::default();
        gate.update(inspect(), Instant::now());
        eprintln!(
            "Clock ready: {}, source: {}",
            gate.ready(Instant::now()),
            gate.source(Instant::now())
        );
    }
}
