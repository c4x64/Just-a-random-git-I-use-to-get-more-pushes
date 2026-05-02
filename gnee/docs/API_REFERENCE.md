# GNEE API Reference

## HAL Interface

```rust
pub trait HalInterface {
    fn is_rooted(&self) -> bool;
    fn allocate_pinned_memory(&self, size: usize, flags: MemFlags) -> Result<PhysicalRegion>;
    fn free_pinned_memory(&self, region: PhysicalRegion) -> Result<()>;
    fn pin_cpu(&self, core_id: u32) -> Result<()>;
    fn submit_command_buffer(&self, cmd: &CommandBuffer) -> Result<Fence>;
    fn get_temperature(&self) -> Result<i32>;
}
```

## JIT Compiler

```rust
pub struct JitCompiler {
    // Private
}

impl JitCompiler {
    pub fn new(config: JitConfig) -> Self;
    pub fn translate(&mut self, addr: u64, code: &[u8]) -> Result<TranslatedBlock>;
    pub fn stats(&self) -> &JitStats;
    pub fn clear_cache(&mut self);
}
```

## Graphics

```rust
pub struct GraphicsSidecar {
    // Private
}

impl GraphicsSidecar {
    pub fn new(hal: Arc<dyn HalInterface>, config: SidecarConfig) -> Result<Self>;
    pub fn upscale_frame(&self, input: u64, output: u64) -> Result<Fence>;
    pub fn generate_frame(&self, input: u64, output: u64) -> Result<Fence>;
}
```

## Memory

```rust
pub struct MemoryRegion {
    // Private
}

impl MemoryRegion {
    pub fn new(base: PhysAddr, size: usize, flags: MemFlags) -> Self;
    pub fn base(&self) -> PhysAddr;
    pub fn size(&self) -> usize;
    pub fn contains(&self, addr: PhysAddr) -> bool;
}
```

## Thermal

```rust
pub struct ThermalHeartbeat {
    // Private
}

impl ThermalHeartbeat {
    pub fn new(hal: Arc<dyn HalInterface>, config: ThermalHeartbeatConfig) -> Self;
    pub fn start(&mut self) -> Result<()>;
    pub fn stop(&mut self) -> Result<()>;
    pub fn get_temperature(&self) -> Result<i32>;
}
```

## Synchronization

```rust
pub struct Fence {
    pub id: u64,
    pub signaled: Arc<AtomicBool>,
}

impl Fence {
    pub fn new() -> Self;
    pub fn signal(&self);
    pub fn wait(&self, timeout_ms: u32) -> Result<()>;
}
```

## License
SPDX-License-Identifier: GPL-2.0
