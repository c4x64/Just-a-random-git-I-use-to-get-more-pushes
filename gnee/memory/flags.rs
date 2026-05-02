// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Memory Flags
 *
 * Copyright (C) 2025 GNEE Team
 */

//! Memory flags and types.

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemType {
    /// Normal memory
    Normal,
    /// Device memory
    Device,
    /// Protected memory
    Protected,
    /// DMA coherent memory
    DmaCoherent,
}

/// Memory flags
#[derive(Debug, Clone, Copy, Default)]
pub struct MemFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub pinned: bool,
    pub coherent: bool,
    pub protected: bool,
    pub cached: bool,
    pub uncached: bool,
    pub writecombine: bool,
}

impl MemFlags {
    /// Create new flags
    pub fn new() -> Self {
        Self::default()
    }

    /// Set readable
    pub fn read(mut self) -> Self {
        self.readable = true;
        self
    }

    /// Set writable
    pub fn write(mut self) -> Self {
        self.writable = true;
        self
    }

    /// Set executable
    pub fn exec(mut self) -> Self {
        self.executable = true;
        self
    }

    /// Set pinned
    pub fn pin(mut self) -> Self {
        self.pinned = true;
        self
    }

    /// Set coherent
    pub fn coherent(mut self) -> Self {
        self.coherent = true;
        self
    }

    /// Set protected
    pub fn protect(mut self) -> Self {
        self.protected = true;
        self
    }

    /// Set uncached
    pub fn uncached(mut self) -> Self {
        self.uncached = true;
        self
    }

    /// Set write-combine
    pub fn writecombine(mut self) -> Self {
        self.writecombine = true;
        self.uncached = true;
        self
    }

    /// Check if readable
    pub fn is_readable(&self) -> bool { self.readable }

    /// Check if writable
    pub fn is_writable(&self) -> bool { self.writable }

    /// Check if executable
    pub fn is_executable(&self) -> bool { self.executable }

    /// Check if pinned
    pub fn is_pinned(&self) -> bool { self.pinned }

    /// Check if coherent
    pub fn is_coherent(&self) -> bool { self.coherent }

    /// Check if protected
    pub fn is_protected(&self) -> bool { self.protected }

    /// Check if cached
    pub fn is_cached(&self) -> bool { !self.uncached }

    /// Check if uncached
    pub fn is_uncached(&self) -> bool { self.uncached }
}

impl Default for MemType {
    fn default() -> Self {
        MemType::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_default() {
        let flags = MemFlags::default();
        assert!(!flags.readable);
        assert!(!flags.writable);
        assert!(!flags.executable);
    }

    #[test]
    fn test_flags_builder() {
        let flags = MemFlags::new().read().write().pin();
        assert!(flags.is_readable());
        assert!(flags.is_writable());
        assert!(flags.is_pinned());
    }

    #[test]
    fn test_mem_type() {
        let t = MemType::Normal;
        assert_eq!(t, MemType::Normal);
    }
}
