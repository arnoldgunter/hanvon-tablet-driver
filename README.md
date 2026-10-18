# Hanvon Tablet Driver

Userspace Linux driver for the Hanvon AM0605 tablet (`0b57:8503`).

## Requirements

- Linux with systemd user services
- A direct USB 2.0 connection or USB 2.0 hub
- Membership in the `input` group

The tablet has known USB 3.x compatibility problems. USB 2.0 is recommended.

## Install a release

Download the `x86_64-linux.tar.gz` archive from the GitHub Releases page,
extract it, and run:

```bash
./install.sh
```

Run the installer as your normal user, not with `sudo`. It uses `sudo`
itself only for installing the udev rule; the systemd service must be
installed in your user account.

The installer:

- installs the binary into `~/.local/bin`
- installs and enables a systemd user service
- installs the required udev rules using `sudo`
- reloads udev and starts the driver in the background

Reconnect the tablet after installation if it was already plugged in.

View logs:

```bash
journalctl --user -u hanvon-tablet-driver.service -f
```

Stop or disable the service:

```bash
systemctl --user disable --now hanvon-tablet-driver.service
```

## Build locally

```bash
cargo build --release
./target/release/hanvon-tablet-driver
```

## Create a release

Push a version tag. GitHub Actions builds the binary and publishes an archive:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow currently publishes an x86_64 Linux binary. Other CPU
architectures need separate cross-compilation targets and release artifacts.
