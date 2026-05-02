// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Command Queue
 *
 * Copyright (C) 2025 GNEE Team
 *
 * Lock-free command queue for GPU command submission.
 */

//! # Command Queue
//!
//! Provides lock-free command queue for efficient GPU command submission.

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::mem;

use crate::error::{Error, Result};

/// Command queue configuration
#[derive(Debug, Clone)]
pub struct CommandQueueConfig {
    /// Queue size (must be power of 2)
    pub size: usize,
    /// Enable producer-consumer mode
    pub producer_consumer: bool,
    /// Enable batching
    pub batching: bool,
    /// Batch size
    pub batch_size: usize,
}

impl Default for CommandQueueConfig {
    fn default() -> Self {
        Self {
            size: 4096,
            producer_consumer: true,
            batching: true,
            batch_size: 16,
        }
    }
}

/// Command queue entry
#[derive(Debug, Clone)]
pub struct CommandQueueEntry {
    /// Command data
    pub data: Vec<u8>,
    /// Command type
    pub type_: CommandType,
    /// Sequence number
    pub seqnum: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Command type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    /// Draw command
    Draw,
    /// Dispatch command
    Dispatch,
    /// Copy command
    Copy,
    /// Barrier command
    Barrier,
    /// Present command
    Present,
    /// Custom command
    Custom,
}

/// Lock-free command queue
pub struct CommandQueue {
    /// Ring buffer
    buffer: Vec<Option<CommandQueueEntry>>,
    /// Buffer size
    size: usize,
    /// Buffer mask
    mask: usize,
    
    /// Producer head
    prod_head: Arc<AtomicU64>,
    /// Producer tail
    prod_tail: Arc<AtomicU64>,
    /// Consumer head
    cons_head: Arc<AtomicU64>,
    /// Consumer tail
    cons_tail: Arc<AtomicU64>,
    
    /// Configuration
    config: CommandQueueConfig,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Statistics
    stats: CommandQueueStats,
}

/// Command queue statistics
#[derive(Debug, Default)]
pub struct CommandQueueStats {
    /// Total commands submitted
    pub total_submitted: Arc<AtomicU64>,
    /// Total commands consumed
    pub total_consumed: Arc<AtomicU64>,
    /// Total commands dropped
    pub total_dropped: Arc<AtomicU64>,
    /// Average queue depth
    pub avg_depth: Arc<AtomicU64>,
    /// Peak queue depth
    pub peak_depth: Arc<AtomicU64>,
}

impl CommandQueue {
    /// Create a new command queue
    pub fn new(config: CommandQueueConfig) -> Result<Self> {
        // Ensure size is power of 2
        let size = config.size.next_power_of_two();
        let mask = size - 1;

        // Initialize buffer
        let buffer = vec![None; size];

        Ok(Self {
            buffer,
            size,
            mask,
            prod_head: Arc::new(AtomicU64::new(0)),
            prod_tail: Arc::new(AtomicU64::new(0)),
            cons_head: Arc::new(AtomicU64::new(0)),
            cons_tail: Arc::new(AtomicU64::new(0)),
            config,
            running: Arc::new(AtomicBool::new(true)),
            stats: CommandQueueStats::default(),
        })
    }

