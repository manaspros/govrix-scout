//! HTTP client for the Govrix Scout management API.

/// Thin wrapper around [`reqwest::Client`] that prepends a base URL.
#[allow(dead_code)]
pub struct ApiClient {
    /// The base URL (no trailing slash), e.g. `http://localhost:4001`.
    pub base_url: String,
    client: reqwest::Client,
}

#[allow(dead_code)]
impl ApiClient {
    /// Create a new client pointing at `base_url`.
    ///
    /// Trailing slashes on `base_url` are stripped so that `url()` always
    /// produces a well-formed path.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Build a full URL by appending `path` (which must start with `/`).
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Perform a GET request and deserialise the JSON response.
    pub async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let resp = self.client.get(self.url(path)).send().await?;
        Ok(resp.json::<T>().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_default_base_url() {
        let c = ApiClient::new("http://localhost:4001");
        assert_eq!(c.base_url, "http://localhost:4001");
    }

    #[test]
    fn build_url_appends_path() {
        let c = ApiClient::new("http://localhost:4001");
        assert_eq!(
            c.url("/api/v1/health"),
            "http://localhost:4001/api/v1/health"
        );
    }
}
