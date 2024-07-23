use snarkvm::{
    console::network::Network,
    ledger::{puzzle::Solution, Block, Transaction},
    prelude::{
        Ciphertext, Field, Identifier, Plaintext, Program, ProgramID, Record, Value, ViewKey,
    },
};

use anyhow::{anyhow, bail, ensure, Error, Result};
use indexmap::IndexMap;
use parking_lot::RwLock;
use rand::thread_rng;
use tracing::{debug, warn};

use std::{marker::PhantomData, ops::Range, sync::Arc, time::Duration};

mod asynchronous;

mod blocking;

pub mod helpers;
pub use helpers::{HTTPClient, LoadBalancer, NodeState, Strategy};

/// Aleo API client for interacting with the Aleo network
#[derive(Clone)]
pub struct AleoRESTClient<N: Network, const BLOCKING: bool> {
    client: HTTPClient,
    load_balancer: Arc<RwLock<LoadBalancer>>,
    network_id: String,
    retries: u32,
    _phantom: PhantomData<N>,
}

impl<N: Network, const BLOCKING: bool> AleoRESTClient<N, BLOCKING> {
    pub fn new(base_url: &str) -> Result<Self> {
        // Create a new client to make requests.
        Self::from_targets(vec![base_url.to_string()])
    }

    /// Create a new Aleo API client with multiple targets
    pub fn from_targets(targets: Vec<String>) -> Result<Self> {
        // Ensure at least one target is provided.
        ensure!(!targets.is_empty(), "No targets provided for load balancer");

        // Create a new client to make requests.
        let client = if BLOCKING {
            HTTPClient::Blocking(
                ureq::AgentBuilder::new()
                    .timeout(Duration::from_secs(20))
                    .build(),
            )
        } else {
            HTTPClient::Async(
                reqwest::ClientBuilder::new()
                    .timeout(Duration::from_secs(20))
                    .build()?,
            )
        };

        // Create a new load balancer with the suggested targets.
        let load_balancer = Arc::new(RwLock::new(LoadBalancer::new(targets, Strategy::Random)?));

        // Return the REST client.
        Ok(AleoRESTClient {
            client,
            load_balancer,
            network_id: Self::network_prefix()?.to_string(),
            retries: 0,
            _phantom: PhantomData,
        })
    }

    /// Get an endpoint from the load balancer.
    pub fn get_endpoint(&self) -> String {
        self.load_balancer.read().get_target().to_string()
    }

    /// Get the targets from the load balancer.
    pub fn get_targets(&self) -> IndexMap<String, NodeState> {
        self.load_balancer.read().get_targets().clone()
    }

    /// Get the healthy targets from the load balancer.
    pub fn get_healthy_targets(&self) -> IndexMap<String, NodeState> {
        self.load_balancer.read().get_healthy_targets()
    }

    /// Get the unhealthy targets from the load balancer.
    pub fn get_unhealthy_targets(&self) -> IndexMap<String, NodeState> {
        self.load_balancer.read().get_unhealthy_targets()
    }

    /// Get network ID being interacted with.
    pub fn network_id(&self) -> &str {
        &self.network_id
    }

    // Get the network REST prefix for the current network.
    fn network_prefix() -> Result<&'static str> {
        Ok(match N::NAME {
            "Aleo Mainnet (v0)" => "mainnet",
            "Aleo Testnet (v0)" => "testnet",
            "Aleo Canary (v0)" => "canary",
            _ => bail!("Unsupported network!"),
        })
    }

    /// Get the number of retries set for the client.
    pub fn retries(&self) -> u32 {
        self.retries
    }

    /// Set the number of retries for the client.
    pub fn set_retries(&mut self, retries: u32) {
        self.retries = retries;
    }

    /// Set the load balancer strategy.
    pub fn set_strategy(&mut self, strategy: Strategy) {
        self.load_balancer.write().set_strategy(strategy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use snarkvm::prelude::MainnetV0;

    #[test]
    fn test_aleo_api_client_from_targets() {
        // Create a new Aleo API client with a single target.
        let client = AleoRESTClient::<MainnetV0, true>::from_targets(vec![
            "http://localhost:3030".to_string()
        ])
        .unwrap();

        // Ensure the client has the correct endpoint and network ID.
        assert_eq!(client.get_endpoint(), "http://localhost:3030");
        assert_eq!(client.network_id(), "mainnet");

        // Test creating a new Aleo API client with multiple targets.
        let targets = vec![
            "http://localhost:3030".to_string(),
            "http://localhost:3031".to_string(),
            "http://localhost:3032".to_string(),
        ];

        // Create a new Aleo API client with multiple targets.
        let client = AleoRESTClient::<MainnetV0, true>::from_targets(targets.clone()).unwrap();

        // Ensure the client gets a random target within the list of targets.
        for _ in 0..200 {
            assert!(&targets.contains(&client.get_endpoint().to_string()));
        }
    }

    #[test]
    fn test_aleo_api_client_new() {
        let client = AleoRESTClient::<MainnetV0, true>::new("http://localhost:3030").unwrap();
        assert_eq!(client.get_endpoint(), "http://localhost:3030");
        assert_eq!(client.network_id(), "mainnet");
    }
}
