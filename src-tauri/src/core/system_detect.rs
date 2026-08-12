//! Small environment probes shared across the app (audio quality auto-tuning,
//! Linux env var setup, etc.). Kept dependency-free and cheap so they can be
//! called at startup without measurable cost.

/// Detect VMs / containers / hypervised environments on Linux.
///
/// Returns the kind of virt detected (e.g. `"kvm"`, `"vmware"`, `"oracle"`,
/// `"microsoft"`, `"qemu"`, `"lxc"`, `"docker"`...), or `None` if running on
/// bare metal / unknown.
///
/// Strategy:
/// 1. Try `systemd-detect-virt` (reliable, available on all modern systemd distros).
/// 2. Fall back to the `hypervisor` CPU flag in `/proc/cpuinfo`.
#[cfg(target_os = "linux")]
pub fn detect_linux_virt() -> Option<String> {
    if let Ok(output) = std::process::Command::new("systemd-detect-virt").output() {
        let virt = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !virt.is_empty() && virt != "none" {
            return Some(virt);
        }
        if output.status.success() {
            // Returned "none" explicitly → bare metal.
            return None;
        }
    }
    // Fallback: parse /proc/cpuinfo.
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        let has_hypervisor = content
            .lines()
            .any(|line| line.starts_with("flags") && line.contains("hypervisor"));
        if has_hypervisor {
            return Some("unknown".to_string());
        }
    }
    None
}

/// Detect SteamOS (Steam Deck) via `/etc/os-release`.
///
/// The Steam Deck is a native machine (no hypervisor) but its Arch-based
/// stack breaks WebKitGTK's EGL display creation when running AppImages
/// built on Ubuntu (`Could not create default EGL display: EGL_BAD_PARAMETER`).
/// We treat it like a VM and default to software rendering there.
#[cfg(target_os = "linux")]
pub fn detect_steamos() -> bool {
    let Ok(content) = std::fs::read_to_string("/etc/os-release") else {
        return false;
    };
    content
        .lines()
        .any(|line| {
            line.strip_prefix("ID=")
                .map(|v| v.trim().trim_matches('"') == "steamos")
                .unwrap_or(false)
        })
}

/// Cross-platform "are we running inside a VM ?" check.
///
/// On Linux uses [`detect_linux_virt`]. On Windows/macOS we currently return
/// `false` since RustMusic users on those platforms typically have enough
/// dedicated resources, and the auto-detection methods are more fragile
/// (would need WMI on Windows, hard to make reliable).
pub fn is_virtualized() -> bool {
    #[cfg(target_os = "linux")]
    {
        return detect_linux_virt().is_some();
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Number of logical CPU cores available to the process.
pub fn logical_cpu_count() -> usize {
    num_cpus::get()
}

/// Best-effort detection of a "network" filesystem path (SMB, NFS, SSHFS,
/// WebDAV, mounted via GVFS on Linux or UNC paths on Windows).
///
/// Used to decide whether to pre-load the file to RAM before decoding :
/// streaming directly from a network mount can stall the decoder thread for
/// hundreds of milliseconds, causing audio underruns. Pre-loading isolates
/// playback from those stalls.
///
/// The heuristic is mostly path-based :
///   - GVFS Linux mounts : `/run/user/<uid>/gvfs/...` or anything with `gvfs`
///   - SMB / CIFS : `smb-share`, `smb://`, UNC (`\\server\...`)
///   - SFTP / SSHFS : `sftp:`
///   - WebDAV : `dav:`, `davs:`
///
/// On Windows it is **not enough**. A share mounted on a drive letter
/// (`S:\Music\...` for `\\NAS\music`) looks exactly like a local disk : no
/// prefix betrays it. Such a path was therefore streamed straight from the
/// network, which cost more than audio stalls — the file handle stayed open
/// for the whole track, and SMB refuses to replace a file that anything still
/// holds open, so editing its tags failed with "access denied". The drive type
/// is asked to the OS for that case.
pub fn is_network_path(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    let by_prefix = s.contains("/gvfs/")
        || s.contains("smb-share")
        || s.contains("smb://")
        || s.starts_with("\\\\") // UNC Windows
        || s.contains("sftp:")
        || s.contains("dav:");

    by_prefix || is_mapped_network_drive(path)
}

/// Is this path on a drive letter that actually points at a network share ?
///
/// Only Windows maps shares onto letters ; everywhere else this is always
/// false and the prefix heuristic above is the whole story.
#[cfg(not(target_os = "windows"))]
fn is_mapped_network_drive(_path: &std::path::Path) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn is_mapped_network_drive(path: &std::path::Path) -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    /// `GetDriveTypeW` return value for a remote (network) drive.
    const DRIVE_REMOTE: u32 = 4;

    // Declared by hand rather than pulling in the `windows` crate for a single
    // call : `kernel32` is already linked by the standard library.
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDriveTypeW(lp_root_path_name: *const u16) -> u32;
    }

    // The call wants a root ("S:\"), not a full path.
    let text = path.to_string_lossy();
    let mut chars = text.chars();
    let (Some(letter), Some(':')) = (chars.next(), chars.next()) else {
        return false;
    };
    if !letter.is_ascii_alphabetic() {
        return false;
    }

    let root: Vec<u16> = OsStr::new(&format!("{letter}:\\"))
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY : `root` is a NUL-terminated UTF-16 buffer that outlives the call,
    // and `GetDriveTypeW` only reads it.
    unsafe { GetDriveTypeW(root.as_ptr()) == DRIVE_REMOTE }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn recognises_network_prefixes() {
        for path in [
            r"\\NAS\music\album\piste.flac",
            "/run/user/1000/gvfs/smb-share:server=nas/piste.flac",
            "smb://nas/music/piste.flac",
            "sftp:host/piste.flac",
        ] {
            assert!(is_network_path(Path::new(path)), "non détecté : {path}");
        }
    }

    #[test]
    fn a_plain_local_path_is_not_network() {
        // Sur un poste sans lecteur mappé, ces chemins doivent rester locaux —
        // sinon tout serait préchargé en RAM sans raison.
        assert!(!is_network_path(Path::new("/home/david/musique/piste.flac")));
        assert!(!is_network_path(Path::new("piste.flac")));
    }

    #[test]
    fn a_drive_letter_alone_does_not_crash_the_lookup() {
        // Le chemin le plus court qui atteigne la branche Windows.
        let _ = is_network_path(Path::new("S:"));
        let _ = is_network_path(Path::new("1:\\pas-une-lettre"));
    }
}
