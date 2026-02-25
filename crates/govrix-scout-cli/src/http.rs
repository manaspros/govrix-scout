#![allow(dead_code)]

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to connect to {url}"))?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("API returned {status}: {body}");
        }
        serde_json::from_str(&body).with_context(|| "invalid JSON response")
    }

    pub async fn post(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .with_context(|| format!("failed to connect to {url}"))?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("API returned {status}: {body}");
        }
        serde_json::from_str(&body).with_context(|| "invalid JSON response")
    }

    pub async fn put_json(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .put(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("failed to connect to {url}"))?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("API returned {status}: {text}");
        }
        serde_json::from_str(&text).with_context(|| "invalid JSON response")
    }
}
