// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - API Shim
 *
 * Copyright (C) 2025 GNEE Team
 *
 * API shim for direct syscall mapping (Windows API -> Linux syscalls).
 */

//! # API Shim
//!
//! Provides direct syscall mapping from Windows API to Linux syscalls.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{Error, Result};

/// API shim configuration
#[derive(Debug, Clone)]
pub struct ShimConfig {
    /// Enable Windows API emulation
    pub windows_api_enabled: bool,
    /// Enable DirectX emulation
    pub directx_enabled: bool,
    /// Enable Vulkan interception
    pub vulkan_intercept: bool,
    /// Enable OpenGL interception
    pub opengl_intercept: bool,
}

impl Default for ShimConfig {
    fn default() -> Self {
        Self {
            windows_api_enabled: true,
            directx_enabled: true,
            vulkan_intercept: true,
            opengl_intercept: false,
        }
    }
}

/// API shim
pub struct ApiShim {
    /// Configuration
    config: ShimConfig,
    /// Syscall mappings
    syscall_mappings: HashMap<u32, SyscallMapping>,
    /// API function mappings
    api_mappings: HashMap<String, ApiMapping>,
    /// Hooked functions
    hooked_functions: HashMap<String, HookedFunction>,
}

/// Syscall mapping
#[derive(Debug, Clone)]
pub struct SyscallMapping {
    /// Windows syscall number
    pub windows_syscall: u32,
    /// Linux syscall number
    pub linux_syscall: u32,
    /// Mapping type
    pub mapping_type: SyscallMappingType,
}

/// Syscall mapping type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallMappingType {
    /// Direct mapping (1:1)
    Direct,
    /// Emulated mapping (requires emulation)
    Emulated,
    /// Stubbed mapping (not supported)
    Stubbed,
}

/// API mapping
#[derive(Debug, Clone)]
pub struct ApiMapping {
    /// API name
    pub name: String,
    /// Target function
    pub target: String,
    /// Mapping type
    pub mapping_type: ApiMappingType,
}

/// API mapping type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiMappingType {
    /// Direct syscall
    DirectSyscall,
    /// Emulated function
    Emulated,
    /// Stubbed function
    Stubbed,
}

/// Hooked function
#[derive(Debug, Clone)]
pub struct HookedFunction {
    /// Function name
    pub name: String,
    /// Original address
    pub original_address: u64,
    /// Hook address
    pub hook_address: u64,
    /// Call count
    pub call_count: u64,
}

impl ApiShim {
    /// Create a new API shim
    pub fn new(config: ShimConfig) -> Self {
        let mut shim = Self {
            config,
            syscall_mappings: HashMap::new(),
            api_mappings: HashMap::new(),
            hooked_functions: HashMap::new(),
        };

        shim.init_mappings();
        shim
    }

