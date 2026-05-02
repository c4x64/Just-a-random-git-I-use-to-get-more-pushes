// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Queue Worker
 *
 * Copyright (C) 2025 GNEE Team
 */

//! Queue worker for processing commands.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::queue::command_queue::CommandQueue;

/// Worker configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub name: String,
    pub priority: u8,
    pub sleep_ms: u32,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            name: "worker".to_string(),
            priority: 50,
            sleep_ms: 1,
        }
    }
}

/// Queue worker
pub struct QueueWorker {
    config: WorkerConfig,
    running: Arc<AtomicBool>,
    processed: Arc<AtomicU64>,
}

use std::sync::atomic::AtomicU64;

impl QueueWorker {
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            processed: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn start(&mut self, queue: Arc<CommandQueue>) -> Result<thread::JoinHandle<()>> {
        if self.running.load(Ordering::Acquire) {
            return Err(Error::InvalidState("Worker already running"));
        }

        self.running.store(true, Ordering::Release);
        let running = Arc::clone(&self.running);
        let processed = Arc::clone(&self.processed);
        let config = self.config.clone();

        let handle = thread::spawn(move || {
            while running.load(Ordering::Acquire) {
                match queue.consume() {
                    Ok(_entry) => {
                        processed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(config.sleep_ms as u64));
                    }
                }
            }
        });

        Ok(handle)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    pub fn processed_count(&self) -> u64 {
        self.processed.load(Ordering::Acquire)
    }

    pub fn config(&self) -> &WorkerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_config_default() {
        let config = WorkerConfig::default();
        assert_eq!(config.name, "worker");
    }

    #[test]
    fn test_worker_creation() {
        let config = WorkerConfig::default();
        let worker = QueueWorker::new(config);
        assert!(!worker.is_running());
    }
}
