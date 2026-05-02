// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Synchronization
 *
 * Copyright (C) 2025 GNEE Team
 *
 * Synchronization primitives for multi-threaded execution.
 */

//! # Synchronization
//!
//! Provides synchronization primitives for multi-threaded execution.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::error::{Error, Result};

/// Fence
#[derive(Debug, Clone)]
pub struct Fence {
    /// Fence ID
    pub id: u64,
    /// Signaled flag
    pub signaled: Arc<AtomicBool>,
    /// Timestamp
    pub timestamp: Arc<AtomicU64>,
}

impl Fence {
    /// Create a new fence
    pub fn new() -> Self {
        Self {
            id: rand::random::<u64>(),
            signaled: Arc::new(AtomicBool::new(false)),
            timestamp: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new fence with specific ID
    pub fn with_id(id: u64) -> Self {
        Self {
            id,
            signaled: Arc::new(AtomicBool::new(false)),
            timestamp: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Signal the fence
    pub fn signal(&self) {
        self.signaled.store(true, Ordering::Release);
        self.timestamp.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::Release
        );
    }

    /// Reset the fence
    pub fn reset(&self) {
        self.signaled.store(false, Ordering::Release);
    }

    /// Check if fence is signaled
    pub fn is_signaled(&self) -> bool {
        self.signaled.load(Ordering::Acquire)
    }

    /// Wait for fence to be signaled
    pub fn wait(&self, timeout_ms: u32) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms as u64);

        while !self.is_signaled() {
            if start.elapsed() >= timeout {
                return Err(Error::Timeout);
            }
            thread::sleep(Duration::from_micros(100));
        }

        Ok(())
    }

    /// Get fence ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get timestamp
    pub fn timestamp(&self) -> u64 {
        self.timestamp.load(Ordering::Acquire)
    }
}

/// Semaphore
#[derive(Debug)]
pub struct Semaphore {
    /// Count
    count: Arc<AtomicU32>,
    /// Maximum count
    max_count: u32,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(initial_count: u32, max_count: u32) -> Self {
        Self {
            count: Arc::new(AtomicU32::new(initial_count)),
            max_count,
        }
    }

    /// Acquire semaphore
    pub fn acquire(&self) -> Result<()> {
        let mut count = self.count.load(Ordering::Acquire);
        
        loop {
            if count == 0 {
                return Err(Error::Busy);
            }
            
            let new_count = count - 1;
            match self.count.compare_exchange_weak(
                count,
                new_count,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => count = actual,
            }
        }
    }

    /// Release semaphore
    pub fn release(&self) -> Result<()> {
        let mut count = self.count.load(Ordering::Acquire);
        
        loop {
            if count >= self.max_count {
                return Err(Error::InvalidState("Semaphore count overflow"));
            }
            
            let new_count = count + 1;
            match self.count.compare_exchange_weak(
                count,
                new_count,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => count = actual,
            }
        }
    }

    /// Try to acquire semaphore (non-blocking)
    pub fn try_acquire(&self) -> Result<()> {
        let mut count = self.count.load(Ordering::Acquire);
        
        if count == 0 {
            return Err(Error::Busy);
        }
        
        let new_count = count - 1;
        match self.count.compare_exchange_weak(
            count,
            new_count,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::Busy),
        }
    }

    /// Get current count
    pub fn count(&self) -> u32 {
        self.count.load(Ordering::Acquire)
    }

    /// Get maximum count
    pub fn max_count(&self) -> u32 {
        self.max_count
    }
}

/// Mutex
#[derive(Debug)]
pub struct Mutex {
    /// Locked flag
    locked: Arc<AtomicBool>,
    /// Owner thread ID
    owner: Arc<AtomicU64>,
    /// Lock count
    count: Arc<AtomicU32>,
}