    /// Initialize syscall and API mappings
    fn init_mappings(&mut self) {
        // Windows syscalls to Linux syscalls
        // This is a simplified mapping - real implementation would be much more comprehensive

        // File operations
        self.syscall_mappings.insert(0x55, SyscallMapping {
            windows_syscall: 0x55, // NtCreateFile
            linux_syscall: 2,     // open
            mapping_type: SyscallMappingType::Emulated,
        });

        self.syscall_mappings.insert(0x56, SyscallMapping {
            windows_syscall: 0x56, // NtOpenFile
            linux_syscall: 2,     // open
            mapping_type: SyscallMappingType::Emulated,
        });

        self.syscall_mappings.insert(0x5A, SyscallMapping {
            windows_syscall: 0x5A, // NtReadFile
            linux_syscall: 0,     // read
            mapping_type: SyscallMappingType::Emulated,
        });

        self.syscall_mappings.insert(0x5B, SyscallMapping {
            windows_syscall: 0x5B, // NtWriteFile
            linux_syscall: 1,     // write
            mapping_type: SyscallMappingType::Emulated,
        });

        // Memory operations
        self.syscall_mappings.insert(0x18, SyscallMapping {
            windows_syscall: 0x18, // NtAllocateVirtualMemory
            linux_syscall: 9,     // mmap
            mapping_type: SyscallMappingType::Emulated,
        });

        self.syscall_mappings.insert(0x19, SyscallMapping {
            windows_syscall: 0x19, // NtFreeVirtualMemory
            linux_syscall: 11,    // munmap
            mapping_type: SyscallMappingType::Emulated,
        });

        // Process operations
        self.syscall_mappings.insert(0x26, SyscallMapping {
            windows_syscall: 0x26, // NtCreateThread
            linux_syscall: 220,   // clone
            mapping_type: SyscallMappingType::Emulated,
        });

        self.syscall_mappings.insert(0x29, SyscallMapping {
            windows_syscall: 0x29, // NtTerminateProcess
            linux_syscall: 231,   // exit_group
            mapping_type: SyscallMappingType::Direct,
        });

        // Synchronization
        self.syscall_mappings.insert(0x10, SyscallMapping {
            windows_syscall: 0x10, // NtCreateEvent
            linux_syscall: 1,     // eventfd
            mapping_type: SyscallMappingType::Emulated,
        });

        self.syscall_mappings.insert(0x11, SyscallMapping {
            windows_syscall: 0x11, // NtSetEvent
            linux_syscall: 1,     // write to eventfd
            mapping_type: SyscallMappingType::Emulated,
        });

        // API function mappings
        self.api_mappings.insert("CreateFileA".to_string(), ApiMapping {
            name: "CreateFileA".to_string(),
            target: "open".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("CreateFileW".to_string(), ApiMapping {
            name: "CreateFileW".to_string(),
            target: "open".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("ReadFile".to_string(), ApiMapping {
            name: "ReadFile".to_string(),
            target: "read".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("WriteFile".to_string(), ApiMapping {
            name: "WriteFile".to_string(),
            target: "write".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("CloseHandle".to_string(), ApiMapping {
            name: "CloseHandle".to_string(),
            target: "close".to_string(),
            mapping_type: ApiMappingType::Direct,
        });

        self.api_mappings.insert("VirtualAlloc".to_string(), ApiMapping {
            name: "VirtualAlloc".to_string(),
            target: "mmap".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("VirtualFree".to_string(), ApiMapping {
            name: "VirtualFree".to_string(),
            target: "munmap".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("CreateThread".to_string(), ApiMapping {
            name: "CreateThread".to_string(),
            target: "clone".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("Sleep".to_string(), ApiMapping {
            name: "Sleep".to_string(),
            target: "nanosleep".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("GetTickCount".to_string(), ApiMapping {
            name: "GetTickCount".to_string(),
            target: "clock_gettime".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });

        self.api_mappings.insert("QueryPerformanceCounter".to_string(), ApiMapping {
            name: "QueryPerformanceCounter".to_string(),
            target: "clock_gettime".to_string(),
            mapping_type: ApiMappingType::Emulated,
        });
    }

    /// Map Windows syscall to Linux syscall
    pub fn map_syscall(&self, windows_syscall: u32) -> Result<u32> {
        let mapping = self.syscall_mappings.get(&windows_syscall)
            .ok_or(Error::InvalidArgument("Unknown syscall"))?;

        match mapping.mapping_type {
            SyscallMappingType::Direct => Ok(mapping.linux_syscall),
            SyscallMappingType::Emulated => Ok(mapping.linux_syscall),
            SyscallMappingType::Stubbed => Err(Error::NotSupported),
        }
    }

    /// Map Windows API function to Linux equivalent
    pub fn map_api(&self, api_name: &str) -> Result<&str> {
        let mapping = self.api_mappings.get(api_name)
            .ok_or(Error::InvalidArgument("Unknown API"))?;

        match mapping.mapping_type {
            ApiMappingType::DirectSyscall | ApiMappingType::Emulated => Ok(&mapping.target),
            ApiMappingType::Stubbed => Err(Error::NotSupported),
        }
    }

    /// Hook a function
    pub fn hook_function(&mut self, name: String, original_address: u64, hook_address: u64) -> Result<()> {
        let hooked = HookedFunction {
            name: name.clone(),
            original_address,
            hook_address,
            call_count: 0,
        };

        self.hooked_functions.insert(name, hooked);
        Ok(())
    }

    /// Unhook a function
    pub fn unhook_function(&mut self, name: &str) -> Result<()> {
        self.hooked_functions.remove(name)
            .ok_or(Error::ResourceNotFound("Function"))?;
        Ok(())
    }

    /// Get hooked function
    pub fn get_hooked_function(&self, name: &str) -> Option<&HookedFunction> {
        self.hooked_functions.get(name)
    }

    /// Get all hooked functions
    pub fn get_hooked_functions(&self) -> Vec<&HookedFunction> {
        self.hooked_functions.values().collect()
    }

    /// Increment call count for a hooked function
    pub fn increment_call_count(&mut self, name: &str) -> Result<()> {
        if let Some(hooked) = self.hooked_functions.get_mut(name) {
            hooked.call_count += 1;
            Ok(())
        } else {
            Err(Error::ResourceNotFound("Function"))
        }
    }

    /// Get configuration
    pub fn config(&self) -> &ShimConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ShimConfig) {
        self.config = config;
    }

    /// Get syscall mapping count
    pub fn syscall_mapping_count(&self) -> usize {
        self.syscall_mappings.len()
    }

    /// Get API mapping count
    pub fn api_mapping_count(&self) -> usize {
        self.api_mappings.len()
    }

    /// Get hooked function count
    pub fn hooked_function_count(&self) -> usize {
        self.hooked_functions.len()
    }
}

/// Windows API emulator
pub struct WindowsApiEmulator {
    /// API shim
    shim: ApiShim,
    /// Emulated handles
    handles: HashMap<u64, EmulatedHandle>,
    /// Next handle ID
    next_handle: u64,
}

/// Emulated handle
#[derive(Debug, Clone)]
pub struct EmulatedHandle {
    /// Handle ID
    id: u64,
    /// Handle type
    type_: HandleType,
    /// Linux file descriptor
    fd: i32,
}

/// Handle type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleType {
    /// File handle
    File,
    /// Event handle
    Event,
    /// Mutex handle
    Mutex,
    /// Thread handle
    Thread,
    /// Process handle
    Process,
}

impl WindowsApiEmulator {
    /// Create a new Windows API emulator
    pub fn new(shim: ApiShim) -> Self {
        Self {
            shim,
            handles: HashMap::new(),
            next_handle: 1,
        }
    }

    /// Emulate CreateFile
    pub fn create_file(&mut self, path: &str, access: u32, share: u32, creation: u32) -> Result<u64> {
        // Convert Windows flags to Linux flags
        let flags = self.convert_file_flags(access, creation);

        // Open file
        let fd = unsafe {
            libc::open(
                path.as_ptr() as *const i8,
                flags,
                0o666,
            )
        };

        if fd < 0 {
            return Err(Error::IoError("Failed to open file"));
        }

        // Create emulated handle
        let handle_id = self.next_handle;
        self.next_handle += 1;

        let handle = EmulatedHandle {
            id: handle_id,
            type_: HandleType::File,
            fd,
        };

        self.handles.insert(handle_id, handle);
        Ok(handle_id)
    }

    /// Emulate ReadFile
    pub fn read_file(&self, handle: u64, buffer: &mut [u8], bytes_to_read: u32) -> Result<u32> {
        let emulated = self.handles.get(&handle)
            .ok_or(Error::InvalidArgument("Invalid handle"))?;

        if emulated.type_ != HandleType::File {
            return Err(Error::InvalidArgument("Not a file handle"));
        }

        let bytes_read = unsafe {
            libc::read(
                emulated.fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                bytes_to_read as usize,
            )
        };

        if bytes_read < 0 {
            return Err(Error::IoError("Failed to read file"));
        }

        Ok(bytes_read as u32)
    }

    /// Emulate WriteFile
    pub fn write_file(&self, handle: u64, buffer: &[u8], bytes_to_write: u32) -> Result<u32> {
        let emulated = self.handles.get(&handle)
            .ok_or(Error::InvalidArgument("Invalid handle"))?;

        if emulated.type_ != HandleType::File {
            return Err(Error::InvalidArgument("Not a file handle"));
        }

        let bytes_written = unsafe {
            libc::write(
                emulated.fd,
                buffer.as_ptr() as *const libc::c_void,
                bytes_to_write as usize,
            )
        };

        if bytes_written < 0 {
            return Err(Error::IoError("Failed to write file"));
        }

        Ok(bytes_written as u32)
    }

    /// Emulate CloseHandle
    pub fn close_handle(&mut self, handle: u64) -> Result<()> {
        let emulated = self.handles.remove(&handle)
            .ok_or(Error::InvalidArgument("Invalid handle"))?;

        let result = unsafe { libc::close(emulated.fd) };
        if result != 0 {
            return Err(Error::IoError("Failed to close handle"));
        }

        Ok(())
    }

    /// Convert Windows file flags to Linux flags
    fn convert_file_flags(&self, access: u32, creation: u32) -> i32 {
        let mut flags = 0i32;

        // Access flags
        if access & 0x80000000 != 0 {
            // GENERIC_READ
            flags |= libc::O_RDONLY;
        }
        if access & 0x40000000 != 0 {
            // GENERIC_WRITE
            flags |= libc::O_WRONLY;
        }

        // Creation flags
        match creation {
            1 => flags |= libc::O_CREAT,   // CREATE_NEW
            2 => flags |= libc::O_CREAT,   // CREATE_ALWAYS
            3 => flags |= libc::O_TRUNC,   // OPEN_ALWAYS
            4 => {},                       // OPEN_EXISTING
            _ => {},
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shim_config_default() {
        let config = ShimConfig::default();
        assert!(config.windows_api_enabled);
        assert!(config.directx_enabled);
        assert!(config.vulkan_intercept);
    }

    #[test]
    fn test_syscall_mapping() {
        let shim = ApiShim::new(ShimConfig::default());
        let linux_syscall = shim.map_syscall(0x55).unwrap();
        assert_eq!(linux_syscall, 2); // open
    }

    #[test]
    fn test_api_mapping() {
        let shim = ApiShim::new(ShimConfig::default());
        let target = shim.map_api("CreateFileA").unwrap();
        assert_eq!(target, "open");
    }

    #[test]
    fn test_hook_function() {
        let mut shim = ApiShim::new(ShimConfig::default());
        shim.hook_function("test".to_string(), 0x1000, 0x2000).unwrap();
        assert_eq!(shim.hooked_function_count(), 1);
    }
}
