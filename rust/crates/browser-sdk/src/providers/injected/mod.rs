//! Injected provider for the browser SDK.
//!
//! Manages connections to injected browser wallets (Phantom extension,
//! external wallets discovered via EIP-6963 and Wallet Standard).
//! Supports multi-chain connections, event forwarding, and auto-connect.

pub mod chains;
pub mod wallet_standard;

pub use chains::{InjectedWalletEthereumChain, InjectedWalletSolanaChain};
pub use wallet_standard::*;

use phantom_chain_interfaces::{EthereumChain, SolanaChain, SolanaConnectOptions};
use phantom_client::constants::AddressFormat;
use phantom_embedded_provider_core::WalletAddress;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::types::{
    AuthOptions, AuthProviderType, ConnectResult, ConnectResultWalletInfo, ConnectStatus, Provider,
};
use crate::wallets::{get_wallet_registry, InjectedWalletInfo, InjectedWalletRegistry};

/// Chain callbacks interface for avoiding circular dependencies.
///
/// Mirrors the TypeScript `ChainCallbacks` interface used by injected
/// wallet chain wrappers to call back into the InjectedProvider without
/// creating a direct circular dependency.
#[async_trait::async_trait]
pub trait ChainCallbacks: Send + Sync {
    /// Connect the wallet.
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    /// Disconnect the wallet.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    /// Check if the wallet is connected.
    fn is_connected(&self) -> bool;
    /// Get the current wallet addresses.
    fn get_addresses(&self) -> Vec<WalletAddress>;
    /// Register an event listener.
    fn on(
        &self,
        event: &str,
        callback: Arc<dyn Fn(serde_json::Value) + Send + Sync>,
    ) -> u64;
    /// Remove an event listener.
    fn off(&self, event: &str, listener_id: u64);
}

/// Configuration for the injected provider.
#[derive(Debug, Clone)]
pub struct InjectedProviderConfig {
    pub address_types: Vec<AddressFormat>,
}

/// State for a specific wallet.
#[derive(Debug, Clone, Default)]
struct WalletState {
    connected: bool,
    addresses: Vec<WalletAddress>,
}

/// Options for internal connect calls.
#[derive(Debug, Clone, Default)]
struct ConnectOptions {
    /// For Solana: use onlyIfTrusted flag.
    only_if_trusted: bool,
    /// For Ethereum: use eth_accounts instead of eth_requestAccounts.
    silent: bool,
    /// Don't set up event listeners (for autoConnect).
    skip_event_listeners: bool,
}

// ---------------------------------------------------------------------------
// EventListenerRegistry
// ---------------------------------------------------------------------------

/// Registry that manages event listeners with unique IDs.
///
/// Listeners are stored per event name and can be added ([`on`](Self::on))
/// or removed ([`off`](Self::off)) by their unique numeric ID.
struct EventListenerRegistry {
    listeners: Mutex<HashMap<String, Vec<(u64, Arc<dyn Fn(serde_json::Value) + Send + Sync>)>>>,
    next_id: AtomicU64,
}

impl EventListenerRegistry {
    fn new() -> Self {
        Self {
            listeners: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Register a callback for the given event. Returns a unique listener ID.
    fn on(
        &self,
        event: &str,
        callback: Arc<dyn Fn(serde_json::Value) + Send + Sync>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut listeners = self.listeners.lock().unwrap();
        listeners
            .entry(event.to_string())
            .or_default()
            .push((id, callback));
        id
    }

    /// Remove a listener by its ID for the given event.
    fn off(&self, event: &str, listener_id: u64) {
        let mut listeners = self.listeners.lock().unwrap();
        if let Some(cbs) = listeners.get_mut(event) {
            cbs.retain(|(id, _)| *id != listener_id);
            if cbs.is_empty() {
                listeners.remove(event);
            }
        }
    }

    /// Emit an event, calling all registered listeners with the provided data.
    ///
    /// Callbacks are invoked outside the lock to avoid deadlocks. Panics inside
    /// callbacks are caught and logged.
    fn emit(&self, event: &str, data: serde_json::Value) {
        // Collect callbacks under the lock, then invoke them outside.
        let callbacks: Vec<Arc<dyn Fn(serde_json::Value) + Send + Sync>> = {
            let listeners = self.listeners.lock().unwrap();
            match listeners.get(event) {
                Some(cbs) => cbs.iter().map(|(_, cb)| cb.clone()).collect(),
                None => return,
            }
        };

        for callback in callbacks {
            let event_data = data.clone();
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                callback(event_data);
            })) {
                error!(?e, event = event, "Event callback panicked");
            }
        }
    }

    /// Get the count of listeners for a given event.
    fn listener_count(&self, event: &str) -> usize {
        let listeners = self.listeners.lock().unwrap();
        listeners.get(event).map_or(0, |cbs| cbs.len())
    }
}

// ---------------------------------------------------------------------------
// InjectedProvider
// ---------------------------------------------------------------------------

