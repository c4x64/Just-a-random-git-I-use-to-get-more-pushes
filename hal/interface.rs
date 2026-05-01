// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - HAL Interface
 *
 * Copyright (C) 2025 GNEE Team
 *
 * Polymorphic Hardware Abstraction Layer that switches behavior
 * based on privilege level (Rooted vs Non-Rooted).
 */

//! # HAL Interface
//!
//! Provides unified hardware access with automatic fallback based on privileges.

use std::fmt::Debug;
use std::io::Result;

/// Memory flags for allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub pinned: bool,
    pub coherent: bool,
    pub protected: bool,
}

impl Default for MemFlags {
    fn default() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
            pinned: false,
            coherent: false,
            protected: false,
        }
    }
}

/// Physical address type
pub type PhysAddr = u64;

/// Virtual address type
pub type VirtualAddr = u64;

/// Thread priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreadPriority {
    Idle,
    Normal,
    High,
    RealTime,
}

/// Fence for GPU synchronization
#[derive(Debug, Clone, Copy)]
pub struct Fence {
    pub id: u64,
    pub signaled: bool,
}

/// Physical memory region
#[derive(Debug)]
pub struct PhysicalRegion {
    pub base: PhysAddr,
    pub size: usize,
    pub flags: MemFlags,
    pub virtual_addr: Option<VirtualAddr>,
}

/// Command buffer for GPU submission
#[derive(Debug)]
pub struct CommandBuffer {
    pub commands: Vec<u8>,
    pub resources: Vec<ResourceHandle>,
}

/// Resource handle
#[derive(Debug, Clone)]
pub struct ResourceHandle {
    pub id: u64,
    pub type_: ResourceType,
}

#[derive(Debug, Clone)]
pub enum ResourceType {
    Buffer,
    Texture,
    Sampler,
    Pipeline,
}

/// Interrupt handler callback
pub type InterruptHandler = Box<dyn Fn() + Send + Sync>;

/// Hardware Abstraction Layer Interface
pub trait HalInterface: Debug + Send + Sync {
    /// Check if running in rooted mode
    fn is_rooted(&self) -> bool;
    
    /// Get privilege level description
    fn privilege_level(&self) -> &str;

    // Memory Management
    fn allocate_pinned_memory(&self, size: usize, flags: MemFlags) -> Result<PhysicalRegion>;
    fn free_pinned_memory(&self, region: PhysicalRegion) -> Result<()>;
    fn map_physical(&self, addr: PhysAddr, size: usize) -> Result<VirtualAddr>;
    fn unmap_physical(&self, addr: VirtualAddr, size: usize) -> Result<()>;
    fn flush_cache(&self, vaddr: VirtualAddr, size: usize) -> Result<()>;
    fn invalidate_cache(&self, vaddr: VirtualAddr, size: usize) -> Result<()>;

    // CPU Control
    fn pin_cpu(&self, core_id: u32) -> Result<()>;
    fn unpin_cpu(&self) -> Result<()>;
    fn set_priority(&self, priority: ThreadPriority) -> Result<()>;
    fn get_core_count(&self) -> u32;
    fn get_online_cores(&self) -> Vec<u32>;

    // GPU Control
    fn submit_command_buffer(&self, cmd: &CommandBuffer) -> Result<Fence>;
    fn wait_fence(&self, fence: Fence, timeout_ms: u32) -> Result<()>;
    fn signal_fence(&self, fence: Fence) -> Result<()>;
    fn reset_fence(&self, fence: Fence) -> Result<()>;

    // Interrupts
    fn register_interrupt(&self, irq: u32, handler: InterruptHandler) -> Result<()>;
    fn unregister_interrupt(&self, irq: u32) -> Result<()>;
    fn enable_interrupt(&self, irq: u32) -> Result<()>;
    fn disable_interrupt(&self, irq: u32) -> Result<()>;

    // Thermal Management
    fn start_thermal_heartbeat(&self) -> Result<()>;
    fn stop_thermal_heartbeat(&self) -> Result<()>;
    fn get_temperature(&self) -> Result<i32>;
    fn get_max_temperature(&self) -> Result<i32>;

    // Information
    fn get_device_info(&self) -> DeviceInfo;
    fn get_capabilities(&self) -> HalCapabilities;
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub model: String,
    pub manufacturer: String,
    pub android_version: String,
    kernel_version: String,
    pub total_ram: u64,
    pub available_ram: u64,
    pub cpu_cores: u32,
    pub gpu_name: String,
}

/// HAL capabilities
#[derive(Debug, Clone)]
pub struct HalCapabilities {
    pub supports_smmu: bool,
    pub supports_ion: bool,
    pub supports_dma_buf: bool,
    pub supports_vulkan: bool,
    pub supports_mfg: bool,
    pub supports_fsr: bool,
    pub max_texture_size: u32,
    pub max_buffer_size: usize,
    pub supported_formats: Vec<String>,
}

/// HAL factory for creating appropriate implementation
pub struct HalFactory;

impl HalFactory {
    /// Create HAL instance based on available privileges
    pub fn create() -> Result<Box<dyn HalInterface>> {
        // Check for root/hypervisor access
        if Self::check_hypervisor_access() {
            #[cfg(feature = "rooted")]
            {
                return Ok(Box::new(crate::hal::rooted::RootedHal::new()?));
            }
            
            #[cfg(not(feature = "rooted"))]
            {
                log::warn!("Hypervisor detected but not compiled with rooted feature");
            }
        }
        
        // Fallback to user-space implementation
        Ok(Box::new(crate::hal::user::UserHal::new()?))
    }
    
    /// Check for hypervisor/EL2 access
    fn check_hypervisor_access() -> bool {
        // Try to read /dev/kvm or check for Magisk
        std::path::Path::new("/dev/kvm").exists()
            || std::path::Path::new("/sbin/.magisk").exists()
            || std::path::Path::new("/data/adb").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_flags_default() {
        let flags = MemFlags::default();
        assert!(flags.readable);
        assert!(flags.writable);
        assert!(!flags.executable);
    }

    #[test]
    fn test_thread_priority_ordering() {
        assert!(ThreadPriority::RealTime > ThreadPriority::High);
        assert!(ThreadPriority::High > ThreadPriority::Normal);
        assert!(ThreadPriority::Normal > ThreadPriority::Idle);
    }
}
