// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Graphics Module
 *
 * Copyright (C) 2025 GNEE Team
 */

//! # Graphics Module
//!
//! Graphics pipeline for FSR/MFG integration.

pub mod sidecar;
pub mod vulkan_interceptor;
pub mod resources;
pub mod pipeline;
pub mod shaders;

pub use sidecar::{GraphicsSidecar, SidecarConfig, FsrContext, MfgContext};
pub use vulkan_interceptor::{VulkanInterceptor, VulkanInterceptorConfig};
pub use resources::{ResourceTracker, TrackedResource, ResourceType};
pub use pipeline::{GraphicsPipeline, PipelineConfig};
pub use shaders::{ComputeShader, ShaderModule};