/// Injected provider that manages connections to browser-injected wallets.
///
/// Supports connecting to Phantom and external wallets (discovered via
/// EIP-6963 and Wallet Standard), multi-chain address resolution, event
/// forwarding from chain providers, and auto-reconnection.
pub struct InjectedProvider {
    address_types: Vec<AddressFormat>,
    wallet_registry: Arc<InjectedWalletRegistry>,
    selected_wallet_id: Arc<RwLock<Option<String>>>,
    wallet_states: Arc<RwLock<HashMap<String, WalletState>>>,
    event_registry: Arc<EventListenerRegistry>,
    /// Track which wallet IDs have had event listeners set up.
    event_listeners_setup: Mutex<HashSet<String>>,
    /// Store chain-level listener IDs for cleanup, keyed by wallet ID.
    /// Each entry is `(event_name, listener_id, chain_kind)` where chain_kind
    /// is `"solana"` or `"ethereum"`.
    chain_listener_ids: Mutex<HashMap<String, Vec<ChainListenerEntry>>>,
    /// Persistent storage for was-connected flag (simulated in-memory for
    /// non-browser environments; in a real browser this would use localStorage).
    was_connected: RwLock<bool>,
    /// Persistent storage for last wallet ID.
    last_wallet_id: RwLock<Option<String>>,
}

/// A recorded chain-level listener that we registered on a chain provider,
/// so we can clean it up later.
struct ChainListenerEntry {
    event: String,
    listener_id: u64,
    chain: ChainKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChainKind {
    Solana,
    Ethereum,
}

impl InjectedProvider {
    /// Create a new injected provider.
    pub fn new(config: InjectedProviderConfig) -> Self {
        let wallet_registry = get_wallet_registry();
        debug!(
            address_types = ?config.address_types,
            "Initializing InjectedProvider"
        );
        Self {
            address_types: config.address_types,
            wallet_registry,
            selected_wallet_id: Arc::new(RwLock::new(None)),
            wallet_states: Arc::new(RwLock::new(HashMap::new())),
            event_registry: Arc::new(EventListenerRegistry::new()),
            event_listeners_setup: Mutex::new(HashSet::new()),
            chain_listener_ids: Mutex::new(HashMap::new()),
            was_connected: RwLock::new(false),
            last_wallet_id: RwLock::new(None),
        }
    }

    // ---------------------------------------------------------------
    // Event management (public API)
    // ---------------------------------------------------------------

    /// Register an event listener.
    ///
    /// Supported events: `"connect"`, `"disconnect"`, `"connect_start"`,
    /// `"connect_error"`.
    ///
    /// Returns a listener ID that can be used with [`off`](Self::off) to
    /// remove the listener.
    pub fn on(
        &self,
        event: &str,
        callback: Arc<dyn Fn(serde_json::Value) + Send + Sync>,
    ) -> u64 {
        debug!(event = event, "Adding event listener");
        self.event_registry.on(event, callback)
    }

    /// Remove an event listener by its ID.
    pub fn off(&self, event: &str, listener_id: u64) {
        debug!(event = event, listener_id = listener_id, "Removing event listener");
        self.event_registry.off(event, listener_id);
    }

    /// Emit an event to all registered listeners.
    fn emit(&self, event: &str, data: serde_json::Value) {
        debug!(
            event = event,
            listener_count = self.event_registry.listener_count(event),
            "Emitting event"
        );
        self.event_registry.emit(event, data);
    }

    // ---------------------------------------------------------------
    // Chain accessors
    // ---------------------------------------------------------------

    /// Access the Solana chain provider for the currently selected wallet.
    ///
    /// Returns an error if Solana is not enabled in the address types, the
    /// wallet has not been discovered, or the wallet does not support Solana.
    pub async fn solana(
        &self,
    ) -> Result<Arc<dyn SolanaChain>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.address_types.contains(&AddressFormat::Solana) {
            return Err("Solana not enabled for this provider".into());
        }

        let wallet_id = self.get_selected_wallet_id().await;

        match self.wallet_registry.get_by_id(&wallet_id) {
            Some(info) => match info.providers {
                Some(ref providers) => match providers.solana {
                    Some(ref provider) => Ok(provider.clone()),
                    None => Err(format!(
                        "Selected wallet \"{}\" does not support Solana.",
                        info.name
                    )
                    .into()),
                },
                None => Err(format!(
                    "Wallet \"{}\" has no providers available.",
                    info.name
                )
                .into()),
            },
            None => Err(format!(
                "Wallet \"{}\" not found. Please ensure wallet discovery has completed.",
                wallet_id
            )
            .into()),
        }
    }

    /// Access the Ethereum chain provider for the currently selected wallet.
    ///
    /// Returns an error if Ethereum is not enabled in the address types, the
    /// wallet has not been discovered, or the wallet does not support Ethereum.
    pub async fn ethereum(
        &self,
    ) -> Result<Arc<dyn EthereumChain>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.address_types.contains(&AddressFormat::Ethereum) {
            return Err("Ethereum not enabled for this provider".into());
        }

        let wallet_id = self.get_selected_wallet_id().await;

