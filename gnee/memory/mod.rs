// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Memory Management
 *
 * Copyright (C) 2025 GNEE Team
 *
 * Memory management for pinned physical RAM and DMA buffers.
 */

//! # Memory Management
//!
//! Provides memory allocation, pinning, and DMA buffer management.

use std::ptr::NonNull;
use std::sync::Arc;

use crate::error::{Error, Result};

/// Physical address type
pub type PhysAddr = u64;

/// Virtual address type
pub type VirtualAddr = u64;

/// Memory flags
#[derive(Debug, Clone, Copy, Default)]
pub struct MemFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub pinned: bool,
    pub coherent: bool,
    pub protected: bool,
}

/// Memory region
#[derive(Debug)]
pub struct MemoryRegion {
    /// Physical base address
    pub base: PhysAddr,
    /// Size in bytes
    pub size: usize,
    /// Memory flags
    pub flags: MemFlags,
    /// Virtual address (if mapped)
    pub virtual_addr: Option<VirtualAddr>,
    /// Reference count
    refcount: Arc<()>,
}

impl MemoryRegion {
    /// Create a new memory region
    pub fn new(base: PhysAddr, size: usize, flags: MemFlags) -> Self {
        Self {
            base,
            size,
            flags,
            virtual_addr: None,
            refcount: Arc::new(()),
        }
    }

    /// Get physical base address
    pub fn base(&self) -> PhysAddr {
        self.base
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get flags
    pub fn flags(&self) -> MemFlags {
        self.flags
    }

    /// Get virtual address
    pub fn virtual_addr(&self) -> Option<VirtualAddr> {
        self.virtual_addr
    }

    /// Set virtual address
    pub fn set_virtual_addr(&mut self, addr: VirtualAddr) {
        self.virtual_addr = Some(addr);
    }

    /// Check if memory is pinned
    pub fn is_pinned(&self) -> bool {
        self.flags.pinned
    }

    /// Check if memory is executable
    pub fn is_executable(&self) -> bool {
        self.flags.executable
    }

    /// Check if memory is readable
    pub fn is_readable(&self) -> bool {
        self.flags.readable
    }

    /// Check if memory is writable
    pub fn is_writable(&self) -> bool {
        self.flags.writable
    }

    /// Get end address (exclusive)
    pub fn end(&self) -> PhysAddr {
        self.base + self.size as u64
    }

    /// Check if address is within this region
    pub fn contains(&self, addr: PhysAddr) -> bool {
        addr >= self.base && addr < self.end()
    }

    /// Get reference count
    pub fn refcount(&self) -> usize {
        Arc::strong_count(&self.refcount)
    }
}

/// Memory allocator
pub struct MemoryAllocator {
    /// Total memory available
    total_memory: usize,
    /// Used memory
    used_memory: usize,
    /// Allocated regions
    regions: Vec<MemoryRegion>,
}

impl MemoryAllocator {
    /// Create a new memory allocator
    pub fn new(total_memory: usize) -> Self {
        Self {
            total_memory,
            used_memory: 0,
            regions: Vec::new(),
        }
    }

    /// Allocate memory region
    pub fn allocate(&mut self, size: usize, flags: MemFlags) -> Result<MemoryRegion> {
        // Check if we have enough memory
        if self.used_memory + size > self.total_memory {
            return Err(Error::OutOfMemory);
        }

        // Find a suitable physical address
        let base = self.find_free_address(size)?;

        // Create memory region
        let region = MemoryRegion::new(base, size, flags);

        // Update statistics
        self.used_memory += size;
        self.regions.push(region.clone());

        Ok(region)
    }

    /// Free memory region
    pub fn free(&mut self, region: MemoryRegion) -> Result<()> {
        // Find and remove region
        let index = self.regions.iter()
            .position(|r| r.base == region.base)
            .ok_or(Error::ResourceNotFound("Memory region"))?;

        let freed_region = self.regions.remove(index);
        self.used_memory -= freed_region.size;

        Ok(())
    }

    /// Find free physical address
    fn find_free_address(&self, size: usize) -> Result<PhysAddr> {
        // Simple linear search for free space
        // In production, would use a more sophisticated allocator
        
        let mut addr = 0x1000u64; // Start at 4KB
        
        for region in &self.regions {
            if addr + size as u64 <= region.base {
                // Found free space before this region
                return Ok(addr);
            }
            
            // Move past this region
            addr = region.end();
            
            // Align to page boundary
            addr = (addr + 0xFFF) & !0xFFF;
        }

        // Check if we have space after all regions
        if addr + size as u64 <= self.total_memory as u64 {
            Ok(addr)
        } else {
            Err(Error::OutOfMemory)
        }
    }

    /// Get total memory
    pub fn total_memory(&self) -> usize {
        self.total_memory
    }

    /// Get used memory
    pub fn used_memory(&self) -> usize {
        self.used_memory
    }

    /// Get free memory
    pub fn free_memory(&self) -> usize {
        self.total_memory - self.used_memory
    }

    /// Get number of allocated regions
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get memory usage percentage
    pub fn usage_percentage(&self) -> f64 {
        if self.total_memory == 0 {
            0.0
        } else {
            (self.used_memory as f64 / self.total_memory as f64) * 100.0
        }
    }
}

/// DMA buffer
#[derive(Debug)]
pub struct DmaBuffer {
    /// Physical address
    pub phys_addr: PhysAddr,
    /// Virtual address
    pub virt_addr: VirtualAddr,
    /// Size
    pub size: usize,
    /// File descriptor (for sharing)
    pub fd: i32,
    /// Flags
    pub flags: MemFlags,
}

impl DmaBuffer {
    /// Create a new DMA buffer
    pub fn new(phys_addr: PhysAddr, virt_addr: VirtualAddr, size: usize, flags: MemFlags) -> Self {
        Self {
            phys_addr,
            virt_addr,
            size,
            fd: -1,
            flags,
        }
    }

