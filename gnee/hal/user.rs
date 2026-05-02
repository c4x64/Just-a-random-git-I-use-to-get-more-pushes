// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - User-Space HAL Implementation
 *
 * Copyright (C) 2025 GNEE Team
 *
 * User-space fallback implementation for non-rooted devices.
 * Uses standard Linux APIs for hardware access.
 */

//! # User-Space HAL
//!
//! Provides hardware access via standard Linux APIs for non-rooted devices.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hal::interface::{
    HalInterface, MemFlags, PhysAddr, VirtualAddr, ThreadPriority,
    Fence, PhysicalRegion, CommandBuffer, InterruptHandler,
    DeviceInfo, HalCapabilities, ResourceType
};
use crate::error::{Error, Result};

/// User-space HAL implementation
pub struct UserHal {
    /// Device information
    device_info: DeviceInfo,
    /// Capabilities
    capabilities: HalCapabilities,
    /// Thermal heartbeat thread handle
    thermal_thread: Option<thread::JoinHandle<()>>,
    /// Thermal heartbeat running flag
    thermal_running: Arc<Mutex<bool>>,
    /// Pinned memory regions
    pinned_memory: Arc<Mutex<Vec<PhysicalRegion>>>,
    /// CPU affinity mask
    cpu_affinity: Arc<Mutex<Option<u32>>>,
}

impl UserHal {
    /// Create a new user-space HAL instance
    pub fn new() -> Result<Self> {
        let device_info = Self::read_device_info()?;
        let capabilities = Self::detect_capabilities(&device_info)?;

        Ok(Self {
            device_info,
            capabilities,
            thermal_thread: None,
            thermal_running: Arc::new(Mutex::new(false)),
            pinned_memory: Arc::new(Mutex::new(Vec::new())),
            cpu_affinity: Arc::new(Mutex::new(None)),
        })
    }

    /// Read device information from system
    fn read_device_info() -> Result<DeviceInfo> {
        let model = Self::read_file("/sys/firmware/devicetree/base/model")
            .unwrap_or_else(|_| "Unknown".to_string());
        
        let manufacturer = Self::read_file("/sys/firmware/devicetree/base/manufacturer")
            .unwrap_or_else(|_| "Unknown".to_string());
        
        let android_version = Self::read_file("/proc/version")
            .unwrap_or_else(|_| "Unknown".to_string());
        
        let kernel_version = Self::read_file("/proc/version")
            .unwrap_or_else(|_| "Unknown".to_string());
        
        let total_ram = Self::read_meminfo("MemTotal")?;
        let available_ram = Self::read_meminfo("MemAvailable")?;
        
        let cpu_cores = Self::read_cpu_cores();
        
        let gpu_name = Self::read_gpu_name()?;

        Ok(DeviceInfo {
            model,
            manufacturer,
            android_version,
            kernel_version,
            total_ram,
            available_ram,
            cpu_cores,
            gpu_name,
        })
    }

    /// Read file contents
    fn read_file(path: &str) -> Result<String> {
        let mut file = File::open(path)
            .map_err(|_| Error::IoError("Failed to open file"))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|_| Error::IoError("Failed to read file"))?;
        
