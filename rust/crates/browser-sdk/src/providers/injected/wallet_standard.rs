//! Wallet Standard types and adapter for the browser SDK.
//!
//! Defines types for interacting with wallets that implement the Wallet Standard
//! protocol, including type definitions and a `WalletStandardSolanaAdapter`
//! that bridges a Wallet Standard wallet to the [`SolanaChain`] trait interface.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use phantom_chain_interfaces::{
    SolanaChain, SolanaConnectOptions, SolanaConnectResult, SolanaNetwork,
    SolanaSendAllTransactionsResult, SolanaSendTransactionResult, SolanaSignMessageResult,
};

/// A Wallet Standard account.
#[derive(Debug, Clone)]
pub struct WalletStandardAccount {
    /// The account's address string.
    pub address: String,
    /// The raw public key bytes.
    pub public_key: Vec<u8>,
    /// CAIP-2 chain identifiers this account supports.
    pub chains: Vec<String>,
    /// Feature identifiers this account supports.
    pub features: Vec<String>,
}

/// Properties that can change on a Wallet Standard wallet.
#[derive(Debug, Clone, Default)]
pub struct StandardEventsChangeProperties {
    /// Updated chain identifiers, if changed.
    pub chains: Option<Vec<String>>,
    /// Updated feature identifiers, if changed.
    pub features: Option<Vec<String>>,
    /// Updated accounts, if changed.
    pub accounts: Option<Vec<WalletStandardAccount>>,
}

/// Standard connect feature.
#[async_trait::async_trait]
pub trait StandardConnectFeature: Send + Sync {
    /// Connect to the wallet, returning the available accounts.
    async fn connect(
        &self,
    ) -> Result<Vec<WalletStandardAccount>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Standard disconnect feature.
#[async_trait::async_trait]
pub trait StandardDisconnectFeature: Send + Sync {
    /// Disconnect from the wallet.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Standard events feature.
pub trait StandardEventsFeature: Send + Sync {
    /// Register a callback for the given event. Returns a listener ID.
    fn on(
        &self,
        event: &str,
        callback: Box<dyn Fn(StandardEventsChangeProperties) + Send + Sync>,
    ) -> u64;

