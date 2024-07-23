use super::*;
use rand::seq::IteratorRandom;

/// Load balancer for selecting a target node.
#[derive(Clone, Debug)]
pub struct LoadBalancer {
    targets: IndexMap<String, NodeState>,
    strategy: Strategy,
}

impl LoadBalancer {
    /// Create a new load balancer with a list of Aleo REST API targets.
    pub fn new(targets: Vec<String>, strategy: Strategy) -> Result<Self> {
        // Ensure at least one target is provided.
        ensure!(!targets.is_empty(), "No targets provided for load balancer");

        // Ensure each target URL is valid.
        for target in &targets {
            ensure!(
                target.starts_with("http://") || target.starts_with("https://"),
                "specified url {target} invalid, the base url must start with or https:// (or http:// if doing local development)"
            );
        }

        // Create a new target to height map.
        let targets = IndexMap::from_iter(
            targets
                .into_iter()
                .map(|target| (target, NodeState::default())),
        );

        Ok(LoadBalancer { targets, strategy })
    }

    /// Returns a random target from the list of targets.
    pub fn get_target(&self) -> &String {
        match self.strategy {
            Strategy::Random => self.targets.keys().choose(&mut thread_rng()).unwrap(),
            Strategy::GreatestHeight => {
                self.targets
                    .iter()
                    .max_by_key(|(_, height)| *height)
                    .unwrap()
                    .0
            }
        }
    }

    /// Returns the list of targets.
    pub fn get_targets(&self) -> &IndexMap<String, NodeState> {
        &self.targets
    }

    /// Returns the list of healthy targets.
    pub fn get_healthy_targets(&self) -> IndexMap<String, NodeState> {
        self.targets
            .iter()
            .filter(|(_, state)| state.healthy)
            .map(|(target, state)| (target.clone(), state.clone()))
            .collect()
    }

    /// Returns the list of unhealthy targets.
    pub fn get_unhealthy_targets(&self) -> IndexMap<String, NodeState> {
        self.targets
            .iter()
            .filter(|(_, state)| !state.healthy)
            .map(|(target, state)| (target.clone(), state.clone()))
            .collect()
    }

    /// Set the load balancer strategy.
    pub fn set_strategy(&mut self, strategy: Strategy) {
        self.strategy = strategy;
    }

    /// Update target height for a given target.
    pub fn update_target_height(&mut self, target: &str, height: Option<u32>, healthy: bool) {
        if let Some(target_height) = self.targets.get_mut(target) {
            target_height.healthy = healthy;
            if let Some(height) = height {
                target_height.height = height;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_balancer() {
        // Create a list of valid targets.
        let targets = vec![
            "http://localhost:3030".to_string(),
            "http://localhost:3031".to_string(),
            "http://localhost:3032".to_string(),
        ];

        // Create a new load balancer.
        let load_balancer = LoadBalancer::new(targets.clone(), Strategy::Random).unwrap();

        // Recover the targets from the load balancer.
        let recovered_targets = load_balancer
            .get_targets()
            .into_iter()
            .map(|(target, _)| target.clone())
            .collect::<Vec<String>>();

        // Ensure the load balancer has the correct targets.
        assert_eq!(recovered_targets, targets);
    }

    #[test]
    fn test_load_balancer_get_target() {
        // Create a list of valid targets.
        let targets = vec![
            "http://localhost:3030".to_string(),
            "http://localhost:3031".to_string(),
            "http://localhost:3032".to_string(),
        ];

        // Create a new load balancer.
        let load_balancer = LoadBalancer::new(targets.clone(), Strategy::Random).unwrap();

        // Get a target from the load balancer.
        let target = load_balancer.get_target();

        // Ensure the target is in the list of targets.
        assert!(targets.contains(target));
    }

    #[test]
    fn test_load_balancer_get_target_empty() {
        // Create an empty list of targets.
        let targets = vec![];

        // Create a new load balancer.
        let load_balancer = LoadBalancer::new(targets.clone(), Strategy::Random);

        // Ensure the load balancer is fails to create.
        assert!(load_balancer.is_err());
    }

    #[test]
    fn test_load_balancer_get_target_invalid() {
        // Create a list of invalid targets.
        let targets = vec!["localhost:3030".to_string()];

        // Create a new load balancer.
        let load_balancer = LoadBalancer::new(targets.clone(), Strategy::Random);

        // Ensure the load balancer is fails to create.
        assert!(load_balancer.is_err());
    }
}
