//! Set haptic feedback intensity of Goodix pressure touchpads on Linux.
//!
//! The touchpad exposes the standard HID Haptics "Intensity" usage as feature
//! report 0x09. Windows sets it from Settings -> Touchpad -> Touchpad feedback.
//! The firmware keeps the last value across reboots but never reports it back,
//! and Windows writes its own value on every boot, so the saved value is
//! re-applied on every Linux boot by touchpad-haptic.service.

use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, ExitCode};
use std::thread::sleep;
use std::time::Duration;

const USAGE: &str = "\
usage:
  touchpad-haptic                 show the detected touchpad and saved level
  touchpad-haptic set N           apply level N (0 = off, 100 = strongest) and save it
  touchpad-haptic try N           apply level N without saving
  touchpad-haptic off             same as \"set 0\"
  touchpad-haptic apply           apply the saved level (used by the systemd service)

Commands that change the level re-run themselves through sudo.";

const CONFIG: &str = "/etc/touchpad-haptic.conf";
const REPORT_ID: u8 = 0x09;

struct Model {
    hid_id: &'static str,
    name: &'static str,
    /// Firmware treats 0 as the strongest feedback and 100 as off.
    inverted: bool,
}

/// Only list devices that were tested.
const MODELS: &[Model] = &[Model {
    hid_id: "0018:000027C6:000001E9",
    name: "Goodix GXTP5100 (27C6:01E9)",
    inverted: true,
}];

/// Haptics page / Simple Haptic Controller / Intensity in report 0x09, range 0-100.
/// Refuse to write if the device does not declare exactly this.
const HAPTIC_INTENSITY_RDESC: &[u8] = &[
    0x05, 0x0e, 0x09, 0x01, 0xa1, 0x02, 0x09, 0x23, 0x85, 0x09, 0x15, 0x00, 0x25, 0x64, 0x75, 0x08,
    0x95, 0x01, 0xb1, 0x02,
];

/// _IOC(_IOC_READ|_IOC_WRITE, 'H', 0x06, len)
const fn hidiocsfeature(len: usize) -> u32 {
    (3 << 30) | ((len as u32) << 16) | ((b'H' as u32) << 8) | 0x06
}

struct Device {
    path: String,
    model: &'static Model,
}

fn find_device() -> Result<Option<Device>, String> {
    let Ok(entries) = fs::read_dir("/sys/class/hidraw") else {
        return Ok(None);
    };
    let mut nodes: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    nodes.sort();
    for node in nodes {
        let Ok(uevent) = fs::read_to_string(node.join("device/uevent")) else {
            continue;
        };
        let Some(hid_id) = uevent.lines().find_map(|l| l.strip_prefix("HID_ID=")) else {
            continue;
        };
        let Some(model) = MODELS.iter().find(|m| m.hid_id == hid_id) else {
            continue;
        };
        let Ok(rdesc) = fs::read(node.join("device/report_descriptor")) else {
            continue;
        };
        if !contains(&rdesc, HAPTIC_INTENSITY_RDESC) {
            return Err(format!(
                "{}: touchpad found, but it does not declare haptic intensity in report 0x09; not touching it",
                node.display()
            ));
        }
        let name = node.file_name().unwrap().to_string_lossy();
        return Ok(Some(Device {
            path: format!("/dev/{name}"),
            model,
        }));
    }
    Ok(None)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn raw_value(level: u8, inverted: bool) -> u8 {
    if inverted { 100 - level } else { level }
}

fn send(level: u8) -> Result<(), String> {
    // Retries cover early boot, when the touchpad may not be bound yet.
    let mut err = String::new();
    for attempt in 0..10 {
        if attempt > 0 {
            sleep(Duration::from_millis(500));
        }
        let Some(dev) = find_device()? else {
            err = "no supported touchpad found".into();
            continue;
        };
        let raw = raw_value(level, dev.model.inverted);
        match write_feature(&dev.path, &[REPORT_ID, raw]) {
            Ok(()) => {
                println!(
                    "{} at {}: haptic level set to {level} (raw {raw})",
                    dev.model.name, dev.path
                );
                return Ok(());
            }
            Err(e) => err = format!("{}: {e}", dev.path),
        }
    }
    Err(err)
}

fn write_feature(path: &str, report: &[u8]) -> std::io::Result<()> {
    let file = OpenOptions::new().read(true).write(true).open(path)?;
    // SAFETY: HIDIOCSFEATURE reads report.len() bytes from a valid buffer of that length.
    let ret = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            hidiocsfeature(report.len()) as libc::Ioctl,
            report.as_ptr(),
        )
    };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn parse_level(s: &str) -> Result<u8, String> {
    match s.trim().parse::<u8>() {
        Ok(v) if v <= 100 => Ok(v),
        _ => Err(format!("invalid level {s:?}, expected an integer 0-100")),
    }
}

fn read_config() -> Result<Option<u8>, String> {
    match fs::read_to_string(CONFIG) {
        Ok(s) => parse_level(&s)
            .map(Some)
            .map_err(|e| format!("{CONFIG}: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{CONFIG}: {e}")),
    }
}

fn ensure_root() -> Result<(), String> {
    if unsafe { libc::geteuid() } == 0 {
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let err = Command::new("sudo")
        .arg(exe)
        .args(std::env::args_os().skip(1))
        .exec();
    Err(format!("failed to run sudo: {err}"))
}

fn run(args: &[String]) -> Result<(), String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["status"] => {
            match find_device()? {
                Some(d) => println!("touchpad: {} at {}", d.model.name, d.path),
                None => println!("touchpad: no supported touchpad found"),
            }
            match read_config()? {
                Some(v) => println!("saved level: {v}"),
                None => println!("saved level: none (left as set by firmware / last OS)"),
            }
        }
        [cmd @ ("set" | "try"), n] => {
            let level = parse_level(n)?;
            ensure_root()?;
            send(level)?;
            if *cmd == "set" {
                fs::write(CONFIG, format!("{level}\n")).map_err(|e| format!("{CONFIG}: {e}"))?;
                println!("saved to {CONFIG}");
            }
        }
        ["off"] => return run(&["set".into(), "0".into()]),
        ["apply"] => {
            let Some(level) = read_config()? else {
                println!("{CONFIG} not found, nothing to apply");
                return Ok(());
            };
            ensure_root()?;
            send(level)?;
        }
        ["-h" | "--help" | "help"] => println!("{USAGE}"),
        ["-V" | "--version"] => println!("touchpad-haptic {}", env!("CARGO_PKG_VERSION")),
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ioctl_number_matches_kernel() {
        // HIDIOCSFEATURE(2) as computed by <linux/hidraw.h> on x86_64.
        assert_eq!(hidiocsfeature(2), 0xC002_4806);
    }

    #[test]
    fn levels() {
        assert_eq!(parse_level("0"), Ok(0));
        assert_eq!(parse_level("100\n"), Ok(100));
        assert!(parse_level("101").is_err());
        assert!(parse_level("-1").is_err());
        assert!(parse_level("x").is_err());
    }

    #[test]
    fn inversion() {
        assert_eq!(raw_value(0, true), 100);
        assert_eq!(raw_value(100, true), 0);
        assert_eq!(raw_value(30, false), 30);
    }

    #[test]
    fn descriptor_match() {
        let mut rdesc = vec![0xc0, 0x05, 0x0d];
        rdesc.extend_from_slice(HAPTIC_INTENSITY_RDESC);
        assert!(contains(&rdesc, HAPTIC_INTENSITY_RDESC));
        assert!(!contains(&rdesc[..10], HAPTIC_INTENSITY_RDESC));
    }
}
