//! SnapRAG Worker Process
//!
//! This binary is used as a worker process for multi-process embedding generation.
//! It communicates with the main process via stdin/stdout using JSON messages.

#[cfg(feature = "local-gpu")]
use snaprag::embeddings::multiprocess::worker_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "local-gpu")]
    {
        worker_main().await?;
    }
    
    #[cfg(not(feature = "local-gpu"))]
    {
        eprintln!("Local GPU feature not enabled. This worker requires the 'local-gpu' feature.");
        std::process::exit(1);
    }
    
    Ok(())
}