    /// Get physical address
    pub fn phys_addr(&self) -> PhysAddr {
        self.phys_addr
    }

    /// Get virtual address
    pub fn virt_addr(&self) -> VirtualAddr {
        self.virt_addr
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get file descriptor
    pub fn fd(&self) -> i32 {
        self.fd
    }

    /// Set file descriptor
    pub fn set_fd(&mut self, fd: i32) {
        self.fd = fd;
    }

    /// Get flags
    pub fn flags(&self) -> MemFlags {
        self.flags
    }

    /// Map to user space
    pub fn map(&self) -> Result<NonNull<u8>> {
        unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                self.size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.fd,
                0,
            );

            if ptr == libc::MAP_FAILED {
                return Err(Error::MappingError);
            }

            Ok(NonNull::new_unchecked(ptr as *mut u8))
        }
    }

    /// Unmap from user space
    pub fn unmap(&self, addr: NonNull<u8>) -> Result<()> {
        unsafe {
            let result = libc::munmap(addr.as_ptr() as *mut libc::c_void, self.size);
            if result != 0 {
                return Err(Error::UnmappingError);
            }
        }
        Ok(())
    }

    /// Sync for CPU access
    pub fn sync_for_cpu(&self) -> Result<()> {
        unsafe {
            let result = libc::msync(
                self.virt_addr as *mut libc::c_void,
                self.size,
                libc::MS_SYNC,
            );
            if result != 0 {
                return Err(Error::SyncError);
            }
        }
        Ok(())
    }

    /// Sync for device access
    pub fn sync_for_device(&self) -> Result<()> {
        // On ARM64, cache management is automatic for DMA buffers
        Ok(())
    }
}

impl Drop for DmaBuffer {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

/// Memory pool for efficient allocation
pub struct MemoryPool {
    /// Block size
    block_size: usize,
    /// Number of blocks
    block_count: usize,
    /// Free blocks
    free_blocks: Vec<usize>,
    /// Allocated blocks
    allocated: Vec<bool>,
    /// Base address
    base: PhysAddr,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(base: PhysAddr, block_size: usize, block_count: usize) -> Self {
        let free_blocks: Vec<usize> = (0..block_count).collect();
        let allocated = vec![false; block_count];

        Self {
            block_size,
            block_count,
            free_blocks,
            allocated,
            base,
        }
    }

    /// Allocate a block from the pool
    pub fn allocate_block(&mut self) -> Result<PhysAddr> {
        if let Some(block_index) = self.free_blocks.pop() {
            self.allocated[block_index] = true;
            Ok(self.base + (block_index * self.block_size) as u64)
        } else {
            Err(Error::OutOfMemory)
        }
    }

    /// Free a block back to the pool
    pub fn free_block(&mut self, addr: PhysAddr) -> Result<()> {
        let offset = (addr - self.base) as usize;
        let block_index = offset / self.block_size;

        if block_index >= self.block_count {
            return Err(Error::InvalidArgument("Invalid address"));
        }

        if !self.allocated[block_index] {
            return Err(Error::InvalidState("Block not allocated"));
        }

        self.allocated[block_index] = false;
        self.free_blocks.push(block_index);

        Ok(())
    }

    /// Get number of free blocks
    pub fn free_block_count(&self) -> usize {
        self.free_blocks.len()
    }

    /// Get number of allocated blocks
    pub fn allocated_block_count(&self) -> usize {
        self.block_count - self.free_blocks.len()
    }

    /// Get total blocks
    pub fn total_blocks(&self) -> usize {
        self.block_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_creation() {
        let flags = MemFlags {
            readable: true,
            writable: true,
            ..Default::default()
        };

        let region = MemoryRegion::new(0x1000, 4096, flags);
        assert_eq!(region.base(), 0x1000);
        assert_eq!(region.size(), 4096);
        assert!(region.is_readable());
        assert!(region.is_writable());
    }

    #[test]
    fn test_memory_region_contains() {
        let region = MemoryRegion::new(0x1000, 4096, MemFlags::default());
        assert!(region.contains(0x1000));
        assert!(region.contains(0x1FFF));
        assert!(!region.contains(0x2000));
    }

    #[test]
    fn test_memory_allocator() {
        let mut allocator = MemoryAllocator::new(1024 * 1024); // 1MB

        let flags = MemFlags::default();
        let region = allocator.allocate(4096, flags).unwrap();
        assert_eq!(region.size(), 4096);
        assert_eq!(allocator.used_memory(), 4096);
        assert_eq!(allocator.free_memory(), 1024 * 1024 - 4096);
    }

    #[test]
    fn test_memory_pool() {
        let mut pool = MemoryPool::new(0x1000, 4096, 16);

        let addr1 = pool.allocate_block().unwrap();
        let addr2 = pool.allocate_block().unwrap();

        assert_eq!(pool.free_block_count(), 14);
        assert_eq!(pool.allocated_block_count(), 2);

        pool.free_block(addr1).unwrap();
        assert_eq!(pool.free_block_count(), 15);
    }
}
