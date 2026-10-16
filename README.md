# fluffsetup

FluffSetup is the first-boot account and system-name setup program for Fluff
Linux. Version 1.5 provides a focused Qt-based graphical setup experience with
the clear, staged layout of a modern first-boot flow.

The setup behavior is written in Rust and exposed to a Qt Quick/QML interface
through the same small CXX-Qt binding stack used by FluffInstall. The resulting
application is a single executable. When started by the FluffSetup Wayland
session, it paints the Fluff Linux background on every connected display and
opens a centered setup card for the hostname, display-name, and password flow
adapted from FluffInstall 0.9.

## Build requirements

- Rust and Cargo
- a C++ compiler, pkg-config, and the Qt 6 Core/GUI/QML development files
- KDE Kirigami's QML module for native wheel/touchpad scrolling at runtime
- Plasma Workspace's installed timezone selector, Qt Location/Positioning and
  tzdata at runtime for the offline map and installed IANA timezone catalog
- the system `xkeyboard-config` registry and `kbd` console maps at runtime
- ICU for localized geographic country names and its system `uconv` tool for
  readable international-name transliteration, without adding a Rust crate

On Fluff Linux or Arch Linux, install the build tools with:

```bash
sudo pacman -S --needed rust gcc pkgconf qt6-base qt6-declarative qt6-location qt6-positioning kirigami plasma-workspace tzdata icu xkeyboard-config
```

Build the release executable from the repository root:

```bash
cargo build --release
```

Run its validation tests with:

```bash
cargo test
```

The locale persistence regression covers every language and regional entry,
including gettext fallback lists, independent regional formats and neutral `C`
formats. On a Fluff machine with `glibc-locales`, also run the read-only native
inventory checks explicitly:

```bash
cargo test --locked every_installed_language_profile_loads_without_locale_warnings -- --ignored --nocapture
cargo test --locked installed_region_formats_match_glibc_categories -- --ignored --nocapture
```

These checks load installed locale data and inspect formatting without changing
the system or user's language, regional preferences or timezone.

The resulting executable is:

```text
target/release/fluffsetup
```

## Stage for the Fluff Linux ISO

FluffInstall expects the compiled binary beside the FluffSetup session files
in the live ISO:

```text
/usr/lib/fluffinstall/fluffsetup/
├── fluffsetup
├── fluffsetup-session
└── fluffsetup.desktop
```


During installation, FluffInstall copies that binary into the installed system
as `/usr/bin/fluffsetup` with mode `0755`. The `fluffsetup-session` launcher
should execute `/usr/bin/fluffsetup`; FluffSetup then opens.

## Current behavior

- offers a randomized `FL-AA0000`-style hostname when none is entered
- validates custom hostnames using the FluffInstall 0.9 rules
- asks for the person's display name rather than exposing a Unix user-name
  field, and stores that original value in the account's full-name/GECOS field
- accepts a blank name as the default display name `User`, deriving `user` or
  the next available numeric variant as its hidden login name
- derives a safe lowercase ASCII login name automatically; a single word is
  used whole, while a multi-word name becomes the first word plus the initial
  of each remaining word
- transliterates international names with ICU, normalizes diacritics, falls
  back safely when no usable letters remain, and adds `2`, `3`, and later
  numeric suffixes whenever the generated login name is already present in the
  system account database, including built-in and service accounts
- starts with U.S. English selected on a language page; language changes preview
  immediately, including the page title and RTL/LTR direction
- lists English and native language names together, with English, Spanish,
  French, Portuguese, Ukrainian, Hebrew, Russian, Arabic, and German first,
  followed by the rest sorted by English name; arrow keys select,
  letters cycle by English initial, and either Enter key continues
- follows language selection with a regional page where multiple country locales
  exist, using native labels such as `Deutsch (Österreich)`; the established
  default country is first, other countries follow by name, and both Enter keys,
  arrows, and country-initial cycling work without showing English duplicates
- uses one Kirigami wheel handler, native touch flicking, and a visible scrollbar;
  the language page's Next button aligns with the rows rather than the scrollbar
- adds a geographic Region page after Welcome, with native country names,
  language-based suggestions, and a KDE globe icon; there is no search field
- provides graphical language, welcome, region, keyboard-layout, system-name, timezone, account, setup-progress, and
  completion pages using the Fluff Linux `#820101` accent
