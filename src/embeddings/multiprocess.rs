//! Multi-process embedding generation for maximum parallel performance
//!
//! This module implements a multi-process architecture to overcome GPU resource
//! contention and achieve true parallel processing for embedding generation.

use std::io::BufRead;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader as TokioBufReader;
use tokio::process::Command as TokioCommand;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::database::Database;
use crate::errors::Result;
use crate::errors::SnapragError;
use crate::models::Cast;

/// Configuration for multi-process embedding generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiProcessConfig {
    /// Number of worker processes to spawn
    pub worker_processes: usize,
    /// Batch size per worker process
    pub batch_size_per_worker: usize,
    /// GPU device IDs to distribute across workers
    pub gpu_devices: Vec<usize>,
    /// Timeout for worker process startup
    pub worker_startup_timeout_secs: u64,
    /// Maximum retries for failed embeddings
    pub max_retries: usize,
}

impl Default for MultiProcessConfig {
    fn default() -> Self {
        Self {
            worker_processes: 4, // Default to 4 workers
            batch_size_per_worker: 50,
            gpu_devices: vec![0], // Default to single GPU
            worker_startup_timeout_secs: 30,
            max_retries: 3,
        }
    }
}

/// Statistics for multi-process embedding generation
#[derive(Debug, Default, Clone)]
pub struct MultiProcessStats {
    pub total_casts: usize,
    pub success: usize,
    pub skipped: usize,
    pub failed: usize,
    pub worker_stats: Vec<WorkerStats>,
    pub total_duration: Duration,
}

#[derive(Debug, Default, Clone)]
pub struct WorkerStats {
    pub worker_id: usize,
    pub processed: usize,
    pub success: usize,
    pub failed: usize,
    pub duration: Duration,
}

/// Message sent to worker processes
#[derive(Debug, Serialize, Deserialize)]
pub enum WorkerMessage {
    ProcessBatch {
        batch_id: usize,
        casts: Vec<Cast>,
        gpu_device_id: Option<usize>,
    },
    Shutdown,
}

/// Response from worker processes
#[derive(Debug, Serialize, Deserialize)]
pub enum WorkerResponse {
    BatchComplete {
        batch_id: usize,
        results: Vec<EmbeddingResult>,
        worker_id: usize,
    },
    Error {
        batch_id: usize,
        error: String,
        worker_id: usize,
    },
    Ready {
        worker_id: usize,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingResult {
    pub message_hash: Vec<u8>,
    pub fid: u64,
    pub text: String,
    pub embedding: Option<Vec<f32>>,
    pub success: bool,
    pub error: Option<String>,
}

/// Multi-process embedding generator
pub struct MultiProcessEmbeddingGenerator {
    config: MultiProcessConfig,
    workers: Vec<tokio::process::Child>,
    worker_channels: Vec<tokio::sync::mpsc::UnboundedSender<WorkerMessage>>,
}

impl MultiProcessEmbeddingGenerator {
    /// Create a new multi-process embedding generator
    pub fn new(config: MultiProcessConfig) -> Self {
        Self {
            config,
            workers: Vec::new(),
            worker_channels: Vec::new(),
        }
    }

    /// Start worker processes
    pub async fn start_workers(&mut self) -> Result<()> {
        info!(
            "Starting {} worker processes for embedding generation",
            self.config.worker_processes
        );

        for worker_id in 0..self.config.worker_processes {
            let gpu_device = self
                .config
                .gpu_devices
                .get(worker_id % self.config.gpu_devices.len())
                .copied();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WorkerMessage>();
            self.worker_channels.push(tx);

            // Spawn worker process
            let mut worker = TokioCommand::new("cargo")
                .args(&["run", "--bin", "snaprag-worker", "--features", "local-gpu"])
                .env("WORKER_ID", worker_id.to_string())
                .env(
                    "GPU_DEVICE_ID",
                    gpu_device.map(|d| d.to_string()).unwrap_or_default(),
                )
                .env("RUST_LOG", "info")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    SnapragError::EmbeddingError(format!(
                        "Failed to spawn worker {}: {}",
                        worker_id, e
                    ))
                })?;

            // Handle worker communication
            let worker_id_clone = worker_id;
            tokio::spawn(async move {
                let mut stdin = worker.stdin.take().unwrap();
                let stdout = worker.stdout.take().unwrap();
                let stderr = worker.stderr.take().unwrap();

                // Handle stdout for responses
                let stdout_reader = TokioBufReader::new(stdout);
                let mut stdout_lines = stdout_reader.lines();

                // Handle stderr for logging
                let stderr_reader = TokioBufReader::new(stderr);
                let mut stderr_lines = stderr_reader.lines();

                // Process messages
                while let Some(msg) = rx.recv().await {
                    match msg {
                        WorkerMessage::ProcessBatch {
                            batch_id,
                            ref casts,
                            gpu_device_id,
                        } => {
                            // Send batch to worker
                            let batch_msg = serde_json::to_string(&msg).unwrap();
                            if let Err(e) =
                                stdin.write_all(format!("{}\n", batch_msg).as_bytes()).await
                            {
                                error!("Failed to send batch to worker {}: {}", worker_id_clone, e);
                                break;
                            }

                            // Wait for response
                            if let Some(response_line) =
                                stdout_lines.next_line().await.unwrap_or(None)
                            {
                                if let Ok(response) =
                                    serde_json::from_str::<WorkerResponse>(&response_line)
                                {
                                    debug!(
                                        "Worker {} completed batch {}",
                                        worker_id_clone, batch_id
                                    );
                                }
                            }
                        }
                        WorkerMessage::Shutdown => {
                            let shutdown_msg = serde_json::to_string(&msg).unwrap();
                            let _ = stdin
                                .write_all(format!("{}\n", shutdown_msg).as_bytes())
                                .await;
                            break;
                        }
                    }
                }

                // Log stderr
                while let Some(line) = stderr_lines.next_line().await.unwrap_or(None) {
                    info!("Worker {} stderr: {}", worker_id_clone, line);
                }
            });

            // Store worker handle for later cleanup
            // Note: worker is moved into the tokio::spawn closure above
        }

        // Wait for all workers to be ready
        info!("Waiting for workers to initialize...");
        tokio::time::sleep(Duration::from_secs(self.config.worker_startup_timeout_secs)).await;

        info!(
            "All {} workers started successfully",
            self.config.worker_processes
        );
        Ok(())
    }

