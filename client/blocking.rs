use super::*;

use crate::call_with_retries;

#[allow(clippy::type_complexity)]
impl<N: Network> AleoRESTClient<N, true> {
    pub fn get(&self, url: &str) -> Result<ureq::Response> {
        call_with_retries!(
            { Ok::<ureq::Response, Error>(self.client.blocking_client()?.get(url).call()?) },
            self.retries
        )
    }

    /// Get the latest block height.
    pub fn latest_height(&self) -> Result<u32> {
        // Prepare the URL.
        let url = format!("{}/{}/latest/height", self.get_endpoint(), self.network_id);

        // Retrieve the latest block height from the network.
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get the latest block hash.
    pub fn latest_hash(&self) -> Result<N::BlockHash> {
        // Prepare the URL.
        let url = format!("{}/{}/latest/hash", self.get_endpoint(), self.network_id);

        // Retrieve the latest block hash from the network.
        debug!("GET latest block hash from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get the latest block.
    pub fn latest_block(&self) -> Result<Block<N>> {
        // Prepare the URL.
        let url = format!("{}/{}/latest/block", self.get_endpoint(), self.network_id);

        // Retrieve the latest block from the network.
        debug!("GET latest block from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get the block matching the specific height from the network.
    pub fn get_block(&self, height: u32) -> Result<Block<N>> {
        // Prepare the URL.
        let url = format!("{}/{}/block/{height}", self.get_endpoint(), self.network_id);

        // Retrieve the block from the network.
        debug!("GET block {height} from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get the block hash matching the specific height from the network.
    pub fn get_block_hash(&self, height: u32) -> Result<N::BlockHash> {
        debug!("GET block hash for block {height}");
        let block = self.get_block(height)?;
        Ok(block.hash())
    }

    /// Get a range of blocks from the network (limited 50 blocks at a time).
    pub fn get_blocks(&self, start_height: u32, end_height: u32) -> Result<Vec<Block<N>>> {
        // Ensure the start height is less than the end height and the range is less than or equal to 50 blocks.
        if start_height >= end_height {
            bail!("Start height must be less than end height");
        } else if end_height - start_height > 50 {
            bail!("Cannot request more than 50 blocks at a time");
        }

        // Prepare the URL.
        let url = format!(
            "{}/{}/blocks?start={start_height}&end={end_height}",
            self.get_endpoint(),
            self.network_id
        );

        // Retrieve the blocks from the network.
        debug!("GET block range {start_height}-{end_height} from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get the epoch number at the latest block height.
    pub fn get_current_epoch(&self) -> Result<(u32, N::BlockHash)> {
        // Get the latest block height.
        let block_height = self.latest_height()?;

        // Get the current epoch for the latest block height.
        self.get_current_epoch_for_height(block_height)
    }

    /// Get the current epoch and block hash.
    pub fn get_current_epoch_for_height(&self, block_height: u32) -> Result<(u32, N::BlockHash)> {
        // Compute the epoch number from the current block height.
        let epoch_number = block_height.saturating_div(N::NUM_BLOCKS_PER_EPOCH);

        // Compute the epoch starting height (a multiple of `NUM_BLOCKS_PER_EPOCH`).
        let epoch_starting_height = (epoch_number).saturating_mul(N::NUM_BLOCKS_PER_EPOCH);

        // If the height is 0, return the default block hash.
        if epoch_starting_height == 0 {
            return Ok((0, N::BlockHash::default()));
        }

        // Ensure the block height is greater than or equal to the epoch starting height.
        ensure!(
            block_height >= epoch_starting_height,
            "Block height is less than the epoch starting height"
        );

        // Retrieve the block height of the block prior to the start of the epoch.
        let epoch_block_hash_number = epoch_starting_height.saturating_sub(1);

        // Retrieve the block hash of the block prior to the start of the epoch.
        let epoch_hash = self.get_block_hash(epoch_block_hash_number)?;
        debug!(
            "Block Height - {block_height} - Epoch starting height: {epoch_starting_height} - Epoch number: {epoch_number} - Epoch block hash: {epoch_hash}"
        );

        // Return the epoch number and epoch hash.
        Ok((epoch_number, epoch_hash))
    }

    /// Retrieve a transaction by via its transaction id
    pub fn get_transaction(&self, transaction_id: &str) -> Result<Transaction<N>> {
        // Prepare the URL.
        let url = format!(
            "{}/{}/transaction/{transaction_id}",
            self.get_endpoint(),
            self.network_id
        )
        .replace('"', "");

        // Retrieve the transaction from the network.
        debug!("GET transaction {transaction_id} from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get pending transactions currently in the mempool.
    pub fn get_memory_pool_transactions(&self) -> Result<Vec<Transaction<N>>> {
        // Prepare the URL.
        let url = format!(
            "{}/{}/memoryPool/transactions",
            self.get_endpoint(),
            self.network_id
        );

        // Retrieve the transactions from the network.
        debug!("GET memory pool transactions from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get a program from the network by its ID. This method will return an error if it does not exist.
    pub fn get_program(&self, program_id: impl TryInto<ProgramID<N>>) -> Result<Program<N>> {
        // Prepare the program ID.
        let program_id = program_id
            .try_into()
            .map_err(|_| anyhow!("Invalid program ID"))?;

        // Perform the request.
        let url = format!(
            "{}/{}/program/{program_id}",
            self.get_endpoint(),
            self.network_id
        );

        // Retrieve the program from the network.
        debug!("GET program {program_id} from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Resolve imports of a program in a depth-first-search order from a program id.
    pub fn get_program_imports(
        &self,
        program_id: impl TryInto<ProgramID<N>>,
    ) -> Result<IndexMap<ProgramID<N>, Program<N>>> {
        // Get the program from the network.
        let program = self.get_program(program_id)?;

        // Recursively resolve imports.
        self.get_program_imports_from_source(&program)
    }

    /// Resolve imports of a program in a depth-first-search order from program source code.
    pub fn get_program_imports_from_source(
        &self,
        program: &Program<N>,
    ) -> Result<IndexMap<ProgramID<N>, Program<N>>> {
        // Initialize a map to store the found imports.
        let mut found_imports = IndexMap::new();

        // Recursively resolve imports.
        for (import_id, _) in program.imports().iter() {
            // Get the imported program from the network.
            let imported_program = self.get_program(import_id)?;

            // Find any nested imports.
            let nested_imports = self.get_program_imports_from_source(&imported_program)?;

            // Check for circular dependencies. If not found, add the imports to the map.
            for (id, import) in nested_imports.into_iter() {
                found_imports
                    .contains_key(&id)
                    .then(|| anyhow!("Circular dependency discovered in program imports"));
                found_imports.insert(id, import);
            }

            // Check to see if the program has already been added to the map. If so there is a circular dependency.
            found_imports
                .contains_key(import_id)
                .then(|| anyhow!("Circular dependency discovered in program imports"));
            found_imports.insert(*import_id, imported_program);
        }

        // Return the found imports.
        Ok(found_imports)
    }

    /// Get all mappings associated with a program.
    pub fn get_program_mappings(
        &self,
        program_id: impl TryInto<ProgramID<N>>,
    ) -> Result<Vec<Identifier<N>>> {
        // Prepare the program ID.
        let program_id = program_id
            .try_into()
            .map_err(|_| anyhow!("Invalid program ID"))?;

        // Prepare the URL.
        let url = format!(
            "{}/{}/program/{program_id}/mappings",
            self.get_endpoint(),
            self.network_id
        );

        // Get the mappings for the program.
        debug!("GET mappings for program {program_id} from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get the current value of a mapping given a specific program, mapping name, and mapping key.
    pub fn get_mapping_value(
        &self,
        program_id: impl TryInto<ProgramID<N>>,
        mapping_name: impl TryInto<Identifier<N>>,
        key: impl TryInto<Plaintext<N>>,
    ) -> Result<Value<N>> {
        // Prepare the program ID.
        let program_id = program_id
            .try_into()
            .map_err(|_| anyhow!("Invalid program ID"))?;

        // Prepare the mapping name.
        let mapping_name = mapping_name
            .try_into()
            .map_err(|_| anyhow!("Invalid mapping name"))?;

        // Prepare the key.
        let key = key.try_into().map_err(|_| anyhow!("Invalid key"))?;

        // Prepare the URL.
        let url = format!(
            "{}/{}/program/{program_id}/mapping/{mapping_name}/{key}",
            self.get_endpoint(),
            self.network_id
        );

        // Retrieve the mapping value if it exists.
        debug!("GET mapping {mapping_name} for program {program_id} with key {key} from {url}");
        Ok(self.get(&url)?.into_json()?)
    }

    /// Get the block hash for a given transaction ID.
    pub fn find_block_hash(&self, transaction_id: N::TransactionID) -> Result<N::BlockHash> {
        // Prepare the URL.
        let url = format!(
            "{}/{}/find/blockHash/{transaction_id}",
            self.get_endpoint(),
            self.network_id
        );

        // Retrieve the block hash from the network.
        Ok(self.get(&url)?.into_json()?)
    }

    /// Returns the transition ID that contains the given `input ID` or `output ID`.
    pub fn find_transition_id(&self, input_or_output_id: Field<N>) -> Result<N::TransitionID> {
        // Prepare the URL.
        let url = format!(
            "{}/{}/find/transitionID/{input_or_output_id}",
            self.get_endpoint(),
            self.network_id
        );

        // Retrieve the transition ID from the network.
        Ok(self.get(&url)?.into_json()?)
    }

    /// Scans the ledger for records that match the given view key.
    pub fn scan(
        &self,
        view_key: impl TryInto<ViewKey<N>>,
        block_heights: Range<u32>,
        max_records: Option<usize>,
    ) -> Result<Vec<(Field<N>, Record<N, Ciphertext<N>>)>> {
        // Prepare the view key.
        let view_key = view_key
            .try_into()
            .map_err(|_| anyhow!("Invalid view key"))?;
        // Compute the x-coordinate of the address.
        let address_x_coordinate = view_key.to_address().to_x_coordinate();

        // Prepare the starting block height, by rounding down to the nearest step of 50.
        let start_block_height = block_heights.start - (block_heights.start % 50);
        // Prepare the ending block height, by rounding up to the nearest step of 50.
        let end_block_height = block_heights.end + (50 - (block_heights.end % 50));

        // Initialize a vector for the records.
        let mut records = Vec::new();

        for start_height in (start_block_height..end_block_height).step_by(50) {
            debug!(
                "Searching blocks {} to {} for records...",
                start_height, end_block_height
            );
            if start_height >= block_heights.end {
                break;
            }
            let end = start_height + 50;
            let end_height = if end > block_heights.end {
                block_heights.end
            } else {
                end
            };

            // Prepare the URL.
            let records_iter = self
                .get_blocks(start_height, end_height)?
                .into_iter()
                .flat_map(|block| block.into_records());

            // Filter the records by the view key.
            records.extend(records_iter.filter_map(|(commitment, record)| {
                match record.is_owner_with_address_x_coordinate(&view_key, &address_x_coordinate) {
                    true => Some((commitment, record)),
                    false => None,
                }
            }));

            if records.len() >= max_records.unwrap_or(usize::MAX) {
                break;
            }
        }

        Ok(records)
    }

    // POST METHODS //
    /// Submit a solution to the Aleo network according to the load balancing strategy.
    pub fn unicast_solution(&self, solution: &Solution<N>) -> Result<String> {
        // Get the endpoint if specified, otherwise select a random endpoint.
        let endpoint = self.get_endpoint();

        // Prepare the URL.
        let url = format!("{}/{}/solution/broadcast", endpoint, self.network_id);

        // Send the solution to the network.
        self.post_solution(solution, &url)
    }

    /// Submit a solution to the Aleo network via a specific URL.
    pub fn post_solution(&self, solution: &Solution<N>, url: &str) -> Result<String> {
        // Send the solution to the network.
        call_with_retries!(
            {
                match self.client.blocking_client()?.post(url).send_json(solution) {
                    Ok(response) => match response.into_string() {
                        Ok(success_response) => Ok(success_response),
                        Err(error) => Err(anyhow!("❌ Solution response was malformed {}", error)),
                    },
                    Err(error) => Err(anyhow!(
                        "❌ Failed to broadcast solution to {}: {}",
                        url,
                        error
                    )),
                }
            },
            self.retries
        )
    }

    /// Submit a transaction to the Aleo network according to the load balancing strategy.
    pub fn unicast_transaction(&self, transaction: &Transaction<N>) -> Result<String> {
        // Prepare the URL.
        let url = format!("{}/{}/deploy", self.get_endpoint(), self.network_id);

        // Send the deploy to the network.
        self.post_transaction(transaction, &url)
    }

    /// Submit a transaction to the Aleo network via a specific URL.
    pub fn post_transaction(&self, transaction: &Transaction<N>, url: &str) -> Result<String> {
        // Send the transaction to the network.
        call_with_retries!(
            {
                match self
                    .client
                    .blocking_client()?
                    .post(url)
                    .send_json(transaction)
                {
                    Ok(response) => match response.into_string() {
                        Ok(success_response) => Ok(success_response),
                        Err(error) => {
                            Err(anyhow!("❌ Transaction response was malformed {}", error))
                        }
                    },
                    Err(error) => {
                        let error_message = match error {
                            ureq::Error::Status(code, response) => {
                                format!("(status code {code}: {:?})", response.into_string()?)
                            }
                            ureq::Error::Transport(err) => format!("({err})"),
                        };

                        match transaction {
                            Transaction::Deploy(..) => Err(anyhow!(
                                "❌ Failed to deploy program to {}: {}",
                                url,
                                error_message
                            )),
                            Transaction::Execute(..) => Err(anyhow!(
                                "❌ Failed to broadcast execution to {}: {}",
                                url,
                                error_message
                            )),
                            Transaction::Fee(..) => Err(anyhow!(
                                "❌ Failed to broadcast fee execution to {}: {}",
                                url,
                                error_message
                            )),
                        }
                    }
                }
            },
            self.retries
        )
    }
}

// Health check methods.
impl<N: Network> AleoRESTClient<N, true> {
    /// Run a health check for an endpoint and update the internal state.
    pub fn health_check(client: Arc<AleoRESTClient<N, true>>, endpoint: String) -> Result<bool> {
        // Retrieve the latest block height from the network.
        let height = client.latest_height();

        // Get the load balancer from the client.
        let load_balancer = client.load_balancer.clone();

        // Update the target height based on the health check result.
        match height {
            Ok(height) => {
                debug!("Health check for {endpoint} updating node to {height}");
                load_balancer
                    .write()
                    .update_target_height(&endpoint, Some(height), true);
                Ok(true)
            }
            Err(e) => {
                warn!("Health check for {endpoint} failed with error {e}, marking as unhealthy");
                load_balancer
                    .write()
                    .update_target_height(&endpoint, None, false);
                Ok(false)
            }
        }
    }

    /// Run a health check for all endpoints and update internal state.
    pub fn run_health_checks(&self) {
        // Get all endpoints from the load balancer.
        let endpoints = self
            .load_balancer
            .read()
            .get_targets()
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        let client = Arc::new(self.clone());

        // Run health checks for all endpoints concurrently.
        for endpoint in endpoints {
            debug!("Running health check for {endpoint}");
            let client_ = client.clone();

            rayon::spawn(move || {
                let _ = Self::health_check(client_, endpoint);
            });
        }
    }

    /// Start the health check loop.
    pub fn start_health_checks(&self, interval: Duration) {
        // Run the health check loop.
        loop {
            debug!("Running health checks...");
            // Run all health checks.
            self.run_health_checks();

            // Wait for the interval before running the health checks again.
            std::thread::sleep(interval);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use snarkvm::prelude::MainnetV0;

    #[test]
    fn test_blocking_health_check_registers_unhealthy() {
        // Initialize the Aleo REST client with two unresponsive endpoints.
        let client = AleoRESTClient::<MainnetV0, true>::from_targets(vec![
            "http://localhost:3030".to_string(),
            "http://localhost:3031".to_string(),
        ])
        .unwrap();

        // Start the health checks.
        let client_ = client.clone();
        rayon::spawn(move || client_.clone().start_health_checks(Duration::from_secs(30)));

        // Sleep to allow the health checks to run through several retries.
        std::thread::sleep(Duration::from_secs(40));

        // Get all targets from the load balancer.
        let all_targets = client.get_targets();

        // Get the healthy targets from the load balancer.
        let healthy_targets = client.get_healthy_targets();

        // Get the unhealthy targets from the load balancer.
        let unhealthy_targets = client.get_unhealthy_targets();

        // Ensure that all targets are marked as unhealthy.
        assert_eq!(all_targets.len(), unhealthy_targets.len());

        // Ensure that there are no healthy targets.
        assert_eq!(healthy_targets.len(), 0);
    }
}
