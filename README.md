# fluffsetup

FluffSetup is the first-boot account and system-name setup program for Fluff
Linux. Version 1.0 provides a focused Qt-based graphical setup experience with
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
- ICU, whose system `uconv` tool provides readable international-name
  transliteration without adding a Rust crate

On Fluff Linux or Arch Linux, install the build tools with:

```bash
sudo pacman -S --needed rust gcc pkgconf qt6-base qt6-declarative icu
```

Build the release executable from the repository root:

```bash
cargo build --release
```

Run its validation tests with:

```bash
cargo test
```

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
- provides graphical welcome, system-name, account, setup-progress, and
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
- atomically saves the current page, system name, display name, and reserved
  internal account name so an interrupted first boot resumes where it stopped;
  passwords are never written to the recovery file
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
password, and account settings have all been applied successfully, but before
the completion summary becomes visible. Closing or interrupting setup before
that point leaves the temporary setup environment available for another attempt.
The root cleanup is a direct `sudo` invocation of FluffSetup's built-in cleanup
mode and does not install or schedule a systemd service. The temporary sudo rule
is removed by that cleanup.

## Development workflow

Changes to FluffSetup are developed on a separate branch and submitted as a
draft pull request before being merged.

## License

MIT