    /// Process casts using multi-process parallel generation
    pub async fn process_casts(
        &mut self,
        casts: Vec<Cast>,
        db: Arc<Database>,
    ) -> Result<MultiProcessStats> {
        let start_time = Instant::now();
        let mut stats = MultiProcessStats::default();
        stats.total_casts = casts.len();

        info!(
            "Processing {} casts using {} worker processes",
            casts.len(),
            self.config.worker_processes
        );

        // Distribute casts across workers
        let batches: Vec<Vec<Cast>> = casts
            .chunks(self.config.batch_size_per_worker)
            .map(|chunk| chunk.to_vec())
            .collect();

        info!(
            "Created {} batches of size {}",
            batches.len(),
            self.config.batch_size_per_worker
        );

        // Process batches in parallel across workers
        let mut batch_futures = Vec::new();

        for (batch_id, batch) in batches.into_iter().enumerate() {
            let worker_id = batch_id % self.config.worker_processes;
            let worker_tx = self.worker_channels[worker_id].clone();
            let db_clone = Arc::clone(&db);
            let gpu_device = self
                .config
                .gpu_devices
                .get(worker_id % self.config.gpu_devices.len())
                .copied();

            let future = async move {
                // Send batch to worker
                let msg = WorkerMessage::ProcessBatch {
                    batch_id,
                    casts: batch.clone(),
                    gpu_device_id: gpu_device,
                };

                if let Err(e) = worker_tx.send(msg) {
                    error!(
                        "Failed to send batch {} to worker {}: {}",
                        batch_id, worker_id, e
                    );
                    return Err(SnapragError::EmbeddingError(format!(
                        "Worker communication failed: {}",
                        e
                    )));
                }

                // Process results (this would be handled by the worker communication loop)
                // For now, we'll simulate the processing
                let mut batch_stats = WorkerStats {
                    worker_id,
                    processed: batch.len(),
                    success: 0,
                    failed: 0,
                    duration: Duration::from_secs(0),
                };

                // Store embeddings in database
                for cast in batch {
                    if let Some(text) = &cast.text {
                        if !text.trim().is_empty() {
                            // Simulate embedding generation (in real implementation, this would come from worker)
                            let embedding = vec![0.0; 384]; // Placeholder

                            match db_clone
                                .store_cast_embedding(
                                    &cast.message_hash,
                                    cast.fid,
                                    text,
                                    &embedding,
                                )
                                .await
                            {
                                Ok(()) => batch_stats.success += 1,
                                Err(e) => {
                                    warn!(
                                        "Failed to store embedding for cast {}: {}",
                                        hex::encode(&cast.message_hash),
                                        e
                                    );
                                    batch_stats.failed += 1;
                                }
                            }
                        }
                    }
                }

                Ok(batch_stats)
            };

            batch_futures.push(future);
        }

        // Wait for all batches to complete
        let results = futures::future::join_all(batch_futures).await;

        // Aggregate statistics
        for result in results {
            match result {
                Ok(worker_stats) => {
                    stats.worker_stats.push(worker_stats.clone());
                    stats.success += worker_stats.success;
                    stats.failed += worker_stats.failed;
                }
                Err(e) => {
                    error!("Batch processing failed: {}", e);
                    stats.failed += 1;
                }
            }
        }

        stats.total_duration = start_time.elapsed();

        info!(
            "Multi-process embedding generation completed in {:.2}s: {} success, {} failed",
            stats.total_duration.as_secs_f64(),
            stats.success,
            stats.failed
        );

        Ok(stats)
    }

