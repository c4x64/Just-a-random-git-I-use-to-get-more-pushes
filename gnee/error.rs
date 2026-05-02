// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Error Handling
 *
 * Copyright (C) 2025 GNEE Team
 *
 * Comprehensive error types and handling for the GNEE engine.
 */

//! # Error Handling
//!
//! Provides unified error types and result handling across all modules.

use std::fmt;
use std::io;

/// Result type alias for GNEE operations
pub type Result<T> = std::result::Result<T, Error>;

/// GNEE error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// I/O error
    IoError(&'static str),
    /// Invalid argument
    InvalidArgument(&'static str),
    /// Invalid state
    InvalidState(&'static str),
    /// Device not found
    DeviceNotFound,
    /// Device not powered
    DeviceNotPowered,
    /// Device already initialized
    DeviceAlreadyInitialized,
    /// Resource not found
    ResourceNotFound(&'static str),
    /// Resource already exists
    ResourceExists,
    /// Out of memory
    OutOfMemory,
    /// Permission denied
    PermissionDenied,
    /// Timeout
    Timeout,
    /// Busy
    Busy,
    /// Not supported
    NotSupported,
    /// Invalid format
    InvalidFormat,
    /// Invalid mode
    InvalidMode,
    /// Invalid framebuffer
    InvalidFramebuffer,
    /// Invalid buffer
    InvalidBuffer,
    /// Buffer too small
    BufferTooSmall,
    /// Buffer too large
    BufferTooLarge,
    /// Alignment error
    AlignmentError,
    /// Mapping error
    MappingError,
    /// Unmapping error
    UnmappingError,
    /// DMA error
    DmaError,
    /// IOMMU error
    IommuError,
    /// SMMU error
    SmmuError,
    /// GPU error
    GpuError(&'static str),
    /// Command buffer error
    CommandBufferError,
    /// Ring buffer error
    RingBufferError,
    /// Queue error
    QueueError,
    /// Fence error
    FenceError,
    /// Semaphore error
    SemaphoreError,
    /// Sync error
    SyncError,
    /// VSync error
    VsyncError,
    /// CRTC error
    CrtcError,
    /// Connector error
    ConnectorError,
    /// Encoder error
    EncoderError,
    /// Plane error
    PlaneError,
    /// Framebuffer error
    FramebufferError,
    /// Modeset error
    ModesetError,
    /// VirtIO error
    VirtioError,
    /// KVM error
    KvmError,
    /// Hypervisor error
    HypervisorError,
    /// Stage-2 MMU error
    Stage2MmuError,
    /// Memory error
    MemoryError,
    /// Allocation error
    AllocationError,
    /// Deallocation error
    DeallocationError,
    /// JIT error
    JitError(&'static str),
    /// Decoder error
    DecoderError(&'static str),
    /// Emitter error
    EmitterError(&'static str),
    /// Optimization error
    OptimizationError(&'static str),
    /// Thermal error
    ThermalError(&'static str),
    /// Graphics error
    GraphicsError(&'static str),
    /// FSR error
    FsrError(&'static str),
    /// MFG error
    MfgError(&'static str),
    /// Optical flow error
    OpticalFlowError(&'static str),
    /// Custom error
    Custom(&'static str),
}

impl Error {
    /// Create a custom error
    pub fn custom(msg: &'static str) -> Self {
        Error::Custom(msg)
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Busy | Error::Timeout | Error::IoError(_)
        )
    }

    /// Check if this is a fatal error
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Error::OutOfMemory
                | Error::PermissionDenied
                | Error::DeviceNotFound
                | Error::InvalidState(_)
        )
    }

    /// Get error description
    pub fn description(&self) -> &'static str {
        match self {
            Error::IoError(s) => s,
            Error::InvalidArgument(s) => s,
            Error::InvalidState(s) => s,
            Error::DeviceNotFound => "Device not found",
            Error::DeviceNotPowered => "Device not powered",
            Error::DeviceAlreadyInitialized => "Device already initialized",
            Error::ResourceNotFound(s) => s,
            Error::ResourceExists => "Resource already exists",
            Error::OutOfMemory => "Out of memory",
            Error::PermissionDenied => "Permission denied",
            Error::Timeout => "Timeout",
            Error::Busy => "Busy",
            Error::NotSupported => "Not supported",
            Error::InvalidFormat => "Invalid format",
            Error::InvalidMode => "Invalid mode",
            Error::InvalidFramebuffer => "Invalid framebuffer",
            Error::InvalidBuffer => "Invalid buffer",
            Error::BufferTooSmall => "Buffer too small",
            Error::BufferTooLarge => "Buffer too large",
            Error::AlignmentError => "Alignment error",
            Error::MappingError => "Mapping error",
            Error::UnmappingError => "Unmapping error",
            Error::DmaError => "DMA error",
            Error::IommuError => "IOMMU error",
            Error::SmmuError => "SMMU error",
            Error::GpuError(s) => s,
            Error::CommandBufferError => "Command buffer error",
            Error::RingBufferError => "Ring buffer error",
            Error::QueueError => "Queue error",
            Error::FenceError => "Fence error",
            Error::SemaphoreError => "Semaphore error",
            Error::SyncError => "Sync error",
            Error::VsyncError => "VSync error",
            Error::CrtcError => "CRTC error",
            Error::ConnectorError => "Connector error",
            Error::EncoderError => "Encoder error",
            Error::PlaneError => "Plane error",
            Error::FramebufferError => "Framebuffer error",
            Error::ModesetError => "Modeset error",
            Error::VirtioError => "VirtIO error",
            Error::KvmError => "KVM error",
            Error::HypervisorError => "Hypervisor error",
            Error::Stage2MmuError => "Stage-2 MMU error",
            Error::MemoryError => "Memory error",
            Error::AllocationError => "Allocation error",
            Error::DeallocationError => "Deallocation error",
            Error::JitError(s) => s,
            Error::DecoderError(s) => s,
            Error::EmitterError(s) => s,
            Error::OptimizationError(s) => s,
            Error::ThermalError(s) => s,
            Error::GraphicsError(s) => s,
            Error::FsrError(s) => s,
            Error::MfgError(s) => s,
            Error::OpticalFlowError(s) => s,
            Error::Custom(s) => s,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl std::error::Error for Error {}

/// Convert from std::io::Error
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        use io::ErrorKind;
        
        match err.kind() {
            ErrorKind::NotFound => Error::ResourceNotFound("file not found"),
            ErrorKind::PermissionDenied => Error::PermissionDenied,
            ErrorKind::AlreadyExists => Error::ResourceExists,
            ErrorKind::WouldBlock => Error::Busy,
            ErrorKind::TimedOut => Error::Timeout,
            ErrorKind::OutOfMemory => Error::OutOfMemory,
            _ => Error::IoError(err.to_string().leak()),
        }
    }
}

/// Convert from alloc::string::FromUtf8Error
#[cfg(feature = "alloc")]
impl From<alloc::string::FromUtf8Error> for Error {
    fn from(_: alloc::string::FromUtf8Error) -> Self {
        Error::InvalidFormat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::DeviceNotFound;
        assert_eq!(format!("{}", err), "Device not found");
        
        let err = Error::InvalidArgument("test");
        assert_eq!(format!("{}", err), "test");
    }

    #[test]
    fn test_error_retryable() {
        assert!(Error::Busy.is_retryable());
        assert!(Error::Timeout.is_retryable());
        assert!(!Error::OutOfMemory.is_retryable());
    }

    #[test]
    fn test_error_fatal() {
        assert!(Error::OutOfMemory.is_fatal());
        assert!(Error::PermissionDenied.is_fatal());
        assert!(!Error::Busy.is_fatal());
    }

    #[test]
    fn test_error_description() {
        assert_eq!(Error::DeviceNotFound.description(), "Device not found");
        assert_eq!(Error::InvalidArgument("test").description(), "test");
    }
}
