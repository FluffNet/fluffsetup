# fluffsetup

FluffSetup is the first-boot account and system-name setup program for Fluff
Linux. It is temporarily CLI-based while the full graphical setup experience
is developed.

The program is written in Rust using only the standard library. It has no
third-party Cargo dependencies. When started by the FluffSetup Wayland
session, the binary opens a separate Konsole window and provides the hostname,
user-name, and password flow adapted from FluffInstall 0.9.

## Build requirements

- Rust and Cargo
- Konsole at runtime

On Fluff Linux or Arch Linux, install the build tools with:

```bash
sudo pacman -S --needed rust
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

For an archiso profile whose `airootfs` directory is in
`/home/shy/flufflinux`, stage the binary with:

```bash
sudo install -Dm755 target/release/fluffsetup \
  /home/shy/flufflinux/airootfs/usr/lib/fluffinstall/fluffsetup/fluffsetup
```

During installation, FluffInstall copies that binary into the installed system
as `/usr/bin/fluffsetup` with mode `0755`. The `fluffsetup-session` launcher
should execute `/usr/bin/fluffsetup`; FluffSetup then opens its CLI in Konsole.

## Current behavior

- offers a randomized `FL-AA0000`-style hostname when none is entered
- validates custom hostnames using the FluffInstall 0.9 rules
- validates a permanent user name and prevents use of the temporary
  `fluffsetup` account name
- creates the permanent user with Zsh and membership in `uucp`, `wheel`, `kvm`,
  and `libvirt`
- sets the permanent user's password

The installed system provides temporary passwordless sudo access to the
`fluffsetup` account. A later FluffSetup revision will remove that temporary
permission after first-boot setup is complete.

## Development workflow

The initial import is committed directly to `main`. Future changes to
FluffSetup must be developed on a separate branch and submitted as a draft pull
request before being merged.

## License

MIT
