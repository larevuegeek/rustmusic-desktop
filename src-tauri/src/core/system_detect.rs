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
/// The heuristic is path-based ; we don't query the actual mount table
/// (would require platform-specific code, more dependencies). Covers the
/// common cases :
///   - GVFS Linux mounts : `/run/user/<uid>/gvfs/...` or anything with `gvfs`
///   - SMB / CIFS : `smb-share`, `smb://`, UNC (`\\server\...`)
///   - SFTP / SSHFS : `sftp:`
///   - WebDAV : `dav:`, `davs:`
pub fn is_network_path(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.contains("/gvfs/")
        || s.contains("smb-share")
        || s.contains("smb://")
        || s.starts_with("\\\\") // UNC Windows
        || s.contains("sftp:")
        || s.contains("dav:")
}
