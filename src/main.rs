// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Allocateur de tout le programme (voir `Cargo.toml`).
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    // Le runtime tokio porte le moteur (base, réseau, scans) ; la boucle de fenêtres
    // tourne sur le thread principal, hors du runtime.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime tokio");
    rustmusic_lib::app::platform::runtime::install(runtime.handle().clone());

    let result = rustmusic_lib::run();

    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
    if let Err(error) = result {
        eprintln!("RustMusic : {error}");
        std::process::exit(1);
    }
}
