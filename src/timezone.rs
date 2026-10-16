//! Installed IANA timezone IDs and localized city/country search, without IPC
//! or system changes until the explicit finalization helper is called.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
struct Zone {
    id: String,
    countries: Vec<String>,
    coordinate: Option<(f64, f64)>,
}

// tzdata's ISO 6709 coordinates contain signed degrees/minutes, optionally
// seconds. Invalid coordinates never affect timezone validation or the list.
fn coordinate(text: &str) -> Option<(f64, f64)> {
    let split = text.get(1..)?.find(['+', '-'])? + 1;
    fn angle(text: &str, digits: usize, limit: f64) -> Option<f64> {
        let sign = match text.as_bytes().first()? {
            b'+' => 1.0,
            b'-' => -1.0,
            _ => return None,
        };
        let value = text.get(1..)?;
        if !matches!(value.len(), n if n == digits + 2 || n == digits + 4)
            || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let degrees: f64 = value[..digits].parse().ok()?;
        let minutes: f64 = value[digits..digits + 2].parse().ok()?;
        let seconds: f64 = if value.len() == digits + 4 {
            value[digits + 2..].parse().ok()?
        } else {
            0.0
        };
        let result = degrees + minutes / 60.0 + seconds / 3600.0;
        (minutes < 60.0 && seconds < 60.0 && result <= limit).then_some(sign * result)
    }
    Some((
        angle(&text[..split], 2, 90.0)?,
        angle(&text[split..], 3, 180.0)?,
    ))
}

fn parse_table(text: &str) -> Vec<Zone> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let countries = fields
                .next()?
                .split(',')
                .filter(|code| {
                    code.len() == 2 && code.bytes().all(|byte| byte.is_ascii_uppercase())
                })
                .map(str::to_string)
                .collect::<Vec<_>>();
            let coordinate = coordinate(fields.next()?);
            let id = fields.next()?;
            (safe_id(id) && !countries.is_empty()).then(|| Zone {
                id: id.into(),
                countries,
                coordinate,
            })
        })
        .collect()
}

fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('/')
        && !id.starts_with('-')
        && !matches!(id, "localtime" | "posixrules")
        && !id.starts_with("posix/")
        && !id.starts_with("right/")
        && id.split('/').all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_+-".contains(&byte))
        })
}

fn valid_in(id: &str, root: &Path) -> bool {
    if !safe_id(id) {
        return false;
    }
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    let Ok(path) = root.join(id).canonicalize() else {
        return false;
    };
    path.starts_with(&root)
        && fs::File::open(path).is_ok_and(|mut file| {
            use std::io::Read;
            let mut magic = [0; 4];
            file.read_exact(&mut magic).is_ok() && &magic == b"TZif"
        })
}

pub fn valid(id: &str) -> bool {
    valid_in(id, Path::new("/usr/share/zoneinfo"))
}

fn zones() -> &'static [Zone] {
    static ZONES: OnceLock<Vec<Zone>> = OnceLock::new();
    ZONES.get_or_init(|| {
        let root = Path::new("/usr/share/zoneinfo");
        let text = fs::read_to_string(root.join("zone.tab"))
            .or_else(|_| fs::read_to_string(root.join("zone1970.tab")))
            .unwrap_or_default();
        let mut zones = parse_table(&text);
        zones.retain(|zone| valid(&zone.id));
        if valid("UTC") {
            zones.push(Zone {
                id: "UTC".into(),
                countries: Vec::new(),
                coordinate: None,
            });
        }
        zones.sort_by(|a, b| a.id.cmp(&b.id));
        zones.dedup_by(|a, b| a.id == b.id);
        zones
    })
}

pub fn current() -> String {
    let root = Path::new("/usr/share/zoneinfo");
    if let (Ok(root), Ok(path)) = (
        root.canonicalize(),
        Path::new("/etc/localtime").canonicalize(),
    ) && let Ok(id) = path.strip_prefix(root)
        && let Some(id) = id.to_str()
        && valid(id)
    {
        return id.into();
    }
    "UTC".into()
}

pub fn initial(saved: &str, region: &str) -> String {
    if valid(saved) {
        return saved.into();
    }
    regional_default(zones(), region, &current())
}

fn regional_default(zones: &[Zone], region: &str, current: &str) -> String {
    // A single country timezone is unambiguous; never guess among multiple.
    let matches = zones
        .iter()
        .filter(|zone| zone.countries.iter().any(|country| country == region))
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches[0].id.clone()
    } else if matches.iter().any(|zone| zone.id == current) {
        current.into()
    } else if !matches.is_empty() {
        String::new()
    } else {
        current.into()
    }
}

