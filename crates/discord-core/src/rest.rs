use crate::model::Message;
use reqwest::{header, Client, Method, StatusCode};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("invalid channel ID or message length (1–2000 characters required)")]
    InvalidInput,
    #[error("invalid authorization header")]
    InvalidToken,
    #[error("network request failed")]
    Network(#[from] reqwest::Error),
    #[error("Discord returned HTTP {0}")]
    Status(StatusCode),
    #[error("rate limit retry budget exhausted")]
    RateLimited,
}
#[derive(Clone)]
pub struct Rest {
    client: Client,
    // Conservative shared request gate; replace with per-bucket scheduling at scale.
    gate: Arc<Mutex<()>>,
}
impl Rest {
    pub fn new(token: &str) -> Result<Self, ApiError> {
        let mut headers = header::HeaderMap::new();
        let mut auth = header::HeaderValue::from_str(&format!("Bot {token}"))
            .map_err(|_| ApiError::InvalidToken)?;
        auth.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth);
        let client = Client::builder()
            .default_headers(headers)
            .user_agent("RustDiscordStarter/0.1")
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .build()?;
        Ok(Self {
            client,
            gate: Arc::new(Mutex::new(())),
        })
    }
    async fn request(
        &self,
        method: Method,
        channel: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response, ApiError> {
        if channel.is_empty() || channel.len() > 20 || !channel.bytes().all(|c| c.is_ascii_digit())
        {
            return Err(ApiError::InvalidInput);
        }
        let _guard = self.gate.lock().await;
        for _ in 0..4 {
            let mut request = self.client.request(
                method.clone(),
                format!("https://discord.com/api/v10/channels/{channel}/messages"),
            );
            if let Some(value) = &body {
                request = request.json(value);
            }
            let response = request.send().await?;
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                let data: Value = response.json().await?;
                let delay = data["retry_after"].as_f64().unwrap_or(1.0).clamp(0.1, 60.0);
                tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                continue;
            }
            if !response.status().is_success() {
                return Err(ApiError::Status(response.status()));
            }
            return Ok(response);
        }
        Err(ApiError::RateLimited)
    }
    pub async fn history(&self, channel: &str) -> Result<Vec<Message>, ApiError> {
        Ok(self
            .request(Method::GET, channel, None)
            .await?
            .json()
            .await?)
    }
    pub async fn send(&self, channel: &str, content: &str) -> Result<Message, ApiError> {
        if content.trim().is_empty() || content.chars().count() > 2000 {
            return Err(ApiError::InvalidInput);
        }
        Ok(self
            .request(
                Method::POST,
                channel,
                Some(json!({
                    "content": content, "allowed_mentions": { "parse": [] }
                })),
            )
            .await?
            .json()
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn rejects_invalid_messages_and_channel_paths_before_network_io() {
        let rest = Rest::new("test-only").unwrap();
        assert!(matches!(
            rest.send("123", "   ").await,
            Err(ApiError::InvalidInput)
        ));
        assert!(matches!(
            rest.send("123", &"x".repeat(2001)).await,
            Err(ApiError::InvalidInput)
        ));
        assert!(matches!(
            rest.history("../users/@me").await,
            Err(ApiError::InvalidInput)
        ));
    }
}
