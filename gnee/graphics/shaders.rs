// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Shader Modules
 *
 * Copyright (C) 2025 GNEE Team
 */

//! Shader module management.

use std::sync::Arc;

use crate::error::{Error, Result};

/// Shader module
#[derive(Debug, Clone)]
pub struct ShaderModule {
    id: u64,
    name: String,
    code: Vec<u8>,
    stage: ShaderStage,
}

/// Shader stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    TessellationControl,
    TessellationEvaluation,
    RayGeneration,
    AnyHit,
    ClosestHit,
    Miss,
    Intersection,
    Callable,
}

/// Compute shader
#[derive(Debug)]
pub struct ComputeShader {
    module: ShaderModule,
    workgroup_size: (u32, u32, u32),
}

impl ShaderModule {
    pub fn new(id: u64, name: &str, code: Vec<u8>, stage: ShaderStage) -> Self {
        Self {
            id,
            name: name.to_string(),
            code,
            stage,
        }
    }

    pub fn id(&self) -> u64 { self.id }
    pub fn name(&self) -> &str { &self.name }
    pub fn code(&self) -> &[u8] { &self.code }
    pub fn stage(&self) -> ShaderStage { self.stage }
    pub fn size(&self) -> usize { self.code.len() }
}

impl ComputeShader {
    pub fn new(module: ShaderModule, workgroup_x: u32, workgroup_y: u32, workgroup_z: u32) -> Self {
        Self {
            module,
            workgroup_size: (workgroup_x, workgroup_y, workgroup_z),
        }
    }

    pub fn module(&self) -> &ShaderModule { &self.module }
    
    pub fn workgroup_size(&self) -> (u32, u32, u32) {
        self.workgroup_size
    }

    pub fn total_workgroup_size(&self) -> u32 {
        self.workgroup_size.0 * self.workgroup_size.1 * self.workgroup_size.2
    }
}

/// Shader compiler
pub struct ShaderCompiler {
    next_id: u64,
}

impl ShaderCompiler {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    pub fn compile_spirv(&mut self, name: &str, spirv: &[u8], stage: ShaderStage) -> Result<ShaderModule> {
        let id = self.next_id;
        self.next_id += 1;

        Ok(ShaderModule::new(id, name, spirv.to_vec(), stage))
    }

    pub fn compile_glsl(&mut self, name: &str, glsl: &str, stage: ShaderStage) -> Result<ShaderModule> {
        // This would require a GLSL compiler
        // For now, return stub
        let id = self.next_id;
        self.next_id += 1;

        Ok(ShaderModule::new(id, name, glsl.as_bytes().to_vec(), stage))
    }
}

impl Default for ShaderCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_module_creation() {
        let code = vec![0x00, 0x01, 0x02, 0x03];
        let module = ShaderModule::new(1, "test", code, ShaderStage::Vertex);
        assert_eq!(module.id(), 1);
        assert_eq!(module.name(), "test");
    }

    #[test]
    fn test_compute_shader() {
        let code = vec![0x00, 0x01, 0x02, 0x03];
        let module = ShaderModule::new(1, "test", code, ShaderStage::Compute);
        let shader = ComputeShader::new(module, 16, 16, 1);
        
        assert_eq!(shader.workgroup_size(), (16, 16, 1));
        assert_eq!(shader.total_workgroup_size(), 256);
    }

    #[test]
    fn test_shader_compiler() {
        let mut compiler = ShaderCompiler::new();
        let spirv = vec![0x03, 0x02, 0x23, 0x07];
        let module = compiler.compile_spirv("test", &spirv, ShaderStage::Vertex).unwrap();
        
        assert_eq!(module.name(), "test");
        assert_eq!(module.stage(), ShaderStage::Vertex);
    }
}
