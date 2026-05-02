// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Graphics Pipeline
 *
 * Copyright (C) 2025 GNEE Team
 */

//! Graphics pipeline management.

use std::sync::Arc;

use crate::error::{Error, Result};
use crate::hal::interface::HalInterface;
use crate::graphics::resources::ResourceTracker;

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub msaa_samples: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fullscreen: false,
            vsync: true,
            msaa_samples: 1,
        }
    }
}

/// Graphics pipeline
pub struct GraphicsPipeline {
    config: PipelineConfig,
    resource_tracker: ResourceTracker,
    initialized: bool,
}

impl GraphicsPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            resource_tracker: ResourceTracker::new(),
            initialized: false,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        if self.initialized {
            return Err(Error::InvalidState("Already initialized"));
        }

        // Initialize pipeline
        self.initialized = true;
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Err(Error::InvalidState("Not initialized"));
        }

        self.resource_tracker.clear();
        self.initialized = false;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.config.width = width;
        self.config.height = height;
        Ok(())
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.config.fullscreen = fullscreen;
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        self.config.vsync = vsync;
    }

    pub fn resource_stats(&self) -> crate::graphics::resources::ResourceStats {
        self.resource_tracker.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let config = PipelineConfig::default();
        let pipeline = GraphicsPipeline::new(config);
        assert!(!pipeline.is_initialized());
    }

    #[test]
    fn test_pipeline_init() {
        let config = PipelineConfig::default();
        let mut pipeline = GraphicsPipeline::new(config);
        pipeline.init().unwrap();
        assert!(pipeline.is_initialized());
    }

    #[test]
    fn test_pipeline_resize() {
        let config = PipelineConfig::default();
        let mut pipeline = GraphicsPipeline::new(config);
        pipeline.resize(2560, 1440).unwrap();
        assert_eq!(pipeline.config().width, 2560);
        assert_eq!(pipeline.config().height, 1440);
    }
}
