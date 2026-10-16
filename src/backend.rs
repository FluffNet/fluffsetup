//! Rust behavior exposed to the FluffSetup QML interface.

use cxx_qt::{CxxQtType, Threading};
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
        #[qproperty(bool, right_to_left, cxx_name = "rightToLeft")]
        #[qproperty(QString, selected_language, cxx_name = "selectedLanguage")]
        #[qproperty(QString, selected_region, cxx_name = "selectedRegion")]
        #[qproperty(QString, selected_timezone, cxx_name = "selectedTimezone")]
        #[qproperty(bool, clock_synchronized, cxx_name = "clockSynchronized")]
        #[qproperty(QString, keyboard_selection, cxx_name = "keyboardSelection")]
        #[qproperty(QString, preview_keyboard_search, cxx_name = "previewKeyboardSearch")]
        #[qproperty(bool, keyboard_session_ready, cxx_name = "keyboardSessionReady")]
        #[qproperty(bool, keyboard_session_failed, cxx_name = "keyboardSessionFailed")]
        #[qproperty(i32, active_keyboard_index, cxx_name = "activeKeyboardIndex")]
        #[qproperty(bool, keyboard_switching, cxx_name = "keyboardSwitching")]
        #[qproperty(bool, preview)]
        #[qproperty(i32, preview_width, cxx_name = "previewWidth")]
        #[qproperty(i32, preview_height, cxx_name = "previewHeight")]
        #[qproperty(QString, snapshot_path, cxx_name = "snapshotPath")]
        #[qproperty(QString, logo_source, cxx_name = "logoSource")]
        #[qproperty(QString, preview_icons, cxx_name = "previewIcons")]
        type SetupBackend = super::SetupBackendRust;

        #[qinvokable]
        fn translate(&self, key: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "languageList"]
        fn language_list(&self) -> QString;
        #[qinvokable]
        #[cxx_name = "baseLanguage"]
        fn base_language(&self, code: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "regionList"]
        fn region_list(&self, code: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "selectLanguage"]
        fn select_language(self: Pin<&mut SetupBackend>, code: &QString);
        #[qinvokable]
        #[cxx_name = "countryList"]
        fn country_list(&self, language: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "selectRegion"]
        fn select_region(self: Pin<&mut SetupBackend>, code: &QString);
        #[qinvokable]
        #[cxx_name = "timezoneList"]
        fn timezone_list(&self, language: &QString, region: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "selectTimezone"]
        fn select_timezone(self: Pin<&mut SetupBackend>, id: &QString);
        #[qinvokable]
        #[cxx_name = "timezoneMapText"]
        fn timezone_map_text(&self, context: &QString, source: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "timezoneClock"]
        fn timezone_clock(&self) -> QString;
        #[qinvokable]
        #[cxx_name = "refreshClockSync"]
        fn refresh_clock_sync(self: Pin<&mut SetupBackend>);
        #[qinvokable]
        #[cxx_name = "keyboardList"]
        fn keyboard_list(
            &self,
            language: &QString,
            selection: &QString,
            region: &QString,
        ) -> QString;
        #[qinvokable]
        #[cxx_name = "addKeyboard"]
        fn add_keyboard(self: Pin<&mut SetupBackend>, id: &QString);
        #[qinvokable]
        #[cxx_name = "removeKeyboard"]
        fn remove_keyboard(self: Pin<&mut SetupBackend>, id: &QString);
        #[qinvokable]
        #[cxx_name = "defaultKeyboard"]
        fn default_keyboard(self: Pin<&mut SetupBackend>, id: &QString);
        #[qinvokable]
        #[cxx_name = "moveKeyboard"]
        fn move_keyboard(self: Pin<&mut SetupBackend>, id: &QString, direction: i32);
        #[qinvokable]
        #[cxx_name = "reorderKeyboard"]
        fn reorder_keyboard(self: Pin<&mut SetupBackend>, id: &QString, destination: i32);
        #[qinvokable]
        #[cxx_name = "syncKeyboardSession"]
        fn sync_keyboard_session(self: Pin<&mut SetupBackend>);
        #[qinvokable]
        #[cxx_name = "refreshKeyboardLayout"]
        fn refresh_keyboard_layout(self: Pin<&mut SetupBackend>);
        #[qinvokable]
        #[cxx_name = "switchKeyboardLayout"]
        fn switch_keyboard_layout(self: Pin<&mut SetupBackend>, id: &QString);

        #[qinvokable]
        #[cxx_name = "exercisePreview"]
        fn exercise_preview(&self) -> bool;
        #[qinvokable]
        #[cxx_name = "previewKeyboardMenu"]
        fn preview_keyboard_menu(&self) -> bool;

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
        #[cxx_name = "previewTimezoneSearch"]
        fn preview_timezone_search(&self) -> QString;

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
    right_to_left: bool,
    selected_language: QString,
    selected_region: QString,
    selected_timezone: QString,
    timezone_customized: bool,
    clock_synchronized: bool,
    clock_sync_inflight: bool,
    clock_gate: crate::time_sync::Gate,
    region_confirmed: bool,
    keyboard_selection: QString,
    preview_keyboard_search: QString,
    keyboards_customized: bool,
    keyboard_session_ready: bool,
    keyboard_session_failed: bool,
    session_keyboard_applied: String,
    session_keyboard_inflight: bool,
    active_keyboard_index: i32,
    keyboard_switching: bool,
    keyboard_control_inflight: bool,
    requested_keyboard: Option<String>,
    preview: bool,
    preview_width: i32,
    preview_height: i32,
    snapshot_path: QString,
    logo_source: QString,
    preview_icons: QString,
}

