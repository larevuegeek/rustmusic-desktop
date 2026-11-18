//! Public lifecycle entry point of the DLNA server.
//!
//! Spawns concurrent tasks once `start()` is called :
//!   1. axum HTTP server (description + SOAP + media)
//!   2. (later) SSDP advertiser
//!   3. (later) SSDP listener
//!
//! All tasks watch a shared `tokio::sync::broadcast::Receiver<()>` so a
//! single `stop()` call cleanly tears everything down.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::core::dlna_server::config::DlnaConfig;
use crate::core::dlna_server::error::DlnaError;
use crate::core::dlna_server::http::router::{build_router, AppState};
use crate::core::dlna_server::library::provider::LibraryProvider;
use crate::core::dlna_server::net::detect_local_ipv4;
use crate::core::dlna_server::ssdp::{advertiser, listener};

pub struct DlnaServer {
    config: Arc<DlnaConfig>,
    provider: Arc<dyn LibraryProvider>,
    /// `Some` when running, `None` when stopped.
    shutdown_tx: Option<broadcast::Sender<()>>,
    handles: Vec<JoinHandle<()>>,
}

impl DlnaServer {
    pub fn new(config: DlnaConfig, provider: Arc<dyn LibraryProvider>) -> Self {
        Self {
            config: Arc::new(config),
            provider,
            shutdown_tx: None,
            handles: Vec::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.shutdown_tx.is_some()
    }

    /// Start the subsystems. Returns once tasks are spawned (HTTP listener bound).
    pub async fn start(&mut self) -> Result<(), DlnaError> {
        if self.is_running() {
            return Err(DlnaError::AlreadyRunning);
        }

        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        // Detect once at start — embedded in DIDL <res> URLs and SSDP LOCATION.
        let local_ip = detect_local_ipv4();

        // ─── HTTP server (axum) ───
        let app_state = AppState {
            config: self.config.clone(),
            provider: self.provider.clone(),
            local_ip,
        };
        let router = build_router(app_state);

        let addr = SocketAddr::new(self.config.bind_address, self.config.port);
        let listener = TcpListener::bind(addr).await.map_err(DlnaError::Io)?;

        log::info!(
            "DLNA server listening on http://{}:{} (LAN IP {})",
            self.config.bind_address, self.config.port, local_ip
        );

        let mut http_shutdown = shutdown_tx.subscribe();
        let http_handle = tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = http_shutdown.recv().await;
                })
                .await;
            log::debug!("DLNA HTTP server stopped");
        });
        self.handles.push(http_handle);

        // ─── SSDP discovery (advertiser + listener) ───
        let adv_config = self.config.clone();
        let adv_shutdown = shutdown_tx.subscribe();
        let adv_handle = tokio::spawn(async move {
            if let Err(e) = advertiser::run_advertiser(adv_config, local_ip, adv_shutdown).await {
                log::error!("SSDP advertiser error: {}", e);
            }
        });
        self.handles.push(adv_handle);

        let lst_config = self.config.clone();
        let lst_shutdown = shutdown_tx.subscribe();
        let lst_handle = tokio::spawn(async move {
            if let Err(e) = listener::run_listener(lst_config, local_ip, lst_shutdown).await {
                log::error!("SSDP listener error: {}", e);
            }
        });
        self.handles.push(lst_handle);

        self.shutdown_tx = Some(shutdown_tx);
        Ok(())
    }

    /// Trigger shutdown signal and await the tasks.
    pub async fn stop(&mut self) -> Result<(), DlnaError> {
        let tx = match self.shutdown_tx.take() {
            Some(tx) => tx,
            None => return Err(DlnaError::NotRunning),
        };
        // Notify all subsystems
        let _ = tx.send(());
        // Wait for tasks to finish (with a generous timeout via just awaiting)
        for handle in self.handles.drain(..) {
            let _ = handle.await;
        }
        log::info!("DLNA server stopped");
        Ok(())
    }
}