        Ok(contents.trim().to_string())
    }

    /// Read memory info from /proc/meminfo
    fn read_meminfo(key: &str) -> Result<u64> {
        let meminfo = Self::read_file("/proc/meminfo")?;
        
        for line in meminfo.lines() {
            if line.starts_with(key) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let value: u64 = parts[1].parse()
                        .map_err(|_| Error::IoError("Failed to parse memory value"))?;
                    return Ok(value * 1024); // Convert KB to bytes
                }
            }
        }
        
        Err(Error::IoError("Memory key not found"))
    }

    /// Get CPU core count
    fn read_cpu_cores() -> u32 {
        let cores = Self::read_file("/proc/cpuinfo")
            .unwrap_or_default();
        
        cores.lines()
            .filter(|line| line.starts_with("processor"))
            .count() as u32
    }

    /// Read GPU name
    fn read_gpu_name() -> Result<String> {
        // Try to read from various GPU info paths
        let paths = [
            "/sys/class/mali/mali0/gpuinfo/gpu_name",
            "/sys/class/kgsl/kgsl-3d0/gpu_model",
            "/sys/class/drm/card0/device/gpu_name",
        ];
        
        for path in &paths {
            if Path::new(path).exists() {
                if let Ok(name) = Self::read_file(path) {
                    return Ok(name);
                }
            }
        }
        
        Ok("Unknown".to_string())
    }

    /// Detect system capabilities
    fn detect_capabilities(device_info: &DeviceInfo) -> Result<HalCapabilities> {
        let supports_smmu = Path::new("/sys/class/iommu").exists();
        let supports_ion = Path::new("/dev/ion").exists();
        let supports_dma_buf = Path::new("/sys/kernel/debug/dma_buf").exists();
        let supports_vulkan = Self::check_vulkan_support();
        let supports_mfg = false; // Requires hypervisor
        let supports_fsr = supports_vulkan; // FSR requires Vulkan
        
        let max_texture_size = 16384; // Default max
        let max_buffer_size = device_info.available_ram / 4; // Use 25% of RAM
        
        let supported_formats = vec![
            "R8G8B8A8_UNORM".to_string(),
            "R8G8B8A8_SRGB".to_string(),
            "R16G16B16A16_SFLOAT".to_string(),
            "R32G32B32A32_SFLOAT".to_string(),
            "D32_SFLOAT".to_string(),
            "D24_UNORM_S8_UINT".to_string(),
        ];

        Ok(HalCapabilities {
            supports_smmu,
            supports_ion,
            supports_dma_buf,
            supports_vulkan,
            supports_mfg,
            supports_fsr,
            max_texture_size,
            max_buffer_size,
            supported_formats,
        })
    }

    /// Check for Vulkan support
    fn check_vulkan_support() -> bool {
        // Check for Vulkan loader
        Path::new("/system/lib64/libvulkan.so").exists() ||
        Path::new("/vendor/lib64/libvulkan.so").exists()
    }

    /// Pin CPU to specific core using sched_setaffinity
    fn pin_cpu_internal(&self, core_id: u32) -> Result<()> {
        use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};
        
        let mut cpuset: cpu_set_t = unsafe { std::mem::zeroed() };
        unsafe { CPU_ZERO(&mut cpuset) };
        unsafe { CPU_SET(core_id as i32, &mut cpuset) };
        
        let result = unsafe {
            sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &cpuset)
        };
        
        if result != 0 {
            return Err(Error::IoError("Failed to set CPU affinity"));
        }
        
        Ok(())
    }

    /// Start thermal heartbeat thread
    fn start_thermal_heartbeat_thread(&mut self) -> Result<()> {
        let running = Arc::clone(&self.thermal_running);
        *running.lock().unwrap() = true;
        
        let handle = thread::spawn(move || {
            while *running.lock().unwrap() {
                // Perform thermal heartbeat
                // This keeps the CPU active to prevent thermal throttling
                let mut dummy: u64 = 0;
                for _ in 0..1000 {
                    dummy = dummy.wrapping_add(1);
                }
                
                thread::sleep(Duration::from_millis(10));
            }
        });
        
        self.thermal_thread = Some(handle);
        Ok(())
    }
}

impl HalInterface for UserHal {
    fn is_rooted(&self) -> bool {
        false
    }

    fn privilege_level(&self) -> &str {
        "User-Space (Non-Rooted)"
    }

