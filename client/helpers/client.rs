use anyhow::{bail, Result};

/// HTTP client for Aleo Network nodes that can be either async or blocking.
#[derive(Clone)]
pub enum HTTPClient {
    Async(reqwest::Client),
    Blocking(ureq::Agent),
}

impl HTTPClient {
    /// Get the async client from the HTTP client. This will fail if the client is blocking.
    pub fn async_client(&self) -> Result<&reqwest::Client> {
        match self {
            HTTPClient::Async(client) => Ok(client),
            HTTPClient::Blocking(_) => bail!("Attempted to get async client from blocking client"),
        }
    }

    /// Get the blocking client from the HTTP client. This will fail if the client is async.
    pub fn blocking_client(&self) -> Result<&ureq::Agent> {
        match self {
            HTTPClient::Async(_) => bail!("Attempted to get blocking client from async client"),
            HTTPClient::Blocking(client) => Ok(client),
        }
    }
}