impl Mutex {
    /// Create a new mutex
    pub fn new() -> Self {
        Self {
            locked: Arc::new(AtomicBool::new(false)),
            owner: Arc::new(AtomicU64::new(0)),
            count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Lock the mutex
    pub fn lock(&self) -> Result<()> {
        let thread_id = Self::current_thread_id();
        
        // Check if already owned by this thread
        if self.owner.load(Ordering::Acquire) == thread_id {
            self.count.fetch_add(1, Ordering::AcqRel);
            return Ok(());
        }
        
        // Try to acquire lock
        while self.locked.compare_exchange_weak(
            false,
            true,
            Ordering::AcqRel,
            Ordering::Acquire,
        ).is_err() {
            thread::sleep(Duration::from_micros(100));
        }
        
        self.owner.store(thread_id, Ordering::Release);
        self.count.store(1, Ordering::Release);
        
        Ok(())
    }

    /// Unlock the mutex
    pub fn unlock(&self) -> Result<()> {
        let thread_id = Self::current_thread_id();
        
        // Check if owned by this thread
        if self.owner.load(Ordering::Acquire) != thread_id {
            return Err(Error::PermissionDenied);
        }
        
        let count = self.count.fetch_sub(1, Ordering::AcqRel);
        
        if count == 1 {
            // Last unlock, release the mutex
            self.owner.store(0, Ordering::Release);
            self.locked.store(false, Ordering::Release);
        }
        
        Ok(())
    }

    /// Try to lock the mutex (non-blocking)
    pub fn try_lock(&self) -> Result<()> {
        let thread_id = Self::current_thread_id();
        
        // Check if already owned by this thread
        if self.owner.load(Ordering::Acquire) == thread_id {
            self.count.fetch_add(1, Ordering::AcqRel);
            return Ok(());
        }
        
        // Try to acquire lock
        if self.locked.compare_exchange_weak(
            false,
            true,
            Ordering::AcqRel,
            Ordering::Acquire,
        ).is_err() {
            return Err(Error::Busy);
        }
        
        self.owner.store(thread_id, Ordering::Release);
        self.count.store(1, Ordering::Release);
        
        Ok(())
    }

    /// Check if mutex is locked
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }

    /// Get current thread ID
    fn current_thread_id() -> u64 {
        // This is a simplified implementation
        // In production, would use proper thread ID
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Read-write lock
#[derive(Debug)]
pub struct RwLock {
    /// Reader count
    readers: Arc<AtomicU32>,
    /// Writer flag
    writer: Arc<AtomicBool>,
    /// Writer thread ID
    writer_id: Arc<AtomicU64>,
}

impl RwLock {
    /// Create a new read-write lock
    pub fn new() -> Self {
        Self {
            readers: Arc::new(AtomicU32::new(0)),
            writer: Arc::new(AtomicBool::new(false)),
            writer_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Acquire read lock
    pub fn read_lock(&self) -> Result<()> {
        // Wait for writer to release
        while self.writer.load(Ordering::Acquire) {
            thread::sleep(Duration::from_micros(100));
        }
        
        // Increment reader count
        self.readers.fetch_add(1, Ordering::AcqRel);
        
        Ok(())
    }

    /// Release read lock
    pub fn read_unlock(&self) -> Result<()> {
        self.readers.fetch_sub(1, Ordering::AcqRel);
        Ok(())
    }

    /// Acquire write lock
    pub fn write_lock(&self) -> Result<()> {
        let thread_id = Self::current_thread_id();
        
        // Check if already owned by this thread
        if self.writer_id.load(Ordering::Acquire) == thread_id {
            return Ok(());
        }
        
        // Wait for all readers and writer to release
        while self.readers.load(Ordering::Acquire) > 0 || self.writer.load(Ordering::Acquire) {
            thread::sleep(Duration::from_micros(100));
        }
        
        // Acquire write lock
        self.writer.store(true, Ordering::Release);
        self.writer_id.store(thread_id, Ordering::Release);
        
        Ok(())
    }

    /// Release write lock
    pub fn write_unlock(&self) -> Result<()> {
        let thread_id = Self::current_thread_id();
        
        // Check if owned by this thread
        if self.writer_id.load(Ordering::Acquire) != thread_id {
            return Err(Error::PermissionDenied);
        }
        
        self.writer.store(false, Ordering::Release);
        self.writer_id.store(0, Ordering::Release);
        
        Ok(())
    }

    /// Try to acquire read lock (non-blocking)
    pub fn try_read_lock(&self) -> Result<()> {
        if self.writer.load(Ordering::Acquire) {
            return Err(Error::Busy);
        }
        
        self.readers.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    /// Try to acquire write lock (non-blocking)
    pub fn try_write_lock(&self) -> Result<()> {
        let thread_id = Self::current_thread_id();
        
        if self.writer_id.load(Ordering::Acquire) == thread_id {
            return Ok(());
        }
        
        if self.readers.load(Ordering::Acquire) > 0 || self.writer.load(Ordering::Acquire) {
            return Err(Error::Busy);
        }
        
        self.writer.store(true, Ordering::Release);
        self.writer_id.store(thread_id, Ordering::Release);
        
        Ok(())
    }

    /// Get current thread ID
    fn current_thread_id() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Condition variable
#[derive(Debug)]
pub struct Condvar {
    /// Waiters count
    waiters: Arc<AtomicU32>,
}

impl Condvar {
    /// Create a new condition variable
    pub fn new() -> Self {
        Self {
            waiters: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Wait on condition variable
    pub fn wait(&self, mutex: &Mutex) -> Result<()> {
        self.waiters.fetch_add(1, Ordering::AcqRel);
        mutex.unlock()?;
        
        // Wait for signal
        while self.waiters.load(Ordering::Acquire) > 0 {
            thread::sleep(Duration::from_micros(100));
        }
        
        mutex.lock()?;
        Ok(())
    }

    /// Signal one waiter
    pub fn signal(&self) -> Result<()> {
        if self.waiters.load(Ordering::Acquire) > 0 {
            self.waiters.fetch_sub(1, Ordering::AcqRel);
        }
        Ok(())
    }

    /// Signal all waiters
    pub fn broadcast(&self) -> Result<()> {
        while self.waiters.load(Ordering::Acquire) > 0 {
            self.waiters.fetch_sub(1, Ordering::AcqRel);
        }
        Ok(())
    }

    /// Get waiter count
    pub fn waiter_count(&self) -> u32 {
        self.waiters.load(Ordering::Acquire)
    }
}

/// Barrier
#[derive(Debug)]
pub struct Barrier {
    /// Count
    count: Arc<AtomicU32>,
    /// Total
    total: u32,
    /// Generation
    generation: Arc<AtomicU32>,
}

impl Barrier {
    /// Create a new barrier
    pub fn new(total: u32) -> Self {
        Self {
            count: Arc::new(AtomicU32::new(0)),
            total,
            generation: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Wait at barrier
    pub fn wait(&self) -> Result<()> {
        let current_gen = self.generation.load(Ordering::Acquire);
        let count = self.count.fetch_add(1, Ordering::AcqRel);
        
        if count + 1 == self.total {
            // Last thread to arrive
            self.count.store(0, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
            Ok(())
        } else {
            // Wait for other threads
            while self.generation.load(Ordering::Acquire) == current_gen {
                thread::sleep(Duration::from_micros(100));
            }
            Ok(())
        }
    }

    /// Get total count
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Get current count
    pub fn count(&self) -> u32 {
        self.count.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fence_creation() {
        let fence = Fence::new();
        assert!(!fence.is_signaled());
        
        fence.signal();
        assert!(fence.is_signaled());
    }

    #[test]
    fn test_fence_wait() {
        let fence = Fence::new();
        
        // Should timeout
        let result = fence.wait(100);
        assert!(result.is_err());
        
        fence.signal();
        
        // Should succeed
        let result = fence.wait(1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_semaphore() {
        let sem = Semaphore::new(2, 2);
        
        assert_eq!(sem.count(), 2);
        
        sem.acquire().unwrap();
        assert_eq!(sem.count(), 1);
        
        sem.acquire().unwrap();
        assert_eq!(sem.count(), 0);
        
        // Should fail
        let result = sem.acquire();
        assert!(result.is_err());
        
        sem.release().unwrap();
        assert_eq!(sem.count(), 1);
    }

    #[test]
    fn test_mutex() {
        let mutex = Mutex::new();
        
        assert!(!mutex.is_locked());
        
        mutex.lock().unwrap();
        assert!(mutex.is_locked());
        
        mutex.unlock().unwrap();
        assert!(!mutex.is_locked());
    }

    #[test]
    fn test_rwlock() {
        let rwlock = RwLock::new();
        
        rwlock.read_lock().unwrap();
        rwlock.read_unlock().unwrap();
        
        rwlock.write_lock().unwrap();
        rwlock.write_unlock().unwrap();
    }

    #[test]
    fn test_barrier() {
        let barrier = Arc::new(Barrier::new(2));
        let mut handles = vec![];
        
        for _ in 0..2 {
            let barrier_clone = Arc::clone(&barrier);
            let handle = thread::spawn(move || {
                barrier_clone.wait().unwrap();
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