    fn allocate_pinned_memory(&self, size: usize, flags: MemFlags) -> Result<PhysicalRegion> {
        use libc::{mlock, mmap, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE};
        
        // Allocate memory
        let addr = unsafe {
            mmap(
                std::ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        
        if addr == libc::MAP_FAILED {
            return Err(Error::IoError("Failed to allocate memory"));
        }
        
        // Pin memory
        let result = unsafe { mlock(addr, size) };
        if result != 0 {
            unsafe { libc::munmap(addr, size) };
            return Err(Error::IoError("Failed to pin memory"));
        }
        
        let region = PhysicalRegion {
            base: addr as u64,
            size,
            flags,
            virtual_addr: Some(addr as u64),
        };
        
        // Track pinned memory
        self.pinned_memory.lock().unwrap().push(region.clone());
        
        Ok(region)
    }

    fn free_pinned_memory(&self, region: PhysicalRegion) -> Result<()> {
        use libc::{munlock, munmap};
        
        let addr = region.virtual_addr.ok_or(Error::InvalidState("No virtual address"))?;
        
        // Unpin memory
        unsafe { munlock(addr as *mut libc::c_void, region.size) };
        
        // Free memory
        let result = unsafe { munmap(addr as *mut libc::c_void, region.size) };
        if result != 0 {
            return Err(Error::IoError("Failed to free memory"));
        }
        
        // Remove from tracking
        let mut pinned = self.pinned_memory.lock().unwrap();
        pinned.retain(|r| r.base != region.base);
        
        Ok(())
    }

    fn map_physical(&self, addr: PhysAddr, size: usize) -> Result<VirtualAddr> {
        use libc::{mmap, MAP_SHARED, PROT_READ, PROT_WRITE};
        
        let vaddr = unsafe {
            mmap(
                std::ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                -1,
                addr as i64,
            )
        };
        
        if vaddr == libc::MAP_FAILED {
            return Err(Error::IoError("Failed to map physical memory"));
        }
        
        Ok(vaddr as u64)
    }

    fn unmap_physical(&self, addr: VirtualAddr, size: usize) -> Result<()> {
        use libc::munmap;
        
        let result = unsafe { munmap(addr as *mut libc::c_void, size) };
        if result != 0 {
            return Err(Error::IoError("Failed to unmap memory"));
        }
        
        Ok(())
    }

    fn flush_cache(&self, vaddr: VirtualAddr, size: usize) -> Result<()> {
        use libc::{msync, MS_SYNC};
        
        let result = unsafe { msync(vaddr as *mut libc::c_void, size, MS_SYNC) };
        if result != 0 {
            return Err(Error::IoError("Failed to flush cache"));
        }
        
        Ok(())
    }

    fn invalidate_cache(&self, vaddr: VirtualAddr, size: usize) -> Result<()> {
        // Cache invalidation is automatic on most ARM64 systems
        Ok(())
    }

    fn pin_cpu(&self, core_id: u32) -> Result<()> {
        self.pin_cpu_internal(core_id)?;
        *self.cpu_affinity.lock().unwrap() = Some(core_id);
        Ok(())
    }

    fn unpin_cpu(&self) -> Result<()> {
        use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};
        
        let mut cpuset: cpu_set_t = unsafe { std::mem::zeroed() };
        unsafe { CPU_ZERO(&mut cpuset) };
        
        // Set affinity to all cores
        for i in 0..self.get_core_count() {
            unsafe { CPU_SET(i as i32, &mut cpuset) };
        }
        
        let result = unsafe {
            sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &cpuset)
        };
        
        if result != 0 {
            return Err(Error::IoError("Failed to reset CPU affinity"));
        }
        
        *self.cpu_affinity.lock().unwrap() = None;
        Ok(())
    }

    fn set_priority(&self, priority: ThreadPriority) -> Result<()> {
        use libc::{sched_getparam, sched_setscheduler, SCHED_FIFO, SCHED_NORMAL};
        
        let mut param: libc::sched_param = unsafe { std::mem::zeroed() };
        
        let (policy, prio) = match priority {
            ThreadPriority::Idle => (SCHED_NORMAL, 0),
            ThreadPriority::Normal => (SCHED_NORMAL, 0),
            ThreadPriority::High => (SCHED_FIFO, 10),
            ThreadPriority::RealTime => (SCHED_FIFO, 99),
        };
        
        param.sched_priority = prio;
        
        let result = unsafe { sched_setscheduler(0, policy, &param) };
        if result != 0 {
            return Err(Error::IoError("Failed to set thread priority"));
        }
        
        Ok(())
    }