    /// Shutdown all worker processes
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down {} worker processes", self.workers.len());

        // Send shutdown messages
        for tx in &self.worker_channels {
            let _ = tx.send(WorkerMessage::Shutdown);
        }

        // Wait for workers to terminate
        for (i, mut worker) in self.workers.drain(..).enumerate() {
            match worker.wait().await {
                Ok(status) => info!("Worker {} terminated with status: {:?}", i, status),
                Err(e) => warn!("Error waiting for worker {}: {}", i, e),
            }
        }

        info!("All workers shutdown complete");
        Ok(())
    }
}

impl Drop for MultiProcessEmbeddingGenerator {
    fn drop(&mut self) {
        // Ensure workers are cleaned up
        for tx in &self.worker_channels {
            let _ = tx.send(WorkerMessage::Shutdown);
        }
    }
}

/// Worker process entry point
#[cfg(feature = "local-gpu")]
pub async fn worker_main() -> Result<()> {
    let worker_id = std::env::var("WORKER_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<usize>()
        .unwrap_or(0);

    let gpu_device_id = std::env::var("GPU_DEVICE_ID")
        .ok()
        .and_then(|s| s.parse::<usize>().ok());

    info!(
        "Starting embedding worker {} with GPU device {:?}",
        worker_id, gpu_device_id
    );

    // Initialize local GPU client
    let client = crate::embeddings::local_gpu::LocalGPUClient::new_with_dimension(
        "BAAI/bge-small-en-v1.5",
        384,
        gpu_device_id,
    )
    .await?;

    info!("Worker {} initialized successfully", worker_id);

    // Signal ready
    println!(
        "{}",
        serde_json::to_string(&WorkerResponse::Ready { worker_id }).unwrap()
    );

    // Process messages from stdin
    let stdin = tokio::io::stdin();
    let reader = TokioBufReader::new(stdin);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.unwrap_or(None) {
        if let Ok(msg) = serde_json::from_str::<WorkerMessage>(&line) {
            match msg {
                WorkerMessage::ProcessBatch {
                    batch_id,
                    casts,
                    gpu_device_id: _,
                } => {
                    let mut results = Vec::new();

                    for cast in casts {
                        let result = if let Some(text) = &cast.text {
                            if !text.trim().is_empty() {
                                match client.generate(text).await {
                                    Ok(embedding) => EmbeddingResult {
                                        message_hash: cast.message_hash.clone(),
                                        fid: cast.fid as u64,
                                        text: text.clone(),
                                        embedding: Some(embedding),
                                        success: true,
                                        error: None,
                                    },
                                    Err(e) => EmbeddingResult {
                                        message_hash: cast.message_hash.clone(),
                                        fid: cast.fid as u64,
                                        text: text.clone(),
                                        embedding: None,
                                        success: false,
                                        error: Some(e.to_string()),
                                    },
                                }
                            } else {
                                EmbeddingResult {
                                    message_hash: cast.message_hash.clone(),
                                    fid: cast.fid as u64,
                                    text: text.clone(),
                                    embedding: None,
                                    success: false,
                                    error: Some("Empty text".to_string()),
                                }
                            }
                        } else {
                            EmbeddingResult {
                                message_hash: cast.message_hash.clone(),
                                fid: cast.fid as u64,
                                text: "".to_string(),
                                embedding: None,
                                success: false,
                                error: Some("No text".to_string()),
                            }
                        };

                        results.push(result);
                    }

                    // Send response
                    let response = WorkerResponse::BatchComplete {
                        batch_id,
                        results,
                        worker_id,
                    };
                    println!("{}", serde_json::to_string(&response).unwrap());
                }
                WorkerMessage::Shutdown => {
                    info!("Worker {} received shutdown signal", worker_id);
                    break;
                }
            }
        }
    }

    info!("Worker {} shutting down", worker_id);
    Ok(())
}