    /// Submit a command (producer)
    pub fn submit(&self, entry: CommandQueueEntry) -> Result<()> {
        if !self.running.load(Ordering::Acquire) {
            return Err(Error::InvalidState("Queue not running"));
        }

        // Reserve slot
        let prod_head = self.prod_head.load(Ordering::Acquire);
        let prod_next = prod_head + 1;

        // Check if queue is full
        if prod_next - self.cons_tail.load(Ordering::Acquire) > self.size as u64 {
            self.stats.total_dropped.fetch_add(1, Ordering::Relaxed);
            return Err(Error::Busy);
        }

        // Update producer head
        if self.prod_head.compare_exchange_weak(
            prod_head,
            prod_next,
            Ordering::AcqRel,
            Ordering::Acquire,
        ).is_err() {
            return Err(Error::Busy);
        }

        // Write entry
        let index = (prod_head & self.mask as u64) as usize;
        self.buffer[index] = Some(entry);

        // Memory barrier before updating tail
        std::sync::atomic::fence(Ordering::Release);

        // Update producer tail
        self.prod_tail.store(prod_next, Ordering::Release);

        // Update statistics
        self.stats.total_submitted.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Consume a command (consumer)
    pub fn consume(&self) -> Result<CommandQueueEntry> {
        if !self.running.load(Ordering::Acquire) {
            return Err(Error::InvalidState("Queue not running"));
        }

        // Check if queue is empty
        let cons_head = self.cons_head.load(Ordering::Acquire);
        if cons_head == self.prod_tail.load(Ordering::Acquire) {
            return Err(Error::Busy);
        }

        // Reserve slot
        let cons_next = cons_head + 1;
        self.cons_head.store(cons_next, Ordering::Release);

        // Read entry
        let index = (cons_head & self.mask as u64) as usize;
        let entry = self.buffer[index].take()
            .ok_or(Error::InvalidState("No entry"))?;

        // Memory barrier before updating tail
        std::sync::atomic::fence(Ordering::Acquire);

        // Update consumer tail
        self.cons_tail.store(cons_next, Ordering::Release);

        // Update statistics
        self.stats.total_consumed.fetch_add(1, Ordering::Relaxed);

        Ok(entry)
    }

    /// Check if queue has entries
    pub fn has_entries(&self) -> bool {
        let cons_head = self.cons_head.load(Ordering::Acquire);
        let prod_tail = self.prod_tail.load(Ordering::Acquire);
        cons_head != prod_tail
    }

    /// Get queue depth
    pub fn depth(&self) -> usize {
        let prod_tail = self.prod_tail.load(Ordering::Acquire);
        let cons_head = self.cons_head.load(Ordering::Acquire);
        (prod_tail - cons_head) as usize
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.size
    }

    /// Get statistics
    pub fn stats(&self) -> &CommandQueueStats {
        &self.stats
    }

    /// Stop the queue
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Check if queue is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Reset the queue
    pub fn reset(&self) {
        self.prod_head.store(0, Ordering::Release);
        self.prod_tail.store(0, Ordering::Release);
        self.cons_head.store(0, Ordering::Release);
        self.cons_tail.store(0, Ordering::Release);
    }

    /// Get configuration
    pub fn config(&self) -> &CommandQueueConfig {
        &self.config
    }
}

/// Batched command queue
pub struct BatchedCommandQueue {
    /// Inner command queue
    inner: CommandQueue,
    /// Current batch
    current_batch: Vec<CommandQueueEntry>,
    /// Batch size
    batch_size: usize,
}

impl BatchedCommandQueue {
    /// Create a new batched command queue
    pub fn new(config: CommandQueueConfig) -> Result<Self> {
        let batch_size = config.batch_size;
        let inner = CommandQueue::new(config)?;

        Ok(Self {
            inner,
            current_batch: Vec::with_capacity(batch_size),
            batch_size,
        })
    }

    /// Submit a command (adds to current batch)
    pub fn submit(&mut self, entry: CommandQueueEntry) -> Result<()> {
        self.current_batch.push(entry);

        // Flush batch if full
        if self.current_batch.len() >= self.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    /// Flush current batch
    pub fn flush(&mut self) -> Result<()> {
        if self.current_batch.is_empty() {
            return Ok(());
        }

        // Create batch entry
        let batch_entry = CommandQueueEntry {
            data: self.serialize_batch(&self.current_batch),
            type_: CommandType::Custom,
            seqnum: self.inner.stats.total_submitted.load(Ordering::Acquire),
            timestamp: self.get_timestamp(),
        };

        // Submit batch
        self.inner.submit(batch_entry)?;

        // Clear current batch
        self.current_batch.clear();

        Ok(())
    }

    /// Serialize batch
    fn serialize_batch(&self, batch: &[CommandQueueEntry]) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Write batch count
        data.extend_from_slice(&(batch.len() as u32).to_le_bytes());
        
        // Write each entry
        for entry in batch {
            // Write entry type
            data.extend_from_slice(&(entry.type_ as u32).to_le_bytes());
            // Write entry size
            data.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            // Write entry data
            data.extend_from_slice(&entry.data);
        }

        data
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Get inner queue statistics
    pub fn stats(&self) -> &CommandQueueStats {
        self.inner.stats()
    }

    /// Get current batch size
    pub fn current_batch_size(&self) -> usize {
        self.current_batch.len()
    }

    /// Check if queue is running
    pub fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    /// Stop the queue
    pub fn stop(&self) {
        self.inner.stop();
    }
}

/// Multi-producer, single-consumer queue
pub struct MpscCommandQueue {
    /// Inner command queue
    inner: CommandQueue,
}

impl MpscCommandQueue {
    /// Create a new MPSC queue
    pub fn new(config: CommandQueueConfig) -> Result<Self> {
        let inner = CommandQueue::new(config)?;
        Ok(Self { inner })
    }

