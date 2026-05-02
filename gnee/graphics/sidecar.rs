// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Graphics Sidecar
 *
 * Copyright (C) 2025 GNEE Team
 *
 * Graphics sidecar for FSR/MFG integration and zero-copy buffer sharing.
 */

//! # Graphics Sidecar
//!
//! Provides FSR upscaling, MFG frame generation, and zero-copy buffer sharing.

use std::sync::Arc;
use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::hal::interface::{HalInterface, CommandBuffer, Fence};

/// Graphics sidecar configuration
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// Enable FSR upscaling
    pub fsr_enabled: bool,
    /// Enable MFG frame generation
    pub mfg_enabled: bool,
    /// Upscale factor (1.0 = native, 2.0 = 2x)
    pub upscale_factor: f32,
    /// Use async compute queue
    pub async_compute: bool,
    /// Enable optical flow for motion vectors
    pub optical_flow: bool,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            fsr_enabled: true,
            mfg_enabled: false,
            upscale_factor: 1.5,
            async_compute: true,
            optical_flow: true,
        }
    }
}

/// Graphics sidecar
pub struct GraphicsSidecar {
    /// Configuration
    config: SidecarConfig,
    /// HAL interface
    hal: Arc<dyn HalInterface>,
    /// FSR context
    fsr_context: Option<FsrContext>,
    /// MFG context
    mfg_context: Option<MfgContext>,
    /// Resource tracker
    resource_tracker: ResourceTracker,
    /// Frame buffer registry
    frame_buffers: HashMap<u64, FrameBuffer>,
}

/// FSR context
#[derive(Debug)]
pub struct FsrContext {
    /// FSR version
    version: String,
    /// FSR quality preset
    quality: FsrQuality,
    /// FSR render resolution
    render_width: u32,
    render_height: u32,
    /// FSR display resolution
    display_width: u32,
    display_height: u32,
}

/// FSR quality preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsrQuality {
    /// Ultra Performance (fastest, lowest quality)
    UltraPerformance,
    /// Performance
    Performance,
    /// Balanced
    Balanced,
    /// Quality
    Quality,
    /// Ultra Quality (slowest, highest quality)
    UltraQuality,
}

/// MFG context
#[derive(Debug)]
pub struct MfgContext {
    /// MFG version
    version: String,
    /// Frame generation factor (2.0 = 2x frame rate)
    generation_factor: f32,
    /// Motion vector buffer
    motion_vectors: Option<MotionVectorBuffer>,
    /// Optical flow engine
    optical_flow: Option<OpticalFlowEngine>,
}

/// Motion vector buffer
#[derive(Debug)]
pub struct MotionVectorBuffer {
    /// Buffer ID
    id: u64,
    /// Width
    width: u32,
    /// Height
    u32,
    /// Format
    format: BufferFormat,
    /// Physical address
    phys_addr: u64,
}

/// Buffer format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferFormat {
    R8G8B8A8Unorm,
    R16G16B16A16Sfloat,
    R16G16Sfloat,  // Motion vectors
    D32Sfloat,
    D24UnormS8Uint,
}

/// Optical flow engine
#[derive(Debug)]
pub struct OpticalFlowEngine {
    /// Engine version
    version: String,
    /// Compute shader
    compute_shader: Vec<u8>,
    /// Performance statistics
    stats: OpticalFlowStats,
}

/// Optical flow statistics
#[derive(Debug, Default)]
pub struct OpticalFlowStats {
    /// Total frames processed
    pub total_frames: u64,
    /// Average processing time (nanoseconds)
    pub avg_time_ns: u64,
    /// Maximum processing time
    pub max_time_ns: u64,
    /// Minimum processing time
    pub min_time_ns: u64,
}

/// Resource tracker
#[derive(Debug)]
pub struct ResourceTracker {
    /// Tracked resources
    resources: HashMap<u64, TrackedResource>,
    /// Next resource ID
    next_id: u64,
}

/// Tracked resource
#[derive(Debug)]
pub struct TrackedResource {
    /// Resource ID
    id: u64,
    /// Resource type
    type_: ResourceType,
    /// Resource handle
    handle: u64,
    /// Creation time
    creation_time: u64,
    /// Last access time
    last_access_time: u64,
}

/// Resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// Color buffer
    ColorBuffer,
    /// Depth buffer
    DepthBuffer,
    /// Velocity buffer
    VelocityBuffer,
    /// Texture
    Texture,
    /// Sampler
    Sampler,
    /// Pipeline
    Pipeline,
}

/// Frame buffer
#[derive(Debug)]
pub struct FrameBuffer {
    /// Frame buffer ID
    id: u64,
    /// Width
    width: u32,
    /// Height
    u32,
    /// Format
    format: BufferFormat,
    /// Color buffer
    color_buffer: Option<u64>,
    /// Depth buffer
    depth_buffer: Option<u64>,
    /// Velocity buffer
    velocity_buffer: Option<u64>,
    /// Presentation timestamp
    timestamp: u64,
}

impl GraphicsSidecar {
    /// Create a new graphics sidecar
    pub fn new(hal: Arc<dyn HalInterface>, config: SidecarConfig) -> Result<Self> {
        let mut sidecar = Self {
            config: config.clone(),
            hal,
            fsr_context: None,
            mfg_context: None,
            resource_tracker: ResourceTracker::new(),
            frame_buffers: HashMap::new(),
        };

        // Initialize FSR if enabled
        if config.fsr_enabled {
            sidecar.init_fsr()?;
        }

        // Initialize MFG if enabled
        if config.mfg_enabled {
            sidecar.init_mfg()?;
        }

        Ok(sidecar)
    }

