# goodix-touchpad-haptic

Control the haptic feedback strength of Goodix pressure touchpads ("forcepads") on Linux.

On Windows the level is set in **Settings → Bluetooth & devices → Touchpad → Touchpad feedback → Intensity**.
Linux has no setting for it, so the touchpad stays at whatever level was last written to it — usually by Windows.
This project adds:

- `touchpad-haptic` — a CLI to change the level (0 = off, 100 = strongest)
- `touchpad-haptic.service` — a systemd unit that re-applies the saved level on every boot, before the login screen
- a `.run` installer

## Supported hardware

| Touchpad | USB/HID ID | Tested on |
|---|---|---|
| Goodix GXTP5100 (GT7868Q) | `27C6:01E9` | Lenovo ThinkBook 14 G7+ IAH (21TK) |

Check yours with `grep -h HID_ID /sys/class/hidraw/*/device/uevent`.
Other Goodix pressure pads (`27C6:01E7`, `01E8`, `01EA`, `01EB`, `0F9x`, …) probably work the same way, but are not enabled until someone tests them — see [Adding a device](#adding-a-device).

## Install

Download `touchpad-haptic-<version>.run` from the [releases](../../releases) page, then:

```sh
sh touchpad-haptic-*.run
touchpad-haptic set 50
```

This installs `/usr/local/bin/touchpad-haptic` and enables `touchpad-haptic.service`.
The CLI is a single static binary (x86_64, no runtime dependencies); only systemd and `sudo` are expected on the system.

### From source

```sh
cargo build --release
./install.sh
```

## Usage

```
touchpad-haptic                 show the detected touchpad and saved level
touchpad-haptic set N           apply level N (0 = off, 100 = strongest) and save it
touchpad-haptic try N           apply level N without saving
touchpad-haptic off             same as "set 0"
touchpad-haptic apply           apply the saved level (used by the systemd service)
```

Changing the level needs root; the CLI re-runs itself through `sudo`. The saved level lives in `/etc/touchpad-haptic.conf`.

## Uninstall

```sh
sh touchpad-haptic-*.run --uninstall   # keep /etc/touchpad-haptic.conf
sh touchpad-haptic-*.run --purge       # remove it too
```

## How it works

The touchpad declares the standard HID *Haptics → Simple Haptic Controller → Intensity* usage as feature report `0x09` with a range of 0–100.
The CLI finds the touchpad's `/dev/hidrawN` node by its HID ID, checks that its report descriptor really contains that usage, and sends `[0x09, value]` with `HIDIOCSFEATURE`.
Nothing else is written to the device — no vendor reports or commands.

Firmware behaviour observed on the GXTP5100:

- **The value is inverted**: raw 0 is the strongest feedback, raw 100 is off. The CLI hides this.
- **The value persists** across suspend, reboot and power-off.
- **It cannot be read back**: the firmware answers GET_REPORT only for reports `0x02` and `0x06`.
- **Windows overwrites it on every boot** with its own slider value — hence the boot-time service for dual-boot systems.

There is no click-force (press threshold) setting: the firmware decides when a press is a click, and neither the HID descriptor nor Windows exposes it. Windows' *Touchpad sensitivity* option is host-side tap/palm rejection, comparable to libinput's disable-while-typing and palm detection.

## Adding a device

1. Find your touchpad's `HID_ID` in `/sys/class/hidraw/hidraw*/device/uevent`.
2. Add it to `MODELS` in `src/main.rs` with `inverted: false`, build, run `touchpad-haptic try 0` and `try 100`, and see which one disables feedback.
3. Open a pull request with the ID, laptop model and the `inverted` value that works.

The CLI refuses to write to a device whose descriptor does not declare haptic intensity in report `0x09`.

## Building the installer

```sh
rustup target add x86_64-unknown-linux-musl
./build-run.sh   # writes dist/touchpad-haptic-<version>.run
```

Pushing a `v*` tag that matches the version in `Cargo.toml` builds the `.run` in GitHub Actions and publishes it as a release.

## License

MIT