impl Default for SetupBackendRust {
    fn default() -> Self {
        let progress = crate::load_setup_progress();
        let saved = !crate::preview_mode()
            && crate::keyboard::validate_selection(&progress.keyboards, crate::keyboard::catalog())
                .is_ok();
        let preview_keyboards = crate::preview_argument("keyboards").filter(|selection| {
            crate::keyboard::validate_selection(selection, crate::keyboard::catalog()).is_ok()
        });
        let preview_customized = preview_keyboards.is_some();
        let preview_clock = crate::preview_mode()
            && crate::preview_argument("clock-sync").as_deref() == Some("confirmed");
        let region = crate::preview_argument("region")
            .filter(|code| crate::country::valid(code))
            .or_else(|| {
                (!crate::preview_mode() && crate::country::valid(&progress.region))
                    .then(|| progress.region.clone())
            })
            .unwrap_or_else(|| crate::country::default_country(crate::i18n::language()));
        Self {
            busy: crate::preview_mode()
                && crate::preview_argument("page").as_deref() == Some("4")
                && crate::preview_argument("error").is_none(),
            completed: false,
            error_message: if crate::preview_argument("error").is_some() {
                QString::from(&crate::i18n::translate("setupError"))
            } else {
                QString::default()
            },
            right_to_left: crate::i18n::rtl(crate::i18n::language()),
            selected_language: QString::from(crate::i18n::language()),
            selected_region: QString::from(&region),
            selected_timezone: QString::from(&crate::timezone::initial(
                &crate::preview_argument("timezone").unwrap_or_else(|| {
                    if crate::preview_mode() {
                        String::new()
                    } else {
                        progress.timezone.clone()
                    }
                }),
                &region,
            )),
            timezone_customized: (!crate::preview_mode()
                && progress.timezone_customized
                && crate::timezone::valid(&progress.timezone))
                || crate::preview_argument("timezone").is_some(),
            clock_synchronized: preview_clock,
            clock_sync_inflight: false,
            clock_gate: if preview_clock {
                crate::time_sync::Gate::preview_confirmed()
            } else {
                crate::time_sync::Gate::default()
            },
            region_confirmed: !crate::preview_mode() && progress.region_confirmed,
            keyboard_selection: QString::from(&if let Some(selection) = preview_keyboards {
                selection
            } else {
                crate::keyboard_profiles::recovered_selection(
                    &progress.keyboards,
                    !crate::preview_mode() && progress.keyboards_customized,
                    crate::i18n::language(),
                    &region,
                    crate::keyboard::catalog(),
                )
            }),
            keyboards_customized: preview_customized || (saved && progress.keyboards_customized),
            keyboard_session_ready: crate::preview_mode(),
            keyboard_session_failed: false,
            session_keyboard_applied: String::new(),
            session_keyboard_inflight: false,
            active_keyboard_index: if crate::preview_mode() { 0 } else { -1 },
            keyboard_switching: false,
            keyboard_control_inflight: false,
            requested_keyboard: None,
            preview_keyboard_search: QString::from(
                &crate::preview_argument("keyboard-search").unwrap_or_default(),
            ),
            preview: crate::preview_mode(),
            preview_width: crate::preview_argument("width")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1280)
                .clamp(240, 8192),
            preview_height: crate::preview_argument("height")
                .and_then(|v| v.parse().ok())
                .unwrap_or(720)
                .clamp(240, 8192),
            snapshot_path: QString::from(&crate::preview_argument("snapshot").unwrap_or_default()),
            logo_source: QString::from(
                &crate::preview_argument("logo")
                    .unwrap_or_else(|| "file:///usr/share/pixmaps/flufflinux-logo.svg".to_string()),
            ),
            preview_icons: QString::from(&crate::preview_argument("icons").unwrap_or_default()),
        }
    }
}

