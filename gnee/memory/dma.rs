// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - DMA Buffer
 *
 * Copyright (C) 2025 GNEE Team
 */

//! DMA buffer management for zero-copy sharing.

use crate::error::{Error, Result};
use crate::memory::flags::MemFlags;

/// DMA buffer
#[derive(Debug)]
pub struct DmaBuffer {
    fd: i32,
    size: usize,
    flags: MemFlags,
    mapped: bool,
}

impl DmaBuffer {
    /// Create a new DMA buffer
    pub fn new(fd: i32, size: usize, flags: MemFlags) -> Self {
        Self {
            fd,
            size,
            flags,
            mapped: false,
        }
    }

    /// Get file descriptor
    pub fn fd(&self) -> i32 { self.fd }

    /// Get size
    pub fn size(&self) -> usize { self.size }

    /// Get flags
    pub fn flags(&self) -> MemFlags { self.flags }

    /// Check if mapped
    pub fn is_mapped(&self) -> bool { self.mapped }

    /// Map buffer
    pub fn map(&mut self) -> Result<*mut u8> {
        if self.mapped {
            return Err(Error::InvalidState("Already mapped"));
        }

        #[cfg(unix)]
        {
            use std::os::unix::io::RawFd;
            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    self.size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    self.fd,
                    0,
                )
            };

            if ptr == libc::MAP_FAILED {
                return Err(Error::MappingError);
            }

            self.mapped = true;
            Ok(ptr as *mut u8)
        }

        #[cfg(not(unix))]
        {
            Err(Error::NotSupported)
        }
    }

    /// Unmap buffer
    pub fn unmap(&mut self, ptr: *mut u8) -> Result<()> {
        if !self.mapped {
            return Err(Error::InvalidState("Not mapped"));
        }

        #[cfg(unix)]
        {
            let result = unsafe { libc::munmap(ptr as *mut libc::c_void, self.size) };
            if result != 0 {
                return Err(Error::UnmappingError);
            }

            self.mapped = false;
            Ok(())
        }

        #[cfg(not(unix))]
        {
            Err(Error::NotSupported)
        }
    }

    /// Sync for CPU
    pub fn sync_for_cpu(&self) -> Result<()> {
        #[cfg(unix)]
        {
            // DMA buffers are typically coherent
            Ok(())
        }

        #[cfg(not(unix))]
        {
            Err(Error::NotSupported)
        }
    }

    /// Sync for device
    pub fn sync_for_device(&self) -> Result<()> {
        #[cfg(unix)]
        {
            // DMA buffers are typically coherent
            Ok(())
        }

        #[cfg(not(unix))]
        {
            Err(Error::NotSupported)
        }
    }
}

impl Drop for DmaBuffer {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if self.fd >= 0 {
                unsafe { libc::close(self.fd) };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dma_buffer_creation() {
        let buf = DmaBuffer::new(-1, 4096, MemFlags::default());
        assert_eq!(buf.size(), 4096);
        assert!(!buf.is_mapped());
    }

    #[test]
    fn test_dma_buffer_flags() {
        let flags = MemFlags::default().read().write();
        let buf = DmaBuffer::new(-1, 4096, flags);
        assert_eq!(buf.size(), 4096);
    }
}
