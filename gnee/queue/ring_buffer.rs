// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Ring Buffer
 *
 * Copyright (C) 2025 GNEE Team
 */

//! Lock-free ring buffer implementation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::error::{Error, Result};

/// Ring buffer configuration
#[derive(Debug, Clone)]
pub struct RingBufferConfig {
    pub size: usize,
    pub blocking: bool,
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            size: 4096,
            blocking: false,
        }
    }
}

/// Lock-free ring buffer
pub struct RingBuffer<T> {
    buffer: Vec<T>,
    mask: usize,
    head: AtomicU64,
    tail: AtomicU64,
    config: RingBufferConfig,
}

impl<T: Default + Clone> RingBuffer<T> {
    pub fn new(config: RingBufferConfig) -> Self {
        let size = config.size.next_power_of_two();
        let mask = size - 1;
        
        let mut buffer = Vec::with_capacity(size);
        buffer.resize_with(size, T::default);

        Self {
            buffer,
            mask,
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            config,
        }
    }

    pub fn push(&self, item: T) -> Result<()> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);
            let next_head = head.wrapping_add(1);

            if next_head.wrapping_sub(tail) > self.mask as u64 {
                if !self.config.blocking {
                    return Err(Error::Busy);
                }
                continue;
            }

            if self.head.compare_exchange_weak(
                head,
                next_head,
                Ordering::AcqRel,
                Ordering::Acquire,
            ).is_ok() {
                let index = (head & self.mask as u64) as usize;
                self.buffer[index] = item;
                return Ok(());
            }
        }
    }

    pub fn pop(&self) -> Result<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);

            if tail == head {
                if !self.config.blocking {
                    return Err(Error::Busy);
                }
                continue;
            }

            if self.tail.compare_exchange_weak(
                tail,
                tail.wrapping_add(1),
                Ordering::AcqRel,
                Ordering::Acquire,
            ).is_ok() {
                let index = (tail & self.mask as u64) as usize;
                return Ok(self.buffer[index].clone());
            }
        }
    }

    pub fn len(&self) -> u64 {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.mask as usize + 1
    }

    pub fn clear(&self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_creation() {
        let config = RingBufferConfig::default();
        let buffer = RingBuffer::<u32>::new(config);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_push_pop() {
        let config = RingBufferConfig::default();
        let buffer = RingBuffer::<u32>::new(config);
        
        buffer.push(42).unwrap();
        assert_eq!(buffer.len(), 1);
        
        let value = buffer.pop().unwrap();
        assert_eq!(value, 42);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_capacity() {
        let config = RingBufferConfig { size: 1024, ..Default::default() };
        let buffer = RingBuffer::<u32>::new(config);
        assert_eq!(buffer.capacity(), 1024);
    }
}