impl qobject::SetupBackend {
    pub fn keyboard_list(
        &self,
        language: &QString,
        selection: &QString,
        region: &QString,
    ) -> QString {
        QString::from(&crate::keyboard::catalog_json(
            &language.to_string(),
            &selection.to_string(),
            &region.to_string(),
        ))
    }

    fn update_keyboards(mut self: Pin<&mut Self>, selection: Option<String>) {
        if *self.busy() || *self.completed() {
            return;
        }
        if let Some(selection) = selection {
            self.as_mut().rust_mut().keyboards_customized = true;
            self.as_mut()
                .set_keyboard_selection(QString::from(&selection));
            self.as_mut().sync_keyboard_session();
        }
    }

    pub fn add_keyboard(self: Pin<&mut Self>, id: &QString) {
        let selection = crate::keyboard::add(
            &self.keyboard_selection().to_string(),
            &id.to_string(),
            crate::keyboard::catalog(),
        );
        self.update_keyboards(selection);
    }

    pub fn remove_keyboard(self: Pin<&mut Self>, id: &QString) {
        let selection =
            crate::keyboard::remove(&self.keyboard_selection().to_string(), &id.to_string());
        self.update_keyboards(selection);
    }

    pub fn default_keyboard(self: Pin<&mut Self>, id: &QString) {
        let selection =
            crate::keyboard::make_default(&self.keyboard_selection().to_string(), &id.to_string());
        self.update_keyboards(selection);
    }

    pub fn move_keyboard(self: Pin<&mut Self>, id: &QString, direction: i32) {
        let selection = crate::keyboard::move_layout(
            &self.keyboard_selection().to_string(),
            &id.to_string(),
            direction,
        );
        self.update_keyboards(selection);
    }

    pub fn reorder_keyboard(self: Pin<&mut Self>, id: &QString, destination: i32) {
        let selection = crate::keyboard::reorder_layout(
            &self.keyboard_selection().to_string(),
            &id.to_string(),
            destination,
        );
        self.update_keyboards(selection);
    }

    pub fn translate(&self, key: &QString) -> QString {
        QString::from(&crate::i18n::translate(&key.to_string()))
    }

    pub fn preview_keyboard_menu(&self) -> bool {
        *self.preview() && crate::preview_argument("keyboard-menu").is_some()
    }