    /// Remove a listener by its ID.
    fn off(&self, event: &str, listener_id: u64);
}

/// Solana sign message feature.
#[async_trait::async_trait]
pub trait SolanaSignMessageFeature: Send + Sync {
    /// Sign a message with the given account.
    async fn sign_message(
        &self,
        message: &[u8],
        account: &WalletStandardAccount,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Solana sign transaction feature.
#[async_trait::async_trait]
pub trait SolanaSignTransactionFeature: Send + Sync {
    /// Sign a serialized transaction with the given account.
    async fn sign_transaction(
        &self,
        transaction: &[u8],
        account: &WalletStandardAccount,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Solana sign and send transaction feature.
#[async_trait::async_trait]
pub trait SolanaSignAndSendTransactionFeature: Send + Sync {
    /// Sign and send a serialized transaction with the given account.
    ///
    /// Returns the transaction signature bytes.
    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
        account: &WalletStandardAccount,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Combined features map for a Wallet Standard wallet.
pub struct WalletStandardFeatures {
    /// Standard connect feature.
    pub connect: Option<Arc<dyn StandardConnectFeature>>,
    /// Standard disconnect feature.
    pub disconnect: Option<Arc<dyn StandardDisconnectFeature>>,
    /// Standard events feature.
    pub events: Option<Arc<dyn StandardEventsFeature>>,
    /// Solana sign message feature.
    pub solana_sign_message: Option<Arc<dyn SolanaSignMessageFeature>>,
    /// Solana sign transaction feature.
    pub solana_sign_transaction: Option<Arc<dyn SolanaSignTransactionFeature>>,
    /// Solana sign and send transaction feature.
    pub solana_sign_and_send_transaction: Option<Arc<dyn SolanaSignAndSendTransactionFeature>>,
}

/// A Wallet Standard wallet.
pub struct WalletStandardWallet {
    /// The wallet's display name.
    pub name: String,
    /// The wallet's icon (typically a data URI).
    pub icon: String,
    /// The wallet's version string.
    pub version: String,
    /// CAIP-2 chain identifiers supported by the wallet.
    pub chains: Vec<String>,
    /// The wallet's features.
    pub features: WalletStandardFeatures,
    /// The wallet's currently connected accounts.
    pub accounts: Vec<WalletStandardAccount>,
}

/// Type alias for event listeners.
type EventListener = Box<dyn Fn(serde_json::Value) + Send + Sync>;

/// Adapter that bridges a Wallet Standard wallet to the [`SolanaChain`]
/// interface.
///
/// Delegates to the underlying wallet's features for all operations and maps
/// Wallet Standard events (`"change"`) to the `SolanaChain` event model
/// (`"connect"`, `"disconnect"`, `"accountChanged"`).
pub struct WalletStandardSolanaAdapter {
    /// The underlying Wallet Standard wallet.
    wallet: Arc<WalletStandardWallet>,
    /// Wallet identifier string (used for debugging/logging).
    #[allow(dead_code)]
    wallet_id: String,
    /// Wallet display name (used for debugging/logging).
    #[allow(dead_code)]
    wallet_name: String,
    /// Currently connected public key.
    public_key: RwLock<Option<String>>,
    /// Event listeners.
    listeners: Mutex<HashMap<String, Vec<(u64, EventListener)>>>,
    /// Monotonic listener ID counter.
    next_listener_id: AtomicU64,
}

impl WalletStandardSolanaAdapter {
    /// Create a new adapter wrapping the given wallet.
    pub fn new(wallet: Arc<WalletStandardWallet>, wallet_id: String, wallet_name: String) -> Self {
        let adapter = Self {
            wallet,
            wallet_id,
            wallet_name,
            public_key: RwLock::new(None),
            listeners: Mutex::new(HashMap::new()),
            next_listener_id: AtomicU64::new(1),
        };
        adapter.setup_event_listeners();
        adapter
    }

    /// Set up event listeners for Wallet Standard "change" events.
    ///
    /// Maps Wallet Standard's single "change" event to the `SolanaChain`
    /// event model: "connect", "disconnect", and "accountChanged".
    fn setup_event_listeners(&self) {
        // In a real browser environment the events feature would call back
        // when the wallet state changes. Here we wire it up structurally.
        // The actual wiring happens through the trait implementations below.
    }

    /// Get the first connected account, if any.
    fn first_account(&self) -> Option<WalletStandardAccount> {
        self.wallet.accounts.first().cloned()
    }

    /// Emit an event to all registered listeners for that event name.
    fn emit(&self, event: &str, value: serde_json::Value) {
        if let Ok(listeners) = self.listeners.lock() {
            if let Some(entries) = listeners.get(event) {
                for (_, callback) in entries {
                    callback(value.clone());
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl SolanaChain for WalletStandardSolanaAdapter {
    fn public_key(&self) -> Option<&str> {
        // Cannot return a reference to RwLock content directly.
        // Callers should use `is_connected()` + `connect()` to get the key.
        None
    }

    fn is_connected(&self) -> bool {
        self.public_key
            .read()
            .map(|pk| pk.is_some())
            .unwrap_or(false)
    }

    async fn connect(
        &self,
        _options: Option<SolanaConnectOptions>,
    ) -> Result<SolanaConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        let connect_feature = self
            .wallet
            .features
            .connect
            .as_ref()
            .ok_or("Wallet Standard connect feature not available")?;

        let accounts = connect_feature.connect().await?;

        // After connecting, try accounts from the result, then from wallet.accounts.
        let first_account = if !accounts.is_empty() {
            Some(accounts[0].clone())
        } else if !self.wallet.accounts.is_empty() {
            Some(self.wallet.accounts[0].clone())
        } else {
            None
        };

        let account = first_account.ok_or("No accounts available after connecting to wallet")?;

        let address = account.address.clone();
        if address.is_empty() {
            return Err("Could not extract address from account".into());
        }

        if let Ok(mut pk) = self.public_key.write() {
            *pk = Some(address.clone());
        }

        Ok(SolanaConnectResult {
            public_key: address,
        })
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref disconnect_feature) = self.wallet.features.disconnect {
            disconnect_feature.disconnect().await?;
        }

        if let Ok(mut pk) = self.public_key.write() {
            *pk = None;
        }

        Ok(())
    }

    async fn get_account(&self) -> Option<String> {
        self.public_key.read().ok().and_then(|pk| pk.clone())
    }

    async fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<SolanaSignMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        let sign_feature = self
            .wallet
            .features
            .solana_sign_message
            .as_ref()
            .ok_or("Wallet Standard signMessage feature not available")?;

        let account = self
            .first_account()
            .or_else(|| self.wallet.accounts.first().cloned())
            .ok_or("No accounts available. Please connect first.")?;

        let signature = sign_feature.sign_message(message, &account).await?;

        if signature.is_empty() {
            return Err("Signature is empty".into());
        }

        let public_key = account.address.clone();

        Ok(SolanaSignMessageResult {
            signature,
            public_key,
        })
    }

    async fn sign_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let sign_feature = self
            .wallet
            .features
            .solana_sign_transaction
            .as_ref()
            .ok_or("Wallet Standard signTransaction feature not available")?;

        if self.wallet.accounts.is_empty() {
            return Err("No accounts available. Please connect first.".into());
        }

        let account = self.wallet.accounts[0].clone();
        let signed_bytes = sign_feature.sign_transaction(transaction, &account).await?;

        if signed_bytes.is_empty() {
            return Err("Empty signed transaction returned from Wallet Standard".into());
        }

        Ok(signed_bytes)
    }

    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<SolanaSendTransactionResult, Box<dyn std::error::Error + Send + Sync>> {
        let feature = self
            .wallet
            .features
            .solana_sign_and_send_transaction
            .as_ref()
            .ok_or("Wallet Standard signAndSendTransaction feature not available")?;

        if self.wallet.accounts.is_empty() {
            return Err("No accounts available. Please connect first.".into());
        }

        let account = self.wallet.accounts[0].clone();
        let signature_bytes = feature
            .sign_and_send_transaction(transaction, &account)
            .await?;

        // Convert signature bytes to base58 string.
        let signature = bs58::encode(&signature_bytes).into_string();

        Ok(SolanaSendTransactionResult { signature })
    }

    async fn sign_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut signed = Vec::with_capacity(transactions.len());
        for tx in transactions {
            signed.push(self.sign_transaction(tx).await?);
        }
        Ok(signed)
    }

    async fn sign_and_send_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<SolanaSendAllTransactionsResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut signatures = Vec::with_capacity(transactions.len());
        for tx in transactions {
            let result = self.sign_and_send_transaction(tx).await?;
            signatures.push(result.signature);
        }
        Ok(SolanaSendAllTransactionsResult { signatures })
    }

    async fn switch_network(
        &self,
        _network: SolanaNetwork,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // NOOP — Wallet Standard does not support network switching.
        Ok(())
    }

    fn on(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64 {
        let id = self.next_listener_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut listeners) = self.listeners.lock() {
            listeners
                .entry(event.to_string())
                .or_default()
                .push((id, listener));
        }
        id
    }

    fn off(&self, event: &str, listener_id: u64) {
        if let Ok(mut listeners) = self.listeners.lock() {
            if let Some(entries) = listeners.get_mut(event) {
                entries.retain(|(id, _)| *id != listener_id);
            }
        }
    }
}

/// Handle a Wallet Standard "change" event on the adapter.
///
/// Maps the change properties to SolanaChain events:
/// - Accounts present → "accountChanged" + "connect"
/// - Empty accounts → "accountChanged"(null) + "disconnect"
pub fn handle_wallet_standard_change(
    adapter: &WalletStandardSolanaAdapter,
    properties: &StandardEventsChangeProperties,
) {
    if let Some(ref accounts) = properties.accounts {
        if !accounts.is_empty() {
            let address = accounts[0].address.clone();
            if !address.is_empty() {
                if let Ok(mut pk) = adapter.public_key.write() {
                    *pk = Some(address.clone());
                }
                adapter.emit("accountChanged", serde_json::Value::String(address.clone()));
                adapter.emit("connect", serde_json::Value::String(address));
            } else {
                if let Ok(mut pk) = adapter.public_key.write() {
                    *pk = None;
                }
                adapter.emit("accountChanged", serde_json::Value::Null);
            }
        } else {
            if let Ok(mut pk) = adapter.public_key.write() {
                *pk = None;
            }
            adapter.emit("accountChanged", serde_json::Value::Null);
            adapter.emit("disconnect", serde_json::Value::Null);
        }
    }
}
