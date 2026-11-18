//! Pont entre le moteur (tokio, commandes asynchrones, événements `emit`) et le thread
//! de l'interface, par la boîte aux lettres de l'application (`karbon_ui::task::Inbox`) :
//! le résultat d'une commande arrive comme une fermeture appliquée à l'application, un
//! événement du moteur comme un appel à `on_engine`, et une horloge bat pendant la
//! lecture. Rien n'attend en boucle : chaque dépôt réveille la fenêtre.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;

use karbon_ui::task::{Inbox, InboxSender};

use crate::app::music_app::MusicApp;
use crate::app::platform::runtime;
use crate::app::{AppHandle, Event};

/// Période de l'horloge de lecture (position du morceau).
const TICK: Duration = Duration::from_millis(250);

/// La poignée du moteur et l'accès à la boîte aux lettres de l'application.
#[derive(Clone)]
pub struct Backend {
    pub app: AppHandle,
    inbox: InboxSender<MusicApp>,
    /// L'horloge ne bat que pendant la lecture.
    playing: Arc<AtomicBool>,
}

impl Backend {
    /// Relie la poignée du moteur à la boîte aux lettres de l'application et lance le
    /// relais des événements du moteur et l'horloge de lecture.
    pub fn new(app: AppHandle, events: Receiver<Event>, inbox: &Inbox<MusicApp>) -> Self {
        let playing = Arc::new(AtomicBool::new(false));

        // Les `emit` du moteur viennent de n'importe quel thread : un relais vers la
        // boîte, sur un thread dédié qui n'a rien d'autre à faire.
        let relay = inbox.sender();
        let _ = std::thread::Builder::new()
            .name("ui-relais".into())
            .spawn(move || {
                for event in events {
                    if !relay.post(move |me| {
                        let changed = me.on_engine(event);
                        me.tray_sync();
                        changed
                    }) {
                        break;
                    }
                }
            });

        // L'horloge de lecture, sur le runtime.
        let tick = inbox.sender();
        let tick_playing = playing.clone();
        runtime::spawn(async move {
            let mut interval = tokio::time::interval(TICK);
            loop {
                interval.tick().await;
                if tick.is_closed() {
                    break;
                }
                if tick_playing.load(Ordering::Relaxed) {
                    tick.post(|me| {
                        let changed = me.tick();
                        me.tray_sync();
                        changed
                    });
                }
            }
        });

        Self {
            app,
            inbox: inbox.sender(),
            playing,
        }
    }

    /// Lance une commande sur le runtime ; son résultat est remis à `then` sur le thread
    /// de l'interface, qui décide si la vue doit être reconstruite.
    pub fn call<T, F, Then>(&self, future: F, then: Then)
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        Then: FnOnce(&mut MusicApp, T) -> bool + Send + 'static,
    {
        let inbox = self.inbox.clone();
        runtime::spawn(async move {
            let value = future.await;
            inbox.post(move |app| {
                let changed = then(app, value);
                app.tray_sync();
                changed
            });
        });
    }

    /// Applique `apply` à l'application sur son thread, depuis n'importe quel thread
    /// (avancement d'un téléchargement) ; `true` reconstruit la vue.
    pub fn post(&self, apply: impl FnOnce(&mut MusicApp) -> bool + Send + 'static) {
        self.inbox.post(apply);
    }

    /// Lance une commande dont le résultat n'intéresse pas l'interface.
    pub fn fire<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        runtime::spawn(future);
    }

    /// Fait battre (ou tait) l'horloge de lecture.
    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
    }
}