- keeps the setup window above the temporary first-boot background and supports
  keyboard tab navigation plus Enter-to-continue on valid text-entry pages;
  tab-focused buttons receive a black outline without showing that outline for
  normal mouse or programmatic focus
- sizes and centers the setup window from its assigned output, including on
  mixed-resolution multi-monitor systems, and rebinds it to the matching
  background whenever its output changes; the complete branded layout scales
  as one unit on narrow portrait and low-resolution outputs
- keeps the setup window application-modal so clicking its temporary
  background cannot move interaction away from setup
- atomically saves the selected language, ordered keyboard layouts/default, current page, system name, display name, and reserved
  internal account name so an interrupted first boot resumes where it stopped;
  passwords are never written to the recovery file
- requires a valid selected row before advancing from language, dialect or
  geographic-region selection, including after recovery; a saved completion-page
  number cannot enable Finish without a successful finalization result
- resolves untouched fields to their visible gray defaults both in the GUI and
  in recovery state (`User` for the display name and the generated system name
  for the hostname), while rejecting entered spaces in a system name
- if power is lost after the permanent account is created, verifies that the
  reserved account still has the expected display name, home directory, and
  shell, then reuses it and resumes the remaining setup steps instead of
  creating a second account; a mismatched existing account is never modified
- keeps both password fields visible by default, clearly warns the user before
  entry, and provides an eye control to hide or reveal both fields together
- creates the permanent user with Zsh and membership in `uucp`, `wheel`, `kvm`,
  and `libvirt`
- grants `libvirt-qemu` access to the permanent user's home directory and gives
  the Flatpak version of Virtual Machine Manager access to home directories
- points Dolphin's home location at the permanent user's home directory
- makes the permanent user's Trash desktop entry root-owned so KDE treats it as
  protected from ordinary removal
- sets the permanent user's password
- applies the selected UTF-8 locale to the system, permanent user, Plasma, and
  AccountsService (when installed) only during finalization; generates the
  selected locale if needed, applies the geographic region's date/time, currency,
  number, measurement, paper-size, address, personal-name and telephone formats
  separately, and preserves unrelated preferences
- writes system locale settings atomically and reloads the system manager's
  defaults, preserving gettext's colon-separated `LANGUAGE` fallback list
  without passing it through `localectl`'s single-locale validation
- displays the same KDE `Cluster` wallpaper used by Fluff Linux on every
  connected output, as part of the running FluffSetup process:
  `/usr/share/wallpapers/Cluster/contents/images/3840x2160.png`
- removes the temporary setup account and every temporary setup artifact before
  showing the successful summary, so restarting or powering off from that page
  cannot start FluffSetup again
- the authorized cleanup removes temporary login settings, recovery state, the
  Wayland session, launcher, passwordless sudo rule, AccountsService record,
  `fluffsetup` account and home directory, and the FluffSetup binary; pressing
  Finish then exits the already-running temporary Wayland session so Plasma
  Login Manager returns to its greeter

The installed system provides passwordless sudo access to the temporary
`fluffsetup` account only during first boot. FluffSetup removes that permission
before showing the completion summary.

Cleanup is deliberately completed only after the hostname, permanent account,
password, language, and account settings have all been applied successfully, but before
the completion summary becomes visible. Closing or interrupting setup before
that point leaves the temporary setup environment available for another attempt.
The root cleanup is a direct `sudo` invocation of FluffSetup's built-in cleanup
mode and does not install or schedule a systemd service. The temporary sudo rule
is removed by that cleanup.

## Development workflow

### Languages and layout previews

The interface includes 96 translated language/script choices in `translations/catalog.tsv`,
`translations/hostname-errors.tsv`, `translations/additional.tsv`, and `translations/windows.tsv`.
The first run selects U.S. English; recovery restores the exact language and region.
`translations/languages.tsv` contains picker names, titles, and standard
UTF-8 locale mappings. `translations/regions.tsv` provides localized country
names for multi-region languages. Regional wording inherits the base catalog,
with sparse overrides in `translations/regional.tsv`; Swiss German uses `ss`
instead of `ß`. Serbian Latin and Valencian preserve their glibc locale modifiers
through selection, recovery, and finalization. Punjabi Gurmukhi and Shahmukhi
are separate choices. Arabic, Hebrew, Persian, Urdu, Pashto, Uyghur, and Shahmukhi Punjabi
use a mirrored right-to-left layout. An empty display name always defaults
to `User`, regardless of the selected language.

