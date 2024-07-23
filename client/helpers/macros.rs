// Retry logic for the Aleo API client.
#[macro_export]
macro_rules! call_with_retries {
    ($block:block, $retries:expr) => {{
        let mut retries = 0;
        loop {
            match $block {
                Ok(result) => break Ok(result),
                Err(e) => {
                    retries += 1;
                    std::thread::sleep(Duration::from_secs(1));
                    if retries >= $retries {
                        break Err(anyhow!("{e:?}"));
                    }
                }
            }
        }
    }};
}

// Retry logic for the Aleo API client.
#[macro_export]
macro_rules! async_call_with_retries {
    ($block:block, $retries:expr) => {{
        let mut retries = 0;
        loop {
            match $block {
                Ok(result) => break Ok(result),
                Err(e) => {
                    retries += 1;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if retries >= $retries {
                        break Err(anyhow!("{e:?}"));
                    }
                }
            }
        }
    }};
}
