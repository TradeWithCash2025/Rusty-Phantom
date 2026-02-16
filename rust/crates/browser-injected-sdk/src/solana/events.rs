//! Event listener system for Solana provider events.

use super::types::PhantomEventType;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Callback for connect events — receives the public key.
pub type ConnectCallback = Arc<dyn Fn(&str) + Send + Sync>;
/// Callback for disconnect events.
pub type DisconnectCallback = Arc<dyn Fn() + Send + Sync>;
/// Callback for account changed events — receives the new public key (or None).
pub type AccountChangedCallback = Arc<dyn Fn(Option<&str>) + Send + Sync>;

/// A unified event callback that can handle any Phantom event.
#[derive(Clone)]
pub enum PhantomEventCallback {
    Connect(ConnectCallback),
    Disconnect(DisconnectCallback),
    AccountChanged(AccountChangedCallback),
}

/// Thread-safe event listener registry for Solana events.
pub struct SolanaEventListeners {
    listeners: Mutex<HashMap<PhantomEventType, Vec<(usize, PhantomEventCallback)>>>,
    next_id: Mutex<usize>,
}

impl SolanaEventListeners {
    /// Create a new empty listener registry.
    pub fn new() -> Self {
        Self {
            listeners: Mutex::new(HashMap::new()),
            next_id: Mutex::new(0),
        }
    }

    /// Add an event listener. Returns an ID that can be used to remove it.
    pub fn add_listener(&self, event: PhantomEventType, callback: PhantomEventCallback) -> usize {
        let mut listeners = self.listeners.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();

        let id = *next_id;
        *next_id += 1;

        listeners.entry(event).or_default().push((id, callback));
        id
    }

    /// Remove an event listener by ID.
    pub fn remove_listener(&self, event: PhantomEventType, id: usize) {
        let mut listeners = self.listeners.lock().unwrap();
        if let Some(list) = listeners.get_mut(&event) {
            list.retain(|(listener_id, _)| *listener_id != id);
            if list.is_empty() {
                listeners.remove(&event);
            }
        }
    }

    /// Trigger a connect event.
    pub fn trigger_connect(&self, public_key: &str) {
        let listeners = self.listeners.lock().unwrap();
        if let Some(list) = listeners.get(&PhantomEventType::Connect) {
            for (_, cb) in list {
                if let PhantomEventCallback::Connect(f) = cb {
                    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        f(public_key);
                    })) {
                        tracing::error!("Error in connect event listener: {:?}", e);
                    }
                }
            }
        }
    }

    /// Trigger a disconnect event.
    pub fn trigger_disconnect(&self) {
        let listeners = self.listeners.lock().unwrap();
        if let Some(list) = listeners.get(&PhantomEventType::Disconnect) {
            for (_, cb) in list {
                if let PhantomEventCallback::Disconnect(f) = cb {
                    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        f();
                    })) {
                        tracing::error!("Error in disconnect event listener: {:?}", e);
                    }
                }
            }
        }
    }

    /// Trigger an account changed event.
    pub fn trigger_account_changed(&self, public_key: Option<&str>) {
        let listeners = self.listeners.lock().unwrap();
        if let Some(list) = listeners.get(&PhantomEventType::AccountChanged) {
            for (_, cb) in list {
                if let PhantomEventCallback::AccountChanged(f) = cb {
                    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        f(public_key);
                    })) {
                        tracing::error!("Error in accountChanged event listener: {:?}", e);
                    }
                }
            }
        }
    }

    /// Add an event listener by event name string with a generic JSON callback.
    /// The callback receives the event data as `serde_json::Value`.
    pub fn add_listener_by_name(
        &self,
        event: &str,
        callback: Box<dyn Fn(serde_json::Value) + Send + Sync>,
    ) -> u64 {
        let cb = Arc::new(callback);
        let (event_type, phantom_cb) = match event {
            "connect" => {
                let cb = cb.clone();
                (
                    PhantomEventType::Connect,
                    PhantomEventCallback::Connect(Arc::new(move |pk: &str| {
                        cb(serde_json::json!(pk));
                    })),
                )
            }
            "disconnect" => {
                let cb = cb.clone();
                (
                    PhantomEventType::Disconnect,
                    PhantomEventCallback::Disconnect(Arc::new(move || {
                        cb(serde_json::Value::Null);
                    })),
                )
            }
            "accountChanged" => {
                let cb = cb.clone();
                (
                    PhantomEventType::AccountChanged,
                    PhantomEventCallback::AccountChanged(Arc::new(move |pk: Option<&str>| {
                        cb(match pk {
                            Some(k) => serde_json::json!(k),
                            None => serde_json::Value::Null,
                        });
                    })),
                )
            }
            _ => return 0,
        };
        self.add_listener(event_type, phantom_cb) as u64
    }

    /// Remove an event listener by event name string and ID.
    pub fn remove_listener_by_name(&self, event: &str, id: u64) {
        let event_type = match event {
            "connect" => PhantomEventType::Connect,
            "disconnect" => PhantomEventType::Disconnect,
            "accountChanged" => PhantomEventType::AccountChanged,
            _ => return,
        };
        self.remove_listener(event_type, id as usize);
    }

    /// Clear all event listeners.
    pub fn clear_all(&self) {
        let mut listeners = self.listeners.lock().unwrap();
        listeners.clear();
    }
}

impl Default for SolanaEventListeners {
    fn default() -> Self {
        Self::new()
    }
}
