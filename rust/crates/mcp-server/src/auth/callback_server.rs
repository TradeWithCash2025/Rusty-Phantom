//! OAuth callback server for receiving authorization codes.
//!
//! Implements a local HTTP server that waits for OAuth callbacks.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::session::types::OAuthCallbackParams;
use crate::utils::logger::Logger;

/// Configuration for the callback server.
pub struct CallbackServerOptions {
    pub port: u16,
    pub host: String,
    pub path: String,
    pub timeout_ms: u64,
}

impl Default for CallbackServerOptions {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "localhost".to_string(),
            path: "/callback".to_string(),
            timeout_ms: 300_000, // 5 minutes
        }
    }
}

/// Local HTTP server that receives OAuth authorization callbacks.
pub struct CallbackServer {
    port: u16,
    host: String,
    path: String,
    timeout_ms: u64,
    logger: Logger,
    is_listening: Arc<AtomicBool>,
}

impl CallbackServer {
    /// Create a new callback server.
    pub fn new(options: CallbackServerOptions) -> Self {
        Self {
            port: options.port,
            host: options.host,
            path: options.path,
            timeout_ms: options.timeout_ms,
            logger: Logger::new("CallbackServer"),
            is_listening: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the callback URL.
    pub fn get_callback_url(&self) -> String {
        format!("http://{}:{}{}", self.host, self.port, self.path)
    }

    /// Bind the TCP listener, signal that it is ready, then wait for an
    /// OAuth callback.  Use this when you need the server to be listening
    /// before you proceed (e.g. before opening the browser).
    ///
    /// Returns a future that resolves to the callback parameters once
    /// the callback is received.
    pub async fn start_and_wait_for_callback(
        &self,
        expected_state: &str,
    ) -> Result<OAuthCallbackParams, Box<dyn std::error::Error + Send + Sync>> {
        self.wait_for_callback(expected_state).await
    }

    /// Wait until the callback server is listening on its port.
    pub async fn wait_for_listening(&self) {
        // If already listening, return immediately
        while !self.is_listening.load(Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    /// Start the server and wait for an OAuth callback.
    pub async fn wait_for_callback(
        &self,
        expected_state: &str,
    ) -> Result<OAuthCallbackParams, Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr).await?;
        self.is_listening.store(true, Ordering::Release);
        self.logger.info(&format!(
            "Callback server listening on {}",
            self.get_callback_url()
        ));

        let (tx, rx) = oneshot::channel::<Result<OAuthCallbackParams, String>>();
        let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));
        let path = self.path.clone();
        let expected_state = expected_state.to_string();
        let logger = self.logger.clone();

        let timeout_ms = self.timeout_ms;
        let handle = tokio::spawn(async move {
            loop {
                let accept_result = tokio::select! {
                    result = listener.accept() => result,
                    _ = tokio::time::sleep(std::time::Duration::from_millis(timeout_ms)) => {
                        let mut guard = tx.lock().await;
                        if let Some(sender) = guard.take() {
                            let _ = sender.send(Err("OAuth callback timeout".to_string()));
                        }
                        return;
                    }
                };

                match accept_result {
                    Ok((mut stream, _)) => {
                        let mut buf = vec![0u8; 8192];
                        let n = match stream.read(&mut buf).await {
                            Ok(n) => n,
                            Err(_) => continue,
                        };
                        let request = String::from_utf8_lossy(&buf[..n]).to_string();

                        // Parse the request line
                        let request_line = request.lines().next().unwrap_or("");
                        let parts: Vec<&str> = request_line.split_whitespace().collect();
                        if parts.len() < 2 || parts[0] != "GET" {
                            let response =
                                format_http_response(404, &get_error_page("Invalid endpoint"));
                            let _ = stream.write_all(response.as_bytes()).await;
                            continue;
                        }

                        let request_path = parts[1];

                        // Skip favicon requests
                        if request_path.contains("favicon.ico") {
                            let response = format_http_response(404, "");
                            let _ = stream.write_all(response.as_bytes()).await;
                            continue;
                        }

                        // Parse path and query string
                        let (url_path, query_string) = match request_path.split_once('?') {
                            Some((p, q)) => (p, q),
                            None => (request_path, ""),
                        };

                        if url_path != path {
                            let response =
                                format_http_response(404, &get_error_page("Invalid endpoint"));
                            let _ = stream.write_all(response.as_bytes()).await;
                            continue;
                        }

                        // Parse query parameters
                        let params = parse_query_string(query_string);

                        let session_id = params.get("session_id").cloned().unwrap_or_default();
                        let response_type =
                            params.get("response_type").cloned().unwrap_or_default();
                        let wallet_id = params.get("wallet_id").cloned().unwrap_or_default();
                        let organization_id =
                            params.get("organization_id").cloned().unwrap_or_default();
                        let auth_user_id = params.get("auth_user_id").cloned().unwrap_or_default();

                        logger.info("Received SSO callback");

                        // Validate session_id (CSRF protection)
                        if session_id.is_empty() || session_id != expected_state {
                            logger.error("Invalid session_id parameter");
                            let response = format_http_response(
                                400,
                                &get_error_page("Authorization failed: Invalid session_id"),
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                            let mut guard = tx.lock().await;
                            if let Some(sender) = guard.take() {
                                let _ =
                                    sender.send(Err("Invalid session_id parameter".to_string()));
                            }
                            return;
                        }

                        if response_type != "success" {
                            let error =
                                format!("SSO flow failed with response_type: {}", response_type);
                            logger.error(&error);
                            let response = format_http_response(
                                400,
                                &get_error_page(&format!(
                                    "Authorization failed: {}",
                                    response_type
                                )),
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                            let mut guard = tx.lock().await;
                            if let Some(sender) = guard.take() {
                                let _ = sender.send(Err(error));
                            }
                            return;
                        }

                        if wallet_id.is_empty() {
                            let response = format_http_response(
                                400,
                                &get_error_page("Authorization failed: Missing wallet_id"),
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                            let mut guard = tx.lock().await;
                            if let Some(sender) = guard.take() {
                                let _ = sender.send(Err("Missing wallet_id parameter".to_string()));
                            }
                            return;
                        }

                        if organization_id.is_empty() {
                            let response = format_http_response(
                                400,
                                &get_error_page("Authorization failed: Missing organization_id"),
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                            let mut guard = tx.lock().await;
                            if let Some(sender) = guard.take() {
                                let _ = sender
                                    .send(Err("Missing organization_id parameter".to_string()));
                            }
                            return;
                        }

                        if auth_user_id.is_empty() {
                            let response = format_http_response(
                                400,
                                &get_error_page("Authorization failed: Missing auth_user_id"),
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                            let mut guard = tx.lock().await;
                            if let Some(sender) = guard.take() {
                                let _ =
                                    sender.send(Err("Missing auth_user_id parameter".to_string()));
                            }
                            return;
                        }

                        // Success
                        logger.info("SSO callback successful");
                        let response = format_http_response(200, &get_success_page());
                        let _ = stream.write_all(response.as_bytes()).await;

                        let mut guard = tx.lock().await;
                        if let Some(sender) = guard.take() {
                            let _ = sender.send(Ok(OAuthCallbackParams {
                                session_id,
                                wallet_id,
                                organization_id,
                                auth_user_id,
                            }));
                        }
                        return;
                    }
                    Err(e) => {
                        logger.error(&format!("Accept error: {}", e));
                        continue;
                    }
                }
            }
        });

        let result = rx.await.map_err(|_| "Callback channel closed")?;
        handle.abort();

        match result {
            Ok(params) => Ok(params),
            Err(e) => Err(e.into()),
        }
    }
}

/// Parse URL query string into a HashMap.
fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            params.insert(url_decode(key), url_decode(value));
        }
    }
    params
}

/// Simple URL decoding.
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

/// Format an HTTP response.
fn format_http_response(status: u16, body: &str) -> String {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body
    )
}

/// Escape HTML special characters.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

/// Generate an HTML success page.
fn get_success_page() -> String {
    r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"><title>Authorization Successful</title>
<style>body{font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;margin:0;background:linear-gradient(135deg,#667eea,#764ba2)}.container{background:#fff;padding:3rem;border-radius:1rem;box-shadow:0 20px 60px rgba(0,0,0,.3);text-align:center;max-width:500px}h1{color:#333}p{color:#666}</style>
</head>
<body><div class="container"><h1>Authorization Successful!</h1><p>You have successfully connected your Phantom wallet.</p><p>You can close this window and return to your application.</p></div></body>
</html>"#.to_string()
}

/// Generate an HTML error page.
fn get_error_page(message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"><title>Authorization Failed</title>
<style>body{{font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;margin:0;background:linear-gradient(135deg,#f093fb,#f5576c)}}.container{{background:#fff;padding:3rem;border-radius:1rem;box-shadow:0 20px 60px rgba(0,0,0,.3);text-align:center;max-width:500px}}h1{{color:#333}}p{{color:#666}}.error-message{{color:#f44336;font-weight:500}}</style>
</head>
<body><div class="container"><h1>Authorization Failed</h1><p class="error-message">{}</p><p>Please close this window and try again.</p></div></body>
</html>"#,
        escape_html(message)
    )
}