The complete branded layout scales uniformly to the assigned screen's
logical dimensions. A 960 by 820 reference layout leaves room for wrapped
translations; there is no minimum scale that would clip a small display.
The existing compact-screen threshold and monitor recovery remain unchanged.
Sidebar symbols use the system icon theme with built-in SVG fallbacks.

Preview mode does not change system settings, save setup progress, or start
privileged helpers. For example, after building:

```sh
LANGUAGE=he QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software \
  target/debug/fluffsetup --preview --page=3 --width=800 --height=600 \
  --snapshot=/tmp/fluffsetup-he.png
python3 tools/render_previews.py --output=/tmp/fluffsetup-previews
```

Preview mode uses the session locale or explicit `--language=he`. Page 0 is
language selection, followed by welcome (1), system name (2), timezone (9), account (3),
progress (4), and summary (5).
The regional picker is page 6, reached between pages 0 and 1. Existing page
numbers remain stable for recovery compatibility. Languages with only one
country skip this dialect page. The geographic Region page is page 8, reached
after Welcome (1), followed by Keyboard (7), system name (2), and timezone (9).
Existing page numbers remain stable. Recovery format v6 also stores the selected
IANA timezone and whether it was manually selected, separate from language,
geographic region and ordered keyboards. Recovered automatic choices still
follow subsequent region changes; explicit choices do not.
It accepts v5, v4, v3, v2 and older progress records. An unfinished account page
without a valid saved timezone resumes at timezone selection. Passwords remain
excluded. `--timezone=Asia/Jerusalem` selects a zone in safe previews.

### Geographic region

The Region page uses the installed ICU ISO country list and country names in the
selected interface language. Countries associated with the supported language
locales appear under Suggested regions, with the selected language/dialect's
country first. Remaining regions follow alphabetically by localized name.
Palestine (`PS`) is excluded from the picker, suggestions and recovered region
values. There is no search field. Arrow keys, Home/End, Page Up/Down, both Enter
keys and country-initial cycling work, including English initials before a local
keyboard has been chosen. Kirigami wheel input, the scrollbar and native touch
flicking share the same list. Next aligns with the country rows.

Geography does not replace the selected interface locale, alter its dialect or
set a timezone. Only during setup finalization, it sets `LC_TIME`, `LC_MONETARY`,
`LC_NUMERIC`, `LC_MEASUREMENT`, `LC_PAPER`, `LC_ADDRESS`, `LC_NAME` and
`LC_TELEPHONE` for the system and permanent user, including
Plasma's separate format settings. `LANG`, `LC_MESSAGES` and `LANGUAGE` continue
to follow the chosen interface language/dialect. Both selections already survive
recovery in the v6 record; finalization passes the restored geographic region
explicitly to the privileged helper. Preview and selection never apply formats.

Formatting uses UTF-8 locales actually installed or supplied by glibc, preferring
the selected language within the country, then English within that country,
then ICU's default country language and other locales for that same country.
Script/dialect modifiers are preserved where available. No locale name is
invented, and another country's formats are never substituted. Regions with no
glibc UTF-8 locale use the neutral `C` locale for these eight categories without
a warning. When the country has no locale in the chosen interface language,
date/month names may follow the formatting locale's language; interface messages
still follow the selected language. Formatting does not change unrelated locale
categories, keyboard order or timezone settings. Paper size, postal
address templates, personal-name ordering and telephone prefixes come from the
same region locale, including inherited/default glibc data. These are locale
preferences for applications that honor them, not changes to the entered user's
name, hostname, keyboard layouts or contact information. Native glibc updates
are discovered at finalization; no hardcoded unsupported-region list is used.

