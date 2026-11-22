//! Traces des plantages (Linux, macOS).
//!
//! Une application lancée depuis le bureau n'a pas de sortie d'erreur visible : les
//! messages des bibliothèques (karbon-ui, Vulkan, Mesa) et la cause d'un plantage
//! disparaissent. Deux filets :
//!
//! - la sortie d'erreur part dans `logs/stderr.log`, quand elle ne va pas à un terminal ;
//! - un plantage par signal (SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGABRT) écrit le signal,
//!   le fil d'exécution et la pile d'appels dans ce fichier et dans le journal, puis
//!   laisse le gestionnaire précédent finir (celui de Rust signale un débordement de pile).

use std::fs::File;
use std::path::Path;

/// Redirige la sortie d'erreur et installe les gestionnaires de plantage. `log` est le
/// journal ouvert, où la trace est recopiée.
pub fn install(log_dir: &Path, log: &File) {
    #[cfg(unix)]
    unix::install(log_dir, log);
    #[cfg(not(unix))]
    let _ = (log_dir, log);
}

#[cfg(unix)]
mod unix {
    use std::ffi::{c_int, c_void};
    use std::fs::File;
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::path::Path;
    use std::sync::atomic::{AtomicI32, Ordering};

    unsafe extern "C" {
        fn backtrace(buffer: *mut *mut c_void, size: c_int) -> c_int;
        fn backtrace_symbols_fd(buffer: *const *mut c_void, size: c_int, fd: c_int);
    }

    /// Au-delà, le fichier de la sortie d'erreur est archivé (un seul gardé).
    const STDERR_MAX_BYTES: u64 = 1024 * 1024;

    const SIGNALS: [c_int; 5] = [libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGFPE, libc::SIGABRT];

    /// Copie du journal propre au gestionnaire (descripteur dupliqué, jamais refermé).
    static LOG_FD: AtomicI32 = AtomicI32::new(-1);

    /// Gestionnaires en place avant les nôtres, rendus au signal après la trace.
    static mut PREVIOUS: [std::mem::MaybeUninit<libc::sigaction>; 5] = [const { std::mem::MaybeUninit::uninit() }; 5];

    pub fn install(log_dir: &Path, log: &File) {
        capture_stderr(log_dir);
        let fd = unsafe { libc::dup(log.as_raw_fd()) };
        LOG_FD.store(fd, Ordering::Relaxed);
        // Premier appel hors plantage : glibc y charge `libgcc_s`, ce qu'un gestionnaire de signal ne peut pas faire.
        let mut frames = [std::ptr::null_mut(); 4];
        unsafe { backtrace(frames.as_mut_ptr(), frames.len() as c_int) };
        for (i, &sig) in SIGNALS.iter().enumerate() {
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = on_crash as *const () as usize;
                // Pile de secours : un débordement de pile doit encore pouvoir être tracé.
                action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
                libc::sigemptyset(&mut action.sa_mask);
                let previous = (*std::ptr::addr_of_mut!(PREVIOUS))[i].as_mut_ptr();
                libc::sigaction(sig, &action, previous);
            }
        }
    }

    /// Sortie d'erreur vers `stderr.log` (en ajout, une ligne d'en-tête par lancement),
    /// sauf quand elle va déjà à un terminal.
    fn capture_stderr(log_dir: &Path) {
        if unsafe { libc::isatty(libc::STDERR_FILENO) } == 1 {
            return;
        }
        let path = log_dir.join("stderr.log");
        if std::fs::metadata(&path).is_ok_and(|m| m.len() > STDERR_MAX_BYTES) {
            let _ = std::fs::rename(&path, log_dir.join("stderr.log.1"));
        }
        let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) else {
            return;
        };
        let _ = writeln!(
            file,
            "\n=== RustMusic {} — {} ===",
            env!("CARGO_PKG_VERSION"),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        if unsafe { libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO) } >= 0 {
            log::info!("Sortie d'erreur → {}", path.display());
        }
    }

    fn signal_name(sig: c_int) -> &'static [u8] {
        match sig {
            libc::SIGSEGV => b"SIGSEGV (acces memoire invalide)",
            libc::SIGBUS => b"SIGBUS (erreur de bus)",
            libc::SIGILL => b"SIGILL (instruction illegale)",
            libc::SIGFPE => b"SIGFPE (erreur arithmetique)",
            libc::SIGABRT => b"SIGABRT (abandon)",
            _ => b"inconnu",
        }
    }

    fn write_all(fd: c_int, bytes: &[u8]) {
        if fd >= 0 {
            unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len()) };
        }
    }

    /// Trace du plantage, en appels sûrs dans un gestionnaire de signal (`write`,
    /// `backtrace_symbols_fd`). Une faute rend la main au gestionnaire précédent, qui la
    /// voit à nouveau quand l'instruction fautive se rejoue.
    extern "C" fn on_crash(sig: c_int, info: *mut libc::siginfo_t, _context: *mut c_void) {
        let mut thread = [0u8; 64];
        let named = unsafe { libc::pthread_getname_np(libc::pthread_self(), thread.as_mut_ptr().cast(), thread.len()) } == 0;
        let thread_len = if named { thread.iter().position(|&b| b == 0).unwrap_or(0) } else { 0 };
        let mut frames = [std::ptr::null_mut(); 64];
        let count = unsafe { backtrace(frames.as_mut_ptr(), frames.len() as c_int) };
        for fd in [libc::STDERR_FILENO, LOG_FD.load(Ordering::Relaxed)] {
            write_all(fd, b"\n[PLANTAGE] RustMusic ");
            write_all(fd, env!("CARGO_PKG_VERSION").as_bytes());
            write_all(fd, b" : signal ");
            write_all(fd, signal_name(sig));
            write_all(fd, b", fil \"");
            write_all(fd, &thread[..thread_len]);
            write_all(fd, b"\"\n");
            if fd >= 0 {
                unsafe { backtrace_symbols_fd(frames.as_ptr(), count, fd) };
            }
            write_all(fd, b"[PLANTAGE] fin de la trace\n");
        }
        // Signal envoyé (`kill`, `abort`) : rien ne le rejouera, il est relancé tel quel.
        let sent = info.is_null() || unsafe { (*info).si_code } <= 0;
        unsafe {
            if sent {
                libc::signal(sig, libc::SIG_DFL);
                libc::raise(sig);
            } else if let Some(i) = SIGNALS.iter().position(|&s| s == sig) {
                let previous = (*std::ptr::addr_of!(PREVIOUS))[i].as_ptr();
                libc::sigaction(sig, previous, std::ptr::null_mut());
            }
        }
    }
}