pub fn list(language: &str, region: &str) -> String {
    // Translate once per language. A region change only reorders suggestions.
    type Row = (usize, String, String, String);
    static CACHE: OnceLock<Mutex<HashMap<String, Vec<Row>>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    let rows = cache.entry(language.into()).or_insert_with(|| {
        zones()
            .iter()
            .enumerate()
            .map(|(index, zone)| {
                let english_city = zone
                    .id
                    .rsplit('/')
                    .next()
                    .unwrap_or(&zone.id)
                    .replace('_', " ");
                let city = crate::country::timezone_city(&zone.id, language)
                    .unwrap_or_else(|| english_city.clone());
                let countries = zone
                    .countries
                    .iter()
                    .map(|code| crate::country::display_name(code, language))
                    .collect::<Vec<_>>()
                    .join(", ");
                let english = zone
                    .countries
                    .iter()
                    .map(|code| crate::country::display_name(code, "en"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let label = if countries.is_empty() {
                    city.clone()
                } else {
                    format!("{city} ({countries})")
                };
                let search = format!(
                    "{label} {city} {countries} {english_city} {english} {} {} {}",
                    zone.id.replace('_', " "),
                    zone.id,
                    zone.countries.join(" ")
                );
                (index, label, english_city, search)
            })
            .collect::<Vec<_>>()
    });
    let suggested = |index: usize| zones()[index].countries.iter().any(|code| code == region);
    let mut ordered = rows.iter().collect::<Vec<_>>();
    ordered.sort_by_cached_key(|(index, label, _, _)| {
        (!suggested(*index), crate::i18n::country_sort_key(label))
    });
    let rows = ordered.into_iter().map(|(index, label, english, search)| {
        let zone = &zones()[*index];
        let center = zone.coordinate.map(|(lat, lon)| format!("[{lat},{lon}]")).unwrap_or_else(|| "null".into());
        format!("{{\"code\":{},\"label\":{},\"english\":{},\"search\":{},\"suggested\":{},\"center\":{center}}}",
            crate::keyboard::json_string(&zone.id), crate::keyboard::json_string(label),
            crate::keyboard::json_string(english), crate::keyboard::json_string(search), suggested(*index))
    }).collect::<Vec<_>>();
    format!("[{}]", rows.join(","))
}

pub fn apply(id: &str) -> Result<(), String> {
    if !valid(id) {
        return Err("Invalid or unavailable timezone.".into());
    }
    apply_checked(id, crate::locale::checked)
}

fn apply_checked(
    id: &str,
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
) -> Result<(), String> {
    if run("/usr/bin/id", &["-u"])? != "0" {
        return Err("Timezone finalization must run as root.".into());
    }
    // This is a system setting inherited by the created user. Do not add a TZ
    // environment override, change the clock/NTP, or force an RTC policy.
    run("/usr/bin/timedatectl", &["set-timezone", id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsafe_ids_and_non_timezone_files_are_rejected() {
        for id in [
            "",
            "../etc/passwd",
            "/etc/localtime",
            "Asia//Jerusalem",
            "Asia/../UTC",
            "-UTC",
            "Asia/Jerusalem;bad",
            "UTC\n",
        ] {
            assert!(!valid(id), "{id}");
        }
        assert!(!valid("zone.tab"));
        assert!(valid("UTC"));
        assert!(valid("Asia/Jerusalem"));
    }
    #[test]
    fn tables_keep_canonical_ids_and_multi_country_metadata() {
        let rows = parse_table(
            "#comment\nIL\t+3146+03514\tAsia/Jerusalem\nBE,NL\t+5050+00420\tEurope/Brussels\nXX\t+0000\t../etc/passwd\n",
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].countries, ["BE", "NL"]);
        assert_eq!(rows[0].coordinate, coordinate("+3146+03514"));
    }
    #[test]
    fn coordinates_accept_both_precisions_and_reject_bad_bounds() {
        assert_eq!(
            coordinate("+404251-0740023"),
            Some((40.71416666666667, -74.00638888888889))
        );
        assert_eq!(
            coordinate("-3352+15113"),
            Some((-33.86666666666667, 151.21666666666667))
        );
        for text in [
            "",
            "+0000",
            "+9160+18100",
            "+3159+03560",
            "+3146+0351499",
            "+ab46+03514",
            "0+3146+03514",
        ] {
            assert!(coordinate(text).is_none(), "{text}");
        }
    }
    #[test]
    fn region_defaults_and_manual_selection_do_not_guess_a_multi_zone_country() {
        let zones = zones();
        for (country, id) in [
            ("IL", "Asia/Jerusalem"),
            ("BE", "Europe/Brussels"),
            ("JP", "Asia/Tokyo"),
            ("DK", "Europe/Copenhagen"),
        ] {
            assert_eq!(regional_default(zones, country, "UTC"), id);
        }
        for country in ["US", "RU"] {
            assert_eq!(regional_default(zones, country, "Asia/Jerusalem"), "");
            let zone = zones
                .iter()
                .find(|zone| zone.countries.iter().any(|code| code == country))
                .unwrap();
            assert_eq!(regional_default(zones, country, &zone.id), zone.id);
            assert_eq!(initial("Europe/Brussels", country), "Europe/Brussels");
        }
        assert_eq!(regional_default(zones, "XX", "UTC"), "UTC");
    }
    #[test]
    fn finalization_only_sets_timezone_and_propagates_failures() {
        let mut calls = Vec::new();
        apply_checked("Asia/Jerusalem", |program, args| {
            calls.push((
                program.to_string(),
                args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>(),
            ));
            Ok(if program.ends_with("/id") { "0" } else { "" }.into())
        })
        .unwrap();
        assert_eq!(
            calls,
            [
                ("/usr/bin/id".into(), vec!["-u".into()]),
                (
                    "/usr/bin/timedatectl".into(),
                    vec!["set-timezone".into(), "Asia/Jerusalem".into()]
                )
            ]
        );
        assert!(apply_checked("UTC", |_, _| Ok("1000".into())).is_err());
        assert!(
            apply_checked("UTC", |program, _| if program.ends_with("/id") {
                Ok("0".into())
            } else {
                Err("unavailable".into())
            })
            .is_err()
        );
        assert!(apply("../etc/passwd").is_err());
    }
    #[test]
    fn catalog_ids_have_real_timezone_files_and_map_coordinates() {
        for zone in zones() {
            assert!(valid(&zone.id));
            if zone.id != "UTC" {
                assert!(zone.coordinate.is_some(), "{}", zone.id);
            }
        }
    }
    #[cfg(unix)]
    #[test]
    fn validation_rejects_symlinks_outside_zoneinfo() {
        let task_dir =
            std::env::temp_dir().join(format!("fluffsetup-zone-test-{}", std::process::id()));
        fs::create_dir_all(task_dir.join("zones")).unwrap();
        fs::write(task_dir.join("outside"), b"TZif").unwrap();
        fs::write(task_dir.join("zones/UTC"), b"TZif").unwrap();
        std::os::unix::fs::symlink(task_dir.join("outside"), task_dir.join("zones/escape"))
            .unwrap();
        assert!(valid_in("UTC", &task_dir.join("zones")));
        assert!(!valid_in("escape", &task_dir.join("zones")));
        fs::remove_dir_all(task_dir).unwrap();
    }
    #[test]
    fn search_contains_english_and_native_city_and_country_names() {
        let rows = list("he", "IL");
        assert!(rows.contains("Asia/Jerusalem"));
        assert!(rows.contains("Jerusalem"));
        assert!(rows.contains("Israel"));
        assert!(rows.contains("ירושלים"));
        assert!(rows.contains("ישראל"));
        assert!(rows.contains("לונדון"));
        assert!(rows.contains("London"));
        assert_eq!(initial("Europe/Brussels", "IL"), "Europe/Brussels");
    }
    #[test]
    fn localized_names_work_beyond_hebrew_without_losing_english_search() {
        for (language, id, native, english) in [
            ("fr", "Europe/London", "Londres", "London"),
            ("de", "Europe/Moscow", "Moskau", "Moscow"),
            ("ru", "Europe/London", "Лондон", "London"),
            ("ar", "Europe/London", "لندن", "London"),
            ("ja", "Asia/Tokyo", "東京", "Tokyo"),
            ("ko", "Asia/Tokyo", "도쿄", "Tokyo"),
            ("zh_Hant", "Asia/Tokyo", "東京", "Tokyo"),
        ] {
            assert_eq!(
                crate::country::timezone_city(id, language).as_deref(),
                Some(native),
                "{language}"
            );
            let rows = list(language, "US");
            assert!(
                rows.contains(native) && rows.contains(english),
                "{language}"
            );
        }
    }
}