    /// Submit a command (producer)
    pub fn submit(&self, entry: CommandQueueEntry) -> Result<()> {
        self.inner.submit(entry)
    }

    /// Consume a command (consumer)
    pub fn consume(&self) -> Result<CommandQueueEntry> {
        self.inner.consume()
    }

    /// Get statistics
    pub fn stats(&self) -> &CommandQueueStats {
        self.inner.stats()
    }
}

/// Single-producer, multi-consumer queue
pub struct SpmcCommandQueue {
    /// Inner command queue
    inner: CommandQueue,
}

impl SpmcCommandQueue {
    /// Create a new SPMC queue
    pub fn new(config: CommandQueueConfig) -> Result<Self> {
        let inner = CommandQueue::new(config)?;
        Ok(Self { inner })
    }

    /// Submit a command (producer)
    pub fn submit(&self, entry: CommandQueueEntry) -> Result<()> {
        self.inner.submit(entry)
    }

    /// Consume a command (consumer)
    pub fn consume(&self) -> Result<CommandQueueEntry> {
        self.inner.consume()
    }

    /// Get statistics
    pub fn stats(&self) -> &CommandQueueStats {
        self.inner.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_queue_config_default() {
        let config = CommandQueueConfig::default();
        assert_eq!(config.size, 4096);
        assert!(config.producer_consumer);
        assert!(config.batching);
    }

    #[test]
    fn test_command_queue_creation() {
        let config = CommandQueueConfig::default();
        let queue = CommandQueue::new(config);
        assert!(queue.is_ok());
    }

    #[test]
    fn test_command_queue_submit_consume() {
        let config = CommandQueueConfig::default();
        let queue = CommandQueue::new(config).unwrap();

        let entry = CommandQueueEntry {
            data: vec![1, 2, 3, 4],
            type_: CommandType::Draw,
            seqnum: 0,
            timestamp: 0,
        };

        queue.submit(entry).unwrap();
        assert!(queue.has_entries());

        let consumed = queue.consume().unwrap();
        assert_eq!(consumed.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_command_queue_depth() {
        let config = CommandQueueConfig::default();
        let queue = CommandQueue::new(config).unwrap();

        assert_eq!(queue.depth(), 0);

        let entry = CommandQueueEntry {
            data: vec![1, 2, 3, 4],
            type_: CommandType::Draw,
            seqnum: 0,
            timestamp: 0,
        };

        queue.submit(entry.clone()).unwrap();
        assert_eq!(queue.depth(), 1);

        queue.consume().unwrap();
        assert_eq!(queue.depth(), 0);
    }

    #[test]
    fn test_batched_command_queue() {
        let config = CommandQueueConfig {
            batch_size: 4,
            ..Default::default()
        };

        let mut queue = BatchedCommandQueue::new(config).unwrap();

        let entry = CommandQueueEntry {
            data: vec![1, 2, 3, 4],
            type_: CommandType::Draw,
            seqnum: 0,
            timestamp: 0,
        };

        queue.submit(entry.clone()).unwrap();
        assert_eq!(queue.current_batch_size(), 1);

        queue.submit(entry.clone()).unwrap();
        queue.submit(entry.clone()).unwrap();
        queue.submit(entry.clone()).unwrap();
        
        // Should have flushed
        assert_eq!(queue.current_batch_size(), 0);
    }
}
