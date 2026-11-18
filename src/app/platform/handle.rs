//! Poignée d'application : remplace `tauri::AppHandle`. Elle porte l'état géré (comme
//! `manage`/`state` de Tauri) et un canal d'événements vers l'interface : `emit` y
//! dépose un événement nommé avec sa charge JSON, exactement ce que recevait le
//! frontend par `listen`. L'interface vide ce canal sur son thread.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use serde::Serialize;

/// Ce que le moteur demande à la fenêtre principale.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowRequest {
    /// Afficher et ramener au premier plan (icône de la zone de notification, seconde instance).
    Show,
    /// Taille fixe du mini-lecteur (`set_compact_window_size`) ; `None` rend la liberté.
    CompactSize(Option<i32>, Option<i32>),
}

/// Un événement remonté à l'interface.
#[derive(Debug, Clone)]
pub enum Event {
    /// `app.emit(name, payload)` : l'événement nommé que le frontend écoutait.
    Emit {
        name: String,
        payload: serde_json::Value,
    },
    /// Une demande à la fenêtre.
    Window(WindowRequest),
    /// `app.exit(code)`.
    Exit(i32),
    /// `app.restart()`.
    Restart,
}

struct Inner {
    states: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    sender: Sender<Event>,
}

/// La poignée, clonable et `Send`, passée aux commandes et aux tâches de fond.
#[derive(Clone)]
pub struct AppHandle {
    inner: Arc<Inner>,
}

impl AppHandle {
    /// Une poignée et le récepteur des événements qu'elle émet.
    pub fn new() -> (Self, Receiver<Event>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let handle = Self {
            inner: Arc::new(Inner {
                states: Mutex::new(HashMap::new()),
                sender,
            }),
        };
        (handle, receiver)
    }

    /// Enregistre une valeur partagée, retrouvée ensuite par son type (`state`).
    pub fn manage<T: Send + Sync + 'static>(&self, value: T) {
        self.inner
            .states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(TypeId::of::<T>(), Arc::new(value));
    }

    /// La valeur gérée de ce type, s'il y en a une.
    pub fn try_state<T: Send + Sync + 'static>(&self) -> Option<State<'_, T>> {
        let states = self.inner.states.lock().unwrap_or_else(|e| e.into_inner());
        let value = states.get(&TypeId::of::<T>())?.clone();
        Arc::downcast::<T>(value)
            .ok()
            .map(|value| State(value, PhantomData))
    }

    /// La valeur gérée de ce type. Panique si elle n'a pas été enregistrée : c'est une
    /// erreur de programmation, comme avec Tauri.
    pub fn state<T: Send + Sync + 'static>(&self) -> State<'_, T> {
        self.try_state::<T>()
            .unwrap_or_else(|| panic!("état non géré : {}", std::any::type_name::<T>()))
    }

    /// Dépose un événement nommé pour l'interface (`listen` du frontend).
    pub fn emit<S: Serialize>(&self, event: &str, payload: S) -> Result<(), String> {
        let payload = serde_json::to_value(payload).map_err(|e| e.to_string())?;
        self.send(Event::Emit {
            name: event.to_string(),
            payload,
        })
    }

    /// Une demande à la fenêtre principale.
    pub fn window(&self, request: WindowRequest) {
        let _ = self.send(Event::Window(request));
    }

    /// Demande l'arrêt de l'application.
    pub fn exit(&self, code: i32) {
        let _ = self.send(Event::Exit(code));
    }

    /// Demande un redémarrage de l'application.
    pub fn restart(&self) {
        let _ = self.send(Event::Restart);
    }

    fn send(&self, event: Event) -> Result<(), String> {
        self.inner
            .sender
            .send(event)
            .map_err(|_| "interface fermée".to_string())
    }
}

/// Une valeur gérée, comme `tauri::State` : se déréférence vers la valeur.
pub struct State<'a, T: 'static>(Arc<T>, PhantomData<&'a ()>);

impl<T: 'static> State<'_, T> {
    /// La valeur elle-même.
    pub fn inner(&self) -> &T {
        &self.0
    }
}

impl<T: 'static> Deref for State<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: 'static> Clone for State<'_, T> {
    fn clone(&self) -> Self {
        State(self.0.clone(), PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_managed_value_is_found_by_type() {
        let (app, _rx) = AppHandle::new();
        app.manage(42u32);
        assert_eq!(*app.state::<u32>(), 42);
        assert!(app.try_state::<String>().is_none());
    }

    #[test]
    fn emit_delivers_the_event_as_json() {
        let (app, rx) = AppHandle::new();
        app.emit("playback-preparing", true).unwrap();
        match rx.recv().unwrap() {
            Event::Emit { name, payload } => {
                assert_eq!(name, "playback-preparing");
                assert_eq!(payload, serde_json::Value::Bool(true));
            }
            other => panic!("inattendu : {other:?}"),
        }
    }
}