    pub fn switch_keyboard_layout(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let selection = self.keyboard_selection().to_string();
        let Some(index) = crate::keyboard::parse_selection(&selection)
            .and_then(|ids| ids.iter().position(|entry| *entry == id))
        else {
            return;
        };
        if *self.busy() || *self.completed() || !*self.keyboard_session_ready() {
            return;
        }
        if *self.preview() {
            self.as_mut().set_active_keyboard_index(index as i32);
            return;
        }
        // Preserve a menu choice made while a status query is in flight.
        // IDs, not indices, keep a queued request safe across reordering.
        self.as_mut().rust_mut().requested_keyboard = Some(id);
        self.refresh_keyboard_layout();
    }

    pub fn refresh_keyboard_layout(mut self: Pin<&mut Self>) {
        if *self.preview()
            || *self.busy()
            || *self.completed()
            || !*self.keyboard_session_ready()
            || self.session_keyboard_inflight
            || self.keyboard_control_inflight
        {
            return;
        }
        let selection = self.keyboard_selection().to_string();
        let requested = self
            .as_mut()
            .rust_mut()
            .requested_keyboard
            .take()
            .and_then(|id| {
                crate::keyboard::parse_selection(&selection)?
                    .iter()
                    .position(|entry| *entry == id)
            });
        self.as_mut().rust_mut().keyboard_control_inflight = true;
        self.as_mut().set_keyboard_switching(requested.is_some());
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = crate::keyboard::active_layout(&selection, requested);
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().rust_mut().keyboard_control_inflight = false;
                backend.as_mut().set_keyboard_switching(false);
                // A removed layout or an old map must never relabel a new map.
                if backend.keyboard_selection().to_string() == selection
                    && backend.session_keyboard_applied == selection
                    && *backend.keyboard_session_ready()
                    && !*backend.busy()
                    && !*backend.completed()
                {
                    match result {
                        Ok(index) => backend.as_mut().set_active_keyboard_index(index),
                        Err(error) => {
                            if *backend.active_keyboard_index() >= 0 {
                                eprintln!("{error}");
                            }
                            backend.as_mut().set_active_keyboard_index(-1);
                            if error == "KWin keyboard configuration changed." {
                                backend.as_mut().set_keyboard_session_failed(true);
                                backend.as_mut().set_keyboard_session_ready(false);
                            }
                        }
                    }
                }
                if !*backend.keyboard_session_ready() {
                    backend.as_mut().sync_keyboard_session();
                } else if backend.requested_keyboard.is_some() {
                    backend.as_mut().refresh_keyboard_layout();
                }
            });
        });
    }

    pub fn sync_keyboard_session(mut self: Pin<&mut Self>) {
        if *self.preview() || *self.busy() || *self.completed() {
            return;
        }
        let selection = self.keyboard_selection().to_string();
        if self.session_keyboard_applied == selection
            && !self.session_keyboard_inflight
            && !*self.keyboard_session_failed()
        {
            self.as_mut().set_keyboard_session_ready(true);
            return;
        }
        self.as_mut().set_keyboard_session_ready(false);
        self.as_mut().set_active_keyboard_index(-1);
        if self.session_keyboard_inflight || self.keyboard_control_inflight {
            return;
        }
        self.as_mut().rust_mut().session_keyboard_inflight = true;
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = crate::keyboard::sync_session(&selection);
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().rust_mut().session_keyboard_inflight = false;
                match result {
                    Ok(()) => {
                        backend.as_mut().rust_mut().session_keyboard_applied = selection.clone();
                        backend.as_mut().set_keyboard_session_failed(false);
                        let ready = backend.keyboard_selection().to_string() == selection;
                        backend.as_mut().set_keyboard_session_ready(ready);
                        if ready {
                            backend.as_mut().set_active_keyboard_index(0);
                        }
                    }
                    Err(error) => {
                        if !*backend.keyboard_session_failed() {
                            eprintln!("{error}");
                        }
                        backend.as_mut().set_keyboard_session_failed(true);
                    }
                }
                // Changes made while this worker ran get applied serially next,
                // never racing a newer selection or blocking the GUI thread.
                if backend.keyboard_selection().to_string() != selection {
                    backend.as_mut().sync_keyboard_session();
                } else if backend.requested_keyboard.is_some() {
                    backend.as_mut().refresh_keyboard_layout();
                }
            });
        });
    }

    pub fn language_list(&self) -> QString {
        let rows = crate::i18n::ordered_languages()
            .iter()
            .map(|language| {
                format!(
                    "{{\"code\":\"{}\",\"english\":\"{}\",\"native\":\"{}\"}}",
                    language.code, language.english, language.native
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        QString::from(&format!("[{rows}]"))
    }

    pub fn exercise_preview(&self) -> bool {
        crate::preview_mode() && crate::preview_argument("exercise").is_some()
    }

    pub fn preview_timezone_search(&self) -> QString {
        QString::from(&if *self.preview() {
            crate::preview_argument("timezone-search").unwrap_or_default()
        } else {
            String::new()
        })
    }

    pub fn base_language(&self, code: &QString) -> QString {
        QString::from(crate::i18n::base_language(&code.to_string()).unwrap_or("en"))
    }

    pub fn region_list(&self, code: &QString) -> QString {
        let code = code.to_string();
        let Some(language) = crate::i18n::metadata(&code) else {
            return QString::from("[]");
        };
        let rows = crate::i18n::ordered_regions(&code).iter().map(|region| {
            let choice = crate::i18n::canonical_choice(region.code).unwrap();
            format!("{{\"code\":\"{choice}\",\"english\":\"{}\",\"native\":\"{} ({})\",\"label\":\"{} ({})\",\"cycle\":\"{}\"}}",
                region.country, language.native, region.country, language.native, region.country, region.country)
        }).collect::<Vec<_>>().join(",");
        QString::from(&format!("[{rows}]"))
    }

    pub fn country_list(&self, language: &QString) -> QString {
        QString::from(&crate::country::list_json(&language.to_string()))
    }

    pub fn select_region(mut self: Pin<&mut Self>, code: &QString) {
        let code = code.to_string();
        if *self.busy() || *self.completed() || !crate::country::valid(&code) {
            return;
        }
        self.as_mut().rust_mut().region_confirmed = true;
        self.as_mut().set_selected_region(QString::from(&code));
        if !self.timezone_customized {
            self.as_mut()
                .set_selected_timezone(QString::from(&crate::timezone::initial("", &code)));
        }
        if !self.keyboards_customized {
            let selection = crate::keyboard::defaults_for_region(
                &self.selected_language().to_string(),
                &code,
                crate::keyboard::catalog(),
            );
            self.as_mut()
                .set_keyboard_selection(QString::from(&selection));
        }
        self.as_mut().sync_keyboard_session();
    }

    pub fn timezone_list(&self, language: &QString, region: &QString) -> QString {
        QString::from(&crate::timezone::list(
            &language.to_string(),
            &region.to_string(),
        ))
    }

    pub fn timezone_clock(&self) -> QString {
        let now = std::time::Instant::now();
        QString::from(&crate::clock::preview(
            &self.selected_timezone().to_string(),
            &self.selected_language().to_string(),
            &self.selected_region().to_string(),
            self.clock_gate.ready(now),
            self.clock_gate.source(now),
        ))
    }

    pub fn refresh_clock_sync(mut self: Pin<&mut Self>) {
        if self.clock_sync_inflight || *self.busy() || *self.completed() {
            return;
        }
        if *self.preview() && crate::preview_argument("clock-sync").as_deref() != Some("system") {
            let confirmed = crate::preview_argument("clock-sync").as_deref() == Some("confirmed");
            self.as_mut().rust_mut().clock_gate = if confirmed {
                crate::time_sync::Gate::preview_confirmed()
            } else {
                crate::time_sync::Gate::default()
            };
            self.as_mut().set_clock_synchronized(confirmed);
            return;
        }
        self.as_mut().rust_mut().clock_sync_inflight = true;
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let observation = crate::time_sync::inspect();
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().rust_mut().clock_sync_inflight = false;
                let ready = backend
                    .as_mut()
                    .rust_mut()
                    .clock_gate
                    .update(observation, std::time::Instant::now());
                backend.as_mut().set_clock_synchronized(ready);
            });
        });
    }

    pub fn timezone_map_text(&self, context: &QString, source: &QString) -> QString {
        QString::from(&crate::keyboard_names::timezone_map_message(
            &self.selected_language().to_string(),
            &context.to_string(),
            &source.to_string(),
        ))
    }

    pub fn select_timezone(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        if !*self.busy() && !*self.completed() && crate::timezone::valid(&id) {
            self.as_mut().rust_mut().timezone_customized = true;
            self.as_mut().set_selected_timezone(QString::from(&id));
        }
    }

    pub fn select_language(mut self: Pin<&mut Self>, code: &QString) {
        if *self.busy() || *self.completed() {
            return;
        }
        let code = code.to_string();
        if !crate::i18n::set_language(&code) {
            return;
        }
        self.as_mut()
            .set_selected_language(QString::from(crate::i18n::language()));
        self.as_mut().set_right_to_left(crate::i18n::rtl(&code));
        if !self.region_confirmed {
            self.as_mut()
                .set_selected_region(QString::from(&crate::country::default_country(&code)));
            if !self.timezone_customized {
                let region = self.selected_region().to_string();
                self.as_mut()
                    .set_selected_timezone(QString::from(&crate::timezone::initial("", &region)));
            }
        }
        if !self.keyboards_customized {
            let region = self.selected_region().to_string();
            self.as_mut().set_keyboard_selection(QString::from(
                &crate::keyboard::defaults_for_region(&code, &region, crate::keyboard::catalog()),
            ));
        }
        if !self.error_message().is_empty() {
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate("setupError")));
        }
        self.as_mut().sync_keyboard_session();
    }

    pub fn current_hostname(&self) -> QString {
        if crate::preview_mode() {
            return QString::from("fluff-laptop");
        }
        QString::from(&crate::current_or_generated_hostname())
    }

    pub fn default_name(&self) -> QString {
        QString::from(crate::default_display_name())
    }

    pub fn resume_page(&self) -> i32 {
        if crate::preview_mode() {
            return crate::preview_argument("page")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
                .clamp(0, 9);
        }
        crate::load_setup_progress().resume_page()
    }

    pub fn saved_hostname(&self) -> QString {
        if crate::preview_mode() {
            return QString::from(&crate::preview_argument("hostname").unwrap_or_default());
        }
        QString::from(&crate::load_setup_progress().hostname)
    }

    pub fn saved_name(&self) -> QString {
        if crate::preview_mode() {
            return QString::default();
        }
        QString::from(&crate::load_setup_progress().display_name)
    }

    pub fn account_already_created(&self) -> bool {
        if crate::preview_mode() {
            return false;
        }
        let progress = crate::load_setup_progress();
        !progress.username.is_empty()
            && crate::account_matches_setup_identity(&progress.username, &progress.display_name)
    }

    pub fn save_progress(&self, page: i32, hostname: &QString, display_name: &QString) {
        if crate::preview_mode() {
            return;
        }
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
            username: if preserve_account {
                previous.username
            } else {
                String::new()
            },
            account_created: preserve_account && previous.account_created,
            language: self.selected_language().to_string(),
            keyboards: self.keyboard_selection().to_string(),
            keyboards_customized: self.keyboards_customized,
            region: self.selected_region().to_string(),
            region_confirmed: self.region_confirmed,
            timezone: self.selected_timezone().to_string(),
            timezone_customized: self.timezone_customized,
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

        QString::from(&crate::hostname_validation_message(
            &hostname,
            crate::i18n::language(),
        ))
    }

    pub fn validate_name(&self, display_name: &QString) -> QString {
        let display_name = display_name.to_string();
        if display_name.trim().is_empty() {
            return QString::default();
        }

        let error = crate::validate_display_name(&display_name)
            .err()
            .map(|_| crate::i18n::translate("nameError"))
            .unwrap_or_default();
        QString::from(&error)
    }

    pub fn start_setup(
        mut self: Pin<&mut Self>,
        hostname: &QString,
        display_name: &QString,
        password: &QString,
    ) {
        if crate::preview_mode() {
            return;
        }
        if *self.busy() || *self.completed() {
            return;
        }
        if crate::i18n::metadata(&self.selected_language().to_string()).is_none()
            || !crate::country::valid(&self.selected_region().to_string())
            || !self.region_confirmed
        {
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate("setupError")));
            return;
        }
        if !crate::timezone::valid(&self.selected_timezone().to_string()) {
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate("setupError")));
            return;
        }
        // A recovered account page may be shown before KWin has restored its
        // keyboards. Do not finalize while that session worker is still active.
        if !*self.keyboard_session_ready() {
            self.as_mut().sync_keyboard_session();
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate(
                    "keyboardSessionError",
                )));
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

        let hostname_error = crate::hostname_validation_message(&hostname, crate::i18n::language());
        if !hostname_error.is_empty() {
            self.as_mut()
                .set_error_message(QString::from(&hostname_error));
            return;
        }
        if crate::validate_display_name(&display_name).is_err() {
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate("nameError")));
            return;
        }
        if password.is_empty() {
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate("passwordError")));
            return;
        }

        let previous = crate::load_setup_progress();
        let username = if !previous.username.is_empty() && previous.display_name == display_name {
            previous.username
        } else {
            crate::generated_username(&display_name)
        };
        if let Err(error) = crate::validate_username(&username) {
            eprintln!("{error}");
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate("setupError")));
            return;
        }
        if crate::user_exists(&username)
            && !crate::account_matches_setup_identity(&username, &display_name)
        {
            eprintln!("The reserved account name belongs to a different system account.");
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate("setupError")));
            return;
        }
        if let Err(error) = crate::save_setup_progress(&crate::SetupProgress {
            page: 3,
            hostname: hostname.clone(),
            display_name: display_name.clone(),
            username: username.clone(),
            account_created: crate::account_matches_setup_identity(&username, &display_name),
            language: self.selected_language().to_string(),
            keyboards: self.keyboard_selection().to_string(),
            keyboards_customized: self.keyboards_customized,
            region: self.selected_region().to_string(),
            region_confirmed: self.region_confirmed,
            timezone: self.selected_timezone().to_string(),
            timezone_customized: self.timezone_customized,
        }) {
            eprintln!("Could not reserve the account for recovery: {error}");
            self.as_mut()
                .set_error_message(QString::from(&crate::i18n::translate("setupError")));
            return;
        }

        self.as_mut().set_error_message(QString::default());
        self.as_mut().set_completed(false);
        self.as_mut().set_busy(true);
        let qt_thread = self.qt_thread();
        let language = self.selected_language().to_string();
        let region = self.selected_region().to_string();
        let timezone = self.selected_timezone().to_string();
        let keyboards = self.keyboard_selection().to_string();

        // Privileged commands must not block Qt's event loop. Their result is
        // sent back through the CXX Qt thread safe queue.
        std::thread::spawn(move || {
            let result = apply_in_child([
                &hostname,
                &display_name,
                &username,
                &language,
                &region,
                &keyboards,
                &timezone,
                &password,
            ]);
            let _ =
                qt_thread.queue(move |mut backend| {
                    backend.as_mut().set_busy(false);
                    match result {
                        Ok(()) => {
                            backend.as_mut().set_completed(true);
                            backend.as_mut().set_error_message(QString::default());
                        }
                        Err(error) => {
                            eprintln!("{error}");
                            backend.as_mut().set_completed(false);
                            backend.as_mut().set_error_message(QString::from(
                                &crate::i18n::translate("setupError"),
                            ));
                        }
                    }
                });
        });
    }
}

fn apply_in_child(details: [&str; 8]) -> Result<(), String> {
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
    let details = crate::setup_payload(details);
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
