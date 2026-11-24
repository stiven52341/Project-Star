//! Showing a file or folder in the system's file manager, through the
//! programs every desktop system already has.

use std::path::Path;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::process::Command;

use star_core::{EngineError, EngineResult};

/// Opens the system's file manager at the folder that holds `path`, with
/// `path` selected where the system allows it: Explorer on Windows, the
/// Finder on macOS, and on Linux the file manager that answers the
/// freedesktop `FileManager1` D-Bus interface, or else whatever
/// `xdg-open` opens the folder with.
///
/// It waits for the system's program to answer, so the editor calls it
/// from a thread of its own.
///
/// # Errors
///
/// Returns [`EngineError::Platform`] if the system's program cannot be
/// started, or on systems without a file manager to show (Android, iOS).
pub fn show_in_file_explorer(path: &Path) -> EngineResult<()> {
    reveal(path).map_err(|error| {
        EngineError::Platform(format!(
            "could not show \"{}\" in the file manager: {error}",
            path.display()
        ))
    })
}

#[cfg(target_os = "windows")]
fn reveal(path: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    // Explorer reads `/select,"path"` as one argument, without the quotes
    // Rust would put around all of it; it also answers 1 when it worked,
    // so its exit code says nothing.
    Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .status()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn reveal(path: &Path) -> std::io::Result<()> {
    Command::new("open").arg("-R").arg(path).status()?;
    Ok(())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn reveal(_path: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "this system has no file manager to show files in",
    ))
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
)))]
fn reveal(path: &Path) -> std::io::Result<()> {
    let selected = Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
        ])
        .arg(format!("array:string:{}", file_uri(path)))
        .arg("string:")
        .output()
        .is_ok_and(|output| output.status.success());
    if selected {
        return Ok(());
    }
    let folder = path.parent().unwrap_or(path);
    Command::new("xdg-open").arg(folder).status()?;
    Ok(())
}

/// The `file://` URI of `path`, with every byte that is not a letter, a
/// digit, `-._~` or `/` written as `%XX`.
#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
)))]
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &byte in path.to_string_lossy().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            uri.push('%');
            uri.push(hex_digit(byte >> 4));
            uri.push(hex_digit(byte & 0x0F));
        }
    }
    uri
}

/// The uppercase hexadecimal digit of `value`, from 0 to 15.
#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
)))]
fn hex_digit(value: u8) -> char {
    char::from_digit(u32::from(value), 16).map_or('0', |digit| digit.to_ascii_uppercase())
}

#[cfg(all(
    test,
    not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "android",
        target_os = "ios"
    ))
))]
mod tests {
    use super::*;

    #[test]
    fn uris_escape_spaces_and_letters_outside_ascii() {
        assert_eq!(
            file_uri(Path::new("/home/ana/My game/niño.png")),
            "file:///home/ana/My%20game/ni%C3%B1o.png"
        );
    }
}