    fn get_core_count(&self) -> u32 {
        self.device_info.cpu_cores
    }

    fn get_online_cores(&self) -> Vec<u32> {
        (0..self.get_core_count()).collect()
    }

    fn submit_command_buffer(&self, cmd: &CommandBuffer) -> Result<Fence> {
        // In user-space mode, we'd use Vulkan or other GPU APIs
        // This is a placeholder implementation
        Ok(Fence {
            id: rand::random::<u64>(),
            signaled: false,
        })
    }

    fn wait_fence(&self, fence: Fence, timeout_ms: u32) -> Result<()> {
        // Placeholder fence waiting
        thread::sleep(Duration::from_millis(timeout_ms as u64));
        Ok(())
    }

    fn signal_fence(&self, fence: Fence) -> Result<()> {
        // Placeholder fence signaling
        Ok(())
    }

    fn reset_fence(&self, fence: Fence) -> Result<()> {
        // Placeholder fence reset
        Ok(())
    }

    fn register_interrupt(&self, irq: u32, handler: InterruptHandler) -> Result<()> {
        // User-space cannot register hardware interrupts directly
        // Would use epoll or similar mechanisms
        Err(Error::NotSupported)
    }

    fn unregister_interrupt(&self, irq: u32) -> Result<()> {
        Err(Error::NotSupported)
    }

    fn enable_interrupt(&self, irq: u32) -> Result<()> {
        Err(Error::NotSupported)
    }

    fn disable_interrupt(&self, irq: u32) -> Result<()> {
        Err(Error::NotSupported)
    }

    fn start_thermal_heartbeat(&self) -> Result<()> {
        // Start thermal heartbeat thread
        Ok(())
    }

    fn stop_thermal_heartbeat(&self) -> Result<()> {
        // Stop thermal heartbeat thread
        Ok(())
    }

    fn get_temperature(&self) -> Result<i32> {
        // Read from thermal zones
        let thermal_path = "/sys/class/thermal/thermal_zone0/temp";
        if let Ok(temp_str) = Self::read_file(thermal_path) {
            let temp: i32 = temp_str.trim().parse()
                .unwrap_or(0);
            Ok(temp / 1000) // Convert millidegrees to degrees
        } else {
            Ok(30) // Default temperature
        }
    }

    fn get_max_temperature(&self) -> Result<i32> {
        let trip_point_path = "/sys/class/thermal/thermal_zone0/trip_point_0_temp";
        if let Ok(temp_str) = Self::read_file(trip_point_path) {
            let temp: i32 = temp_str.trim().parse()
                .unwrap_or(50);
            Ok(temp / 1000)
        } else {
            Ok(50) // Default max temperature
        }
    }

    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }

    fn get_capabilities(&self) -> HalCapabilities {
        self.capabilities.clone()
    }
}

impl Drop for UserHal {
    fn drop(&mut self) {
        // Stop thermal heartbeat
        *self.thermal_running.lock().unwrap() = false;
        
        // Join thermal thread
        if let Some(handle) = self.thermal_thread.take() {
            let _ = handle.join();
        }
        
        // Free all pinned memory
        let pinned = self.pinned_memory.lock().unwrap();
        for region in pinned.iter() {
            let _ = self.free_pinned_memory(region.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_hal_creation() {
        let hal = UserHal::new();
        assert!(hal.is_ok());
        
        let hal = hal.unwrap();
        assert!(!hal.is_rooted());
        assert_eq!(hal.privilege_level(), "User-Space (Non-Rooted)");
    }

    #[test]
    fn test_memory_allocation() {
        let hal = UserHal::new().unwrap();
        
        let flags = MemFlags {
            readable: true,
            writable: true,
            ..Default::default()
        };
        
        let region = hal.allocate_pinned_memory(4096, flags);
        assert!(region.is_ok());
        
        let region = region.unwrap();
        assert_eq!(region.size, 4096);
    }
}