    /// Initialize FSR
    fn init_fsr(&mut self) -> Result<()> {
        let fsr_context = FsrContext {
            version: "2.2.1".to_string(),
            quality: FsrQuality::Balanced,
            render_width: 1920,
            render_height: 1080,
            display_width: (1920.0 * self.config.upscale_factor) as u32,
            display_height: (1080.0 * self.config.upscale_factor) as u32,
        };

        self.fsr_context = Some(fsr_context);
        Ok(())
    }

    /// Initialize MFG
    fn init_mfg(&mut self) -> Result<()> {
        let mfg_context = MfgContext {
            version: "1.0.0".to_string(),
            generation_factor: 2.0,
            motion_vectors: None,
            optical_flow: if self.config.optical_flow {
                Some(OpticalFlowEngine::new()?)
            } else {
                None
            },
        };

        self.mfg_context = Some(mfg_context);
        Ok(())
    }

    /// Track a resource
    pub fn track_resource(&mut self, type_: ResourceType, handle: u64) -> Result<u64> {
        let id = self.resource_tracker.track(type_, handle);
        Ok(id)
    }

    /// Untrack a resource
    pub fn untrack_resource(&mut self, id: u64) -> Result<()> {
        self.resource_tracker.untrack(id)
    }

    /// Get tracked resource
    pub fn get_resource(&self, id: u64) -> Option<&TrackedResource> {
        self.resource_tracker.get(id)
    }

    /// Register frame buffer
    pub fn register_framebuffer(&mut self, fb: FrameBuffer) -> Result<()> {
        self.frame_buffers.insert(fb.id, fb);
        Ok(())
    }

    /// Unregister frame buffer
    pub fn unregister_framebuffer(&mut self, id: u64) -> Result<()> {
        self.frame_buffers.remove(&id);
        Ok(())
    }

    /// Get frame buffer
    pub fn get_framebuffer(&self, id: u64) -> Option<&FrameBuffer> {
        self.frame_buffers.get(&id)
    }

    /// Upscale frame using FSR
    pub fn upscale_frame(&self, input: u64, output: u64) -> Result<Fence> {
        if !self.config.fsr_enabled {
            return Err(Error::NotSupported);
        }

        // Submit FSR upscaling command
        let cmd = CommandBuffer {
            commands: vec![],
            resources: vec![],
        };

        self.hal.submit_command_buffer(&cmd)
    }

    /// Generate frame using MFG
    pub fn generate_frame(&self, input: u64, output: u64) -> Result<Fence> {
        if !self.config.mfg_enabled {
            return Err(Error::NotSupported);
        }

        // Submit MFG frame generation command
        let cmd = CommandBuffer {
            commands: vec![],
            resources: vec![],
        };

        self.hal.submit_command_buffer(&cmd)
    }

    /// Process motion vectors
    pub fn process_motion_vectors(&self, current: u64, previous: u64) -> Result<()> {
        if !self.config.mfg_enabled {
            return Err(Error::NotSupported);
        }

        let mfg_context = self.mfg_context.as_ref()
            .ok_or(Error::InvalidState("MFG not initialized"))?;

        if let Some(optical_flow) = &mfg_context.optical_flow {
            optical_flow.process(current, previous)?;
        }

        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &SidecarConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: SidecarConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }

    /// Get FSR context
    pub fn fsr_context(&self) -> Option<&FsrContext> {
        self.fsr_context.as_ref()
    }

    /// Get MFG context
    pub fn mfg_context(&self) -> Option<&MfgContext> {
        self.mfg_context.as_ref()
    }
}

impl ResourceTracker {
    /// Create a new resource tracker
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            next_id: 1,
        }
    }

    /// Track a resource
    pub fn track(&mut self, type_: ResourceType, handle: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let resource = TrackedResource {
            id,
            type_,
            handle,
            creation_time: now,
            last_access_time: now,
        };

        self.resources.insert(id, resource);
        id
    }

    /// Untrack a resource
    pub fn untrack(&mut self, id: u64) -> Result<()> {
        self.resources.remove(&id)
            .ok_or(Error::ResourceNotFound("Resource"))?;
        Ok(())
    }

    /// Get tracked resource
    pub fn get(&self, id: u64) -> Option<&TrackedResource> {
        self.resources.get(&id)
    }

    /// Get all resources of a type
    pub fn get_by_type(&self, type_: ResourceType) -> Vec<&TrackedResource> {
        self.resources.values()
            .filter(|r| r.type_ == type_)
            .collect()
    }

    /// Get resource count
    pub fn count(&self) -> usize {
        self.resources.len()
    }
}

impl OpticalFlowEngine {
    /// Create a new optical flow engine
    pub fn new() -> Result<Self> {
        Ok(Self {
            version: "1.0.0".to_string(),
            compute_shader: vec![],
            stats: OpticalFlowStats::default(),
        })
    }

    /// Process optical flow
    pub fn process(&self, current: u64, previous: u64) -> Result<()> {
        // Process optical flow using compute shader
        // This would execute the optical flow compute shader
        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> &OpticalFlowStats {
        &self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidecar_config_default() {
        let config = SidecarConfig::default();
        assert!(config.fsr_enabled);
        assert!(!config.mfg_enabled);
        assert_eq!(config.upscale_factor, 1.5);
    }

    #[test]
    fn test_resource_tracker() {
        let mut tracker = ResourceTracker::new();
        let id = tracker.track(ResourceType::ColorBuffer, 0x1000);
        assert_eq!(id, 1);
        assert_eq!(tracker.count(), 1);
    }

    #[test]
    fn test_fsr_quality() {
        assert_eq!(FsrQuality::UltraPerformance as i32, 0);
        assert_eq!(FsrQuality::UltraQuality as i32, 4);
    }
}