Geography also determines keyboard defaults using Linux XKB equivalents
guided by [Microsoft's default input profiles](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/default-input-locales-for-windows-language-packs).
Windows uses both language and region, including Belgian French, Swiss German/
French, and Arabic AZERTY in Algeria, Morocco and Tunisia. The embedded
`data/keyboard-profiles.tsv` maps 248 language/region profiles across all 96
supported language/script choices to XKB. `data/keyboard-regions.tsv` supplies
189 regional defaults; regions without a known profile safely fall back to U.S.
English. These are Linux equivalents of the documented input profiles, not
byte-for-byte Windows keyboard drivers. Period/comma and other minor differences
can share a main national XKB layout. Explicit `preferred|fallback` alternatives
are script-compatible; unavailable variants are never replaced with another script.

There is no blanket U.S. secondary. Documented language/region profiles are used
first. For combinations not listed by Microsoft, regional profiles are combined
with the selected language's input, duplicates removed, and a Latin fallback
included only when needed. A regional Latin keyboard can replace a redundant
U.S. secondary. When the selected language is already one of an unlisted region's
defaults, that keyboard is preferred. English uses an existing regional Latin
keyboard without adding a second English layout. Israel always starts with U.S.
English and Hebrew, followed by the selected language if needed (Russian gives
`us,il,ru`). Windows-only-English fallback profiles are supplemented with the
selected language's actual input where available, rather than losing that input.

For example: English/Belgium is Belgian AZERTY only; French/Canada is Canadian
French only; Arabic/Algeria is Arabic AZERTY plus French; Russian/Germany is German
plus Russian; French/Germany is German plus French. Exact Microsoft profiles and
the smart extension for unlisted/fallback combinations are deliberately distinct.
The four-group XKB limit reserves the regional primary, required language input
and an ASCII-capable keyboard before optional auxiliary profiles. Hostnames and
passwords must remain typeable even when all native profiles use non-Latin scripts.

Only variants needed by documented main profiles are offered, such as U.S.
International for Dutch, UK Extended for Welsh, and required script layouts;
this does not reintroduce hundreds of optional dead-key/hardware variants.
Kurmanji uses its Latin XKB keyboard, not Microsoft's different Sorani input.
Arabic-script Punjabi uses an Urdu keyboard, as Microsoft's profile does, never
the unrelated Arabic-only `pk:ara` variant. Windows IMEs are not installed: Japanese
and Korean entries provide physical keyboards, not complete IME input; Chinese
keeps a Latin physical keyboard until input-method integration is implemented.
As a temporary exception to Latin-fallback deduplication, Japanese, Korean and
both Chinese script choices explicitly include U.S. English QWERTY, even when
another Latin keyboard is present. Automatic profiles containing Japanese or
Korean keyboards also include it. Existing default order is preserved, U.S.
English is added only once, and room is reserved within the four-group limit.
Manually removed or reordered keyboards remain untouched; a missing automatic
U.S. fallback is only suggested. This does not provide Japanese/Chinese/Korean
composition or conversion. TODO: add proper system input-method engines and
Wayland/Plasma integration in Fluff Linux, then connect FluffSetup's selections
and reconsider this temporary fallback policy.

Once keyboards have been customized, changing region or language preserves the
selection and order. Missing defaults for the new region become suggestions;
they are not silently re-added. A confirmed region also survives language changes.
Untouched automatic selections are recalculated on recovery under the current
profile rules, so older automatic U.S. secondaries do not become permanent.
The page uses KDE Breeze's original [globe SVG](https://github.com/KDE/breeze-icons/blob/master/icons/actions/32/globe.svg),
bundled with the existing Breeze license files for consistent rendering.

### Keyboard selection

The keyboard page reads the installed XKB registry and offers main national
layouts plus the language/script and regional variants needed by the supported
locale defaults, and major alternative arrangements such as QWERTY, AZERTY,
QWERTZ, Dvorak, Colemak and BEPO. Optional minor dead-key, punctuation and hardware
subvariants are not offered; a required main input profile is still retained.
Known main Latin arrangements are labeled explicitly;
non-Latin arrangements are not guessed. Older saved selections remain visible
for recovery. Up to four layouts can be selected from one searchable list.
Suggested layouts appears only when one or more regional defaults are absent
from the selected layouts; all other choices remain alphabetical. Selecting the
missing defaults removes that section. A search that does not match a missing
default also hides the section.
Live search covers the list using English names, translated names, native
aliases and layout IDs. The search bar can also be used to test typing input.
The sidebar shows a keyboard indicator on keyboard testing, hostname and user
creation and timezone search only, and only when more than one keyboard layout is selected. Removing
layouts down to one hides the button and closes its menu; adding a second layout
shows it again. Hidden indicators do not poll KWin's active group.
An outlined button displays the full translated keyboard name
with KDE Breeze's small monochrome keyboard-layout icon and a menu arrow.
The original [22-pixel SVG](https://github.com/KDE/breeze-icons/blob/master/icons/status/22/keyboard-layout.svg)
is bundled with the existing Breeze artwork license
so the icon does not depend on the installed theme. Its width fits the
sidebar and stays 252 by 64 reference pixels regardless of the active layout.
The complete button scales with the interface. Layout qualifiers such as
`(QWERTY)` occupy the second line; long names fit within the same two-line face
without shrinking the button or splitting a short name such as Hebrew.
The popup spans only the sidebar, ending at its edge in both LTR and RTL.
Long layout names wrap and rows grow to display the full name without clipping.
Its menu switches the active typing layout without changing the saved default
or order, then returns focus to the previously active input field.
The indicator follows KWin's actual active group, including Alt+Shift and
Win+Space changes. Layout queries and switches run outside the GUI thread;
unconfirmed or unavailable groups are not shown as active. Both current and
legacy KWin keyboard-control interfaces are supported.
Language, locale, region and keyboard lists retain native touchscreen dragging/flicking
alongside one Kirigami wheel handler each. Touch scrolling does not activate rows;
tapping selects a language/locale or adds a keyboard.
The up/down controls swap a selected layout with its neighbor and preserve the
order for recovery and the created user's keyboard switching. The first layout
is always the default; moving another layout to the top also changes the default.
Selected layouts also support mouse and single-finger touch drag-to-reorder from
the grip or name area. The moving row and insertion marker show the destination;
releasing commits one atomic ordered update. Dragging to the top makes that layout
the default. The arrows and remove buttons remain available. There are no
separate default-selection radio buttons; the first row's label identifies it.
Selected rows have extra horizontal inset around the grip. Removal uses KDE
Breeze's original red [edit-delete SVG](https://github.com/KDE/breeze-icons/blob/master/icons/actions/22/edit-delete.svg), bundled under the existing Breeze icon
license; add-layout plus signs use App Center's green action accent. Disabled
actions remain visibly subdued, and the last layout still cannot be removed.
Taps, outside drops, Escape and interrupted gestures do not reorder. Model,
visibility, direction and display-size changes cancel an active drag safely.
Next aligns with the available layout rows, excluding the scrollbar gutter in
both LTR and RTL, just like the language/region pages.
The first row is the default, selected by ordering the list. Duplicate layouts
and removing the last layout are prevented. Israel starts with `us,il`, with
U.S. English as the default; French/France and German/Germany use only their
matching layouts. Russian/Russia starts with `ru,us`. Other combinations follow
the language/region profile rules above. Unavailable language input retains the
installed regional keyboards and an ASCII-capable fallback. Choosing
a display language does not install an input-method engine for languages such
as Chinese or Japanese. Selected layouts are activated in the dedicated temporary
FluffSetup KWin Wayland session. Alt+Shift and Win+Space switch between them while
typing during setup. Changes are serialized, acknowledged by KWin, and retried if
its keyboard service is temporarily unavailable; Next waits for activation.
Preview mode never changes a desktop's keyboard settings. System locale and
permanent user keyboard settings are still deferred until setup finalization.

After account creation, a user-owned `~/.config/kxkbrc` receives aligned
`LayoutList`/`VariantList` entries in default-first order. The skeleton's keyboard
model and existing compose/group-switch options are preserved; Alt+Shift and
Win+Space are added without duplicate options. `/etc/vconsole.conf` is updated with
the default layout's installed console counterpart through
`localectl --no-convert set-keymap`, preserving fonts and avoiding system X11
layout changes. French/Belgian AZERTY variants use compatible console maps
when an exact map is unavailable; BEPO/Dvorak/QWERTY are never changed to AZERTY.
If no console counterpart exists, the existing console map is retained. Arabic
AZERTY uses a French-geometry console fallback when an exact map is unavailable.

The panel uses KDE Breeze's `input-keyboard` icon, with the original SVG bundled
as a fallback under the existing Breeze artwork license in `assets/COPYING-ICONS`.
The SVG source is [KDE Breeze input-keyboard](https://github.com/KDE/breeze-icons/blob/master/icons/devices/64/input-keyboard.svg).
Keyboard page messages are translated for all 96 language/script choices.
Layout names use the installed `xkeyboard-config` gettext translations, with
region/script-safe fallbacks and English descriptions when a translation is
unavailable. Live activation uses KDE's system `kwriteconfig6` and D-Bus tools,
without additional Rust dependencies. Preview-only
`--keyboard-registry=/path/to/evdev.lst`, `--keyboard-translations=/path/to/locale`,
`--keyboards=us,il`, `--keyboard-search=French` and `--region=IL` support safe visual tests.
Run `tools/render_previews.py --country-only` to audit geographic Region (page 8).
Run `tools/render_previews.py --keyboard-only` to audit the keyboard page across
all language and regional choices, including four-layout/search/empty states.

Use `--hostname='my laptop'` on page 2 to preview hostname validation. Only
applicable errors appear, one per line; whitespace is not also reported as
an invalid character. The same rules protect setup's privileged helper.

The preview tool renders all languages at 800 by 600 and additionally tests
Hebrew, English, German, Arabic, Persian, Urdu, Pashto, both Chinese scripts,
Mongolian, and all eleven latest language/script additions at nine screen sizes. It also renders every regional choice
on every page at 800 by 600, and nine regional lists at all nine sizes,
including portrait, 16:9, 16:10, 4:3, and ultrawide. It records text-bound diagnostics
in `layout-audit.json`, including the expected error counts for valid names,
spaces, symbols, long names, and combined failures. These synthetic checks
do not replace testing on the target Linux compositor or native-speaker
translation review.

Run the picker interaction tests with Qt's `qmltestrunner -input tests/qml`.
They require the real Kirigami module and cover key navigation, both Enter
keys, letter cycling, pointer selection, touch flicking, wheel scrolling,
scrollbar interaction, and bounds. Physical mouse/touchpad feel still needs
testing on Fluff Linux.

Changes to FluffSetup are developed on a separate branch and submitted as a
draft pull request before being merged.

### Timezone selection

After system name, the timezone page uses KDE's installed timezone geometry and
offline map tiles with Qt Location's native `MapView`, `GeoJsonData` and
`MapPolygon` types. A small application-owned surface resolves each polygon's
timezone ID from the display model before rendering, including all pieces of
multi-polygons. It never reads a visual parent's timezone ID. The installed
KDE `GeoJsonDelegate` with its unsafe parent lookup is not instantiated, and no
system KDE files or package data are modified. Map clicks synchronize with the
localized search/list below; there is no duplicate map dropdown. The list
reserves enough room for at least five entries, shrinking the map as needed on
small displays. The list below its live search shows the selected geographic
region's timezones until a
query is entered. A selected timezone outside that region stays visible too.
Searching covers English and native city/country names and
IANA IDs across all installed choices. A region with exactly one timezone
auto-selects it. For multi-zone regions, an existing timezone in that region is
retained; otherwise selection is required instead of guessing a city. Explicit
or recovered timezone choices survive region and language changes.

Search text is explicitly dark on white, with red/white text selection,
independent of the desktop palette. Next, regular Enter and numpad Enter share
one confirmation path. An active search confirms the selected matching timezone
or its first result; no matches disable continuation without erasing the existing
timezone. Clearing the search preserves that selection. Pending input-method
composition or a rejected backend selection cannot advance the page.
The native map loads only on the page's first visible visit, then remains loaded
for rapid Next/Back navigation. Detached polygons retain their own IDs;
model removal, reload and destruction cannot use a missing visual parent to
select a timezone. Empty, unknown or shutdown-time map selections are ignored,
and Rust still validates any choice before saving/applying it. The map is
released with the page's owner; monitor selection, sizing and map-framing rules
are unchanged.

List, search and map choices synchronize the highlighted area. Initial,
recovered and manual choices fit all parts of the selected outline through Qt's
public Map API, including disjoint islands and dateline crossings. Framing uses
the actual viewport and the largest zoom that preserves the full border with
padding. The initial selection and camera are supplied before map creation;
framing reads the full geometry directly rather than waiting for rendered
delegates. Only viewport-intersecting outlines and the selected area are drawn.
Native camera commits settle before presenting the fitted map, avoiding an
initial world-view flash and zoom animation. Wide or portrait maps letterbox only when Qt's minimum zoom would crop
the area; padding reduces near the projection's edge. Outlines extending beyond
Web Mercator's polar coverage keep the existing coordinate/zoom fallback for
now. Missing geometry also retains that fallback rather than inventing an area.
The map retains its zoom-button behavior, with white faces, black symbols,
larger input targets and the installer's secondary-button hover/pressed colors.
Wheel/touchpad zoom, touchscreen pinch and panning remain native. Zoom is bounded
by KDE's bundled offline map tiles. No duplicate
application wheel handler is installed on the framework-managed map. The results
list uses the existing Kirigami handler and native touch flicking.

Timezone city/country names use installed ICU data. KDE's map controls use its
existing gettext catalog; missing translations fall back to English. All 96
language choices have translated page/search/fallback text, including Hebrew
`אזורי זמן`. The bundled KDE Breeze
[clock SVG](https://github.com/KDE/breeze-icons/blob/master/icons/preferences/32/preferences-system-time.svg)
uses the existing artwork license. If the map module is missing, the translated
search/list remains usable. macOS development previews show that fallback, not
a pretend map; actual-map tests and screenshots require the Linux runtime.

Only setup finalization runs the root helper's validated
`timedatectl set-timezone` command. The new user inherits the system timezone;
no `TZ` environment override is created. Browsing, recovery and preview do not
change system settings. NTP, network timezone detection, manual clock changes
and RTC policy are intentionally left unchanged for a later update.
The terminal fallback applies a valid recovered timezone choice; without one,
it leaves the existing system timezone unchanged.

The clock panel shows minute-only live time and the date beneath its icon once
a timezone is selected and online system synchronization is confirmed. Qt uses
that timezone's daylight-saving rules and the planned regional formatting locale
(neutral C fallback where unavailable), preserving localized 12/24-hour formats.
The display reevaluates every second and immediately after selection changes,
but never displays seconds.

Rust reads the existing system service, not a FluffSetup-owned internet clock.
On Fluff's systemd-timesyncd setup, the readout requires NetworkManager's full
connectivity result with enabled/available connectivity checks, timedated's NTP
and synchronized flags, and a valid, non-ignored recent timesyncd NTP sample.
Its successful-sync marker must be newer than the service's current poll interval
plus 30 seconds. Status is checked in a bounded background worker every five
seconds while the timezone page is visible; confirmation expires after at most
15 seconds if checks stop returning. After an observed offline/status failure or
server change, a new successful sample is required before the readout returns.
Offline, stale, unavailable and unverified alternate-daemon status fail closed:
hide time/date, keeping the static clock icon and manual map/list usable.

The NTP server comes from the installed service's configuration and is not
hardcoded by FluffSetup. The observer does not start NetworkManager/timesyncd,
enable or restart NTP, change servers, adjust the system clock, or change timezone.
Reading timedated's metadata may activate that metadata-only service on demand.
Confirmed status is a confidence check, not a guarantee of perfect accuracy or
instant network-change detection. Rust owns synchronization gating, conversion
and format selection;
`native/clock.h` contains only thin bindings to QLocale overloads absent from
the existing Rust Qt bindings. No new dependency or C++ application logic is used.

Safe previews default to an unconfirmed/hidden readout. Use
`--clock-sync=confirmed` for explicitly synthetic clock screenshots,
`--clock-sync=offline` for the hidden state, or `--clock-sync=system` on Linux to
read the installed service without changing settings. The preview renderer
accepts the same option; synthetic confirmed results do not prove a real NTP sync.

The real-map tests are separate from portable picker tests:
`qmltestrunner -input tests/linuxqml`. They need installed KDE map data/resources and
Qt Location. Synthetic button, wheel and touch tests do not establish physical
mouse/touchpad/touchscreen feel; that still needs device testing.
Use `tools/render_previews.py --pages 9 --cold-start` to audit automatic initial
framing without first exercising a second selection. Native map tests check
every installed outline within projection coverage, plus cold-start, resize,
landscape/compact/portrait, dateline and manual pan/zoom cases.
Lifecycle tests also detach real polygons, clear/reload the native geometry
model and interrupt repeated creation/destruction; every QML warning fails
these tests rather than being ignored.
Use `--timezone-search=London` to include typed search in safe previews, or
`--workers 4` to bound concurrent renders on a smaller test machine. Captures
wait a bounded interval for native framing to settle; unresolved framing is
still reported as an audit failure. The search-seeding argument is ignored in
normal setup.

## License

MIT, except the bundled KDE Breeze icons (LGPL-3.0-or-later;
see `assets/COPYING-ICONS`).
