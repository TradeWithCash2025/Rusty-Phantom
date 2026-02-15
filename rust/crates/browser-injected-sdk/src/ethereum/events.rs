//! Event listener system for Ethereum provider events.

use super::types::EthereumEventType;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A callback for Ethereum events — receives arbitrary data.
pub type EthereumEventCallback = Arc<dyn Fn(serde_json::Value) + Send + Sync>;

/// Thread-safe event listener registry for Ethereum events.
pub struct EthereumEventListeners {
    listeners: Mutex<HashMap<EthereumEventType, Vec<(usize, EthereumEventCallback)>>>,
    next_id: Mutex<usize>,
}

impl EthereumEventListeners {
    /// Create a new empty listener registry.
    pub fn new() -> Self {
        Self {
            listeners: Mutex::new(HashMap::new()),
            next_id: Mutex::new(0),
        }
    }

    /// Add an event listener. Returns an ID that can be used to remove it.
    pub fn add_listener(
        &self,
        event: EthereumEventType,
        callback: EthereumEventCallback,
    ) -> usize {
        let mut listeners = self.listeners.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();

        let id = *next_id;
        *next_id += 1;

        listeners
            .entry(event)
            .or_insert_with(Vec::new)
            .push((id, callback));
        id
    }

    /// Remove an event listener by ID.
    pub fn remove_listener(&self, event: EthereumEventType, id: usize) {
        let mut listeners = self.listeners.lock().unwrap();
        if let Some(list) = listeners.get_mut(&event) {
            list.retain(|(listener_id, _)| *listener_id != id);
            if list.is_empty() {
                listeners.remove(&event);
            }
        }
    }

    /// Trigger an event with data.
    pub fn trigger_event(&self, event: EthereumEventType, data: serde_json::Value) {
        let listeners = self.listeners.lock().unwrap();
        if let Some(list) = listeners.get(&event) {
            for (_, cb) in list {
                let cb = cb.clone();
                let data = data.clone();
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    cb(data);
                })) {
                    tracing::error!("Error in {:?} event listener: {:?}", event, e);
                }
            }
        }
    }

    /// Add an event listener by event name string. Returns a listener ID.
    pub fn add_listener_by_name(
        &self,
        event: &str,
        callback: Box<dyn Fn(serde_json::Value) + Send + Sync>,
    ) -> u64 {
        let event_type = match event {
            "connect" => EthereumEventType::Connect,
            "disconnect" => EthereumEventType::Disconnect,
            "accountsChanged" => EthereumEventType::AccountsChanged,
            "chainChanged" => EthereumEventType::ChainChanged,
            _ => return 0,
        };
        self.add_listener(event_type, Arc::from(callback)) as u64
    }

    /// Remove an event listener by event name string and ID.
    pub fn remove_listener_by_name(&self, event: &str, id: u64) {
        let event_type = match event {
            "connect" => EthereumEventType::Connect,
            "disconnect" => EthereumEventType::Disconnect,
            "accountsChanged" => EthereumEventType::AccountsChanged,
            "chainChanged" => EthereumEventType::ChainChanged,
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

impl Default for EthereumEventListeners {
    fn default() -> Self {
        Self::new()
    }
}
