// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Resource Tracking
 *
 * Copyright (C) 2025 GNEE Team
 */

//! Resource tracking for GPU resources.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, Result};

/// Resource tracker
pub struct ResourceTracker {
    resources: HashMap<u64, TrackedResource>,
    next_id: AtomicU64,
}

/// Tracked resource
#[derive(Debug, Clone)]
pub struct TrackedResource {
    pub id: u64,
    pub type_: ResourceType,
    pub handle: u64,
    pub size: u64,
    pub creation_time: u64,
    pub last_access: u64,
    pub access_count: u64,
}

/// Resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Image,
    ImageView,
    Buffer,
    BufferView,
    Sampler,
    DescriptorSet,
    DescriptorSetLayout,
    PipelineLayout,
    Pipeline,
    RenderPass,
    Framebuffer,
    ShaderModule,
    QueryPool,
    Semaphore,
    Fence,
    Event,
    DeviceMemory,
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn track(&mut self, type_: ResourceType, handle: u64, size: u64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = self.timestamp();

        let resource = TrackedResource {
            id,
            type_,
            handle,
            size,
            creation_time: now,
            last_access: now,
            access_count: 1,
        };

        self.resources.insert(id, resource);
        id
    }

    pub fn untrack(&mut self, id: u64) -> Result<()> {
        self.resources.remove(&id)
            .ok_or(Error::ResourceNotFound("Resource"))?;
        Ok(())
    }

    pub fn get(&self, id: u64) -> Option<&TrackedResource> {
        self.resources.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut TrackedResource> {
        self.resources.get_mut(&id)
    }

    pub fn access(&mut self, id: u64) -> Result<()> {
        if let Some(resource) = self.resources.get_mut(&id) {
            resource.last_access = self.timestamp();
            resource.access_count += 1;
            Ok(())
        } else {
            Err(Error::ResourceNotFound("Resource"))
        }
    }

    pub fn count(&self) -> usize {
        self.resources.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &TrackedResource> {
        self.resources.values()
    }

    pub fn find_by_type(&self, type_: ResourceType) -> Vec<&TrackedResource> {
        self.resources.values()
            .filter(|r| r.type_ == type_)
            .collect()
    }

    pub fn find_by_handle(&self, handle: u64) -> Option<&TrackedResource> {
        self.resources.values().find(|r| r.handle == handle)
    }

    fn timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn clear(&mut self) {
        self.resources.clear();
    }

    pub fn stats(&self) -> ResourceStats {
        let mut stats = ResourceStats::default();
        
        for resource in self.resources.values() {
            stats.total_count += 1;
            stats.total_size += resource.size;
            
            match resource.type_ {
                ResourceType::Image => stats.image_count += 1,
                ResourceType::Buffer => stats.buffer_count += 1,
                ResourceType::Sampler => stats.sampler_count += 1,
                ResourceType::Pipeline => stats.pipeline_count += 1,
                _ => {}
            }
        }

        stats
    }
}

#[derive(Debug, Default)]
pub struct ResourceStats {
    pub total_count: u64,
    pub total_size: u64,
    pub image_count: u64,
    pub buffer_count: u64,
    pub sampler_count: u64,
    pub pipeline_count: u64,
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let tracker = ResourceTracker::new();
        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_track_resource() {
        let mut tracker = ResourceTracker::new();
        let id = tracker.track(ResourceType::Image, 0x1000, 4096);
        assert_eq!(id, 1);
        assert_eq!(tracker.count(), 1);
    }

    #[test]
    fn test_untrack_resource() {
        let mut tracker = ResourceTracker::new();
        let id = tracker.track(ResourceType::Image, 0x1000, 4096);
        tracker.untrack(id).unwrap();
        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_find_by_type() {
        let mut tracker = ResourceTracker::new();
        tracker.track(ResourceType::Image, 0x1000, 4096);
        tracker.track(ResourceType::Image, 0x2000, 8192);
        tracker.track(ResourceType::Buffer, 0x3000, 1024);
        
        let images = tracker.find_by_type(ResourceType::Image);
        assert_eq!(images.len(), 2);
    }
}