        match self.wallet_registry.get_by_id(&wallet_id) {
            Some(info) => match info.providers {
                Some(ref providers) => match providers.ethereum {
                    Some(ref provider) => Ok(provider.clone()),
                    None => Err(format!(
                        "Selected wallet \"{}\" does not support Ethereum.",
                        info.name
                    )
                    .into()),
                },
                None => Err(format!(
                    "Wallet \"{}\" has no providers available.",
                    info.name
                )
                .into()),
            },
            None => Err(format!(
                "Wallet \"{}\" not found. Please ensure wallet discovery has completed.",
                wallet_id
            )
            .into()),
        }
    }

    // ---------------------------------------------------------------
    // Wallet state helpers
    // ---------------------------------------------------------------

    #[allow(dead_code)]
    async fn get_wallet_state(&self, wallet_id: &str) -> WalletState {
        let states = self.wallet_states.read().await;
        states.get(wallet_id).cloned().unwrap_or_default()
    }

    async fn set_wallet_state(&self, wallet_id: &str, state: WalletState) {
        let mut states = self.wallet_states.write().await;
        states.insert(wallet_id.to_string(), state);
    }

    /// Update wallet state with new addresses for a specific address type.
    ///
    /// Replaces all existing addresses of the given type with the new addresses,
    /// keeping addresses of other types intact. Mirrors the TypeScript
    /// `updateWalletAddresses` method.
    ///
    /// Returns the updated full list of addresses.
    fn update_wallet_addresses_sync(
        wallet_states: &RwLock<HashMap<String, WalletState>>,
        wallet_id: &str,
        new_addresses: &[String],
        address_type: AddressFormat,
    ) -> Vec<WalletAddress> {
        let mut states = wallet_states.try_write().unwrap_or_else(|_| {
            // Fall back: spin until available. In practice the lock is
            // short-lived so this should never block long.
            loop {
                if let Ok(guard) = wallet_states.try_write() {
                    break guard;
                }
                std::thread::yield_now();
            }
        });
        let state = states
            .entry(wallet_id.to_string())
            .or_insert_with(WalletState::default);

        // Keep addresses of other types, replace same-type addresses.
        let other_addresses: Vec<WalletAddress> = state
            .addresses
            .iter()
            .filter(|a| a.address_type != address_type)
            .cloned()
            .collect();
        let typed_addresses: Vec<WalletAddress> = new_addresses
            .iter()
            .map(|addr| WalletAddress {
                address_type: address_type.clone(),
                address: addr.clone(),
            })
            .collect();

        let mut updated = other_addresses;
        updated.extend(typed_addresses);

        state.connected = !updated.is_empty();
        state.addresses = updated.clone();

        updated
    }

    /// Remove addresses of a specific type from the wallet state (sync).
    ///
    /// Used by disconnect / accountChanged(null) handlers to strip a chain's
    /// addresses and update connected status.
    fn remove_addresses_of_type_sync(
        wallet_states: &RwLock<HashMap<String, WalletState>>,
        wallet_id: &str,
        address_type: AddressFormat,
    ) {
        if let Ok(mut states) = wallet_states.try_write() {
            if let Some(state) = states.get_mut(wallet_id) {
                state.addresses.retain(|a| a.address_type != address_type);
                state.connected = !state.addresses.is_empty();
            }
        }
    }

    /// Get the selected wallet ID (async).
    async fn get_selected_wallet_id(&self) -> String {
        self.selected_wallet_id
            .read()
            .await
            .clone()
            .unwrap_or_else(|| "phantom".to_string())
    }

    /// Get the selected wallet ID (sync, best-effort with try_read).
    fn get_wallet_id_sync(&self) -> String {
        self.selected_wallet_id
            .try_read()
            .ok()
            .and_then(|guard| guard.clone())
            .unwrap_or_else(|| "phantom".to_string())
    }

    // ---------------------------------------------------------------
    // Wallet validation
    // ---------------------------------------------------------------

    /// Validate a wallet ID and select it. Returns the wallet info on success.
    async fn validate_and_select_wallet(
        &self,
        requested_wallet_id: &str,
    ) -> Result<InjectedWalletInfo, Box<dyn std::error::Error + Send + Sync>> {
        if !self.wallet_registry.has(requested_wallet_id) {
            error!(
                wallet_id = requested_wallet_id,
                "Unknown injected wallet id requested"
            );
            return Err(
                format!("Unknown injected wallet id: {}", requested_wallet_id).into(),
            );
        }

        let wallet_info = self.wallet_registry.get_by_id(requested_wallet_id);
        match wallet_info {
            Some(info) if info.providers.is_some() => {
                *self.selected_wallet_id.write().await =
                    Some(requested_wallet_id.to_string());
                debug!(
                    wallet_id = requested_wallet_id,
                    "Selected injected wallet for connection"
                );
                Ok(info)
            }
            _ => {
                warn!(
                    wallet_id = requested_wallet_id,
                    "Wallet not available for connection"
                );
                Err(format!(
                    "Wallet not available for connection: {}",
                    requested_wallet_id
                )
                .into())
            }
        }
    }

    // ---------------------------------------------------------------
    // Chain connection logic
    // ---------------------------------------------------------------

    /// Connect to the wallet's chain providers and collect addresses.
    async fn connect_to_wallet(
        &self,
        wallet_info: &InjectedWalletInfo,
        options: &ConnectOptions,
    ) -> Result<Vec<WalletAddress>, Box<dyn std::error::Error + Send + Sync>> {
        let providers = match wallet_info.providers {
            Some(ref p) => p,
            None => {
                let wallet_id = self.get_selected_wallet_id().await;
                let err_msg = format!(
                    "Wallet adapter not available for wallet: {}",
                    wallet_id
                );
                error!(wallet_id = %wallet_id, "Wallet adapter not available");
                self.emit(
                    "connect_error",
                    serde_json::json!({
                        "error": err_msg,
                        "source": if options.skip_event_listeners { "auto-connect" } else { "manual-connect" },
                    }),
                );
                return Err(err_msg.into());
            }
        };

        let wallet_id = self.get_selected_wallet_id().await;
        debug!(
            wallet_id = %wallet_id,
            wallet_name = %wallet_info.name,
            "Connecting via wallet"
        );

        // Set up event listeners unless skipped (autoConnect sets up after
        // successful connection).
        if !options.skip_event_listeners {
            self.setup_event_listeners(wallet_info);
        }

        let mut connected_addresses: Vec<WalletAddress> = Vec::new();
        let connect_source = if options.skip_event_listeners {
            "auto-connect"
        } else {
            "manual-connect"
        };

        // --- Solana ---
        if self.address_types.contains(&AddressFormat::Solana) {
            if let Some(ref solana_provider) = providers.solana {
                debug!(
                    wallet_id = %wallet_id,
                    wallet_name = %wallet_info.name,
                    only_if_trusted = options.only_if_trusted,
                    "Attempting Solana connection"
                );
                let connect_opts = if options.only_if_trusted {
                    Some(SolanaConnectOptions {
                        only_if_trusted: Some(true),
                    })
                } else {
                    None
                };

                match solana_provider.connect(connect_opts).await {
                    Ok(result) => {
                        connected_addresses.push(WalletAddress {
                            address_type: AddressFormat::Solana,
                            address: result.public_key.clone(),
                        });
                        info!(
                            address = %result.public_key,
                            wallet_id = %wallet_id,
                            wallet_name = %wallet_info.name,
                            "Solana connected successfully"
                        );
                    }
                    Err(err) => {
                        warn!(
                            error = %err,
                            wallet_id = %wallet_id,
                            wallet_name = %wallet_info.name,
                            "Failed to connect Solana, stopping"
                        );
                        self.emit(
                            "connect_error",
                            serde_json::json!({
                                "error": err.to_string(),
                                "source": connect_source,
                            }),
                        );
                        return Err(err);
                    }
                }
            }
        }

        // --- Ethereum ---
        if self.address_types.contains(&AddressFormat::Ethereum) {
            if let Some(ref ethereum_provider) = providers.ethereum {
                debug!(
                    wallet_id = %wallet_id,
                    wallet_name = %wallet_info.name,
                    silent = options.silent,
                    "Attempting Ethereum connection"
                );

                let accounts_result = if options.silent {
                    // For silent connection, use eth_accounts instead of
                    // eth_requestAccounts.
                    ethereum_provider
                        .request("eth_accounts", None)
                        .await
                        .and_then(|val| {
                            serde_json::from_value::<Vec<String>>(val).map_err(|e| {
                                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                            })
                        })
                } else {
                    ethereum_provider.connect().await
                };

                match accounts_result {
                    Ok(accounts) => {
                        if !accounts.is_empty() {
                            for address in &accounts {
                                connected_addresses.push(WalletAddress {
                                    address_type: AddressFormat::Ethereum,
                                    address: address.clone(),
                                });
                            }
                            info!(
                                addresses = ?accounts,
                                wallet_id = %wallet_id,
                                wallet_name = %wallet_info.name,
                                "Ethereum connected successfully"
                            );
                        }
                    }
                    Err(err) => {
                        warn!(
                            error = %err,
                            wallet_id = %wallet_id,
                            wallet_name = %wallet_info.name,
                            "Failed to connect Ethereum, stopping"
                        );
                        self.emit(
                            "connect_error",
                            serde_json::json!({
                                "error": err.to_string(),
                                "source": connect_source,
                            }),
                        );
                        return Err(err);
                    }
                }
            }
        }

        Ok(connected_addresses)
    }

    /// Finalize a connection by updating state, persisting flags, and emitting
    /// events.
    async fn finalize_connection(
        &self,
        connected_addresses: Vec<WalletAddress>,
        auth_provider: Option<AuthProviderType>,
        wallet_id: Option<String>,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        if connected_addresses.is_empty() {
            let err_msg = "Failed to connect to any supported wallet provider";
            self.emit(
                "connect_error",
                serde_json::json!({
                    "error": err_msg,
                    "source": "manual-connect",
                }),
            );
            return Err(err_msg.into());
        }

        // Update wallet state.
        let selected = self.get_selected_wallet_id().await;
        self.set_wallet_state(
            &selected,
            WalletState {
                connected: true,
                addresses: connected_addresses.clone(),
            },
        )
        .await;

        debug!(
            address_count = connected_addresses.len(),
            "Finalized connection with addresses"
        );

        // Persist was-connected flag.
        *self.was_connected.write().await = true;
        if let Some(ref wid) = wallet_id {
            *self.last_wallet_id.write().await = Some(wid.clone());
        }

        // Build wallet info for the result.
        let wallet = wallet_id.as_ref().and_then(|wid| {
            self.wallet_registry.get_by_id(wid).map(|info| {
                ConnectResultWalletInfo {
                    id: info.id,
                    name: info.name,
                    icon: info.icon,
                    address_types: info.address_types,
                    rdns: info.rdns,
                    discovery: info.discovery,
                }
            })
        });

        let result = ConnectResult {
            addresses: connected_addresses.clone(),
            wallet_id: wallet_id.clone(),
            auth_user_id: None,
            auth_provider,
            status: Some(ConnectStatus::Completed),
            wallet,
        };

        self.emit(
            "connect",
            serde_json::json!({
                "addresses": connected_addresses.iter().map(|a| {
                    serde_json::json!({
                        "addressType": format!("{:?}", a.address_type),
                        "address": a.address,
                    })
                }).collect::<Vec<_>>(),
                "source": "manual-connect",
                "walletId": wallet_id,
            }),
        );

        Ok(result)
    }

    // ---------------------------------------------------------------
    // Event listener setup for chain providers
    // ---------------------------------------------------------------

    /// Set up event listeners on the wallet's chain providers that forward
    /// events (connect, disconnect, accountChanged/accountsChanged) to this
    /// provider's event registry.
    ///
    /// Cleans up listeners for previously selected wallets to prevent stale
    /// events from causing wallet ID flicker.
    fn setup_event_listeners(&self, wallet_info: &InjectedWalletInfo) {
        let wallet_id = self.get_wallet_id_sync();

        // Check if already set up.
        {
            let setup = self.event_listeners_setup.lock().unwrap();
            if setup.contains(&wallet_id) {
                debug!(wallet_id = %wallet_id, "Event listeners already set up for wallet");
                return;
            }
        }

        // Clean up event listeners from other wallets.
        self.cleanup_other_wallet_listeners(&wallet_id);

        debug!(wallet_id = %wallet_id, "Setting up event listeners");

        let mut new_entries: Vec<ChainListenerEntry> = Vec::new();

        if let Some(ref providers) = wallet_info.providers {
            // Clone Arc-wrapped fields so closures can capture them.
            let ws = self.wallet_states.clone();
            let er = self.event_registry.clone();
            let swid = self.selected_wallet_id.clone();

            // --- Solana event listeners ---
            if self.address_types.contains(&AddressFormat::Solana) {
                if let Some(ref solana_provider) = providers.solana {
                    // Solana connect handler
                    let wid = wallet_id.clone();
                    let ws_c = ws.clone();
                    let er_c = er.clone();
                    let swid_c = swid.clone();
                    let lid = solana_provider.on(
                        "connect",
                        Box::new(move |data| {
                            let public_key = data.as_str().unwrap_or("").to_string();
                            debug!(wallet_id = %wid, public_key = %public_key, "Solana connect event received");
                            if !public_key.is_empty() {
                                let new_addrs = Self::update_wallet_addresses_sync(
                                    &ws_c, &wid, &[public_key], AddressFormat::Solana,
                                );
                                let sel = swid_c.try_read().ok().and_then(|g| g.clone());
                                er_c.emit("connect", serde_json::json!({
                                    "addresses": new_addrs.iter().map(|a| serde_json::json!({
                                        "addressType": format!("{:?}", a.address_type),
                                        "address": a.address,
                                    })).collect::<Vec<_>>(),
                                    "source": "wallet",
                                    "walletId": sel,
                                }));
                            }
                        }),
                    );
                    new_entries.push(ChainListenerEntry {
                        event: "connect".to_string(),
                        listener_id: lid,
                        chain: ChainKind::Solana,
                    });

                    // Solana disconnect handler
                    let wid = wallet_id.clone();
                    let ws_c = ws.clone();
                    let er_c = er.clone();
                    let lid = solana_provider.on(
                        "disconnect",
                        Box::new(move |_data| {
                            debug!(wallet_id = %wid, "Solana disconnect event received");
                            Self::remove_addresses_of_type_sync(
                                &ws_c, &wid, AddressFormat::Solana,
                            );
                            er_c.emit("disconnect", serde_json::json!({
                                "source": "wallet",
                            }));
                        }),
                    );
                    new_entries.push(ChainListenerEntry {
                        event: "disconnect".to_string(),
                        listener_id: lid,
                        chain: ChainKind::Solana,
                    });

                    // Solana accountChanged handler
                    let wid = wallet_id.clone();
                    let ws_c = ws.clone();
                    let er_c = er.clone();
                    let swid_c = swid.clone();
                    let lid = solana_provider.on(
                        "accountChanged",
                        Box::new(move |data| {
                            // data can be a string (new public key) or null (disconnected)
                            let public_key = data.as_str().map(|s| s.to_string());
                            debug!(wallet_id = %wid, ?public_key, "Solana account changed event received");
                            if let Some(pk) = public_key {
                                let new_addrs = Self::update_wallet_addresses_sync(
                                    &ws_c, &wid, &[pk], AddressFormat::Solana,
                                );
                                let sel = swid_c.try_read().ok().and_then(|g| g.clone());
                                er_c.emit("connect", serde_json::json!({
                                    "addresses": new_addrs.iter().map(|a| serde_json::json!({
                                        "addressType": format!("{:?}", a.address_type),
                                        "address": a.address,
                                    })).collect::<Vec<_>>(),
                                    "source": "wallet-account-change",
                                    "walletId": sel,
                                }));
                            } else {
                                Self::remove_addresses_of_type_sync(
                                    &ws_c, &wid, AddressFormat::Solana,
                                );
                                er_c.emit("disconnect", serde_json::json!({
                                    "source": "wallet-account-change",
                                }));
                            }
                        }),
                    );
                    new_entries.push(ChainListenerEntry {
                        event: "accountChanged".to_string(),
                        listener_id: lid,
                        chain: ChainKind::Solana,
                    });
                }
            }

            // --- Ethereum event listeners ---
            if self.address_types.contains(&AddressFormat::Ethereum) {
                if let Some(ref ethereum_provider) = providers.ethereum {
                    // Ethereum connect handler
                    let wid = wallet_id.clone();
                    let ws_c = ws.clone();
                    let er_c = er.clone();
                    let swid_c = swid.clone();
                    let lid = ethereum_provider.on(
                        "connect",
                        Box::new(move |data| {
                            // data may be an array of accounts or { chainId: ... }
                            let accounts: Vec<String> = if let Some(arr) = data.as_array() {
                                arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                            } else {
                                vec![]
                            };
                            debug!(wallet_id = %wid, ?accounts, "Ethereum connect event received");
                            if !accounts.is_empty() {
                                let new_addrs = Self::update_wallet_addresses_sync(
                                    &ws_c, &wid, &accounts, AddressFormat::Ethereum,
                                );
                                let sel = swid_c.try_read().ok().and_then(|g| g.clone());
                                er_c.emit("connect", serde_json::json!({
                                    "addresses": new_addrs.iter().map(|a| serde_json::json!({
                                        "addressType": format!("{:?}", a.address_type),
                                        "address": a.address,
                                    })).collect::<Vec<_>>(),
                                    "source": "wallet",
                                    "walletId": sel,
                                }));
                            }
                        }),
                    );
                    new_entries.push(ChainListenerEntry {
                        event: "connect".to_string(),
                        listener_id: lid,
                        chain: ChainKind::Ethereum,
                    });

                    // Ethereum disconnect handler
                    let wid = wallet_id.clone();
                    let ws_c = ws.clone();
                    let er_c = er.clone();
                    let lid = ethereum_provider.on(
                        "disconnect",
                        Box::new(move |_data| {
                            debug!(wallet_id = %wid, "Ethereum disconnect event received");
                            Self::remove_addresses_of_type_sync(
                                &ws_c, &wid, AddressFormat::Ethereum,
                            );
                            er_c.emit("disconnect", serde_json::json!({
                                "source": "wallet",
                            }));
                        }),
                    );
                    new_entries.push(ChainListenerEntry {
                        event: "disconnect".to_string(),
                        listener_id: lid,
                        chain: ChainKind::Ethereum,
                    });

                    // Ethereum accountsChanged handler
                    let wid = wallet_id.clone();
                    let ws_c = ws.clone();
                    let er_c = er.clone();
                    let swid_c = swid.clone();
                    let lid = ethereum_provider.on(
                        "accountsChanged",
                        Box::new(move |data| {
                            let accounts: Vec<String> = if let Some(arr) = data.as_array() {
                                arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                            } else {
                                vec![]
                            };
                            debug!(wallet_id = %wid, ?accounts, "Ethereum accounts changed event received");
                            if !accounts.is_empty() {
                                let new_addrs = Self::update_wallet_addresses_sync(
                                    &ws_c, &wid, &accounts, AddressFormat::Ethereum,
                                );
                                let sel = swid_c.try_read().ok().and_then(|g| g.clone());
                                er_c.emit("connect", serde_json::json!({
                                    "addresses": new_addrs.iter().map(|a| serde_json::json!({
                                        "addressType": format!("{:?}", a.address_type),
                                        "address": a.address,
                                    })).collect::<Vec<_>>(),
                                    "source": "wallet-account-change",
                                    "walletId": sel,
                                }));
                            } else {
                                Self::remove_addresses_of_type_sync(
                                    &ws_c, &wid, AddressFormat::Ethereum,
                                );
                                er_c.emit("disconnect", serde_json::json!({
                                    "source": "wallet-account-change",
                                }));
                            }
                        }),
                    );
                    new_entries.push(ChainListenerEntry {
                        event: "accountsChanged".to_string(),
                        listener_id: lid,
                        chain: ChainKind::Ethereum,
                    });
                }
            }
        }

        // Store listener IDs and mark as set up.
        {
            let mut chain_ids = self.chain_listener_ids.lock().unwrap();
            chain_ids.insert(wallet_id.clone(), new_entries);
        }
        {
            let mut setup = self.event_listeners_setup.lock().unwrap();
            setup.insert(wallet_id);
        }
    }

    /// Clean up chain-level event listeners for wallets other than the given ID.
    fn cleanup_other_wallet_listeners(&self, current_wallet_id: &str) {
        let mut setup = self.event_listeners_setup.lock().unwrap();
        let mut chain_ids = self.chain_listener_ids.lock().unwrap();

        let to_remove: Vec<String> = setup
            .iter()
            .filter(|id| *id != current_wallet_id)
            .cloned()
            .collect();

        for existing_wallet_id in &to_remove {
            if let Some(entries) = chain_ids.remove(existing_wallet_id) {
                if let Some(old_info) = self.wallet_registry.get_by_id(existing_wallet_id) {
                    Self::remove_chain_listeners_for_entries(&old_info, &entries);
                }
                debug!(
                    wallet_id = %existing_wallet_id,
                    "Cleaned up event listeners for wallet"
                );
            }
            setup.remove(existing_wallet_id);
        }
    }

    /// Clean up chain-level event listeners for a specific wallet.
    fn cleanup_chain_listeners(&self, wallet_id: &str) {
        let mut chain_ids = self.chain_listener_ids.lock().unwrap();
        if let Some(entries) = chain_ids.remove(wallet_id) {
            if let Some(wallet_info) = self.wallet_registry.get_by_id(wallet_id) {
                Self::remove_chain_listeners_for_entries(&wallet_info, &entries);
            }
        }
        let mut setup = self.event_listeners_setup.lock().unwrap();
        setup.remove(wallet_id);
    }

    /// Remove chain-level listeners described by the given entries from the
    /// wallet's providers.
    fn remove_chain_listeners_for_entries(
        wallet_info: &InjectedWalletInfo,
        entries: &[ChainListenerEntry],
    ) {
        if let Some(ref providers) = wallet_info.providers {
            for entry in entries {
                match entry.chain {
                    ChainKind::Solana => {
                        if let Some(ref solana) = providers.solana {
                            solana.off(&entry.event, entry.listener_id);
                        }
                    }
                    ChainKind::Ethereum => {
                        if let Some(ref ethereum) = providers.ethereum {
                            ethereum.off(&entry.event, entry.listener_id);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Provider trait implementation
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl Provider for InjectedProvider {
    async fn connect(
        &self,
        auth_options: &AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            address_types = ?self.address_types,
            provider = ?auth_options.provider,
            "Starting injected provider connect"
        );

        if auth_options.provider != AuthProviderType::Injected {
            return Err(format!(
                "Invalid provider for injected connection: {:?}. Must be Injected",
                auth_options.provider
            )
            .into());
        }

        let requested_wallet_id = auth_options
            .wallet_id
            .clone()
            .unwrap_or_else(|| "phantom".to_string());

        self.emit(
            "connect_start",
            serde_json::json!({
                "source": "manual-connect",
                "providerType": "injected",
                "walletId": &requested_wallet_id,
            }),
        );

        match self.validate_and_select_wallet(&requested_wallet_id).await {
            Ok(wallet_info) => {
                let options = ConnectOptions::default();
                match self.connect_to_wallet(&wallet_info, &options).await {
                    Ok(connected_addresses) => {
                        let selected = self.get_selected_wallet_id().await;
                        self.finalize_connection(
                            connected_addresses,
                            Some(AuthProviderType::Injected),
                            Some(selected),
                        )
                        .await
                    }
                    Err(err) => {
                        self.emit(
                            "connect_error",
                            serde_json::json!({
                                "error": err.to_string(),
                                "source": "manual-connect",
                            }),
                        );
                        Err(err)
                    }
                }
            }
            Err(err) => {
                self.emit(
                    "connect_error",
                    serde_json::json!({
                        "error": err.to_string(),
                        "source": "manual-connect",
                    }),
                );
                Err(err)
            }
        }
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting injected provider disconnect");

        let wallet_id = self.get_selected_wallet_id().await;

        // Disconnect from chain providers.
        if let Some(wallet_info) = self.wallet_registry.get_by_id(&wallet_id) {
            if let Some(ref providers) = wallet_info.providers {
                if self.address_types.contains(&AddressFormat::Solana) {
                    if let Some(ref solana_provider) = providers.solana {
                        if let Err(err) = solana_provider.disconnect().await {
                            warn!(error = %err, "Failed to disconnect Solana");
                        } else {
                            debug!("Solana disconnected successfully");
                        }
                    }
                }
                if self.address_types.contains(&AddressFormat::Ethereum) {
                    if let Some(ref ethereum_provider) = providers.ethereum {
                        if let Err(err) = ethereum_provider.disconnect().await {
                            warn!(error = %err, "Failed to disconnect Ethereum");
                        } else {
                            debug!("Ethereum disconnected successfully");
                        }
                    }
                }
            }
        }

        // Clean up chain-level event listeners.
        self.cleanup_chain_listeners(&wallet_id);

        // Update wallet state.
        self.set_wallet_state(
            &wallet_id,
            WalletState {
                connected: false,
                addresses: vec![],
            },
        )
        .await;

        // Clear persisted state.
        *self.was_connected.write().await = false;
        debug!("Cleared was-connected flag to prevent auto-reconnect");

        self.emit(
            "disconnect",
            serde_json::json!({
                "source": "manual-disconnect",
            }),
        );

        info!("Injected provider disconnected successfully");
        Ok(())
    }

    fn get_addresses(&self) -> Vec<WalletAddress> {
        let wallet_id = self.get_wallet_id_sync();
        // Best-effort sync access via try_read.
        if let Ok(states) = self.wallet_states.try_read() {
            states
                .get(&wallet_id)
                .map(|s| s.addresses.clone())
                .unwrap_or_default()
        } else {
            vec![]
        }
    }

    fn is_connected(&self) -> bool {
        let wallet_id = self.get_wallet_id_sync();
        if let Ok(states) = self.wallet_states.try_read() {
            states
                .get(&wallet_id)
                .map(|s| s.connected)
                .unwrap_or(false)
        } else {
            false
        }
    }

    async fn auto_connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Attempting auto-connect");

        // Check if previously connected.
        let was_connected = *self.was_connected.read().await;
        if !was_connected {
            debug!("Skipping auto-connect: user was not previously connected");
            return Ok(());
        }

        let last_wallet_id = self
            .last_wallet_id
            .read()
            .await
            .clone()
            .unwrap_or_else(|| "phantom".to_string());

        debug!(
            last_wallet_id = %last_wallet_id,
            "User was previously connected, attempting auto-connect"
        );

        self.emit(
            "connect_start",
            serde_json::json!({
                "source": "auto-connect",
                "providerType": "injected",
            }),
        );

        match self.validate_and_select_wallet(&last_wallet_id).await {
            Ok(wallet_info) => {
                let options = ConnectOptions {
                    only_if_trusted: true,
                    silent: true,
                    skip_event_listeners: true,
                };

                let connected_addresses = match self
                    .connect_to_wallet(&wallet_info, &options)
                    .await
                {
                    Ok(addrs) => addrs,
                    Err(err) => {
                        debug!(
                            error = %err,
                            wallet_id = %last_wallet_id,
                            "Auto-connect failed (expected if not trusted)"
                        );
                        vec![]
                    }
                };

                if connected_addresses.is_empty() {
                    debug!("Auto-connect failed: no trusted connections available");
                    self.emit(
                        "connect_error",
                        serde_json::json!({
                            "error": "No trusted connections available",
                            "source": "auto-connect",
                        }),
                    );
                    return Ok(());
                }

                // Set up event listeners after successful connection.
                self.setup_event_listeners(&wallet_info);

                // Update wallet state.
                let selected = self.get_selected_wallet_id().await;
                self.set_wallet_state(
                    &selected,
                    WalletState {
                        connected: true,
                        addresses: connected_addresses.clone(),
                    },
                )
                .await;

                self.emit(
                    "connect",
                    serde_json::json!({
                        "addresses": connected_addresses.iter().map(|a| {
                            serde_json::json!({
                                "addressType": format!("{:?}", a.address_type),
                                "address": a.address,
                            })
                        }).collect::<Vec<_>>(),
                        "source": "auto-connect",
                        "walletId": &selected,
                    }),
                );

                info!(
                    address_count = connected_addresses.len(),
                    wallet_id = %selected,
                    "Auto-connect successful"
                );

                Ok(())
            }
            Err(err) => {
                debug!(
                    error = %err,
                    "Auto-connect failed with error"
                );
                self.emit(
                    "connect_error",
                    serde_json::json!({
                        "error": err.to_string(),
                        "source": "auto-connect",
                    }),
                );
                Ok(())
            }
        }
    }

    fn get_enabled_address_types(&self) -> Vec<AddressFormat> {
        // For external wallets, return the wallet's own address types.
        let wallet_id = self.get_wallet_id_sync();
        if wallet_id != "phantom" {
            if let Some(info) = self.wallet_registry.get_by_id(&wallet_id) {
                return info.address_types;
            }
        }
        self.address_types.clone()
    }

    async fn solana(
        &self,
    ) -> Result<Arc<dyn SolanaChain>, Box<dyn std::error::Error + Send + Sync>> {
        // Delegate to the inherent method.
        InjectedProvider::solana(self).await
    }

    async fn ethereum(
        &self,
    ) -> Result<Arc<dyn EthereumChain>, Box<dyn std::error::Error + Send + Sync>> {
        // Delegate to the inherent method.
        InjectedProvider::ethereum(self).await
    }
}
