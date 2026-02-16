//! Dynamic Client Registration (DCR) client implementation.
//!
//! Implements RFC 7591: OAuth 2.0 Dynamic Client Registration Protocol.

use crate::session::types::DCRClientConfig;
use crate::utils::logger::Logger;
use serde::{Deserialize, Serialize};

/// RFC 7591 Dynamic Client Registration request payload.
#[derive(Debug, Serialize)]
struct DCRRegistrationRequest {
    client_name: String,
    redirect_uris: Vec<String>,
    grant_types: Vec<String>,
    response_types: Vec<String>,
    application_type: String,
    token_endpoint_auth_method: String,
}

/// RFC 7591 Dynamic Client Registration response.
#[derive(Debug, Deserialize)]
struct DCRRegistrationResponse {
    client_id: String,
    client_secret: String,
    client_id_issued_at: u64,
}

/// Dynamic Client Registration (DCR) client for registering OAuth clients
/// with the Phantom authorization server.
pub struct DCRClient {
    auth_base_url: String,
    app_id: String,
    logger: Logger,
}

impl DCRClient {
    /// Create a new DCR client.
    pub fn new(auth_base_url: &str, app_id: &str) -> Self {
        Self {
            auth_base_url: auth_base_url.to_string(),
            app_id: app_id.to_string(),
            logger: Logger::new("DCR"),
        }
    }

    /// Register a new OAuth client dynamically with the authorization server.
    pub async fn register(
        &self,
        redirect_uri: &str,
    ) -> Result<DCRClientConfig, Box<dyn std::error::Error + Send + Sync>> {
        let registration_endpoint = format!("{}/oauth/register", self.auth_base_url);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let client_name = format!("{}-{}", self.app_id, now);

        let payload = DCRRegistrationRequest {
            client_name: client_name.clone(),
            redirect_uris: vec![redirect_uri.to_string()],
            grant_types: vec![
                "authorization_code".to_string(),
                "refresh_token".to_string(),
            ],
            response_types: vec!["code".to_string()],
            application_type: "native".to_string(),
            token_endpoint_auth_method: "client_secret_basic".to_string(),
        };

        self.logger
            .info(&format!("Registering OAuth client: {}", client_name));

        let client = reqwest::Client::new();
        let response = client
            .post(&registration_endpoint)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            self.logger
                .error(&format!("Failed to register OAuth client: {}", error_text));
            return Err(format!("Dynamic Client Registration failed: {}", error_text).into());
        }

        let dcr_response: DCRRegistrationResponse = response.json().await?;

        self.logger.info(&format!(
            "Successfully registered client: {}",
            dcr_response.client_id
        ));

        Ok(DCRClientConfig {
            client_id: dcr_response.client_id,
            client_secret: dcr_response.client_secret,
            client_id_issued_at: dcr_response.client_id_issued_at,
        })
    }
}
